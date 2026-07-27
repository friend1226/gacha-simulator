use crate::compile::CompiledModel;
use crate::ir::NumericBackend;
use crate::numeric::{F64, Prob, ScaledF64};
use serde::Serialize;
use std::collections::HashMap;
use std::hash::Hash;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct DpOptions {
    pub prune_log10: Option<f64>,
}

impl Default for DpOptions {
    fn default() -> Self { Self { prune_log10: Some(-18.0) } }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct State {
    control: Vec<u32>,
    counts: Vec<u32>,
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
) -> DpResult {
    match model.numeric {
        NumericBackend::F64 => run_generic::<F64>(model, options, progress, "f64"),
        NumericBackend::Scaled | NumericBackend::Exact =>
            run_generic::<ScaledF64>(model, options, progress, "scaled"),
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
    let initial = State {
        control: model.control_init.clone(),
        counts: vec![0; model.state_leaves.len()],
    };
    let mut layer = HashMap::from([(initial, P::one())]);
    let mut hit_pmf = model.condition.as_ref().map(|_| vec![P::zero(); model.max_trials as usize + 1]);
    let mut pruned_mass = 0.0;
    let mut completed_trials = 0;
    for trial in 1..=model.max_trials {
        let mut next: HashMap<State, P> = HashMap::new();
        for (state, mass) in layer {
            let ci = model.control_index(&state.control);
            let ti = if model.prob_table.trial_dependent { trial as usize - 1 } else { 0 };
            for (leaf, p_leaf) in converted[ci][ti].iter().enumerate() {
                if p_leaf.is_zero() { continue; }
                let contribution = mass.mul(p_leaf);
                let mut successor = state.clone();
                if let Some(position) = model.state_leaves.iter().position(|tracked| *tracked == leaf) {
                    successor.counts[position] += 1;
                }
                model.apply_transitions(&mut successor.control, leaf, trial);
                model.apply_triggers_sparse(&mut successor.control, &mut successor.counts, trial);
                if condition_matches_sparse(model, &successor.counts, trial) {
                    if let Some(pmf) = &mut hit_pmf { pmf[trial as usize].add_assign(&contribution); }
                    continue;
                }
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
        completed_trials = trial;
        if !progress(trial, model.max_trials) { break; }
    }
    let mut joint: HashMap<Vec<u32>, P> = HashMap::new();
    for (state, probability) in layer {
        let key = model.tracked_leaves.iter().map(|leaf| {
            model.state_leaves.iter().position(|state_leaf| state_leaf == leaf)
                .map(|position| state.counts[position]).unwrap_or(0)
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

fn condition_matches_sparse(model: &CompiledModel, counts: &[u32], trial: u32) -> bool {
    let Some(program) = &model.condition else { return false; };
    crate::expr::eval(program, |name| {
        let entity = name.strip_prefix('n').map(lower_first).unwrap_or_else(|| name.to_owned());
        model.entity_count_sparse(counts, &entity)
            .map(|value| crate::rational::Rational::from_integer(value.into()))
    }, trial).and_then(|value| value.boolean()).unwrap_or(false)
}

fn lower_first(value: &str) -> String {
    let mut chars = value.chars();
    chars.next().map(|c| c.to_lowercase().collect::<String>() + chars.as_str()).unwrap_or_default()
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
    use crate::{compile, ir::ModelIr};
    use serde_json::json;

    #[test]
    fn binomial_mass_is_conserved() {
        let ir: ModelIr = serde_json::from_value(json!({
            "irVersion":1,"name":"coin","entities":[{"id":"hit","name":"hit","prob":{"lit":"1/2"}}],
            "stateVars":[],"probRules":[],"transitions":[],"triggers":[],
            "run":{"maxTrials":10,"trackJoint":["hit"],"numeric":"scaled"}
        })).unwrap();
        let model = compile(&ir).unwrap();
        let result = run_dp(&model, DpOptions { prune_log10: None }, |_,_| true);
        let total: f64 = result.joint.iter().map(|c| c.probability).sum();
        assert!((total - 1.0).abs() < 1e-12);
        assert!((result.joint[5].probability - 252.0 / 1024.0).abs() < 1e-12);
    }
}
