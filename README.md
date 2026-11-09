# Leitor de Cupons Fiscais

## Visão Geral
App desktop local em Rust que lê fotos de cupons fiscais brasileiros, extrai itens via OCR + LLM local, permite classificação manual (casa / Igor / Renan) e calcula totais por grupo.

## Status atual (auditoria deste PR)
- Workspace Rust com crates `core`, `db` e `ui`.
- UI com tela inicial, seleção de imagem, tabela de itens classificados e totais por grupo.
- Core com cálculo de totais e validação de conciliação.
- DB com esquema inicial e operações básicas de salvar/listar/buscar.
- OCR e LLM ainda não integrados no fluxo completo de ponta a ponta.
- Este PR é somente de documentação e planejamento.

## Stack Tecnológica
- **Rust** (edition 2021) com workspace Cargo
- **OCR**: `alvio-ocr` (PP-OCRv6, ONNX)
- **LLM local**: Ollama com modelo `qwen2.5:7b-instruct-q4_K_M`
- **Banco de Dados**: SQLite via `rusqlite`
- **UI**: `eframe` / `egui`

## Estrutura do Workspace
O projeto usa um workspace Cargo com três crates:
- `core`: Tipos, cálculos e lógica de domínio
- `db`: Camada de persistência SQLite
- `ui`: Interface gráfica desktop

## Build e Testes

### Build
```bash
cargo build --workspace
```

### Testes
```bash
cargo test --workspace
```

### Clippy
```bash
cargo clippy --workspace -- -D warnings
```

### Formatação
```bash
cargo fmt --all
```

## Dependências Externas
- **Ollama**: instalação/download de modelo são etapa explícita de setup local; execução do app permanece offline.
- **Modelo**: configurável; `qwen2.5:7b-instruct-q4_K_M` é referência inicial, não decisão final.
- **GTK**: Necessário para a UI desktop (Linux: `libgtk-3-dev`)

## Executável
O binário principal é `leitor-cupons`, compilado via `cargo run -p ui --release`.

## Planejamento por etapas
Backlog sequencial em `/home/runner/work/calculator/calculator/BACKLOG.md` (um PR por etapa, sem iniciar etapa seguinte antes de revisão/merge da atual).

## Comandos de validação executados nesta auditoria
Os comandos abaixo foram executados durante esta auditoria (sem afirmar sucesso quando não houve):

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --lib --bins
```
