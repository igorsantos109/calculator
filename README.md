# Leitor de Cupons Fiscais

## Visão Geral
App desktop local em Rust que lê fotos de cupons fiscais brasileiros, extrai itens via OCR + LLM local, permite classificação manual (casa / Igor / Renan) e calcula totais por grupo.

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
- **Ollama**: Instale e inicie o Ollama localmente (https://ollama.com)
- **Modelo**: `qwen2.5:7b-instruct-q4_K_M` deve estar disponível
- **GTK**: Necessário para a UI desktop (Linux: `libgtk-3-dev`)

## Executável
O binário principal é `leitor-cupons`, compilado via `cargo run -p ui --release`.