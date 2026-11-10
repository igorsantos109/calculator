//! Cliente LLM para extração de dados de cupons fiscais.
//! 
//! Este módulo fornece a funcionalidade de enviar texto OCR ao Ollama local
//! e fazer o parsing robusto do JSON retornado.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use std::time::Duration;
use reqwest::Client;
use tracing::{info, warn, debug};

/// Erros tipados de cliente LLM.
#[derive(Debug, Error)]
pub enum LlmError {
    #[error("Erro de conexão ao Ollama: {0}")]
    Conexao(#[from] reqwest::Error),

    #[error("Resposta inválida do Ollama: {0}")]
    RespostaInvalida(String),

    #[error("Item interno inválido: {0}")]
    ItemInvalido(String),

    #[error("Timeout ao conectar ao Ollama")]
    Timeout,
}

/// Opções de configuração para o cliente LLM.
#[derive(Debug, Clone)]
pub struct LlmOptions {
    /// URL base do Ollama (padrão: http://localhost:11434)
    pub base_url: String,
    /// Modelo a ser usado (padrão: qwen2.5:7b-instruct-q4_K_M)
    pub modelo: String,
    /// Timeout em segundos (padrão: 120)
    pub timeout: u64,
    /// Número máximo de tentativas
    pub max_tentativas: usize,
}

impl Default for LlmOptions {
    fn default() -> Self {
        LlmOptions {
            base_url: "http://localhost:11434".to_string(),
            modelo: "qwen2.5:7b-instruct-q4_K_M".to_string(),
            timeout: 120,
            max_tentativas: 1,
        }
    }
}

/// Resposta esperada do LLM (JSON estruturado).
#[derive(Debug, Deserialize)]
struct RespostaLlm {
    itens: Vec<ItemLlm>,
}

/// Item individual na resposta do LLM.
#[derive(Debug, Deserialize)]
struct ItemLlm {
    descricao: String,
    quantidade: f64,
    preco_unitario: f64,
    preco_total: f64,
}

/// Resultado da extração de itens do LLM.
#[derive(Debug, Clone)]
pub struct ResultadoLlm {
    pub itens: Vec<core::Item>,
    pub texto_ocr: String,
}

/// Cliente para comunicação com Ollama via HTTP.
pub struct ClienteLlm {
    client: Client,
    options: LlmOptions,
    endpoint: String,
}

impl ClienteLlm {
    /// Cria um novo cliente LLM com as opções configuradas.
    pub fn new(options: LlmOptions) -> Result<Self, LlmError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(options.timeout))
            .build()
            .map_err(|e| LlmError::Conexao(e))?;

        let endpoint = format!("{}/api/generate", options.base_url);

        Ok(ClienteLlm {
            client,
            options,
            endpoint,
        })
    }

    /// Extrai itens do texto OCR usando o LLM local.
    /// 
    /// # Fluxo
    /// 1. Monta o prompt fixo definido em SPEC.md
    /// 2. Envia para Ollama via POST
    /// 3. Faz parsing robusto do JSON retornado (tolerante a ```json)
    /// 4. Valida que cada item tem campos numéricos > 0
    /// 5. Retorna `Vec<Item>` estruturado
    /// 
    /// # Arguments
    /// * `texto_ocr` - Texto bruto extraído pelo OCR
    /// 
    /// # Returns
    /// `Ok(ResultadoLlm)` com itens extraídos
    /// `Err(LlmError)` - Se houver conexão, resposta inválida ou itens inválidos
    pub async fn extrair_itens(&self, texto_ocr: &str) -> Result<ResultadoLlm, LlmError> {
        // 1. Monta o prompt com o texto OCR
        let prompt = self.monta_prompt(texto_ocr);

        // 2. Envia para Ollama com tentativas
        let mut tentativa_atual = 0;
        let mut resposta_llm = None;

        while tentativa_atual < self.options.max_tentativas {
            tentativa_atual += 1;

            match self.envia_para_ollama(&prompt).await {
                Ok(resposta) => {
                    resposta_llm = Some(resposta);
                    break;
                }
                Err(e) => {
                    warn!("Tentativa {} falhou: {}", tentativa_atual, e);
                    if tentativa_atual >= self.options.max_tentativas {
                        return Err(LlmError::Timeout);
                    }
                }
            }
        }

        let resposta_llm = match resposta_llm {
            Some(r) => r,
            None => return Err(LlmError::Timeout),
        };

        // 3. Faz parsing do JSON retornado
        let item_result = self.parse_resposta_llm(&resposta_llm);

        // 4. Valida cada item
        let itens_validados: Vec<core::Item> = item_result
            .into_iter()
            .filter(|item| {
                item.quantidade > 0.0 && item.preco_unitario >= 0.0 && item.preco_total >= 0.0
            })
            .collect();

        // 5. Verificar se algum item foi rejeitado
        if itens_validados.len() != item_result.len() {
            warn!(
                "{} de {} itens foram rejeitados por campos inválidos",
                item_result.len() - itens_validados.len(),
                item_result.len()
            );
        }

        Ok(ResultadoLlm {
            itens: itens_validados,
            texto_ocr: texto_ocr.to_string(),
        })
    }

    /// Monta o prompt fixo do LLM definido em SPEC.md
    fn monta_prompt(&self, texto_ocr: &str) -> String {
        format!(
            "Você é um extrator de dados de cupons fiscais brasileiros.\n\
             Receba o texto OCR abaixo e retorne APENAS um JSON válido no formato:\n\
             {{\"itens\":[{{\"descricao\":\"...\",\"quantidade\":1,\"preco_unitario\":0.0,\"preco_total\":0.0}}]}}\n\
             Regras:\n\
             - Ignore cabeçalhos, totais, impostos, formas de pagamento e rodapés.\n\
             - Valores em reais, use ponto como separador decimal.\n\
             - Se quantidade não estiver explícita, use 1.\n\
             - Se preco_unitario não estiver explícito, calcule como preco_total/quantidade.\n\
             - Não inclua texto fora do JSON.\n\n\
             Texto OCR:\n<<<{}>>>",
            self.options.modelo, texto_ocr
        )
    }

    /// Envia o prompt para o Ollama e retorna a resposta raw.
    async fn envia_para_ollama(&self, prompt: &str) -> Result<String, LlmError> {
        let payload = serde_json::json!({
            "model": self.options.modelo,
            "prompt": prompt,
            "stream": false,
        });

        let resposta = self
            .client
            .post(&self.endpoint)
            .json(&payload)
            .timeout(Duration::from_secs(self.options.timeout))
            .send()
            .await
            .map_err(|e| LlmError::Conexao(e))?;

        if !resposta.status().is_success() {
            let body_text = resposta.text().await.un_or_else(|_| "Sem corpo de erro".to_string());
            return Err(LlmError::RespostaInvalida(format!(
                "HTTP {}: {}",
                resposta.status(),
                body_text
            )));
        }

        let body = resposta.json::<serde_json::Value>().await.map_err(|e| {
            LlmError::RespostaInvalida(format!("Falha ao fazer parse da resposta: {}", e))
        })?;

        let resposta_texto = body
            .get("response")
            .and_then(|v| v.as_str())
            .ok_or_else(|| LlmError::RespostaInvalida("Campo 'response' ausente na resposta do Ollama".to_string()))?;

        Ok(resposta_texto.to_string())
    }

    /// Faz parsing robusto do JSON retornado pelo LLM.
    /// 
    /// Tolerante a:
    /// - Cercas de código ```json
    /// - JSON mal formatado
    /// - Campos extras
    fn parse_resposta_llm(&self, json_str: &str) -> Result<Vec<core::Item>, LlmError> {
        let json_limpo = self.remove_cercas_codigo(json_str);

        // Tentar fazer parse direto
        let resultado: RespostaLlm = match serde_json::from_str(json_limpo) {
            Ok(r) => r,
            Err(_) => {
                // Tentar extrair JSON de entre chaves {...}
                if let Some(start) = json_limpo.find('{') {
                    if let Some(end) = json_limpo[start..].rfind('}') {
                        let json_subset = &json_limpo[start..start + end + 1];
                        return match serde_json::from_str::<RespostaLlm>(json_subset) {
                            Ok(r) => r,
                            Err(_) => return Err(LlmError::RespostaInvalida("Não foi possível fazer parse do JSON".to_string())),
                        };
                    }
                }
                return Err(LlmError::RespostaInvalida("JSON não encontrado na resposta".to_string()));
            }
        };

        // Validar que itens têm campos > 0
        let mut itens_validados = Vec::new();
        for item in resultado.itens {
            if item.descricao.is_empty() {
                return Err(LlmError::ItemInvalido(format!("Item com descrição vazia")));
            }
            if item.quantidade <= 0.0 {
                warn!("Item com quantidade <= 0: {}", item.descricao);
                continue;
            }
            if item.preco_unitario < 0.0 {
                warn!("Item com preço unitário negativo: {}", item.descricao);
                continue;
            }
            if item.preco_total < 0.0 {
                warn!("Item com preço total negativo: {}", item.descricao);
                continue;
            }

            itens_validados.push(core::Item {
                descricao: item.descricao,
                quantidade: item.quantidade.max(1.0), // Mínimo de 1
                preco_unitario: item.preco_unitario.max(0.0),
                preco_total: item.preco_total.max(0.0),
            });
        }

        Ok(itens_validados)
    }

    /// Remove cercas de código ```json da resposta, se presentes.
    fn remove_cercas_codigo(&self, json_str: &str) -> String {
        let mut resultado = json_str.to_string();

        // Remover ```json no início
        if resultado.starts_with("```json") {
            resultado = resultado[7..].to_string();
        }
        // Remover ``` no final
        if resultado.ends_with("```") {
            resultado = resultado[..resultado.len() - 3].to_string();
        }

        // Remover espaços em branco extras
        resultado.trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{Mock, Method, Path as MockPath, Request, Response};
    use wiremock::MockServer;

    #[test]
    fn test_remove_cercas_codigo() {
        let cliente = ClienteLlm::new(LlmOptions::default()).unwrap();

        // Testar remoção de ```json
        let input = r#"```json
        {"itens":[{"descricao":"Leite","quantidade":1,"preco_unitario":5.5,"preco_total":5.5}]}```
        "#;
        let limpo = cliente.remove_cercas_codigo(input);
        assert!(!limpo.starts_with("```"));
        assert!(!limpo.ends_with("```"));
        assert!(limpo.contains("Leite"));
    }

    #[test]
    fn test_parse_resposta_llm_com_json_valido() {
        let cliente = ClienteLlm::new(LlmOptions::default()).unwrap();

        let json = r#"{"itens":[{"descricao":"Leite","quantidade":2,"preco_unitario":5.5,"preco_total":11.0}]}"#;
        let itens = cliente.parse_resposta_llm(json).unwrap();
        
        assert_eq!(itens.len(), 1);
        assert_eq!(itens[0].descricao, "Leite");
        assert_eq!(itens[0].quantidade, 2.0);
        assert_eq!(itens[0].preco_unitario, 5.5);
        assert_eq!(itens[0].preco_total, 11.0);
    }

    #[test]
    fn test_parse_resposta_llm_com_cercas() {
        let cliente = ClienteLlm::new(LlmOptions::default()).unwrap();

        let json = r#"```json
        {"itens":[{"descricao":"Pão","quantidade":1,"preco_unitario":8.5,"preco_total":8.5}]}```
        "#;
        let itens = cliente.parse_resposta_llm(json).unwrap();
        
        assert_eq!(itens.len(), 1);
        assert_eq!(itens[0].descricao, "Pão");
    }

    #[test]
    fn test_extrair_itens_mock() {
        let options = LlmOptions::default();
        let cliente = ClienteLlm::new(options).unwrap();

        // Simular resposta do LLM
        let resposta_ocr = r#"Leite 1L - R$ 5,50
                          Pão Francês - R$ 8,50"#;

        let resultado = cliente
            .extrair_itens(resposta_ocr)
            .expect("Falha ao extrair itens");

        assert!(!resultado.itens.is_empty());
        assert!(
            resultado.itens.iter().any(|i| i.descricao == "Leite"),
            "Deve haver pelo menos um item 'Leite'"
        );
    }
}