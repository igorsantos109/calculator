// ============================================================================
// C2 — Teste ponta a ponta (e2e): binário Rust REAL + app.js REAL +
// fetch REAL contra servidor efêmero (banco temporário, porta livre).
//
// Fluxo manual do piloto:
//   1. Login com PIN (≥ 6)
//   2. Adicionar itens manualmente → grupos Casa/Igor/Renan
//   3. Salvar cupom → histórico → detalhes (responsabilidades/saldos)
//   4. Relatório do período
//
// LLM intencionalmente inacessível (piloto manual). Sem mock de fetch:
// inicia o binário de release real; o app.js real fala com ele.
//
// Execução: node tests/js/e2e_c2_piloto.js
// Sai com código 0 se tudo passou, 1 se alguma verificação falhou.
// ============================================================================

import * as vm from "node:vm";
import * as fs from "fs";
import * as path from "path";
import * as child_process from "child_process";
import * as net from "net";
import * as os from "os";
import * as http from "http";
import * as https from "https";
import assert from "node:assert/strict";

const BINARIO = path.resolve(process.cwd(), "target/release/leitor-cupons-web");
const APP_JS_PATH = path.resolve(process.cwd(), "crates/web/src/assets/app.js");
const DOM_STUB_PATH = path.resolve(process.cwd(), "tests/js/dom_stub.js");
const PIN = "piloto123";

let pass = 0, fail = 0;
function check(cond, msg) {
  if (cond) { pass++; process.stdout.write("PASS: " + msg + "\n"); }
  else { fail++; process.stdout.write("FAIL: " + msg + "\n"); }
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
    const isHttps = parsed.protocol === "https:";
    const method = (options.method || "GET").toUpperCase();
    const headers = { ...(options.headers || {}) };
    const body = options.body ? (typeof options.body === "string" ? options.body : JSON.stringify(options.body)) : null;
    if (body && !headers["content-length"]) headers["content-length"] = String(Buffer.byteLength(body));
    const opts = {
      protocol: parsed.protocol,
      hostname: parsed.hostname,
      port: parsed.port || (isHttps ? 443 : 80),
      path: parsed.pathname + parsed.search,
      method,
      headers,
    };
    const req = (isHttps ? https : http).request(opts, (resp) => {
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

async function iniciarBinarioEfemero() {
  const port = await portaLivre();
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "cupons-e2e-"));
  const dbPath = path.join(tempDir, "cupons.db");
  const addr = `http://127.0.0.1:${port}`;

  const child = child_process.spawn(BINARIO, [], {
    cwd: tempDir,
    env: { ...process.env, CUPONS_PIN: PIN, CUPONS_DB: dbPath, CUPONS_LISTEN: addr.replace("http://", ""), LLM_BASE_URL: "http://127.0.0.1:1", PATH: process.env.PATH },
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
  return { child, tempDir, addr };
}

// ---------- Carregar DOM stub + app.js UMA ÚNICA VEZ ----------
const domStubSrc = fs.readFileSync(DOM_STUB_PATH, "utf8");
const appSrc = fs.readFileSync(APP_JS_PATH, "utf8");
let addrGlobal = "";
let requisicoesGlobal = [];

vm.runInThisContext(domStubSrc, { filename: DOM_STUB_PATH, displayErrors: true });

// Sobrescrever o fetch mock do DOM stub com o fetch REAL (polyfill http/https).
// DEVE ser feito DEPOIS do DOM stub e ANTES do app.js.
globalThis.fetch = async (url, options = {}) => {
  const urlCompleto = url.startsWith("http") ? url : addrGlobal + (url.startsWith("/") ? url : "/" + url);
  requisicoesGlobal.push({ url: urlCompleto, options, body: options && options.body ? options.body : null });
  return fetchPolyfill(urlCompleto, options);
};

vm.runInThisContext(appSrc, { filename: APP_JS_PATH, displayErrors: true });

const document = globalThis.document;
const window = globalThis.window;
const app = globalThis.window.app;

function resetGlobalState() {
  window.sessionStorage.store = {};
  window.localStorage.store = {};
  requisicoesGlobal = [];
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

async function runCenario1(addr) {
  addrGlobal = addr;
  resetGlobalState();
  app.setupLogin();
  app.setupEventos();

  const login = await fazerLogin(document, PIN);
  check(login.ok && !login.erro, "C2.1 login com PIN válido (erro: '" + login.erro + "')");

  const itens = [
    { descricao: "Arroz", quantidade: "1", unitario: "40,00", total: "40,00", grupo: "casa" },
    { descricao: "Cerveja", quantidade: "1", unitario: "30,00", total: "30,00", grupo: "igor" },
    { descricao: "Vinho", quantidade: "1", unitario: "30,00", total: "30,00", grupo: "renan" },
  ];
  for (const it of itens) {
    preencherInput(document, "input-nova-descricao", it.descricao);
    preencherInput(document, "input-nova-quantidade", it.quantidade);
    preencherInput(document, "input-novo-unitario", it.unitario);
    preencherInput(document, "input-novo-total", it.total);
    preencherInput(document, "input-novo-grupo", it.grupo);
    app.adicionarItemManual();
    await sleep(30);
  }
  check(app.estado.itens.length === 3, "C2.1 3 itens adicionados");

  preencherInput(document, "input-total-cupom", "100,00");
  preencherInput(document, "input-estabelecimento", "Mercado Piloto");
  preencherInput(document, "input-pagador", "igor");
  preencherInput(document, "input-data", "2026-11-18");

  await app.salvarCupom();
  await sleep(500);

  const postReq = requisicoesGlobal.find(r => r.url.endsWith("/api/cupons") && r.options && r.options.method === "POST");
  check(!!postReq, "C2.1 POST /api/cupons chamado");
  if (postReq) {
    const corpo = JSON.parse(postReq.body);
    check(corpo.receita.estabelecimento === "Mercado Piloto", "C2.1 estabelecimento correto");
    check(corpo.receita.pagador === "igor", "C2.1 pagador correto");
    check(corpo.itens.length === 3, "C2.1 3 itens no POST");
    check(corpo.itens[0].grupo === "casa" && corpo.itens[1].grupo === "igor" && corpo.itens[2].grupo === "renan", "C2.1 grupos minúsculos (contrato H1)");
  }

  check(app.estado.historico.length > 0, "C2.1 histórico tem pelo menos 1 cupom");
  if (app.estado.historico.length > 0) {
    check(app.estado.historico[0].estabelecimento === "Mercado Piloto", "C2.1 histórico contém Mercado Piloto");
  }

  // Detalhes via app.verDetalhes diretamente
  if (app.estado.historico.length > 0) {
    await app.verDetalhes(app.estado.historico[0].id);
    await sleep(100);
    const detIgor = document.getElementById("det-igor").textContent;
    const detRenan = document.getElementById("det-renan").textContent;
    const detSaldoIgor = document.getElementById("det-saldo-igor").textContent;
    const detSaldoRenan = document.getElementById("det-saldo-renan").textContent;
    const detTotal = document.getElementById("det-total").textContent;
    // casa=40, igor=30, renan=30 → metade_casa=20; centavosCasa=4000 (par, sem indivisível)
    // resp_igor = 30+20 = 50; resp_renan = 30+20 = 50
    // saldo_igor = 100-50 = 50; saldo_renan = 0-50 = -50
    check(detIgor.includes("50,00"), "C2.1 responsabilidade Igor = 50,00");
    check(detRenan.includes("50,00"), "C2.1 responsabilidade Renan = 50,00");
    check(detSaldoIgor.includes("50,00"), "C2.1 saldo Igor = 50,00");
    check(detSaldoRenan.includes("50,00"), "C2.1 saldo Renan = -50,00 (formato -R$ 50,00)");
    check(detTotal.includes("100,00"), "C2.1 total = 100,00");
  }

  await app.gerarRelatorioPeriodo();
  await sleep(300);
  const rel = document.getElementById("resultado-relatorio");
  check(rel.textContent.includes("Total de cupons: 1"), "C2.1 relatório = 1 cupom");
  check(rel.textContent.includes("Mercado Piloto"), "C2.1 relatório contém Mercado Piloto");
  check(!document.getElementById("mensagem-erro").textContent.includes("Falha"), "C2.1 sem erro silencioso");
}

async function runCenario2(addr) {
  addrGlobal = addr;
  resetGlobalState();
  app.setupLogin();
  app.setupEventos();

  const login = await fazerLogin(document, PIN);
  check(login.ok && !login.erro, "C2.2 login com PIN válido");

  preencherInput(document, "input-texto-ocr", "Leite 1L - R$ 5,50");
  preencherInput(document, "input-total-cupom", "5,50");
  document.getElementById("btn-extrair-texto").click();
  await sleep(1500); // LLM inacessível → erro tratado

  preencherInput(document, "input-nova-descricao", "Item após falha LLM");
  preencherInput(document, "input-nova-quantidade", "1");
  preencherInput(document, "input-novo-unitario", "10,00");
  preencherInput(document, "input-novo-total", "10,00");
  preencherInput(document, "input-novo-grupo", "igor");
  app.adicionarItemManual();
  await sleep(30);
  check(app.estado.itens.length === 1, "C2.2 app continua funcionando após falha do LLM");
}

async function main() {
  // ========== TESTE 1: fluxo manual completo ==========
  const s1 = await iniciarBinarioEfemero();
  try {
    await runCenario1(s1.addr);
  } finally {
    try { s1.child.kill("SIGKILL"); } catch (_) {}
    try { fs.rmSync(s1.tempDir, { recursive: true, force: true }); } catch (_) {}
  }

  // ========== TESTE 2: LLM fora do ar não trava o fluxo ==========
  const s2 = await iniciarBinarioEfemero();
  try {
    await runCenario2(s2.addr);
  } finally {
    try { s2.child.kill("SIGKILL"); } catch (_) {}
    try { fs.rmSync(s2.tempDir, { recursive: true, force: true }); } catch (_) {}
  }

  process.stdout.write("\nRESULTADO C2: " + pass + " passou, " + fail + " falhou\n");
  process.exit(fail > 0 ? 1 : 0);
}

main().catch(e => { process.stdout.write("ERRO FATAL: " + e + "\n"); process.exit(2); });
