# Backlog Sequencial — Leitor de Cupons Fiscais

Este backlog organiza a execução em etapas sequenciais (um PR por etapa, com revisão e merge antes da próxima).

## Etapa 1 — Auditoria e especificação (este PR)
- **Escopo:** inventariar estado atual, revisar documentação e separar estado atual de comportamento planejado.
- **Dependências:** nenhuma.
- **Critérios de aceite verificáveis:**
  - `SPEC.md`, `README.md` e `.github/copilot-instructions.md` atualizados.
  - decisões pendentes claramente marcadas como propostas.
  - comandos de validação executados registrados sem declarar sucesso indevido.

## Etapa 2 — Dinheiro e regras financeiras (após aprovação humana)
- **Escopo:** desenhar e implementar modelo monetário final em centavos (`i64`/`INTEGER`), regras de arredondamento e conciliação.
- **Dependências:** aprovação humana das decisões financeiras pendentes.
- **Critérios de aceite verificáveis:**
  - operações monetárias sem `f64` no caminho crítico final.
  - tolerância de conciliação de 5 centavos inteiros.
  - testes cobrindo overflow, arredondamento e divergências de linha.

## Etapa 3 — SQLite, migrações e rascunhos
- **Escopo:** separar rascunho e cupom confirmado, adicionar migrações e preparar persistência transacional com `foreign_keys` por conexão.
- **Dependências:** etapa 2 concluída.
- **Critérios de aceite verificáveis:**
  - migração segura definida e aplicada em ambiente de teste.
  - rascunho permite pendências; confirmação exige validações.
  - testes de persistência cobrindo criação, leitura, atualização e exclusão (se exclusão existir).

## Etapa 4 — Fluxo manual completo
- **Escopo:** finalizar fluxo sem OCR/LLM: entrada e edição de itens, classificação, confirmação, histórico e relatório.
- **Dependências:** etapas 2 e 3 concluídas.
- **Critérios de aceite verificáveis:**
  - usuário conclui todo fluxo manual sem OCR e sem Ollama.
  - relatório diferencia responsabilidade individual e saldo.
  - mensagens de UI/erro em pt-BR.

## Etapa 5 — Prova de conceito e OCR
- **Escopo:** PoC para OCR com validação técnica (dependência, licença, build, português, offline pós-instalação, tempo e memória).
- **Dependências:** etapa 4 concluída.
- **Critérios de aceite verificáveis:**
  - relatório técnico comparando opções/abordagens.
  - comparação imagem original vs pré-processada antes de fixar binarização.
  - decisão documentada sem substituição silenciosa de dependência.

## Etapa 6 — Parser determinístico e LLM opcional
- **Escopo:** parser determinístico para formatos conhecidos e fallback opcional para LLM local.
- **Dependências:** etapa 5 concluída.
- **Critérios de aceite verificáveis:**
  - pipeline respeita: OCR -> parser -> LLM opcional -> revisão.
  - Ollama com timeout, erros tipados e resposta estruturada validada.
  - campos desconhecidos permanecem nulos.

## Etapa 7 — Integração e recuperação de falhas
- **Escopo:** serviço de processamento fora da thread de UI, progresso real por etapa e recuperação sem perda de revisão.
- **Dependências:** etapa 6 concluída.
- **Critérios de aceite verificáveis:**
  - pipeline executa sem bloquear UI.
  - falhas preservam estado revisável.
  - logs não incluem conteúdo integral de cupons por padrão.

## Etapa 8 — Empacotamento, privacidade e operação offline
- **Escopo:** empacotamento local, diretrizes operacionais offline e checklist de privacidade.
- **Dependências:** etapa 7 concluída.
- **Critérios de aceite verificáveis:**
  - execução offline documentada de ponta a ponta.
  - checklist de privacidade aplicado.
  - instruções de distribuição local validadas.
