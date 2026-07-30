use crate::compile::{CompiledModel, TrackedDimension};
use crate::engine_dp::{MarginalAxis, MarginalCell, MarginalSeriesPoint};
#[cfg(not(target_arch = "wasm32"))]
use crate::snapshot::SnapshotSession;
use crate::state::{StateCodec, StateCodecError};
use num_bigint::BigInt;
use num_integer::Integer;
use num_rational::BigRational;
use num_traits::{One, ToPrimitive, Zero};
use rustc_hash::FxHashMap;
use serde::Serialize;
use std::collections::BTreeMap;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;
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
        Self {
            max_trials: 2_000,
            max_states: 200_000,
            max_memory_bytes: 2 * 1024 * 1024 * 1024,
            reduce_layers: false,
        }
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
    pub model_hash: String,
    pub trials: u32,
    pub tracked_leaf_ids: Vec<String>,
    pub joint: Vec<ExactCell>,
    pub first_hit: Option<ExactFirstHitResult>,
    pub denominator: String,
    pub elapsed_ms: u64,
    pub peak_states: usize,
    pub clamp_events: u64,
    pub accumulator_clamp_events: u64,
    pub trial_series: ExactTrialSeriesResult,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExactTrialSeriesResult {
    pub mode: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub marginal: Vec<MarginalSeriesPoint>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub checkpoints: Vec<ExactCheckpointSeriesPoint>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExactCheckpointSeriesPoint {
    pub trial: u32,
    pub joint: Vec<ExactCell>,
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
        return Err(ExactError::TrialLimit {
            actual: model.max_trials,
            limit: options.max_trials,
        });
    }
    let started = Instant::now();
    let lcm = &model.exact_lcm;
    let weights: Vec<Vec<Vec<BigInt>>> = model
        .prob_table
        .entries
        .iter()
        .map(|by_trial| {
            by_trial
                .iter()
                .map(|p| {
                    p.exact
                        .iter()
                        .map(|r| r.numer() * (lcm / r.denom()))
                        .collect()
                })
                .collect()
        })
        .collect();
    let codec = StateCodec::with_accumulators(
        &model.control_max,
        &model.accumulator_max,
        &model.state_count_max,
    )?;
    let initial = codec.encode_full(
        &model.control_init,
        &model.accumulator_init,
        &vec![0; model.state_leaves.len()],
    )?;
    let mut cells = FxHashMap::from_iter([(initial, BigInt::one())]);
    let mut denominator = BigInt::one();
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(session) = snapshot.as_deref_mut() {
        session.on_exact_layer(model, &codec, 0, &cells, &denominator);
    }
    let mut peak_states = 1usize;
    let mut hit_pmf = model
        .condition
        .as_ref()
        .map(|_| vec![BigInt::zero(); model.max_trials as usize + 1]);
    let mut trial = 0u32;
    let mut accumulator_clamp_events = 0u64;
    let mut trial_series = ExactTrialSeriesResult {
        mode: match model.trial_series {
            crate::ir::TrialSeriesMode::None => "none",
            crate::ir::TrialSeriesMode::Marginal => "marginal",
            crate::ir::TrialSeriesMode::Checkpoints => "checkpoints",
        }
        .into(),
        ..Default::default()
    };
    while trial < model.max_trials {
        let draw_trial = trial + 1;
        let consumed_trials = model.consumed_trials_after(draw_trial);
        let next_trial = draw_trial + consumed_trials;
        if let Some(pmf) = &mut hit_pmf {
            for numerator in pmf {
                *numerator *= lcm;
            }
        }
        let source: Vec<_> = cells.into_iter().collect();
        let expanded = expand_exact_layer(model, &codec, &weights, &source, draw_trial);
        let mut next = expanded.cells;
        accumulator_clamp_events += expanded.accumulator_clamps;
        if let Some(pmf) = &mut hit_pmf {
            for (hit_trial, contribution) in expanded.hits {
                pmf[hit_trial] += contribution;
            }
        }
        denominator *= lcm;
        if options.reduce_layers {
            let gcd = next
                .values()
                .chain(hit_pmf.iter().flat_map(|pmf| pmf.iter()))
                .fold(denominator.clone(), |g, n| g.gcd(n));
            if gcd > BigInt::one() {
                denominator /= &gcd;
                for value in next.values_mut() {
                    *value /= &gcd;
                }
                if let Some(pmf) = &mut hit_pmf {
                    for value in pmf {
                        *value /= &gcd;
                    }
                }
            }
        }
        if next.len() > options.max_states {
            return Err(ExactError::StateLimit {
                actual: next.len(),
                limit: options.max_states,
            });
        }
        let estimated = next.len() as u64 * ((denominator.bits() + 7) / 8 + 64);
        if estimated > options.max_memory_bytes {
            return Err(ExactError::MemoryLimit {
                actual: estimated,
                limit: options.max_memory_bytes,
            });
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
        match model.trial_series {
            crate::ir::TrialSeriesMode::Marginal => {
                trial_series.marginal.push(exact_marginalize_layer(
                    model,
                    &codec,
                    next_trial,
                    &next,
                    &denominator,
                ));
            }
            crate::ir::TrialSeriesMode::Checkpoints
                if model.series_checkpoints.contains(&next_trial) =>
            {
                trial_series.checkpoints.push(ExactCheckpointSeriesPoint {
                    trial: next_trial,
                    joint: exact_joint_cells(model, &codec, &next, &denominator),
                });
            }
            _ => {}
        }
        cells = next;
        trial = next_trial;
        if !progress(trial, model.max_trials) {
            return Err(ExactError::Cancelled);
        }
    }
    let denominator_string = denominator.to_string();
    let joint = exact_joint_cells(model, &codec, &cells, &denominator);
    let first_hit = hit_pmf.map(|pmf| summarize_exact_first_hit(&pmf, &denominator));
    Ok(ExactResult {
        numeric: "exact".into(),
        model_hash: model.model_hash_hex(),
        trials: trial,
        tracked_leaf_ids: model.tracked_ids.clone(),
        joint,
        first_hit,
        denominator: denominator_string,
        elapsed_ms: started.elapsed().as_millis() as u64,
        peak_states,
        clamp_events: model.prob_table.clamp_events,
        accumulator_clamp_events,
        trial_series,
    })
}

struct ExactLayerExpansion {
    cells: FxHashMap<u64, BigInt>,
    hits: Vec<(usize, BigInt)>,
    accumulator_clamps: u64,
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
    left.accumulator_clamps += right.accumulator_clamps;
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
    let ti = if model.prob_table.trial_dependent {
        draw_trial as usize - 1
    } else {
        0
    };
    let mut cells = FxHashMap::default();
    cells.reserve(source.len().saturating_mul(2));
    let mut hits = Vec::new();
    let mut base_control = vec![0; codec.control_len()];
    let mut base_accumulators = vec![0; codec.accumulator_len()];
    let mut base_counts = vec![0; codec.count_len()];
    let mut successor_control = vec![0; codec.control_len()];
    let mut successor_accumulators = vec![0; codec.accumulator_len()];
    let mut successor_counts = vec![0; codec.count_len()];
    let mut transition_before = vec![0; codec.control_len()];
    let mut accumulator_clamps = 0u64;
    for (state, numerator) in source {
        let ci = codec.control_index(*state);
        let probability_ci = model.probability_table_index(ci);
        codec.decode_full_into(
            *state,
            &mut base_control,
            &mut base_accumulators,
            &mut base_counts,
        );
        for (leaf, weight) in weights[probability_ci][ti].iter().enumerate() {
            if weight.is_zero() {
                continue;
            }
            let contribution = numerator * weight;
            successor_control.copy_from_slice(&base_control);
            successor_accumulators.copy_from_slice(&base_accumulators);
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
            accumulator_clamps += model.apply_accumulators(
                &successor_control,
                &mut successor_accumulators,
                leaf,
                draw_trial,
            );
            if model.condition_matches_sparse(&successor_counts, draw_trial) {
                hits.push((draw_trial as usize, contribution));
                continue;
            }
            let mut grant_hit = None;
            let (applied_consumed, grant_clamps) = model.apply_triggers_sparse(
                &mut successor_control,
                &mut successor_accumulators,
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
            accumulator_clamps += grant_clamps;
            debug_assert_eq!(applied_consumed, consumed_trials);
            if let Some(hit_trial) = grant_hit {
                hits.push((hit_trial as usize, contribution));
                continue;
            }
            let successor = codec
                .encode_full(
                    &successor_control,
                    &successor_accumulators,
                    &successor_counts,
                )
                .expect("compiled successor state must fit its codec");
            *cells.entry(successor).or_default() += contribution;
        }
    }
    ExactLayerExpansion {
        cells,
        hits,
        accumulator_clamps,
    }
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
    let mut accumulator_clamps = 0u64;
    for (state, numerator) in source {
        let control_index = codec.control_index(*state);
        let probability_control_index = model.probability_table_index(control_index);
        for (leaf, weight) in weights[probability_control_index][probability_trial]
            .iter()
            .enumerate()
        {
            if weight.is_zero() {
                continue;
            }
            let next_control = if model.prob_table.control_invariant {
                0
            } else {
                model.transition_control_index(control_index, leaf, draw_trial)
            };
            let mut successor = codec.replace_control_index(*state, next_control);
            for accumulator in 0..codec.accumulator_len() {
                let current = codec.accumulator_value(*state, accumulator);
                let transition = model.accumulator_transition(
                    accumulator,
                    next_control,
                    current,
                    leaf,
                    draw_trial,
                );
                successor =
                    codec.replace_accumulator_index(successor, accumulator, transition.value);
                accumulator_clamps += u64::from(transition.clamped);
            }
            if let Some(position) = model.state_leaf_position(leaf) {
                successor = codec.increment_count(successor, position);
            }
            *cells.entry(successor).or_default() += numerator * weight;
        }
    }
    ExactLayerExpansion {
        cells,
        hits: Vec::new(),
        accumulator_clamps,
    }
}

fn exact_tracked_values(model: &CompiledModel, codec: &StateCodec, state: u64) -> Vec<u32> {
    let (_, accumulators, counts) = codec.decode_full(state);
    model
        .tracked_dimensions
        .iter()
        .map(|dimension| match dimension {
            TrackedDimension::Leaf(leaf) => model
                .state_leaves
                .iter()
                .position(|state_leaf| state_leaf == leaf)
                .map(|position| counts[position])
                .unwrap_or(0),
            TrackedDimension::Accumulator(index) => accumulators[*index],
            TrackedDimension::DerivedAccumulator(index) => model.derived_accumulator_leaves[*index]
                .iter()
                .filter_map(|leaf| {
                    model
                        .state_leaves
                        .iter()
                        .position(|state_leaf| state_leaf == leaf)
                        .map(|position| counts[position])
                })
                .sum(),
        })
        .collect()
}

fn exact_joint_cells(
    model: &CompiledModel,
    codec: &StateCodec,
    cells: &FxHashMap<u64, BigInt>,
    denominator: &BigInt,
) -> Vec<ExactCell> {
    let mut marginalized: BTreeMap<Vec<u32>, BigInt> = BTreeMap::new();
    for (state, numerator) in cells {
        let key = exact_tracked_values(model, codec, *state);
        *marginalized.entry(key).or_default() += numerator;
    }
    let denominator_string = denominator.to_string();
    marginalized
        .into_iter()
        .map(|(counts, numerator)| ExactCell {
            counts,
            probability: BigRational::new(numerator.clone(), denominator.clone())
                .to_f64()
                .unwrap_or(0.0),
            numerator: numerator.to_string(),
            denominator: denominator_string.clone(),
        })
        .collect()
}

fn exact_marginalize_layer(
    model: &CompiledModel,
    codec: &StateCodec,
    trial: u32,
    layer: &FxHashMap<u64, BigInt>,
    denominator: &BigInt,
) -> MarginalSeriesPoint {
    let mut axes: Vec<BTreeMap<u32, BigInt>> = model
        .tracked_dimensions
        .iter()
        .map(|_| BTreeMap::new())
        .collect();
    for (state, numerator) in layer {
        for (axis, value) in exact_tracked_values(model, codec, *state)
            .into_iter()
            .enumerate()
        {
            *axes[axis].entry(value).or_default() += numerator;
        }
    }
    MarginalSeriesPoint {
        trial,
        axes: axes
            .into_iter()
            .enumerate()
            .map(|(axis, cells)| MarginalAxis {
                id: model.tracked_ids[axis].clone(),
                cells: cells
                    .into_iter()
                    .map(|(value, numerator)| {
                        let probability = BigRational::new(numerator.clone(), denominator.clone())
                            .to_f64()
                            .unwrap_or(0.0);
                        MarginalCell {
                            value,
                            probability,
                            display: format!("{probability:.11e}"),
                        }
                    })
                    .collect(),
            })
            .collect(),
    }
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

fn summarize_exact_first_hit(pmf: &[BigInt], denominator: &BigInt) -> ExactFirstHitResult {
    let mut running = BigInt::zero();
    let mut cumulative = Vec::with_capacity(pmf.len());
    for numerator in pmf {
        running += numerator;
        cumulative.push(running.clone());
    }
    let success = running;
    let failure = denominator - &success;
    let weighted: BigInt = pmf
        .iter()
        .enumerate()
        .map(|(trial, numerator)| numerator * BigInt::from(trial))
        .sum();
    let mean = (!success.is_zero()).then(|| {
        BigRational::new(weighted, success.clone())
            .to_f64()
            .unwrap_or(0.0)
    });
    let levels = [
        (0.5, BigInt::from(1), BigInt::from(2)),
        (0.75, BigInt::from(3), BigInt::from(4)),
        (0.9, BigInt::from(9), BigInt::from(10)),
        (0.95, BigInt::from(19), BigInt::from(20)),
        (0.99, BigInt::from(99), BigInt::from(100)),
    ];
    let percentiles = levels
        .into_iter()
        .map(|(level, numerator, level_denominator)| {
            let target = &success * numerator;
            let trial = cumulative
                .iter()
                .position(|value| value * &level_denominator >= target)
                .unwrap_or(cumulative.len().saturating_sub(1));
            (level, trial as u32)
        })
        .collect();
    ExactFirstHitResult {
        pmf: pmf
            .iter()
            .cloned()
            .map(|numerator| exact_probability(numerator, denominator))
            .collect(),
        cdf: cumulative
            .into_iter()
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
        }))
        .unwrap();
        let model = compile(&ir).unwrap();
        let run = |threads| {
            rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .unwrap()
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
