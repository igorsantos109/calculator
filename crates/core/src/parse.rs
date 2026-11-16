//! Parsing e validação centralizados de entradas do usuário.
//!
//! Formato brasileiro: vírgula como separador decimal, ponto como separador
//! de milhar (ex.: "1.234,56"). Rejeita NaN, infinito e valores não finitos.
//! Datas aceitas: `AAAA-MM-DD` (ISO) ou `DD/MM/AAAA` (brasileiro).

use thiserror::Error;

/// Erros de parsing de valores monetários/quantidades.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum ParseError {
    #[error("Valor vazio")]
    Empty,
    #[error("Formato inválido: '{0}'")]
    InvalidFormat(String),
    #[error("Valor negativo não permitido: {0}")]
    Negative(f64),
    #[error("Valor não é um número finito (NaN/infinito)")]
    NotFinite,
    #[error("Quantidade deve ser maior que zero")]
    QuantidadeZero,
    #[error("Valor excede o limite suportado")]
    Overflow,
    #[error("Data inválida: '{0}'. Use AAAA-MM-DD ou DD/MM/AAAA")]
    DataInvalida(String),
}

/// Limite superior de valor aceito (1 trilhão) para evitar overflow.
const LIMITE_MAX: f64 = 1_000_000_000_000.0;

/// Faz parsing de um valor monetário em formato brasileiro.
///
/// Aceita: "5,50", "1.234,56", "1234", "0,50", "12.5" (ponto como decimal
/// quando há ≤ 2 dígitos após o último ponto e não há vírgula).
/// Rejeita: vazio, "NaN", "inf", "-inf", negativos, mais de 2 casas decimais.
pub fn parse_valor(input: &str) -> Result<f64, ParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ParseError::Empty);
    }

    // Rejeitar explicitamente NaN/inf antes de qualquer normalização
    let lower = trimmed.to_lowercase();
    if lower == "nan"
        || lower == "inf"
        || lower == "+inf"
        || lower == "-inf"
        || lower == "infinity"
        || lower == "+infinity"
        || lower == "-infinity"
    {
        return Err(ParseError::NotFinite);
    }

    let normalized = if trimmed.contains(',') {
        let parts: Vec<&str> = trimmed.split(',').collect();
        if parts.len() != 2 {
            return Err(ParseError::InvalidFormat(trimmed.to_string()));
        }
        let integer_part = parts[0].replace('.', "");
        let decimal_part = parts[1];
        if decimal_part.len() > 2 {
            return Err(ParseError::InvalidFormat(trimmed.to_string()));
        }
        if integer_part.is_empty() && decimal_part.is_empty() {
            return Err(ParseError::InvalidFormat(trimmed.to_string()));
        }
        let int_str = if integer_part.is_empty() {
            "0"
        } else {
            &integer_part
        };
        format!("{}.{}", int_str, decimal_part)
    } else if trimmed.contains('.') {
        let parts: Vec<&str> = trimmed.split('.').collect();
        if parts.len() == 2 && parts[1].len() <= 2 && !parts[1].is_empty() {
            // "12.5" ou "12.50" — ponto como decimal
            format!("{}.{}", parts[0], parts[1])
        } else {
            // "1.234" ou "1.234.567" — ponto como milhar
            trimmed.replace('.', "")
        }
    } else {
        trimmed.to_string()
    };

    let value: f64 = normalized
        .parse()
        .map_err(|_| ParseError::InvalidFormat(trimmed.to_string()))?;

    if !value.is_finite() {
        return Err(ParseError::NotFinite);
    }
    if value < 0.0 {
        return Err(ParseError::Negative(value));
    }
    if value > LIMITE_MAX {
        return Err(ParseError::Overflow);
    }
    Ok(value)
}

/// Faz parsing de uma quantidade (deve ser > 0, pode ser fracionária: 0,5 kg).
///
/// Aceita "0,5", "2", "2.5", "2,5". Rejeita zero, negativos, NaN, inf.
/// Nunca arredonda nem converte para inteiro silenciosamente.
pub fn parse_quantidade(input: &str) -> Result<f64, ParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ParseError::Empty);
    }
    let lower = trimmed.to_lowercase();
    if lower == "nan"
        || lower == "inf"
        || lower == "+inf"
        || lower == "-inf"
        || lower == "infinity"
        || lower == "+infinity"
        || lower == "-infinity"
    {
        return Err(ParseError::NotFinite);
    }
    let normalized = trimmed.replace(',', ".");
    let value: f64 = normalized
        .parse()
        .map_err(|_| ParseError::InvalidFormat(trimmed.to_string()))?;
    if !value.is_finite() {
        return Err(ParseError::NotFinite);
    }
    if value <= 0.0 {
        if value == 0.0 {
            return Err(ParseError::QuantidadeZero);
        }
        return Err(ParseError::Negative(value));
    }
    if value > LIMITE_MAX {
        return Err(ParseError::Overflow);
    }
    Ok(value)
}

/// Valida uma data nos formatos `AAAA-MM-DD` ou `DD/MM/AAAA`.
///
/// Retorna a data normalizada em `AAAA-MM-DD`.
pub fn validar_data(input: &str) -> Result<String, ParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ParseError::DataInvalida(input.to_string()));
    }

    // Formato ISO: AAAA-MM-DD
    if trimmed.len() == 10
        && trimmed.chars().nth(4) == Some('-')
        && trimmed.chars().nth(7) == Some('-')
    {
        let ano: i64 = trimmed[0..4]
            .parse()
            .map_err(|_| ParseError::DataInvalida(trimmed.to_string()))?;
        let mes: u32 = trimmed[5..7]
            .parse()
            .map_err(|_| ParseError::DataInvalida(trimmed.to_string()))?;
        let dia: u32 = trimmed[8..10]
            .parse()
            .map_err(|_| ParseError::DataInvalida(trimmed.to_string()))?;
        if data_valida(ano, mes, dia) {
            return Ok(trimmed.to_string());
        }
        return Err(ParseError::DataInvalida(trimmed.to_string()));
    }

    // Formato brasileiro: DD/MM/AAAA
    if trimmed.len() == 10
        && trimmed.chars().nth(2) == Some('/')
        && trimmed.chars().nth(5) == Some('/')
    {
        let dia: u32 = trimmed[0..2]
            .parse()
            .map_err(|_| ParseError::DataInvalida(trimmed.to_string()))?;
        let mes: u32 = trimmed[3..5]
            .parse()
            .map_err(|_| ParseError::DataInvalida(trimmed.to_string()))?;
        let ano: i64 = trimmed[6..10]
            .parse()
            .map_err(|_| ParseError::DataInvalida(trimmed.to_string()))?;
        if data_valida(ano, mes, dia) {
            return Ok(format!("{:04}-{:02}-{:02}", ano, mes, dia));
        }
        return Err(ParseError::DataInvalida(trimmed.to_string()));
    }

    Err(ParseError::DataInvalida(trimmed.to_string()))
}

/// Verifica se ano/mês/dia formam uma data gregoriana válida.
fn data_valida(ano: i64, mes: u32, dia: u32) -> bool {
    if !(1..=9999).contains(&ano) || !(1..=12).contains(&mes) || dia < 1 {
        return false;
    }
    let dias_no_mes = match mes {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (ano % 4 == 0 && ano % 100 != 0) || ano % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => return false,
    };
    dia <= dias_no_mes
}

/// Converte um valor em reais (f64) para centavos inteiros (i64), com arredondamento.
pub fn reais_para_centavos(valor: f64) -> i64 {
    (valor * 100.0).round() as i64
}

/// Converte centavos inteiros para reais (f64).
pub fn centavos_para_reais(centavos: i64) -> f64 {
    centavos as f64 / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- parse_valor ----
    #[test]
    fn test_parse_valor_formatos_brasileiros() {
        assert_eq!(parse_valor("5,50").unwrap(), 5.5);
        assert_eq!(parse_valor("1.234,56").unwrap(), 1234.56);
        assert_eq!(parse_valor("0,50").unwrap(), 0.5);
        assert_eq!(parse_valor(",50").unwrap(), 0.5);
        assert_eq!(parse_valor("1234").unwrap(), 1234.0);
        assert_eq!(parse_valor("12.5").unwrap(), 12.5);
        assert_eq!(parse_valor("0").unwrap(), 0.0);
        assert_eq!(parse_valor("1.234").unwrap(), 1234.0);
    }

    #[test]
    fn test_parse_valor_rejeita_nan_inf() {
        assert!(matches!(parse_valor("NaN"), Err(ParseError::NotFinite)));
        assert!(matches!(parse_valor("nan"), Err(ParseError::NotFinite)));
        assert!(matches!(parse_valor("inf"), Err(ParseError::NotFinite)));
        assert!(matches!(parse_valor("-inf"), Err(ParseError::NotFinite)));
        assert!(matches!(
            parse_valor("infinity"),
            Err(ParseError::NotFinite)
        ));
        assert!(matches!(
            parse_valor("Infinity"),
            Err(ParseError::NotFinite)
        ));
    }

    #[test]
    fn test_parse_valor_rejeita_invalidos() {
        assert!(matches!(parse_valor(""), Err(ParseError::Empty)));
        assert!(matches!(parse_valor("   "), Err(ParseError::Empty)));
        assert!(matches!(
            parse_valor("abc"),
            Err(ParseError::InvalidFormat(_))
        ));
        assert!(matches!(parse_valor("-1"), Err(ParseError::Negative(_))));
        assert!(matches!(parse_valor("-5,50"), Err(ParseError::Negative(_))));
        assert!(matches!(
            parse_valor("1,2,3"),
            Err(ParseError::InvalidFormat(_))
        ));
        assert!(matches!(
            parse_valor("1.234,567"),
            Err(ParseError::InvalidFormat(_))
        ));
    }

    // ---- parse_quantidade ----
    #[test]
    fn test_parse_quantidade_fracionaria() {
        assert_eq!(parse_quantidade("0,5").unwrap(), 0.5);
        assert_eq!(parse_quantidade("0.5").unwrap(), 0.5);
        assert_eq!(parse_quantidade("2,5").unwrap(), 2.5);
        assert_eq!(parse_quantidade("1").unwrap(), 1.0);
        assert_eq!(parse_quantidade("2,125").unwrap(), 2.125);
    }

    #[test]
    fn test_parse_quantidade_rejeita_zero_negativo_nan() {
        assert!(matches!(
            parse_quantidade("0"),
            Err(ParseError::QuantidadeZero)
        ));
        assert!(matches!(
            parse_quantidade("0,0"),
            Err(ParseError::QuantidadeZero)
        ));
        assert!(matches!(
            parse_quantidade("-1"),
            Err(ParseError::Negative(_))
        ));
        assert!(matches!(
            parse_quantidade("NaN"),
            Err(ParseError::NotFinite)
        ));
        assert!(matches!(
            parse_quantidade("inf"),
            Err(ParseError::NotFinite)
        ));
        assert!(matches!(parse_quantidade(""), Err(ParseError::Empty)));
    }

    // ---- validar_data ----
    #[test]
    fn test_validar_data_iso() {
        assert_eq!(validar_data("2026-11-15").unwrap(), "2026-11-15");
        assert_eq!(validar_data("2024-02-29").unwrap(), "2024-02-29"); // bissexto
    }

    #[test]
    fn test_validar_data_brasileira() {
        assert_eq!(validar_data("15/11/2026").unwrap(), "2026-11-15");
        assert_eq!(validar_data("29/02/2024").unwrap(), "2024-02-29");
    }

    #[test]
    fn test_validar_data_invalida() {
        assert!(matches!(
            validar_data("2026-13-01"),
            Err(ParseError::DataInvalida(_))
        ));
        assert!(matches!(
            validar_data("2026-02-30"),
            Err(ParseError::DataInvalida(_))
        ));
        assert!(matches!(
            validar_data("2025-02-29"),
            Err(ParseError::DataInvalida(_))
        )); // não bissexto
        assert_eq!(validar_data("31/12/2026").unwrap(), "2026-12-31"); // válido
        assert!(matches!(
            validar_data("32/01/2026"),
            Err(ParseError::DataInvalida(_))
        ));
        assert!(matches!(
            validar_data("15-11-2026"),
            Err(ParseError::DataInvalida(_))
        ));
        assert!(matches!(
            validar_data("2026/11/15"),
            Err(ParseError::DataInvalida(_))
        ));
        assert!(matches!(validar_data(""), Err(ParseError::DataInvalida(_))));
        assert!(matches!(
            validar_data("abc"),
            Err(ParseError::DataInvalida(_))
        ));
    }

    // ---- conversão centavos ----
    #[test]
    fn test_conversao_centavos() {
        assert_eq!(reais_para_centavos(20.05), 2005);
        assert_eq!(reais_para_centavos(0.06), 6);
        assert_eq!(centavos_para_reais(2005), 20.05);
        assert_eq!(reais_para_centavos(1234.56), 123456);
    }
}
