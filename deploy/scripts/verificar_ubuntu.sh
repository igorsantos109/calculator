#!/usr/bin/env bash
# ============================================================
# Script de verificação somente leitura — PC de teste Ubuntu
# Alvo: 192.168.1.184 (rede interna, sem interface gráfica)
#
# NÃO altera nada no sistema. Apenas coleta informações para
# validar se o ambiente está pronto para rodar o leitor de cupons.
# ============================================================
set -u

BINARIO="${1:-./target/release/leitor-cupons-web}"
PORTA="${2:-8080}"
HOST="${3:-127.0.0.1}"

echo "============================================================"
echo "  VERIFICAÇÃO SOMENTE LEITURA — Leitor de Cupons Fiscais"
echo "  Alvo: $HOST:$PORTA"
echo "  Data: $(date)"
echo "============================================================"
echo ""

# ---------- 1. Versão da glibc ----------
echo "[1/6] Versão da glibc do ambiente de execução:"
if command -v ldd >/dev/null 2>&1; then
    ldd --version 2>/dev/null | head -1
else
    echo "  ldd não encontrado; glibc não pôde ser verificada"
fi
echo ""

# ---------- 2. Binário release ----------
echo "[2/6] Binário release:"
if [ -x "$BINARIO" ]; then
    echo "  OK: $BINARIO (executável)"
    echo "  Tamanho: $(du -h "$BINARIO" 2>/dev/null | cut -f1)"
else
    echo "  FALHA: binário não encontrado ou não executável: $BINARIO"
fi
echo ""

# ---------- 3. Porta livre ----------
echo "[3/6] Porta $PORTA em $HOST:"
if command -v ss >/dev/null 2>&1; then
    if ss -ltn 2>/dev/null | grep -q ":${PORTA} "; then
        echo "  Porta $PORTA já está em uso (algo está escutando)."
    else
        echo "  Porta $PORTA está livre."
    fi
elif command -v netstat >/dev/null 2>&1; then
    if netstat -ltn 2>/dev/null | grep -q ":${PORTA} "; then
        echo "  Porta $PORTA já está em uso."
    else
        echo "  Porta $PORTA está livre."
    fi
else
    echo "  ss/netstat não disponíveis; não foi possível verificar a porta."
fi
echo ""

# ---------- 4. ONNX / modelos OCR ----------
echo "[4/6] OCR (ONNX Runtime e modelos alvio-ocr):"
OCR_DIR="${OCR_DIR:-./ocr}"
if [ -d "$OCR_DIR" ]; then
    echo "  Pasta OCR existe: $OCR_DIR"
    ls -la "$OCR_DIR" 2>/dev/null | head -10
else
    echo "  Pasta OCR não encontrada: $OCR_DIR"
    echo "  (AVISO: OCR real não está disponível neste build offline."
    echo "   Use Modo manual ou colar texto.)"
fi
echo ""

# ---------- 5. Alcance do llama-server (/health) ----------
echo "[5/6] Alcance do llama-server (/health):"
LLM_BASE="${LLM_BASE_URL:-http://127.0.0.1:11434/v1}"
HEALTH_URL="${LLM_BASE%/}/health"
if command -v curl >/dev/null 2>&1; then
    codigo=$(curl -s -o /dev/null -w "%{http_code}" --connect-timeout 5 "$HEALTH_URL" 2>/dev/null || echo "000")
    if [ "$codigo" = "200" ]; then
        echo "  OK: llama-server alcançável em $HEALTH_URL (HTTP 200)"
    elif [ "$codigo" = "000" ]; then
        echo "  Indisponível: não foi possível conectar a $HEALTH_URL"
        echo "  (Use servidor HTTP falso nos testes ou configure llm_base_url)"
    else
        echo "  Resposta HTTP $codigo em $HEALTH_URL"
    fi
else
    echo "  curl não disponível; não foi possível testar alcance."
fi
echo ""

# ---------- 6. /api/saude e permissões do banco ----------
echo "[6/6] API /api/saude e banco:"
if command -v curl >/dev/null 2>&1; then
    resposta=$(curl -s --connect-timeout 5 "http://${HOST}:${PORTA}/api/saude" 2>/dev/null || echo "")
    if [ -n "$resposta" ]; then
        echo "  /api/saude respondeu:"
        echo "  $resposta" | head -5
    else
        echo "  /api/saude não respondeu (servidor não está rodando ou porta incorreta)."
    fi
else
    echo "  curl não disponível; não foi possível consultar /api/saude."
fi

DB_PATH="${CUPONS_DB:-./cupons.db}"
if [ -e "$DB_PATH" ]; then
    echo "  Banco: $DB_PATH"
    echo "  Permissões: $(ls -ld "$DB_PATH" 2>/dev/null | cut -d' ' -f1)"
    echo "  Proprietário: $(ls -ld "$DB_PATH" 2>/dev/null | cut -d' ' -f3)"
else
    echo "  Banco não existe ainda (será criado na primeira execução): $DB_PATH"
fi

echo ""
echo "============================================================"
echo "  FIM DA VERIFICAÇÃO (somente leitura)"
echo "============================================================"
