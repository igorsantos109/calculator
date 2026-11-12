//! Core types and calculations for the receipt scanner app.
//!
//! This crate defines the domain models, calculation logic, and error types
//! used throughout the application.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Enum representing the classification group for a receipt item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Grupo {
    /// Gasto comum da casa (dividido 50/50 entre Igor e Renan)
    Casa,
    /// Gasto pessoal do Igor
    Igor,
    /// Gasto pessoal do Renan
    Renan,
}

/// Quem pagou o cupom.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Pagador {
    #[default]
    /// Igor pagou
    Igor,
    /// Renan pagou
    Renan,
}

impl std::fmt::Display for Pagador {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Pagador::Igor => write!(f, "igor"),
            Pagador::Renan => write!(f, "renan"),
        }
    }
}

impl std::fmt::Display for Grupo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Grupo::Casa => write!(f, "casa"),
            Grupo::Igor => write!(f, "igor"),
            Grupo::Renan => write!(f, "renan"),
        }
    }
}

/// Error when total validation fails
#[derive(Debug, Clone, Error)]
#[error("|Σ(preco_total) - total_cupom| = {0:.2} > 0.05")]
pub struct ValidacaoTotalError(pub f64);

/// Erros deste crate.
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("Validação de total falhou: |Σ(preco_total) - total_cupom| > 0.05")]
    ValidacaoTotal(#[from] ValidacaoTotalError),

    #[error("Item não tem grupo atribuído")]
    ItemSemGrupo,
}

/// Um item de cupom fiscal com descrição, quantidade e preços.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    /// Descrição do item (ex: "Leite 1L")
    pub descricao: String,
    /// Quantidade (ex: 2 para dois litros)
    pub quantidade: f64,
    /// Preço unitário em reais
    pub preco_unitario: f64,
    /// Preço total em reais (quantidade * preço_unitario)
    pub preco_total: f64,
}

impl Item {
    /// Calcula o preço total a partir da quantidade e preço unitário
    pub fn calcular_preco_total(quantidade: f64, preco_unitario: f64) -> f64 {
        quantidade * preco_unitario
    }
}

/// Um item de cupom já classificado com seu grupo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemClassificado {
    pub item: Item,
    pub grupo: Grupo,
}

/// Totais por grupo após classificação.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Totais {
    pub casa: f64,
    pub igor: f64,
    pub renan: f64,
    pub geral: f64,
}

/// Responsabilidades individuais calculadas a partir dos totais.
/// - Casa é dividida 50/50 entre Igor e Renan.
/// - O centavo indivisível (se houver) fica com quem NÃO pagou.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Responsabilidades {
    /// Responsabilidade do Igor (pessoal + metade da casa + centavo se couber)
    pub igor: f64,
    /// Responsabilidade do Renan (pessoal + metade da casa + centavo se couber)
    pub renan: f64,
    /// Total geral (igor + renan)
    pub geral: f64,
    /// Quem pagou o cupom
    pub pagador: Pagador,
    /// Valor pago pelo pagador
    pub valor_pago: f64,
    /// Saldo: positivo = deve receber, negativo = deve pagar
    pub saldo_igor: f64,
    pub saldo_renan: f64,
}

/// Calcula as responsabilidades individuais.
///
/// # Regras
/// - Itens pessoais (Igor/Renan): responsabilidade integral do respectivo dono.
/// - Itens Casa: divididos 50/50.
/// - Centavo indivisível (diferença de 0,01 ao dividir casa): fica com quem NÃO pagou.
/// - Saldo = valor_pago - responsabilidade (positivo = crédito, negativo = débito).
pub fn calcular_responsabilidades(
    totais: &Totais,
    pagador: Pagador,
    total_cupom: f64,
) -> Responsabilidades {
    // Metade da casa
    let _metade_casa = totais.casa / 2.0;
    let centavos_casa = (totais.casa * 100.0).round() as i64;
    let centavos_metade = centavos_casa / 2;
    let centavo_indivisivel = centavos_casa % 2;

    // Responsabilidade base (sem centavo indivisível)
    let mut igor = totais.igor + (centavos_metade as f64 / 100.0);
    let mut renan = totais.renan + (centavos_metade as f64 / 100.0);

    // Centavo indivisível vai para quem NÃO pagou
    if centavo_indivisivel == 1 {
        match pagador {
            Pagador::Igor => renan += 0.01,
            Pagador::Renan => igor += 0.01,
        }
    }

    let geral = igor + renan;

    // Saldo = valor_pago - responsabilidade
    // Quem pagou tem valor_pago = total_cupom, o outro tem 0
    let (valor_pago_igor, valor_pago_renan) = match pagador {
        Pagador::Igor => (total_cupom, 0.0),
        Pagador::Renan => (0.0, total_cupom),
    };

    let saldo_igor = valor_pago_igor - igor;
    let saldo_renan = valor_pago_renan - renan;

    Responsabilidades {
        igor,
        renan,
        geral,
        pagador,
        valor_pago: total_cupom,
        saldo_igor,
        saldo_renan,
    }
}

/// Soma os itens classificados por grupo e retorna totais gerais.
///
/// # Fórmula
/// - `casa` = soma dos itens com grupo `Casa`
/// - `igor` = soma dos itens com grupo `Igor`
/// - `renan` = soma dos itens com grupo `Renan`
/// - `geral` = `casa` + `igor` + `renan`
pub fn calcular_totais(itens: &[ItemClassificado]) -> Totais {
    let mut totals = Totais::default();

    for item in itens {
        match item.grupo {
            Grupo::Casa => totals.casa += item.item.preco_total,
            Grupo::Igor => totals.igor += item.item.preco_total,
            Grupo::Renan => totals.renan += item.item.preco_total,
        }
    }

    totals.geral = totals.casa + totals.igor + totals.renan;
    totals
}

/// Valida se a soma dos preços totais dos itens está dentro da tolerância do total do cupom.
///
/// Tolerância: `|Σ(preco_total dos itens) − total_cupom| ≤ 0.05`
pub fn validar_total(itens: &[Item], total_cupom: f64) -> Result<(), CoreError> {
    let soma: f64 = itens.iter().map(|item| item.preco_total).sum();
    let diferencial = (soma - total_cupom).abs();

    if diferencial <= 0.05 {
        Ok(())
    } else {
        Err(CoreError::ValidacaoTotal(ValidacaoTotalError(diferencial)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calcular_totais_vazia() {
        let resultado = calcular_totais(&[]);
        assert_eq!(resultado.casa, 0.0);
        assert_eq!(resultado.igor, 0.0);
        assert_eq!(resultado.renan, 0.0);
        assert_eq!(resultado.geral, 0.0);
    }

    #[test]
    fn test_calcular_totais_lista_mista() {
        let itens = vec![
            ItemClassificado {
                item: Item {
                    descricao: "Produto A".to_string(),
                    quantidade: 2.0,
                    preco_unitario: 10.0,
                    preco_total: 20.0,
                },
                grupo: Grupo::Casa,
            },
            ItemClassificado {
                item: Item {
                    descricao: "Produto B".to_string(),
                    quantidade: 1.0,
                    preco_unitario: 15.0,
                    preco_total: 15.0,
                },
                grupo: Grupo::Igor,
            },
            ItemClassificado {
                item: Item {
                    descricao: "Produto C".to_string(),
                    quantidade: 3.0,
                    preco_unitario: 5.0,
                    preco_total: 15.0,
                },
                grupo: Grupo::Renan,
            },
        ];

        let totals = calcular_totais(&itens);
        assert_eq!(totals.casa, 20.0);
        assert_eq!(totals.igor, 15.0);
        assert_eq!(totals.renan, 15.0);
        assert_eq!(totals.geral, 50.0);
    }

    #[test]
    fn test_validar_total_ok() {
        let itens = vec![
            Item {
                descricao: "Produto A".to_string(),
                quantidade: 2.0,
                preco_unitario: 10.0,
                preco_total: 20.0,
            },
            Item {
                descricao: "Produto B".to_string(),
                quantidade: 1.0,
                preco_unitario: 5.0,
                preco_total: 5.0,
            },
        ];

        // Soma = 25.0, total_cupom = 25.0 → diferencial = 0.0 ≤ 0.05
        let resultado = validar_total(&itens, 25.0);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_validar_total_com_tolerancia() {
        let itens = vec![Item {
            descricao: "Produto A".to_string(),
            quantidade: 2.0,
            preco_unitario: 10.0,
            preco_total: 20.0,
        }];

        // Soma = 20.0, total_cupom = 20.1 → diferencial = 0.1 > 0.05
        let resultado = validar_total(&itens, 20.1);
        assert!(resultado.is_err());
    }

    #[test]
    fn test_calcular_responsabilidades_casa_par() {
        // Casa = 10.00 (par), Igor = 5.00, Renan = 3.00, total = 18.00
        // Igor pagou
        let totais = Totais {
            casa: 10.0,
            igor: 5.0,
            renan: 3.0,
            geral: 18.0,
        };
        let resp = calcular_responsabilidades(&totais, Pagador::Igor, 18.0);

        // Casa 10.00 / 2 = 5.00 cada (sem centavo indivisível)
        // Igor: 5.00 (pessoal) + 5.00 (casa) = 10.00
        // Renan: 3.00 (pessoal) + 5.00 (casa) = 8.00
        assert!((resp.igor - 10.0).abs() < 0.001);
        assert!((resp.renan - 8.0).abs() < 0.001);
        assert!((resp.geral - 18.0).abs() < 0.001);
        assert_eq!(resp.pagador, Pagador::Igor);

        // Saldo: Igor pagou 18.00, deve 10.00 -> +8.00 (crédito)
        // Renan pagou 0, deve 8.00 -> -8.00 (débito)
        assert!((resp.saldo_igor - 8.0).abs() < 0.001);
        assert!((resp.saldo_renan + 8.0).abs() < 0.001);
    }

    #[test]
    fn test_calcular_responsabilidades_casa_impar_igor_pagou() {
        // Casa = 10.01 (ímpar), Igor = 5.00, Renan = 3.00, total = 18.01
        // Igor pagou -> centavo vai para Renan
        let totais = Totais {
            casa: 10.01,
            igor: 5.0,
            renan: 3.0,
            geral: 18.01,
        };
        let resp = calcular_responsabilidades(&totais, Pagador::Igor, 18.01);

        // 1001 centavos / 2 = 500 cada, resto 1 -> vai para Renan (não pagou)
        // Igor: 5.00 + 5.00 = 10.00
        // Renan: 3.00 + 5.00 + 0.01 = 8.01
        assert!((resp.igor - 10.0).abs() < 0.001);
        assert!((resp.renan - 8.01).abs() < 0.001);
        assert!((resp.geral - 18.01).abs() < 0.001);

        // Igor pagou 18.01, deve 10.00 -> +8.01
        // Renan pagou 0, deve 8.01 -> -8.01
        assert!((resp.saldo_igor - 8.01).abs() < 0.001);
        assert!((resp.saldo_renan + 8.01).abs() < 0.001);
    }

    #[test]
    fn test_calcular_responsabilidades_casa_impar_renan_pagou() {
        // Casa = 10.01 (ímpar), Igor = 5.00, Renan = 3.00, total = 18.01
        // Renan pagou -> centavo vai para Igor
        let totais = Totais {
            casa: 10.01,
            igor: 5.0,
            renan: 3.0,
            geral: 18.01,
        };
        let resp = calcular_responsabilidades(&totais, Pagador::Renan, 18.01);

        // 1001 centavos / 2 = 500 cada, resto 1 -> vai para Igor (não pagou)
        // Igor: 5.00 + 5.00 + 0.01 = 10.01
        // Renan: 3.00 + 5.00 = 8.00
        assert!((resp.igor - 10.01).abs() < 0.001);
        assert!((resp.renan - 8.0).abs() < 0.001);
        assert!((resp.geral - 18.01).abs() < 0.001);

        // Renan pagou 18.01, deve 8.00 -> +10.01
        // Igor pagou 0, deve 10.01 -> -10.01
        assert!((resp.saldo_renan - 10.01).abs() < 0.001);
        assert!((resp.saldo_igor + 10.01).abs() < 0.001);
    }

    #[test]
    fn test_calcular_responsabilidades_sem_casa() {
        // Sem itens de casa, apenas pessoais
        let totais = Totais {
            casa: 0.0,
            igor: 20.0,
            renan: 10.0,
            geral: 30.0,
        };
        let resp = calcular_responsabilidades(&totais, Pagador::Igor, 30.0);

        assert!((resp.igor - 20.0).abs() < 0.001);
        assert!((resp.renan - 10.0).abs() < 0.001);
        assert!((resp.saldo_igor - 10.0).abs() < 0.001); // Igor pagou 30, deve 20 -> +10
        assert!((resp.saldo_renan + 10.0).abs() < 0.001); // Renan pagou 0, deve 10 -> -10
    }
}

/// Módulo OCR para extração de texto de imagens de cupons fiscais.
pub mod ocr;
