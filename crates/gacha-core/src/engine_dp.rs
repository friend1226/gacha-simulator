use crate::compile::{CompiledModel, TrackedDimension};
use crate::engine_exact::{run_exact, ExactError, ExactOptions, ExactResult};
use crate::ir::NumericBackend;
use crate::numeric::{Prob, ScaledF64, F64};
#[cfg(not(target_arch = "wasm32"))]
use crate::snapshot::{
    load_snapshot, LoadedSnapshot, SnapshotError, SnapshotManifest, SnapshotOptions,
    SnapshotPolicy, SnapshotSession,
};
use crate::state::StateCodec;
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
pub struct DpOptions {
    pub prune_log10: Option<f64>,
}

impl Default for DpOptions {
    fn default() -> Self {
        Self {
            prune_log10: Some(-18.0),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DpCell {
    pub counts: Vec<u32>,
    pub probability: f64,
    pub display: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DpResult {
    pub numeric: String,
    pub model_hash: String,
    pub trials: u32,
    pub tracked_leaf_ids: Vec<String>,
    pub joint: Vec<DpCell>,
    pub first_hit: Option<FirstHitResult>,
    pub pruned_mass: f64,
    pub elapsed_ms: u64,
    pub clamp_events: u64,
    pub accumulator_clamp_events: u64,
    pub trial_series: TrialSeriesResult,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TrialSeriesResult {
    pub mode: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub marginal: Vec<MarginalSeriesPoint>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub checkpoints: Vec<CheckpointSeriesPoint>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarginalSeriesPoint {
    pub trial: u32,
    pub axes: Vec<MarginalAxis>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarginalAxis {
    pub id: String,
    pub cells: Vec<MarginalCell>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarginalCell {
    pub value: u32,
    pub probability: f64,
    pub display: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointSeriesPoint {
    pub trial: u32,
    pub joint: Vec<DpCell>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum DpRunResult {
    Approximate(DpResult),
    Exact(ExactResult),
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, thiserror::Error)]
pub enum SnapshotRunError {
    #[error(transparent)]
    Exact(#[from] ExactError),
    #[error(transparent)]
    Snapshot(#[from] SnapshotError),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FirstHitResult {
    pub pmf: Vec<f64>,
    pub cdf: Vec<f64>,
    pub failure_reachable: f64,
    pub mean: Option<f64>,
    pub percentiles: Vec<(f64, u32)>,
}

pub fn run_dp(
    model: &CompiledModel,
    options: DpOptions,
    progress: impl FnMut(u32, u32) -> bool,
) -> Result<DpRunResult, ExactError> {
    match model.numeric {
        NumericBackend::F64 => Ok(DpRunResult::Approximate(run_generic::<F64>(
            model, options, progress, "f64",
        ))),
        NumericBackend::Scaled => Ok(DpRunResult::Approximate(run_generic::<ScaledF64>(
            model, options, progress, "scaled",
        ))),
        NumericBackend::Exact => {
            run_exact(model, ExactOptions::default(), progress).map(DpRunResult::Exact)
        }
    }
}

pub fn run_dp_f64(
    model: &CompiledModel,
    options: DpOptions,
    progress: impl FnMut(u32, u32) -> bool,
) -> DpResult {
    run_generic::<F64>(model, options, progress, "f64")
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run_dp_with_snapshots(
    model: &CompiledModel,
    options: DpOptions,
    snapshot_options: SnapshotOptions,
    progress: impl FnMut(u32, u32) -> bool,
) -> Result<(DpRunResult, SnapshotManifest), SnapshotRunError> {
    let mut session = SnapshotSession::new(model, snapshot_options)?;
    let result = match model.numeric {
        NumericBackend::F64 => DpRunResult::Approximate(run_generic_with_snapshot::<F64>(
            model,
            options,
            progress,
            "f64",
            Some(&mut session),
        )),
        NumericBackend::Scaled => DpRunResult::Approximate(run_generic_with_snapshot::<ScaledF64>(
            model,
            options,
            progress,
            "scaled",
            Some(&mut session),
        )),
        NumericBackend::Exact => DpRunResult::Exact(crate::engine_exact::run_exact_with_snapshot(
            model,
            ExactOptions::default(),
            progress,
            &mut session,
        )?),
    };
    Ok((result, session.finish()?))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn restore_dp_snapshot(
    model: &CompiledModel,
    options: DpOptions,
    output_dir: std::path::PathBuf,
    target_layer: u32,
) -> Result<LoadedSnapshot, SnapshotRunError> {
    let mut pinned_layers = std::collections::BTreeSet::new();
    pinned_layers.insert(target_layer);
    let (_, manifest) = run_dp_with_snapshots(
        model,
        options,
        SnapshotOptions {
            output_dir,
            policy: SnapshotPolicy::Checkpoint,
            pinned_layers,
            confirm_full: false,
        },
        |done, _| done < target_layer,
    )?;
    let target_name = format!("layer-{target_layer:06}");
    let path = manifest
        .files
        .iter()
        .find(|path| path.file_stem().and_then(|name| name.to_str()) == Some(target_name.as_str()))
        .ok_or_else(|| {
            SnapshotError::Invalid(format!("target layer {target_layer} was not produced",))
        })?;
    Ok(load_snapshot(path, model.model_hash)?)
}

fn run_generic<P: Prob>(
    model: &CompiledModel,
    options: DpOptions,
    progress: impl FnMut(u32, u32) -> bool,
    backend: &str,
) -> DpResult {
    run_generic_with_snapshot::<P>(model, options, progress, backend, None)
}

fn run_generic_with_snapshot<P: Prob>(
    model: &CompiledModel,
    options: DpOptions,
    mut progress: impl FnMut(u32, u32) -> bool,
    backend: &str,
    mut snapshot: Option<&mut SnapshotSession>,
) -> DpResult {
    let started = Instant::now();
    let converted: Vec<Vec<Vec<P>>> = model
        .prob_table
        .entries
        .iter()
        .map(|by_trial| {
            by_trial
                .iter()
                .map(|leaf_probs| leaf_probs.exact.iter().map(P::from_rational).collect())
                .collect()
        })
        .collect();
    let codec = StateCodec::with_accumulators(
        &model.control_max,
        &model.accumulator_max,
        &model.state_count_max,
    )
    .expect("compiler must reject state spaces that exceed u64");
    let initial = codec
        .encode_full(
            &model.control_init,
            &model.accumulator_init,
            &vec![0; model.state_leaves.len()],
        )
        .expect("compiled initial state must fit its codec");
    let mut layer = FxHashMap::from_iter([(initial, P::one())]);
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(session) = snapshot.as_deref_mut() {
        session.on_approx_layer(model, &codec, 0, &layer);
    }
    let mut hit_pmf = model
        .condition
        .as_ref()
        .map(|_| vec![P::zero(); model.max_trials as usize + 1]);
    let mut pruned_mass = 0.0;
    let mut completed_trials = 0;
    let mut accumulator_clamp_events = 0u64;
    let mut trial_series = TrialSeriesResult {
        mode: match model.trial_series {
            crate::ir::TrialSeriesMode::None => "none",
            crate::ir::TrialSeriesMode::Marginal => "marginal",
            crate::ir::TrialSeriesMode::Checkpoints => "checkpoints",
        }
        .into(),
        ..Default::default()
    };
    let mut trial = 0u32;
    while trial < model.max_trials {
        let draw_trial = trial + 1;
        let consumed_trials = model.consumed_trials_after(draw_trial);
        let next_trial = draw_trial + consumed_trials;
        let source: Vec<_> = layer.into_iter().collect();
        let expanded = expand_layer(model, &codec, &converted, &source, draw_trial);
        let mut next = expanded.cells;
        if let Some(pmf) = &mut hit_pmf {
            for (hit_trial, contribution) in expanded.hits {
                pmf[hit_trial].add_assign(&contribution);
            }
        }
        accumulator_clamp_events += expanded.accumulator_clamps;
        if let Some(threshold) = options.prune_log10 {
            next.retain(|_, p| {
                let keep = p.magnitude_log10().map(|m| m >= threshold).unwrap_or(true);
                if !keep {
                    pruned_mass += p.to_f64_lossy();
                }
                keep
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(session) = snapshot.as_deref_mut() {
            session.on_approx_layer(model, &codec, next_trial, &next);
        }
        match model.trial_series {
            crate::ir::TrialSeriesMode::Marginal => {
                trial_series
                    .marginal
                    .push(marginalize_layer(model, &codec, next_trial, &next));
            }
            crate::ir::TrialSeriesMode::Checkpoints
                if model.series_checkpoints.contains(&next_trial) =>
            {
                trial_series.checkpoints.push(CheckpointSeriesPoint {
                    trial: next_trial,
                    joint: joint_cells(model, &codec, &next),
                });
            }
            _ => {}
        }
        layer = next;
        completed_trials = next_trial;
        trial = next_trial;
        if !progress(trial, model.max_trials) {
            break;
        }
    }
    let cells = joint_cells(model, &codec, &layer);
    let first_hit = hit_pmf.map(|values| summarize_first_hit(&values));
    DpResult {
        numeric: backend.into(),
        model_hash: model.model_hash_hex(),
        trials: completed_trials,
        tracked_leaf_ids: model.tracked_ids.clone(),
        joint: cells,
        first_hit,
        pruned_mass,
        elapsed_ms: started.elapsed().as_millis() as u64,
        clamp_events: model.prob_table.clamp_events,
        accumulator_clamp_events,
        trial_series,
    }
}

struct LayerExpansion<P> {
    cells: FxHashMap<u64, P>,
    hits: Vec<(usize, P)>,
    accumulator_clamps: u64,
}

#[cfg(feature = "parallel")]
fn expand_layer<P: Prob>(
    model: &CompiledModel,
    codec: &StateCodec,
    converted: &[Vec<Vec<P>>],
    source: &[(u64, P)],
    draw_trial: u32,
) -> LayerExpansion<P> {
    if source.len() <= 1_024 {
        return expand_chunk(model, codec, converted, source, draw_trial);
    }
    let middle = source.len() / 2;
    let (left, right) = source.split_at(middle);
    let (left, right) = rayon::join(
        || expand_layer(model, codec, converted, left, draw_trial),
        || expand_layer(model, codec, converted, right, draw_trial),
    );
    merge_expansions(left, right)
}

#[cfg(not(feature = "parallel"))]
fn expand_layer<P: Prob>(
    model: &CompiledModel,
    codec: &StateCodec,
    converted: &[Vec<Vec<P>>],
    source: &[(u64, P)],
    draw_trial: u32,
) -> LayerExpansion<P> {
    expand_chunk(model, codec, converted, source, draw_trial)
}

fn merge_expansions<P: Prob>(
    mut left: LayerExpansion<P>,
    right: LayerExpansion<P>,
) -> LayerExpansion<P> {
    left.cells.reserve(right.cells.len());
    for (state, contribution) in right.cells {
        left.cells
            .entry(state)
            .or_insert_with(P::zero)
            .add_assign(&contribution);
    }
    left.hits.extend(right.hits);
    left.accumulator_clamps += right.accumulator_clamps;
    left
}

fn expand_chunk<P: Prob>(
    model: &CompiledModel,
    codec: &StateCodec,
    converted: &[Vec<Vec<P>>],
    source: &[(u64, P)],
    draw_trial: u32,
) -> LayerExpansion<P> {
    if model.packed_transition_fast_path() {
        return expand_packed_chunk(model, codec, converted, source, draw_trial);
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
    for (state, mass) in source {
        let ci = codec.control_index(*state);
        codec.decode_full_into(
            *state,
            &mut base_control,
            &mut base_accumulators,
            &mut base_counts,
        );
        for (leaf, p_leaf) in converted[ci][ti].iter().enumerate() {
            if p_leaf.is_zero() {
                continue;
            }
            let contribution = mass.mul(p_leaf);
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
            cells
                .entry(successor)
                .or_insert_with(P::zero)
                .add_assign(&contribution);
        }
    }
    LayerExpansion {
        cells,
        hits,
        accumulator_clamps,
    }
}

fn expand_packed_chunk<P: Prob>(
    model: &CompiledModel,
    codec: &StateCodec,
    converted: &[Vec<Vec<P>>],
    source: &[(u64, P)],
    draw_trial: u32,
) -> LayerExpansion<P> {
    let probability_trial = if model.prob_table.trial_dependent {
        draw_trial as usize - 1
    } else {
        0
    };
    let mut cells = FxHashMap::default();
    cells.reserve(source.len().saturating_mul(2));
    let mut accumulator_clamps = 0u64;
    for (state, mass) in source {
        let control_index = codec.control_index(*state);
        for (leaf, probability) in converted[control_index][probability_trial]
            .iter()
            .enumerate()
        {
            if probability.is_zero() {
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
            let contribution = mass.mul(probability);
            cells
                .entry(successor)
                .or_insert_with(P::zero)
                .add_assign(&contribution);
        }
    }
    LayerExpansion {
        cells,
        hits: Vec::new(),
        accumulator_clamps,
    }
}

fn tracked_values(model: &CompiledModel, codec: &StateCodec, state: u64) -> Vec<u32> {
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

fn joint_cells<P: Prob>(
    model: &CompiledModel,
    codec: &StateCodec,
    layer: &FxHashMap<u64, P>,
) -> Vec<DpCell> {
    let mut joint: BTreeMap<Vec<u32>, P> = BTreeMap::new();
    for (state, probability) in layer {
        let key = tracked_values(model, codec, *state);
        joint
            .entry(key)
            .or_insert_with(P::zero)
            .add_assign(probability);
    }
    joint
        .into_iter()
        .map(|(counts, probability)| DpCell {
            counts,
            probability: probability.to_f64_lossy(),
            display: probability.to_decimal_string(12),
        })
        .collect()
}

fn marginalize_layer<P: Prob>(
    model: &CompiledModel,
    codec: &StateCodec,
    trial: u32,
    layer: &FxHashMap<u64, P>,
) -> MarginalSeriesPoint {
    let mut axes: Vec<BTreeMap<u32, P>> = model
        .tracked_dimensions
        .iter()
        .map(|_| BTreeMap::new())
        .collect();
    for (state, probability) in layer {
        for (axis, value) in tracked_values(model, codec, *state).into_iter().enumerate() {
            axes[axis]
                .entry(value)
                .or_insert_with(P::zero)
                .add_assign(probability);
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
                    .map(|(value, probability)| MarginalCell {
                        value,
                        probability: probability.to_f64_lossy(),
                        display: probability.to_decimal_string(12),
                    })
                    .collect(),
            })
            .collect(),
    }
}

fn summarize_first_hit<P: Prob>(pmf: &[P]) -> FirstHitResult {
    let pmf: Vec<f64> = pmf.iter().map(|value| value.to_f64_lossy()).collect();
    let mut running = 0.0;
    let cdf: Vec<f64> = pmf
        .iter()
        .map(|value| {
            running += value;
            running
        })
        .collect();
    let success = running;
    let weighted: f64 = pmf
        .iter()
        .enumerate()
        .map(|(trial, p)| trial as f64 * p)
        .sum();
    let mean = (success > 0.0).then_some(weighted / success);
    let levels = [0.5, 0.75, 0.9, 0.95, 0.99];
    let percentiles = levels
        .into_iter()
        .map(|level| {
            let target = success * level;
            let trial = cdf
                .iter()
                .position(|value| *value >= target)
                .unwrap_or(cdf.len().saturating_sub(1));
            (level, trial as u32)
        })
        .collect();
    FirstHitResult {
        pmf,
        cdf,
        failure_reachable: (1.0 - success).max(0.0),
        mean,
        percentiles,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_mc::{run_mc, McOptions};
    use crate::ir::{ModelIr, Trigger};
    use crate::{compile, run_exact, ExactOptions};
    use serde_json::json;
    use std::collections::{BTreeMap, BTreeSet};

    const WILSON_95_Z: f64 = 1.959963984540054;

    fn bernoulli_ir(name: &str, probability: &str, max_trials: u32) -> ModelIr {
        serde_json::from_value(json!({
            "irVersion": 1,
            "name": name,
            "entities": [{"id": "hit", "name": "hit", "prob": {"lit": probability}}],
            "stateVars": [],
            "probRules": [],
            "transitions": [],
            "triggers": [],
            "run": {"maxTrials": max_trials, "trackJoint": ["hit"], "numeric": "scaled"}
        }))
        .unwrap()
    }

    fn cross_validation_models() -> Vec<ModelIr> {
        let mut models = vec![
            bernoulli_ir("coin", "1/2", 4),
            bernoulli_ir("one-third", "1/3", 5),
            bernoulli_ir("two-fifths", "2/5", 6),
            bernoulli_ir("three-quarters", "3/4", 4),
            serde_json::from_value(json!({
                "irVersion": 1,
                "name": "two top-level entities",
                "entities": [
                    {"id": "rare", "name": "rare", "prob": {"lit": "1/4"}},
                    {"id": "bonus", "name": "bonus", "prob": {"lit": "1/5"}}
                ],
                "stateVars": [],
                "probRules": [],
                "transitions": [],
                "triggers": [],
                "run": {
                    "maxTrials": 4,
                    "trackJoint": ["rare", "bonus"],
                    "numeric": "scaled"
                }
            }))
            .unwrap(),
            serde_json::from_value(json!({
                "irVersion": 1,
                "name": "nested entity",
                "entities": [{
                    "id": "rare",
                    "name": "rare",
                    "prob": {"lit": "1/2"},
                    "children": [{"id": "pickup", "name": "pickup", "prob": {"lit": "1/5"}}]
                }],
                "stateVars": [],
                "probRules": [],
                "transitions": [],
                "triggers": [],
                "run": {
                    "maxTrials": 4,
                    "trackJoint": ["pickup", "rare__self"],
                    "numeric": "scaled"
                }
            }))
            .unwrap(),
            serde_json::from_value(json!({
                "irVersion": 1,
                "name": "grant after normal draw",
                "entities": [{"id": "hit", "name": "hit", "prob": {"lit": "1/3"}}],
                "stateVars": [],
                "probRules": [],
                "transitions": [],
                "triggers": [{
                    "at": {"trialCount": 3},
                    "grant": {
                        "leaf": "hit",
                        "amount": 1,
                        "consumesTrial": false,
                        "appliesTransitions": false
                    }
                }],
                "run": {"maxTrials": 5, "trackJoint": ["hit"], "numeric": "scaled"}
            }))
            .unwrap(),
            serde_json::from_value(json!({
                "irVersion": 1,
                "name": "state-dependent pity",
                "entities": [{"id": "hit", "name": "hit", "prob": {"lit": "1/4"}}],
                "stateVars": [{"id": "pity", "init": 0, "max": 2, "role": "control"}],
                "probRules": [{
                    "target": "hit",
                    "expr": {
                        "if": {"ge": [{"var": "pity"}, {"lit": "2"}]},
                        "then": {"lit": "3/4"},
                        "else": {"lit": "1/4"}
                    }
                }],
                "transitions": [
                    {"when": {"leafOf": "hit"}, "set": {"pity": {"lit": "0"}}},
                    {
                        "when": {"not": {"leafOf": "hit"}},
                        "set": {"pity": {"add": [{"var": "pity"}, {"lit": "1"}]}}
                    }
                ],
                "triggers": [],
                "run": {"maxTrials": 6, "trackJoint": ["hit"], "numeric": "scaled"}
            }))
            .unwrap(),
        ];

        let mut blue_archive: ModelIr =
            serde_json::from_str(include_str!("../../../presets/blue-archive-pickup.json"))
                .unwrap();
        blue_archive.name = "blue archive preset (short CI horizon)".into();
        blue_archive.run.max_trials = 4;
        models.push(blue_archive);

        let mut simple_pity: ModelIr =
            serde_json::from_str(include_str!("../../../presets/simple-pity.json")).unwrap();
        simple_pity.name = "simple pity preset (near-pity CI horizon)".into();
        simple_pity.state_vars[0].init = 87;
        simple_pity.run.max_trials = 5;
        simple_pity.run.condition = None;
        models.push(simple_pity);

        models
    }

    fn probability_map(result: &DpResult) -> BTreeMap<Vec<u32>, f64> {
        result
            .joint
            .iter()
            .map(|cell| (cell.counts.clone(), cell.probability))
            .collect()
    }

    #[test]
    fn binomial_mass_is_conserved() {
        let ir: ModelIr = serde_json::from_value(json!({
            "irVersion":1,"name":"coin","entities":[{"id":"hit","name":"hit","prob":{"lit":"1/2"}}],
            "stateVars":[],"probRules":[],"transitions":[],"triggers":[],
            "run":{"maxTrials":10,"trackJoint":["hit"],"numeric":"scaled"}
        }))
        .unwrap();
        let model = compile(&ir).unwrap();
        let result = run_dp(&model, DpOptions { prune_log10: None }, |_, _| true).unwrap();
        let DpRunResult::Approximate(result) = result else {
            panic!("scaled backend must use approximate DP");
        };
        let total: f64 = result.joint.iter().map(|c| c.probability).sum();
        assert!((total - 1.0).abs() < 1e-12);
        assert!((result.joint[5].probability - 252.0 / 1024.0).abs() < 1e-12);
    }

    #[test]
    fn control_invariant_probability_table_collapses_unused_control_state() {
        let baseline = bernoulli_ir("baseline", "1/3", 20);
        let with_unused_control: ModelIr = serde_json::from_value(json!({
            "irVersion": 1,
            "name": "unused control",
            "entities": [{"id": "hit", "name": "hit", "prob": {"lit": "1/3"}}],
            "stateVars": [{"id": "pity", "init": 0, "max": 179, "role": "control"}],
            "probRules": [],
            "transitions": [
                {"when": {"leafOf": "hit"}, "set": {"pity": {"lit": "0"}}},
                {"when": {"not": {"leafOf": "hit"}}, "set": {
                    "pity": {"add": [{"var": "pity"}, {"lit": "1"}]}
                }}
            ],
            "triggers": [],
            "run": {"maxTrials": 20, "trackJoint": ["hit"], "numeric": "scaled"}
        }))
        .unwrap();
        let baseline = compile(&baseline).unwrap();
        let optimized = compile(&with_unused_control).unwrap();
        assert!(optimized.prob_table.control_invariant);

        let baseline = run_generic::<ScaledF64>(
            &baseline,
            DpOptions { prune_log10: None },
            |_, _| true,
            "scaled",
        );
        let optimized = run_generic::<ScaledF64>(
            &optimized,
            DpOptions { prune_log10: None },
            |_, _| true,
            "scaled",
        );
        assert_eq!(
            baseline
                .joint
                .iter()
                .map(|cell| (&cell.counts, &cell.display))
                .collect::<Vec<_>>(),
            optimized
                .joint
                .iter()
                .map(|cell| (&cell.counts, &cell.display))
                .collect::<Vec<_>>(),
        );
    }

    #[test]
    fn exact_numeric_dispatches_to_bigint_engine_and_reports_clamps() {
        let ir: ModelIr = serde_json::from_value(json!({
            "irVersion":1,"name":"clamped exact",
            "entities":[{
                "id":"rare","name":"rare","prob":{"lit":"1/4"},
                "children":[{"id":"pickup","name":"pickup","prob":{"lit":"1/2"}}]
            }],
            "nestingPolicy":"clampChildren",
            "stateVars":[],"probRules":[],"transitions":[],"triggers":[],
            "run":{"maxTrials":2,"trackJoint":["pickup"],"numeric":"exact"}
        }))
        .unwrap();
        let model = compile(&ir).unwrap();
        let result = run_dp(&model, DpOptions { prune_log10: None }, |_, _| true).unwrap();
        let DpRunResult::Exact(result) = result else {
            panic!("exact backend must use BigInt DP");
        };

        assert_eq!(result.numeric, "exact");
        assert_eq!(result.denominator, "16");
        assert_eq!(result.clamp_events, 1);
        assert_eq!(
            result
                .joint
                .iter()
                .map(|cell| cell.numerator.as_str())
                .collect::<Vec<_>>(),
            vec!["9", "6", "1"],
        );
    }

    #[test]
    fn monte_carlo_and_dp_cross_validate_for_ten_models() {
        const RUNS: u64 = 1_000_000;

        let models = cross_validation_models();
        assert_eq!(
            models.len(),
            10,
            "§9.1 requires at least ten cross-validation models"
        );

        let mut checked_cells = 0usize;
        let mut outliers = Vec::new();
        for (case, ir) in models.into_iter().enumerate() {
            let model = compile(&ir).unwrap();
            let dp = run_generic::<ScaledF64>(
                &model,
                DpOptions { prune_log10: None },
                |_, _| true,
                "scaled",
            );
            let mc = run_mc(
                &model,
                McOptions {
                    runs: RUNS,
                    seed: 0x5eed_cafe_u64 + case as u64,
                    confidence_z: WILSON_95_Z,
                    batch_size: 100_000,
                },
                |_, _| true,
            );

            assert_eq!(mc.runs, RUNS);
            assert_eq!(mc.tracked_leaf_ids, dp.tracked_leaf_ids);
            let dp_cells = probability_map(&dp);
            let mc_cells: BTreeMap<_, _> = mc
                .joint
                .iter()
                .map(|cell| (cell.counts.clone(), cell.interval))
                .collect();
            let keys: BTreeSet<_> = dp_cells.keys().chain(mc_cells.keys()).cloned().collect();
            for counts in keys {
                let probability = dp_cells.get(&counts).copied().unwrap_or(0.0);
                let interval = mc_cells
                    .get(&counts)
                    .copied()
                    .unwrap_or_else(|| crate::report::wilson(0, RUNS, WILSON_95_Z));
                checked_cells += 1;
                if probability < interval.lower || probability > interval.upper {
                    outliers.push(format!(
                        "{} {:?}: DP={probability:.12e}, Wilson=[{:.12e}, {:.12e}]",
                        model.name, counts, interval.lower, interval.upper,
                    ));
                }
            }
        }

        assert!(
            outliers.len() * 100 < checked_cells * 5,
            "{} of {checked_cells} cells ({:.2}%) fell outside Wilson 95% intervals:\n{}",
            outliers.len(),
            outliers.len() as f64 * 100.0 / checked_cells as f64,
            outliers.join("\n"),
        );
    }

    #[test]
    fn exact_and_scaled_dp_agree_within_relative_error() {
        let ir: ModelIr = serde_json::from_value(json!({
            "irVersion": 1,
            "name": "exact-scaled parity",
            "entities": [{
                "id": "star3",
                "name": "star3",
                "prob": {"lit": "2/5"},
                "children": [{"id": "pickup", "name": "pickup", "prob": {"lit": "1/7"}}]
            }],
            "stateVars": [{"id": "pity", "init": 0, "max": 2, "role": "control"}],
            "probRules": [{
                "target": "star3",
                "expr": {
                    "if": {"ge": [{"var": "pity"}, {"lit": "2"}]},
                    "then": {"lit": "3/5"},
                    "else": {"lit": "2/5"}
                }
            }],
            "transitions": [
                {"when": {"leafOf": "star3"}, "set": {"pity": {"lit": "0"}}},
                {
                    "when": {"not": {"leafOf": "star3"}},
                    "set": {"pity": {"add": [{"var": "pity"}, {"lit": "1"}]}}
                }
            ],
            "triggers": [{
                "at": {"trialCount": 7},
                "grant": {
                    "leaf": "pickup",
                    "amount": 1,
                    "consumesTrial": false,
                    "appliesTransitions": true
                }
            }],
            "run": {
                "maxTrials": 12,
                "trackJoint": ["pickup", "star3__self"],
                "numeric": "exact"
            }
        }))
        .unwrap();
        let model = compile(&ir).unwrap();
        let exact = run_exact(&model, ExactOptions::default(), |_, _| true).unwrap();
        let scaled = run_generic::<ScaledF64>(
            &model,
            DpOptions { prune_log10: None },
            |_, _| true,
            "scaled",
        );

        assert_eq!(exact.tracked_leaf_ids, scaled.tracked_leaf_ids);
        let exact_cells: BTreeMap<_, _> = exact
            .joint
            .iter()
            .map(|cell| (cell.counts.clone(), cell.probability))
            .collect();
        let scaled_cells = probability_map(&scaled);
        assert_eq!(
            exact_cells.keys().collect::<Vec<_>>(),
            scaled_cells.keys().collect::<Vec<_>>()
        );
        for (counts, expected) in exact_cells {
            let actual = scaled_cells[&counts];
            let relative_error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
            assert!(
                relative_error <= 1e-10,
                "{counts:?}: exact={expected:.16e}, scaled={actual:.16e}, relative error={relative_error:.3e}",
            );
        }
    }

    #[test]
    fn guaranteed_pickup_shifts_parent_entity_distribution_by_one() {
        let base: ModelIr = serde_json::from_value(json!({
            "irVersion": 1,
            "name": "grant propagation baseline",
            "entities": [{
                "id": "star3",
                "name": "star3",
                "prob": {"lit": "0.03"},
                "children": [{"id": "pickup", "name": "pickup", "prob": {"lit": "0.007"}}]
            }],
            "stateVars": [],
            "probRules": [],
            "transitions": [],
            "triggers": [],
            "run": {"maxTrials": 200, "trackJoint": ["star3"], "numeric": "scaled"}
        }))
        .unwrap();
        let mut granted = base.clone();
        granted.name = "grant propagation with pickup".into();
        granted.triggers = serde_json::from_value::<Vec<Trigger>>(json!([{
            "at": {"trialCount": 200},
            "grant": {
                "leaf": "pickup",
                "amount": 1,
                "consumesTrial": false,
                "appliesTransitions": true
            }
        }]))
        .unwrap();

        let baseline_model = compile(&base).unwrap();
        let granted_model = compile(&granted).unwrap();
        let baseline = run_generic::<ScaledF64>(
            &baseline_model,
            DpOptions { prune_log10: None },
            |_, _| true,
            "scaled",
        );
        let with_grant = run_generic::<ScaledF64>(
            &granted_model,
            DpOptions { prune_log10: None },
            |_, _| true,
            "scaled",
        );

        fn parent_distribution(result: &DpResult) -> BTreeMap<u32, f64> {
            let mut distribution = BTreeMap::new();
            for cell in &result.joint {
                *distribution.entry(cell.counts.iter().sum()).or_default() += cell.probability;
            }
            distribution
        }

        let baseline_star3 = parent_distribution(&baseline);
        let granted_star3 = parent_distribution(&with_grant);
        assert!(!granted_star3.contains_key(&0));
        assert_eq!(baseline_star3.len(), granted_star3.len());
        for (count, probability) in baseline_star3 {
            let shifted = granted_star3.get(&(count + 1)).copied().unwrap_or_default();
            assert!(
                (shifted - probability).abs() <= 1e-12,
                "nStar3={count} did not shift exactly once: baseline={probability:.16e}, shifted={shifted:.16e}",
            );
        }
    }
}
