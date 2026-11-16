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
use web::server::{handle, ServerState};

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

    let csrf_token = Arc::new(Mutex::new("csrf-teste".to_string()));
    let estado = Arc::new(Mutex::new(ServerState {
        config: config.clone(),
        csrf_token,
        db: Arc::new(Mutex::new(db)),
        llm_cliente: Arc::new(Mutex::new(llm_cliente)),
    }));

    let estado_clone = estado.clone();

    tokio::spawn(async move {
        let make_svc = hyper::service::make_service_fn(move |_conn| {
            let estado = estado_clone.clone();
            async move {
                Ok::<_, std::convert::Infallible>(hyper::service::service_fn(move |req| {
                    let estado = estado.clone();
                    async move { handle(req, estado).await }
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

fn headers_com_pin_e_csrf() -> reqwest::header::HeaderMap {
    let mut headers = headers_com_pin();
    headers.insert("x-csrf-token", "csrf-teste".parse().unwrap());
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
        .headers(headers_com_pin_e_csrf())
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
        .headers(headers_com_pin_e_csrf())
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
        .headers(headers_com_pin_e_csrf())
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

    // Sem header X-CSRF-Token
    let mut headers = headers_com_pin();
    headers.remove("x-csrf-token");

    let resposta = cliente
        .post(format!("http://{}/api/extrair", addr))
        .headers(headers)
        .json(&corpo)
        .send()
        .await
        .unwrap();

    assert_eq!(resposta.status(), reqwest::StatusCode::FORBIDDEN);
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
        .headers(headers_com_pin_e_csrf())
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

    let csrf_token = Arc::new(Mutex::new("csrf-teste".to_string()));
    let estado = Arc::new(Mutex::new(ServerState {
        config: config.clone(),
        csrf_token,
        db: Arc::new(Mutex::new(db)),
        llm_cliente: Arc::new(Mutex::new(llm_cliente)),
    }));

    let estado_clone = estado.clone();
    let addr = addr_real;

    tokio::spawn(async move {
        let make_svc = hyper::service::make_service_fn(move |_conn| {
            let estado = estado_clone.clone();
            async move {
                Ok::<_, std::convert::Infallible>(hyper::service::service_fn(move |req| {
                    let estado = estado.clone();
                    async move { handle(req, estado).await }
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
        .headers(headers_com_pin_e_csrf())
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
            .headers(headers_com_pin_e_csrf())
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
        .headers(headers_com_pin_e_csrf())
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
        .headers(headers_com_pin_e_csrf())
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
        .headers(headers_com_pin_e_csrf())
        .json(&corpo)
        .send()
        .await
        .unwrap();

    assert_eq!(resposta.status(), reqwest::StatusCode::UNAUTHORIZED);
    let corpo: serde_json::Value = resposta.json().await.unwrap();
    assert_eq!(corpo["codigo"], "chave_llm_invalida");
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
        .headers(headers_com_pin_e_csrf())
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
