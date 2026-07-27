use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WilsonInterval {
    pub estimate: f64,
    pub lower: f64,
    pub upper: f64,
    pub confidence: f64,
}

pub fn wilson(successes: u64, trials: u64, z: f64) -> WilsonInterval {
    if trials == 0 {
        return WilsonInterval { estimate: 0.0, lower: 0.0, upper: 1.0, confidence: 0.95 };
    }
    let n = trials as f64;
    let p = successes as f64 / n;
    let z2 = z * z;
    let denominator = 1.0 + z2 / n;
    let center = (p + z2 / (2.0 * n)) / denominator;
    let radius = z * ((p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt()) / denominator;
    WilsonInterval {
        estimate: p,
        lower: (center - radius).max(0.0),
        upper: (center + radius).min(1.0),
        confidence: 0.95,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wilson_contains_estimate() {
        let ci = wilson(34, 1000, 1.959963984540054);
        assert!(ci.lower < ci.estimate && ci.estimate < ci.upper);
    }
}

