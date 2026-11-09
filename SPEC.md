# Leitor de Cupons Fiscais — Especificação Revisada

## 1) Objetivo
Aplicativo desktop local em Rust para registrar cupons fiscais, revisar itens manualmente, classificar gastos por grupo e calcular totais.

## 2) Estado atual auditado (repositório neste PR)

### 2.1 Stack e arquitetura já presentes
- Workspace Rust edition 2021 com crates `core`, `db` e `ui`.
- UI desktop com `eframe`/`egui`.
- Persistência SQLite via `rusqlite`.
- Dependências de OCR (`alvio-ocr`) e LLM local (`ollama-rest`) declaradas no `core`.
- CI com `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test` e `cargo build --release`.

### 2.2 Comportamentos já implementados
- Cálculo de totais por grupo e validação de conciliação com tolerância de R$ 0,05.
- Esquema inicial de banco com tabelas `receipts` e `items`.
- UI básica para selecionar imagem, visualizar/editar grupos e exibir totais.

### 2.3 Lacunas relevantes encontradas
- Fluxo manual completo ainda não está fechado (botão salvar ainda placeholder na UI).
- OCR/LLM não estão integrados de ponta a ponta no fluxo de uso.
- Modelo financeiro atual usa `f64`/`REAL`, ainda sem migração para centavos `i64`/`INTEGER`.
- Separação formal entre rascunho e cupom confirmado ainda não existe.

## 3) Princípios técnicos (mantidos)
1. Rust edition 2021.
2. UI com `egui/eframe`.
3. SQLite local.
4. Runtime 100% local, sem APIs externas em execução.
5. Ollama somente em loopback local.
6. Sem Python/Node em código de produção.
7. `thiserror` no domínio e `anyhow` apenas em UI/binário.

## 4) Comportamento planejado (proposto, pendente validação humana)

### 4.1 Prioridade de entrega
- MVP manual completo primeiro: entrada/edição de itens, classificação, persistência, histórico e relatório funcionando sem OCR e sem Ollama.
- IA (OCR/LLM) como assistência opcional posterior.

### 4.2 Dinheiro e conciliação
- Planejar valores monetários finais como centavos (`i64` no Rust e `INTEGER` no SQLite).
- Operações monetárias com verificação de overflow.
- Quantidade e preço unitário com precisão decimal suficiente para itens por peso.
- Definir política explícita de arredondamento.
- Definir migração segura dos dados existentes (sem migrar neste PR).
- Tolerância de conciliação planejada: 5 centavos inteiros.
- Preservar o total impresso por linha e sinalizar divergências sem correção silenciosa.

### 4.3 Estados do cupom
- Rascunho: grupo opcional, permite pendências.
- Confirmado: grupo obrigatório e validações completas (campos, grupos e conciliação).

### 4.4 Regras propostas (não aprovadas ainda)
- Casa dividida 50/50 entre Igor e Renan.
- Um pagador por cupom no MVP.
- Relatório com responsabilidade individual e saldo a acertar.
- Centavo indivisível atribuído deterministicamente a Igor.

### 4.5 Extração e revisão manual
- Extração deve incluir: `estabelecimento`, `data_compra`, `total_cupom` e `itens`.
- Campos desconhecidos ficam nulos e devem ser revisados manualmente.
- Não inventar dados ausentes.
- Tratamento explícito para descontos, acréscimos e valores zero.

### 4.6 Pipeline técnico planejado
`imagem opcional -> OCR -> parser determinístico limitado a formatos conhecidos -> LLM local quando necessário/solicitado -> revisão -> persistência`.

No uso de Ollama (planejado):
- `format` com JSON Schema.
- timeout explícito.
- erros tipados.
- validação de domínio independente da resposta estruturada.
- modelo configurável.
- sem logar conteúdo integral de cupons por padrão.

### 4.7 OCR como prova de conceito antes de adoção definitiva
- Não assumir `alvio-ocr`, versão atual ou PP-OCRv6 como decisão final.
- Planejar PoC para validar: dependência, licença, build no sistema-alvo, português, operação offline após instalação, tempo e memória.
- Comparar imagem original vs pré-processada antes de tornar binarização obrigatória.
- Não substituir dependência silenciosamente.

### 4.8 Serviço de processamento e persistência
- Planejar serviço de processamento reutilizável fora da thread de UI.
- Exibir etapas reais de progresso.
- Recuperar falhas sem perder revisão manual.
- Persistência com migrações, transações e `foreign_keys` ativadas por conexão.
- Incluir contrato de exclusão caso haja testes de exclusão.
- Formatação monetária pt-BR.

## 5) Prompt planejado para extração (sem contexto de commits/status)
```
Você é um extrator de dados de cupons fiscais brasileiros.
Receba o texto OCR e retorne APENAS JSON válido.

Formato esperado:
{
  "estabelecimento": "string | null",
  "data_compra": "YYYY-MM-DD | null",
  "total_cupom": "number | null",
  "itens": [
    {
      "descricao": "string",
      "quantidade": "number | null",
      "preco_unitario": "number | null",
      "preco_total": "number | null"
    }
  ]
}

Regras:
- Não invente dados ausentes; use null quando não houver evidência.
- Pode retornar valores zero quando presentes no cupom.
- Preserve descontos e acréscimos quando identificáveis.
- Não inclua texto fora do JSON.
```

## 6) Fora de escopo deste PR
- Implementar mudanças de domínio, banco, UI, OCR ou LLM.
- Rodar migração de dados.
- Criar issues automaticamente.
