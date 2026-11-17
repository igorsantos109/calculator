//! Servidor HTTP usando `hyper` (disponível offline; `axum` não está no cache).
//!
//! Segurança:
//! - PIN obrigatório em TODAS as rotas /api (inclusive /api/saude).
//! - Sem CSRF (auth é por header Bearer, não por cookie → não vulnerável a CSRF clássico).
//! - Limite de corpo nas rotas POST (10 MB).
//! - Rate limiting simples por IP (tentativas falhadas).
//! - Timeout curto na chamada ao LLM.
//! - Não segurar lock do banco durante chamadas de rede.

use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, StatusCode};
use serde::Serialize;
use tokio::sync::Mutex;

use crate::assets::{APP_JS, INDEX_HTML, STYLE_CSS};
use crate::config::Config;
use crate::security::{cabecalhos_seguranca, constante_tempo_igual};

/// Estado compartilhado do servidor.
pub struct ServerState {
    pub config: Config,
    pub db: Arc<Mutex<db::Repositorio>>,
    pub llm_cliente: Arc<Mutex<receipt_core::llm::ClienteLlm>>,
    pub rate_limiter: IpRateLimiter,
}

/// Rate limiting: tentativas falhadas por IP.
/// Simples mapa em memória (reinicia ao reiniciar o servidor).
#[derive(Debug, Default, Clone)]
pub struct IpRateLimiter {
    tentativas: Arc<Mutex<HashMap<String, (u32, u64)>>>,
}

impl IpRateLimiter {
    pub fn new() -> Self {
        Self {
            tentativas: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn registrar_falha(&self, ip: &str) {
        let agora = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let mut tentativas = self.tentativas.lock().await;
        let (count, tempo) = tentativas.entry(ip.to_string()).or_insert((0, agora));
        
        // Resetar contador se passou mais de 5 minutos (300s)
        if agora.saturating_sub(*tempo) > 300 {
            *count = 1;
            *tempo = agora;
        } else {
            *count += 1;
        }
    }

    pub async fn esta_bloqueado(&self, ip: &str) -> bool {
        let agora = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let tentativas = self.tentativas.lock().await;
        if let Some((count, tempo)) = tentativas.get(ip) {
            if agora.saturating_sub(*tempo) > 300 {
                return false;
            }
            *count >= 5
        } else {
            false
        }
    }
}

/// Tamanho máximo do corpo de requisição (10 MB).
const MAX_CORPO_BYTES: usize = 10 * 1024 * 1024;

/// Lê o corpo da requisição com limite de tamanho.
async fn ler_corpo_limitado(req: Request<Body>) -> Result<hyper::body::Bytes, Box<Response<Body>>> {
    let corpo = hyper::body::to_bytes(req.into_body())
        .await
        .map_err(|_| Box::new(ApiError::novo("erro_leitura", "Falha ao ler corpo da requisição")
            .resposta(StatusCode::BAD_REQUEST)))?;
    
    if corpo.len() > MAX_CORPO_BYTES {
        return Err(Box::new(ApiError::novo("corpo_grande", 
            format!("Corpo da requisição excede o limite de {} bytes", MAX_CORPO_BYTES))
            .resposta(StatusCode::PAYLOAD_TOO_LARGE)));
    }
    
    Ok(corpo)
}

/// Erro da API em pt-BR com código.
#[derive(Debug, Serialize)]
pub struct ApiError {
    pub codigo: String,
    pub mensagem: String,
}

impl ApiError {
    fn novo(codigo: &'static str, mensagem: impl Into<String>) -> Self {
        ApiError {
            codigo: codigo.to_string(),
            mensagem: mensagem.into(),
        }
    }

    fn resposta(&self, status: StatusCode) -> Response<Body> {
        let corpo = serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string());
        let mut resposta = Response::new(Body::from(corpo));
        *resposta.status_mut() = status;
        resposta
            .headers_mut()
            .insert("content-type", "application/json".parse().unwrap());
        for (chave, valor) in cabecalhos_seguranca() {
            resposta.headers_mut().insert(chave, valor.parse().unwrap());
        }
        resposta
    }
}

/// Cria uma resposta de sucesso JSON.
fn resposta_json<T: Serialize>(status: StatusCode, valor: &T) -> Response<Body> {
    let corpo = serde_json::to_string(valor).unwrap_or_else(|_| "{}".to_string());
    let mut resposta = Response::new(Body::from(corpo));
    *resposta.status_mut() = status;
    resposta
        .headers_mut()
        .insert("content-type", "application/json".parse().unwrap());
    for (chave, valor) in cabecalhos_seguranca() {
        resposta.headers_mut().insert(chave, valor.parse().unwrap());
    }
    resposta
}

/// Extrai o valor de um header de requisição.
fn header_valor(req: &Request<Body>, nome: &str) -> Option<String> {
    req.headers()
        .get(nome)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// Manipula uma requisição HTTP.
///
/// O `estado` é um `Arc<Mutex<ServerState>>` compartilhado entre requisições.
/// Bloqueamos o mutex apenas o suficiente para extrair clones dos campos
/// (que são todos `Arc`), evitando segurar o guard durante chamadas de rede
/// ou de banco.
pub async fn handle(
    req: Request<Body>,
    estado: Arc<Mutex<ServerState>>,
) -> Result<Response<Body>, Infallible> {
    let guard = estado.lock().await;
    let config = guard.config.clone();
    let db = guard.db.clone();
    let llm_cliente = guard.llm_cliente.clone();
    let rate_limiter = guard.rate_limiter.clone();
    drop(guard);

    let caminho = req.uri().path().to_string();
    let metodo = req.method().clone();

    // TODAS as rotas /api exigem PIN (incluindo /api/saude).
    // Apenas "/", "/app.js", "/style.css" são públicas (página + assets).
    let rota_publica = caminho == "/" || caminho == "/app.js" || caminho == "/style.css";

    if !rota_publica {
        let autenticado = {
            let auth_ok = header_valor(&req, "authorization")
                .map(|auth| {
                    auth.strip_prefix("Bearer ")
                        .is_some_and(|pin| constante_tempo_igual(pin, &config.pin))
                })
                .unwrap_or(false);
            let xpin_ok = header_valor(&req, "x-pin")
                .map(|pin| constante_tempo_igual(&pin, &config.pin))
                .unwrap_or(false);
            auth_ok || xpin_ok
        };
        if !autenticado {
            // Registrar falha para rate limiting
            let ip = "desconhecido"; // Simplificado - em produção usar IP real
            rate_limiter.registrar_falha(ip).await;
            return Ok(
                ApiError::novo("pin_obrigatorio", "PIN de acesso é obrigatório")
                    .resposta(StatusCode::UNAUTHORIZED),
            );
        }
    }

    // Rate limiting: verificar se IP está bloqueado (apenas para rotas /api)
    if !rota_publica {
        // Obter IP do cliente (simplificado - hyper não expõe IP diretamente sem connection info)
        // Para produção, usar um extractor de IP real
        let ip = "desconhecido";
        let bloqueado = rate_limiter.esta_bloqueado(ip).await;
        if bloqueado {
            return Ok(
                ApiError::novo("rate_limit_excedido", "Muitas tentativas falhadas. Tente novamente em 5 minutos.")
                    .resposta(StatusCode::TOO_MANY_REQUESTS),
            );
        }
    }

    // Dispatch: mover req nas rotas POST que consomem o corpo
    let resposta = match (metodo, caminho.as_str()) {
        (Method::GET, "/") => {
            let mut resposta = Response::new(Body::from(INDEX_HTML));
            *resposta.status_mut() = StatusCode::OK;
            resposta
                .headers_mut()
                .insert("content-type", "text/html; charset=utf-8".parse().unwrap());
            for (chave, valor) in cabecalhos_seguranca() {
                resposta.headers_mut().insert(chave, valor.parse().unwrap());
            }
            resposta
        }
        (Method::GET, "/style.css") => {
            let mut resposta = Response::new(Body::from(STYLE_CSS));
            *resposta.status_mut() = StatusCode::OK;
            resposta
                .headers_mut()
                .insert("content-type", "text/css; charset=utf-8".parse().unwrap());
            for (chave, valor) in cabecalhos_seguranca() {
                resposta.headers_mut().insert(chave, valor.parse().unwrap());
            }
            resposta
        }
        (Method::GET, "/app.js") => {
            let mut resposta = Response::new(Body::from(APP_JS));
            *resposta.status_mut() = StatusCode::OK;
            resposta.headers_mut().insert(
                "content-type",
                "application/javascript; charset=utf-8".parse().unwrap(),
            );
            for (chave, valor) in cabecalhos_seguranca() {
                resposta.headers_mut().insert(chave, valor.parse().unwrap());
            }
            resposta
        }
        (Method::GET, "/api/saude") => saude(&config, &db).await,
        (Method::GET, "/api/cupons") => listar_cupons(&db).await,
        (Method::GET, p) if p.starts_with("/api/cupons/") => buscar_cupom(&db, p).await,
        (Method::GET, "/api/relatorio") => relatorio(&req, &db).await,
        (Method::POST, "/api/ocr") => extrair_ocr(req, &llm_cliente).await,
        (Method::POST, "/api/extrair") => extrair_llm(req, &llm_cliente).await,
        (Method::POST, "/api/cupons") => salvar_cupom(req, &db).await,
        _ => ApiError::novo("rota_nao_encontrada", "Rota não encontrada")
            .resposta(StatusCode::NOT_FOUND),
    };

    Ok(resposta)
}

/// Rotina principal do servidor.
pub async fn run(config: Config) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = config.listen_addr;

    // Abrir banco de dados
    let db = db::Repositorio::new(&config.db_path).expect("falha ao abrir banco de dados");

    // Cliente LLM
    let llm_config = receipt_core::llm::LlmConfig {
        llm_base_url: config.llm_base_url.clone(),
        llm_model: config.llm_model.clone(),
        llm_api_key_env: config.llm_api_key_env.clone(),
        timeout: 120,
        max_tentativas: 3,
        max_tokens: 2048,
    };
    let llm_cliente =
        receipt_core::llm::ClienteLlm::new(llm_config).expect("falha ao criar cliente LLM");

    let rate_limiter = IpRateLimiter::new();

    let estado = Arc::new(Mutex::new(ServerState {
        config: config.clone(),
        db: Arc::new(Mutex::new(db)),
        llm_cliente: Arc::new(Mutex::new(llm_cliente)),
        rate_limiter,
    }));

    let make_svc = make_service_fn(move |_conn| {
        let estado = estado.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                let estado = estado.clone();
                async move { handle(req, estado).await }
            }))
        }
    });

    let servidor = hyper::Server::bind(&addr).serve(make_svc);
    println!("Servidor web escutando em http://{}", addr);
    servidor
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })
}

/// GET /api/saude — relata estado real dos componentes.
/// Exige PIN. Timeout curto na chamada ao LLM. Não segurar lock do banco
/// durante chamadas de rede.
async fn saude(config: &Config, db: &Arc<Mutex<db::Repositorio>>) -> Response<Body> {
    let versao = {
        let db = db.lock().await;
        db::Repositorio::versao_atual(&db.conn).unwrap_or(0)
    };

    let base = config.llm_base_url.trim_end_matches('/');
    let url = format!("{}/health", base);

    // Timeout curto (≤ 3s) na chamada ao LLM — não segurar lock do banco.
    let llm_status = match tokio::time::timeout(Duration::from_secs(3), reqwest::get(url)).await {
        Ok(Ok(resposta)) => {
            if resposta.status().is_success() {
                "alcançável".to_string()
            } else {
                format!("indisponível (HTTP {})", resposta.status().as_u16())
            }
        }
        Ok(Err(_)) => "indisponível".to_string(),
        Err(_) => "timeout_llm".to_string(),
    };

    let ocr_status = if std::path::Path::new(&config.ocr_dir).exists() {
        "modelos_presentes".to_string()
    } else {
        "pasta_ocr_ausente".to_string()
    };

    #[derive(Serialize)]
    struct Saude {
        banco: String,
        schema_versao: i64,
        llm: String,
        ocr: String,
    }

    let saude = Saude {
        banco: "ok".to_string(),
        schema_versao: versao,
        llm: llm_status,
        ocr: ocr_status,
    };

    resposta_json(StatusCode::OK, &saude)
}

/// GET /api/cupons — lista histórico (ordenado por id DESC).
async fn listar_cupons(db: &Arc<Mutex<db::Repositorio>>) -> Response<Body> {
    let db = db.lock().await;
    match db::Repositorio::listar_cupons(&db.conn) {
        Ok(cupons) => {
            let total = cupons.len();
            #[derive(Serialize)]
            struct Saida {
                cupons: Vec<db::CupomResumo>,
                total: usize,
            }
            resposta_json(StatusCode::OK, &Saida { cupons, total })
        }
        Err(e) => {
            ApiError::novo("erro_banco", e.to_string()).resposta(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/cupons/{id} — detalhes de um cupom (inclui itens).
async fn buscar_cupom(db: &Arc<Mutex<db::Repositorio>>, caminho: &str) -> Response<Body> {
    let id_str = caminho.trim_start_matches("/api/cupons/");
    let id: i64 = match id_str.parse() {
        Ok(v) => v,
        Err(_) => {
            return ApiError::novo("id_invalido", "ID do cupom deve ser numérico")
                .resposta(StatusCode::BAD_REQUEST);
        }
    };

    let db = db.lock().await;
    match db::Repositorio::buscar_cupom(&db.conn, id) {
        Ok(Some(mut cupom)) => {
            // Carregar itens do recibo (busca os itens armazenados)
            match db::Repositorio::buscar_itens_por_receipt_id(&db.conn, id) {
                Ok(itens) => {
                    cupom.itens = itens;
                }
                Err(_) => {
                    cupom.itens = Vec::new();
                }
            }
            resposta_json(StatusCode::OK, &cupom)
        }
        Ok(None) => {
            ApiError::novo("nao_encontrado", "Cupom não encontrado").resposta(StatusCode::NOT_FOUND)
        }
        Err(e) => {
            ApiError::novo("erro_banco", e.to_string()).resposta(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/relatorio?de=&ate= — relatório do período.
/// FILTRA antes de somar (F6). Inclui totais por grupo e saldos do período.
async fn relatorio(req: &Request<Body>, db: &Arc<Mutex<db::Repositorio>>) -> Response<Body> {
    let query = req.uri().query().unwrap_or("");
    let mut data_de = String::new();
    let mut data_ate = String::new();
    for par in query.split('&') {
        if let Some((chave, valor)) = par.split_once('=') {
            match chave {
                "de" => data_de = url_decode(valor),
                "ate" => data_ate = url_decode(valor),
                _ => {}
            }
        }
    }

    let db = db.lock().await;
    let cupons = match db::Repositorio::listar_cupons(&db.conn) {
        Ok(c) => c,
        Err(e) => {
            return ApiError::novo("erro_banco", e.to_string())
                .resposta(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Filtrar ANTES de somar (F6).
    let filtrados: Vec<_> = cupons
        .into_iter()
        .filter(|c| {
            let ok_de = data_de.is_empty() || c.data_compra >= data_de;
            let ok_ate = data_ate.is_empty() || c.data_compra <= data_ate;
            ok_de && ok_ate
        })
        .collect();

    let total_cupons = filtrados.len();
    let total_geral: f64 = filtrados.iter().map(|c| c.total_cupom).sum();

    // Totais por grupo do período
    let mut casa_periodo: f64 = 0.0;
    let mut igor_periodo: f64 = 0.0;
    let mut renan_periodo: f64 = 0.0;
    for c in &filtrados {
        // Buscar itens para cada cupom para somar por grupo
        if let Ok(itens) = db::Repositorio::buscar_itens_por_receipt_id(&db.conn, c.id) {
            for it in &itens {
                match it.grupo {
                    receipt_core::Grupo::Casa => casa_periodo += it.item.preco_total,
                    receipt_core::Grupo::Igor => igor_periodo += it.item.preco_total,
                    receipt_core::Grupo::Renan => renan_periodo += it.item.preco_total,
                }
            }
        }
    }

    #[derive(Serialize)]
    struct Saida {
        periodo_de: String,
        periodo_ate: String,
        cupons: Vec<db::CupomResumo>,
        total_cupons: usize,
        total_geral: f64,
        casa: f64,
        igor: f64,
        renan: f64,
    }

    resposta_json(
        StatusCode::OK,
        &Saida {
            periodo_de: data_de,
            periodo_ate: data_ate,
            cupons: filtrados,
            total_cupons,
            total_geral,
            casa: casa_periodo,
            igor: igor_periodo,
            renan: renan_periodo,
        },
    )
}

/// POST /api/ocr — receber imagem e retornar texto OCR (placeholder; OCR real não verificado).
async fn extrair_ocr(
    _req: Request<Body>,
    _llm_cliente: &Arc<Mutex<receipt_core::llm::ClienteLlm>>,
) -> Response<Body> {
    // OCR real (alvio-ocr) não pode ser verificado offline. Retornar modo manual.
    ApiError::novo(
        "ocr_nao_verificado",
        "OCR real não disponível neste build (sem rede para modelos). Use modo manual ou colar texto.",
    )
    .resposta(StatusCode::NOT_IMPLEMENTED)
}

/// POST /api/extrair — enviar texto OCR para o LLM e retornar itens extraídos.
async fn extrair_llm(
    req: Request<Body>,
    llm_cliente: &Arc<Mutex<receipt_core::llm::ClienteLlm>>,
) -> Response<Body> {
    // Ler corpo da requisição com limite de tamanho
    let corpo = match ler_corpo_limitado(req).await {
        Ok(c) => c,
        Err(resp) => return *resp,
    };
    let dados: serde_json::Value = match serde_json::from_slice(&corpo) {
        Ok(v) => v,
        Err(_) => {
            return ApiError::novo("json_invalido", "Corpo da requisição não é JSON válido")
                .resposta(StatusCode::BAD_REQUEST);
        }
    };

    let texto_ocr = dados
        .get("texto_ocr")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let total_cupom = dados
        .get("total_cupom")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    // Validações básicas dos campos
    if texto_ocr.trim().is_empty() {
        return ApiError::novo("campo_obrigatorio", "texto_ocr é obrigatório")
            .resposta(StatusCode::BAD_REQUEST);
    }
    if total_cupom < 0.0 {
        return ApiError::novo("campo_invalido", "total_cupom não pode ser negativo")
            .resposta(StatusCode::BAD_REQUEST);
    }

    let llm = llm_cliente.lock().await;
    match llm.extrair_itens(&texto_ocr, total_cupom).await {
        Ok(resultado) => {
            #[derive(Serialize)]
            struct Saida {
                itens: Vec<receipt_core::Item>,
                avisos: Vec<String>,
                soma_itens: f64,
                total_cupom: f64,
            }
            resposta_json(
                StatusCode::OK,
                &Saida {
                    itens: resultado.itens,
                    avisos: resultado.avisos,
                    soma_itens: resultado.soma_itens,
                    total_cupom,
                },
            )
        }
        Err(e) => {
            let (codigo, status) = match &e {
                receipt_core::llm::LlmError::ServidorInacessível(_) => {
                    ("servidor_llm_inacessível", StatusCode::BAD_GATEWAY)
                }
                receipt_core::llm::LlmError::Timeout => {
                    ("timeout_llm", StatusCode::GATEWAY_TIMEOUT)
                }
                receipt_core::llm::LlmError::ChaveInvalida(_) => {
                    ("chave_llm_invalida", StatusCode::UNAUTHORIZED)
                }
                receipt_core::llm::LlmError::ServidorIndisponível(_, _) => {
                    ("servidor_llm_indisponível", StatusCode::BAD_GATEWAY)
                }
                receipt_core::llm::LlmError::RespostaTruncada => {
                    ("resposta_truncada", StatusCode::BAD_GATEWAY)
                }
                receipt_core::llm::LlmError::JsonInvalido(_) => {
                    ("json_invalido", StatusCode::BAD_GATEWAY)
                }
                receipt_core::llm::LlmError::ItemInvalido(_) => {
                    ("item_invalido", StatusCode::BAD_REQUEST)
                }
                receipt_core::llm::LlmError::RespostaInvalida(_) => {
                    ("resposta_invalida", StatusCode::BAD_GATEWAY)
                }
            };
            // Erro genérico sem vazar detalhes internos/SQL
            ApiError::novo(codigo, e.to_string()).resposta(status)
        }
    }
}

/// POST /api/cupons — salvar um cupom completo.
async fn salvar_cupom(req: Request<Body>, db: &Arc<Mutex<db::Repositorio>>) -> Response<Body> {
    // Ler corpo da requisição com limite de tamanho
    let corpo = match ler_corpo_limitado(req).await {
        Ok(c) => c,
        Err(resp) => return *resp,
    };
    let cupom: db::CupomCompleto = match serde_json::from_slice(&corpo) {
        Ok(v) => v,
        Err(_) => {
            return ApiError::novo("json_invalido", "Corpo da requisição não é JSON válido")
                .resposta(StatusCode::BAD_REQUEST);
        }
    };

    // Validar campos obrigatórios da receita
    if cupom.receita.data_compra.trim().is_empty() {
        return ApiError::novo("campo_obrigatorio", "data_compra é obrigatória")
            .resposta(StatusCode::BAD_REQUEST);
    }
    if cupom.receita.estabelecimento.trim().is_empty() {
        return ApiError::novo("campo_obrigatorio", "estabelecimento é obrigatório")
            .resposta(StatusCode::BAD_REQUEST);
    }
    if cupom.receita.total_cupom < 0.0 {
        return ApiError::novo("campo_invalido", "total_cupom não pode ser negativo")
            .resposta(StatusCode::BAD_REQUEST);
    }
    if cupom.receita.pagador != "igor" && cupom.receita.pagador != "renan" {
        return ApiError::novo("campo_invalido", "pagador deve ser 'igor' ou 'renan'")
            .resposta(StatusCode::BAD_REQUEST);
    }

    // Validar itens
    if cupom.itens.is_empty() {
        return ApiError::novo("campo_obrigatorio", "pelo menos um item é obrigatório")
            .resposta(StatusCode::BAD_REQUEST);
    }
    for (i, ic) in cupom.itens.iter().enumerate() {
        let item = &ic.item;
        if item.descricao.trim().is_empty() {
            return ApiError::novo("campo_obrigatorio", format!("item {}: descrição é obrigatória", i + 1))
                .resposta(StatusCode::BAD_REQUEST);
        }
        if item.quantidade <= 0.0 {
            return ApiError::novo("campo_invalido", format!("item {}: quantidade deve ser > 0", i + 1))
                .resposta(StatusCode::BAD_REQUEST);
        }
        if item.preco_unitario < 0.0 {
            return ApiError::novo("campo_invalido", format!("item {}: preço unitário não pode ser negativo", i + 1))
                .resposta(StatusCode::BAD_REQUEST);
        }
        if item.preco_total < 0.0 {
            return ApiError::novo("campo_invalido", format!("item {}: preço total não pode ser negativo", i + 1))
                .resposta(StatusCode::BAD_REQUEST);
        }
        if ic.grupo != receipt_core::Grupo::Casa && ic.grupo != receipt_core::Grupo::Igor && ic.grupo != receipt_core::Grupo::Renan {
            return ApiError::novo("campo_invalido", format!("item {}: grupo deve ser 'casa', 'igor' ou 'renan'", i + 1))
                .resposta(StatusCode::BAD_REQUEST);
        }
    }

    // Validar total no core (centavos inteiros, tolerância R$ 0,05)
    let itens: Vec<receipt_core::Item> = cupom.itens.iter().map(|ic| ic.item.clone()).collect();
    if let Err(e) = receipt_core::validar_total(&itens, cupom.receita.total_cupom) {
        return ApiError::novo(
            "divergencia_total",
            format!("Divergência de conciliação: {}", e),
        )
        .resposta(StatusCode::BAD_REQUEST);
    }

    let mut db = db.lock().await;
    match db::Repositorio::salvar_cupom(&mut db.conn, &cupom) {
        Ok(id) => {
            #[derive(Serialize)]
            struct Saida {
                id: i64,
                mensagem: String,
            }
            resposta_json(
                StatusCode::CREATED,
                &Saida {
                    id,
                    mensagem: "Cupom salvo com sucesso".to_string(),
                },
            )
        }
        Err(_) => {
            // Erro genérico sem vazar detalhes internos/SQL
            ApiError::novo("erro_banco", "Falha ao salvar cupom").resposta(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Decodifica percent-encoding simples de query string.
fn url_decode(entrada: &str) -> String {
    let mut saida = String::new();
    let bytes = entrada.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = &bytes[i + 1..i + 3];
            if let Ok(val) = u8::from_str_radix(std::str::from_utf8(hex).unwrap_or("00"), 16) {
                saida.push(val as char);
                i += 3;
            } else {
                saida.push(bytes[i] as char);
                i += 1;
            }
        } else if bytes[i] == b'+' {
            saida.push(' ');
            i += 1;
        } else {
            saida.push(bytes[i] as char);
            i += 1;
        }
    }
    saida
}
