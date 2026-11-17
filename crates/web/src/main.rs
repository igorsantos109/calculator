//! Ponto de entrada do servidor web do leitor de cupons fiscais.
//!
//! Execução:
//!   CUPONS_PIN=1234 cargo run -p web

use web::{config_from_env_validado, run};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = config_from_env_validado();

    // Aviso de risco se escutar em endereço LAN (não 127.0.0.1)
    if config.listen_addr.ip().is_unspecified() || !config.listen_addr.ip().is_loopback() {
        eprintln!(
            "AVISO DE SEGURANÇA: o servidor está escutando em {} (não apenas 127.0.0.1). \
             Acesso pela LAN expõe o PIN e os dados de cupons à rede local. \
             Somente use em rede interna confiável.",
            config.listen_addr
        );
    }

    run(config).await?;
    Ok(())
}
