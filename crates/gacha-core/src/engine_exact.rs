use crate::compile::CompiledModel;
use num_bigint::BigInt;
use num_integer::Integer;
use num_rational::BigRational;
use num_traits::{One, ToPrimitive, Zero};
use serde::Serialize;
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct ExactOptions {
    pub max_trials: u32,
    pub max_states: usize,
    pub max_memory_bytes: u64,
    pub reduce_layers: bool,
}

impl Default for ExactOptions {
    fn default() -> Self {
        Self { max_trials: 2_000, max_states: 200_000, max_memory_bytes: 2 * 1024 * 1024 * 1024, reduce_layers: false }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct State {
    control: Vec<u32>,
    counts: Vec<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExactCell {
    pub counts: Vec<u32>,
    pub numerator: String,
    pub denominator: String,
    pub probability: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExactResult {
    pub trials: u32,
    pub tracked_leaf_ids: Vec<String>,
    pub joint: Vec<ExactCell>,
    pub denominator: String,
    pub elapsed_ms: u64,
    pub peak_states: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum ExactError {
    #[error("exact mode unavailable for this model")]
    Unavailable,
    #[error("maxTrials {actual} exceeds exact limit {limit}")]
    TrialLimit { actual: u32, limit: u32 },
    #[error("layer state count {actual} exceeds exact limit {limit}")]
    StateLimit { actual: usize, limit: usize },
    #[error("estimated layer memory {actual} exceeds exact limit {limit}")]
    MemoryLimit { actual: u64, limit: u64 },
    #[error("execution cancelled")]
    Cancelled,
}

pub fn run_exact(
    model: &CompiledModel,
    options: ExactOptions,
    mut progress: impl FnMut(u32, u32) -> bool,
) -> Result<ExactResult, ExactError> {
    if !model.analysis.exact_available || model.exact_lcm.bits() > 128 {
        return Err(ExactError::Unavailable);
    }
    if model.max_trials > options.max_trials {
        return Err(ExactError::TrialLimit { actual: model.max_trials, limit: options.max_trials });
    }
    let started = Instant::now();
    let lcm = &model.exact_lcm;
    let weights: Vec<Vec<Vec<BigInt>>> = model.prob_table.entries.iter().map(|by_trial| {
        by_trial.iter().map(|p| p.exact.iter().map(|r| r.numer() * (lcm / r.denom())).collect()).collect()
    }).collect();
    let initial = State { control: model.control_init.clone(), counts: vec![0; model.state_leaves.len()] };
    let mut cells = HashMap::from([(initial, BigInt::one())]);
    let mut denominator = BigInt::one();
    let mut peak_states = 1usize;
    for trial in 1..=model.max_trials {
        let mut next: HashMap<State, BigInt> = HashMap::new();
        for (state, numerator) in cells {
            let ci = model.control_index(&state.control);
            let ti = if model.prob_table.trial_dependent { trial as usize - 1 } else { 0 };
            for (leaf, weight) in weights[ci][ti].iter().enumerate() {
                if weight.is_zero() { continue; }
                let mut successor = state.clone();
                if let Some(position) = model.state_leaves.iter().position(|tracked| *tracked == leaf) {
                    successor.counts[position] += 1;
                }
                model.apply_transitions(&mut successor.control, leaf, trial);
                model.apply_triggers_sparse(&mut successor.control, &mut successor.counts, trial);
                *next.entry(successor).or_default() += &numerator * weight;
            }
        }
        denominator *= lcm;
        if options.reduce_layers {
            let gcd = next.values().fold(denominator.clone(), |g, n| g.gcd(n));
            if gcd > BigInt::one() {
                denominator /= &gcd;
                for value in next.values_mut() { *value /= &gcd; }
            }
        }
        if next.len() > options.max_states {
            return Err(ExactError::StateLimit { actual: next.len(), limit: options.max_states });
        }
        let estimated = next.len() as u64 * ((denominator.bits() + 7) / 8 + 64);
        if estimated > options.max_memory_bytes {
            return Err(ExactError::MemoryLimit { actual: estimated, limit: options.max_memory_bytes });
        }
        peak_states = peak_states.max(next.len());
        let sum: BigInt = next.values().sum();
        assert_eq!(sum, denominator, "exact DP mass conservation failure");
        cells = next;
        if !progress(trial, model.max_trials) { return Err(ExactError::Cancelled); }
    }
    let mut marginalized: HashMap<Vec<u32>, BigInt> = HashMap::new();
    for (state, numerator) in cells {
        let key = model.tracked_leaves.iter().map(|leaf| {
            model.state_leaves.iter().position(|state_leaf| state_leaf == leaf)
                .map(|position| state.counts[position]).unwrap_or(0)
        }).collect();
        *marginalized.entry(key).or_default() += numerator;
    }
    let denominator_string = denominator.to_string();
    let mut joint: Vec<_> = marginalized.into_iter().map(|(counts, numerator)| ExactCell {
        counts,
        probability: BigRational::new(numerator.clone(), denominator.clone()).to_f64().unwrap_or(0.0),
        numerator: numerator.to_string(),
        denominator: denominator_string.clone(),
    }).collect();
    joint.sort_by(|a, b| a.counts.cmp(&b.counts));
    Ok(ExactResult {
        trials: model.max_trials,
        tracked_leaf_ids: model.tracked_leaves.iter().map(|i| model.leaves[*i].id.clone()).collect(),
        joint,
        denominator: denominator_string,
        elapsed_ms: started.elapsed().as_millis() as u64,
        peak_states,
    })
}
