//! Servidor HTTP do leitor de cupons fiscais.
//!
//! Reaproveita `receipt_core` e `db`. Escuta em endereço configurável
//! (padrão 127.0.0.1). API JSON com erros em pt-BR e códigos de erro.
//! Segurança: PIN/token com comparação em tempo constante, CSRF,
//! limite de upload, sanitização de nomes de arquivo, cabeçalhos de
//! segurança, sem CDN.

pub mod assets;
pub mod config;
pub mod security;
pub mod server;

pub use config::{config_from_env, config_from_env_validado, Config};
pub use server::{handle, run, ServerState};
