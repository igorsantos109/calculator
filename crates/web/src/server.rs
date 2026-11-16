//! Servidor HTTP usando `hyper` (disponível offline; `axum` não está no cache).

use std::convert::Infallible;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, StatusCode};
use serde::Serialize;
use tokio::sync::Mutex;

use crate::config::Config;
use crate::security::{cabecalhos_seguranca, constante_tempo_igual, validar_csrf};

/// Estado compartilhado do servidor.
pub struct ServerState {
    pub config: Config,
    pub csrf_token: Arc<Mutex<String>>,
    pub db: Arc<Mutex<db::Repositorio>>,
    pub llm_cliente: Arc<Mutex<receipt_core::llm::ClienteLlm>>,
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
    let csrf_token = guard.csrf_token.clone();
    let db = guard.db.clone();
    let llm_cliente = guard.llm_cliente.clone();
    drop(guard);

    let caminho = req.uri().path().to_string();
    let metodo = req.method().clone();

    // Rotas que não exigem PIN: /api/saude e OPTIONS (preflight CORS)
    let rota_publica = caminho == "/api/saude" || caminho == "/";

    if !rota_publica {
        // Autenticação via header/query (não consome req)
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
            let query_ok = req.uri().query().is_some_and(|q| {
                q.split('&').any(|par| match par.split_once('=') {
                    Some((k, v)) => k == "pin" && constante_tempo_igual(v, &config.pin),
                    None => false,
                })
            });
            auth_ok || xpin_ok || query_ok
        };
        if !autenticado {
            return Ok(
                ApiError::novo("pin_obrigatorio", "PIN de acesso é obrigatório")
                    .resposta(StatusCode::UNAUTHORIZED),
            );
        }
    }

    // Rotas que exigem CSRF para métodos que mudam estado
    let muda_estado = matches!(
        metodo,
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    );
    if muda_estado && !rota_publica {
        let token_fornecido = header_valor(&req, "x-csrf-token");
        let token_esperado = csrf_token.lock().await.clone();
        if !validar_csrf(token_fornecido.as_deref(), &token_esperado).is_ok() {
            return Ok(ApiError::novo("csrf_invalido", "Token CSRF inválido")
                .resposta(StatusCode::FORBIDDEN));
        }
    }

    // Dispatch: mover req nas rotas POST que consomem o corpo
    let resposta = match (metodo, caminho.as_str()) {
        (Method::GET, "/api/saude") => saude(&config, &db).await,
        (Method::GET, "/api/cupons") => listar_cupons(&db).await,
        (Method::GET, p) if p.starts_with("/api/cupons/") => buscar_cupom(&db, p).await,
        (Method::GET, "/api/relatorio") => relatorio(&req, &db).await,
        (Method::POST, "/api/ocr") => extrair_ocr(req, &llm_cliente).await,
        (Method::POST, "/api/extrair") => extrair_llm(req, &llm_cliente).await,
        (Method::POST, "/api/cupons") => salvar_cupom(req, &db).await,
        (Method::OPTIONS, _) => {
            let mut resposta = Response::new(Body::empty());
            *resposta.status_mut() = StatusCode::OK;
            resposta
                .headers_mut()
                .insert("access-control-allow-origin", "*".parse().unwrap());
            resposta.headers_mut().insert(
                "access-control-allow-methods",
                "GET, POST, OPTIONS".parse().unwrap(),
            );
            resposta.headers_mut().insert(
                "access-control-allow-headers",
                "content-type, authorization, x-pin, x-csrf-token"
                    .parse()
                    .unwrap(),
            );
            for (chave, valor) in cabecalhos_seguranca() {
                resposta.headers_mut().insert(chave, valor.parse().unwrap());
            }
            resposta
        }
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

    // Token CSRF inicial
    let csrf_token = Arc::new(Mutex::new(csrf_novo_token()));

    let estado = Arc::new(Mutex::new(ServerState {
        config: config.clone(),
        csrf_token,
        db: Arc::new(Mutex::new(db)),
        llm_cliente: Arc::new(Mutex::new(llm_cliente)),
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

fn csrf_novo_token() -> String {
    let agora = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{:x}", agora)
}

/// GET /api/saude — relata estado real dos componentes.
async fn saude(config: &Config, db: &Arc<Mutex<db::Repositorio>>) -> Response<Body> {
    let db = db.lock().await;
    let versao = db::Repositorio::versao_atual(&db.conn).unwrap_or(0);

    let base = config.llm_base_url.trim_end_matches('/');
    let url = format!("{}/health", base);

    let llm_status = match reqwest::get(url).await {
        Ok(resposta) => {
            if resposta.status().is_success() {
                "alcançável".to_string()
            } else {
                format!("indisponível (HTTP {})", resposta.status().as_u16())
            }
        }
        Err(_) => "indisponível".to_string(),
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

/// GET /api/cupons/{id} — detalhes de um cupom.
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
        Ok(Some(cupom)) => resposta_json(StatusCode::OK, &cupom),
        Ok(None) => {
            ApiError::novo("nao_encontrado", "Cupom não encontrado").resposta(StatusCode::NOT_FOUND)
        }
        Err(e) => {
            ApiError::novo("erro_banco", e.to_string()).resposta(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/relatorio?de=&ate= — relatório do período.
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

    let total_geral: f64 = cupons.iter().map(|c| c.total_cupom).sum();

    let filtrados: Vec<_> = cupons
        .into_iter()
        .filter(|c| {
            let ok_de = data_de.is_empty() || c.data_compra >= data_de;
            let ok_ate = data_ate.is_empty() || c.data_compra <= data_ate;
            ok_de && ok_ate
        })
        .collect();

    let total_cupons = filtrados.len();

    #[derive(Serialize)]
    struct Saida {
        periodo_de: String,
        periodo_ate: String,
        cupons: Vec<db::CupomResumo>,
        total_cupons: usize,
        total_geral: f64,
    }

    resposta_json(
        StatusCode::OK,
        &Saida {
            periodo_de: data_de,
            periodo_ate: data_ate,
            cupons: filtrados,
            total_cupons,
            total_geral,
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
    // Ler corpo da requisição
    let corpo = hyper::body::to_bytes(req.into_body())
        .await
        .unwrap_or_default();
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
            ApiError::novo(codigo, e.to_string()).resposta(status)
        }
    }
}

/// POST /api/cupons — salvar um cupom completo.
async fn salvar_cupom(req: Request<Body>, db: &Arc<Mutex<db::Repositorio>>) -> Response<Body> {
    let corpo = hyper::body::to_bytes(req.into_body())
        .await
        .unwrap_or_default();
    let cupom: db::CupomCompleto = match serde_json::from_slice(&corpo) {
        Ok(v) => v,
        Err(_) => {
            return ApiError::novo("json_invalido", "Corpo da requisição não é JSON válido")
                .resposta(StatusCode::BAD_REQUEST);
        }
    };

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
        Err(e) => {
            ApiError::novo("erro_banco", e.to_string()).resposta(StatusCode::INTERNAL_SERVER_ERROR)
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
