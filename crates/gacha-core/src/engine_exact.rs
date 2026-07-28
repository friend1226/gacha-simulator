use crate::compile::CompiledModel;
use crate::state::{StateCodec, StateCodecError};
use num_bigint::BigInt;
use num_integer::Integer;
use num_rational::BigRational;
use num_traits::{One, ToPrimitive, Zero};
use rustc_hash::FxHashMap;
use serde::Serialize;
use std::collections::BTreeMap;
use std::time::Instant;
#[cfg(not(target_arch = "wasm32"))]
use crate::snapshot::SnapshotSession;
#[cfg(target_arch = "wasm32")]
struct SnapshotSession;

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
pub struct ExactProbability {
    pub numerator: String,
    pub denominator: String,
    pub probability: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExactFirstHitResult {
    pub pmf: Vec<ExactProbability>,
    pub cdf: Vec<ExactProbability>,
    pub failure_reachable: ExactProbability,
    pub mean: Option<f64>,
    pub percentiles: Vec<(f64, u32)>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExactResult {
    pub numeric: String,
    pub trials: u32,
    pub tracked_leaf_ids: Vec<String>,
    pub joint: Vec<ExactCell>,
    pub first_hit: Option<ExactFirstHitResult>,
    pub denominator: String,
    pub elapsed_ms: u64,
    pub peak_states: usize,
    pub clamp_events: u64,
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
    #[error("state encoding failed: {0}")]
    StateEncoding(#[from] StateCodecError),
}

pub fn run_exact(
    model: &CompiledModel,
    options: ExactOptions,
    progress: impl FnMut(u32, u32) -> bool,
) -> Result<ExactResult, ExactError> {
    run_exact_internal(model, options, progress, None)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn run_exact_with_snapshot(
    model: &CompiledModel,
    options: ExactOptions,
    progress: impl FnMut(u32, u32) -> bool,
    snapshot: &mut SnapshotSession,
) -> Result<ExactResult, ExactError> {
    run_exact_internal(model, options, progress, Some(snapshot))
}

fn run_exact_internal(
    model: &CompiledModel,
    options: ExactOptions,
    mut progress: impl FnMut(u32, u32) -> bool,
    mut snapshot: Option<&mut SnapshotSession>,
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
    let codec = StateCodec::new(&model.control_max, &model.state_count_max)?;
    let initial = codec.encode(&model.control_init, &vec![0; model.state_leaves.len()])?;
    let mut cells = FxHashMap::from_iter([(initial, BigInt::one())]);
    let mut denominator = BigInt::one();
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(session) = snapshot.as_deref_mut() {
        session.on_exact_layer(model, &codec, 0, &cells, &denominator);
    }
    let mut peak_states = 1usize;
    let mut hit_pmf = model.condition.as_ref()
        .map(|_| vec![BigInt::zero(); model.max_trials as usize + 1]);
    let mut trial = 0u32;
    while trial < model.max_trials {
        let draw_trial = trial + 1;
        let consumed_trials = model.consumed_trials_after(draw_trial);
        let next_trial = draw_trial + consumed_trials;
        if let Some(pmf) = &mut hit_pmf {
            for numerator in pmf { *numerator *= lcm; }
        }
        let source: Vec<_> = cells.into_iter().collect();
        let expanded = expand_exact_layer(model, &codec, &weights, &source, draw_trial);
        let mut next = expanded.cells;
        if let Some(pmf) = &mut hit_pmf {
            for (hit_trial, contribution) in expanded.hits {
                pmf[hit_trial] += contribution;
            }
        }
        denominator *= lcm;
        if options.reduce_layers {
            let gcd = next.values()
                .chain(hit_pmf.iter().flat_map(|pmf| pmf.iter()))
                .fold(denominator.clone(), |g, n| g.gcd(n));
            if gcd > BigInt::one() {
                denominator /= &gcd;
                for value in next.values_mut() { *value /= &gcd; }
                if let Some(pmf) = &mut hit_pmf {
                    for value in pmf { *value /= &gcd; }
                }
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
        let mut sum: BigInt = next.values().sum();
        if let Some(pmf) = &hit_pmf {
            sum += pmf.iter().sum::<BigInt>();
        }
        assert_eq!(sum, denominator, "exact DP mass conservation failure");
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(session) = snapshot.as_deref_mut() {
            session.on_exact_layer(model, &codec, next_trial, &next, &denominator);
        }
        cells = next;
        trial = next_trial;
        if !progress(trial, model.max_trials) { return Err(ExactError::Cancelled); }
    }
    let mut marginalized: BTreeMap<Vec<u32>, BigInt> = BTreeMap::new();
    for (state, numerator) in cells {
        let (_, state_counts) = codec.decode(state);
        let key = model.tracked_leaves.iter().map(|leaf| {
            model.state_leaves.iter().position(|state_leaf| state_leaf == leaf)
                .map(|position| state_counts[position]).unwrap_or(0)
        }).collect();
        *marginalized.entry(key).or_default() += numerator;
    }
    let denominator_string = denominator.to_string();
    let joint = marginalized.into_iter().map(|(counts, numerator)| ExactCell {
        counts,
        probability: BigRational::new(numerator.clone(), denominator.clone()).to_f64().unwrap_or(0.0),
        numerator: numerator.to_string(),
        denominator: denominator_string.clone(),
    }).collect();
    let first_hit = hit_pmf.map(|pmf| summarize_exact_first_hit(&pmf, &denominator));
    Ok(ExactResult {
        numeric: "exact".into(),
        trials: trial,
        tracked_leaf_ids: model.tracked_leaves.iter().map(|i| model.leaves[*i].id.clone()).collect(),
        joint,
        first_hit,
        denominator: denominator_string,
        elapsed_ms: started.elapsed().as_millis() as u64,
        peak_states,
        clamp_events: model.prob_table.clamp_events,
    })
}

struct ExactLayerExpansion {
    cells: FxHashMap<u64, BigInt>,
    hits: Vec<(usize, BigInt)>,
}

#[cfg(feature = "parallel")]
fn expand_exact_layer(
    model: &CompiledModel,
    codec: &StateCodec,
    weights: &[Vec<Vec<BigInt>>],
    source: &[(u64, BigInt)],
    draw_trial: u32,
) -> ExactLayerExpansion {
    if source.len() <= 1_024 {
        return expand_exact_chunk(model, codec, weights, source, draw_trial);
    }
    let middle = source.len() / 2;
    let (left, right) = source.split_at(middle);
    let (left, right) = rayon::join(
        || expand_exact_layer(model, codec, weights, left, draw_trial),
        || expand_exact_layer(model, codec, weights, right, draw_trial),
    );
    merge_exact_expansions(left, right)
}

#[cfg(not(feature = "parallel"))]
fn expand_exact_layer(
    model: &CompiledModel,
    codec: &StateCodec,
    weights: &[Vec<Vec<BigInt>>],
    source: &[(u64, BigInt)],
    draw_trial: u32,
) -> ExactLayerExpansion {
    expand_exact_chunk(model, codec, weights, source, draw_trial)
}

fn merge_exact_expansions(
    mut left: ExactLayerExpansion,
    right: ExactLayerExpansion,
) -> ExactLayerExpansion {
    left.cells.reserve(right.cells.len());
    for (state, contribution) in right.cells {
        *left.cells.entry(state).or_default() += contribution;
    }
    left.hits.extend(right.hits);
    left
}

fn expand_exact_chunk(
    model: &CompiledModel,
    codec: &StateCodec,
    weights: &[Vec<Vec<BigInt>>],
    source: &[(u64, BigInt)],
    draw_trial: u32,
) -> ExactLayerExpansion {
    if model.packed_transition_fast_path() {
        return expand_exact_packed_chunk(model, codec, weights, source, draw_trial);
    }
    let consumed_trials = model.consumed_trials_after(draw_trial);
    let ti = if model.prob_table.trial_dependent { draw_trial as usize - 1 } else { 0 };
    let mut cells = FxHashMap::default();
    cells.reserve(source.len().saturating_mul(2));
    let mut hits = Vec::new();
    let mut base_control = vec![0; codec.control_len()];
    let mut base_counts = vec![0; codec.count_len()];
    let mut successor_control = vec![0; codec.control_len()];
    let mut successor_counts = vec![0; codec.count_len()];
    let mut transition_before = vec![0; codec.control_len()];
    for (state, numerator) in source {
            let ci = codec.control_index(*state);
            codec.decode_into(*state, &mut base_control, &mut base_counts);
            for (leaf, weight) in weights[ci][ti].iter().enumerate() {
                if weight.is_zero() { continue; }
                let contribution = numerator * weight;
                successor_control.copy_from_slice(&base_control);
                successor_counts.copy_from_slice(&base_counts);
                if let Some(position) = model.state_leaf_position(leaf) {
                    successor_counts[position] += 1;
                }
                model.apply_transitions_buffered(
                    &mut successor_control,
                    &mut transition_before,
                    leaf,
                    draw_trial,
                );
                if model.condition_matches_sparse(&successor_counts, draw_trial) {
                    hits.push((draw_trial as usize, contribution));
                    continue;
                }
                let mut grant_hit = None;
                let applied_consumed = model.apply_triggers_sparse(
                    &mut successor_control,
                    &mut successor_counts,
                    draw_trial,
                    |grant_counts, grant_trial| {
                        if grant_hit.is_none()
                            && model.condition_matches_sparse(grant_counts, grant_trial)
                        {
                            grant_hit = Some(grant_trial);
                        }
                    },
                );
                debug_assert_eq!(applied_consumed, consumed_trials);
                if let Some(hit_trial) = grant_hit {
                    hits.push((hit_trial as usize, contribution));
                    continue;
                }
                let successor = codec.encode(&successor_control, &successor_counts)
                    .expect("compiled successor state must fit its codec");
                *cells.entry(successor).or_default() += contribution;
            }
        }
    ExactLayerExpansion { cells, hits }
}

fn expand_exact_packed_chunk(
    model: &CompiledModel,
    codec: &StateCodec,
    weights: &[Vec<Vec<BigInt>>],
    source: &[(u64, BigInt)],
    draw_trial: u32,
) -> ExactLayerExpansion {
    let probability_trial = if model.prob_table.trial_dependent {
        draw_trial as usize - 1
    } else {
        0
    };
    let mut cells = FxHashMap::default();
    cells.reserve(source.len().saturating_mul(2));
    for (state, numerator) in source {
        let control_index = codec.control_index(*state);
        for (leaf, weight) in weights[control_index][probability_trial].iter().enumerate() {
            if weight.is_zero() { continue; }
            let next_control = if model.prob_table.control_invariant {
                0
            } else {
                model.transition_control_index(control_index, leaf, draw_trial)
            };
            let mut successor = codec.replace_control_index(*state, next_control);
            if let Some(position) = model.state_leaf_position(leaf) {
                successor = codec.increment_count(successor, position);
            }
            *cells.entry(successor).or_default() += numerator * weight;
        }
    }
    ExactLayerExpansion { cells, hits: Vec::new() }
}

fn exact_probability(numerator: BigInt, denominator: &BigInt) -> ExactProbability {
    ExactProbability {
        probability: BigRational::new(numerator.clone(), denominator.clone())
            .to_f64()
            .unwrap_or(0.0),
        numerator: numerator.to_string(),
        denominator: denominator.to_string(),
    }
}

fn summarize_exact_first_hit(
    pmf: &[BigInt],
    denominator: &BigInt,
) -> ExactFirstHitResult {
    let mut running = BigInt::zero();
    let mut cumulative = Vec::with_capacity(pmf.len());
    for numerator in pmf {
        running += numerator;
        cumulative.push(running.clone());
    }
    let success = running;
    let failure = denominator - &success;
    let weighted: BigInt = pmf.iter().enumerate()
        .map(|(trial, numerator)| numerator * BigInt::from(trial))
        .sum();
    let mean = (!success.is_zero()).then(|| {
        BigRational::new(weighted, success.clone()).to_f64().unwrap_or(0.0)
    });
    let levels = [
        (0.5, BigInt::from(1), BigInt::from(2)),
        (0.75, BigInt::from(3), BigInt::from(4)),
        (0.9, BigInt::from(9), BigInt::from(10)),
        (0.95, BigInt::from(19), BigInt::from(20)),
        (0.99, BigInt::from(99), BigInt::from(100)),
    ];
    let percentiles = levels.into_iter().map(|(level, numerator, level_denominator)| {
        let target = &success * numerator;
        let trial = cumulative.iter()
            .position(|value| value * &level_denominator >= target)
            .unwrap_or(cumulative.len().saturating_sub(1));
        (level, trial as u32)
    }).collect();
    ExactFirstHitResult {
        pmf: pmf.iter().cloned()
            .map(|numerator| exact_probability(numerator, denominator))
            .collect(),
        cdf: cumulative.into_iter()
            .map(|numerator| exact_probability(numerator, denominator))
            .collect(),
        failure_reachable: exact_probability(failure, denominator),
        mean,
        percentiles,
    }
}

#[cfg(all(test, feature = "parallel"))]
mod tests {
    use super::*;
    use crate::{compile, ir::ModelIr};
    use serde_json::json;

    #[test]
    fn exact_result_is_identical_across_thread_counts() {
        let ir: ModelIr = serde_json::from_value(json!({
            "irVersion": 1,
            "name": "parallel exact determinism",
            "entities": [
                {"id": "hit", "name": "hit", "prob": {"lit": "1/3"}},
                {"id": "bonus", "name": "bonus", "prob": {"lit": "1/5"}}
            ],
            "stateVars": [],
            "probRules": [],
            "transitions": [],
            "triggers": [],
            "run": {
                "maxTrials": 12,
                "trackJoint": ["hit", "bonus"],
                "numeric": "exact",
                "condition": {"ge": [{"var": "nHit"}, {"lit": "3"}]}
            }
        })).unwrap();
        let model = compile(&ir).unwrap();
        let run = |threads| {
            rayon::ThreadPoolBuilder::new().num_threads(threads).build().unwrap()
                .install(|| run_exact(&model, ExactOptions::default(), |_, _| true).unwrap())
        };
        let mut one = run(1);
        let mut four = run(4);
        one.elapsed_ms = 0;
        four.elapsed_ms = 0;

        assert_eq!(
            serde_json::to_vec(&one).unwrap(),
            serde_json::to_vec(&four).unwrap(),
        );
    }
}
