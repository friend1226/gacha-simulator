use crate::rational::{rational_to_decimal, Rational};
use num_bigint::BigInt;
use num_traits::{ToPrimitive, Zero};
use serde::{Deserialize, Serialize};

pub trait Prob: Clone + Send + Sync + 'static {
    fn zero() -> Self;
    fn one() -> Self;
    fn from_ratio(num: &BigInt, den: &BigInt) -> Self;
    fn add_assign(&mut self, other: &Self);
    fn mul(&self, other: &Self) -> Self;
    fn is_zero(&self) -> bool;
    fn to_f64_lossy(&self) -> f64;
    fn to_decimal_string(&self, sig_digits: usize) -> String;
    fn magnitude_log10(&self) -> Option<f64>;

    fn from_rational(value: &Rational) -> Self {
        Self::from_ratio(value.numer(), value.denom())
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub struct F64(pub f64);

impl Prob for F64 {
    fn zero() -> Self { Self(0.0) }
    fn one() -> Self { Self(1.0) }
    fn from_ratio(num: &BigInt, den: &BigInt) -> Self {
        Self(num.to_f64().unwrap_or_else(|| if num.sign() == num_bigint::Sign::Minus { f64::NEG_INFINITY } else { f64::INFINITY })
            / den.to_f64().unwrap_or(f64::INFINITY))
    }
    fn add_assign(&mut self, other: &Self) { self.0 += other.0; }
    fn mul(&self, other: &Self) -> Self { Self(self.0 * other.0) }
    fn is_zero(&self) -> bool { self.0 == 0.0 }
    fn to_f64_lossy(&self) -> f64 { self.0 }
    fn to_decimal_string(&self, sig_digits: usize) -> String {
        format!("{:.*e}", sig_digits.saturating_sub(1), self.0)
    }
    fn magnitude_log10(&self) -> Option<f64> {
        if self.0 == 0.0 { Some(f64::NEG_INFINITY) } else { Some(self.0.abs().log10()) }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub struct ScaledF64 {
    pub mantissa: f64,
    pub exponent: i64,
}

impl ScaledF64 {
    pub fn new(value: f64) -> Self {
        if value == 0.0 { return Self::default(); }
        assert!(value.is_finite() && value >= 0.0, "probability must be finite and non-negative");
        let (mantissa, exponent) = frexp_bits(value);
        Self { mantissa, exponent }
    }

    fn normalize(mut mantissa: f64, mut exponent: i64) -> Self {
        if mantissa == 0.0 { return Self::default(); }
        assert!(mantissa.is_finite() && mantissa >= 0.0, "probability overflow");
        let (m, e) = frexp_bits(mantissa);
        mantissa = m;
        exponent = exponent.checked_add(e).expect("ScaledF64 exponent overflow");
        Self { mantissa, exponent }
    }
}

fn frexp_bits(value: f64) -> (f64, i64) {
    debug_assert!(value.is_finite() && value > 0.0);
    let bits = value.to_bits();
    let raw_exp = ((bits >> 52) & 0x7ff) as i32;
    if raw_exp == 0 {
        let scaled = value * f64::from_bits(((1023 + 54) as u64) << 52);
        let (m, e) = frexp_bits(scaled);
        return (m, e - 54);
    }
    let mantissa_bits = (bits & ((1u64 << 52) - 1)) | ((1023u64) << 52);
    (f64::from_bits(mantissa_bits), raw_exp as i64 - 1023)
}

impl Prob for ScaledF64 {
    fn zero() -> Self { Self::default() }
    fn one() -> Self { Self { mantissa: 1.0, exponent: 0 } }
    fn from_ratio(num: &BigInt, den: &BigInt) -> Self {
        if num.is_zero() { return Self::zero(); }
        let n_bits = num.bits() as i64;
        let d_bits = den.bits() as i64;
        let shift_n = (n_bits - 53).max(0) as usize;
        let shift_d = (d_bits - 53).max(0) as usize;
        let n = (num >> shift_n).to_f64().expect("BigInt conversion");
        let d = (den >> shift_d).to_f64().expect("BigInt conversion");
        Self::normalize(n / d, shift_n as i64 - shift_d as i64)
    }
    fn add_assign(&mut self, other: &Self) {
        if other.is_zero() { return; }
        if self.is_zero() { *self = *other; return; }
        let diff = self.exponent - other.exponent;
        if diff >= 60 { return; }
        if diff <= -60 { *self = *other; return; }
        if diff >= 0 {
            *self = Self::normalize(self.mantissa + other.mantissa * 2f64.powi(-(diff as i32)), self.exponent);
        } else {
            *self = Self::normalize(self.mantissa * 2f64.powi(diff as i32) + other.mantissa, other.exponent);
        }
    }
    fn mul(&self, other: &Self) -> Self {
        if self.is_zero() || other.is_zero() { return Self::zero(); }
        Self::normalize(
            self.mantissa * other.mantissa,
            self.exponent.checked_add(other.exponent).expect("ScaledF64 exponent overflow"),
        )
    }
    fn is_zero(&self) -> bool { self.mantissa == 0.0 }
    fn to_f64_lossy(&self) -> f64 {
        if self.exponent > i32::MAX as i64 { return f64::INFINITY; }
        if self.exponent < i32::MIN as i64 { return 0.0; }
        self.mantissa * 2f64.powi(self.exponent as i32)
    }
    fn to_decimal_string(&self, sig_digits: usize) -> String {
        if self.is_zero() { return "0".into(); }
        let log10 = self.mantissa.log10() + self.exponent as f64 * std::f64::consts::LOG10_2;
        let decimal_exp = log10.floor() as i64;
        let coefficient = 10f64.powf(log10 - decimal_exp as f64);
        format!("{:.*}e{:+}", sig_digits.saturating_sub(1), coefficient, decimal_exp)
    }
    fn magnitude_log10(&self) -> Option<f64> {
        if self.is_zero() { Some(f64::NEG_INFINITY) }
        else { Some(self.mantissa.log10() + self.exponent as f64 * std::f64::consts::LOG10_2) }
    }
}

pub fn exact_decimal(value: &Rational, sig_digits: usize) -> String {
    rational_to_decimal(value, sig_digits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rational::parse_literal;

    #[test]
    fn scaled_extreme_probability_survives() {
        let p = ScaledF64::from_rational(&parse_literal("0.007").unwrap());
        let mut value = ScaledF64::one();
        for _ in 0..200 { value = value.mul(&p); }
        assert!(!value.is_zero());
        assert!(value.magnitude_log10().unwrap() < -430.0);
        assert_eq!(value.to_f64_lossy(), 0.0);
    }

    #[test]
    fn scaled_add_aligns_exponents() {
        let mut a = ScaledF64::new(0.5);
        a.add_assign(&ScaledF64::new(0.25));
        assert!((a.to_f64_lossy() - 0.75).abs() < 1e-15);
    }
}

