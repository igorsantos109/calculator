/// Modelos de dados e tipos compartilhados pela aplicação.

use serde::{Deserialize, Serialize};
use crate::error::error::ValidacaoTotal;

/// Enumeração dos grupos possíveis para classificação de itens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
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
#[derive(Debug, Clone)]
pub struct ItemClassificado {
    pub item: Item,
    pub grupo: Grupo,
}

/// Totais por grupo após classificação.
#[derive(Debug, Default, Serialize)]
pub struct Totais {
    /// Soma dos preços totais dos itens classificados como 'casa'
    pub casa: f64,
    /// Soma dos preços totais dos itens classificados como 'igor'
    pub igor: f64,
    /// Soma dos preços totais dos itens classificados como 'renan'
    pub renan: f64,
    /// Soma geral = casa + igor + renan
    pub geral: f64,
}

/// Calcula totais a partir de uma fatia de itens classificados.
///
/// # Returns
/// `Totais` com `casa`, `igor`, `renan` somados respectivamente,
/// e `geral = casa + igor + renan`.
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
/// 
/// # Returns
/// `Ok(())` se a validação passar, `Err(ValidacaoTotal)` se falhar.
pub fn validar_total(itens: &[Item], total_cupom: f64) -> Result<(), ValidacaoTotal> {
    let soma: f64 = itens.iter().map(|item| item.preco_total).sum();
    let diferencial = (soma - total_cupom).abs();

    if diferencial <= 0.05 {
        Ok(())
    } else {
        Err(ValidacaoTotal(diferencial))
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
    fn test_calcular_totais_com_itens() {
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
        ];

        let totals = calcular_totais(&itens);
        assert_eq!(totals.casa, 20.0);
        assert_eq!(totals.igor, 15.0);
        assert_eq!(totals.renan, 0.0);
        assert_eq!(totals.geral, 35.0);
    }

    #[test]
    fn test_validar_total_ok_exato() {
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
    fn test_validar_total_fora_tolerancia() {
        let itens = vec![
            Item {
                descricao: "Produto A".to_string(),
                quantidade: 2.0,
                preco_unitario: 10.0,
                preco_total: 20.0,
            },
        ];

        // Soma = 20.0, total_cupom = 20.2 → diferencial = 0.2 > 0.05
        let resultado = validar_total(&itens, 20.2);
        assert!(resultado.is_err());
    }

    #[test]
    fn test_validar_total_com_tolerancia_limite() {
        let itens = vec![
            Item {
                descricao: "Produto A".to_string(),
                quantidade: 2.0,
                preco_unitario: 10.0,
                preco_total: 20.0,
            },
        ];

        // Soma = 20.0, total_cupom = 20.05 → diferencial = 0.05 ≤ 0.05 (no limite)
        let resultado = validar_total(&itens, 20.05);
        assert!(resultado.is_ok());
    }
}