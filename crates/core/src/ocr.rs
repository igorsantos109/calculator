//! OCR module for receipt scanning.
//! 
//! This module provides functionality to extract text from receipt images
//! using the alvio-ocr crate (PP-OCRv6, ONNX) with basic preprocessing.

use alvio_ocr::{OcrEngine, OcrOptions, OcrResult};
use image::{ImageBuffer, Rgb, Dynamic};
use imageproc::{
    drawing::DrawTarget,
    grayscale::grayscale,
    threshold::adaptive_threshold,
};
use thiserror::Error;
use std::path::Path;

/// Erros tipados de OCR.
#[derive(Debug, Error)]
pub enum OcrError {
    #[error("Imagem inválida: o arquivo não é uma imagem válida ou está corrompida")]
    ImagemInvalida(#[from] image::ImageError),

    #[error("Falha no reconhecimento OCR")]
    FalhaReconhecimento(String),

    #[error("Caminho de imagem inválido")]
    CaminhoInvalido,
}

/// Opções de configuração para OCR.
#[derive(Debug, Clone)]
pub struct OcrOptionsWrapper {
    /// Taxa de corte para binarização adaptativa (0.0 a 1.0)
    pub cut_rate: f64,
    /// Número de threads para OCR
    pub threads: usize,
}

impl Default for OcrOptionsWrapper {
    fn default() -> Self {
        OcrOptionsWrapper {
            cut_rate: 0.5,
            threads: 1,
        }
    }
}

/// Estrutura que encapsula o resultado do OCR.
#[derive(Debug, Clone)]
pub struct OcrOutput {
    /// Texto bruto extraído
    pub texto: String,
    /// Confiança média do reconhecimento (0.0 a 1.0)
    pub confianca_media: f64,
    /// Largura da imagem processada
    pub largura: usize,
    /// Altura da imagem processada
    pub altura: usize,
}

/// Função principal: extrai texto de uma imagem de cupom fiscal.
/// 
/// # Fluxo
/// 1. Carrega a imagem usando `image` crate
/// 2. Converte para escala de grayscale
/// 3. Aplica binarização adaptativa para melhorar OCR
/// 4. Envia ao motor OCR `alvio-ocr`
/// 
/// # Arguments
/// * `caminho` - Caminho para o arquivo de imagem (PNG, JPG, JPEG)
/// 
/// # Returns
/// `Ok(texto)` - Texto bruto extraído do cupom
/// `Err(OcrError)` - Se houver falha na imagem ou reconhecimento
pub fn extrair_texto(caminho: &Path) -> Result<String, OcrError> {
    // Validar que o caminho existe
    if !caminho.exists() {
        return Err(OcrError::CaminhoInvalido);
    }

    // Carregar imagem
    let img = match image::open(caminho) {
        Ok(img) => img,
        Err(e) => return Err(OcrError::ImagemInvalida(e)),
    };

    // Converter para RGB se necessário
    let img_rgb = img.to_rgb8();
    let (largura, altura) = img_rgb.dimensions();

    // 1. Converter para escala de grayscale
    let gray = grayscale(&img_rgb);

    // 2. Aplicar binarização adaptativa para melhorar OCR
    // Isso ajuda a separar o texto do fundo do cupom
    let binary = adaptive_threshold(&gray, (cut_rate * 255.0) as u8, 3);

    // 3. Preparar opções para o OCR
    let mut options = OcrOptions::default();
    options.cutm = Some(3); // Modelo de corte
    options.set_threads(threads);

    // 4. Executar OCR usando alvio-ocr
    let engine = OcrEngine::default();
    let result: OcrResult = engine.run_ocr(&binary, &options).map_err(|e| {
        OcrError::FalhaReconhecimento(format!("Erro OCR: {}", e))
    })?;

    // Extrair apenas o texto do resultado
    let texto = result.text.trim().to_string();

    // Retornar apenas se houver texto significativo
    if texto.is_empty() {
        warn!("Nenhum texto foi reconhecido na imagem");
    }

    Ok(texto)
}

/// Função auxiliar para criar opções padrão de OCR
fn get_default_options() -> OcrOptionsWrapper {
    OcrOptionsWrapper::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    #[should_panic]
    fn test_caminho_invalido() {
        let _ = extrair_texto(Path::new("/caminho/que/nao/existe.jpg"));
    }

    #[test]
    #[ignore]
    fn test_extrair_texto_com_fixture() {
        // Usar uma imagem de teste do repositório
        let caminho = Path::new("tests/fixtures/cupom_exemplo.jpg");
        
        if !caminho.exists() {
            // Se o fixture não existir, pular teste
            return;
        }

        let texto = extrair_texto(caminho).expect("Falha ao extrair texto");
        assert!(!texto.is_empty(), "Texto extraído deve ser não vazio");
        assert!(
            texto.len() > 5,
            "Texto deve ter mais de 5 caracteres"
        );
    }

    #[test]
    fn test_opcoes_default() {
        let opcoes = get_default_options();
        assert_eq!(opcoes.cut_rate, 0.5);
        assert_eq!(opcoes.threads, 1);
    }
}