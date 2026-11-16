//! Assets estáticos embutidos no binário (sem CDN).
//!
//! Os arquivos HTML/CSS/JS são incluídos em tempo de compilação via
//! `include_str!`, permitindo que o binário sirva a página interativa
//! sem depender de arquivos externos ou rede.

/// Página única (HTML/CSS/JS próprios).
pub const INDEX_HTML: &str = include_str!("assets/index.html");
pub const STYLE_CSS: &str = include_str!("assets/style.css");
pub const APP_JS: &str = include_str!("assets/app.js");
