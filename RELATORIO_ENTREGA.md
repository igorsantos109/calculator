# Relatório de Entrega — Leitor de Cupons Fiscais

## Contexto
Projeto: leitor de cupons fiscais (workspace Rust: core, db, ui → ui retirado).
Checkout: /home/igor/ia-sh/calculador
Remoto publicado: igorsantos109/calculator, main = c8ce9bf.
Branch local: agent/web-llm (publicada em origin/agent/web-llm).

## SHA inicial / final
- SHA inicial (main): c8ce9bf
- SHA final (agent/web-llm): **c21a008** (16 commits à frente de c8ce9bf)

## Estado por ID (tabela de entrega final)

| ID   | Estado        | Commit    | Prova                                                        |
|------|---------------|-----------|--------------------------------------------------------------|
| D1   | CONCLUÍDO     | 916e106   | testes 20.05 vs 20.00 ok; 100.05 vs 100.00 ok; 0.06 rejeita |
| D2   | CONCLUÍDO     | 916e106   | módulo core::parse, 9 testes passando                        |
| D3   | CONCLUÍDO     | 916e106   | testes órfão/cascade/FK passando (3 testes)                  |
| D4   | CONCLUÍDO     | 916e106   | testes backup (arquivo existente/novo) passando (2 testes)   |
| D5   | CONCLUÍDO     | 916e106   | parse_grupo erro; ORDER BY id; histórico id DESC (3 testes)  |
| D6   | CONCLUÍDO     | 916e106   | SchemaVersionAusenteBancoLegado removido; ordem de módulos   |
| D7   | CONCLUÍDO     | 916e106   | varredura: sem conversão silenciosa de quantidade            |
| D8   | CONCLUÍDO     | f42ff40   | CI --locked OK (fmt, clippy, test, build)                    |
| ui   | REMOVIDO      | a968874   | crates/ui retirado do workspace (reversível)                 |
| ETAPA 2 | CONCLUÍDO   | dcdb499   | llm.rs reescrito para llama-server; 14 testes (incl. F8)     |
| ETAPA 3 | CONCLUÍDO   | ae260dc   | crate crates/web (servidor HTTP); 16 testes de integração    |
| ETAPA 4 | CONCLUÍDO   | a803185   | página interativa (HTML/CSS/JS embutidos)                    |
| ETAPA 5 | CONCLUÍDO   | 8adab48   | deploy/config, deploy/systemd, deploy/scripts, ROTEIRO_TESTE.md |
| F1–F11 | CONCLUÍDO   | d970d9e..1aac839 | Correções críticas pós-ETAPA 5 (ver abaixo)            |
| G1–G7 | CONCLUÍDO   | 652e9a1..c21a008 | Correções de regressão/críticas pós-F1-F11 (ver abaixo) |

## Correções F1–F11 aplicadas (pós-ETAPA 5)

| Fix | Descrição | Status |
|-----|-----------|--------|
| F1 | Autenticação e CSRF: removido CSRF (auth por Bearer header), PIN obrigatório ≥6 chars, auth em TODAS as rotas /api (incl. /api/saude), rate limiting simples por IP | ✅ |
| F2 | app.js: todos elementos DOM definidos; setupEventos sem abortar; addEventListener em vez de onchange/onclick inline; escape correto (uma vez); sem innerHTML com dados não escapados | ✅ |
| F3 | JS sintaxe verificada com Node (--check OK); teste execução real pendente (requer jsdom não disponível offline) | ✅/⚠️ |
| F4 | GET /api/cupons/{id} devolve itens via buscar_itens_por_receipt_id | ✅ |
| F5 | /api/saude exige PIN; timeout ≤3s na chamada ao LLM; não segura lock do banco durante rede | ✅ |
| F6 | Relatório filtra antes de somar; totais por grupo e saldos do período | ✅ |
| F7 | Limite de corpo 10 MB (constante); OPTIONS CORS * removido; validação na API; não vaza SQL | ✅ |
| F8 | llm.rs: corte string por char (sem panic); delimitador à prova de ">>>"; teste injeção real; retry 5xx testado com max_tentativas>1; testes vazios removidos/afirmados | ✅ |
| F9 | reqwest mantido com default-features (native-tls/libssl); rustls não disponível offline; documentada dependência libssl | ✅ |
| F10 | Contagem de testes reconciliada: db 18, core 36 (era 34+1=35, +12 llm - testes removidos = 36), web 16 | ✅ |
| F11 | RELATORIO_ENTREGA.md, HANDOFF.md atualizados; DIVERGENCIAS.md versionado | ✅ |

## Correções G1–G7 aplicadas (pós-F1-F11, regressões identificadas)

| Fix | Descrição | Commit | Status |
|-----|-----------|--------|--------|
| G1 | server.rs: rota_publica inclui /app.js e /style.css (assets sem PIN) | 652e9a1 | ✅ |
| G2 | app.js: tela de login PIN, sessionStorage, Authorization em todas chamadas | 9b5ff2f | ✅ |
| G3 | app.js: data-campo=grupo no select, handler aceita grupo | d373b26 | ✅ |
| G4 | app.js: ouvintes em setupEventos (uma vez), não em renderItens | 90b96ff | ✅ |
| G5 | server.rs: limite corpo real (413), validação campos, sem SQL vazado | 385cadd | ✅ |
| G6 | server.rs: rate limiting por IP (5 tentativas/5min → 429) | c21a008 | ✅ |
| G7 | Script Node com DOM simulado (stub) documentado; jsdom pendente | TESTE_JS_DOM.md | ✅/⚠️ |

## Testes por pacote (estado final)
- receipt_core: 36 aprovados, 0 falhos, 1 ignorado (OCR fixture)
- db: 18 aprovados, 0 falhos, 0 ignorados
- web: 16 testes de integração aprovados
- doc-tests: 0
- **Total: 70 testes passando**

## Dependências (offline, --locked)
- receipt_core: serde, serde_json, thiserror, image, imageproc, alvio-ocr, tracing, reqwest (0.12, json), tokio (1, full)
- dev: wiremock (0.6)
- db: receipt_core, rusqlite (bundled), thiserror, serde, serde_json
- dev: tempfile (3)
- web: receipt_core, db, serde, serde_json, thiserror, tokio, hyper (0.14, full), tower-http, reqwest
- dev: tempfile, wiremock
- ui removido (eframe, egui, rfd, dirs, etc. retirados do lock)

## Decisões pendentes de Igor
- D10: diferença de conciliação ≠ 0 bloqueia o cupom ou vai para alguém? Até decidir, bloquear na camada web e documentar em DIVERGENCIAS.md.
- Versão do Ubuntu alvo (para comparar glibc do build: 2.41).
- Chave do llama-server (se houver) — nunca gravar em versão.
- Firewall/rede para 192.168.1.184.

## Limitações (NÃO VERIFICADO)
- OCR real (alvio-ocr sem fixture, teste ignorado)
- LLM real / servidor llama-server real (testes usam servidor HTTP falso)
- Navegador real (interface não renderizada em navegador real)
- Rede real / intranet (testes em localhost)
- Banco real (~cupons.db nunca usado; testes usam banco temporário)

## Como rodar (estado final)
```bash
cd /home/igor/ia-sh/calculador
export PATH="$HOME/.cargo/bin:$PATH"
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --offline --locked -- -D warnings
cargo test --workspace --offline --locked
cargo build --workspace --release --offline --locked
```

## Artefatos de entrega (ETAPA 5)
- deploy/config/config.toml — exemplo de configuração (db, escuta, PIN via env, llm_base_url, llm_model, pasta OCR)
- deploy/systemd/leitor-cupons.service — unidade systemd de exemplo (usuário, não instalada)
- deploy/scripts/verificar_ubuntu.sh — verificação somente leitura (glibc, porta, OCR, /health, /api/saude, banco)
- ROTEIRO_TESTE.md — passos, resultado esperado, coleta de logs sem expor cupom, backup/restaura

## Próxima ação
Validar no Ubuntu 192.168.1.184 com `deploy/scripts/verificar_ubuntu.sh`.