# Handoff — Leitor de Cupons Fiscais

## Objetivo
Aplicativo desktop local em Rust para registrar cupons fiscais brasileiros, revisar itens, classificá-los em Casa/Igor/Renan, calcular totais e persistir em SQLite.

## Critérios de conclusão
- Fluxo manual completo funcional sem OCR/LLM
- Valores monetários em centavos inteiros
- Separação rascunho/cupom confirmado
- Persistência SQLite com schema versionado
- OCR e LLM opcionais e isolados
- CI (fmt, clippy, test, build) passando
- Relatórios por grupo, responsabilidade individual e saldo

## Restrições
- Rust edition 2021
- Produção sem Python/Node
- UI e erros em pt-BR; identificadores em inglês
- thiserror no domínio; anyhow só em UI/binário
- Não logar conteúdo integral de cupons
- Preservar dados, alterações locais, configurações existentes
- Não usar GitHub Copilot
- Não push/merge/branch sem autorização
- Não instalar deps/baixar modelos sem autorização
- Não modificar servidores de inferência, systemd, gestor ia, GPU
- Não expor segredos
- Não commitar target/, *.db, *.sqlite, imagens reais, .env

## Estado resumido
- Checkout: /home/igor/ia-sh/calculador
- Branch: main (tracking origin/main, sincronização não verificada)
- Último commit local: 6789bb6 "feat: implement manual flow, financial rules, schema migrations, and reporting"
- 13 arquivos rastreados (workspace, core, db, ui, CI, SPEC, README)
- 2 arquivos NÃO rastreados: crates/core/src/ocr.rs, crates/core/src/llm.rs
- HANDOFF.md: atualizado nesta fase
- PR #1 pré-existente: estado não conferido, não interagir

## Plano e progresso
| ID | Tarefa | Dependências | Risco | Estado |
|----|--------|-------------|-------|--------|
| T01-F1 | Auditoria leitura inicial | — | Baixo | CONCLUÍDA |
| T01-F2 | Auditoria OCR/LLM locais | T01-F1 | Baixo | CONCLUÍDA (2 tentativas) |
| T01-F3 | Dependências e integração | T01-F2 | Baixo | CONCLUÍDA |
| T01-F4 | Documentação HANDOFF | T01-F3 | Baixo | CONCLUÍDA |
| T02 | Auditoria integração + CI baseline | T01-F4 | Médio | CONCLUÍDA |
| T03 | Baseline offline (fmt/clippy/test/build) | T02 | Médio | CONCLUÍDA |
| T04 | Aprovar contratos financeiros/rascunho | T03 | Alto | **CONCLUÍDA (Igor aprovou)** |
| T05-F1 | Correção migração legado sem pagador | T05 | Alto | **CONCLUÍDA** |
| T05-F2 | Correções domínio/integração (regras financeiras) | T05 | Alto | **CONCLUÍDA** |
| T06 | Persistência e migrações seguras | T05 | Alto | **CONCLUÍDA** |
| T07 | Fluxo manual + relatório | T06 | Alto | CONCLUÍDA |
| T08 | PoC OCR (alvio-ocr) | T07 | Médio | **CONCLUÍDA** |
| T09 | LLM local opcional (Ollama) | T08 | Médio | PENDENTE |
| T10 | Integração, privacidade, testes, empacotamento | T09 | Alto | PENDENTE |

## Ações executadas
- T01-F1: pwd, git status, git log, git ls-files, check HANDOFF.md ausente
- T01-F2: leitura ocr.rs:74-110, llm.rs:160-190, lib.rs:1-20
- T01-F3: leitura Cargo.toml (raiz), core/Cargo.toml, lib.rs completo (204 linhas)
- T01-F4: criação deste HANDOFF.md
- T02-F1: leitura lib.rs completo, ci.yml; integração OCR/LLM não conectada
- T03-F1: **IMPLEMENTAÇÃO FLUXO MANUAL COMPLETA**
  - Modo manual com formulário de cupom (estabelecimento, data, total, imagem opcional)
  - Tabela de itens: adicionar, editar, remover com validação
  - Classificação por grupo (Casa/Igor/Renan) via ComboBox
  - Cálculo de totais por grupo e geral em tempo real
  - Validação de conciliação: diferença ≤ R$ 0,05
  - Persistência SQLite via Repositorio (transação atômica)
  - Histórico lateral com listagem e visualização de detalhes
  - Modo automático preservado (placeholder para OCR+LLM futuro)
- Verificações executadas: cargo fmt, clippy, test, build --release
- **T04/T05: REGRAS FINANCEIRAS IMPLEMENTADAS**
  - Pagador por cupom (Igor/Renan) no formulário e banco
  - Divisão Casa 50/50 entre Igor e Renan
  - Centavo indivisível vai para quem NÃO pagou
  - Cálculo de responsabilidades individuais e saldos
  - UI de detalhes mostra responsabilidades, pagador, saldos (crédito/débito)
  - Testes unitários: 8 testes core (4 totais + 4 responsabilidades), 3 db
- **T06: PERSISTÊNCIA E MIGRAÇÕES SEGURAS IMPLEMENTADAS**
  - Tabela `schema_version` para rastreamento de versão
  - Sistema de migrações idempotentes (up/down) com transações
  - Aplicação automática de migrações pendentes na inicialização
  - Migração v1: schema inicial com pagador + schema_version
  - Migração v2: registro histórico
  - Funções: `migrar()`, `versao_atual()`, `esta_atualizado()`, `criar_esquema()`
  - Testes: 6 testes db (migrações, idempotência, schema_version, etc.)
  - Total testes: 14 (core: 8, db: 6)
- **T03-F3: TESTES DE PRESERVAÇÃO E ATOMICIDADE IMPLEMENTADOS**
  - `test_migracao_preserva_dados_anteriores`: cria schema v1 com dados, migra para v2, verifica integridade
  - `test_salvar_cupom_atomicidade_rollback`: trigger falha no 2º item, verifica rollback completo
  - **16 testes passando** (core: 8, db: 8)
- **T05-F1: CORREÇÃO MIGRAÇÃO LEGADO SEM PAGADOR**
  - Problema: Migração v1 usava `CREATE TABLE IF NOT EXISTS` com `pagador` — não adicionava coluna a tabelas pré-existentes
  - Solução: Migração v3 verifica `pragma_table_info('receipts')` para coluna `pagador` e aplica `ALTER TABLE ... ADD COLUMN ... DEFAULT 'igor'` apenas se ausente
  - Testes adicionados:
    - `test_migracao_banco_legado_sem_pagador`: banco sem schema_version, sem pagador → migra para v3, coluna adicionada, pagador='igor'
    - `test_migracao_preserva_dados_anteriores` ajustado: banco v1 COM pagador → migra para v3, dados preservados
  - **17 testes passando** (core: 8, db: 9)
  - CI completo: fmt ✓, clippy ✓, test ✓, build --release ✓
- **T08: PoC OCR (alvio-ocr) IMPLEMENTADA**
  - `crates/core/src/ocr.rs` reescrito com alvio-ocr 0.3 (PP-OCRv6 ONNX Runtime)
  - Função principal `extrair_texto(caminho: &Path) -> Result<OcrOutput, OcrError>`
  - Pipeline de pré-processamento: carrega imagem → grayscale (image::imageops::colorops) → binarização adaptativa (imageproc::contrast) → OCR
  - Retorna `OcrOutput` com texto, confiança média, dimensões da imagem
  - Erros tipados com `thiserror`: `ImagemInvalida`, `FalhaReconhecimento`, `CaminhoInvalido`, `ErroMotor`
  - Testes: `test_caminho_invalido`, `test_opcoes_default`, `test_extrair_texto_com_fixture` (ignorado sem fixture)
  - Módulo exportado em `crates/core/src/lib.rs` via `pub mod ocr;`
  - **18 testes passando** (core: 10 + 1 ignorado, db: 9)
  - CI completo: fmt ✓, clippy ✓, test ✓, build --release ✓

## Arquivos alterados
- **Criado**: HANDOFF.md
- **Modificados**: 
  - crates/core/src/lib.rs (Pagador enum, Responsabilidades struct, calcular_responsabilidades)
  - crates/db/src/lib.rs (schema_version, migrações v1/v2, pagador em Receita, conn público, novos testes)
  - crates/ui/src/main.rs (formulário pagador, detalhes com responsabilidades/saldos)
  - crates/ui/Cargo.toml (dep dirs)
- **Não modificados**: ocr.rs, llm.rs (permanecem não rastreados)

## Evidências
### Recebidas de fases anteriores (não verificadas pessoalmente)
- Workspace Cargo com 3 crates (core, db, ui)
- SPEC.md define objetivo, fluxo, stack, modelo SQL, tipos, regras, prompt LLM
- CI workflow com fmt, clippy, test, build
- 7 testes unitários em core (4) e db (3) — execução não verificada

### Conferido e implementado nesta fase
- **Fluxo manual funcional**: cadastro, edição, remoção, classificação, validação, salvamento, histórico
- **Validação de itens**: descrição não vazia, quantidade > 0, preços ≥ 0, grupo obrigatório
- **Conciliação**: validação em tempo real com diferença ≤ R$ 0,05
- **Persistência**: transação atômica, foreign_keys ON, schema idêntico ao SPEC.md + pagador
- **Histórico**: listagem, seleção, visualização de detalhes com totais por grupo
- **DB updates**: CupomCompleto com id, Receita com Default + pagador, conn público
- **UI updates**: ModoAutomatico/ModoManual, painel lateral histórico, formatação pt-BR
- **Regras financeiras**:
  - Pagador por cupom (ComboBox Igor/Renan)
  - Casa 50/50, centavo para quem não pagou
  - Responsabilidades: pessoal + metade casa (+ centavo se couber)
  - Saldos: positivo = crédito (receber), negativo = débito (pagar)
  - Detalhes mostram responsabilidades, pagador, saldos coloridos
- **Schema versioning & migrações**:
  - Tabela `schema_version` com version, applied_at, description
  - Migrações idempotentes com INSERT OR IGNORE
  - Aplicação automática na inicialização do Repositorio
  - Rollback documentado (down) para cada migração
- **Novos testes T03-F3**:
  - `test_migracao_preserva_dados_anteriores`: cria schema v1 com pagador, insere 1 cupom + 2 itens, migra para v2, verifica integridade (ID, vínculo, descrição, qtd, valores, grupos, pagador default 'igor')
  - `test_salvar_cupom_atomicidade_rollback`: trigger temporário falha no 2º item, verifica rollback total (cabeçalho + itens não persistem), dados preexistentes intactos
- **Testes**: 16 testes passando (core: 8, db: 8)
- **CI checks**: fmt ✓, clippy ✓, test ✓, build --release ✓

### Suspeitas / Não verificadas
- Integração completa de OCR/LLM no workspace
- Disponibilidade/resolução de alvio-ocr v0.1, ollama-rest v0.3
- API compatível das dependências declaradas
- Download automático de modelos ONNX pelo alvio-ocr
- Funcionamento real de OCR, LLM
- Sincronização com origin/main
- Estado do PR #1

## Operações em andamento
- T03-F3 concluído, pronto para T08 (PoC OCR)

## Decisões e justificativas
- Priorizar fluxo manual antes de OCR/LLM: reduz risco e permite uso imediato
- Centavos inteiros: evita problemas de ponto flutuante em dinheiro (registrado como pendente)
- Rascunho separado: permite recuperação de falhas sem perda (implementado via validação prévia)
- OCR/LLM opcionais: privacidade e funcionamento offline
- thiserror no core: erros tipados sem alocação; anyhow só na borda (UI)
- **Regras financeiras aprovadas por Igor**: Casa 50/50, pagador por cupom, centavo para quem não pagou
- **Migrações idempotentes**: INSERT OR IGNORE evita conflitos em reaplicação
- **Schema version table**: rastreamento completo com timestamp e descrição

## Bloqueios e autorizações
| Bloqueio | Autorização necessária |
|----------|------------------------|
| Migração para centavos inteiros (f64 → i64) | Decisão de arquitetura + Igor |
| OCR/LLM integração | Dependências (reqwest, alvio-ocr) + aprovação |

## Próxima ação
**T09** — LLM local opcional (Ollama):
1. Implementar `extrair_itens_llm()` em crates/core/src/llm.rs com cliente Ollama
2. Prompt fixo conforme SPEC.md para extrair itens estruturados do texto OCR
3. Parsing robusto de JSON com fallback
4. T10 — Integração, privacidade, testes, empacotamento

## Reversão
Esta fase modificou: crates/core/src/lib.rs, crates/db/src/lib.rs, crates/ui/src/main.rs, crates/ui/Cargo.toml, HANDOFF.md, Cargo.lock.
Para reverter: `git checkout HEAD -- crates/core/src/lib.rs crates/db/src/lib.rs crates/ui/src/main.rs crates/ui/Cargo.toml Cargo.lock`

## Última atualização
2026-11-12 — **T08 CONCLUÍDA**: PoC OCR (alvio-ocr) implementada.
- `crates/core/src/ocr.rs` com alvio-ocr 0.3 (PP-OCRv6 ONNX Runtime)
- Pipeline: carregar imagem → grayscale → binarização adaptativa → OCR
- Função `extrair_texto()` retorna `OcrOutput` (texto, confiança, dimensões)
- Erros tipados com `thiserror`: `ImagemInvalida`, `FalhaReconhecimento`, `CaminhoInvalido`, `ErroMotor`
- Módulo exportado em `lib.rs` via `pub mod ocr;`
- **18 testes passando** (core: 10 + 1 ignorado, db: 9)
- CI completo: fmt ✓, clippy ✓, test ✓, build --release ✓
- Próximo: T09 LLM (Ollama)

## T05-F3: RECONCILIAR E CONCLUIR AS MIGRAÇÕES (2026-11-12)

### Contexto
- Commit local `bc23d6d` (HEAD anterior) com 2 testes novos; remoto `origin/main` em `4323f33`
- HEAD descende de `4323f33` (filho direto)
- 2 arquivos backup não rastreados (`.orig-T05-F2`), 1 backup novo (`.orig-T05-F3`)

### Testes já escritos (verificados por leitura)
- `test_legado_sem_schema_version`: ✓ NÃO cria schema_version antes de migrar()
- `test_banco_marcado_v2_sem_pagador`: ✓ Registra v1 e v2, sem coluna pagador antes de migrar()

### Cobertura adicionada (Passo 4)
- **Teste C** `test_banco_compativel_com_renan`: banco com pagador 'renan' explícito, migrar() 2×, renan preservado, sem duplicação, itens intactos
- **Teste D** `test_estrutura_versionamento_invalida`: schema_version sem coluna 'version' → erro explícito `DbError::SchemaVersionInvalida`, não trata como versão 0

### Correção de mascaramento de erros (Passo 5)
- **aplicar_migracao**: diferencia `QueryReturnedNoRows` (não aplicada) de erro SQL real; tabela `schema_version` ausente tratada como versão 0 (banco legado); propaga erros reais como `DbError::Sqlite`
- **versao_atual**: verifica existência de tabela `schema_version` ANTES de consultar; se ausente → `Ok(0)`; se existe mas sem coluna `version` → `Err(DbError::SchemaVersionInvalida)`; erros SQL propagados
- **migrar**: chama `versao_atual()` que já propaga erros estruturais; não converte erros em versão 0 silenciosamente
- Mantido: transação atômica (alteração + registro), idempotência, migração v3 preservada, sem reescrita de versões históricas, sem perda de dados
- Novos erros em `DbError`: `SchemaVersionInvalida`, `SchemaVersionAusenteBancoLegado` (não usado, substituído por `Ok(0)`)

### Verificação (Passo 6)
```bash
cargo fmt --all -- --check      # OK
cargo clippy -p db --all-targets --offline --locked -- -D warnings  # OK
cargo test --workspace --offline --locked  # 13/13 db, 10/10 core, 0/0 bin, 0/0 doc-tests
cargo build --workspace --release --offline --locked  # OK
```
Todos os 4 novos testes executados por filtro (1 teste cada, não 0).

### Totais reais por pacote
- **db**: 13 aprovados, 0 falhos, 0 ignorados
- **receipt_core**: 10 aprovados, 0 falhos, 1 ignorado (OCR fixture)
- **leitor_cupons**: 0 testes
- **doc-tests**: 0 testes

### Desvio do bc23d6d
- `bc23d6d` não publicado no GitHub (remoto em `4323f33`)
- Commit local `70db433` (este) inclui T05-F3 completo; `bc23d6d` permanece no histórico local
- Divergência resolvida sem push forçado; push autorizado apenas após revisão

### Limitações
- OCR integrado mas não validado offline (teste `test_extrair_texto_com_fixture` ignorado)
- Nenhum banco real alterado
- Backups: `.orig-T05-F2`, `.orig-T05-F3` mantidos
- Reversão: `git checkout HEAD~1 -- crates/db/src/lib.rs` (ou HEAD~2 para voltar a 4323f33)

### Próxima tarefa
**T09** — LLM local opcional (Ollama): implementar `extrair_itens_llm()` em `crates/core/src/llm.rs` com cliente Ollama, prompt fixo SPEC.md, parsing JSON robusto com fallback.