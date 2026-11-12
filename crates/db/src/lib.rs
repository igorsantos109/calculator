//! Database layer for the receipt scanner app.
//!
//! This crate provides the `Repositorio` struct that handles all SQLite operations
//! for persisting receipts and items, including schema versioning and migrations.

use receipt_core::{Grupo, Item, ItemClassificado};
use rusqlite::{params, Connection, Result as SqliteResult};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Estrutura que encapsula todos os dados de um cupom fiscal completo.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CupomCompleto {
    /// ID do cupom no banco de dados (0 para novos cupons)
    pub id: i64,
    /// Cabeçalho do recibo
    pub receita: Receita,
    /// Itens classificados do recibo
    pub itens: Vec<ItemClassificado>,
}

/// Cabeçalho do recibo (dados do topo do cupom)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Receita {
    /// Data da compra no formato ISO 8601 (YYYY-MM-DD)
    pub data_compra: String,
    /// Nome do estabelecimento
    pub estabelecimento: String,
    /// Valor total do cupom
    pub total_cupom: f64,
    /// Caminho da imagem do cupom
    pub imagem_path: String,
    /// Quem pagou o cupom
    pub pagador: String,
}

/// Resumo do cupom para listagem
#[derive(Debug, Clone, Serialize)]
pub struct CupomResumo {
    pub id: i64,
    pub data_compra: String,
    pub estabelecimento: String,
    pub total_cupom: f64,
    pub pagador: String,
}

/// Erros deste crate.
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("Erro de banco de dados: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("Cupom não encontrado: id={0}")]
    CupomNaoEncontrado(i64),

    #[error("Item não tem grupo atribuído")]
    ItemSemGrupo,

    #[error("Falha ao criar esquema de banco de dados")]
    EsquemaFalhou,

    #[error("Falha na migração: {0}")]
    MigracaoFalhou(String),

    #[error("Tabela schema_version existe mas não possui a coluna 'version'")]
    SchemaVersionInvalida,

    #[error("Tabela schema_version ausente em banco legado (esperado)")]
    SchemaVersionAusenteBancoLegado,
}

/// Versão atual do schema do banco de dados.
const SCHEMA_VERSION: i64 = 3;

/// Representa uma migração de schema.
#[allow(dead_code)]
struct Migracao {
    versao: i64,
    descricao: &'static str,
    up: &'static str,
    #[allow(dead_code)]
    down: &'static str,
}

/// Lista de migrações ordenadas por versão.
/// v1: Schema inicial (receipts, items) com pagador
/// v2: Registro histórico de migração pagador (já incluída na v1)
/// v3: Adiciona coluna pagador se ausente (migração corretiva para bancos legados)
const MIGRACOES: &[Migracao] = &[
    Migracao {
        versao: 1,
        descricao: "Schema inicial: receipts e items",
        up: "PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS receipts (
    id INTEGER PRIMARY KEY,
    data_compra TEXT,
    estabelecimento TEXT,
    total_cupom REAL NOT NULL,
    imagem_path TEXT,
    pagador TEXT NOT NULL CHECK(pagador IN ('igor','renan')) DEFAULT 'igor',
    criado_em TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS items (
    id INTEGER PRIMARY KEY,
    receipt_id INTEGER NOT NULL REFERENCES receipts(id) ON DELETE CASCADE,
    descricao TEXT NOT NULL,
    quantidade REAL NOT NULL,
    preco_unitario REAL NOT NULL,
    preco_total REAL NOT NULL,
    grupo TEXT NOT NULL CHECK(grupo IN ('casa','igor','renan'))
);

CREATE INDEX IF NOT EXISTS idx_items_receipt ON items(receipt_id);

CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT (datetime('now')),
    description TEXT
);",
        down: "DROP TABLE IF EXISTS items;
DROP TABLE IF EXISTS receipts;
DROP TABLE IF EXISTS schema_version;",
    },
    Migracao {
        versao: 2,
        descricao: "Adiciona coluna pagador em receipts (já incluída na v1, mantida para histórico)",
        up: "-- A coluna pagador já foi incluída na v1.
-- Esta migração serve como registro histórico.
INSERT OR IGNORE INTO schema_version (version, description) VALUES (2, 'Coluna pagador em receipts');",
        down: "-- Não há rollback simples para adição de coluna.
-- Em produção, considerar backup antes de downgrade.",
    },
    Migracao {
        versao: 3,
        descricao: "Adiciona coluna pagador se ausente (migração corretiva para bancos legados sem a coluna)",
        up: "ALTER TABLE receipts ADD COLUMN pagador TEXT NOT NULL CHECK(pagador IN ('igor','renan')) DEFAULT 'igor';

INSERT OR IGNORE INTO schema_version (version, description) VALUES (3, 'Adiciona coluna pagador se ausente');",
        down: "-- Rollback: remove a coluna pagador (não recomendado em produção)
-- ALTER TABLE receipts DROP COLUMN pagador; -- SQLite não suporta DROP COLUMN diretamente
-- Em produção, considerar backup antes de downgrade.",
    },
];

/// Repositório para operações de banco de dados SQLite.
#[allow(dead_code)]
pub struct Repositorio {
    /// Conexão com o banco de dados
    pub conn: Connection,
}

impl Repositorio {
    /// Cria o esquema do banco de dados se não existir.
    ///
    /// Esta função deve ser chamada antes de qualquer operação de leitura/escrita.
    /// Ativa foreign keys e cria as tabelas `receipts`, `items` e `schema_version`.
    pub fn criar_esquema(conn: &mut Connection) -> SqliteResult<(), DbError> {
        // Aplicar migrações na ordem
        for migracao in MIGRACOES {
            Self::aplicar_migracao(conn, migracao)?;
        }
        Ok(())
    }

    /// Aplica uma migração se ainda não foi aplicada.
    fn aplicar_migracao(conn: &mut Connection, migracao: &Migracao) -> SqliteResult<(), DbError> {
        // Verificar se a tabela schema_version existe
        let tabela_existe: bool = match conn.query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='schema_version'",
            [],
            |_| Ok(true),
        ) {
            Ok(v) => v,
            Err(rusqlite::Error::QueryReturnedNoRows) => false,
            Err(e) => return Err(DbError::Sqlite(e)),
        };

        // Verificar se a migração já foi aplicada (se tabela existe)
        let ja_aplicada: bool = if tabela_existe {
            match conn.query_row(
                "SELECT 1 FROM schema_version WHERE version = ?1",
                params![migracao.versao],
                |_| Ok(true),
            ) {
                Ok(v) => v,
                Err(rusqlite::Error::QueryReturnedNoRows) => false,
                Err(e) => return Err(DbError::Sqlite(e)),
            }
        } else {
            false
        };

        if ja_aplicada {
            return Ok(());
        }

        // Para a migração v3 (adicionar coluna pagador se ausente),
        // verificamos se a coluna já existe antes de tentar adicionar
        if migracao.versao == 3 {
            // Verificar se a coluna pagador já existe
            let coluna_existe: bool = match conn.query_row(
                "SELECT 1 FROM pragma_table_info('receipts') WHERE name = 'pagador'",
                [],
                |_| Ok(true),
            ) {
                Ok(v) => v,
                Err(rusqlite::Error::QueryReturnedNoRows) => false,
                Err(e) => return Err(DbError::Sqlite(e)),
            };

            if !coluna_existe {
                // Aplicar migração em transação
                let tx = conn.transaction()?;

                // Adicionar coluna pagador
                tx.execute(
                    "ALTER TABLE receipts ADD COLUMN pagador TEXT NOT NULL CHECK(pagador IN ('igor','renan')) DEFAULT 'igor'",
                    [],
                )?;

                // Registrar versão aplicada
                tx.execute(
                    "INSERT OR IGNORE INTO schema_version (version, description) VALUES (?1, ?2)",
                    params![migracao.versao, migracao.descricao],
                )?;

                tx.commit()?;
            } else {
                // Coluna já existe, apenas registrar a migração se não estiver registrada
                let ja_aplicada: bool = match conn.query_row(
                    "SELECT 1 FROM schema_version WHERE version = ?1",
                    params![migracao.versao],
                    |_| Ok(true),
                ) {
                    Ok(v) => v,
                    Err(rusqlite::Error::QueryReturnedNoRows) => false,
                    Err(e) => return Err(DbError::Sqlite(e)),
                };

                if !ja_aplicada {
                    conn.execute(
                        "INSERT OR IGNORE INTO schema_version (version, description) VALUES (?1, ?2)",
                        params![migracao.versao, migracao.descricao],
                    )?;
                }
            }

            Ok(())
        } else {
            // Aplicar migração em transação
            let tx = conn.transaction()?;

            // Executar SQL UP
            tx.execute_batch(migracao.up).map_err(|e| {
                DbError::MigracaoFalhou(format!(
                    "Falha na migração v{}: {} - {}",
                    migracao.versao, migracao.descricao, e
                ))
            })?;

            // Registrar versão aplicada
            tx.execute(
                "INSERT OR IGNORE INTO schema_version (version, description) VALUES (?1, ?2)",
                params![migracao.versao, migracao.descricao],
            )?;

            tx.commit()?;
            Ok(())
        }
    }

    /// Obtém a versão atual do schema no banco.
    pub fn versao_atual(conn: &Connection) -> SqliteResult<i64, DbError> {
        // Primeiro, verificar se a tabela schema_version existe
        let tabela_existe: bool = match conn.query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='schema_version'",
            [],
            |_| Ok(true),
        ) {
            Ok(v) => v,
            Err(rusqlite::Error::QueryReturnedNoRows) => false,
            Err(e) => return Err(DbError::Sqlite(e)),
        };

        if !tabela_existe {
            // Banco legado sem tabela de versionamento - versão 0
            return Ok(0);
        }

        // Verificar se a tabela tem a coluna 'version'
        let tem_coluna_version: bool = match conn.query_row(
            "SELECT 1 FROM pragma_table_info('schema_version') WHERE name = 'version'",
            [],
            |_| Ok(true),
        ) {
            Ok(v) => v,
            Err(rusqlite::Error::QueryReturnedNoRows) => false,
            Err(e) => return Err(DbError::Sqlite(e)),
        };

        if !tem_coluna_version {
            // Tabela existe mas não tem coluna version - estrutura inválida
            return Err(DbError::SchemaVersionInvalida);
        }

        // Tabela existe e tem coluna version - ler versão
        let versao: i64 = match conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        ) {
            Ok(v) => v,
            Err(rusqlite::Error::QueryReturnedNoRows) => 0,
            Err(e) => return Err(DbError::Sqlite(e)),
        };
        Ok(versao)
    }

    /// Executa todas as migrações pendentes até a versão alvo.
    pub fn migrar(conn: &mut Connection) -> SqliteResult<(), DbError> {
        let versao_atual = Self::versao_atual(conn)?;
        let versoes_pendentes: Vec<&Migracao> = MIGRACOES
            .iter()
            .filter(|m| m.versao > versao_atual)
            .collect();

        for migracao in versoes_pendentes {
            Self::aplicar_migracao(conn, migracao)?;
        }

        Ok(())
    }

    /// Verifica se o banco está na versão mais recente.
    pub fn esta_atualizado(conn: &Connection) -> SqliteResult<bool, DbError> {
        let versao_atual = Self::versao_atual(conn)?;
        Ok(versao_atual >= SCHEMA_VERSION)
    }

    /// Salva um cupom completo no banco de dados dentro de uma transação atômica.
    ///
    /// # Arguments
    /// * `conn` - Conexão mutável com o banco de dados
    /// * `cupom` - Dados do cupom a serem salvos
    ///
    /// # Returns
    /// O ID do recibo inserido, ou erro se a transação falhar.
    pub fn salvar_cupom(
        conn: &mut Connection,
        cupom: &CupomCompleto,
    ) -> SqliteResult<i64, DbError> {
        // Begin transaction
        let tx = conn.transaction()?;

        // Inserir cabeçalho do recibo
        tx.execute(
            "INSERT INTO receipts (data_compra, estabelecimento, total_cupom, imagem_path, pagador) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                cupom.receita.data_compra,
                cupom.receita.estabelecimento,
                cupom.receita.total_cupom,
                cupom.receita.imagem_path,
                cupom.receita.pagador
            ],
        )?;

        let receita_id = tx.last_insert_rowid();

        // Inserir itens
        for item in &cupom.itens {
            tx.execute(
                "INSERT INTO items (receipt_id, descricao, quantidade, preco_unitario, preco_total, grupo) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    receita_id,
                    item.item.descricao,
                    item.item.quantidade,
                    item.item.preco_unitario,
                    item.item.preco_total,
                    format!("{}", item.grupo)
                ],
            )?;
        }

        // Commit transaction
        tx.commit()?;

        Ok(receita_id)
    }

    /// Lista todos os cupons salvos no banco de dados.
    ///
    /// # Returns
    /// Vetor de `CupomResumo` com os cupons encontrados.
    pub fn listar_cupons(conn: &Connection) -> SqliteResult<Vec<CupomResumo>, DbError> {
        let mut stmt = conn.prepare(
            "SELECT id, data_compra, estabelecimento, total_cupom, pagador FROM receipts ORDER BY criado_em DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(CupomResumo {
                id: row.get(0)?,
                data_compra: row.get(1)?,
                estabelecimento: row.get(2)?,
                total_cupom: row.get(3)?,
                pagador: row.get(4)?,
            })
        })?;

        let mut cupons = Vec::new();
        for row in rows {
            cupons.push(row?);
        }

        Ok(cupons)
    }

    /// Busca um cupom específico pelo ID.
    ///
    /// # Arguments
    /// * `id` - ID do recibo a ser buscado
    ///
    /// # Returns
    /// `Some(CupomCompleto)` se encontrado, `None` se não existir.
    pub fn buscar_cupom(
        conn: &Connection,
        id: i64,
    ) -> SqliteResult<Option<CupomCompleto>, DbError> {
        let mut stmt = conn.prepare(
            "SELECT id, data_compra, estabelecimento, total_cupom, imagem_path, pagador FROM receipts WHERE id = ?1",
        )?;

        let result = stmt.query_row([id], |row| {
            Ok(CupomCompleto {
                id: row.get(0)?,
                receita: Receita {
                    data_compra: row.get(1)?,
                    estabelecimento: row.get(2)?,
                    total_cupom: row.get(3)?,
                    imagem_path: row.get(4)?,
                    pagador: row.get(5)?,
                },
                itens: Vec::new(), // Será populado com left join se necessário
            })
        });

        match result {
            Ok(cupom) => Ok(Some(cupom)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Busca itens de um cupom específico pelo ID do recibo.
    ///
    /// # Arguments
    /// * `receipt_id` - ID do recibo cujos itens devem ser buscados
    ///
    /// # Returns
    /// `Vec<ItemClassificado>` com os itens encontrados.
    pub fn buscar_itens_por_receipt_id(
        conn: &Connection,
        receipt_id: i64,
    ) -> SqliteResult<Vec<ItemClassificado>, DbError> {
        let mut stmt = conn.prepare(
            "SELECT descricao, quantidade, preco_unitario, preco_total, grupo FROM items WHERE receipt_id = ?1",
        )?;

        let rows = stmt.query_map([receipt_id], |row| {
            let grupo_str: String = row.get(4)?;
            Ok(ItemClassificado {
                item: Item {
                    descricao: row.get(0)?,
                    quantidade: row.get(1)?,
                    preco_unitario: row.get(2)?,
                    preco_total: row.get(3)?,
                },
                grupo: parse_grupo(&grupo_str),
            })
        })?;

        let mut itens = Vec::new();
        for row in rows {
            itens.push(row?);
        }

        Ok(itens)
    }
}

/// Converte uma string para o enum `Grupo`.
fn parse_grupo(grupo_str: &str) -> Grupo {
    match grupo_str.to_lowercase().as_str() {
        "casa" => Grupo::Casa,
        "igor" => Grupo::Igor,
        "renan" => Grupo::Renan,
        _ => Grupo::Casa, // Default fallback
    }
}

impl Repositorio {
    /// Cria uma nova instância de `Repositorio` conectando ao banco de dados especificado.
    ///
    /// O arquivo do banco de dados será criado se não existir.
    /// Aplica migrações automaticamente.
    ///
    /// # Arguments
    /// * `path` - Caminho para o arquivo do banco de dados SQLite
    ///
    /// # Returns
    /// `Result<Self, DbError>` - Nova instância do repositório ou erro de conexão.
    pub fn new(path: &Path) -> Result<Self, DbError> {
        let mut conn = Connection::open(path).map_err(DbError::from)?;

        // Apply migrations automatically
        Self::migrar(&mut conn)?;

        Ok(Repositorio { conn })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_criar_esquema() {
        let mut conn = Connection::open_in_memory().unwrap();

        let result = Repositorio::criar_esquema(&mut conn);
        assert!(result.is_ok());
    }

    #[test]
    fn test_salvar_e_buscar_cupom() {
        let mut conn = Connection::open_in_memory().unwrap();

        // Garantir esquema
        Repositorio::criar_esquema(&mut conn).unwrap();

        let cupom = CupomCompleto {
            id: 0,
            receita: Receita {
                data_compra: "2024-01-15".to_string(),
                estabelecimento: "Supermercado XYZ".to_string(),
                total_cupom: 125.50,
                imagem_path: "/path/to/image.jpg".to_string(),
                pagador: "igor".to_string(),
            },
            itens: vec![
                ItemClassificado {
                    item: Item {
                        descricao: "Leite".to_string(),
                        quantidade: 2.0,
                        preco_unitario: 5.50,
                        preco_total: 11.00,
                    },
                    grupo: Grupo::Casa,
                },
                ItemClassificado {
                    item: Item {
                        descricao: "Pão".to_string(),
                        quantidade: 1.0,
                        preco_unitario: 8.50,
                        preco_total: 8.50,
                    },
                    grupo: Grupo::Igor,
                },
            ],
        };

        let receipt_id = Repositorio::salvar_cupom(&mut conn, &cupom).unwrap();
        assert_eq!(receipt_id, 1);

        // Buscar cupom
        let encontrado = Repositorio::buscar_cupom(&conn, 1).unwrap();
        assert!(encontrado.is_some());

        let cupom_encontrado = encontrado.unwrap();
        assert_eq!(cupom_encontrado.receita.estabelecimento, "Supermercado XYZ");
        assert_eq!(cupom_encontrado.receita.total_cupom, 125.50);
        assert_eq!(cupom_encontrado.receita.pagador, "igor");

        // Buscar itens
        let itens = Repositorio::buscar_itens_por_receipt_id(&conn, 1).unwrap();
        assert_eq!(itens.len(), 2);
        assert_eq!(itens[0].item.descricao, "Leite");
        assert_eq!(itens[1].item.descricao, "Pão");
    }

    #[test]
    fn test_listar_cupons() {
        let mut conn = Connection::open_in_memory().unwrap();

        Repositorio::criar_esquema(&mut conn).unwrap();

        // Inserir diretamente via SQL para teste
        conn.execute(
            "INSERT INTO receipts (data_compra, estabelecimento, total_cupom, imagem_path, pagador) VALUES ('2024-01-15', 'Teste', 50.0, '/path/img.jpg', 'igor')",
            [],
        ).unwrap();

        let cupons = Repositorio::listar_cupons(&conn).unwrap();
        assert_eq!(cupons.len(), 1);
        assert_eq!(cupons[0].estabelecimento, "Teste");
        assert_eq!(cupons[0].total_cupom, 50.0);
        assert_eq!(cupons[0].pagador, "igor");
    }

    #[test]
    fn test_migracoes() {
        let mut conn = Connection::open_in_memory().unwrap();

        // Aplicar migrações
        let result = Repositorio::migrar(&mut conn);
        assert!(result.is_ok());

        // Verificar versão
        let versao = Repositorio::versao_atual(&conn).unwrap();
        assert_eq!(versao, 3); // SCHEMA_VERSION atual

        // Verificar se está atualizado
        let atualizado = Repositorio::esta_atualizado(&conn).unwrap();
        assert!(atualizado);

        // Tentar aplicar novamente (idempotente)
        let result2 = Repositorio::migrar(&mut conn);
        assert!(result2.is_ok());

        // Verificar tabelas criadas
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table'")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert!(tables.contains(&"receipts".to_string()));
        assert!(tables.contains(&"items".to_string()));
        assert!(tables.contains(&"schema_version".to_string()));
    }

    #[test]
    fn test_migracao_idempotente() {
        let mut conn = Connection::open_in_memory().unwrap();

        // Aplicar migrações múltiplas vezes
        for _ in 0..3 {
            Repositorio::migrar(&mut conn).unwrap();
        }

        let versao = Repositorio::versao_atual(&conn).unwrap();
        assert_eq!(versao, 3);
    }

    #[test]
    fn test_schema_version_table() {
        let mut conn = Connection::open_in_memory().unwrap();
        Repositorio::criar_esquema(&mut conn).unwrap();

        // Verificar conteúdo da tabela schema_version
        let mut stmt = conn
            .prepare("SELECT version, description FROM schema_version ORDER BY version")
            .unwrap();
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .unwrap();

        let versoes: Vec<(i64, String)> = rows.collect::<Result<Vec<_>, _>>().unwrap();

        assert_eq!(versoes.len(), 3);
        assert_eq!(versoes[0].0, 1);
        assert!(versoes[0].1.contains("inicial"));
        assert_eq!(versoes[1].0, 2);
        assert!(versoes[1].1.contains("pagador"));
        assert_eq!(versoes[2].0, 3);
        assert!(versoes[2].1.contains("pagador se ausente"));
    }

    /// Testa que a migração preserva dados existentes de um schema anterior.
    /// Cria um banco com schema v1 (sem schema_version, mas com pagador), insere dados,
    /// executa migração para v2, e verifica integridade dos dados.
    #[test]
    fn test_migracao_preserva_dados_anteriores() {
        let mut conn = Connection::open_in_memory().unwrap();

        // Criar schema v1 (sem schema_version, mas COM pagador - como a migração v1 cria)
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;

CREATE TABLE receipts (
    id INTEGER PRIMARY KEY,
    data_compra TEXT,
    estabelecimento TEXT,
    total_cupom REAL NOT NULL,
    imagem_path TEXT,
    pagador TEXT NOT NULL CHECK(pagador IN ('igor','renan')) DEFAULT 'igor',
    criado_em TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE items (
    id INTEGER PRIMARY KEY,
    receipt_id INTEGER NOT NULL REFERENCES receipts(id) ON DELETE CASCADE,
    descricao TEXT NOT NULL,
    quantidade REAL NOT NULL,
    preco_unitario REAL NOT NULL,
    preco_total REAL NOT NULL,
    grupo TEXT NOT NULL CHECK(grupo IN ('casa','igor','renan'))
);

CREATE INDEX idx_items_receipt ON items(receipt_id);",
        )
        .unwrap();

        // Inserir dados no schema v1
        conn.execute(
            "INSERT INTO receipts (data_compra, estabelecimento, total_cupom, imagem_path, criado_em) 
             VALUES ('2024-01-15', 'Mercado Antigo', 100.0, '/old/img.jpg', '2024-01-15T10:00:00')",
            [],
        ).unwrap();
        let receipt_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO items (receipt_id, descricao, quantidade, preco_unitario, preco_total, grupo) 
             VALUES (?1, 'Leite', 2.0, 5.0, 10.0, 'casa')",
            params![receipt_id],
        ).unwrap();

        conn.execute(
            "INSERT INTO items (receipt_id, descricao, quantidade, preco_unitario, preco_total, grupo) 
             VALUES (?1, 'Pão', 1.0, 8.0, 8.0, 'igor')",
            params![receipt_id],
        ).unwrap();

        // Verificar dados antes da migração
        let count_before: i64 = conn
            .query_row("SELECT COUNT(*) FROM receipts", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count_before, 1);
        let items_before: i64 = conn
            .query_row("SELECT COUNT(*) FROM items", [], |r| r.get(0))
            .unwrap();
        assert_eq!(items_before, 2);

        // Executar migração para v2
        let result = Repositorio::migrar(&mut conn);
        assert!(result.is_ok());

        // Verificar versão
        let versao = Repositorio::versao_atual(&conn).unwrap();
        assert_eq!(versao, 3);

        // Verificar que dados foram preservados
        let count_after: i64 = conn
            .query_row("SELECT COUNT(*) FROM receipts", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count_after, 1);

        let items_after: i64 = conn
            .query_row("SELECT COUNT(*) FROM items", [], |r| r.get(0))
            .unwrap();
        assert_eq!(items_after, 2);

        // Verificar integridade dos dados via repositório
        let cupom = Repositorio::buscar_cupom(&conn, receipt_id)
            .unwrap()
            .unwrap();
        assert_eq!(cupom.receita.estabelecimento, "Mercado Antigo");
        assert_eq!(cupom.receita.total_cupom, 100.0);
        // pagador deve ter valor padrão 'igor' (DEFAULT da migração)
        assert_eq!(cupom.receita.pagador, "igor");

        let itens = Repositorio::buscar_itens_por_receipt_id(&conn, receipt_id).unwrap();
        assert_eq!(itens.len(), 2);
        assert_eq!(itens[0].item.descricao, "Leite");
        assert_eq!(itens[0].grupo, Grupo::Casa);
        assert_eq!(itens[1].item.descricao, "Pão");
        assert_eq!(itens[1].grupo, Grupo::Igor);

        // Executar migração novamente (idempotência)
        let result2 = Repositorio::migrar(&mut conn);
        assert!(result2.is_ok());

        // Verificar que dados não foram alterados
        let count_final: i64 = conn
            .query_row("SELECT COUNT(*) FROM receipts", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count_final, 1);
    }

    /// Testa que falha no salvamento não deixa cupom parcialmente gravado (atomicidade).
    /// Provoca falha via trigger após primeiro item e verifica rollback completo.
    #[test]
    fn test_salvar_cupom_atomicidade_rollback() {
        let mut conn = Connection::open_in_memory().unwrap();
        Repositorio::criar_esquema(&mut conn).unwrap();

        // Inserir cupom existente para verificar preservação
        conn.execute(
            "INSERT INTO receipts (data_compra, estabelecimento, total_cupom, imagem_path, pagador) 
             VALUES ('2024-01-01', 'Existente', 50.0, '/exist.jpg', 'igor')",
            [],
        ).unwrap();
        let existing_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO items (receipt_id, descricao, quantidade, preco_unitario, preco_total, grupo) 
             VALUES (?1, 'Existente Item', 1.0, 50.0, 50.0, 'casa')",
            params![existing_id],
        ).unwrap();

        // Criar trigger que falha no segundo item inserido
        conn.execute_batch(
            "CREATE TEMPORARY TRIGGER trigger_falha_segundo_item
             AFTER INSERT ON items
             WHEN (SELECT COUNT(*) FROM items WHERE receipt_id = NEW.receipt_id) > 1
             BEGIN
                 SELECT RAISE(ABORT, 'falha simulada segundo item');
             END;",
        )
        .unwrap();

        // Tentar salvar cupom com 2 itens - deve falhar no segundo
        let cupom_falha = CupomCompleto {
            id: 0,
            receita: Receita {
                data_compra: "2024-01-20".to_string(),
                estabelecimento: "Teste Falha".to_string(),
                total_cupom: 30.0,
                imagem_path: "/fail.jpg".to_string(),
                pagador: "renan".to_string(),
            },
            itens: vec![
                ItemClassificado {
                    item: Item {
                        descricao: "Primeiro Item".to_string(),
                        quantidade: 1.0,
                        preco_unitario: 15.0,
                        preco_total: 15.0,
                    },
                    grupo: Grupo::Casa,
                },
                ItemClassificado {
                    item: Item {
                        descricao: "Segundo Item".to_string(),
                        quantidade: 1.0,
                        preco_unitario: 15.0,
                        preco_total: 15.0,
                    },
                    grupo: Grupo::Igor,
                },
            ],
        };

        let resultado = Repositorio::salvar_cupom(&mut conn, &cupom_falha);
        assert!(
            resultado.is_err(),
            "Salvamento deveria falhar devido ao trigger"
        );

        // Verificar que cupom NÃO foi inserido (rollback do cabeçalho)
        let count_receipts: i64 = conn
            .query_row("SELECT COUNT(*) FROM receipts", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            count_receipts, 1,
            "Apenas o cupom pré-existente deve permanecer"
        );

        // Verificar que itens NÃO foram inseridos (rollback dos itens)
        let count_items: i64 = conn
            .query_row("SELECT COUNT(*) FROM items", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            count_items, 1,
            "Apenas o item pré-existente deve permanecer"
        );

        // Verificar que dados preexistentes permanecem intactos
        let cupom_existente = Repositorio::buscar_cupom(&conn, existing_id)
            .unwrap()
            .unwrap();
        assert_eq!(cupom_existente.receita.estabelecimento, "Existente");
        assert_eq!(cupom_existente.receita.total_cupom, 50.0);

        let itens_existentes =
            Repositorio::buscar_itens_por_receipt_id(&conn, existing_id).unwrap();
        assert_eq!(itens_existentes.len(), 1);
        assert_eq!(itens_existentes[0].item.descricao, "Existente Item");

        // Limpar trigger temporário
        conn.execute_batch("DROP TRIGGER trigger_falha_segundo_item;")
            .unwrap();
    }

    /// Teste A: Banco legado sem schema_version e sem pagador
    #[test]
    fn test_migracao_banco_legado_sem_pagador() {
        let mut conn = Connection::open_in_memory().unwrap();

        // Criar schema legado SEM pagador (simulando banco antigo real)
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;

CREATE TABLE receipts (
    id INTEGER PRIMARY KEY,
    data_compra TEXT,
    estabelecimento TEXT,
    total_cupom REAL NOT NULL,
    imagem_path TEXT,
    criado_em TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE items (
    id INTEGER PRIMARY KEY,
    receipt_id INTEGER NOT NULL REFERENCES receipts(id) ON DELETE CASCADE,
    descricao TEXT NOT NULL,
    quantidade REAL NOT NULL,
    preco_unitario REAL NOT NULL,
    preco_total REAL NOT NULL,
    grupo TEXT NOT NULL CHECK(grupo IN ('casa','igor','renan'))
);

CREATE INDEX idx_items_receipt ON items(receipt_id);",
        )
        .unwrap();

        // Inserir dados legados
        conn.execute(
        "INSERT INTO receipts (data_compra, estabelecimento, total_cupom, imagem_path, criado_em) 
         VALUES ('2024-01-15', 'Mercado Legado', 150.0, '/legacy/img.jpg', '2024-01-15T10:00:00')",
        [],
    )
    .unwrap();
        let receipt_id = conn.last_insert_rowid();

        conn.execute(
        "INSERT INTO items (receipt_id, descricao, quantidade, preco_unitario, preco_total, grupo) 
         VALUES (?1, 'Arroz', 1.0, 20.0, 20.0, 'casa')",
        params![receipt_id],
    )
    .unwrap();

        conn.execute(
        "INSERT INTO items (receipt_id, descricao, quantidade, preco_unitario, preco_total, grupo) 
         VALUES (?1, 'Feijão', 1.0, 8.0, 8.0, 'igor')",
        params![receipt_id],
    )
    .unwrap();

        // Verificar estrutura antes
        let cols_before: Vec<String> = conn
            .prepare("PRAGMA table_info(receipts)")
            .unwrap()
            .query_map([], |row| row.get(1))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        println!("Colunas antes: {:?}", cols_before);
        assert!(
            !cols_before.contains(&"pagador".to_string()),
            "pagador não deve existir antes"
        );

        // Criar tabela schema_version e inserir v1
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now')),
            description TEXT
        );",
        )
        .unwrap();

        conn.execute(
            "INSERT INTO schema_version (version, description) VALUES (1, 'Schema inicial')",
            [],
        )
        .unwrap();

        // Executar migração
        let result = Repositorio::migrar(&mut conn);
        assert!(result.is_ok(), "Migração deveria ter sucesso");

        // Verificar que coluna pagador foi adicionada
        let cols_after: Vec<String> = conn
            .prepare("PRAGMA table_info(receipts)")
            .unwrap()
            .query_map([], |row| row.get(1))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        println!("Colunas depois: {:?}", cols_after);
        assert!(
            cols_after.contains(&"pagador".to_string()),
            "pagador deve existir depois"
        );

        // Verificar versão
        let versao = Repositorio::versao_atual(&conn).unwrap();
        assert_eq!(versao, 3);

        // Verificar que dados foram preservados via repositório
        let cupom = Repositorio::buscar_cupom(&conn, 1).unwrap().unwrap();
        assert_eq!(cupom.receita.estabelecimento, "Mercado Legado");
        assert_eq!(cupom.receita.total_cupom, 150.0);
        // pagador deve ter valor padrão 'igor' (DEFAULT da coluna adicionada)
        assert_eq!(cupom.receita.pagador, "igor");

        let itens = Repositorio::buscar_itens_por_receipt_id(&conn, 1).unwrap();
        assert_eq!(itens.len(), 2);
        assert_eq!(itens[0].item.descricao, "Arroz");
        assert_eq!(itens[0].grupo, Grupo::Casa);
        assert_eq!(itens[1].item.descricao, "Feijão");
        assert_eq!(itens[1].grupo, Grupo::Igor);
    }

    /// Teste A: Legado realmente sem schema_version.
    /// Cria receipts/items SEM pagador E SEM schema_version,
    /// então chama migrar() que deve adicionar a coluna e registrar v3.
    #[test]
    fn test_legado_sem_schema_version() {
        let mut conn = Connection::open_in_memory().unwrap();

        // Criar schema legado SEM pagador E SEM schema_version
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;

CREATE TABLE receipts (
    id INTEGER PRIMARY KEY,
    data_compra TEXT,
    estabelecimento TEXT,
    total_cupom REAL NOT NULL,
    imagem_path TEXT,
    criado_em TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE items (
    id INTEGER PRIMARY KEY,
    receipt_id INTEGER NOT NULL REFERENCES receipts(id) ON DELETE CASCADE,
    descricao TEXT NOT NULL,
    quantidade REAL NOT NULL,
    preco_unitario REAL NOT NULL,
    preco_total REAL NOT NULL,
    grupo TEXT NOT NULL CHECK(grupo IN ('casa','igor','renan'))
);

CREATE INDEX idx_items_receipt ON items(receipt_id);",
        )
        .unwrap();

        // Inserir dados legados sem pagador
        conn.execute(
            "INSERT INTO receipts (data_compra, estabelecimento, total_cupom, imagem_path, criado_em) 
             VALUES ('2024-03-10', 'Mercado Legado', 200.0, '/legacy2/img.jpg', '2024-03-10T10:00:00')",
            [],
        )
        .unwrap();
        let receipt_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO items (receipt_id, descricao, quantidade, preco_unitario, preco_total, grupo) 
             VALUES (?1, 'Arroz', 3.0, 15.0, 45.0, 'casa')",
            params![receipt_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO items (receipt_id, descricao, quantidade, preco_unitario, preco_total, grupo) 
             VALUES (?1, 'Feijão', 2.0, 25.0, 50.0, 'igor')",
            params![receipt_id],
        )
        .unwrap();

        // Verificar estrutura antes: pagador deve NÃO existir
        let cols_before: Vec<String> = conn
            .prepare("PRAGMA table_info(receipts)")
            .unwrap()
            .query_map([], |row| row.get(1))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert!(
            !cols_before.contains(&"pagador".to_string()),
            "pagador não deve existir antes da migração"
        );

        // NÃO criar schema_version - simular banco legado verdadeiro
        // Executar migração (v3 deve adicionar coluna pagador)
        let result = Repositorio::migrar(&mut conn);
        assert!(
            result.is_ok(),
            "Migração deveria ter sucesso ao adicionar coluna pagador ausente"
        );

        // Verificar que coluna pagador foi adicionada
        let cols_after: Vec<String> = conn
            .prepare("PRAGMA table_info(receipts)")
            .unwrap()
            .query_map([], |row| row.get(1))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert!(
            cols_after.contains(&"pagador".to_string()),
            "pagador deve existir depois da migração"
        );

        // Verificar versão avançou para 3
        let versao = Repositorio::versao_atual(&conn).unwrap();
        assert_eq!(versao, 3, "Versão deveria ser 3 após migração");

        // Verificar dados preservados via repositório
        let cupom = Repositorio::buscar_cupom(&conn, receipt_id)
            .unwrap()
            .unwrap();
        assert_eq!(cupom.receita.estabelecimento, "Mercado Legado");
        assert_eq!(cupom.receita.total_cupom, 200.0);
        // Coluna nova tem DEFAULT 'igor', então pagador deve ser 'igor'
        assert_eq!(cupom.receita.pagador, "igor");

        let itens = Repositorio::buscar_itens_por_receipt_id(&conn, receipt_id).unwrap();
        assert_eq!(itens.len(), 2);
        assert_eq!(itens[0].item.descricao, "Arroz");
        assert_eq!(itens[0].grupo, Grupo::Casa);
        assert_eq!(itens[1].item.descricao, "Feijão");
        assert_eq!(itens[1].grupo, Grupo::Igor);
    }

    /// Teste B: Banco marcado como v2 sem pagador.
    /// Cria schema com v1 e v2 registrados, mas sem coluna pagador,
    /// então migrar() (v3) deve adicionar a coluna preservando dados.
    #[test]
    fn test_banco_marcado_v2_sem_pagador() {
        let mut conn = Connection::open_in_memory().unwrap();

        // Criar tabelas base
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;

CREATE TABLE receipts (
    id INTEGER PRIMARY KEY,
    data_compra TEXT,
    estabelecimento TEXT,
    total_cupom REAL NOT NULL,
    imagem_path TEXT,
    criado_em TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE items (
    id INTEGER PRIMARY KEY,
    receipt_id INTEGER NOT NULL REFERENCES receipts(id) ON DELETE CASCADE,
    descricao TEXT NOT NULL,
    quantidade REAL NOT NULL,
    preco_unitario REAL NOT NULL,
    preco_total REAL NOT NULL,
    grupo TEXT NOT NULL CHECK(grupo IN ('casa','igor','renan'))
);

CREATE INDEX idx_items_receipt ON items(receipt_id);",
        )
        .unwrap();

        // Inserir dados legados
        conn.execute(
            "INSERT INTO receipts (data_compra, estabelecimento, total_cupom, imagem_path, criado_em) 
             VALUES ('2024-06-01', 'Mercado Teste', 300.0, '/test/img.jpg', '2024-06-01T10:00:00')",
            [],
        )
        .unwrap();
        let receipt_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO items (receipt_id, descricao, quantidade, preco_unitario, preco_total, grupo) 
             VALUES (?1, 'Carne', 1.0, 100.0, 100.0, 'casa')",
            params![receipt_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO items (receipt_id, descricao, quantidade, preco_unitario, preco_total, grupo) 
             VALUES (?1, 'Salada', 2.0, 50.0, 100.0, 'igor')",
            params![receipt_id],
        )
        .unwrap();

        // Criar schema_version e registrar versões 1 e 2 (como se tivesse passado pelas migrações antigas)
        // Mas NÃO adicionar coluna pagador (banco pularam esta etapa ou veio de sistema legado)
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT (datetime('now')),
                description TEXT
            );",
        )
        .unwrap();

        conn.execute(
            "INSERT INTO schema_version (version, description) VALUES (1, 'Schema inicial')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO schema_version (version, description) VALUES (2, 'Migração histórica')",
            [],
        )
        .unwrap();

        // Verificar que pagador não existe
        let cols_before: Vec<String> = conn
            .prepare("PRAGMA table_info(receipts)")
            .unwrap()
            .query_map([], |row| row.get(1))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert!(
            !cols_before.contains(&"pagador".to_string()),
            "pagador não deve existir antes da migração v3"
        );

        // Executar migração v3
        let result = Repositorio::migrar(&mut conn);
        assert!(
            result.is_ok(),
            "Migração v3 deveria ter sucesso ao adicionar coluna pagador"
        );

        // Verificar que coluna pagador foi adicionada
        let cols_after: Vec<String> = conn
            .prepare("PRAGMA table_info(receipts)")
            .unwrap()
            .query_map([], |row| row.get(1))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert!(
            cols_after.contains(&"pagador".to_string()),
            "pagador deve existir depois da migração v3"
        );

        // Verificar versão avançou para 3
        let versao = Repositorio::versao_atual(&conn).unwrap();
        assert_eq!(versao, 3, "Versão deveria avançar de 2 para 3");

        // Verificar dados preservados via repositório
        let cupom = Repositorio::buscar_cupom(&conn, receipt_id)
            .unwrap()
            .unwrap();
        assert_eq!(cupom.receita.estabelecimento, "Mercado Teste");
        assert_eq!(cupom.receita.total_cupom, 300.0);
        // Coluna nova tem DEFAULT 'igor', então pagador deve ser 'igor'
        assert_eq!(cupom.receita.pagador, "igor");

        let itens = Repositorio::buscar_itens_por_receipt_id(&conn, receipt_id).unwrap();
        assert_eq!(itens.len(), 2);
        assert_eq!(itens[0].item.descricao, "Carne");
        assert_eq!(itens[0].grupo, Grupo::Casa);
        assert_eq!(itens[1].item.descricao, "Salada");
        assert_eq!(itens[1].grupo, Grupo::Igor);
    }

    /// Teste C: Banco já compatível (com pagador renan explícito).
    /// Insere pagador explicitamente como 'renan', executa migrar() duas vezes,
    /// verifica que renan é preservado e não há duplicação.
    #[test]
    fn test_banco_compativel_com_renan() {
        let mut conn = Connection::open_in_memory().unwrap();

        // Criar schema completo (v1 já inclui pagador)
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;

CREATE TABLE receipts (
    id INTEGER PRIMARY KEY,
    data_compra TEXT,
    estabelecimento TEXT,
    total_cupom REAL NOT NULL,
    imagem_path TEXT,
    pagador TEXT NOT NULL CHECK(pagador IN ('igor','renan')) DEFAULT 'igor',
    criado_em TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE items (
    id INTEGER PRIMARY KEY,
    receipt_id INTEGER NOT NULL REFERENCES receipts(id) ON DELETE CASCADE,
    descricao TEXT NOT NULL,
    quantidade REAL NOT NULL,
    preco_unitario REAL NOT NULL,
    preco_total REAL NOT NULL,
    grupo TEXT NOT NULL CHECK(grupo IN ('casa','igor','renan'))
);

CREATE INDEX idx_items_receipt ON items(receipt_id);

CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT (datetime('now')),
    description TEXT
);",
        )
        .unwrap();

        // Inserir cupom com pagador explicitamente 'renan'
        conn.execute(
            "INSERT INTO receipts (data_compra, estabelecimento, total_cupom, imagem_path, pagador, criado_em) 
             VALUES ('2024-07-01', 'Mercado Atual', 400.0, '/current/img.jpg', 'renan', '2024-07-01T10:00:00')",
            [],
        )
        .unwrap();
        let receipt_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO items (receipt_id, descricao, quantidade, preco_unitario, preco_total, grupo) 
             VALUES (?1, 'Produto A', 2.0, 20.0, 40.0, 'casa')",
            params![receipt_id],
        )
        .unwrap();

        // Registrar versão 3 (já compatível)
        conn.execute(
            "INSERT INTO schema_version (version, description) VALUES (3, 'Schema completo')",
            [],
        )
        .unwrap();

        // Primeira execução de migrar() - deve ser idempotente
        let result1 = Repositorio::migrar(&mut conn);
        assert!(result1.is_ok(), "Primeira migrar() deveria ter sucesso");

        // Verificar que pagador 'renan' foi preservado
        let cupom = Repositorio::buscar_cupom(&conn, receipt_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            cupom.receita.pagador, "renan",
            "Pagador renan deve ser preservado"
        );
        assert_eq!(cupom.receita.estabelecimento, "Mercado Atual");
        assert_eq!(cupom.receita.total_cupom, 400.0);

        // Segunda execução de migrar() - testar idempotência
        let result2 = Repositorio::migrar(&mut conn);
        assert!(
            result2.is_ok(),
            "Segunda migrar() deveria ter sucesso (idempotente)"
        );

        // Verificar que dados ainda estão corretos após segunda execução
        let cupom2 = Repositorio::buscar_cupom(&conn, receipt_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            cupom2.receita.pagador, "renan",
            "Pagador renan deve ser preservado após segunda migrar"
        );
        assert_eq!(cupom2.receita.estabelecimento, "Mercado Atual");
        assert_eq!(cupom2.receita.total_cupom, 400.0);

        // Verificar que não há duplicação de itens
        let itens = Repositorio::buscar_itens_por_receipt_id(&conn, receipt_id).unwrap();
        assert_eq!(itens.len(), 1, "Não deveria haver duplicação de itens");
        assert_eq!(itens[0].item.descricao, "Produto A");
        assert_eq!(itens[0].grupo, Grupo::Casa);
    }

    /// Teste D: Estrutura de versionamento inválida.
    /// schema_version EXISTS mas sem a coluna 'version' - migrar() deve retornar erro,
    /// não tratar banco como novo (não deve mascarar erro como versão 0).
    #[test]
    fn test_estrutura_versionamento_invalida() {
        let mut conn = Connection::open_in_memory().unwrap();

        // Criar tabelas base sem schema_version
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;

CREATE TABLE receipts (
    id INTEGER PRIMARY KEY,
    data_compra TEXT,
    estabelecimento TEXT,
    total_cupom REAL NOT NULL,
    imagem_path TEXT,
    criado_em TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE items (
    id INTEGER PRIMARY KEY,
    receipt_id INTEGER NOT NULL REFERENCES receipts(id) ON DELETE CASCADE,
    descricao TEXT NOT NULL,
    quantidade REAL NOT NULL,
    preco_unitario REAL NOT NULL,
    preco_total REAL NOT NULL,
    grupo TEXT NOT NULL CHECK(grupo IN ('casa','igor','renan'))
);

CREATE INDEX idx_items_receipt ON items(receipt_id);",
        )
        .unwrap();

        // Criar schema_version SEM a coluna 'version' (apenas 'id' column)
        conn.execute_batch(
            "CREATE TABLE schema_version (
                id INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )
        .unwrap();

        // Inserir registro sem a coluna version
        conn.execute(
            "INSERT INTO schema_version (id, applied_at) VALUES (1, '2024-01-01')",
            [],
        )
        .unwrap();

        // versao_atual() deve retornar erro explícito
        let versao = Repositorio::versao_atual(&conn);
        assert!(
            versao.is_err(),
            "versao_atual deveria retornar erro para schema_version inválida"
        );
        match versao {
            Err(DbError::SchemaVersionInvalida) => {
                // Erro esperado
            }
            Err(e) => panic!("Esperado SchemaVersionInvalida, recebido: {:?}", e),
            Ok(v) => panic!("Esperado erro, mas obteve versão {}", v),
        }

        // migrar() deve propagar o erro
        let result = Repositorio::migrar(&mut conn);
        assert!(
            result.is_err(),
            "migrar() deveria retornar erro para schema_version inválida"
        );
        match result {
            Err(DbError::SchemaVersionInvalida) => {
                // Erro esperado
            }
            Err(e) => panic!("Esperado SchemaVersionInvalida, recebido: {:?}", e),
            Ok(_) => panic!("Esperado erro, mas migrar() retornou Ok"),
        }
    }
}
