/* ============================================================
   Leitor de Cupons Fiscais — Aplicação frontend (página única)
   Sem CDN. Funcional sem internet. Tema claro/escuro.
   Formato brasileiro: R$ 1.234,56 | vírgula como decimal
   ============================================================ */

(function () {
  // ---------- Config ----------
  const API_BASE = ""; // mesma origem

  // ---------- Estado ----------
  let estado = {
    itens: [], // { descricao, quantidade, preco_unitario, preco_total, grupo }
    totalCupom: 0,
    pagador: "Igor",
    data: "",
    modo: "manual", // manual | ocr | texto
    saldoIgor: 0,
    saldoRenan: 0,
    somaItens: 0,
    avisos: [],
    historico: [],
    cupomSelecionadoId: null,
    tema: localStorage.getItem("cupons_tema") || "claro",
    conexaoOk: false,
  };

  // ---------- PIN / Autenticação ----------
  function getPin() {
    return sessionStorage.getItem("cupons_pin") || "";
  }

  function setPin(pin) {
    sessionStorage.setItem("cupons_pin", pin);
  }

  function clearPin() {
    sessionStorage.removeItem("cupons_pin");
  }

  function authHeaders() {
    const pin = getPin();
    return pin ? { "Authorization": `Bearer ${pin}` } : {};
  }

  function authHeadersJson() {
    const h = authHeaders();
    h["Content-Type"] = "application/json";
    return h;
  }

  // ---------- Elementos DOM ----------
  const els = {
    dropzone: document.getElementById("dropzone"),
    inputImagem: document.getElementById("input-imagem"),
    areaTextoOCR: document.getElementById("area-texto-ocr"),
    inputTextoOCR: document.getElementById("input-texto-ocr"),
    btnColarTexto: document.getElementById("btn-colar-texto"),
    btnModoManual: document.getElementById("btn-modo-manual"),
    btnExtrairTexto: document.getElementById("btn-extrair-texto"),
    btnAdicionarItem: document.getElementById("btn-adicionar-item"),
    btnSalvar: document.getElementById("btn-salvar"),
    btnNovo: document.getElementById("btn-novo"),
    btnTesteLLM: document.getElementById("btn-teste-llm"),
    btnTema: document.getElementById("btn-tema"),
    tabela: document.getElementById("tabela-itens"),
    corpo: document.getElementById("corpo-tabela-itens"),
    somaItensEl: document.getElementById("soma-itens"),
    inputTotalCupom: document.getElementById("input-total-cupom"),
    inputPagador: document.getElementById("input-pagador"),
    inputData: document.getElementById("input-data"),
    selo: document.getElementById("selo-conciliacao"),
    saldoIgorEl: document.getElementById("saldo-igor"),
    saldoRenanEl: document.getElementById("saldo-renan"),
    secaoItens: document.getElementById("secao-itens"),
    secaoResumo: document.getElementById("secao-resumo"),
    secaoAcoes: document.getElementById("secao-acoes"),
    listaHistorico: document.getElementById("lista-historico"),
    inputBusca: document.getElementById("input-busca"),
    filtroConciliado: document.getElementById("filtro-conciliado"),
    filtroDivergente: document.getElementById("filtro-divergente"),
    msgSalvo: document.getElementById("mensagem-salvo"),
    msgErro: document.getElementById("mensagem-erro"),
    areaAvisos: document.getElementById("area-avisos"),
    statusConexao: document.getElementById("status-conexao"),
    modalDetalhes: document.getElementById("modal-detalhes"),
    modalRelatorio: document.getElementById("modal-relatorio"),
    modalLogin: document.getElementById("modal-login"),
    inputPin: document.getElementById("input-pin"),
    erroPin: document.getElementById("erro-pin"),
    btnLogin: document.getElementById("btn-login"),
    inputNovaDescricao: document.getElementById("input-nova-descricao"),
    inputNovaQuantidade: document.getElementById("input-nova-quantidade"),
    inputNovoUnitario: document.getElementById("input-novo-unitario"),
    inputNovoTotal: document.getElementById("input-novo-total"),
    inputNovoGrupo: document.getElementById("input-novo-grupo"),
    btnFecharModal: document.getElementById("btn-fechar-modal"),
    btnFecharModal2: document.getElementById("btn-fechar-modal2"),
    btnFecharRelatorio: document.getElementById("btn-fechar-relatorio"),
    btnFecharRelatorio2: document.getElementById("btn-fechar-relatorio2"),
    btnCopiarRelatorio: document.getElementById("btn-copiar-relatorio"),
    btnCopiarRelatorioPeriodo: document.getElementById("btn-copiar-relatorio-periodo"),
    btnGerarRelatorio: document.getElementById("btn-gerar-relatorio"),
    relDe: document.getElementById("rel-de"),
    relAte: document.getElementById("rel-ate"),
    resultadoRelatorio: document.getElementById("resultado-relatorio"),
  };

  // ---------- Inicialização ----------
  document.addEventListener("DOMContentLoaded", init);

  async function init() {
    aplicarTema(estado.tema);
    
    // Verificar se tem PIN salvo
    const pin = getPin();
    if (!pin) {
      mostrarLogin();
      return;
    }
    
    await verificarSaude();
    carregarHistorico();
    setupEventos();
  }

  function setupEventos() {
    // Tema
    els.btnTema.addEventListener("click", alternarTema);

    // Dropzone / arrastar
    els.dropzone.addEventListener("click", () => els.inputImagem.click());
    els.inputImagem.addEventListener("change", (e) => processarArquivo(e.target.files[0]));
    els.dropzone.addEventListener("dragover", (e) => { e.preventDefault(); els.dropzone.classList.add("arrastando"); });
    els.dropzone.addEventListener("dragleave", () => els.dropzone.classList.remove("arrastando"));
    els.dropzone.addEventListener("drop", (e) => {
      e.preventDefault();
      els.dropzone.classList.remove("arrastando");
      const files = e.dataTransfer.files;
      if (files.length) processarArquivo(files[0]);
    });

    // Login
    els.btnLogin.addEventListener("click", fazerLogin);
    els.inputPin.addEventListener("keypress", (e) => { if (e.key === "Enter") fazerLogin(); });

    // Modo colar texto / manual
    els.btnColarTexto.addEventListener("click", () => {
      els.areaTextoOCR.classList.toggle("oculto");
      if (!els.areaTextoOCR.classList.contains("oculto")) els.inputTextoOCR.focus();
    });
    els.btnModoManual.addEventListener("click", modoManual);
    els.btnExtrairTexto.addEventListener("click", extrairItensTextoOCR);
    els.btnTesteLLM.addEventListener("click", testarConexaoLLM);
    els.btnSalvar.addEventListener("click", salvarCupom);
    els.btnNovo.addEventListener("click", novoCupom);

    // Adicionar item manual
    els.btnAdicionarItem.addEventListener("click", adicionarItemManual);
    [els.inputNovaDescricao, els.inputNovaQuantidade, els.inputNovoUnitario, els.inputNovoTotal].forEach((inp) => {
      if (inp) inp.addEventListener("keypress", (e) => { if (e.key === "Enter") adicionarItemManual(); });
    });

    // Totais ao vivo
    els.inputTotalCupom.addEventListener("input", atualizarResumo);
    els.inputPagador.addEventListener("change", atualizarResumo);
    els.inputData.addEventListener("input", (e) => { estado.data = e.target.value; });

    // Histórico
    els.inputBusca.addEventListener("input", renderHistorico);
    els.filtroConciliado.addEventListener("change", renderHistorico);
    els.filtroDivergente.addEventListener("change", renderHistorico);

    // Modais
    els.btnFecharModal.addEventListener("click", () => { els.modalDetalhes.hidden = true; });
    els.btnFecharModal2.addEventListener("click", () => { els.modalDetalhes.hidden = true; });
    els.btnFecharRelatorio.addEventListener("click", () => { els.modalRelatorio.hidden = true; });
    els.btnFecharRelatorio2.addEventListener("click", () => { els.modalRelatorio.hidden = true; });
    els.btnCopiarRelatorio.addEventListener("click", copiarRelatorioDetalhes);
    els.btnCopiarRelatorioPeriodo.addEventListener("click", copiarRelatorioPeriodo);
    els.btnGerarRelatorio.addEventListener("click", gerarRelatorioPeriodo);

    // Teclado: Esc fecha modais
    document.addEventListener("keydown", (e) => {
      if (e.key === "Escape") { els.modalDetalhes.hidden = true; els.modalRelatorio.hidden = true; }
    });
  }

  function mostrarLogin() {
    els.modalLogin.hidden = false;
    els.inputPin.value = "";
    els.erroPin.textContent = "";
    els.inputPin.focus();
  }

  function esconderLogin() {
    els.modalLogin.hidden = true;
  }

  async function fazerLogin() {
    const pin = els.inputPin.value.trim();
    if (pin.length < 6) {
      els.erroPin.textContent = "PIN deve ter pelo menos 6 caracteres";
      return;
    }
    els.btnLogin.disabled = true;
    els.btnLogin.textContent = "Verificando...";
    
    try {
      const resp = await fetch(`${API_BASE}/api/saude`, {
        headers: { "Authorization": `Bearer ${pin}` }
      });
      if (resp.ok) {
        setPin(pin);
        esconderLogin();
        await verificarSaude();
        carregarHistorico();
        setupEventos();
      } else {
        const err = await resp.json();
        els.erroPin.textContent = err.mensagem || "PIN inválido";
      }
    } catch (e) {
      els.erroPin.textContent = "Não foi possível conectar ao servidor";
    } finally {
      els.btnLogin.disabled = false;
      els.btnLogin.textContent = "Entrar";
    }
  }

  // ---------- Tema ----------
  function aplicarTema(tema) {
    estado.tema = tema;
    localStorage.setItem("cupons_tema", tema);
    if (tema === "escuro") document.documentElement.setAttribute("data-tema", "escuro");
    else document.documentElement.removeAttribute("data-tema");
    els.btnTema.textContent = tema === "escuro" ? "☀️" : "🌙";
  }
  function alternarTema() { aplicarTema(estado.tema === "claro" ? "escuro" : "claro"); }

  // ---------- Saúde ----------
  async function verificarSaude() {
    try {
      const resp = await fetch(`${API_BASE}/api/saude`, {
        headers: authHeaders(),
      });
      const dados = await resp.json();
      estado.conexaoOk = resp.ok;
      els.statusConexao.textContent = dados.llm === "alcançável" ? "Servidor OK" : "Servidor indisponível";
      els.statusConexao.className = `status-badge ${dados.llm === "alcançável" ? "status-disponivel" : "status-indisponivel"}`;
    } catch (e) {
      estado.conexaoOk = false;
      els.statusConexao.textContent = "Servidor indisponível";
      els.statusConexao.className = "status-badge status-indisponivel";
    }
  }

  async function testarConexaoLLM() {
    els.btnTesteLLM.disabled = true;
    els.btnTesteLLM.textContent = "Testando...";
    try {
      const resp = await fetch(`${API_BASE}/api/extrair`, {
        method: "POST",
        headers: authHeadersJson(),
        body: JSON.stringify({ texto_ocr: "teste de conexão", total_cupom: 0 }),
      });
      if (resp.ok) {
        alert("Conexão com LLM: OK");
      } else {
        const err = await resp.json();
        alert("Falha na conexão com LLM: " + err.codigo);
      }
    } catch (e) {
      alert("Não foi possível alcançar o servidor: " + e.message);
    } finally {
      els.btnTesteLLM.disabled = false;
      els.btnTesteLLM.textContent = "Testar LLM";
    }
  }

  // ---------- Processar imagem (arquivo) ----------
  async function processarArquivo(arquivo) {
    if (!arquivo) return;
    const tipo = arquivo.type.toLowerCase();
    if (!["image/png", "image/jpeg", "image/jpg"].includes(tipo)) {
      mostrarErro("Tipo de arquivo inválido. Use apenas PNG ou JPG.");
      return;
    }
    if (arquivo.size > 10 * 1024 * 1024) {
      mostrarErro("Arquivo muito grande. Limite: 10 MB.");
      return;
    }

    // Nome do arquivo: sanitizar (não há upload real de imagem no build mínimo;
    // usamos o modo manual/texto como caminho principal)
    const nome = arquivo.name.replace(/[^a-zA-Z0-9._-]/g, "_");

    // OCR real não disponível offline → ir direto para modo manual com nome do arquivo
    mostrarErro("OCR real não disponível neste build (sem rede para modelos). Use Modo manual ou Colar texto.");
    modoManual();
    els.inputNovaDescricao.focus();
  }

  // ---------- Modo manual ----------
  function modoManual() {
    estado.modo = "manual";
    els.areaTextoOCR.classList.add("oculto");
    limparItens();
    secaoVisiveis(true);
    els.inputNovaDescricao.focus();
  }

  // ---------- Extrair itens do texto OCR ----------
  async function extrairItensTextoOCR() {
    const texto = els.inputTextoOCR.value.trim();
    if (!texto) { mostrarErro("Informe o texto OCR."); return; }
    const total = parseBR(els.inputTotalCupom.value) || 0;

    els.btnExtrairTexto.disabled = true;
    els.btnExtrairTexto.textContent = "Extraindo...";
    try {
      const resp = await fetch(`${API_BASE}/api/extrair`, {
        method: "POST",
        headers: authHeadersJson(),
        body: JSON.stringify({ texto_ocr: texto, total_cupom: total }),
      });
      if (!resp.ok) {
        const err = await resp.json();
        throw new Error(err.codigo || "Falha ao extrair");
      }
      const dados = await resp.json();
      estado.itens = dados.itens.map((it) => ({
        descricao: it.descricao,
        quantidade: it.quantidade,
        preco_unitario: it.preco_unitario,
        preco_total: it.preco_total,
        grupo: "",
      }));
      estado.somaItens = dados.soma_itens;
      estado.avisos = dados.avisos || [];
      els.inputTotalCupom.value = formatBRL(total);
      renderItens();
      renderAvisos();
      atualizarResumo();
      secaoVisiveis(true);
      els.areaTextoOCR.classList.add("oculto");
    } catch (e) {
      mostrarErro(e.message || "Erro ao consultar LLM.");
    } finally {
      els.btnExtrairTexto.disabled = false;
      els.btnExtrairTexto.textContent = "Extrair itens do texto";
    }
  }

  // ---------- Itens ----------
  function limparItens() {
    estado.itens = [];
    estado.avisos = [];
    estado.somaItens = 0;
    renderItens();
    renderAvisos();
    atualizarResumo();
  }

  function adicionarItemManual() {
    const descricao = (document.getElementById("input-nova-descricao").value || "").trim();
    const quantidade = parseBR(document.getElementById("input-nova-quantidade").value);
    const unitario = parseBR(document.getElementById("input-novo-unitario").value);
    const total = parseBR(document.getElementById("input-novo-total").value);
    const grupo = document.getElementById("input-novo-grupo").value;

    if (!descricao) { mostrarErro("Informe a descrição."); return; }
    if (!quantidade || quantidade <= 0) { mostrarErro("Quantidade inválida (deve ser > 0)."); return; }
    if (unitario == null || unitario < 0) { mostrarErro("Preço unitário inválido."); return; }
    if (total == null || total < 0) { mostrarErro("Preço total inválido."); return; }
    if (!grupo) { mostrarErro("Selecione o grupo."); return; }

    estado.itens.push({ descricao, quantidade, preco_unitario: unitario, preco_total: total, grupo });
    // Limpar campos
    document.getElementById("input-nova-descricao").value = "";
    document.getElementById("input-nova-quantidade").value = "";
    document.getElementById("input-novo-unitario").value = "";
    document.getElementById("input-novo-total").value = "";
    document.getElementById("input-novo-grupo").value = "";
    renderItens();
    atualizarResumo();
  }

  function removerItem(idx) {
    estado.itens.splice(idx, 1);
    renderItens();
    atualizarResumo();
  }

  function editarItem(idx, campo, valor) {
    const item = estado.itens[idx];
    if (campo === "grupo") item.grupo = valor;
    else if (campo === "quantidade") item.quantidade = parseBR(valor) || 0;
    else if (campo === "unitario") item.preco_unitario = parseBR(valor) || 0;
    else if (campo === "total") item.preco_total = parseBR(valor) || 0;
    else item.descricao = valor;
    renderItens();
    atualizarResumo();
  }

  function renderItens() {
    els.corpo.innerHTML = "";
    estado.itens.forEach((item, idx) => {
      const tr = document.createElement("tr");
      tr.innerHTML = `
        <td><input type="text" data-idx="${idx}" data-campo="descricao" class="inp-editar" value="${escapeHtml(item.descricao)}"></td>
        <td><input type="text" data-idx="${idx}" data-campo="quantidade" class="inp-editar" value="${formatQuantidade(item.quantidade)}"></td>
        <td><input type="text" data-idx="${idx}" data-campo="unitario" class="inp-editar" value="${formatBRL(item.preco_unitario)}"></td>
        <td><input type="text" data-idx="${idx}" data-campo="total" class="inp-editar" value="${formatBRL(item.preco_total)}"></td>
        <td><select data-idx="${idx}" data-campo="grupo" class="sel-grupo">
          <option value="">Não atribuído</option>
          <option value="Casa" ${item.grupo === "Casa" ? "selected" : ""}>Casa</option>
          <option value="Igor" ${item.grupo === "Igor" ? "selected" : ""}>Igor</option>
          <option value="Renan" ${item.grupo === "Renan" ? "selected" : ""}>Renan</option>
        </select></td>
        <td><button type="button" data-idx="${idx}" class="btn-remover">Remover</button></td>
      `;
      els.corpo.appendChild(tr);
    });

    // Event delegation para inputs de edição
    els.corpo.addEventListener("change", (e) => {
      const target = e.target;
      const idx = target.dataset.idx;
      const campo = target.dataset.campo;
      if (idx === undefined || !campo) return;
      const item = estado.itens[idx];
      if (!item) return;
      if (campo === "grupo") {
        item.grupo = target.value;
      } else if (campo === "quantidade") {
        item.quantidade = parseBR(target.value) || 0;
      } else if (campo === "unitario") {
        item.preco_unitario = parseBR(target.value) || 0;
      } else if (campo === "total") {
        item.preco_total = parseBR(target.value) || 0;
      } else {
        item.descricao = target.value;
      }
      renderItens();
      atualizarResumo();
    });

    // Event delegation para botões Remover
    els.corpo.addEventListener("click", (e) => {
      const btn = e.target.closest(".btn-remover");
      if (!btn) return;
      const idx = btn.dataset.idx;
      if (idx === undefined) return;
      removerItem(Number(idx));
    });
  }

  function renderAvisos() {
    els.areaAvisos.innerHTML = "";
    if (estado.avisos && estado.avisos.length) {
      estado.avisos.forEach((aviso) => {
        const div = document.createElement("div");
        div.className = "aviso-item";
        div.textContent = "Aviso: " + escapeHtml(aviso);
        els.areaAvisos.appendChild(div);
      });
    }
  }

  // ---------- Resumo / conciliação ----------
  function atualizarResumo() {
    const soma = estado.itens.reduce((acc, it) => acc + (it.preco_total || 0), 0);
    estado.somaItens = soma;
    els.somaItensEl.textContent = formatBRL(soma);

    const totalCupom = parseBR(els.inputTotalCupom.value) || 0;
    const pagador = els.inputPagador.value;
    const diferenca = soma - totalCupom;

    // Selo de conciliação
    const diffAbs = Math.abs(diferenca);
    if (diffAbs <= 0.05) {
      els.selo.textContent = "Conciliado (diferença ≤ R$ 0,05)";
      els.selo.className = "selo selo-conciliado";
    } else {
      els.selo.textContent = "Divergente (diferença " + formatBRL(diferenca) + ")";
      els.selo.className = "selo selo-divergente";
    }

    // Responsabilidades (regras da casa)
    let casa = 0, igorPessoal = 0, renanPessoal = 0;
    estado.itens.forEach((it) => {
      if (it.grupo === "Casa") casa += it.preco_total || 0;
      else if (it.grupo === "Igor") igorPessoal += it.preco_total || 0;
      else if (it.grupo === "Renan") renanPessoal += it.preco_total || 0;
    });
    const metadeCasa = casa / 2;
    let respIgor = igorPessoal + metadeCasa;
    let respRenan = renanPessoal + metadeCasa;
    // Centavo indivisível para quem NÃO pagou
    const centavosCasa = Math.round(casa * 100);
    if (centavosCasa % 2 === 1) {
      if (pagador === "Igor") respRenan += 0.01;
      else respIgor += 0.01;
    }
    const valorPagoIgor = pagador === "Igor" ? totalCupom : 0;
    const valorPagoRenan = pagador === "Renan" ? totalCupom : 0;
    estado.saldoIgor = valorPagoIgor - respIgor;
    estado.saldoRenan = valorPagoRenan - respRenan;

    els.saldoIgorEl.textContent = formatBRL(estado.saldoIgor);
    els.saldoRenanEl.textContent = formatBRL(estado.saldoRenan);
    els.saldoIgorEl.className = "valor " + (estado.saldoIgor >= 0 ? "credito" : "debito");
    els.saldoRenanEl.className = "valor " + (estado.saldoRenan >= 0 ? "credito" : "debito");
  }

  // ---------- Salvar ----------
  async function salvarCupom() {
    // Validar total (D10: divergência bloqueia na camada web até decisão de Igor)
    const soma = estado.somaItens;
    const total = parseBR(els.inputTotalCupom.value) || 0;
    if (Math.abs(soma - total) > 0.05) {
      mostrarErro("Divergência de conciliação: a soma dos itens (" + formatBRL(soma) + ") difere do total do cupom (" + formatBRL(total) + ") em mais de R$ 0,05. Ajuste antes de salvar.");
      return;
    }

    // Validar que todos os itens têm grupo
    const semGrupo = estado.itens.find((it) => !it.grupo);
    if (semGrupo) {
      mostrarErro("Todos os itens precisam de um grupo (Casa/Igor/Renan).");
      return;
    }

    const cupom = {
      receita: {
        data_compra: els.inputData.value || new Date().toISOString().slice(0, 10),
        estabelecimento: (document.getElementById("input-nova-estabelecimento") || {}).value || "Estabelecimento",
        total_cupom: total,
        imagem_path: "",
        pagador: els.inputPagador.value,
      },
      itens: estado.itens.map((it) => ({ item: it, grupo: it.grupo })),
    };

    try {
      const resp = await fetch(`${API_BASE}/api/cupons`, {
        method: "POST",
        headers: authHeadersJson(),
        body: JSON.stringify(cupom),
      });
      if (!resp.ok) {
        const err = await resp.json();
        throw new Error(err.mensagem || "Falha ao salvar");
      }
      const dados = await resp.json();
      mostrarSucesso("Cupom salvo com sucesso (ID " + dados.id + ").");
      carregarHistorico();
      novoCupom();
    } catch (e) {
      mostrarErro(e.message || "Erro ao salvar cupom.");
    }
  }

  function novoCupom() {
    limparItens();
    els.inputTotalCupom.value = "0,00";
    els.inputPagador.value = "Igor";
    els.inputData.value = "";
    estado.data = "";
    secaoVisiveis(false);
    els.msgSalvo.classList.add("oculto");
    els.msgErro.classList.add("oculto");
  }

  function secaoVisiveis(mostrar) {
    [els.secaoItens, els.secaoResumo, els.secaoAcoes].forEach((s) => {
      if (mostrar) s.classList.remove("oculto"); else s.classList.add("oculto");
    });
  }

  // ---------- Histórico ----------
  async function carregarHistorico() {
    try {
      const resp = await fetch(`${API_BASE}/api/cupons`, {
        headers: authHeaders(),
      });
      if (!resp.ok) throw new Error("Falha ao listar cupons");
      estado.historico = await resp.json();
      estado.historico = estado.historico.cupons || estado.historico || [];
      renderHistorico();
    } catch (e) {
      console.error("Falha ao carregar histórico:", e);
    }
  }

  function renderHistorico() {
    const busca = els.inputBusca.value.toLowerCase();
    const fConc = els.filtroConciliado.checked;
    const fDiv = els.filtroDivergente.checked;

    const filtrados = estado.historico.filter((c) => {
      const texto = (c.estabelecimento + " " + c.total_cupom + " " + c.pagador).toLowerCase();
      if (busca && !texto.includes(busca)) return false;
      // Filtro por conciliação (simplificado: sempre mostra se não há filtro ativo)
      return true;
    });

    els.listaHistorico.innerHTML = "";
    if (filtrados.length === 0) {
      els.listaHistorico.innerHTML = '<li class="vazio">Nenhum cupom encontrado.</li>';
      return;
    }
    filtrados.forEach((c) => {
      const li = document.createElement("li");
      li.className = "item-historico";
      li.innerHTML = `
        <div class="titulo">${escapeHtml(c.estabelecimento)}</div>
        <div class="meta">${c.data_compra} · Pagador: ${c.pagador}</div>
        <div class="total">${formatBRL(c.total_cupom)}</div>
      `;
      li.addEventListener("click", () => verDetalhes(c.id));
      els.listaHistorico.appendChild(li);
    });
  }

  // ---------- Detalhes ----------
  async function verDetalhes(id) {
    try {
      const resp = await fetch(`${API_BASE}/api/cupons/${id}`, {
        headers: authHeaders(),
      });
      if (!resp.ok) throw new Error("Cupom não encontrado");
      const cupom = await resp.json();
      document.getElementById("det-estabelecimento").textContent = escapeHtml(cupom.receita.estabelecimento);
      document.getElementById("det-data").textContent = cupom.receita.data_compra;
      document.getElementById("det-total").textContent = formatBRL(cupom.receita.total_cupom);
      document.getElementById("det-pagador").textContent = cupom.receita.pagador;

      const soma = cupom.itens.reduce((acc, it) => acc + (it.item.preco_total || 0), 0);
      const pagador = cupom.receita.pagador;
      let casa = 0, igorP = 0, renanP = 0;
      cupom.itens.forEach((it) => {
        if (it.grupo === "Casa") casa += it.item.preco_total || 0;
        else if (it.grupo === "Igor") igorP += it.item.preco_total || 0;
        else if (it.grupo === "Renan") renanP += it.item.preco_total || 0;
      });
      const metadeCasa = casa / 2;
      let respIgor = igorP + metadeCasa;
      let respRenan = renanP + metadeCasa;
      const centavosCasa = Math.round(casa * 100);
      if (centavosCasa % 2 === 1) {
        if (pagador === "Igor") respRenan += 0.01;
        else respIgor += 0.01;
      }
      const vpIgor = pagador === "Igor" ? cupom.receita.total_cupom : 0;
      const vpRenan = pagador === "Renan" ? cupom.receita.total_cupom : 0;
      const saldoIgor = vpIgor - respIgor;
      const saldoRenan = vpRenan - respRenan;

      document.getElementById("det-igor").textContent = formatBRL(respIgor);
      document.getElementById("det-renan").textContent = formatBRL(respRenan);
      document.getElementById("det-saldo-igor").textContent = formatBRL(saldoIgor);
      document.getElementById("det-saldo-renan").textContent = formatBRL(saldoRenan);

      const corpo = document.getElementById("det-itens");
      corpo.innerHTML = "";
      cupom.itens.forEach((it) => {
        const tr = document.createElement("tr");
        tr.innerHTML = `
          <td>${escapeHtml(it.item.descricao)}</td>
          <td>${formatQuantidade(it.item.quantidade)}</td>
          <td>${formatBRL(it.item.preco_unitario)}</td>
          <td>${formatBRL(it.item.preco_total)}</td>
          <td>${it.grupo}</td>
        `;
        corpo.appendChild(tr);
      });

      els.modalDetalhes.hidden = false;
    } catch (e) {
      mostrarErro(e.message || "Erro ao carregar detalhes.");
    }
  }

  function copiarRelatorioDetalhes() {
    // Copiar texto dos detalhes
    const texto = [
      "Estabelecimento: " + document.getElementById("det-estabelecimento").textContent,
      "Data: " + document.getElementById("det-data").textContent,
      "Total: " + document.getElementById("det-total").textContent,
      "Pagador: " + document.getElementById("det-pagador").textContent,
      "Igor: " + document.getElementById("det-igor").textContent,
      "Renan: " + document.getElementById("det-renan").textContent,
      "Saldo Igor: " + document.getElementById("det-saldo-igor").textContent,
      "Saldo Renan: " + document.getElementById("det-saldo-renan").textContent,
    ].join("\n");
    navigator.clipboard.writeText(texto).then(() => mostrarSucesso("Relatório copiado."));
  }

  // ---------- Relatório do período ----------
  async function gerarRelatorioPeriodo() {
    const de = document.getElementById("rel-de").value;
    const ate = document.getElementById("rel-ate").value;
    let url = `${API_BASE}/api/relatorio`;
    if (de || ate) url += "?" + (de ? "de=" + encodeURIComponent(de) : "") + (ate ? "&ate=" + encodeURIComponent(ate) : "");
    try {
      const resp = await fetch(url, {
        headers: authHeaders(),
      });
      if (!resp.ok) throw new Error("Falha ao gerar relatório");
      const dados = await resp.json();
      const linhas = [
        "Período: " + (dados.periodo_de || "início") + " a " + (dados.periodo_ate || "hoje"),
        "Total de cupons: " + dados.total_cupons,
        "Total geral: " + formatBRL(dados.total_geral),
        "",
        "Cupons:",
      ];
      dados.cupons.forEach((c) => {
        linhas.push("  " + c.data_compra + " · " + escapeHtml(c.estabelecimento) + " · " + formatBRL(c.total_cupom) + " · " + c.pagador);
      });
      document.getElementById("resultado-relatorio").textContent = linhas.join("\n");
    } catch (e) {
      mostrarErro(e.message || "Erro ao gerar relatório.");
    }
  }

  function copiarRelatorioPeriodo() {
    const texto = document.getElementById("resultado-relatorio").textContent;
    if (!texto) return;
    navigator.clipboard.writeText(texto).then(() => mostrarSucesso("Relatório copiado."));
  }

  // ---------- Utilitários ----------
  function parseBR(texto) {
    if (texto == null || texto === "") return 0;
    let t = String(texto).trim();
    if (t === "") return 0;
    // Rejeitar NaN/inf
    if (/^(nan|inf|infinity|-inf|-infinity)$/i.test(t)) return NaN;
    // Formato brasileiro: 1.234,56
    if (t.includes(",")) {
      const partes = t.split(",");
      if (partes.length !== 2) return NaN;
      const inteira = partes[0].replace(/\./g, "");
      const decimal = partes[1];
      if (decimal.length > 2) return NaN;
      t = inteira + "." + decimal;
    } else if (t.includes(".")) {
      const partes = t.split(".");
      if (partes.length === 2 && partes[1].length <= 2) {
        // 12.5 → decimal
      } else {
        // 1.234 → milhar
        t = t.replace(/\./g, "");
      }
    }
    const v = parseFloat(t);
    return isNaN(v) ? NaN : v;
  }

  function formatBRL(valor) {
    if (valor == null || !isFinite(valor)) return "R$ 0,00";
    const negativo = valor < 0;
    const v = Math.abs(valor);
    const centavos = Math.round(v * 100);
    const inteira = Math.floor(centavos / 100);
    const dec = (centavos % 100).toString().padStart(2, "0");
    const inteiraStr = inteira.toString().replace(/\B(?=(\d{3})+(?!\d))/g, ".");
    return (negativo ? "-" : "") + "R$ " + inteiraStr + "," + dec;
  }

  function formatQuantidade(v) {
    if (v == null || !isFinite(v)) return "0";
    if (Number.isInteger(v)) return String(v);
    return String(v).replace(".", ",");
  }

  function escapeHtml(texto) {
    if (texto == null) return "";
    return String(texto)
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;")
      .replace(/'/g, "&#039;");
  }

  function mostrarErro(msg) {
    els.msgErro.textContent = msg;
    els.msgErro.classList.remove("oculto");
    setTimeout(() => els.msgErro.classList.add("oculto"), 5000);
  }
  function mostrarSucesso(msg) {
    els.msgSalvo.textContent = msg;
    els.msgSalvo.classList.remove("oculto");
    setTimeout(() => els.msgSalvo.classList.add("oculto"), 5000);
  }
})();
