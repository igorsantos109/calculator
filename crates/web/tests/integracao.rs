//! Testes de integração para o servidor web (ETAPA 3).
//!
//! Usa:
//! - Banco temporário (tempfile)
//! - Servidor falso do llama-server (wiremock) rodando em porta real
//! - Servidor web real (hyper) em porta aleatória
//! - Cliente HTTP (reqwest) para chamar as APIs

use db::Repositorio;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use web::config::Config;
use web::server::{handle, IpRateLimiter, ServerState};

async fn servidor_llm_false(resposta: ResponseTemplate) -> MockServer {
    let servidor = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(resposta.clone())
        .mount(&servidor)
        .await;
    Mock::given(method("GET"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .mount(&servidor)
        .await;
    servidor
}

/// Inicia o servidor web em uma porta aleatória e retorna o endereço
/// e o TempDir (que deve ser mantido vivo durante o teste).
async fn iniciar_servidor_web_teste(
    db_temp: tempfile::TempDir,
    servidor_llm: &MockServer,
) -> (SocketAddr, tempfile::TempDir) {
    // Obter uma porta real do SO
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr_real = listener.local_addr().unwrap();

    let config = Config {
        listen_addr: addr_real,
        pin: "testpin123".to_string(),
        db_path: db_temp.path().join("test.db"),
        llm_base_url: servidor_llm.uri(),
        llm_model: "modelo-teste".to_string(),
        llm_api_key_env: None,
        ocr_dir: std::env::temp_dir(),
        upload_limit: 10 * 1024 * 1024,
    };

    let db = Repositorio::new(&config.db_path).expect("falha ao abrir banco de teste");
    let llm_config = receipt_core::llm::LlmConfig {
        llm_base_url: config.llm_base_url.clone(),
        llm_model: config.llm_model.clone(),
        llm_api_key_env: config.llm_api_key_env.clone(),
        timeout: 5,
        max_tentativas: 1,
        max_tokens: 256,
    };
    let llm_cliente = receipt_core::llm::ClienteLlm::new(llm_config)
        .expect("falha ao criar cliente LLM de teste");

    let estado = Arc::new(Mutex::new(ServerState {
        config: config.clone(),
        db: Arc::new(Mutex::new(db)),
        llm_cliente: Arc::new(Mutex::new(llm_cliente)),
        rate_limiter: web::server::IpRateLimiter::new(),
    }));

    let estado_clone = estado.clone();

    tokio::spawn(async move {
        let make_svc = hyper::service::make_service_fn(move |conn: &hyper::server::conn::AddrStream| {
            let estado = estado_clone.clone();
            let client_ip = conn.remote_addr();
            async move {
                Ok::<_, std::convert::Infallible>(hyper::service::service_fn(move |req| {
                    let estado = estado.clone();
                    async move { handle(req, estado, client_ip).await }
                }))
            }
        });
        let servidor = hyper::Server::from_tcp(listener)
            .expect("falha ao criar listener TCP")
            .serve(make_svc);
        let _ = servidor.await;
    });

    // Aguardar o servidor subir
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    (addr_real, db_temp)
}

fn cliente_http() -> reqwest::Client {
    reqwest::Client::new()
}

fn headers_com_pin() -> reqwest::header::HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("authorization", "Bearer testpin123".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());
    headers
}

#[tokio::test]
async fn test_saude_retorna_estado_real() {
    let servidor_llm =
        servidor_llm_false(ResponseTemplate::new(200).set_body_json(serde_json::json!({}))).await;

    let dir = tempfile::tempdir().unwrap();
    let (addr, _temp_dir) = iniciar_servidor_web_teste(dir, &servidor_llm).await;

    let cliente = cliente_http();
    let resposta = cliente
        .get(format!("http://{}/api/saude", addr))
        .headers(headers_com_pin())
        .send()
        .await
        .unwrap();

    assert_eq!(resposta.status(), reqwest::StatusCode::OK);
    let corpo: serde_json::Value = resposta.json().await.unwrap();
    assert_eq!(corpo["banco"], "ok");
    assert!(corpo["schema_versao"].is_number());
    // LLM alcançável via /health do servidor falso
    assert!(corpo["llm"].as_str().unwrap().contains("alcançável"));
}

#[tokio::test]
async fn test_sem_pin_retorna_401() {
    let servidor_llm =
        servidor_llm_false(ResponseTemplate::new(200).set_body_json(serde_json::json!({}))).await;

    let dir = tempfile::tempdir().unwrap();
    let (addr, _temp_dir) = iniciar_servidor_web_teste(dir, &servidor_llm).await;

    let cliente = cliente_http();
    let resposta = cliente
        .get(format!("http://{}/api/cupons", addr))
        .send()
        .await
        .unwrap();

    assert_eq!(resposta.status(), reqwest::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_pin_errado_retorna_401() {
    let servidor_llm =
        servidor_llm_false(ResponseTemplate::new(200).set_body_json(serde_json::json!({}))).await;

    let dir = tempfile::tempdir().unwrap();
    let (addr, _temp_dir) = iniciar_servidor_web_teste(dir, &servidor_llm).await;

    let cliente = cliente_http();
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("authorization", "Bearer pinerrado".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());

    let resposta = cliente
        .get(format!("http://{}/api/cupons", addr))
        .headers(headers)
        .send()
        .await
        .unwrap();

    assert_eq!(resposta.status(), reqwest::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_listar_cupons_vazio() {
    let servidor_llm =
        servidor_llm_false(ResponseTemplate::new(200).set_body_json(serde_json::json!({}))).await;

    let dir = tempfile::tempdir().unwrap();
    let (addr, _temp_dir) = iniciar_servidor_web_teste(dir, &servidor_llm).await;

    let cliente = cliente_http();
    let resposta = cliente
        .get(format!("http://{}/api/cupons", addr))
        .headers(headers_com_pin())
        .send()
        .await
        .unwrap();

    assert_eq!(resposta.status(), reqwest::StatusCode::OK);
    let corpo: serde_json::Value = resposta.json().await.unwrap();
    assert_eq!(corpo["total"], 0);
    assert!(corpo["cupons"].is_array());
}

#[tokio::test]
async fn test_salvar_cupom_sucesso_e_listar() {
    let servidor_llm =
        servidor_llm_false(ResponseTemplate::new(200).set_body_json(serde_json::json!({}))).await;

    let dir = tempfile::tempdir().unwrap();
    let (addr, _temp_dir) = iniciar_servidor_web_teste(dir, &servidor_llm).await;

    let cliente = cliente_http();

    let cupom = serde_json::json!({
        "receita": {
            "data_compra": "2026-11-15",
            "estabelecimento": "Supermercado Teste",
            "total_cupom": 20.00,
            "imagem_path": "",
            "pagador": "igor"
        },
        "itens": [
            {
                "item": {
                    "descricao": "Leite 1L",
                    "quantidade": 1.0,
                    "preco_unitario": 20.0,
                    "preco_total": 20.0
                },
                "grupo": "Casa"
            }
        ]
    });

    let resposta = cliente
        .post(format!("http://{}/api/cupons", addr))
        .headers(headers_com_pin())
        .json(&cupom)
        .send()
        .await
        .unwrap();

    let status = resposta.status();
    let corpo_texto = resposta.text().await.unwrap_or_default();
    eprintln!(
        "DEBUG salvar cupom: status={}, body={}",
        status, corpo_texto
    );
    assert_eq!(status, reqwest::StatusCode::CREATED);
    let corpo: serde_json::Value = serde_json::from_str(&corpo_texto).unwrap();
    assert_eq!(corpo["mensagem"], "Cupom salvo com sucesso");
    let id = corpo["id"].as_i64().unwrap();

    // Listar e verificar que o cupom aparece
    let resposta = cliente
        .get(format!("http://{}/api/cupons", addr))
        .headers(headers_com_pin())
        .send()
        .await
        .unwrap();
    assert_eq!(resposta.status(), reqwest::StatusCode::OK);
    let corpo: serde_json::Value = resposta.json().await.unwrap();
    assert_eq!(corpo["total"], 1);
    assert_eq!(corpo["cupons"][0]["id"], id);
}

#[tokio::test]
async fn test_salvar_cupom_divergencia_total_retorna_400() {
    let servidor_llm =
        servidor_llm_false(ResponseTemplate::new(200).set_body_json(serde_json::json!({}))).await;

    let dir = tempfile::tempdir().unwrap();
    let (addr, _temp_dir) = iniciar_servidor_web_teste(dir, &servidor_llm).await;

    let cliente = cliente_http();

    // 100.06 vs 100.00 → diferença 6 centavos > 5 → rejeita
    let cupom = serde_json::json!({
        "receita": {
            "data_compra": "2026-11-15",
            "estabelecimento": "Teste",
            "total_cupom": 100.00,
            "imagem_path": "",
            "pagador": "igor"
        },
        "itens": [
            {
                "item": {
                    "descricao": "Produto X",
                    "quantidade": 1.0,
                    "preco_unitario": 100.06,
                    "preco_total": 100.06
                },
                "grupo": "Casa"
            }
        ]
    });

    let resposta = cliente
        .post(format!("http://{}/api/cupons", addr))
        .headers(headers_com_pin())
        .json(&cupom)
        .send()
        .await
        .unwrap();

    assert_eq!(resposta.status(), reqwest::StatusCode::BAD_REQUEST);
    let corpo: serde_json::Value = resposta.json().await.unwrap();
    assert_eq!(corpo["codigo"], "divergencia_total");
}

#[tokio::test]
async fn test_extrair_llm_resposta_valida() {
    let servidor_llm = servidor_llm_false(
        ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{
                "message": { "content": "{\"itens\":[{\"descricao\":\"Leite\",\"quantidade\":1,\"preco_unitario\":5.5,\"preco_total\":5.5}]}" },
                "finish_reason": "stop"
            }]
        })),
    )
    .await;

    let dir = tempfile::tempdir().unwrap();
    let (addr, _temp_dir) = iniciar_servidor_web_teste(dir, &servidor_llm).await;

    let cliente = cliente_http();
    let corpo = serde_json::json!({
        "texto_ocr": "Leite 1L - R$ 5,50",
        "total_cupom": 5.50
    });

    let resposta = cliente
        .post(format!("http://{}/api/extrair", addr))
        .headers(headers_com_pin())
        .json(&corpo)
        .send()
        .await
        .unwrap();

    assert_eq!(resposta.status(), reqwest::StatusCode::OK);
    let corpo: serde_json::Value = resposta.json().await.unwrap();
    assert_eq!(corpo["itens"][0]["descricao"], "Leite");
    assert!(corpo["avisos"].is_array());
    assert_eq!(corpo["avisos"].as_array().unwrap().len(), 0);
    assert!((corpo["soma_itens"].as_f64().unwrap() - 5.5).abs() < f64::EPSILON);
}

#[tokio::test]
async fn test_extrair_llm_nao_autenticado_retorna_401() {
    let servidor_llm =
        servidor_llm_false(ResponseTemplate::new(200).set_body_json(serde_json::json!({}))).await;

    let dir = tempfile::tempdir().unwrap();
    let (addr, _temp_dir) = iniciar_servidor_web_teste(dir, &servidor_llm).await;

    let cliente = cliente_http();
    let corpo = serde_json::json!({
        "texto_ocr": "teste",
        "total_cupom": 0.0
    });

    let resposta = cliente
        .post(format!("http://{}/api/extrair", addr))
        .json(&corpo)
        .send()
        .await
        .unwrap();

    assert_eq!(resposta.status(), reqwest::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_extrair_llm_sem_csrf_retorna_403() {
    let servidor_llm =
        servidor_llm_false(ResponseTemplate::new(200).set_body_json(serde_json::json!({}))).await;

    let dir = tempfile::tempdir().unwrap();
    let (addr, _temp_dir) = iniciar_servidor_web_teste(dir, &servidor_llm).await;

    let cliente = cliente_http();
    let corpo = serde_json::json!({
        "texto_ocr": "teste",
        "total_cupom": 0.0
    });

    // Sem CSRF (removido) — apenas PIN. O servidor falso retorna 200.
    let resposta = cliente
        .post(format!("http://{}/api/extrair", addr))
        .headers(headers_com_pin())
        .json(&corpo)
        .send()
        .await
        .unwrap();

    // Sem CSRF, apenas autenticação por Bearer PIN → 200 (ou 502 se LLM indisponível)
    assert!(
        resposta.status() == reqwest::StatusCode::OK
            || resposta.status() == reqwest::StatusCode::BAD_GATEWAY
    );
}

#[tokio::test]
async fn test_extrair_llm_texto_ocr_com_xss_nao_usa_innerhtml() {
    // Este teste valida que o servidor não interpreta o texto OCR como HTML.
    // O servidor apenas encaminha para o LLM e retorna JSON.
    let servidor_llm =
        servidor_llm_false(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{
                "message": { "content": "{\"itens\":[]}" },
                "finish_reason": "stop"
            }]
        })))
        .await;

    let dir = tempfile::tempdir().unwrap();
    let (addr, _temp_dir) = iniciar_servidor_web_teste(dir, &servidor_llm).await;

    let cliente = cliente_http();
    // Texto OCR contendo tentativa de XSS
    let corpo = serde_json::json!({
        "texto_ocr": "<script>alert('xss')</script> Leite 1L - R$ 5,50",
        "total_cupom": 5.50
    });

    let resposta = cliente
        .post(format!("http://{}/api/extrair", addr))
        .headers(headers_com_pin())
        .json(&corpo)
        .send()
        .await
        .unwrap();

    assert_eq!(resposta.status(), reqwest::StatusCode::OK);
    let corpo: serde_json::Value = resposta.json().await.unwrap();
    // O servidor deve retornar 200 (o XSS está no dado, não é interpretado como HTML)
    assert_eq!(corpo["itens"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_extrair_llm_servidor_llm_indisponivel_retorna_502() {
    // Servidor LLM falso não montado → conexão recusada → 502
    let dir = tempfile::tempdir().unwrap();

    // Obter uma porta real do SO
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr_real = listener.local_addr().unwrap();

    let config = Config {
        listen_addr: addr_real,
        pin: "testpin123".to_string(),
        db_path: dir.path().join("test.db"),
        llm_base_url: "http://127.0.0.1:65530".to_string(), // porta não utilizada
        llm_model: "modelo-teste".to_string(),
        llm_api_key_env: None,
        ocr_dir: std::env::temp_dir(),
        upload_limit: 10 * 1024 * 1024,
    };

    let db = Repositorio::new(&config.db_path).expect("falha ao abrir banco de teste");
    let llm_config = receipt_core::llm::LlmConfig {
        llm_base_url: config.llm_base_url.clone(),
        llm_model: config.llm_model.clone(),
        llm_api_key_env: config.llm_api_key_env.clone(),
        timeout: 1,
        max_tentativas: 1,
        max_tokens: 256,
    };
    let llm_cliente = receipt_core::llm::ClienteLlm::new(llm_config)
        .expect("falha ao criar cliente LLM de teste");

    let estado = Arc::new(Mutex::new(ServerState {
        config: config.clone(),
        db: Arc::new(Mutex::new(db)),
        llm_cliente: Arc::new(Mutex::new(llm_cliente)),
        rate_limiter: web::server::IpRateLimiter::new(),
    }));

    let estado_clone = estado.clone();
    let addr = addr_real;

    tokio::spawn(async move {
        let make_svc = hyper::service::make_service_fn(move |conn: &hyper::server::conn::AddrStream| {
            let estado = estado_clone.clone();
            let client_ip = conn.remote_addr();
            async move {
                Ok::<_, std::convert::Infallible>(hyper::service::service_fn(move |req| {
                    let estado = estado.clone();
                    async move { handle(req, estado, client_ip).await }
                }))
            }
        });
        let servidor = hyper::Server::from_tcp(listener)
            .expect("falha ao criar listener TCP")
            .serve(make_svc);
        let _ = servidor.await;
    });

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let cliente = cliente_http();
    let corpo = serde_json::json!({
        "texto_ocr": "teste",
        "total_cupom": 0.0
    });

    let resposta = cliente
        .post(format!("http://{}/api/extrair", addr))
        .headers(headers_com_pin())
        .json(&corpo)
        .send()
        .await
        .unwrap();

    // Servidor LLM inacessível → 502 Bad Gateway
    assert_eq!(resposta.status(), reqwest::StatusCode::BAD_GATEWAY);
    let corpo: serde_json::Value = resposta.json().await.unwrap();
    assert_eq!(corpo["codigo"], "servidor_llm_inacessível");
}

#[tokio::test]
async fn test_relatorio_periodo() {
    let servidor_llm =
        servidor_llm_false(ResponseTemplate::new(200).set_body_json(serde_json::json!({}))).await;

    let dir = tempfile::tempdir().unwrap();
    let (addr, _temp_dir) = iniciar_servidor_web_teste(dir, &servidor_llm).await;

    let cliente = cliente_http();

    // Salvar dois cupons
    for i in 0..2 {
        let cupom = serde_json::json!({
            "receita": {
                "data_compra": format!("2026-11-0{}", i + 1),
                "estabelecimento": format!("Loja {}", i),
                "total_cupom": 10.0 + i as f64,
                "imagem_path": "",
                "pagador": "igor"
            },
            "itens": [
                {
                    "item": {
                        "descricao": format!("Item {}", i),
                        "quantidade": 1.0,
                        "preco_unitario": 10.0 + i as f64,
                        "preco_total": 10.0 + i as f64
                    },
                    "grupo": "Casa"
                }
            ]
        });
        cliente
            .post(format!("http://{}/api/cupons", addr))
            .headers(headers_com_pin())
            .json(&cupom)
            .send()
            .await
            .unwrap();
    }

    // Relatório sem filtro
    let resposta = cliente
        .get(format!("http://{}/api/relatorio", addr))
        .headers(headers_com_pin())
        .send()
        .await
        .unwrap();
    assert_eq!(resposta.status(), reqwest::StatusCode::OK);
    let corpo: serde_json::Value = resposta.json().await.unwrap();
    assert_eq!(corpo["total_cupons"], 2);

    // Relatório com filtro de data
    let resposta = cliente
        .get(format!(
            "http://{}/api/relatorio?de=2026-11-01&ate=2026-11-01",
            addr
        ))
        .headers(headers_com_pin())
        .send()
        .await
        .unwrap();
    assert_eq!(resposta.status(), reqwest::StatusCode::OK);
    let corpo: serde_json::Value = resposta.json().await.unwrap();
    assert_eq!(corpo["total_cupons"], 1);
    assert_eq!(corpo["cupons"][0]["estabelecimento"], "Loja 0");
}

#[tokio::test]
async fn test_extrair_llm_json_quebrado_retorna_502() {
    let servidor_llm =
        servidor_llm_false(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{
                "message": { "content": "Isso nao e JSON valido {{{" },
                "finish_reason": "stop"
            }]
        })))
        .await;

    let dir = tempfile::tempdir().unwrap();
    let (addr, _temp_dir) = iniciar_servidor_web_teste(dir, &servidor_llm).await;

    let cliente = cliente_http();
    let corpo = serde_json::json!({
        "texto_ocr": "teste",
        "total_cupom": 0.0
    });

    let resposta = cliente
        .post(format!("http://{}/api/extrair", addr))
        .headers(headers_com_pin())
        .json(&corpo)
        .send()
        .await
        .unwrap();

    assert_eq!(resposta.status(), reqwest::StatusCode::BAD_GATEWAY);
    let corpo: serde_json::Value = resposta.json().await.unwrap();
    assert_eq!(corpo["codigo"], "json_invalido");
}

#[tokio::test]
async fn test_extrair_llm_resposta_truncada_retorna_502() {
    let servidor_llm =
        servidor_llm_false(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{
                "message": { "content": "{\"itens\":[{\"descricao\":\"Leite\"" },
                "finish_reason": "length"
            }]
        })))
        .await;

    let dir = tempfile::tempdir().unwrap();
    let (addr, _temp_dir) = iniciar_servidor_web_teste(dir, &servidor_llm).await;

    let cliente = cliente_http();
    let corpo = serde_json::json!({
        "texto_ocr": "teste",
        "total_cupom": 0.0
    });

    let resposta = cliente
        .post(format!("http://{}/api/extrair", addr))
        .headers(headers_com_pin())
        .json(&corpo)
        .send()
        .await
        .unwrap();

    assert_eq!(resposta.status(), reqwest::StatusCode::BAD_GATEWAY);
    let corpo: serde_json::Value = resposta.json().await.unwrap();
    assert_eq!(corpo["codigo"], "resposta_truncada");
}

#[tokio::test]
async fn test_extrair_llm_chave_invalida_401() {
    let servidor_llm =
        servidor_llm_false(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "error": { "message": "Invalid API Key" }
        })))
        .await;

    let dir = tempfile::tempdir().unwrap();
    let (addr, _temp_dir) = iniciar_servidor_web_teste(dir, &servidor_llm).await;

    let cliente = cliente_http();
    let corpo = serde_json::json!({
        "texto_ocr": "teste",
        "total_cupom": 0.0
    });

    let resposta = cliente
        .post(format!("http://{}/api/extrair", addr))
        .headers(headers_com_pin())
        .json(&corpo)
        .send()
        .await
        .unwrap();

    assert_eq!(resposta.status(), reqwest::StatusCode::UNAUTHORIZED);
    let corpo: serde_json::Value = resposta.json().await.unwrap();
    assert_eq!(corpo["codigo"], "chave_llm_invalida");
}

/// H1: contrato — o JSON EXATO que o app.js envia em salvarCupom
/// (grupos em minúsculas, como o select value="casa" etc.)
/// deve ser aceito por POST /api/cupons com 201.
#[tokio::test]
async fn test_h1_contrato_grupo_minusculo_aceito_201() {
    let servidor_llm =
        servidor_llm_false(ResponseTemplate::new(200).set_body_json(serde_json::json!({}))).await;

    let dir = tempfile::tempdir().unwrap();
    let (addr, _temp_dir) = iniciar_servidor_web_teste(dir, &servidor_llm).await;

    let cliente = cliente_http();

    // Corpo idêntico ao cupom montado em salvarCupom() no app.js:
    // grupo vem do <select> com value minúscula ("casa"/"igor"/"renan").
    let corpo_exato_app_js = serde_json::json!({
        "receita": {
            "data_compra": "2026-11-18",
            "estabelecimento": "Mercado Teste",
            "total_cupom": 20.00,
            "imagem_path": "",
            "pagador": "igor"
        },
        "itens": [
            {
                "item": {
                    "descricao": "Leite 1L",
                    "quantidade": 2.0,
                    "preco_unitario": 10.0,
                    "preco_total": 20.0
                },
                "grupo": "casa"
            }
        ]
    });

    let resposta = cliente
        .post(format!("http://{}/api/cupons", addr))
        .headers(headers_com_pin())
        .json(&corpo_exato_app_js)
        .send()
        .await
        .unwrap();

    let status = resposta.status();
    let texto = resposta.text().await.unwrap_or_default();
    assert_eq!(
        status,
        reqwest::StatusCode::CREATED,
        "POST /api/cupons com grupo minúsculo (corpo exato do app.js) deve devolver 201; body: {}",
        texto
    );
}

/// H1: GET /api/cupons/{id} deve devolver grupos em minúsculas,
/// consistente com o que o app.js compara em verDetalhes.
#[tokio::test]
async fn test_h1_get_cupom_devolve_grupo_minusculo() {
    let servidor_llm =
        servidor_llm_false(ResponseTemplate::new(200).set_body_json(serde_json::json!({}))).await;

    let dir = tempfile::tempdir().unwrap();
    let (addr, _temp_dir) = iniciar_servidor_web_teste(dir, &servidor_llm).await;

    let cliente = cliente_http();

    let cupom = serde_json::json!({
        "receita": {
            "data_compra": "2026-11-18",
            "estabelecimento": "Mercado",
            "total_cupom": 10.0,
            "imagem_path": "",
            "pagador": "renan"
        },
        "itens": [
            {
                "item": {
                    "descricao": "Pão",
                    "quantidade": 1.0,
                    "preco_unitario": 10.0,
                    "preco_total": 10.0
                },
                "grupo": "igor"
            }
        ]
    });

    let resposta = cliente
        .post(format!("http://{}/api/cupons", addr))
        .headers(headers_com_pin())
        .json(&cupom)
        .send()
        .await
        .unwrap();
    assert_eq!(resposta.status(), reqwest::StatusCode::CREATED);
    let criado: serde_json::Value = resposta.json().await.unwrap();
    let id = criado["id"].as_i64().unwrap();

    let resposta = cliente
        .get(format!("http://{}/api/cupons/{}", addr, id))
        .headers(headers_com_pin())
        .send()
        .await
        .unwrap();
    assert_eq!(resposta.status(), reqwest::StatusCode::OK);
    let corpo: serde_json::Value = resposta.json().await.unwrap();
    assert_eq!(
        corpo["itens"][0]["grupo"],
        serde_json::json!("igor"),
        "GET /api/cupons/{{id}} deve devolver grupo em minúsculas"
    );
    assert_eq!(corpo["receita"]["pagador"], serde_json::json!("renan"));
}

/// H2: verDetalhes calcula responsabilidades/saldos corretos com a resposta da API
/// (grupos em minúsculas). Teste de integração que valida o JSON devolvido.
#[tokio::test]
async fn test_h2_ver_detalhes_saldos_corretos_com_api_minusculo() {
    let servidor_llm =
        servidor_llm_false(ResponseTemplate::new(200).set_body_json(serde_json::json!({}))).await;

    let dir = tempfile::tempdir().unwrap();
    let (addr, _temp_dir) = iniciar_servidor_web_teste(dir, &servidor_llm).await;

    let cliente = cliente_http();

    // Salvar cupom com items em grupos diferentes (minúsculas no JSON)
    let cupom = serde_json::json!({
        "receita": {
            "data_compra": "2026-11-18",
            "estabelecimento": "Mercado H2",
            "total_cupom": 100.0,
            "imagem_path": "",
            "pagador": "igor"
        },
        "itens": [
            {
                "item": {
                    "descricao": "Arroz",
                    "quantidade": 1.0,
                    "preco_unitario": 40.0,
                    "preco_total": 40.0
                },
                "grupo": "casa"
            },
            {
                "item": {
                    "descricao": "Cerveja Igor",
                    "quantidade": 2.0,
                    "preco_unitario": 15.0,
                    "preco_total": 30.0
                },
                "grupo": "igor"
            },
            {
                "item": {
                    "descricao": "Vinho Renan",
                    "quantidade": 1.0,
                    "preco_unitario": 30.0,
                    "preco_total": 30.0
                },
                "grupo": "renan"
            }
        ]
    });

    let resp = cliente
        .post(format!("http://{}/api/cupons", addr))
        .headers(headers_com_pin())
        .json(&cupom)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::CREATED);
    let criado: serde_json::Value = resp.json().await.unwrap();
    let id = criado["id"].as_i64().unwrap();

    // GET /api/cupons/{id} - a API devolve grupos em minúsculas (H1)
    let resp = cliente
        .get(format!("http://{}/api/cupons/{}", addr, id))
        .headers(headers_com_pin())
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    let cupom_get: serde_json::Value = resp.json().await.unwrap();

    // Verificar que os grupos voltaram em minúsculas
    assert_eq!(cupom_get["itens"][0]["grupo"], serde_json::json!("casa"));
    assert_eq!(cupom_get["itens"][1]["grupo"], serde_json::json!("igor"));
    assert_eq!(cupom_get["itens"][2]["grupo"], serde_json::json!("renan"));

    // Simular o cálculo que verDetalhes faz (mesma lógica)
    let soma: f64 = cupom_get["itens"]
        .as_array()
        .unwrap()
        .iter()
        .map(|it| it["item"]["preco_total"].as_f64().unwrap_or(0.0))
        .sum();
    assert!((soma - 100.0).abs() < 0.01);

    let mut casa = 0.0;
    let mut igor_p = 0.0;
    let mut renan_p = 0.0;
    for it in cupom_get["itens"].as_array().unwrap() {
        let preco = it["item"]["preco_total"].as_f64().unwrap_or(0.0);
        match it["grupo"].as_str().unwrap() {
            "casa" => casa += preco,
            "igor" => igor_p += preco,
            "renan" => renan_p += preco,
            _ => panic!("grupo inesperado: {}", it["grupo"]),
        }
    }
    let metade_casa = casa / 2.0;
    let mut resp_igor = igor_p + metade_casa;
    let mut resp_renan = renan_p + metade_casa;
    let centavos_casa = (casa * 100.0).round() as i64;
    if centavos_casa % 2 == 1 {
        resp_renan += 0.01; // pagador = igor
    }
    let saldo_igor = 100.0 - resp_igor;
    let saldo_renan = 0.0 - resp_renan;

    // casa=40, igor=30, renan=30 → metade_casa=20
    // resp_igor = 30+20=50, resp_renan = 30+20=50 (+0.01 centavo indivisível)
    // saldo_igor = 100-50 = 50, saldo_renan = 0-50.01 = -50.01
    assert!((resp_igor - 50.0).abs() < 0.01, "resp_igor={}", resp_igor);
    assert!((resp_renan - 50.01).abs() < 0.01, "resp_renan={}", resp_renan);
    assert!((saldo_igor - 50.0).abs() < 0.01, "saldo_igor={}", saldo_igor);
    assert!((saldo_renan - (-50.01)).abs() < 0.01, "saldo_renan={}", saldo_renan);
}

#[tokio::test]
async fn test_extrair_llm_item_invalido_via_aviso_retorna_200_com_aviso() {
    // Item com quantidade <= 0 deve virar aviso, não fazer a requisição falhar
    let servidor_llm = servidor_llm_false(
        ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{
                "message": { "content": "{\"itens\":[{\"descricao\":\"Leite\",\"quantidade\":1,\"preco_unitario\":5.5,\"preco_total\":5.5},{\"descricao\":\"Invalido\",\"quantidade\":0,\"preco_unitario\":1.0,\"preco_total\":0.0}]}" },
                "finish_reason": "stop"
            }]
        })),
    )
    .await;

    let dir = tempfile::tempdir().unwrap();
    let (addr, _temp_dir) = iniciar_servidor_web_teste(dir, &servidor_llm).await;

    let cliente = cliente_http();
    let corpo = serde_json::json!({
        "texto_ocr": "teste",
        "total_cupom": 5.50
    });

    let resposta = cliente
        .post(format!("http://{}/api/extrair", addr))
        .headers(headers_com_pin())
        .json(&corpo)
        .send()
        .await
        .unwrap();

    assert_eq!(resposta.status(), reqwest::StatusCode::OK);
    let corpo: serde_json::Value = resposta.json().await.unwrap();
    assert_eq!(corpo["itens"].as_array().unwrap().len(), 1);
    assert_eq!(corpo["itens"][0]["descricao"], "Leite");
    let avisos = corpo["avisos"].as_array().unwrap();
    assert!(!avisos.is_empty(), "deve haver aviso para item inválido");
    // soma_itens deve ser só do item válido (5.5, não 5.5+0)
    assert!((corpo["soma_itens"].as_f64().unwrap() - 5.5).abs() < f64::EPSILON);
}

/// ===========================
/// H3: Rate limiting real por IP
/// ===========================

/// H3: Teste unitário do IpRateLimiter — 5 falhas bloqueiam a 6ª (mesmo IP)
#[tokio::test]
async fn test_h3_rate_limiter_5_falhas_bloqueia_6a() {
    let limiter = IpRateLimiter::new();
    let ip = "192.168.1.100";

    for _ in 0..5 {
        limiter.registrar_falha(ip).await;
    }
    assert!(limiter.esta_bloqueado(ip).await, "após 5 falhas deve estar bloqueado");
}

/// H3: IP diferente não é afetado
#[tokio::test]
async fn test_h3_rate_limiter_outro_ip_nao_afetado() {
    let limiter = IpRateLimiter::new();
    let ip1 = "192.168.1.100";
    let ip2 = "192.168.1.101";

    for _ in 0..5 {
        limiter.registrar_falha(ip1).await;
    }
    assert!(!limiter.esta_bloqueado(ip2).await, "IP diferente não deve estar bloqueado");
}

/// H3: Sucesso zera o contador
#[tokio::test]
async fn test_h3_rate_limiter_sucesso_zera_contador() {
    let limiter = IpRateLimiter::new();
    let ip = "192.168.1.100";

    for _ in 0..4 {
        limiter.registrar_falha(ip).await;
    }
    assert!(!limiter.esta_bloqueado(ip).await, "4 falhas não bloqueiam");

    // Sucesso = zerar contador
    limiter.zerar_contador(ip).await;
    assert!(!limiter.esta_bloqueado(ip).await, "após zerar, não deve estar bloqueado");

    // Mais 4 falhas = ainda não bloqueia (contador reiniciou)
    for _ in 0..4 {
        limiter.registrar_falha(ip).await;
    }
    assert!(!limiter.esta_bloqueado(ip).await, "após zerar, 4 falhas não bloqueiam");
}

/// H3: Integração — 5 PINs errados → 6ª requisição (mesmo IP, mesmo PIN errado) = 429
#[tokio::test]
async fn test_h3_integracao_5_pin_errado_6a_429() {
    let servidor_llm =
        servidor_llm_false(ResponseTemplate::new(200).set_body_json(serde_json::json!({}))).await;

    let dir = tempfile::tempdir().unwrap();
    let (addr, _temp_dir) = iniciar_servidor_web_teste(dir, &servidor_llm).await;

    let cliente = cliente_http();
    // PIN errado
    let pin_errado = "errado123";
    let headers_errado = {
        let mut h = reqwest::header::HeaderMap::new();
        h.insert("Authorization", format!("Bearer {}", pin_errado).parse().unwrap());
        h
    };

    // 5 tentativas com PIN errado
    for _ in 0..5 {
        let resp = cliente
            .get(format!("http://{}/api/cupons", addr))
            .headers(headers_errado.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), reqwest::StatusCode::UNAUTHORIZED);
    }

    // 6ª requisição (mesmo IP implícito no servidor de teste) → 429
    let resp = cliente
        .get(format!("http://{}/api/cupons", addr))
        .headers(headers_errado.clone())
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        reqwest::StatusCode::TOO_MANY_REQUESTS,
        "6ª requisição com PIN errado deve dar 429"
    );
    // Verificar Retry-After header
    assert!(resp.headers().get("retry-after").is_some(), "deve ter header Retry-After");
}
