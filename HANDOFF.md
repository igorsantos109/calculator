# Handoff — Leitor de Cupons Fiscais

## Objetivo
Aplicativo local para registrar cupons fiscais brasileiros, revisar itens, classificá-los em Casa/Igor/Renan, calcular totais e persistir em SQLite.
Interface: SOMENTE página web (servida pelo próprio programa). egui e rfd removidos do projeto.
Backend LLM: llama-server (llama.cpp) em outro PC da LAN, via POST {llm_base_url}/chat/completions (formato OpenAI).

## Critérios de conclusão
- Fluxo manual completo funcional sem OCR/LLM
- Valores monetários em centavos inteiros
- Separação rascunho/cupom confirmado
- Persistência SQLite com schema versionado
- OCR e LLM opcionais e isolados
- CI (fmt, clippy, test, build) passando com --locked
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
- Nunca usar ~/cupons.db; testes usam banco temporário
- Não acessar pc-sala, serviços, gestor "ia", GPU, firewall, configs de inferência
- llm_base_url do pc-sala é só um endereço configurável; nos testes usar servidor HTTP falso

## Estado atual (após PARADA 3 / ETAPA 3)
- Checkout: /home/igor/ia-sh/calculador
- Branch: agent/web-llm (a partir de main c8ce9bf)
- HEAD local: ae260dc (5 commits à frente de c8ce9bf)
- origin/main: c8ce9bf (igual a main local) — sem push, sem pull
- Working tree limpo (salvo DIVERGENCIAS.md não rastreado, pré-existente)
- Crates no workspace: receipt_core (core), db, web. crates/ui retirado do workspace
  (commit separado, reversível — restaurar members no Cargo.toml reintegra)

## Commits locais (agent/web-llm)
1. 916e106 — ETAPA1: corrigir D1-D7 (validar_total em centavos, parsing centralizado,
   FK, backup, grupo)
2. a968874 — ETAPA1: retirar crates/ui do workspace (commit separado, reversível)
3. c8bb5be — PARADA 1: atualizar documentos
4. dcdb499 — ETAPA 2: reescrever llm.rs para llama-server (API OpenAI)
5. 4cef37a — PARADA 2: atualizar documentos após ETAPA 2
6. ae260dc — ETAPA 3: criar crate crates/web (servidor HTTP, API, testes de integração)

## Plano e progresso
| ID  | Tarefa                                                | Estado         |
|-----|-------------------------------------------------------|----------------|
| ETAPA 0 | Reconciliação + baseline (fmt/clippy/test/build)      | CONCLUÍDA      |
| ETAPA 1 | Depuração profunda D1-D7 + remover ui do workspace    | CONCLUÍDA      |
| PARADA 1 | Relatório + saída bruta + commits locais              | CONCLUÍDA      |
| ETAPA 2 | Cliente LLM para llama-server (API OpenAI) + testes   | CONCLUÍDA      |
| PARADA 2 | Relatório + saída bruta                               | CONCLUÍDA      |
| ETAPA 3 | Servidor web + API + testes de integração             | CONCLUÍDA      |
| PARADA 3 | Relatório + saída bruta                               | AGUARDANDO Igor|
| ETAPA 4 | Página interativa (HTML/CSS/JS embutidos)             | PENDENTE       |
| ETAPA 5 | Pacote + ROTEIRO_TESTE.md para Ubuntu 192.168.1.184   | PENDENTE       |

## Totais de testes (após ETAPA 3)
- receipt_core: 34 aprovados, 0 falhos, 1 ignorado (OCR fixture)
- db: 18 aprovados, 0 falhos, 0 ignorados
- web: 16 testes de integração aprovados (servidor falso + banco temporário)
- doc-tests: 0

## Ambiente de build
- cargo 1.99.1, rustc 1.99.1
- glibc: 2.41 (Debian 2.41-12+deb13u4) — importante para compatibilidade no Ubuntu alvo
- Offline: --offline --locked funcionando
- axum NÃO disponível offline → servidor web implementado com hyper (já no índice)

## Decisões pendentes de Igor
- D10: diferença de conciliação ≠ 0 bloqueia o cupom ou vai para alguém? Até decidir, bloquear na camada web e documentar em DIVERGENCIAS.md (não alterar core).
- Versão do Ubuntu alvo (para comparar glibc do build com o destino).
- Chave do llama-server (se houver) — nunca gravar em versão.
- Firewall/rede para 192.168.1.184.

## Arquivos alterados (ETAPA 3)
- Criado: crates/web/ (Cargo.toml, config.rs, lib.rs, main.rs, security.rs, server.rs, tests/integracao.rs)
- Modificados: Cargo.toml (raiz), Cargo.lock, crates/db/src/lib.rs (id com serde(default))

## Operações em andamento
- Nenhuma (PARADA 3)

## Última atualização
2026-11-16 — ETAPA 3 concluída. HEAD em ae260dc. Aguardando liberação para ETAPA 4.