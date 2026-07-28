use crate::compile::CompiledModel;
use crate::engine_exact::{run_exact, ExactError, ExactOptions, ExactResult};
use crate::ir::NumericBackend;
use crate::numeric::{F64, Prob, ScaledF64};
use crate::state::StateCodec;
use serde::Serialize;
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct DpOptions {
    pub prune_log10: Option<f64>,
}

impl Default for DpOptions {
    fn default() -> Self { Self { prune_log10: Some(-18.0) } }
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
    pub trials: u32,
    pub tracked_leaf_ids: Vec<String>,
    pub joint: Vec<DpCell>,
    pub first_hit: Option<FirstHitResult>,
    pub pruned_mass: f64,
    pub elapsed_ms: u64,
    pub clamp_events: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum DpRunResult {
    Approximate(DpResult),
    Exact(ExactResult),
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
        NumericBackend::F64 => Ok(DpRunResult::Approximate(
            run_generic::<F64>(model, options, progress, "f64"),
        )),
        NumericBackend::Scaled => Ok(DpRunResult::Approximate(
            run_generic::<ScaledF64>(model, options, progress, "scaled"),
        )),
        NumericBackend::Exact => run_exact(model, ExactOptions::default(), progress)
            .map(DpRunResult::Exact),
    }
}

pub fn run_dp_f64(
    model: &CompiledModel,
    options: DpOptions,
    progress: impl FnMut(u32, u32) -> bool,
) -> DpResult {
    run_generic::<F64>(model, options, progress, "f64")
}

fn run_generic<P: Prob>(
    model: &CompiledModel,
    options: DpOptions,
    mut progress: impl FnMut(u32, u32) -> bool,
    backend: &str,
) -> DpResult {
    let started = Instant::now();
    let converted: Vec<Vec<Vec<P>>> = model.prob_table.entries.iter().map(|by_trial| {
        by_trial.iter().map(|leaf_probs| leaf_probs.exact.iter().map(P::from_rational).collect()).collect()
    }).collect();
    let codec = StateCodec::new(&model.control_max, &model.state_count_max)
        .expect("compiler must reject state spaces that exceed u64");
    let initial = codec.encode(&model.control_init, &vec![0; model.state_leaves.len()])
        .expect("compiled initial state must fit its codec");
    let mut layer = HashMap::from([(initial, P::one())]);
    let mut hit_pmf = model.condition.as_ref().map(|_| vec![P::zero(); model.max_trials as usize + 1]);
    let mut pruned_mass = 0.0;
    let mut completed_trials = 0;
    let mut trial = 0u32;
    while trial < model.max_trials {
        let draw_trial = trial + 1;
        let consumed_trials = model.consumed_trials_after(draw_trial);
        let next_trial = draw_trial + consumed_trials;
        let mut next: HashMap<u64, P> = HashMap::new();
        for (state, mass) in layer {
            let ci = codec.control_index(state);
            let ti = if model.prob_table.trial_dependent { draw_trial as usize - 1 } else { 0 };
            let (base_control, base_counts) = codec.decode(state);
            let mut successor_control = base_control.clone();
            let mut successor_counts = base_counts.clone();
            for (leaf, p_leaf) in converted[ci][ti].iter().enumerate() {
                if p_leaf.is_zero() { continue; }
                let contribution = mass.mul(p_leaf);
                successor_control.copy_from_slice(&base_control);
                successor_counts.copy_from_slice(&base_counts);
                if let Some(position) = model.state_leaves.iter().position(|tracked| *tracked == leaf) {
                    successor_counts[position] += 1;
                }
                model.apply_transitions(&mut successor_control, leaf, draw_trial);
                if model.condition_matches_sparse(&successor_counts, draw_trial) {
                    if let Some(pmf) = &mut hit_pmf { pmf[draw_trial as usize].add_assign(&contribution); }
                    continue;
                }
                let mut grant_hit = None;
                let applied_consumed = model.apply_triggers_sparse(
                    &mut successor_control,
                    &mut successor_counts,
                    draw_trial,
                    |grant_counts, grant_trial| {
                        if grant_hit.is_none() && model.condition_matches_sparse(grant_counts, grant_trial) {
                            grant_hit = Some(grant_trial);
                        }
                    },
                );
                debug_assert_eq!(applied_consumed, consumed_trials);
                if let Some(hit_trial) = grant_hit {
                    if let Some(pmf) = &mut hit_pmf { pmf[hit_trial as usize].add_assign(&contribution); }
                    continue;
                }
                let successor = codec.encode(&successor_control, &successor_counts)
                    .expect("compiled successor state must fit its codec");
                next.entry(successor).or_insert_with(P::zero).add_assign(&contribution);
            }
        }
        if let Some(threshold) = options.prune_log10 {
            next.retain(|_, p| {
                let keep = p.magnitude_log10().map(|m| m >= threshold).unwrap_or(true);
                if !keep { pruned_mass += p.to_f64_lossy(); }
                keep
            });
        }
        layer = next;
        completed_trials = next_trial;
        trial = next_trial;
        if !progress(trial, model.max_trials) { break; }
    }
    let mut joint: HashMap<Vec<u32>, P> = HashMap::new();
    for (state, probability) in layer {
        let (_, state_counts) = codec.decode(state);
        let key = model.tracked_leaves.iter().map(|leaf| {
            model.state_leaves.iter().position(|state_leaf| state_leaf == leaf)
                .map(|position| state_counts[position]).unwrap_or(0)
        }).collect();
        joint.entry(key).or_insert_with(P::zero).add_assign(&probability);
    }
    let mut cells: Vec<_> = joint.into_iter().map(|(counts, probability)| DpCell {
        counts,
        probability: probability.to_f64_lossy(),
        display: probability.to_decimal_string(12),
    }).collect();
    cells.sort_by(|a, b| a.counts.cmp(&b.counts));
    let first_hit = hit_pmf.map(|values| summarize_first_hit(&values));
    DpResult {
        numeric: backend.into(),
        trials: completed_trials,
        tracked_leaf_ids: model.tracked_leaves.iter().map(|i| model.leaves[*i].id.clone()).collect(),
        joint: cells,
        first_hit,
        pruned_mass,
        elapsed_ms: started.elapsed().as_millis() as u64,
        clamp_events: model.prob_table.clamp_events,
    }
}

fn summarize_first_hit<P: Prob>(pmf: &[P]) -> FirstHitResult {
    let pmf: Vec<f64> = pmf.iter().map(|value| value.to_f64_lossy()).collect();
    let mut running = 0.0;
    let cdf: Vec<f64> = pmf.iter().map(|value| { running += value; running }).collect();
    let success = running;
    let weighted: f64 = pmf.iter().enumerate().map(|(trial, p)| trial as f64 * p).sum();
    let mean = (success > 0.0).then_some(weighted / success);
    let levels = [0.5, 0.75, 0.9, 0.95, 0.99];
    let percentiles = levels.into_iter().map(|level| {
        let target = success * level;
        let trial = cdf.iter().position(|value| *value >= target).unwrap_or(cdf.len().saturating_sub(1));
        (level, trial as u32)
    }).collect();
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
        })).unwrap()
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
            })).unwrap(),
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
            })).unwrap(),
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
            })).unwrap(),
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
            })).unwrap(),
        ];

        let mut blue_archive: ModelIr =
            serde_json::from_str(include_str!("../../../presets/blue-archive-pickup.json")).unwrap();
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
        result.joint.iter().map(|cell| (cell.counts.clone(), cell.probability)).collect()
    }

    #[test]
    fn binomial_mass_is_conserved() {
        let ir: ModelIr = serde_json::from_value(json!({
            "irVersion":1,"name":"coin","entities":[{"id":"hit","name":"hit","prob":{"lit":"1/2"}}],
            "stateVars":[],"probRules":[],"transitions":[],"triggers":[],
            "run":{"maxTrials":10,"trackJoint":["hit"],"numeric":"scaled"}
        })).unwrap();
        let model = compile(&ir).unwrap();
        let result = run_dp(&model, DpOptions { prune_log10: None }, |_,_| true).unwrap();
        let DpRunResult::Approximate(result) = result else {
            panic!("scaled backend must use approximate DP");
        };
        let total: f64 = result.joint.iter().map(|c| c.probability).sum();
        assert!((total - 1.0).abs() < 1e-12);
        assert!((result.joint[5].probability - 252.0 / 1024.0).abs() < 1e-12);
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
        })).unwrap();
        let model = compile(&ir).unwrap();
        let result = run_dp(&model, DpOptions { prune_log10: None }, |_,_| true).unwrap();
        let DpRunResult::Exact(result) = result else {
            panic!("exact backend must use BigInt DP");
        };

        assert_eq!(result.numeric, "exact");
        assert_eq!(result.denominator, "16");
        assert_eq!(result.clamp_events, 1);
        assert_eq!(
            result.joint.iter().map(|cell| cell.numerator.as_str()).collect::<Vec<_>>(),
            vec!["9", "6", "1"],
        );
    }

    #[test]
    fn monte_carlo_and_dp_cross_validate_for_ten_models() {
        const RUNS: u64 = 1_000_000;

        let models = cross_validation_models();
        assert_eq!(models.len(), 10, "§9.1 requires at least ten cross-validation models");

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
            let mc_cells: BTreeMap<_, _> = mc.joint.iter()
                .map(|cell| (cell.counts.clone(), cell.interval))
                .collect();
            let keys: BTreeSet<_> = dp_cells.keys().chain(mc_cells.keys()).cloned().collect();
            for counts in keys {
                let probability = dp_cells.get(&counts).copied().unwrap_or(0.0);
                let interval = mc_cells.get(&counts).copied()
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
        })).unwrap();
        let model = compile(&ir).unwrap();
        let exact = run_exact(&model, ExactOptions::default(), |_, _| true).unwrap();
        let scaled = run_generic::<ScaledF64>(
            &model,
            DpOptions { prune_log10: None },
            |_, _| true,
            "scaled",
        );

        assert_eq!(exact.tracked_leaf_ids, scaled.tracked_leaf_ids);
        let exact_cells: BTreeMap<_, _> = exact.joint.iter()
            .map(|cell| (cell.counts.clone(), cell.probability))
            .collect();
        let scaled_cells = probability_map(&scaled);
        assert_eq!(exact_cells.keys().collect::<Vec<_>>(), scaled_cells.keys().collect::<Vec<_>>());
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
        })).unwrap();
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
        }])).unwrap();

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
