use gacha_core::engine_dp::{DpOptions, DpResult, DpRunResult};
use gacha_core::engine_exact::ExactOptions;
use gacha_core::report::wilson;
use gacha_core::{
    compile, run_dp, run_exact, run_mc, McOptions, ModelIr,
};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{Signed, ToPrimitive, Zero};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};

const WILSON_95_Z: f64 = 1.959_963_984_540_054;

fn compile_nested_model(
    numeric: &str,
    max_trials: u32,
    triggers: Vec<Value>,
    parent_probability: &str,
    child_probability: &str,
) -> gacha_core::CompiledModel {
    let ir: ModelIr = serde_json::from_value(json!({
        "irVersion": 1,
        "name": "nested validation model",
        "entities": [{
            "id": "star3",
            "name": "3-star",
            "prob": { "lit": parent_probability },
            "children": [{
                "id": "pickup",
                "name": "pickup",
                "prob": { "lit": child_probability }
            }]
        }],
        "nestingPolicy": "clampChildren",
        "stateVars": [],
        "probRules": [],
        "transitions": [],
        "triggers": triggers,
        "run": {
            "maxTrials": max_trials,
            "trackJoint": ["star3"],
            "numeric": numeric
        }
    }))
    .expect("test IR must deserialize");

    compile(&ir).expect("test IR must compile")
}

fn run_scaled(model: &gacha_core::CompiledModel) -> DpResult {
    let result = run_dp(
        model,
        DpOptions { prune_log10: None },
        |_, _| true,
    )
    .expect("scaled DP must run");

    match result {
        DpRunResult::Approximate(result) => result,
        DpRunResult::Exact(_) => panic!("scaled model unexpectedly used exact DP"),
    }
}

#[test]
fn monte_carlo_and_dp_agree_within_wilson_intervals() {
    let model = compile_nested_model("scaled", 12, Vec::new(), "0.03", "0.007");
    let dp = run_scaled(&model);
    let mc = run_mc(
        &model,
        McOptions {
            runs: 1_000_000,
            seed: 42,
            confidence_z: WILSON_95_Z,
            batch_size: 50_000,
        },
        |_, _| true,
    );

    assert_eq!(mc.runs, 1_000_000);
    assert_eq!(mc.seed, 42);
    assert_eq!(mc.tracked_leaf_ids, dp.tracked_leaf_ids);

    let mc_cells: HashMap<_, _> = mc
        .joint
        .iter()
        .map(|cell| (cell.counts.clone(), cell))
        .collect();
    let zero_occurrence = wilson(0, mc.runs, WILSON_95_Z);
    let mut outside = Vec::new();

    for cell in &dp.joint {
        let interval = mc_cells
            .get(&cell.counts)
            .map(|mc_cell| mc_cell.interval)
            .unwrap_or(zero_occurrence);
        if cell.probability < interval.lower || cell.probability > interval.upper {
            outside.push((
                cell.counts.clone(),
                cell.probability,
                interval.lower,
                interval.upper,
            ));
        }
    }

    assert!(
        dp.joint.len() >= 50,
        "cross-validation model should exercise a non-trivial joint distribution"
    );
    assert!(
        outside.len() * 20 < dp.joint.len(),
        "{} of {} DP cells ({:.2}%) fell outside MC Wilson intervals: {:?}",
        outside.len(),
        dp.joint.len(),
        outside.len() as f64 * 100.0 / dp.joint.len() as f64,
        outside,
    );
}

#[test]
fn exact_and_scaled_dp_agree_to_ten_decimal_places_including_extreme_tail() {
    let scaled_ir: ModelIr = serde_json::from_value(json!({
        "irVersion": 1,
        "name": "extreme tail scaled",
        "entities": [{
            "id": "hit",
            "name": "hit",
            "prob": { "lit": "0.007" }
        }],
        "stateVars": [],
        "probRules": [],
        "transitions": [],
        "triggers": [],
        "run": {
            "maxTrials": 200,
            "trackJoint": ["hit"],
            "numeric": "scaled"
        }
    }))
    .expect("scaled IR must deserialize");
    let mut exact_ir = scaled_ir.clone();
    exact_ir.name = "extreme tail exact".into();
    exact_ir.run.numeric = gacha_core::ir::NumericBackend::Exact;

    let scaled = run_scaled(&compile(&scaled_ir).expect("scaled IR must compile"));
    let exact_model = compile(&exact_ir).expect("exact IR must compile");
    let exact = run_exact(
        &exact_model,
        ExactOptions::default(),
        |_, _| true,
    )
    .expect("exact DP must run");

    assert_eq!(scaled.joint.len(), exact.joint.len());
    let scaled_cells: HashMap<_, _> = scaled
        .joint
        .iter()
        .map(|cell| (cell.counts.clone(), cell))
        .collect();
    let mut max_relative_error = 0.0_f64;

    for exact_cell in &exact.joint {
        let scaled_cell = scaled_cells
            .get(&exact_cell.counts)
            .expect("scaled DP must contain every exact cell");
        let numerator = exact_cell
            .numerator
            .parse::<BigInt>()
            .expect("exact numerator must be an integer");
        let denominator = exact_cell
            .denominator
            .parse::<BigInt>()
            .expect("exact denominator must be an integer");
        let exact_probability = BigRational::new(numerator, denominator);
        let scaled_probability = gacha_core::rational::parse_literal(&scaled_cell.display)
            .expect("scaled display must remain an exact decimal literal");

        let relative_error = if exact_probability.is_zero() {
            assert!(scaled_probability.is_zero());
            0.0
        } else {
            ((scaled_probability - &exact_probability) / exact_probability)
                .abs()
                .to_f64()
                .expect("relative error must fit in f64")
        };
        max_relative_error = max_relative_error.max(relative_error);
    }

    assert!(
        max_relative_error <= 1e-10,
        "maximum exact/scaled relative error was {max_relative_error:e}"
    );

    let extreme = scaled_cells
        .get(&vec![200])
        .expect("all-hit extreme-tail cell must exist");
    assert_ne!(extreme.display, "0");
    assert!(
        extreme.display.contains("e-"),
        "extreme probability should use scientific notation: {}",
        extreme.display
    );
}

fn marginal_star3(result: &DpResult) -> BTreeMap<u32, f64> {
    let mut marginal = BTreeMap::new();
    for cell in &result.joint {
        let star3_count = cell.counts.iter().sum();
        *marginal.entry(star3_count).or_insert(0.0) += cell.probability;
    }
    marginal
}

#[test]
fn pickup_grant_shifts_derived_parent_distribution_by_exactly_one() {
    let without_grant = run_scaled(&compile_nested_model(
        "scaled",
        200,
        Vec::new(),
        "1/2",
        "1/4",
    ));
    let with_grant = run_scaled(&compile_nested_model(
        "scaled",
        200,
        vec![json!({
            "at": { "trialCount": 200 },
            "grant": {
                "leaf": "pickup",
                "amount": 1,
                "consumesTrial": false,
                "appliesTransitions": false
            }
        })],
        "1/2",
        "1/4",
    ));

    assert_eq!(
        without_grant.tracked_leaf_ids,
        vec!["pickup".to_owned(), "star3__self".to_owned()]
    );
    assert_eq!(with_grant.tracked_leaf_ids, without_grant.tracked_leaf_ids);

    let base = marginal_star3(&without_grant);
    let granted = marginal_star3(&with_grant);
    assert_eq!(base.len(), 201);
    assert_eq!(granted.len(), 201);
    assert_eq!(base.keys().next(), Some(&0));
    assert_eq!(granted.keys().next(), Some(&1));
    assert_eq!(base.keys().next_back(), Some(&200));
    assert_eq!(granted.keys().next_back(), Some(&201));

    for (count, probability) in base {
        let shifted = granted
            .get(&(count + 1))
            .expect("grant distribution must contain the +1 shifted count");
        assert!(
            (shifted - probability).abs() <= 1e-12,
            "nStar3={count} probability {probability:e} did not shift to nStar3={} ({shifted:e})",
            count + 1,
        );
    }
}
