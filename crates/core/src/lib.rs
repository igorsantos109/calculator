//! Core types and calculations for the receipt scanner app.
//!
//! This crate defines the domain models, calculation logic, and error types
//! used throughout the application.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Enum representing the classification group for a receipt item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Grupo {
    /// Gasto comum da casa
    Casa,
    /// Gasto pessoal do Igor
    Igor,
    /// Gasto pessoal do Renan
    Renan,
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
}
