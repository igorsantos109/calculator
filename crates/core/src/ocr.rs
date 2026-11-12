//! OCR module for receipt scanning.
//!
//! This module provides functionality to extract text from receipt images
//! using the alvio-ocr crate (PP-OCRv6, ONNX) with basic preprocessing.

use alvio_ocr::{OcrEngine, OcrError as AlvioOcrError, OcrResult};
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgb};
use imageproc::contrast::adaptive_threshold;
use std::path::Path;
use thiserror::Error;

/// Erros tipados de OCR.
#[derive(Debug, Error)]
pub enum OcrError {
    #[error("Imagem inválida: o arquivo não é uma imagem válida ou está corrompida")]
    ImagemInvalida(#[from] image::ImageError),

    #[error("Falha no reconhecimento OCR: {0}")]
    FalhaReconhecimento(String),

    #[error("Caminho de imagem inválido: {0}")]
    CaminhoInvalido(String),

    #[error("Erro interno do motor OCR: {0}")]
    ErroMotor(#[from] AlvioOcrError),
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
/// `Ok(OcrOutput)` - Resultado completo do OCR
/// `Err(OcrError)` - Se houver falha na imagem ou reconhecimento
pub fn extrair_texto(caminho: &Path) -> Result<OcrOutput, OcrError> {
    // Validar que o caminho existe
    if !caminho.exists() {
        return Err(OcrError::CaminhoInvalido(caminho.display().to_string()));
    }

    // Carregar imagem
    let img = image::open(caminho)?;
    let (largura, altura) = img.dimensions();

    // Converter para grayscale (Luma<u8>)
    let gray = image::imageops::colorops::grayscale(&img.to_rgb8());

    // Aplicar binarização adaptativa para melhorar OCR
    // O block_radius 3 significa janela de 7x7 pixels
    let block_radius = 3u32;
    let binary = adaptive_threshold(&gray, block_radius);

    // Converter Luma<u8> binarizado para RGB para o alvio-ocr
    let binary_rgb: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::from_fn(binary.width(), binary.height(), |x, y| {
            let pixel = binary.get_pixel(x, y)[0];
            Rgb([pixel, pixel, pixel])
        });
    let dynamic_img = DynamicImage::ImageRgb8(binary_rgb);

    // Criar motor OCR com configuração padrão
    let engine = OcrEngine::auto().map_err(|e| {
        OcrError::FalhaReconhecimento(format!("Erro ao inicializar motor OCR: {}", e))
    })?;

    // Executar OCR
    let result: OcrResult = engine.recognize_image(&dynamic_img)?;

    // Extrair texto e confiança do resultado
    let texto = result.text.trim().to_string();
    let confianca_media = result.confidence as f64;

    // Log se nenhum texto foi reconhecido
    if texto.is_empty() {
        tracing::warn!(
            "Nenhum texto foi reconhecido na imagem: {}",
            caminho.display()
        );
    }

    Ok(OcrOutput {
        texto,
        confianca_media,
        largura: largura as usize,
        altura: altura as usize,
    })
}

/// Função auxiliar para extrair apenas o texto (compatibilidade)
pub fn extrair_texto_simples(caminho: &Path) -> Result<String, OcrError> {
    extrair_texto(caminho).map(|output| output.texto)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_caminho_invalido() {
        let resultado = extrair_texto(Path::new("/caminho/que/nao/existe.jpg"));
        assert!(resultado.is_err());
        match resultado {
            Err(OcrError::CaminhoInvalido(_)) => {}
            _ => panic!("Erro esperado: CaminhoInvalido"),
        }
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

        let output = extrair_texto(caminho).expect("Falha ao extrair texto");
        assert!(
            !output.texto.is_empty(),
            "Texto extraído deve ser não vazio"
        );
        assert!(
            output.texto.len() > 5,
            "Texto deve ter mais de 5 caracteres"
        );
        assert!(
            output.confianca_media >= 0.0 && output.confianca_media <= 1.0,
            "Confiança deve estar entre 0.0 e 1.0"
        );
    }

    #[test]
    fn test_opcoes_default() {
        let opcoes = OcrOptionsWrapper::default();
        assert_eq!(opcoes.cut_rate, 0.5);
        assert_eq!(opcoes.threads, 1);
    }
}
