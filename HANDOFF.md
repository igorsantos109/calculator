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

## Estado atual (após PARADA 2 / ETAPA 2)
- Checkout: /home/igor/ia-sh/calculador
- Branch: agent/web-llm (a partir de main c8ce9bf)
- HEAD local: dcdb499 (4 commits à frente de c8ce9bf)
- origin/main: c8ce9bf (igual a main local) — sem push, sem pull
- Working tree limpo (salvo DIVERGENCIAS.md não rastreado, pré-existente)
- Crates no workspace: receipt_core (core), db. crates/ui retirado do workspace
  (commit separado, reversível — restaurar members no Cargo.toml reintegra)

## Commits locais (agent/web-llm)
1. 916e106 — ETAPA1: corrigir D1-D7 (validar_total em centavos, parsing centralizado,
   FK, backup, grupo)
2. a968874 — ETAPA1: retirar crates/ui do workspace (commit separado, reversível)
3. c8bb5be — PARADA 1: atualizar HANDOFF.md, AGENT_LOG.md e RELATORIO_ENTREGA.md
4. dcdb499 — ETAPA 2: reescrever llm.rs para llama-server (API OpenAI)
   - POST {llm_base_url}/chat/completions com mensagem system fixa
   - Texto OCR como DADO delimitado (ignorar instruções no cupom)
   - response_format json_schema com schema {itens:[{descricao,quantidade,preco_unitario,preco_total}]}
   - temperature 0.1, max_tokens configurável, header Authorization Bearer se houver chave
   - Config: llm_base_url, llm_model, llm_api_key_env (nome da variavel, nunca o valor),
     timeout, max_tentativas, max_tokens
   - Erros tipados: ServidorInacessível, Timeout, ChaveInvalida (401/403),
     ServidorIndisponível (5xx), RespostaInvalida, RespostaTruncada (length),
     JsonInvalido, ItemInvalido
   - Retry apenas em erros transitórios (conexão, timeout, 500/502/503/504)
   - Validação de domínio independente: finitos, qtd>0, preços>=0, descrição não vazia
   - Item inválido vira aviso (nunca descartado silenciosamente), soma dos itens reportada
   - Nunca logar texto integral do cupom nem a chave
   - Testes com servidor HTTP falso (wiremock): válido, cercas ```json, JSON quebrado,
     401, 500, truncado, itens inválidos → avisos, injeção de prompt no OCR,
     servidor inacessível

## Plano e progresso
| ID  | Tarefa                                                | Estado         |
|-----|-------------------------------------------------------|----------------|
| ETAPA 0 | Reconciliação + baseline (fmt/clippy/test/build)      | CONCLUÍDA      |
| ETAPA 1 | Depuração profunda D1-D7 + remover ui do workspace    | CONCLUÍDA      |
| PARADA 1 | Relatório + saída bruta + commits locais              | CONCLUÍDA      |
| ETAPA 2 | Cliente LLM para llama-server (API OpenAI) + testes   | CONCLUÍDA      |
| PARADA 2 | Relatório + saída bruta                               | AGUARDANDO QA/Igor|
| ETAPA 3 | Novo crate crates/web (servidor HTTP, API, testes)    | PENDENTE       |
| ETAPA 4 | Página interativa (HTML/CSS/JS embutidos)             | PENDENTE       |
| ETAPA 5 | Pacote + ROTEIRO_TESTE.md para Ubuntu 192.168.1.184   | PENDENTE       |

## Totais de testes (após ETAPA 2)
- receipt_core: 34 aprovados, 0 falhos, 1 ignorado (OCR fixture)
- db: 22 aprovados, 0 falhos, 0 ignorados
- doc-tests: 0

## Ambiente de build
- cargo 1.99.1, rustc 1.99.1
- glibc: 2.41 (Debian 2.41-12+deb13u4) — importante para compatibilidade no Ubuntu alvo
- Offline: --offline --locked funcionando

## Decisões pendentes de Igor
- D10: diferença de conciliação ≠ 0 bloqueia o cupom ou vai para alguém? Até decidir, bloquear na camada web e documentar em DIVERGENCIAS.md (não alterar core).
- Versão do Ubuntu alvo (para comparar glibc do build com o destino).
- Chave do llama-server (se houver) — nunca gravar em versão.
- Firewall/rede para 192.168.1.184.

## Arquivos alterados (ETAPA 2)
- Modificados: Cargo.toml (raiz), Cargo.lock, crates/core/Cargo.toml,
  crates/core/src/lib.rs (declaração pub mod llm), crates/core/src/llm.rs (reescrito)

## Operações em andamento
- Nenhuma (PARADA 2)

## Última atualização
2026-11-16 — ETAPA 2 concluída. HEAD em dcdb499. Aguardando liberação para ETAPA 3.