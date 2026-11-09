//! Database layer for the receipt scanner app.
//!
//! This crate provides the `Repositorio` struct that handles all SQLite operations
//! for persisting receipts and items.

use receipt_core::{Grupo, Item, ItemClassificado};
use rusqlite::{params, Connection, Result as SqliteResult};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Estrutura que encapsula todos os dados de um cupom fiscal completo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CupomCompleto {
    /// Cabeçalho do recibo
    pub receita: Receita,
    /// Itens classificados do recibo
    pub itens: Vec<ItemClassificado>,
}

/// Cabeçalho do recibo (dados do topo do cupom)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receita {
    /// Data da compra no formato ISO 8601 (YYYY-MM-DD)
    pub data_compra: String,
    /// Nome do estabelecimento
    pub estabelecimento: String,
    /// Valor total do cupom
    pub total_cupom: f64,
    /// Caminho da imagem do cupom
    pub imagem_path: String,
}

/// Resumo do cupom para listagem
#[derive(Debug, Clone, Serialize)]
pub struct CupomResumo {
    pub id: i64,
    pub data_compra: String,
    pub estabelecimento: String,
    pub total_cupom: f64,
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
}

/// Repositório para operações de banco de dados SQLite.
#[allow(dead_code)]
pub struct Repositorio {
    /// Conexão com o banco de dados
    conn: Connection,
}

impl Repositorio {
    /// Cria o esquema do banco de dados se não existir.
    ///
    /// Esta função deve ser chamada antes de qualquer operação de leitura/escrita.
    /// Ativa foreign keys e cria as tabelas `receipts` e `items`.
    pub fn criar_esquema(conn: &Connection) -> SqliteResult<(), DbError> {
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;

    CREATE TABLE IF NOT EXISTS receipts (
        id INTEGER PRIMARY KEY,
        data_compra TEXT,
        estabelecimento TEXT,
        total_cupom REAL NOT NULL,
        imagem_path TEXT,
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

    CREATE INDEX IF NOT EXISTS idx_items_receipt ON items(receipt_id);",
        )?;

        Ok(())
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
            "INSERT INTO receipts (data_compra, estabelecimento, total_cupom, imagem_path) VALUES (?1, ?2, ?3, ?4)",
            params![
                cupom.receita.data_compra,
                cupom.receita.estabelecimento,
                cupom.receita.total_cupom,
                cupom.receita.imagem_path
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
            "SELECT id, data_compra, estabelecimento, total_cupom FROM receipts ORDER BY criado_em DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(CupomResumo {
                id: row.get(0)?,
                data_compra: row.get(1)?,
                estabelecimento: row.get(2)?,
                total_cupom: row.get(3)?,
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
            "SELECT id, data_compra, estabelecimento, total_cupom, imagem_path FROM receipts WHERE id = ?1",
        )?;

        let result = stmt.query_row([id], |row| {
            Ok(CupomCompleto {
                receita: Receita {
                    data_compra: row.get(1)?,
                    estabelecimento: row.get(2)?,
                    total_cupom: row.get(3)?,
                    imagem_path: row.get(4)?,
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
    ///
    /// # Arguments
    /// * `path` - Caminho para o arquivo do banco de dados SQLite
    ///
    /// # Returns
    /// `Result<Self, DbError>` - Nova instância do repositório ou erro de conexão.
    pub fn new(path: &Path) -> Result<Self, DbError> {
        let conn = Connection::open(path).map_err(DbError::from)?;

        // Create schema immediately
        Self::criar_esquema(&conn)?;

        Ok(Repositorio { conn })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_criar_esquema() {
        let conn = Connection::open_in_memory().unwrap();

        let result = Repositorio::criar_esquema(&conn);
        assert!(result.is_ok());
    }

    #[test]
    fn test_salvar_e_buscar_cupom() {
        let mut conn = Connection::open_in_memory().unwrap();

        // Garantir esquema
        Repositorio::criar_esquema(&mut conn).unwrap();

        let cupom = CupomCompleto {
            receita: Receita {
                data_compra: "2024-01-15".to_string(),
                estabelecimento: "Supermercado XYZ".to_string(),
                total_cupom: 125.50,
                imagem_path: "/path/to/image.jpg".to_string(),
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
            "INSERT INTO receipts (data_compra, estabelecimento, total_cupom, imagem_path) VALUES ('2024-01-15', 'Teste', 50.0, '/path/img.jpg')",
            [],
        ).unwrap();

        let cupons = Repositorio::listar_cupons(&conn).unwrap();
        assert_eq!(cupons.len(), 1);
        assert_eq!(cupons[0].estabelecimento, "Teste");
        assert_eq!(cupons[0].total_cupom, 50.0);
    }
}
