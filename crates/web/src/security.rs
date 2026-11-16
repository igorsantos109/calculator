//! Segurança do servidor web: PIN, CSRF, validação de upload, cabeçalhos.

use std::time::{SystemTime, UNIX_EPOCH};

/// Erros de segurança.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SecurityError {
    #[error("PIN ausente na requisição")]
    PinAusente,
    #[error("PIN inválido")]
    PinInvalido,
    #[error("Token CSRF ausente")]
    CsrfAusente,
    #[error("Token CSRF inválido")]
    CsrfInvalido,
    #[error("Upload excede o limite de {0} bytes")]
    UploadMuitoGrande(usize),
    #[error("Tipo de arquivo inválido: {0}")]
    TipoArquivoInvalido(String),
    #[error("Nome de arquivo inseguro: {0}")]
    NomeArquivoInseguro(String),
}

/// Comparação em tempo constante para evitar ataques de timing.
pub fn constante_tempo_igual(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut resultado = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) {
        resultado |= x ^ y;
    }
    resultado == 0
}

/// Valida o PIN informado contra o PIN configurado.
pub fn validar_pin(
    pin_fornecido: Option<&str>,
    pin_configurado: &str,
) -> Result<(), SecurityError> {
    let pin = pin_fornecido.ok_or(SecurityError::PinAusente)?;
    if constante_tempo_igual(pin, pin_configurado) {
        Ok(())
    } else {
        Err(SecurityError::PinInvalido)
    }
}

/// Gera um token CSRF simples baseado no tempo atual.
pub fn gerar_token_csrf() -> String {
    let agora = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{:x}", agora)
}

/// Valida o token CSRF (deve coincidir com o token da sessão/cookie).
pub fn validar_csrf(
    token_fornecido: Option<&str>,
    token_esperado: &str,
) -> Result<(), SecurityError> {
    let token = token_fornecido.ok_or(SecurityError::CsrfAusente)?;
    if constante_tempo_igual(token, token_esperado) {
        Ok(())
    } else {
        Err(SecurityError::CsrfInvalido)
    }
}

/// Valida o limite de upload.
pub fn validar_limite_upload(tamanho: usize, limite: usize) -> Result<(), SecurityError> {
    if tamanho > limite {
        return Err(SecurityError::UploadMuitoGrande(limite));
    }
    Ok(())
}

/// Valida o tipo de arquivo (somente png/jpg/jpeg).
pub const TIPOS_PERMITIDOS: &[&str] = &["image/png", "image/jpeg", "image/jpg"];

pub fn validar_tipo_arquivo(content_type: &str) -> Result<(), SecurityError> {
    let tipo = content_type.to_lowercase();
    if TIPOS_PERMITIDOS.iter().any(|t| tipo.contains(t)) {
        Ok(())
    } else {
        Err(SecurityError::TipoArquivoInvalido(content_type.to_string()))
    }
}

/// Sanitiza o nome do arquivo: remove caminho e caracteres perigosos.
pub fn sanear_nome_arquivo(nome: &str) -> Result<String, SecurityError> {
    if nome.is_empty() {
        return Err(SecurityError::NomeArquivoInseguro("vazio".to_string()));
    }
    // Pegar apenas o nome base (remover diretórios)
    let base = std::path::Path::new(nome)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| SecurityError::NomeArquivoInseguro(nome.to_string()))?;

    // Rejeitar nomes que contêm path traversal
    if nome.contains("..") || nome.contains('/') || nome.contains('\\') {
        return Err(SecurityError::NomeArquivoInseguro(nome.to_string()));
    }

    // Permitir apenas alfanumérico, ponto, underline, hífen
    if !base
        .chars()
        .all(|c| c.is_alphanumeric() || c == '.' || c == '_' || c == '-')
    {
        return Err(SecurityError::NomeArquivoInseguro(base.to_string()));
    }

    Ok(base.to_string())
}

/// Cabeçalhos de segurança para todas as respostas.
pub fn cabecalhos_seguranca() -> Vec<(&'static str, &'static str)> {
    vec![
        ("X-Content-Type-Options", "nosniff"),
        ("X-Frame-Options", "DENY"),
        ("Content-Security-Policy", "default-src 'self';"),
        ("Referrer-Policy", "no-referrer"),
        ("Cache-Control", "no-store"),
    ]
}
