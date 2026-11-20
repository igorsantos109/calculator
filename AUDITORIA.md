# AUDITORIA.md — Pacote para revisor independente (C10)

Leitor de Cupons Fiscais — release candidate piloto (branch `agent/web-llm`).

Este pacote permite que um revisor de outra família de modelo (carregado no pc-sala,
apenas para a fase de auditoria, autorizado por Igor) verifique o trabalho sem
depender da IA que implementou. A IA implementadora NÃO dá o próprio aceite.

## Como usar este pacote

1. Leia `DECISOES.md` (decisões de Igor: push proibido, D10 adiado, C1/C3/C4
   pendentes, modelo do auditor autorizado só na fase de auditoria).
2. Leia `PROVAS.md` (registro de provas C1–C10 com comandos, saídas e vereditos).
3. Rode `tools/verificar.sh` — executa todas as verificações e imprime um resumo.
4. Revise o diff (`git diff` + `git status`) dos arquivos alterados.
5. Registre o aceite (ou rejeição) por critério. Apenas Igor aprova ("ACEITO").

## Lista de arquivos relevantes

| Arquivo | Função |
|---------|--------|
| `crates/core/src/lib.rs` | Tipos de domínio, regras financeiras (Casa 50/50, centavo indivisível), validação de total em centavos |
| `crates/db/src/lib.rs` | Repositório SQLite, schema versionado (v1–v3), migrações, backup antes de migrar, atomicidade |
| `crates/web/src/server.rs` | Servidor HTTP (hyper), rotas /api, PIN, rate limiting por IP, limite de corpo, erros genéricos, data validada |
| `crates/web/src/config.rs` | Config via variáveis de ambiente (CUPONS_PIN, CUPONS_DB, LLM_BASE_URL, etc.) |
| `crates/web/src/security.rs` | PIN comparação em tempo constante, cabeçalhos de segurança (CSP `default-src 'self'`) |
| `crates/web/src/assets/app.js` | Frontend: login, itens, salvar, histórico, detalhes, relatório, saldos |
| `crates/web/src/assets/index.html` | Página única (sem CDN) |
| `crates/web/tests/integracao.rs` | 26 testes de integração Rust (banco temp, LLM falso, servidor real) |
| `tests/js/test_app.js` | 5 testes JS (node:test + DOM simulado) |
| `tests/js/test_app_simple.js` | 4 testes JS (variante enxuta) |
| `tests/js/dom_stub.js` | DOM mínimo para testar app.js em Node |
| `tests/js/e2e_c2_piloto.js` | Teste ponta a ponta C2: binário real + app.js real + fetch real |
| `deploy/scripts/verificar_ubuntu.sh` | Verificação somente leitura no Ubuntu alvo (glibc, porta, OCR, /health, banco) |
| `ROTEIRO_TESTE.md` | Roteiro de teste para o PC Ubuntu sem GUI |

## Como rodar cada verificação

```bash
# 0. Ambiente
export PATH="$HOME/.cargo/bin:$PATH"   # cargo/rustc
cd /home/igor/ia-sh/calculador

# 1. Formatação
cargo fmt --all -- --check

# 2. Clippy (tratar avisos como erro)
cargo clippy --workspace --all-targets -- -D warnings

# 3. Testes Rust (offline, lockfile fixo)
cargo test --workspace --offline --locked

# 4. Testes JS (9 testes)
node --test tests/js/test_app.js tests/js/test_app_simple.js

# 5. Teste e2e C2 (binário real + app.js real + fetch real)
# Pré-requisito: cargo build --release --workspace --locked
node tests/js/e2e_c2_piloto.js

# 6. Build release
cargo build --release --workspace --locked

# 7. Verificação somente leitura no Ubuntu alvo (executar no 192.168.1.184)
# ./deploy/scripts/verificar_ubuntu.sh ./target/release/leitor-cupons-web 8080 127.0.0.1
```

## Mapa critério → teste

| Critério | Onde é testado | Veredito (PROVAS.md) |
|----------|---------------|----------------------|
| C1 Ubuntu alvo | `ldd`/`objdump -T` no destino | NÃO VERIFICADA |
| C2 Fluxo completo | `tests/js/e2e_c2_piloto.js` (19/19) | TESTADO |
| C3 Centavos inteiros | (adiado por Igor) | NÃO VERIFICADA |
| C4 Rascunho/confirmado | (adiado por Igor) | NÃO VERIFICADA |
| C5 Segurança | `integracao.rs` (PIN, rate limit, corpo, LLM erro genérico, data) + `security.rs` (CSP) | TESTADO |
| C6 Falhas visíveis | `test_buscar_cupom_existente_retorna_itens` + atomicidade db | TESTADO |
| C7 Regras financeiras | `core` testes de responsabilidades + e2e saldos | TESTADO (D10 pendente) |
| C8 Qualidade | fmt, clippy, `cargo test --workspace`, JS, e2e, build release | TESTADO |
| C9 Pacote + roteiro Ubuntu | `ROTEIRO_TESTE.md`, `deploy/scripts/verificar_ubuntu.sh` | NÃO VERIFICADA |
| C10 Pacote de auditoria | Este arquivo + `tools/verificar.sh` | NÃO VERIFICADA (gerar agora) |

## Riscos conhecidos

- **C1 não testada no destino real**: sem acesso à rede nem ao PC 192.168.1.184, não se pode provar que o binário roda no Ubuntu alvo. Se o destino for 22.04/24.04 (glibc < 2.41), o build atual não roda.
- **C3 e C4 adiados**: migração monetária e separação rascunho/confirmado não estão no piloto. São entregas separadas propostas em `DECISOES.md`.
- **OCR real não verificável offline**: `alvio-ocr`/ONNX não disponível sem rede. O modo manual/colar-texto é o caminho principal.
- **LLM real não verificável offline**: llama-server em outro PC não acessível. Testes usam servidor HTTP falso.
- **Navegador real não testado**: o teste e2e usa DOM simulado (Node), não um navegador de verdade.
- **Rede real não testada**: testes rodam em localhost. Firewall/aceso LAN não verificados.
- **D10 pendente**: diferença de conciliação ≠ 0 bloqueia na camada web até Igor decidir.
- **Backups `.bak` na raiz**: gerados automaticamente por `backup_antes_de_migrar()` quando o banco existe no CWD. Movidos para quarentena por Igor; recomendação de gravar em subdiretório dedicado.

## O que NÃO foi testado (registrado por exigência do protocolo)

- OCR real (imagem → texto via alvio-ocr/ONNX)
- LLM real (llama-server em outro PC da LAN)
- Navegador real (Chrome/Firefox; só DOM simulado em Node)
- Rede real / firewall / acesso pela LAN (apenas localhost)
- Execução no Ubuntu 192.168.1.184 (C1 NÃO VERIFICADA)
- Dados reais (nenhum banco/backup real foi lido, movido ou commitado)

## Arquivos alterados nesta sessão (piloto)

- `crates/web/src/server.rs` — C6 (buscar_cupom não engole erro de itens), C5 (erros do LLM genéricos sem vazar URL, validação de data no servidor)
- `crates/web/src/assets/app.js` — `await carregarHistorico()` após salvar; exposição de `setupEventos`, `extrairItensTextoOCR`, `carregarHistorico`, `gerarRelatorioPeriodo` para teste
- `crates/web/tests/integracao.rs` — +3 testes (data inválida 400, LLM erro não vaza URL, buscar_cupom retorna itens)
- `tests/js/e2e_c2_piloto.js` — novo (teste ponta a ponta C2)
- `DECISOES.md` — novo (decisões de Igor)
- `PROVAS.md` — novo (registro de provas C1–C10)
- `AUDITORIA.md` — novo (este arquivo)
- `tools/verificar.sh` — novo (script único de verificação)

## Assinatura do revisor

Preencher e devolver a Igor:

- Revisor (nome/família de modelo): ____________________
- Data: ____________________
- `tools/verificar.sh` passou? (S/N): ____
- Critérios ACEITOS: ____________________
- Critérios REJEITADOS (com motivo): ____________________
- Observações: ____________________

A IA implementadora não assina este termo.
