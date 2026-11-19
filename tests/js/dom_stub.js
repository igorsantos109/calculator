// ============================================================
// DOM Stub mínimo para testar app.js em Node (sem jsdom)
// ============================================================

class MockElement {
  constructor(tag, id = "") {
    this.tagName = tag.toUpperCase();
    this.id = id;
    this.className = "";
    this.hidden = false;
    this.value = "";
    this.textContent = "";
    this.innerHTML = "";
    this.dataset = {};
    this.attributes = {};
    this.children = [];
    this.eventListeners = new Map();
    this.parentElement = null;
    this.style = {};
    this.required = false;
  }

  setAttribute(name, value) {
    this.attributes[name] = value;
    if (name === "id") this.id = value;
    if (name === "class") this.className = value;
    if (name === "data-campo") this.dataset.campo = value;
    if (name === "data-idx") this.dataset.idx = value;
    if (name === "required") this.required = true;
  }

  getAttribute(name) {
    return this.attributes[name];
  }

  addEventListener(event, handler) {
    if (!this.eventListeners.has(event)) {
      this.eventListeners.set(event, []);
    }
    this.eventListeners.get(event).push(handler);
  }

  removeEventListener(event, handler) {
    if (this.eventListeners.has(event)) {
      const handlers = this.eventListeners.get(event);
      const idx = handlers.indexOf(handler);
      if (idx > -1) handlers.splice(idx, 1);
    }
  }

  dispatchEvent(event) {
    const handlers = this.eventListeners.get(event.type);
    if (handlers) {
      handlers.forEach(h => h(event));
    }
  }

  closest(selector) {
    if (selector === ".btn-remover" && this.className.includes("btn-remover")) return this;
    return this.parentElement;
  }

  // classList mínimo para mostrarErro/mostrarSucesso
  get classList() {
    return {
      add: (c) => { this.className = this.className ? this.className + " " + c : c; },
      remove: (c) => { this.className = this.className.split(" ").filter(x => x !== c).join(" ").trim(); },
      contains: (c) => this.className.split(" ").includes(c),
      toggle: (c) => { this.className = this.className.split(" ").includes(c) ? this.className.split(" ").filter(x => x !== c).join(" ").trim() : this.className ? this.className + " " + c : c; },
    };
  }

  querySelector(selector) {
    if (selector.startsWith("#")) {
      const id = selector.slice(1);
      return globalThis.document.getElementById(id);
    }
    return this.children.find(c => c.tagName.toLowerCase() === selector || 
      (selector.startsWith(".") && c.className.includes(selector.slice(1))) ||
      (selector.startsWith("[data-") && c.dataset && selector.includes(c.dataset)));
  }

  querySelectorAll(selector) {
    if (selector === "input[data-campo]") {
      return this.children.filter(c => c.dataset && c.dataset.campo);
    }
    if (selector === ".btn-remover") {
      return this.children.filter(c => c.className && c.className.includes("btn-remover"));
    }
    return [];
  }

  appendChild(child) {
    child.parentElement = this;
    this.children.push(child);
    return child;
  }

  remove() {
    if (this.parentElement) {
      const idx = this.parentElement.children.indexOf(this);
      if (idx > -1) this.parentElement.children.splice(idx, 1);
    }
  }

  focus() {}

  click() {
    this.dispatchEvent({ type: "click", target: this });
  }
}

class MockInput extends MockElement {
  constructor(id = "") {
    super("input", id);
    this.type = "text";
  }
}

class MockSelect extends MockElement {
  constructor(id = "") {
    super("select", id);
    this.options = [];
  }
}

class MockButton extends MockElement {
  constructor(id = "") {
    super("button", id);
  }
}

class MockTextarea extends MockElement {
  constructor(id = "") {
    super("textarea", id);
  }
}

class MockModal extends MockElement {
  constructor(id = "") {
    super("div", id);
    this.hidden = true;
  }
}

class MockDocument {
  constructor() {
    this.elements = new Map();
    this.body = new MockElement("body");
    this.head = new MockElement("head");
    this.documentElement = new MockElement("html");
    this.activeElement = this.body;
    this.eventListeners = new Map();
  }

  createElement(tag) {
    const el = new MockElement(tag);
    return el;
  }

  getElementById(id) {
    return this.elements.get(id) || null;
  }

  registerElement(el) {
    if (el.id) this.elements.set(el.id, el);
  }

  addEventListener(event, handler) {
    if (!this.eventListeners) this.eventListeners = new Map();
    if (!this.eventListeners.has(event)) this.eventListeners.set(event, []);
    this.eventListeners.get(event).push(handler);
  }

  dispatchEvent(event) {
    if (this.eventListeners && this.eventListeners.has(event.type)) {
      this.eventListeners.get(event.type).forEach(h => h(event));
    }
  }
}

// Global stub
const document = new MockDocument();
globalThis.document = document;
globalThis.window = {
  localStorage: {
    store: {},
    getItem(key) { return this.store[key] || null; },
    setItem(key, val) { this.store[key] = val; },
    removeItem(key) { delete this.store[key]; }
  },
  sessionStorage: {
    store: {},
    getItem(key) { return this.store[key] || null; },
    setItem(key, val) { this.store[key] = val; },
    removeItem(key) { delete this.store[key]; }
  },
  navigator: {
    clipboard: {
      writeText(text) {
        return Promise.resolve();
      }
    }
  },
  alert: (msg) => { console.log("[ALERT]", msg); },
  setTimeout: setTimeout,
  clearTimeout: clearTimeout,
  location: { origin: "http://localhost:8080" }
};

// Expor localStorage e sessionStorage globalmente
globalThis.localStorage = globalThis.window.localStorage;
globalThis.sessionStorage = globalThis.window.sessionStorage;

// Helper to create standard elements
function createStandardElements() {
  const ids = [
    "dropzone", "input-imagem", "area-texto-ocr", "input-texto-ocr",
    "btn-colar-texto", "btn-modo-manual", "btn-extrair-texto",
    "btn-adicionar-item", "btn-salvar", "btn-novo", "btn-teste-llm",
    "btn-tema", "tabela-itens", "corpo-tabela-itens", "soma-itens",
    "input-total-cupom", "input-estabelecimento", "input-pagador", "input-data", "selo-conciliacao",
    "saldo-igor", "saldo-renan", "secao-itens", "secao-resumo", "secao-acoes",
    "lista-historico", "input-busca", "filtro-conciliado", "filtro-divergente",
    "mensagem-salvo", "mensagem-erro", "area-avisos", "status-conexao",
    "modal-detalhes", "modal-relatorio", "modal-login", "input-pin",
    "erro-pin", "btn-login", "input-nova-descricao", "input-nova-quantidade",
    "input-novo-unitario", "input-novo-total", "input-novo-grupo",
    "btn-fechar-modal", "btn-fechar-modal2", "btn-fechar-relatorio",
    "btn-fechar-relatorio2", "btn-copiar-relatorio", "btn-copiar-relatorio-periodo",
    "btn-gerar-relatorio", "rel-de", "rel-ate", "resultado-relatorio",
    "det-estabelecimento", "det-data", "det-total", "det-pagador",
    "det-igor", "det-renan", "det-saldo-igor", "det-saldo-renan", "det-itens"
  ];

  ids.forEach(id => {
    let el;
    if (id.startsWith("input-") || id === "input-pin" || id.startsWith("input-nova") || id === "rel-de" || id === "rel-ate") {
      el = new MockInput(id);
    } else if (id.startsWith("select") || id === "input-novo-grupo" || id === "input-pagador" || id === "filtro-conciliado" || id === "filtro-divergente") {
      el = new MockSelect(id);
    } else if (id.startsWith("btn-")) {
      el = new MockButton(id);
    } else if (id === "input-texto-ocr") {
      el = new MockTextarea(id);
    } else if (id.startsWith("modal-")) {
      el = new MockModal(id);
    } else {
      el = new MockElement("div", id);
    }
    document.registerElement(el);
  });

  // Set up DOM hierarchy
  const body = document.body;
  body.children = Array.from(document.elements.values());
  
  // Special elements
  document.getElementById("corpo-tabela-itens").parentElement = document.getElementById("tabela-itens");
  document.getElementById("tabela-itens").parentElement = body;
  document.getElementById("modal-login").parentElement = body;
  document.getElementById("modal-detalhes").parentElement = body;
  document.getElementById("modal-relatorio").parentElement = body;
}

// Initialize
createStandardElements();

// Mock fetch
globalThis.fetch = async (url, options = {}) => {
  // Mock login
  if (url === "/api/saude") {
    if (options.headers && options.headers.Authorization) {
      const pin = options.headers.Authorization.replace("Bearer ", "");
      if (pin.length >= 6) {
        return {
          ok: true,
          json: async () => ({ banco: "ok", schema_versao: 3, llm: "indisponível", ocr: "pasta_ocr_ausente" })
        };
      }
    }
    return {
      ok: false,
      status: 401,
      json: async () => ({ codigo: "pin_obrigatorio", mensagem: "PIN de acesso é obrigatório" })
    };
  }
  
  // Mock extrair
  if (url === "/api/extrair") {
    return {
      ok: true,
      json: async () => ({
        itens: [{ descricao: "Leite 1L", quantidade: 2, preco_unitario: 5.50, preco_total: 11.00 }],
        avisos: [],
        soma_itens: 11.00,
        total_cupom: 11.00
      })
    };
  }
  
  // Mock salvar cupom
  if (url === "/api/cupons" && options.method === "POST") {
    return {
      ok: true,
      status: 201,
      json: async () => ({ id: 1, mensagem: "Cupom salvo com sucesso" })
    };
  }
  
  // Mock listar cupons
  if (url === "/api/cupons" && options.method === "GET") {
    return {
      ok: true,
      json: async () => ({ cupons: [], total: 0 })
    };
  }
  
  // Mock detalhes
  if (url.startsWith("/api/cupons/") && options.method === "GET") {
    return {
      ok: true,
      json: async () => ({
        id: 1,
        receita: { data_compra: "2026-11-18", estabelecimento: "Mercado", total_cupom: 11.00, imagem_path: "", pagador: "igor" },
        itens: [{ item: { descricao: "Leite 1L", quantidade: 2, preco_unitario: 5.50, preco_total: 11.00 }, grupo: "casa" }]
      })
    };
  }
  
  // Mock relatório
  if (url.startsWith("/api/relatorio")) {
    return {
      ok: true,
      json: async () => ({
        periodo_de: "",
        periodo_ate: "",
        cupons: [],
        total_cupons: 0,
        total_geral: 0,
        casa: 0,
        igor: 0,
        renan: 0
      })
    };
  }
  
  return {
    ok: false,
    status: 404,
    json: async () => ({ codigo: "rota_nao_encontrada", mensagem: "Rota não encontrada" })
  };
};

// Export for Node
if (typeof module !== "undefined" && module.exports) {
  module.exports = { document, MockElement, MockInput, MockSelect, MockButton, MockTextarea, MockModal };
}