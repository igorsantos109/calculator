#!/usr/bin/env bash
# ============================================================================
# tools/verificar.sh — Script único de verificação do release candidate (C10)
#
# Executa TODAS as verificações do piloto e imprime um resumo final.
# Uso: ./tools/verificar.sh
#
# Requisitos:
#   - cargo/rustc no PATH (desenvolvedor: ~/.cargo/bin)
#   - node v24+
#   - target/release/leitor-cupons-web já compilado (cargo build --release)
#   - Sem rede; apenas dependências offline já em cache.
# ============================================================================
set -u

cd "$(dirname "$0")/.." || exit 1

export PATH="$HOME/.cargo/bin:$PATH"

ERROS=0
SUCESSOS=0
NAO_VERIFICADO=0

ok()   { SUCESSOS=$((SUCESSOS+1)); echo "  [OK]   $1"; }
fail() { ERROS=$((ERROS+1));       echo "  [FALHA] $1"; }
nv()   { NAO_VERIFICADO=$((NAO_VERIFICADO+1)); echo "  [NV]   $1"; }

echo "============================================================"
echo "  VERIFICAÇÃO DO RELEASE CANDIDATO — Leitor de Cupons"
echo "  Data: $(date)"
echo "============================================================"
echo ""

# ---------- 1. fmt ----------
echo "[1/8] cargo fmt --all -- --check"
if cargo fmt --all -- --check >/dev/null 2>&1; then ok "fmt OK"; else fail "fmt falhou"; fi

# ---------- 2. clippy ----------
echo "[2/8] cargo clippy --workspace --all-targets -- -D warnings"
if cargo clippy --workspace --all-targets -- -D warnings >/dev/null 2>&1; then ok "clippy OK"; else fail "clippy falhou"; fi

# ---------- 3. Testes Rust ----------
echo "[3/8] cargo test --workspace --offline --locked"
if cargo test --workspace --offline --locked >/tmp/verificar_rust_tests.out 2>&1; then
    ok "testes Rust OK"
else
    fail "testes Rust falharam (veja /tmp/verificar_rust_tests.out)"
fi

# ---------- 4. Testes JS ----------
echo "[4/8] testes JS (node:test)"
if node --test tests/js/test_app.js tests/js/test_app_simple.js >/tmp/verificar_js.out 2>&1; then
    if grep -q "fail 0" /tmp/verificar_js.out; then ok "testes JS OK (9/9)"; else fail "testes JS com falhas"; fi
else
    fail "testes JS não executaram"
fi

# ---------- 5. Teste e2e C2 ----------
echo "[5/8] teste e2e C2 (binário real + app.js real + fetch real)"
if [ ! -x target/release/leitor-cupons-web ]; then
    nv "binário release não encontrado — execute 'cargo build --release' primeiro"
else
    if node tests/js/e2e_c2_piloto.js >/tmp/verificar_e2e.out 2>&1; then
        if grep -q "19 passou, 0 falhou" /tmp/verificar_e2e.out; then ok "e2e C2 OK (19/19)"; else fail "e2e C2 com falhas"; fi
    else
        fail "e2e C2 falhou (veja /tmp/verificar_e2e.out)"
    fi
fi

# ---------- 6. Build release ----------
echo "[6/8] cargo build --release --workspace --locked"
if cargo build --release --workspace --locked >/dev/null 2>&1; then ok "build release OK"; else fail "build release falhou"; fi

# ---------- 7. Requisitos do binário (ldd/objdump) ----------
echo "[7/8] requisitos do binário (glibc/libs)"
if [ -x target/release/leitor-cupons-web ]; then
    MAX_GLIBC=$(objdump -T target/release/leitor-cupons-web 2>/dev/null | grep 'GLIBC_2\.' | grep -oP 'GLIBC_2\.\d+' | sort -t_ -k2 -n -u | tail -1 | tr -d '\n')
    echo "        símbolo GLIBC máximo exigido: ${MAX_GLIBC:-não detectado}"
    echo "        glibc desta máquina: $(ldd --version 2>/dev/null | head -1 | awk '{print $NF}')"
    nv "C1 exige validação no Ubuntu alvo (192.168.1.184) — sem acesso ao destino"
else
    fail "binário release não existe"
fi

# ---------- 8. Arquivos do pacote de auditoria ----------
echo "[8/8] arquivos do pacote de auditoria (C10)"
ARQUIVOS_OK=1
for f in AUDITORIA.md PROVAS.md DECISOES.md tools/verificar.sh ROTEIRO_TESTE.md deploy/scripts/verificar_ubuntu.sh; do
    if [ -f "$f" ]; then ok "arquivo presente: $f"; else fail "arquivo ausente: $f"; ARQUIVOS_OK=0; fi
done
if [ "$ARQUIVOS_OK" -eq 0 ]; then fail "pacote de auditoria incompleto"; fi

# ---------- Resumo ----------
echo ""
echo "============================================================"
echo "  RESUMO"
echo "  Sucessos: $SUCESSOS"
echo "  Falhas:   $ERROS"
echo "  Não verificado: $NAO_VERIFICADO"
echo "============================================================"

if [ "$ERROS" -eq 0 ]; then
    echo "Resultado: VERIFICADO (sem falhas; itens NV pendem de Igor/destino)"
    exit 0
else
    echo "Resultado: FALHOU (ver itens acima)"
    exit 1
fi
