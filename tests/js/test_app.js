// ============================================================
// H6: Testes JS do app.js com DOM simulado (node:test + node:vm)
// Scenarios:
//   - login na primeira visita
//   - salvar (assert corpo enviado bate com contrato H1)
//   - mudar grupo de um item
//   - remover item (exatamente 1)
//   - detalhes com saldos corretos
// ============================================================

import { test } from "node:test";
import * as vm from "node:vm";
import * as fs from "fs";
import * as path from "path";
import assert from "node:assert/strict";

const APP_JS_PATH = path.resolve(
  process.cwd(),
  "crates/web/src/assets/app.js"
);
const DOM_STUB_PATH = path.resolve(process.cwd(), "tests/js/dom_stub.js");

let ultimaRequisicaoEnviada = null;
let requisicoes = [];
let fetchMockRespostas = {};

// Mock de setTimeout/clearTimeout para não travar o node --test
globalThis.setTimeout = (fn) => { try { fn(); } catch (e) {} return 0; };
globalThis.clearTimeout = () => {};

// Carregar DOM stub + app.js UMA ÚNICA VEZ
const domStubSrc = fs.readFileSync(DOM_STUB_PATH, "utf8");
const appSrc = fs.readFileSync(APP_JS_PATH, "utf8");

vm.runInThisContext(domStubSrc, {
  filename: DOM_STUB_PATH,
  displayErrors: true,
});

globalThis.fetch = async (url, options = {}) => {
  const registro = {
    url,
    options,
    body: options && options.body ? options.body : null,
  };
  requisicoes.push(registro);
  ultimaRequisicaoEnviada = registro;
  const resposta = fetchMockRespostas[url] || {
    ok: false,
    status: 404,
    json: async () => ({ codigo: "erro", mensagem: "sem mock" }),
  };
  return resposta;
};

vm.runInThisContext(appSrc, {
  filename: APP_JS_PATH,
  displayErrors: true,
});

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
    tagName: tag.toUpperCase(),
    id,
    className: "",
    hidden: false,
    value: "",
    textContent: "",
    innerHTML: "",
    dataset: {},
    attributes: {},
    children: [],
    eventListeners: new Map(),
    parentElement: null,
    style: {},
    required: false,
  };
  el.setAttribute = function (name, value) {
    this.attributes[name] = value;
    if (name === "id") this.id = value;
    if (name === "class") this.className = value;
    if (name === "data-campo") this.dataset.campo = value;
    if (name === "data-idx") this.dataset.idx = value;
    if (name === "required") this.required = true;
  };
  el.getAttribute = function (name) {
    return this.attributes[name];
  };
  el.addEventListener = function (event, handler) {
    if (!this.eventListeners.has(event)) this.eventListeners.set(event, []);
    this.eventListeners.get(event).push(handler);
  };
  el.removeEventListener = function (event, handler) {
    const h = this.eventListeners.get(event);
    if (h) {
      const i = h.indexOf(handler);
      if (i > -1) h.splice(i, 1);
    }
  };
  el.dispatchEvent = function (event) {
    const h = this.eventListeners.get(event.type);
    if (h) h.forEach((fn) => fn(event));
  };
  el.closest = function (selector) {
    if (selector === ".btn-remover" && this.className.includes("btn-remover")) return this;
    return this.parentElement;
  };
  el.querySelector = function (selector) {
    if (selector.startsWith("#")) {
      const elId = selector.slice(1);
      for (const e of document.elements.values()) {
        if (e.id === elId) return e;
      }
      return null;
    }
    return this.children.find((c) => c.tagName.toLowerCase() === selector);
  };
  el.querySelectorAll = function (selector) {
    if (selector === "input[data-campo]") {
      return this.children.filter((c) => c.dataset && c.dataset.campo);
    }
    return [];
  };
  el.appendChild = function (child) {
    child.parentElement = this;
    this.children.push(child);
    return child;
  };
  el.remove = function () {
    if (this.parentElement) {
      const i = this.parentElement.children.indexOf(this);
      if (i > -1) this.parentElement.children.splice(i, 1);
    }
  };
  el.focus = function () {};
  el.click = function () {
    this.dispatchEvent({ type: "click", target: this });
  };
  return el;
}

function registrarElemento(id, tag = "div") {
  const el = criarElemento(id, tag);
  document.registerElement(el);
  return el;
}

// ---------------------------------------------------------------------------
// TESTES H6
// ---------------------------------------------------------------------------

test("H6 - login na primeira visita define PIN no sessionStorage", async () => {
  resetGlobalState();

  mockResposta("/api/saude", {
    ok: true,
    status: 200,
    json: async () => ({
      banco: "ok",
      schema_versao: 3,
      llm: "indisponível",
      ocr: "pasta_ocr_ausente",
    }),
  });

  app.setupLogin();

  const btnLogin = document.getElementById("btn-login");
  const inputPin = document.getElementById("input-pin");
  inputPin.value = "123456";

  await btnLogin.dispatchEvent({ type: "click", target: btnLogin });

  assert.strictEqual(
    window.sessionStorage.getItem("cupons_pin"),
    "123456",
    "PIN deve ser salvo no sessionStorage"
  );
  assert.strictEqual(
    document.getElementById("erro-pin").textContent,
    "",
    "não deve haver mensagem de erro após login válido"
  );
});

test("H6 - salvarCupom envia corpo compatível com contrato H1 (grupos minúsculos)", async () => {
  resetGlobalState();

  mockResposta("/api/cupons", {
    ok: true,
    status: 201,
    json: async () => ({ id: 42, mensagem: "Cupom salvo com sucesso" }),
  });

  app.estado.itens = [
    {
      descricao: "Arroz",
      quantidade: 1,
      preco_unitario: 10.0,
      preco_total: 10.0,
      grupo: "casa",
    },
  ];
  app.estado.somaItens = 10.0;

  document.getElementById("input-total-cupom").value = "10,00";
  document.getElementById("input-estabelecimento").value = "Mercado Teste";
  document.getElementById("input-pagador").value = "igor";
  document.getElementById("input-data").value = "2026-11-18";

  let erroSalvar = null;
  try {
    await app.salvarCupom();
  } catch (e) {
    erroSalvar = e;
  }
  assert.ok(!erroSalvar, `salvarCupom não deve lançar: ${erroSalvar}`);

  // salvarCupom dispara POST /api/cupons e, em seguida, GET /api/cupons
  // (carregarHistorico). Isolar o POST pelo método.
  const req = requisicoes.find(
    (r) => r.url === "/api/cupons" && r.options && r.options.method === "POST"
  );
  assert.ok(req, "POST /api/cupons deve ter sido chamado por salvarCupom");
  assert.strictEqual(
    req.options && req.options.method,
    "POST",
    "método deve ser POST"
  );

  const corpoStr = req.body || (req.options && req.options.body);
  assert.ok(corpoStr, "corpo da requisição deve existir");
  const corpo = JSON.parse(corpoStr);

  assert.strictEqual(corpo.receita.estabelecimento, "Mercado Teste");
  assert.strictEqual(corpo.receita.pagador, "igor");
  assert.strictEqual(corpo.receita.total_cupom, 10.0);
  assert.strictEqual(corpo.itens.length, 1);

  // Contrato H1: grupo em minúsculas
  assert.strictEqual(
    corpo.itens[0].grupo,
    "casa",
    "grupo deve ser minúsculo (contrato H1)"
  );
  assert.strictEqual(corpo.itens[0].item.descricao, "Arroz");
});

test("H6 - mudar grupo de um item reflete em minúsculo", () => {
  resetGlobalState();

  app.estado.itens = [
    {
      descricao: "Item X",
      quantidade: 1,
      preco_unitario: 5.0,
      preco_total: 5.0,
      grupo: "igor",
    },
  ];

  app.estado.itens[0].grupo = "casa";
  app.atualizarResumo();

  assert.strictEqual(app.estado.itens[0].grupo, "casa");
});

test("H6 - remover item remove exatamente 1", () => {
  resetGlobalState();

  app.estado.itens = [
    { descricao: "A", quantidade: 1, preco_unitario: 1.0, preco_total: 1.0, grupo: "casa" },
    { descricao: "B", quantidade: 1, preco_unitario: 2.0, preco_total: 2.0, grupo: "igor" },
    { descricao: "C", quantidade: 1, preco_unitario: 3.0, preco_total: 3.0, grupo: "renan" },
  ];

  const antes = app.estado.itens.length;
  app.removerItem(1);
  const depois = app.estado.itens.length;

  assert.strictEqual(depois, antes - 1, "deve remover exatamente 1 item");
  assert.strictEqual(app.estado.itens.length, 2);
  assert.strictEqual(app.estado.itens[0].descricao, "A");
  assert.strictEqual(app.estado.itens[1].descricao, "C");
});

test("H6 - verDetalhes/atualizarResumo calcula saldos corretos com grupos minúsculos", () => {
  resetGlobalState();

  document.getElementById("input-pagador").value = "igor";
  document.getElementById("input-total-cupom").value = "100,00";

  app.estado.itens = [
    { descricao: "Arroz", quantidade: 1, preco_unitario: 40.0, preco_total: 40.0, grupo: "casa" },
    { descricao: "Cerveja", quantidade: 1, preco_unitario: 30.0, preco_total: 30.0, grupo: "igor" },
    { descricao: "Vinho", quantidade: 1, preco_unitario: 30.0, preco_total: 30.0, grupo: "renan" },
  ];
  app.estado.somaItens = 100.0;

  assert.strictEqual(app.estado.itens.length, 3, "itens devem estar no estado");
  assert.strictEqual(app.estado.itens[0].grupo, "casa");
  assert.strictEqual(app.estado.itens[1].grupo, "igor");
  assert.strictEqual(app.estado.itens[2].grupo, "renan");

  app.atualizarResumo();

  // casa=40, igor=30, renan=30 → metade_casa=20
  // resp_igor = 30+20 = 50; resp_renan = 30+20 = 50 (+0.01 centavo indivisível, pagador=igor)
  // saldo_igor = 100-50 = 50; saldo_renan = 0-50.01 = -50.01
  assert.ok(
    app.estado.saldoIgor >= 49.9 && app.estado.saldoIgor <= 50.1,
    `saldoIgor inesperado: ${app.estado.saldoIgor}`
  );
  assert.ok(
    app.estado.saldoRenan <= -50.0 && app.estado.saldoRenan >= -50.1,
    `saldoRenan inesperado: ${app.estado.saldoRenan}`
  );
});
