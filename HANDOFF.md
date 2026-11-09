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
- Último commit local: 1e873dc "ci: add GitHub Actions workflow"
- 13 arquivos rastreados (workspace, core, db, ui, CI, SPEC, README)
- 2 arquivos NÃO rastreados: crates/core/src/ocr.rs, crates/core/src/llm.rs
- HANDOFF.md: criado nesta fase
- PR #1 pré-existente: estado não conferido, não interagir

## Plano e progresso
| ID | Tarefa | Dependências | Risco | Estado |
|----|--------|-------------|-------|--------|
| T01-F1 | Auditoria leitura inicial | — | Baixo | CONCLUÍDA |
| T01-F2 | Auditoria OCR/LLM locais | T01-F1 | Baixo | CONCLUÍDA (2 tentativas) |
| T01-F3 | Dependências e integração | T01-F2 | Baixo | CONCLUÍDA |
| T01-F4 | Documentação HANDOFF | T01-F3 | Baixo | EM ANDAMENTO |
| T02 | Auditoria integração + CI baseline | T01-F4 | Médio | PENDENTE |
| T03 | Baseline offline (cargo fmt/clippy/test/build) | T02 | Médio | PENDENTE |
| T04 | Aprovar contratos financeiros/rascunho | T03 | Alto | PENDENTE |
| T05 | Correções domínio/integração | T04 | Alto | PENDENTE |
| T06 | Persistência e migrações seguras | T05 | Alto | PENDENTE |
| T07 | Fluxo manual + relatório | T06 | Alto | PENDENTE |
| T08 | PoC OCR (alvio-ocr) | T07 | Médio | PENDENTE |
| T09 | LLM local opcional (Ollama) | T08 | Médio | PENDENTE |
| T10 | Integração, privacidade, testes, empacotamento | T09 | Alto | PENDENTE |

## Ações executadas
- T01-F1: pwd, git status, git log, git ls-files, check HANDOFF.md ausente
- T01-F2: leitura ocr.rs:74-110, llm.rs:160-190, lib.rs:1-20
- T01-F3: leitura Cargo.toml (raiz), core/Cargo.toml, lib.rs completo (204 linhas)
- T01-F4: criação deste HANDOFF.md

## Arquivos alterados
- **Criado**: HANDOFF.md (este arquivo)
- **Não modificados**: ocr.rs, llm.rs (permanecem não rastreados)

## Evidências
### Recebidas de fases anteriores (não verificadas pessoalmente)
- Workspace Cargo com 3 crates (core, db, ui)
- SPEC.md define objetivo, fluxo, stack, modelo SQL, tipos, regras, prompt LLM
- CI workflow com fmt, clippy, test, build
- 7 testes unitários em core (4) e db (3) — execução não verificada
- Build release não executado

### Conferido pessoalmente nesta fase
- Git status: apenas ocr.rs e llm.rs como untracked
- core/Cargo.toml: serde/serde_json/thiserror herdados; image, imageproc, alvio-ocr, ollama-rest, regex, tracing declarados
- **reqwest AUSENTE** do manifesto (usado em llm.rs)
- **wiremock AUSENTE** do manifesto (usado em testes de llm.rs)
- lib.rs (204 linhas): sem `pub mod ocr;` nem `pub mod llm;`
- ocr.rs:74-110 — `cut_rate` e `threads` usados sem definição no trecho
- llm.rs:160-190 — `format!` com 1 placeholder e 2 argumentos (defeito estático)

### Suspeitas / Não verificadas
- Integração completa de OCR/LLM no workspace
- Disponibilidade/resolução de alvio-ocr v0.1, ollama-rest v0.3
- API compatível das dependências declaradas
- Download automático de modelos ONNX pelo alvio-ocr
- Funcionamento real de OCR, LLM, DB, UI
- Compilação do workspace (arquivos não rastreados não participam)
- Sincronização com origin/main
- Estado do PR #1

## Operações em andamento
- Nenhuma (fase de documentação concluída)

## Decisões e justificativas
- Priorizar fluxo manual antes de OCR/LLM: reduz risco e permite uso imediato
- Centavos inteiros: evita problemas de ponto flutuante em dinheiro
- Rascunho separado: permite recuperação de falhas sem perda
- OCR/LLM opcionais: privacidade e funcionamento offline
- thiserror no core: erros tipados sem alocação; anyhow só na borda (UI)

## Bloqueios e autorizações
| Bloqueio | Autorização necessária |
|----------|------------------------|
| reqwest/wiremock ausentes no core/Cargo.toml | Adicionar deps |
| ocr.rs e llm.rs não integrados ao crate | Declarar módulos em lib.rs |
| Defeitos estáticos (cut_rate, format!) | Corrigir código |
| Contratos financeiros (50/50, pagador, centavo) | Aprovação de Igor |
| Build/teste baseline | Autorização para cargo |

## Próxima ação
T02 — Auditoria mínima de integração e CI:
1. Ler lib.rs completo para confirmar ausência de módulos
2. Ler .github/workflows/ci.yml para confirmar steps
3. Propor correções mínimas (deps, mod declarations, fix format!) com escopo, risco e reversão
4. Aguardar autorização para T03 (baseline offline)

## Reversão
Esta fase apenas criou HANDOFF.md. Sua remoção (`rm HANDOFF.md`) reverteria a fase, mas não deve ser executada.

## Última atualização
2025-11-09 — Fase T01-F4 concluída, aguardando revisão.