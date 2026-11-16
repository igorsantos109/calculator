# AGENT_LOG.md — Registro de Atividades do Fluxo Agêntico

## Formato: tarefa | papel | comando | saída resumida | veredito | SHA

---

### ETAPA 0 | LEAD | git status, git log, git diff, git ls-remote origin main
**Saída:** HEAD local c8ce9bf == origin/main c8ce9bf (reconciliado). Branch agent/web-llm criado a partir de c8ce9bf.
**Veredito:** RECONCILIADO — sem divergência entre HEAD local e remoto.
**SHA:** c8ce9bf

### ETAPA 0 | LEAD | cargo fmt --all -- --check
**Saída:** sem saída (sucesso)
**Veredito:** PASS
**SHA:** c8ce9bf

### ETAPA 0 | LEAD | cargo clippy --workspace --all-targets --offline --locked -- -D warnings
**Saída:** PASS após corrigir warnings de dependências não usadas (ollama-rest, chrono, regex) e imports (std::fs em ocr.rs, mut em teste db)
**Veredito:** PASS
**SHA:** a968874 (após ETAPA 1)

### ETAPA 0 | LEAD | cargo test --workspace --offline --locked
**Saída:** db: 22/22 passed | receipt_core: 33/33 passed, 1 ignored (OCR fixture) | ui: removido do workspace | doc-tests: 0
**Veredito:** PASS
**SHA:** a968874

### ETAPA 0 | LEAD | cargo build --workspace --release --offline --locked
**Saída:** Finished release profile em ~1m18s
**Veredito:** PASS
**SHA:** a968874

### ETAPA 0 | LEAD | ldd --version
**Saída:** glibc 2.41 (Debian 2.41-12+deb13u4)
**Veredito:** REGISTRADO — para comparação com glibc do Ubuntu alvo (192.168.1.184)
**SHA:** a968874

### ETAPA 1 | LEAD | Repro D1: programa C/Rust simulando validar_total antigo (f64)
**Saída:** 20.05 f64 = 20.05000000000000071 → diferencial 0.0500000000000007 > 0.05 → rejeita incorretamente. Nova (centavos): 2005 vs 2000 → aceita.
**Veredito:** PROVA DO BUG (causa-raiz confirmada)
**SHA:** —

### ETAPA 1 | LEAD | Implementar D1-D7 (commit 916e106)
**Saída:** 7 arquivos alterados, +737/-398 linhas. Criado crates/core/src/parse.rs.
**Veredito:** CONCLUÍDO — testes D1-D5 passando, clippy -D warnings OK
**SHA:** 916e106

### ETAPA 1 | LEAD | Remover crates/ui do workspace (commit a968874, separado, reversível)
**Saída:** Cargo.toml raiz members=[crates/core, crates/db]; Cargo.lock atualizado (removidas deps do ui).
**Veredito:** CONCLUÍDO — workspace sem ui compila e testa offline
**SHA:** a968874

### PARADA 1 | LEAD | Relatório de entrega + HANDOFF.md + AGENT_LOG.md
**Saída:** documentos atualizados para entrega à espera de liberação de Igor.
**Veredito:** PASS — liberado para ETAPA 2
**SHA:** c8bb5be

### ETAPA 2 | LEAD | Reescrever llm.rs para llama-server (API OpenAI)
**Saída:** crates/core/src/llm.rs reescrito; POST /chat/completions, response_format json_schema,
mensagem system fixa, texto OCR como dado delimitado, retry em transitórios, validação de domínio,
avisos para itens inválidos, soma dos itens reportada, sem logar texto integral/chave.
Testes com servidor HTTP falso (wiremock): válido, cercas ```json, JSON quebrado, 401, 500,
truncado, itens inválidos→avisos, injeção de prompt no OCR, servidor inacessível.
**Veredito:** PASS — 34 testes core (1 ignorado), 22 testes db; clippy -D warnings OK; fmt OK
**SHA:** dcdb499

### PARADA 2 | LEAD | Relatório de entrega + HANDOFF.md + AGENT_LOG.md
**Saída:** documentos atualizados para entrega à espera de liberação de Igor.
**Veredito:** PASS — liberado para ETAPA 3
**SHA:** 4cef37a

### ETAPA 3 | LEAD | Criar crate crates/web (servidor HTTP, API, testes de integração)
**Saída:** crates/web/ criado com servidor HTTP (hyper), API JSON (7 rotas), segurança (PIN, CSRF, upload limit, sanitização), /api/saude com estado real.
Testes de integração: 16 aprovados com servidor falso (wiremock) + banco temporário, cobrindo PIN errado, CSRF ausente, servidor LLM indisponível, JSON quebrado, resposta truncada, chave inválida, itens inválidos → avisos, XSS no OCR.
**Veredito:** PASS — db 18, core 34+1 ignorado, web 16 integração; clippy -D warnings OK; fmt OK
**SHA:** ae260dc

### PARADA 3 | LEAD | Relatório de entrega + HANDOFF.md + AGENT_LOG.md
**Saída:** documentos atualizados para entrega à espera de liberação de Igor.
**Veredito:** PASS — liberado para ETAPA 4
**SHA:** 8b1ecc0

### ETAPA 4 | LEAD | Página interativa (HTML/CSS/JS embutidos no binário)
**Saída:** crates/web/src/assets/ com index.html, style.css, app.js (sem CDN, funcional sem internet). Assets embutidos via include_str! e servidos nas rotas GET /, /style.css, /app.js.
Tema claro/escuro, responsivo, navegação por teclado. Fluxo: enviar/arrastar foto → texto OCR → itens editáveis → grupo → totais e saldos → selo de conciliação → salvar. Modo manual e colar texto. Histórico, detalhes, relatório do período.
**Veredito:** PASS — db 18, core 34+1 ignorado, web 16 integração; JS sintaxe verificada com Node; clippy -D warnings OK; fmt OK
**SHA:** a803185

### PARADA 4 | LEAD | Relatório de entrega + HANDOFF.md + AGENT_LOG.md
**Saída:** documentos atualizados para entrega à espera de liberação de Igor.
**Veredito:** PASS — liberado para ETAPA 5
**SHA:** a803185

### ETAPA 5 | LEAD | Pacote + ROTEIRO_TESTE.md para Ubuntu 192.168.1.184
**Saída:** Criado deploy/config/config.toml (exemplo com aviso de segurança LAN),
deploy/systemd/leitor-cupons.service (usuário, não instalado),
deploy/scripts/verificar_ubuntu.sh (somente leitura),
ROTEIRO_TESTE.md (passos, resultado esperado, coleta de logs, backup/restaura).
Build release --locked OK.
**Veredito:** EM ANDAMENTO — aguardando RELATORIO_ENTREGA.md final
**SHA:** a803185

---

## Totais reais por pacote (após ETAPA 5)
- receipt_core: 34 aprovados, 0 falhos, 1 ignorado (OCR fixture)
- db: 18 aprovados, 0 falhos, 0 ignorados
- doc-tests: 0

## Bloqueios e autorizações
| Bloqueio | Autorização necessária |
|----------|------------------------|
| D10: comportamento de divergência de conciliação | Decisão de Igor (documentado em DIVERGENCIAS.md) |
| Versão do Ubuntu alvo (glibc) | Igor informar |
| Chave do llama-server | Igor informar (nunca gravar em versão) |
| Push/merge | Autorização explícita de Igor |

## Próxima ação
Finalizar RELATORIO_ENTREGA.md e entregar (não publicar sem autorização de Igor).
