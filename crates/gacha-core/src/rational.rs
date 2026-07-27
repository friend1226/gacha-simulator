use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};
use thiserror::Error;

pub type Rational = BigRational;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LiteralError {
    #[error("literal is empty")]
    Empty,
    #[error("invalid rational literal: {0}")]
    Invalid(String),
    #[error("division by zero")]
    ZeroDenominator,
}

pub fn parse_literal(input: &str) -> Result<Rational, LiteralError> {
    let s = input.trim();
    if s.is_empty() {
        return Err(LiteralError::Empty);
    }
    if let Some((a, b)) = s.split_once('/') {
        if b.contains('/') {
            return Err(LiteralError::Invalid(s.into()));
        }
        let num = parse_integer(a.trim())?;
        let den = parse_integer(b.trim())?;
        if den.is_zero() {
            return Err(LiteralError::ZeroDenominator);
        }
        return Ok(Rational::new(num, den));
    }

    let (mantissa, exponent) = match s.find(|c| c == 'e' || c == 'E') {
        Some(pos) => {
            let exp = s[pos + 1..]
                .parse::<i64>()
                .map_err(|_| LiteralError::Invalid(s.into()))?;
            (&s[..pos], exp)
        }
        None => (s, 0),
    };
    let negative = mantissa.starts_with('-');
    let unsigned = if mantissa.starts_with('-') || mantissa.starts_with('+') {
        &mantissa[1..]
    } else {
        mantissa
    };
    let (whole, frac) = match unsigned.split_once('.') {
        Some((w, f)) => (w, f),
        None => (unsigned, ""),
    };
    if whole.is_empty() && frac.is_empty()
        || !whole.chars().all(|c| c.is_ascii_digit())
        || !frac.chars().all(|c| c.is_ascii_digit())
    {
        return Err(LiteralError::Invalid(s.into()));
    }
    let digits = format!("{}{}", if whole.is_empty() { "0" } else { whole }, frac);
    let mut num = parse_integer(&digits)?;
    if negative {
        num = -num;
    }
    let scale = frac.len() as i64 - exponent;
    if scale >= 0 {
        Ok(Rational::new(num, pow10(scale as u64)))
    } else {
        Ok(Rational::from_integer(num * pow10((-scale) as u64)))
    }
}

fn parse_integer(s: &str) -> Result<BigInt, LiteralError> {
    s.parse::<BigInt>()
        .map_err(|_| LiteralError::Invalid(s.into()))
}

fn pow10(exp: u64) -> BigInt {
    let mut result = BigInt::one();
    let mut base = BigInt::from(10u8);
    let mut n = exp;
    while n > 0 {
        if n & 1 == 1 {
            result *= &base;
        }
        n >>= 1;
        if n > 0 {
            base = &base * &base;
        }
    }
    result
}

pub fn rational_to_decimal(value: &Rational, sig_digits: usize) -> String {
    if value.is_zero() {
        return "0".into();
    }
    let sign = if value.is_negative() { "-" } else { "" };
    let abs = value.abs();
    let digits = sig_digits.max(1);
    let scale = pow10(digits as u64);
    let scaled = (abs.numer() * &scale) / abs.denom();
    let raw = scaled.to_string();
    if raw.len() <= digits {
        format!("{sign}0.{:0>width$}", raw, width = digits)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned()
    } else {
        let split = raw.len() - digits;
        format!("{sign}{}.{}", &raw[..split], &raw[split..])
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_decimal_without_f64() {
        assert_eq!(parse_literal("0.007").unwrap(), Rational::new(7.into(), 1000.into()));
        assert_eq!(parse_literal("1/3").unwrap(), Rational::new(1.into(), 3.into()));
        assert_eq!(parse_literal("3e-5").unwrap(), Rational::new(3.into(), 100000.into()));
        assert_eq!(parse_literal("-1.25e2").unwrap(), Rational::from_integer((-125).into()));
    }
}
