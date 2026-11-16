# ROTEIRO_TESTE.md — PC Ubuntu sem interface gráfica (192.168.1.184)

Este roteiro descreve como testar o leitor de cupons fiscais em um
PC Ubuntu Server sem GUI, acessível pela rede interna. Todos os passos
são somente leitura, exceto onde explicitamente indicado.

> **NÃO VERIFICADO**: OCR real, LLM real, navegador real e rede real
> não foram testados neste build. O caminho mínimo funcional é o modo
> Manual + extração por texto colado.

---

## 0. Pré-requisitos (apenas leitura)

| Item | Como verificar | Esperado |
|------|---------------|----------|
| glibc do destino | `ldd --version` | glibc ≥ 2.41 (compatível com build) |
| Binário release | `ls -l target/release/leitor-cupons-web` | Executável, ~15–30 MB |
| Porta livre | `ss -ltn \| grep :8080` | Porta 8080 livre |
| llama-server /health | `curl http://<llm_base>/health` | HTTP 200 (ou indisponível — OK) |
| Banco | Caminho em CUPONS_DB | Nunca ~/cupons.db |

> **Aviso de glibc**: o binário foi compilado em glibc 2.41. Se o Ubuntu
> alvo tiver glibc menor (ex.: 2.35 no Ubuntu 22.04), o binário não
> rodará. Nesse caso, gere um build no destino ou use musl (binário
> estático). O OCR (alvio-ocr/ONNX) depende de bibliotecas nativas;
> sem rede para baixar modelos, **OCR NO ALVO = NÃO VERIFICADO**.

---

## 1. Instalação do binário (somente cópia)

```bash
# No PC de teste
mkdir -p ~/leitor-cupons
# Copie o binário release gerado no PC de build
scp build/target/release/leitor-cupons-web ~/leitor-cupons/
chmod +x ~/leitor-cupons/leitor-cupons-web
```

Não instale nem habilite systemd automaticamente. A unidade systemd de
exemplo está em `deploy/systemd/leitor-cupons.service`.

---

## 2. Execução com PIN

```bash
cd ~/leitor-cupons
export CUPONS_PIN="1234"              # defina um PIN forte
export CUPONS_DB="/tmp/cupons_teste.db"  # banco temporário para teste
export LLM_BASE_URL="http://127.0.0.1:11434/v1"
export LLM_MODEL="llama-3.1-8b"
export OCR_DIR="./ocr"

./leitor-cupons-web
```

Saída esperada:
```
Servidor web escutando em http://127.0.0.1:8080
```

Acesse por navegador em outro PC da rede: `http://192.168.1.184:8080`
(se tiver configurado `listen_addr = 0.0.0.0` ou `192.168.1.184`).

---

## 3. Script de verificação (somente leitura)

```bash
chmod +x deploy/scripts/verificar_ubuntu.sh
./deploy/scripts/verificar_ubuntu.sh ./target/release/leitor-cupons-web 8080 127.0.0.1
```

Saída esperada: todas as 6 seções preenchidas sem erros críticos.

---

## 4. Casos de teste (API — ponta a ponta com banco temporário)

Use `curl` ou um navegador. O PIN deve ser enviado em
`Authorization: Bearer <PIN>` ou header `X-PIN`.

### 4.1 Saúde
```bash
curl -s http://127.0.0.1:8080/api/saude
```
Esperado: JSON com `banco: "ok"`, `schema_versao` numérico, `llm`
("alcançável" ou "indisponível"), `ocr` ("modelos_presentes" ou
"pasta_ocr_ausente").

### 4.2 Sem PIN → 401
```bash
curl -s -o /dev/null -w "%{http_code}" http://127.0.0.1:8080/api/cupons
```
Esperado: `401`.

### 4.3 PIN errado → 401
```bash
curl -s -o /dev/null -w "%{http_code}" -H "Authorization: Bearer errado" http://127.0.0.1:8080/api/cupons
```
Esperado: `401`.

### 4.4 Sem CSRF → 403 (métodos que mudam estado)
```bash
curl -s -o /dev/null -w "%{http_code}" -X POST \
  -H "Authorization: Bearer 1234" \
  -H "Content-Type: application/json" \
  -d '{"receita":{"data_compra":"2026-11-15","estabelecimento":"T","total_cupom":10,"imagem_path":"","pagador":"igor"},"itens":[]}' \
  http://127.0.0.1:8080/api/cupons
```
Esperado: `403` (sem header `X-CSRF-Token`).

### 4.5 Salvar cupom válido → 201
```bash
curl -s -X POST http://127.0.0.1:8080/api/cupons \
  -H "Authorization: Bearer 1234" -H "X-CSRF-Token: abc" \
  -H "Content-Type: application/json" \
  -d '{"receita":{"data_compra":"2026-11-15","estabelecimento":"Super","total_cupom":20,"imagem_path":"","pagador":"igor"},"itens":[{"item":{"descricao":"Leite","quantidade":1,"preco_unitario":20,"preco_total":20},"grupo":"Casa"}]}'
```
Esperado: `201`, JSON com `id` e `mensagem`.

### 4.6 Divergência de conciliação → 400 (D10)
Soma dos itens difere do total em mais de 5 centavos:
```bash
curl -s -o /dev/null -w "%{http_code}" -X POST ... \
  -d '{"receita":{"...","total_cupom":100,"pagador":"igor"},"itens":[{"item":{"descricao":"X","quantidade":1,"preco_unitario":100.06,"preco_total":100.06},"grupo":"Casa"}]}'
```
Esperado: `400`, `codigo: "divergencia_total"`.

### 4.7 Listar cupons → 200
```bash
curl -s -H "Authorization: Bearer 1234" http://127.0.0.1:8080/api/cupons
```
Esperado: `200`, array `cupons` com o cupom salvo.

### 4.8 Relatório do período → 200
```bash
curl -s "http://127.0.0.1:8080/api/relatorio?de=2026-11-01&ate=2026-11-30" \
  -H "Authorization: Bearer 1234"
```
Esperado: `200`, `total_cupons` e `total_geral`.

### 4.9 Extrair itens via LLM (servidor falso ou real)
```bash
curl -s -X POST http://127.0.0.1:8080/api/extrair \
  -H "Authorization: Bearer 1234" -H "X-CSRF-Token: abc" \
  -H "Content-Type: application/json" \
  -d '{"texto_ocr":"Leite 1L - R$ 5,50","total_cupom":5.50}'
```
Esperado: `200`, `itens` com o item, `avisos` vazio, `soma_itens: 5.5`.

---

## 5. Casos de teste — Segurança

| Caso | Comando | Esperado |
|------|---------|----------|
| Upload grande (>10 MB) | POST /api/ocr com arquivo > 10 MB | 413 ou erro |
| Tipo inválido | POST /api/ocr com .txt | Erro |
| Path traversal | Nome de arquivo com `../` | Rejeitado |
| XSS no OCR | texto_ocr com `<script>...` | Retornado como texto, não executado |
| Requisição sem CSRF | POST sem `X-CSRF-Token` | 403 |

---

## 6. Como coletar logs sem expor texto de cupom

O servidor NÃO loga o texto integral do cupom nem a chave do LLM.
Para coletar logs de operação:

```bash
# Executar em foreground e observar stdout/stderr
./leitor-cupons-web 2>&1 | tee ~/leitor-cupons/log_teste.txt
```

Nunca envie o `log_teste.txt` contendo dados de cupom. Os logs de erro
contêm apenas códigos (ex.: `divergencia_total`, `servidor_llm_inacessível`),
não o conteúdo do cupom.

---

## 7. Restaurar backup do banco

Antes de migrar um banco existente, o programa cria automaticamente um
backup com timestamp ao lado do arquivo:

```
cupons.db.20261116T141500.bak
```

Para restaurar:
```bash
cp cupons.db.20261116T141500.bak cupons.db
```

---

## 8. O que NÃO foi testado (limitações)

- OCR real (alvio-ocr/ONNX sem modelos offline)
- LLM real / servidor llama-server real (testes usam servidor HTTP falso)
- Navegador real (interface não foi renderizada em navegador)
- Rede real / intranet (testes em localhost)
- Banco real (~cupons.db nunca usado)

---

## 9. Limitações conhecidas

| Limitação | Impacto | Mitigação |
|-----------|---------|-----------|
| OCR não disponível offline | Não pode extrair texto de imagem | Use Modo manual ou Colar texto |
| glibc 2.41 | Pode não rodar em Ubuntu com glibc < 2.41 | Build no destino ou musl |
| D10 pendente | Divergência ≠ 0 bloqueia no servidor web | Aguardar decisão de Igor |
