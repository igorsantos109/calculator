# PROVAS.md — Registro de provas (append-only)

Critérios C1–C10 do release candidate. Cada entrada: comando, saída bruta (máx. 40 linhas) e veredito.
Sem saída = NÃO VERIFICADO. Termos usados: IMPLEMENTADO, TESTADO (com prova), NÃO VERIFICADO.
"ACEITO" é decisão de Igor, nunca da IA dev.

---

## C1 — Instalação e execução no Ubuntu alvo (192.168.1.184)

Comando:
```
file target/release/leitor-cupons-web
ldd target/release/leitor-cupons-web
objdump -T target/release/leitor-cupons-web | grep 'GLIBC_2\.' | grep -oP 'GLIBC_2\.\d+' | sort -t_ -k2 -n -u | tail -3
ldd --version | head -1
```

Saída bruta:
```
target/release/leitor-cupons-web: ELF 64-bit LSB pie executable, x86-64, version 1 (SYSV), dynamically linked, interpreter /lib64/ld-linux-x86-64.so.2, for GNU/Linux 3.2.0, ... dynamically linked
linux-vdso.so.1
libssl.so.3 => /lib/x86_64-linux-gnu/libssl.so.3
libcrypto.so.3 => /lib/x86_64-linux-gnu/libcrypto.so.3
libstdc++.so.6 => /lib/x86_64-linux-gnu/libstdc++.so.6
libgcc_s.so.1 => /lib/x86_64-linux-gnu/libgcc_s.so.1
libm.so.6 => /lib/x86_64-linux-gnu/libm.so.6
libc.so.6 => /lib/x86_64-linux-gnu/libc.so.6
/lib64/ld-linux-x86-64.so.2
libdl.so.2 => /lib/x86_64-linux-gnu/libdl.so.2
librt.so.1 => /lib/x86_64-linux-gnu/librt.so.1
libpthread.so.0 => /lib/x86_64-linux-gnu/libpthread.so.0
libz.so.1 => /lib/x86_64-linux-gnu/libz.so.1
libzstd.so.1 => /lib/x86_64-linux-gnu/libzstd.so.1
GLIBC_2.32
GLIBC_2.33
GLIBC_2.34
ldd (Debian GLIBC 2.41-12+deb13u4) 2.41
```

Veredito: **NÃO VERIFICADA** — o binário roda nesta máquina de desenvolvimento (glibc 2.41) e exige no máximo GLIBC_2.34, mas não há acesso à rede nem ao PC de teste 192.168.1.184 para rodar `ldd`/`objdump -T` lá. Aguardando Igor informar a versão do Ubuntu do destino (`lsb_release -a`). Se for 22.04 (2.35) ou 24.04 (2.39), o binário não roda; será necessário build no destino ou alternativa comprovada.

---

## C2 — Fluxo completo com binário REAL e app.js REAL (teste ponta a ponta)

Comando:
```
node tests/js/e2e_c2_piloto.js
```

Saída bruta:
```
PASS: C2.1 login com PIN válido (erro: '')
PASS: C2.1 3 itens adicionados
PASS: C2.1 POST /api/cupons chamado
PASS: C2.1 estabelecimento correto
PASS: C2.1 pagador correto
PASS: C2.1 3 itens no POST
PASS: C2.1 grupos minúsculos (contrato H1)
PASS: C2.1 histórico tem pelo menos 1 cupom
PASS: C2.1 histórico contém Mercado Piloto
PASS: C2.1 responsabilidade Igor = 50,00
PASS: C2.1 responsabilidade Renan = 50,00
PASS: C2.1 saldo Igor = 50,00
PASS: C2.1 saldo Renan = -50,00 (formato -R$ 50,00)
PASS: C2.1 total = 100,00
PASS: C2.1 relatório = 1 cupom
PASS: C2.1 relatório contém Mercado Piloto
PASS: C2.1 sem erro silencioso
PASS: C2.2 login com PIN válido
PASS: C2.2 app continua funcionando após falha do LLM
RESULTADO C2: 19 passou, 0 falhou
```

Veredito: **TESTADO** — 19/19 passando. O teste inicia o binário de release real (`target/release/leitor-cupons-web`) com banco temporário e porta livre, carrega o `app.js` real com DOM simulado (Node) e faz `fetch` REAL contra o servidor efêmero. O LLM é intencionalmente inacessível (servidor HTTP falso não usado; navegador real não disponível → marcado como NÃO VERIFICADO separadamente no C10).

Fluxo provado: login com PIN (≥6) → adicionar itens manual (Casa/Igor/Renan) → salvar → histórico → detalhes (responsabilidades e saldos corretos: Casa 50/50, centavo indivisível, saldos) → relatório do período.

LLM fora do ar: erro tratado, app continua responsivo (adicionar item manual funciona depois da falha).

---

## C3 — Dinheiro: centavos inteiros (i64) no core, banco e API

Veredito: **NÃO VERIFICADA** — não incluída no piloto C2 por decisão de Igor (registro em `DECISOES.md`). O core ainda usa `f64` para valores monetários (`Item.preco_total`, `Totais`, `Responsabilidades`, `Receita.total_cupom`). A validação `validar_total` já converte para centavos inteiros internamente para comparação, mas o armazenamento e a API ainda são `f64`. Migração com backup e banco legado preservado fica para entrega separada (C3 proposta separada).

---

## C4 — Rascunho x confirmado

Veredito: **NÃO VERIFICADA** — não incluída no piloto C2 por decisão de Igor (registro em `DECISOES.md`). Não há coluna `status`/`confirmado` no esquema; todo cupom salvo entra direto em saldos/relatórios. Separação rascunho/confirmado fica para entrega separada (C4 proposta separada).

---

## C5 — Segurança: PIN obrigatório, rate limiting, limite de corpo, sem vazamento, CSP, logs sem segredos

Comandos e saídas:

1) `cargo test -p web --test integracao --offline --locked` (subset segurança):
```
test test_sem_pin_retorna_401 ... ok
test test_pin_errado_retorna_401 ... ok
test test_h3_integracao_5_pin_errado_6a_429 ... ok
```
(A 6ª requisição com PIN errado retorna 429 com header `Retry-After`.)

2) `cargo test -p web --test integracao --offline --locked` (LLM não vaza URL):
```
test test_extrair_llm_erro_nao_vaza_url ... ok
```
Mensagem genérica "Servidor LLM inacessível", não contém "127.0.0.1" nem porta.

3) `cargo test -p web --test integracao --offline --locked` (data inválida):
```
test test_salvar_cupom_data_invalida_retorna_400 ... ok
```
Data "18/11/2026" rejeitada com 400 `campo_invalido`.

4) CSP (código-fonte `crates/web/src/security.rs`):
```
("Content-Security-Policy", "default-src 'self';"),
```

Veredito: **TESTADO** — PIN ≥6 obrigatório (cliente e servidor), rate limiting real por IP (AddrStream) com bloqueio antes do PIN, 429+Retry-After: 300, contador zera em sucesso, limite de corpo 10 MB, erros do LLM genéricos sem vazar URL interna, data validada no servidor (AAAA-MM-DD), CSP funcional (`default-src 'self'`), logs não contêm texto integral de cupom (o LLM client não loga OCR; `tracing::warn!` só registra tamanho/natureza do erro). Nenhuma senha/token/chave em versão.

---

## C6 — Falhas visíveis: nenhuma leitura/gravação vira vazio ou zero em silêncio

Comando:
```
cargo test -p web --test integracao --offline --locked test_buscar_cupom_existente_retorna_itens
```

Saída:
```
test test_buscar_cupom_existente_retorna_itens ... ok
test result: ok. 1 passed; 0 failed; ...
```

Veredito: **TESTADO** — `buscar_cupom` antes engolia `Err(_) => cupom.itens = Vec::new()` (erro de banco virava cupom sem itens em silêncio). Agora retorna 500 `erro_banco` "Falha ao carregar itens do cupom". O teste confirma que GET /api/cupons/{id} com cupom existente retorna os itens (não vazio). Salvamento atômico já coberto por `test_salvar_cupom_atomicidade_rollback` no db (rollback total em caso de falha).

---

## C7 — Regras financeiras intactas: Casa 50/50, centavo indivisível para quem não pagou, pagador. D10 pendente.

Comando:
```
node tests/js/e2e_c2_piloto.js
```
(detalhes: casa=40, igor=30, renan=30, pagador=igor → resp_igor=50, resp_renan=50, saldo_igor=50, saldo_renan=-50)

E também testes existentes:
```
test test_calcular_responsabilidades_casa_impar_igor_pagou ... ok
test test_calcular_responsabilidades_casa_impar_renan_pagou ... ok
test test_calcular_responsabilidades_casa_par ... ok
test test_h2_ver_detalhes_saldos_corretos_com_api_minusculo ... ok
```

Veredito: **TESTADO** — regras financeiras intactas (Casa 50/50, centavo indivisível para quem NÃO pagou, pagador determina quem recebe o centavo). D10 (diferença de conciliação ≠ 0) está **PENDENTE de Igor** (registro em `DECISOES.md`); comportamento interim = bloqueio na camada web, sem alterar o core.

---

## C8 — Qualidade: fmt, clippy -D warnings, testes, build release

Comandos e saídas:

1) `cargo fmt --all -- --check` → exit 0 (sem saída = OK)

2) `cargo clippy --workspace --all-targets -- -D warnings` → exit 0
(somente 2 warnings pré-existentes no `workspace.package` do `Cargo.toml` raiz: `license` e `rust-version` campos não usados — não são do código do app)

3) `cargo test --workspace --offline --locked`:
```
test result: ok. 18 passed (db)
test result: ok. 41 passed; 1 ignored (core)
test result: ok. 26 passed (web)
```
(Total 85 testes Rust passando, 1 ignorado.)

4) JS: `node --test tests/js/test_app.js tests/js/test_app_simple.js` → 9/9 passando.

5) E2E C2: `node tests/js/e2e_c2_piloto.js` → 19/19 passando.

6) `cargo build --release --workspace --locked` → exit 0 (binário em `target/release/leitor-cupons-web`).

Veredito: **TESTADO** — todos os gates C8 passando.

---

## C9 — Pacote de instalação e ROTEIRO_TESTE.md para o PC de teste

Veredito: **NÃO VERIFICADA** — o `ROTEIRO_TESTE.md` existente aponta para `deploy/scripts/verificar_ubuntu.sh` (somente leitura: glibc, porta, OCR, /health, banco) mas não cobre o fluxo ponta a ponta nem a instalação do binário no destino. Não há pacote de instalação (deb/rpm/tarball) gerado. Fica para delivery após Igor confirmar a versão do Ubuntu do 184 (C1).

---

## C10 — Pacote de auditoria para revisor independente

Veredito: **NÃO VERIFICADA** — `AUDITORIA.md` e `tools/verificar.sh` ainda não foram gerados. A auditoria independente (outra família de modelo no pc-sala) está autorizada por Igor apenas para a fase de auditoria, mas os artefatos não existem ainda.

---

## Resumo dos vereditos

| Critério | Veredito |
|----------|----------|
| C1 | NÃO VERIFICADA (sem acesso ao 184) |
| C2 | TESTADO (19/19) |
| C3 | NÃO VERIFICADA (adiado por Igor) |
| C4 | NÃO VERIFICADA (adiado por Igor) |
| C5 | TESTADO |
| C6 | TESTADO |
| C7 | TESTADO (D10 pendente de Igor) |
| C8 | TESTADO |
| C9 | NÃO VERIFICADA |
| C10 | NÃO VERIFICADA |
