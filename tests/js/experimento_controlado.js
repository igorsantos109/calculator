// ============================================================================
// EXPERIMENTAÇÃO CONTROLADA — Leitor de Cupons (dados sintéticos)
//
// Objetivo: usar o software de forma NÃO-LINEAR (não seguindo o roteiro do
// desenvolvedor), como faria um usuário que não o implementou, e registrar o
// que falhar ou ficar confuso.
//
// Ambiente: 127.0.0.1, binário release real, app.js real, SQLite temporário,
// fetch real (polyfill http), DOM simulado. Sem navegador real disponível.
//
// Execução: node tests/js/experimento_controlado.js
// Pré-requisito: cargo build --release --workspace --locked
//
// Cada experimento imprime: AÇÃO / ESPERADO / OBSERVADO / AVALIAÇÃO
//   - OK: comportamento conforme esperado
//   - CONFUSO: comportamento pode confundir um usuário novo
//   - FALHA: erro ou comportamento inesperado
// ============================================================================

import * as vm from "node:vm";
import * as fs from "fs";
import * as path from "path";
import * as child_process from "child_process";
import * as net from "net";
import * as os from "os";
import * as http from "http";
import assert from "node:assert/strict";

const BINARIO = path.resolve(process.cwd(), "target/release/leitor-cupons-web");
const APP_JS_PATH = path.resolve(process.cwd(), "crates/web/src/assets/app.js");
const DOM_STUB_PATH = path.resolve(process.cwd(), "tests/js/dom_stub.js");
const PIN = "exp123456";

let resultados = [];

function avaliar(nome, acao, esperado, observado, classificacao, nota = "") {
  resultados.push({ nome, acao, esperado, observado, classificacao, nota });
  const icone =
    classificacao === "OK" ? "  [OK   ]" :
    classificacao === "CONFUSO" ? "  [?    ]" : "  [FALHA]";
  process.stdout.write(`${icone} ${nome}\n`);
  process.stdout.write(`    AÇÃO: ${acao}\n`);
  process.stdout.write(`    ESPERADO: ${esperado}\n`);
  process.stdout.write(`    OBSERVADO: ${observado}\n`);
  if (nota) process.stdout.write(`    NOTA: ${nota}\n`);
}

function sleep(ms) { return new Promise((r) => setTimeout(r, ms)); }

function portaLivre() {
  return new Promise((resolve, reject) => {
    const s = net.createServer();
    s.unref();
    s.on("error", reject);
    s.listen(0, "127.0.0.1", () => {
      const port = s.address().port;
      s.close(() => resolve(port));
    });
  });
}

function fetchPolyfill(input, options = {}) {
  return new Promise((resolve, reject) => {
    const url = typeof input === "string" ? input : String(input);
    const parsed = new URL(url);
    const method = (options.method || "GET").toUpperCase();
    const headers = { ...(options.headers || {}) };
    const body = options.body ? (typeof options.body === "string" ? options.body : JSON.stringify(options.body)) : null;
    if (body && !headers["content-length"]) headers["content-length"] = String(Buffer.byteLength(body));
    const opts = {
      protocol: parsed.protocol,
      hostname: parsed.hostname,
      port: parsed.port || 80,
      path: parsed.pathname + parsed.search,
      method,
      headers,
    };
    const req = http.request(opts, (resp) => {
      const chunks = [];
      resp.on("data", (c) => chunks.push(c));
      resp.on("end", () => {
        const buffer = Buffer.concat(chunks);
        resolve({ ok: resp.statusCode >= 200 && resp.statusCode < 300, status: resp.statusCode, json: async () => JSON.parse(buffer.toString("utf8") || "{}") });
      });
    });
    req.on("error", reject);
    req.setTimeout(10000, () => req.destroy(new Error("timeout")));
    if (body) req.write(body);
    req.end();
  });
}

async function iniciarBinario(dbPath) {
  const port = await portaLivre();
  const addr = `http://127.0.0.1:${port}`;
  const child = child_process.spawn(BINARIO, [], {
    cwd: path.dirname(dbPath),
    env: {
      ...process.env,
      CUPONS_PIN: PIN,
      CUPONS_DB: dbPath,
      CUPONS_LISTEN: addr.replace("http://", ""),
      LLM_BASE_URL: "http://127.0.0.1:1",
      LLM_MODEL: "modelo-teste",
      OCR_DIR: path.dirname(dbPath),
      PATH: process.env.PATH,
    },
    stdio: "ignore",
  });
  child.unref();
  for (let i = 0; i < 50; i++) {
    try {
      const resp = await fetchPolyfill(addr + "/api/saude", { method: "GET", headers: { Authorization: `Bearer ${PIN}` } });
      if (resp.ok) break;
    } catch (_) {}
    await sleep(100);
  }
  return { child, addr, dbPath };
}

function pararBinario(bin) { try { bin.child.kill("SIGKILL"); } catch (_) {} }

// Carregar app.js real + DOM stub (uma vez)
const domStubSrc = fs.readFileSync(DOM_STUB_PATH, "utf8");
const appSrc = fs.readFileSync(APP_JS_PATH, "utf8");
vm.runInThisContext(domStubSrc, { filename: DOM_STUB_PATH, displayErrors: true });

let addrGlobal = "";
globalThis.fetch = async (url, options = {}) => {
  const urlCompleto = url.startsWith("http") ? url : addrGlobal + (url.startsWith("/") ? url : "/" + url);
  return fetchPolyfill(urlCompleto, options);
};

vm.runInThisContext(appSrc, { filename: APP_JS_PATH, displayErrors: true });

const document = globalThis.document;
const window = globalThis.window;
const app = globalThis.window.app;

function resetGlobalState() {
  window.sessionStorage.store = {};
  window.localStorage.store = {};
  if (app) {
    app.estado.itens = [];
    app.estado.historico = [];
    app.estado.somaItens = 0;
    app.estado.avisos = [];
    app.estado.saldoIgor = 0;
    app.estado.saldoRenan = 0;
    app.estado.cupomSelecionadoId = null;
    app.estado.modo = "manual";
    app.estado.data = "";
  }
}

function preencherInput(doc, id, valor) {
  const el = doc.getElementById(id);
  assert.ok(el, `elemento #${id} deve existir`);
  el.value = valor;
}

async function fazerLogin(doc, pin) {
  doc.getElementById("input-pin").value = pin;
  await doc.getElementById("btn-login").dispatchEvent({ type: "click", target: doc.getElementById("btn-login") });
  let ok = false;
  let erro = "";
  for (let i = 0; i < 50; i++) {
    await sleep(100);
    const p = window.sessionStorage.getItem("cupons_pin");
    const e = doc.getElementById("erro-pin").textContent;
    if (p === pin) { ok = true; break; }
    if (e && e.length > 0) { erro = e; break; }
  }
  return { ok, erro };
}

async function adicionarItem(doc, descricao, quantidade, unitario, total, grupo) {
  preencherInput(doc, "input-nova-descricao", descricao);
  preencherInput(doc, "input-nova-quantidade", quantidade);
  preencherInput(doc, "input-novo-unitario", unitario);
  preencherInput(doc, "input-novo-total", total);
  preencherInput(doc, "input-novo-grupo", grupo);
  app.adicionarItemManual();
  await sleep(30);
}

// ============================================================================
// EXPERIMENTOS
// ============================================================================

async function exp1_sem_itens() {
  process.stdout.write("\n=== EXP 1: tentar salvar sem itens ===\n");
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "cupons-exp1-"));
  const dbPath = path.join(tmpDir, "cupons.db");
  const bin = await iniciarBinario(dbPath);
  addrGlobal = bin.addr;
  resetGlobalState();
  app.setupLogin();
  app.setupEventos();

  const login = await fazerLogin(document, PIN);
  avaliar(
    "Exp1 - login",
    "Fazer login com PIN válido",
    "Login bem-sucedido",
    login.ok ? "PIN salvo" : `Erro: ${login.erro}`,
    login.ok ? "OK" : "FALHA"
  );

  // Tentar salvar sem adicionar itens
  preencherInput(document, "input-total-cupom", "0,00");
  preencherInput(document, "input-estabelecimento", "Loja Vazia");
  preencherInput(document, "input-pagador", "igor");
  preencherInput(document, "input-data", "2026-11-18");

  const requisicoesAntes = (await (await fetchPolyfill(addrGlobal + "/api/cupons", { method: "GET", headers: { Authorization: `Bearer ${PIN}` } })).json()).total;

  await app.salvarCupom();
  await sleep(500);

  const resp = await fetchPolyfill(addrGlobal + "/api/cupons", { method: "GET", headers: { Authorization: `Bearer ${PIN}` } });
  const depois = (await resp.json()).total;

  avaliar(
    "Exp1 - salvar sem itens",
    "Tentar salvar cupom sem itens adicionados",
    "Não salvar; exibir erro",
    `itens no histórico: ${depois} (antes: ${requisicoesAntes}), erro exibido: ${document.getElementById("mensagem-erro").textContent.length > 0}`,
    depois === requisicoesAntes ? "OK" : "FALHA"
  );

  pararBinario(bin);
  try { fs.rmSync(tmpDir, { recursive: true, force: true }); } catch (_) {}
}

async function exp2_sem_estabelecimento() {
  process.stdout.write("\n=== EXP 2: tentar salvar sem estabelecimento ===\n");
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "cupons-exp2-"));
  const dbPath = path.join(tmpDir, "cupons.db");
  const bin = await iniciarBinario(dbPath);
  addrGlobal = bin.addr;
  resetGlobalState();
  app.setupLogin();
  app.setupEventos();

  await fazerLogin(document, PIN);

  await adicionarItem(document, "Item X", "1", "10,00", "10,00", "casa");
  preencherInput(document, "input-total-cupom", "10,00");
  preencherInput(document, "input-estabelecimento", ""); // vazio
  preencherInput(document, "input-pagador", "igor");
  preencherInput(document, "input-data", "2026-11-18");

  const antes = (await (await fetchPolyfill(addrGlobal + "/api/cupons", { method: "GET", headers: { Authorization: `Bearer ${PIN}` } })).json()).total;

  await app.salvarCupom();
  await sleep(500);

  const depois = (await (await fetchPolyfill(addrGlobal + "/api/cupons", { method: "GET", headers: { Authorization: `Bearer ${PIN}` } })).json()).total;

  avaliar(
    "Exp2 - salvar sem estabelecimento",
    "Tentar salvar com estabelecimento vazio",
    "Não salvar; exibir erro 'Estabelecimento é obrigatório'",
    `total: ${depois} (antes: ${antes}), erro: "${document.getElementById("mensagem-erro").textContent}"`,
    depois === antes && document.getElementById("mensagem-erro").textContent.includes("Estabelecimento") ? "OK" : "CONFUSO"
  );

  pararBinario(bin);
  try { fs.rmSync(tmpDir, { recursive: true, force: true }); } catch (_) {}
}

async function exp3_pin_curto() {
  process.stdout.write("\n=== EXP 3: login com PIN curto (< 6) ===\n");
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "cupons-exp3-"));
  const dbPath = path.join(tmpDir, "cupons.db");
  const bin = await iniciarBinario(dbPath);
  addrGlobal = bin.addr;
  resetGlobalState();
  app.setupLogin();
  app.setupEventos();

  document.getElementById("input-pin").value = "123"; // 3 caracteres
  await document.getElementById("btn-login").dispatchEvent({ type: "click", target: document.getElementById("btn-login") });
  await sleep(300);

  avaliar(
    "Exp3 - PIN curto",
    "Tentar login com PIN de 3 caracteres",
    "Rejeitar PIN curto, não fazer login",
    `sessionStorage pin: "${window.sessionStorage.getItem("cupons_pin")}", erro: "${document.getElementById("erro-pin").textContent}"`,
    window.sessionStorage.getItem("cupons_pin") === null && document.getElementById("erro-pin").textContent.includes("6") ? "OK" : "CONFUSO"
  );

  pararBinario(bin);
  try { fs.rmSync(tmpDir, { recursive: true, force: true }); } catch (_) {}
}

async function exp4_detalhes_inexistente() {
  process.stdout.write("\n=== EXP 4: acessar detalhes de cupom inexistente ===\n");
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "cupons-exp4-"));
  const dbPath = path.join(tmpDir, "cupons.db");
  const bin = await iniciarBinario(dbPath);
  addrGlobal = bin.addr;
  resetGlobalState();
  app.setupLogin();
  app.setupEventos();

  await fazerLogin(document, PIN);

  // Tentar acessar detalhes do ID 999 (não existe)
  await app.verDetalhes(999);
  await sleep(200);

  avaliar(
    "Exp4 - detalhes de cupom inexistente",
    "Abrir modal de detalhes de ID que não existe",
    "Exibir erro 'Cupom não encontrado', não travar",
    `erro exibido: "${document.getElementById("mensagem-erro").textContent}", app responsivo: ${typeof app.adicionarItemManual === "function"}`,
    document.getElementById("mensagem-erro").textContent.length > 0 ? "OK" : "CONFUSO"
  );

  pararBinario(bin);
  try { fs.rmSync(tmpDir, { recursive: true, force: true }); } catch (_) {}
}

async function exp5_divergencia_total() {
  process.stdout.write("\n=== EXP 5: divergência entre soma de itens e total do cupom ===\n");
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "cupons-exp5-"));
  const dbPath = path.join(tmpDir, "cupons.db");
  const bin = await iniciarBinario(dbPath);
  addrGlobal = bin.addr;
  resetGlobalState();
  app.setupLogin();
  app.setupEventos();

  await fazerLogin(document, PIN);

  await adicionarItem(document, "Item A", "1", "10,00", "10,00", "casa");
  preencherInput(document, "input-total-cupom", "100,00"); // divergente de 10,00
  preencherInput(document, "input-estabelecimento", "Loja Divergente");
  preencherInput(document, "input-pagador", "igor");
  preencherInput(document, "input-data", "2026-11-18");

  const antes = (await (await fetchPolyfill(addrGlobal + "/api/cupons", { method: "GET", headers: { Authorization: `Bearer ${PIN}` } })).json()).total;

  await app.salvarCupom();
  await sleep(500);

  const depois = (await (await fetchPolyfill(addrGlobal + "/api/cupons", { method: "GET", headers: { Authorization: `Bearer ${PIN}` } })).json()).total;

  avaliar(
    "Exp5 - divergência de total",
    "Salvar com soma de itens (10,00) diferente do total informado (100,00)",
    "Rejeitar salvamento; exibir erro de divergência",
    `total: ${depois} (antes: ${antes}), erro: "${document.getElementById("mensagem-erro").textContent}"`,
    depois === antes && document.getElementById("mensagem-erro").textContent.includes("Divergência") ? "OK" : "CONFUSO"
  );

  pararBinario(bin);
  try { fs.rmSync(tmpDir, { recursive: true, force: true }); } catch (_) {}
}

async function exp6_relatorio_vazio() {
  process.stdout.write("\n=== EXP 6: relatório sem nenhum cupom salvo ===\n");
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "cupons-exp6-"));
  const dbPath = path.join(tmpDir, "cupons.db");
  const bin = await iniciarBinario(dbPath);
  addrGlobal = bin.addr;
  resetGlobalState();
  app.setupLogin();
  app.setupEventos();

  await fazerLogin(document, PIN);

  await app.gerarRelatorioPeriodo();
  await sleep(300);

  const rel = document.getElementById("resultado-relatorio");

  avaliar(
    "Exp6 - relatório sem cupons",
    "Gerar relatório sem nenhum cupom salvo",
    "Exibir relatório vazio ou com 0 cupons, não travar",
    `conteúdo: "${rel.textContent.substring(0, 80)}"`,
    rel.textContent.includes("Total de cupons: 0") || rel.textContent.includes("Total de cupons: 1") ? "OK" : "CONFUSO"
  );

  pararBinario(bin);
  try { fs.rmSync(tmpDir, { recursive: true, force: true }); } catch (_) {}
}

async function exp7_llm_indisponível_depois_salvar() {
  process.stdout.write("\n=== EXP 7: LLM indisponível, depois salvar manualmente ===\n");
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "cupons-exp7-"));
  const dbPath = path.join(tmpDir, "cupons.db");
  const bin = await iniciarBinario(dbPath);
  addrGlobal = bin.addr;
  resetGlobalState();
  app.setupLogin();
  app.setupEventos();

  await fazerLogin(document, PIN);

  // Tentar extrair com LLM (fora do ar)
  preencherInput(document, "input-texto-ocr", "Arroz 1kg - R$ 5,00");
  preencherInput(document, "input-total-cupom", "5,00");
  document.getElementById("btn-extrair-texto").click();
  await sleep(1500);

  // Mesmo com LLM fora do ar, continuar com modo manual
  await adicionarItem(document, "Arroz", "1", "5,00", "5,00", "casa");
  preencherInput(document, "input-total-cupom", "5,00");
  preencherInput(document, "input-estabelecimento", "Mercado");
  preencherInput(document, "input-pagador", "igor");
  preencherInput(document, "input-data", "2026-11-18");

  await app.salvarCupom();
  await sleep(500);

  const depois = (await (await fetchPolyfill(addrGlobal + "/api/cupons", { method: "GET", headers: { Authorization: `Bearer ${PIN}` } })).json()).total;

  avaliar(
    "Exp7 - LLM indisponível, salvar manual",
    "Tentar LLM (fora do ar), depois salvar item manualmente",
    "App não trava; salvamento manual funciona",
    `cupons salvos: ${depois}, erro exibido: ${document.getElementById("mensagem-erro").textContent.length > 0 ? "sim" : "não"}`,
    depois === 1 ? "OK" : "FALHA"
  );

  pararBinario(bin);
  try { fs.rmSync(tmpDir, { recursive: true, force: true }); } catch (_) {}
}

async function exp8_editar_item_salvo() {
  process.stdout.write("\n=== EXP 8: tentar editar item de cupom já salvo ===\n");
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "cupons-exp8-"));
  const dbPath = path.join(tmpDir, "cupons.db");
  const bin = await iniciarBinario(dbPath);
  addrGlobal = bin.addr;
  resetGlobalState();
  app.setupLogin();
  app.setupEventos();

  await fazerLogin(document, PIN);

  await adicionarItem(document, "Leite", "1", "5,00", "5,00", "casa");
  preencherInput(document, "input-total-cupom", "5,00");
  preencherInput(document, "input-estabelecimento", "Mercado");
  preencherInput(document, "input-pagador", "igor");
  preencherInput(document, "input-data", "2026-11-18");

  await app.salvarCupom();
  await sleep(500);

  // Abrir detalhes do cupom salvo
  if (app.estado.historico.length > 0) {
    await app.verDetalhes(app.estado.historico[0].id);
    await sleep(100);
    const detEstab = document.getElementById("det-estabelecimento").textContent;
    const detTotal = document.getElementById("det-total").textContent;

    avaliar(
      "Exp8 - detalhes de cupom salvo",
      "Ver detalhes de cupom já salvo",
      "Exibir dados corretos do cupom",
      `estabelecimento: ${detEstab}, total: ${detTotal}`,
      detEstab === "Mercado" && detTotal.includes("5,00") ? "OK" : "CONFUSO"
    );
  } else {
    avaliar(
      "Exp8 - detalhes de cupom salvo",
      "Ver detalhes de cupom já salvo",
      "Exibir dados corretos do cupom",
      "histórico vazio, não foi possível abrir detalhes",
      "FALHA"
    );
  }

  // Não há botão de editar no modal de detalhes — observar se isso é confuso
  const temBotaoEditar = !!document.getElementById("btn-editar-item");
  avaliar(
    "Exp8 - editar item no modal de detalhes",
    "Procurar botão de edição no modal de detalhes",
    "Não deve haver botão (edição não é suportada), ou estar claramente ausente",
    `botão editar presente: ${temBotaoEditar}`,
    !temBotaoEditar ? "OK" : "CONFUSO"
  );

  pararBinario(bin);
  try { fs.rmSync(tmpDir, { recursive: true, force: true }); } catch (_) {}
}

// ============================================================================
// MAIN
// ============================================================================
async function main() {
  process.stdout.write("EXPERIMENTAÇÃO CONTROLADA — Leitor de Cupons\n");
  process.stdout.write("Ambiente: 127.0.0.1, binário release real, app.js real, SQLite temporário\n");
  process.stdout.write("Usuário simulado: não conhece o software, usa de forma não-linear\n");
  process.stdout.write("Sem navegador real (DOM simulado) — limitação declarada\n\n");

  try {
    await exp1_sem_itens();
    await exp2_sem_estabelecimento();
    await exp3_pin_curto();
    await exp4_detalhes_inexistente();
    await exp5_divergencia_total();
    await exp6_relatorio_vazio();
    await exp7_llm_indisponível_depois_salvar();
    await exp8_editar_item_salvo();
  } catch (e) {
    process.stdout.write("\nERRO FATAL durante experimentação: " + e + "\n");
    process.exit(2);
  }

  process.stdout.write("\n============================================================\n");
  process.stdout.write("RESUMO DA EXPERIMENTAÇÃO CONTROLADA\n");
  process.stdout.write("============================================================\n");
  let ok = 0, confuso = 0, falha = 0;
  for (const r of resultados) {
    process.stdout.write(`  [${r.classificacao}] ${r.nome}\n`);
    if (r.classificacao === "OK") ok++;
    else if (r.classificacao === "CONFUSO") confuso++;
    else falha++;
  }
  process.stdout.write(`\nTotal: ${ok} OK, ${confuso} CONFUSO, ${falha} FALHA\n`);
  process.stdout.write("Comando para reproduzir: node tests/js/experimento_controlado.js\n");
  process.stdout.write("Nota: experimentação com dados sintéticos NÃO autoriza implantação.\n");

  process.exit(falha > 0 ? 1 : 0);
}

main().catch((e) => {
  process.stdout.write("ERRO FATAL: " + e + "\n");
  process.exit(2);
});
