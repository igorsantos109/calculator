//! Configuração do servidor web.

use std::net::SocketAddr;
use std::path::PathBuf;

/// Configuração do servidor web.
#[derive(Debug, Clone)]
pub struct Config {
    /// Endereço para escutar (padrão: 127.0.0.1:8080)
    pub listen_addr: SocketAddr,
    /// PIN/token de acesso (comparação em tempo constante, obrigatório, ≥ 6 caracteres)
    pub pin: String,
    /// Caminho para o banco de dados SQLite
    pub db_path: PathBuf,
    /// URL base do llama-server
    pub llm_base_url: String,
    /// Modelo/alias do llama-server
    pub llm_model: String,
    /// Nome da variável de ambiente com a chave do LLM
    pub llm_api_key_env: Option<String>,
    /// Pasta para imagens de OCR
    pub ocr_dir: PathBuf,
    /// Limite de upload em bytes (padrão: 10 MB)
    pub upload_limit: usize,
}

/// Tamanho mínimo do PIN (caracteres).
pub const PIN_MIN_LEN: usize = 6;

/// Erros de configuração.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("PIN de acesso é obrigatório. Defina CUPONS_PIN com pelo menos 6 caracteres.")]
    PinObrigatorio,
    #[error("PIN deve ter pelo menos {0} caracteres.")]
    PinCurto(usize),
}

impl Config {
    /// Valida a configuração (chamado antes de iniciar o servidor).
    pub fn validar(&self) -> Result<(), ConfigError> {
        if self.pin.is_empty() {
            return Err(ConfigError::PinObrigatorio);
        }
        if self.pin.len() < PIN_MIN_LEN {
            return Err(ConfigError::PinCurto(PIN_MIN_LEN));
        }
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            listen_addr: "127.0.0.1:8080".parse().expect("endereço padrão inválido"),
            pin: String::new(),
            db_path: PathBuf::from("cupons.db"),
            llm_base_url: "http://127.0.0.1:11434/v1".to_string(),
            llm_model: "llama-3.1-8b".to_string(),
            llm_api_key_env: None,
            ocr_dir: PathBuf::from("ocr"),
            upload_limit: 10 * 1024 * 1024,
        }
    }
}

/// Carrega configuração a partir de variáveis de ambiente.
pub fn config_from_env() -> Config {
    let mut cfg = Config::default();

    if let Ok(addr) = std::env::var("CUPONS_LISTEN") {
        if let Ok(parsed) = addr.parse::<SocketAddr>() {
            cfg.listen_addr = parsed;
        }
    }
    if let Ok(pin) = std::env::var("CUPONS_PIN") {
        cfg.pin = pin;
    }
    if let Ok(db) = std::env::var("CUPONS_DB") {
        cfg.db_path = PathBuf::from(db);
    }
    if let Ok(url) = std::env::var("LLM_BASE_URL") {
        cfg.llm_base_url = url;
    }
    if let Ok(model) = std::env::var("LLM_MODEL") {
        cfg.llm_model = model;
    }
    if let Ok(key_env) = std::env::var("LLM_API_KEY_ENV") {
        if !key_env.is_empty() {
            cfg.llm_api_key_env = Some(key_env);
        }
    }
    if let Ok(dir) = std::env::var("OCR_DIR") {
        cfg.ocr_dir = PathBuf::from(dir);
    }
    if let Ok(limit) = std::env::var("UPLOAD_LIMIT_MB") {
        if let Ok(mb) = limit.parse::<usize>() {
            cfg.upload_limit = mb * 1024 * 1024;
        }
    }

    cfg
}

/// Carrega configuração do ambiente e valida. Panic se o PIN estiver
/// ausente ou for curto (o servidor não inicia sem PIN válido).
pub fn config_from_env_validado() -> Config {
    let cfg = config_from_env();
    cfg.validar()
        .expect("CUPONS_PIN é obrigatório e deve ter pelo menos 6 caracteres");
    cfg
}
