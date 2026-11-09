# Leitor de Cupons Fiscais — Especificação

## Objetivo
App desktop local que lê cupons fiscais brasileiros e divide gastos entre três entes:
- **Casa** (comum)
- **Igor** (pessoal)
- **Renan** (pessoal)

## Fluxo do usuário
1. Seleciona imagem do cupom.
2. App executa OCR → texto bruto.
3. LLM local converte texto em `Vec<Item>` (JSON estruturado).
4. App valida: soma dos itens ≈ total do cupom.
5. Usuário revisa/edita tabela de itens e atribui grupo a cada um.
6. App calcula e exibe totais por grupo + relatório por pessoa.
7. App salva cupom + itens no SQLite.

## Stack
| Camada | Crate/Tecnologia |
|---|---|
| OCR | `alvio-ocr` (PP-OCRv6, ONNX) |
| LLM local | Ollama via `ollama-rest` — modelo `qwen2.5:7b-instruct-q4_K_M` |
| Pré-processamento | `image`, `imageproc` |
| DB | `rusqlite` (bundled) |
| UI | `eframe` / `egui` |
| Erros | `thiserror`, `anyhow` (só na UI) |
| Serialização | `serde`, `serde_json` |

## Modelo de dados (SQLite)
```sql
CREATE TABLE receipts (
  id INTEGER PRIMARY KEY,
  data_compra TEXT,           -- ISO 8601 (YYYY-MM-DD)
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
CREATE INDEX idx_items_receipt ON items(receipt_id);
```

## Tipos Rust (crate `core`)
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub descricao: String,
    pub quantidade: f64,
    pub preco_unitario: f64,
    pub preco_total: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Grupo { Casa, Igor, Renan }

#[derive(Debug, Clone)]
pub struct ItemClassificado {
    pub item: Item,
    pub grupo: Grupo,
}

#[derive(Debug, Default, Serialize)]
pub struct Totais {
    pub casa: f64,
    pub igor: f64,
    pub renan: f64,
    pub geral: f64,
}
```

## Regras de negócio
- **Validação aritmética**: `|Σ(preco_total dos itens) − total_cupom| ≤ 0.05` → OK; senão marcar cupom como "revisar".
- **Cálculo**: cada item soma integralmente ao seu grupo. `geral = casa + igor + renan`.
- **Persistência**: ao salvar, itens sem grupo atribuído são rejeitados com erro claro.

## Prompt fixo do LLM (crate `core::llm`)
```
Você é um extrator de dados de cupons fiscais brasileiros.
Receba o texto OCR abaixo e retorne APENAS um JSON válido no formato:
{"itens":[{"descricao":"...","quantidade":1,"preco_unitario":0.0,"preco_total":0.0}]}
Regras:
- Ignore cabeçalhos, totais, impostos, formas de pagamento e rodapés.
- Valores em reais, use ponto como separador decimal.
- Se quantidade não estiver explícita, use 1.
- Se preco_unitario não estiver explícito, calcule como preco_total/quantidade.
- Não inclua texto fora do JSON.

Texto OCR:
<<<{ocr_text}>>>
```

## Fora de escopo (por agora)
- Sincronização em nuvem.
- Autenticação multiusuário.
- Suporte a cupons de outros países.
- OCR de PDFs multipágina.