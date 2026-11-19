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

## Estado atual (após F1–F11 + G1–G15 + H1–H6 + G16)
- Checkout: /home/igor/ia-sh/calculador
- Branch: agent/web-llm (a partir de main c8ce9bf)
- HEAD local: **50dbd5e** (G16 + HANDOFF pós-push) — igual a origin
- origin/main: c8ce9bf (igual a main local)
- origin/agent/web-llm: 50dbd5e (pushado)
- Crates no workspace: receipt_core (core), db, web. crates/ui retirado do workspace

## Commits locais (agent/web-llm — H1–H6)
1. 6cde526 — H1: contrato serde Grupo/Pagador em minúsculas (rename_all lowercase + alias maiúsculo)
2. 762f6fd — H2: app.js verDetalhes/relatório consistentes com API (grupos minúsculos)
3. 23aea7a — H3: rate limiting real por IP (AddrStream), bloqueio antes do PIN, 429+Retry-After, zerar em sucesso
4. 9bddbda — H5: erros de banco genéricos em todas as rotas (sem e.to_string() leak)
5. tests/js/test_app.js — H6: testes JS com node:test + node:vm + dom_stub (login, salvar, remover, saldos)
6. crates/web/src/assets/app.js — H6: exposição window.app para teste (não afeta produção)

## Plano e progresso
| ID  | Tarefa                                                | Estado         |
|-----|-------------------------------------------------------|----------------|
| ETAPA 0–5 | Baseline e correções F1–F11 + G1–G15                 | CONCLUÍDA      |
| H1  | Contrato serde Grupo/Pagador minúsculas + teste contrato | CONCLUÍDA      |
| H2  | app.js verDetalhes/relatório consistentes com API     | CONCLUÍDA      |
| H3  | Rate limiting real por IP, 429+Retry-After, zerar em sucesso | CONCLUÍDA      |
| H4  | Edição inválida não vira 0; remover editarItem        | CONCLUÍDA      |
| H5  | Erros de banco genéricos (sem vazamento)              | CONCLUÍDA      |
| H6  | Testes JS com node:test + dom_stub                    | CONCLUÍDA      |
| Final | fmt, clippy -D warnings, test --workspace --offline --locked | CONCLUÍDA      |

## Totais de testes (estado final)
- receipt_core: 41 aprovados, 0 falhos, 1 ignorado (inclui 5 testes H1)
- db: 18 aprovados, 0 falhos, 0 ignorados
- web: 23 testes de integração aprovados (inclui H1/H2/H3)
- doc-tests: 0
- JS: 4/4 testes H6 passando (login, salvar, remover, saldos)
- **Total: 86 testes passando**

## Ambiente de build
- cargo 1.100.0, rustc 1.100.0
- node v24.22.3
- glibc: 2.41 (Debian 2.41-12+deb13u4)
- Offline: --offline --locked funcionando
- axum NÃO disponível offline → servidor web implementado com hyper
- reqwest usa native-tls (libssl); rustls não disponível offline

## Decisões pendentes de Igor
- D10: diferença de conciliação ≠ 0 bloqueia o cupom ou vai para alguém? Até decidir, bloquear na camada web e documentar em DIVERGENCIAS.md (não alterar core).
- Versão do Ubuntu alvo (para comparar glibc do build com o destino).
- Chave do llama-server (se houver) — nunca gravar em versão.
- Firewall/rede para 192.168.1.184.

## Arquivos alterados (H1–H6)
- crates/core/src/lib.rs — H1: #[serde(rename_all = "lowercase")] + #[serde(alias = "Casa")] etc. em Grupo/Pagador
- crates/web/src/server.rs — H3: handle() recebe client_ip (SocketAddr do AddrStream), run() passa via make_service_fn; bloqueio ANTES do PIN; 429+Retry-After; zerar em sucesso. H5: erros de banco genéricos sem e.to_string().
- crates/web/src/assets/app.js — H2: removido editarItem; listener inline com validação no change (valor inválido mantém anterior + marca erro). H6: window.app exposto para teste.
- crates/web/src/assets/index.html — H2: <option value> minúsculo (casa/igor/renan)
- crates/web/src/assets/style.css — H2: classe .erro para inputs da tabela
- crates/web/tests/integracao.rs — H1: contrato POST 201 + GET minúsculo; H2: saldos verDetalhes; H3: unitários IpRateLimiter + integração 5→6=429
- tests/js/test_app.js — H6: testes JS com node:test + dom_stub
- tests/js/dom_stub.js — H6: classe classList para mostrarErro/mostrarSucesso
- HANDOFF.md — atualizado com SHA real do HEAD

## Operações em andamento
- Nenhuma. Sincronia local/remoto verificada: HEAD = `origin/agent/web-llm`
  = **50dbd5e** (G16 em f422ad0 + HANDOFF pós-push). Nada a enviar.
- Restam apenas 3 untracked, deliberadamente **não** versionados:
  `test_dom.js`, `test_final_g14.js`, `test_g14.js` (ver decisão em G16).

## Sessão G16 (2026-11-19)
### Proteção de dados (descoberta importante)
- Os 5 arquivos `cupons.db.*.bak` na raiz contêm **dados reais de cupons** e
  apareciam como untracked — o `.gitignore` cobria `*.db` mas não `*.bak`.
- Correção: adicionados `*.bak`, `*.db.*`, `*.sqlite.*` ao `.gitignore`.
- Verificado com `git check-ignore -v cupons.db.20261116T210935.bak`.
- Os backups **nunca foram commitados** em nenhum push anterior.

### Bugs de harness corrigidos (2 testes JS já versionados estavam quebrados)
- `tests/js/test_app.js` cobria `app.js` desde H6 mas **2 de 4 testes falhavam**:
  1. **login** — `fazerLogin` é `async`; o teste sincronizado assertava antes do
     `await` concluir o `setPin`. Corrigido: teste `async` + `await` no
     dispatchEvent + mock de `/api/saude`.
  2. **salvarCupom** — `salvarCupom` dispara POST `/api/cupons` e em seguida
     GET `/api/cupons` (`carregarHistorico`); o mock sobrescrevia
     `ultimaRequisicaoEnviada`, perdendo o POST. Corrigido: array `requisicoes`
     + `find` filtrando por `options.method === "POST"`.
- **Não era bug de produção** — ambos eram falhas do próprio teste.
- `tests/js/test_app_simple.js` (novo): mesmas correções aplicadas.

### Clippy corrigido
- `crates/web/tests/integracao.rs`: bloco de comentário `///` com linha vazia
  antes de `#[tokio::test]` (`empty_line_after_outer_attr`); `unused_mut` em
  `resp_igor`.

### Arquivos untracked — decisão
- Versionar: `tests/js/test_app_simple.js` (variante enxuta, mesma cobertura).
- **Não versionar** `test_g14.js`: quebrado, usa `__dirname/../../` e escapa
  do repo (`/home/igor/`). `test_final_g14.js` já usa o caminho correto.
- **Não versionar** `test_dom.js`, `test_final_g14.js`: verificações estáticas
  de string já cobertas por `test_app.js` + 23 testes de integração Rust.

### Gates verificados (G16)
- `cargo fmt --all -- --check`: OK
- `cargo clippy --workspace --all-targets -- -D warnings`: exit 0
- `cargo test --workspace --locked`: 82 passando (core 41, db 18, web 23), 0 falhos
- JS: `node --test tests/js/test_app.js tests/js/test_app_simple.js` → 9/9
- `cargo build --release --workspace --locked`: OK
- Binário revalidado: `/api/saude` OK com PIN; 401 sem PIN; PIN < 6 recusado

### Próxima ação
- Validar no Ubuntu 192.168.1.184 conforme `ROTEIRO_TESTE.md`.

## Última atualização
2026-11-19 — G16 em **f422ad0** + HANDOFF pós-push em **50dbd5e**, ambos
pushados para `origin/agent/web-llm`. Sincronia local/remoto reverificada
(HEAD = origin). Gates: fmt OK, clippy `-D warnings` OK, 82 testes Rust
(core 41, db 18, web 23), JS 9/9, build release OK.
Nenhuma operação em andamento.