import { test } from "node:test";
import * as vm from "node:vm";
import * as fs from "fs";
import * as path from "path";
import assert from "node:assert/strict";

const APP_JS_PATH = path.resolve(process.cwd(), "crates/web/src/assets/app.js");
const DOM_STUB_PATH = path.resolve(process.cwd(), "tests/js/dom_stub.js");

let ultimaRequisicaoEnviada = null;
let requisicoes = [];
let fetchMockRespostas = {};

globalThis.setTimeout = (fn) => { try { fn(); } catch (e) {} return 0; };
globalThis.clearTimeout = () => {};

const domStubSrc = fs.readFileSync(DOM_STUB_PATH, "utf8");
const appSrc = fs.readFileSync(APP_JS_PATH, "utf8");

vm.runInThisContext(domStubSrc, { filename: DOM_STUB_PATH, displayErrors: true });

globalThis.fetch = async (url, options = {}) => {
  const registro = { url, options, body: options && options.body ? options.body : null };
  requisicoes.push(registro);
  ultimaRequisicaoEnviada = registro;
  const resposta = fetchMockRespostas[url] || {
    ok: false, status: 404,
    json: async () => ({ codigo: "erro", mensagem: "sem mock" }),
  };
  return resposta;
};

vm.runInThisContext(appSrc, { filename: APP_JS_PATH, displayErrors: true });

const document = globalThis.document;
const window = globalThis.window;
const app = globalThis.window.app;

function resetGlobalState() {
  window.sessionStorage.store = {};
  window.localStorage.store = {};
  ultimaRequisicaoEnviada = null;
  requisicoes = [];
  fetchMockRespostas = {};
}

function mockResposta(url, resposta) {
  fetchMockRespostas[url] = resposta;
}

function criarElemento(id, tag = "div") {
  const el = {
    tagName: tag.toUpperCase(), id, className: "", hidden: false, value: "",
    textContent: "", innerHTML: "", dataset: {}, attributes: {}, children: [],
    eventListeners: new Map(), parentElement: null, style: {}, required: false,
  };
  el.setAttribute = function (n, v) { this.attributes[n] = v; if (n === "id") this.id = v; if (n === "class") this.className = v; if (n === "data-campo") this.dataset.campo = v; if (n === "data-idx") this.dataset.idx = v; if (n === "required") this.required = true; };
  el.getAttribute = function (n) { return this.attributes[n]; };
  el.addEventListener = function (ev, h) { if (!this.eventListeners.has(ev)) this.eventListeners.set(ev, []); this.eventListeners.get(ev).push(h); };
  el.removeEventListener = function (ev, h) { const hh = this.eventListeners.get(ev); if (hh) { const i = hh.indexOf(h); if (i > -1) hh.splice(i, 1); } };
  el.dispatchEvent = function (ev) { const hh = this.eventListeners.get(ev.type); if (hh) hh.forEach((fn) => fn(ev)); };
  el.closest = function (sel) { if (sel === ".btn-remover" && this.className.includes("btn-remover")) return this; return this.parentElement; };
  el.querySelector = function (sel) {
    if (sel.startsWith("#")) {
      const elId = sel.slice(1);
      for (const e of document.elements.values()) if (e.id === elId) return e;
      return null;
    }
    return this.children.find((c) => c.tagName.toLowerCase() === sel);
  };
  el.querySelectorAll = function (sel) {
    if (sel === "input[data-campo]") return this.children.filter((c) => c.dataset && c.dataset.campo);
    return [];
  };
  el.appendChild = function (child) { child.parentElement = this; this.children.push(child); return child; };
  el.remove = function () { if (this.parentElement) { const i = this.parentElement.children.indexOf(this); if (i > -1) this.parentElement.children.splice(i, 1); } };
  el.focus = function () {};
  el.click = function () { this.dispatchEvent({ type: "click", target: this }); };
  return el;
}

function registrarElemento(id, tag = "div") {
  const el = criarElemento(id, tag);
  document.registerElement(el);
  return el;
}

test("H6 - login define PIN no sessionStorage", async () => {
  resetGlobalState();
  mockResposta("/api/saude", {
    ok: true, status: 200,
    json: async () => ({ banco: "ok", schema_versao: 3, llm: "indisponível", ocr: "pasta_ocr_ausente" }),
  });
  app.setupLogin();
  const btnLogin = document.getElementById("btn-login");
  const inputPin = document.getElementById("input-pin");
  inputPin.value = "123456";
  await btnLogin.dispatchEvent({ type: "click", target: btnLogin });
  assert.strictEqual(window.sessionStorage.getItem("cupons_pin"), "123456");
});

test("H6 - salvarCupom envia corpo com grupo minúsculo", async () => {
  resetGlobalState();
  mockResposta("/api/cupons", {
    ok: true, status: 201,
    json: async () => ({ id: 42, mensagem: "Cupom salvo com sucesso" }),
  });
  app.estado.itens = [{ descricao: "Arroz", quantidade: 1, preco_unitario: 10.0, preco_total: 10.0, grupo: "casa" }];
  app.estado.somaItens = 10.0;
  document.getElementById("input-total-cupom").value = "10,00";
  document.getElementById("input-estabelecimento").value = "Mercado Teste";
  document.getElementById("input-pagador").value = "igor";
  document.getElementById("input-data").value = "2026-11-18";
  let err = null;
  try { await app.salvarCupom(); } catch (e) { err = e; }
  assert.ok(!err, `salvarCupom não deve lançar: ${err}`);
  // salvarCupom dispara POST /api/cupons e depois GET (carregarHistorico);
  // isolar o POST pelo método.
  const req = requisicoes.find(
    (r) => r.url === "/api/cupons" && r.options && r.options.method === "POST"
  );
  assert.ok(req, "POST /api/cupons deve ser chamado");
  assert.strictEqual(req.options.method, "POST");
  const corpoStr = req.body || (req.options && req.options.body);
  assert.ok(corpoStr, "corpo deve existir");
  const corpo = JSON.parse(corpoStr);
  assert.strictEqual(corpo.itens[0].grupo, "casa", "grupo minúsculo (contrato H1)");
});

test("H6 - remover item remove exatamente 1", () => {
  resetGlobalState();
  app.estado.itens = [
    { descricao: "A", preco_total: 1.0, grupo: "casa" },
    { descricao: "B", preco_total: 2.0, grupo: "igor" },
    { descricao: "C", preco_total: 3.0, grupo: "renan" },
  ];
  app.removerItem(1);
  assert.strictEqual(app.estado.itens.length, 2);
  assert.strictEqual(app.estado.itens[0].descricao, "A");
  assert.strictEqual(app.estado.itens[1].descricao, "C");
});

test("H6 - saldos com grupos minúsculos", () => {
  resetGlobalState();
  document.getElementById("input-pagador").value = "igor";
  document.getElementById("input-total-cupom").value = "100,00";
  app.estado.itens = [
    { descricao: "Arroz", preco_total: 40.0, grupo: "casa" },
    { descricao: "Cerveja", preco_total: 30.0, grupo: "igor" },
    { descricao: "Vinho", preco_total: 30.0, grupo: "renan" },
  ];
  app.estado.somaItens = 100.0;
  app.atualizarResumo();
  assert.ok(app.estado.saldoIgor >= 49.9 && app.estado.saldoIgor <= 50.1, `saldoIgor: ${app.estado.saldoIgor}`);
  assert.ok(app.estado.saldoRenan <= -50.0 && app.estado.saldoRenan >= -50.1, `saldoRenan: ${app.estado.saldoRenan}`);
});
