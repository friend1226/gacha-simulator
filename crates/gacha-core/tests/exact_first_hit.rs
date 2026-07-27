use gacha_core::engine_dp::{DpOptions, DpRunResult};
use gacha_core::{
    compile, run_dp, run_exact, ExactOptions, ModelIr,
};
use serde_json::json;

fn first_hit_model(numeric: &str, probability: &str, max_trials: u32) -> ModelIr {
    serde_json::from_value(json!({
        "irVersion": 1,
        "name": "exact first hit",
        "entities": [{
            "id": "hit",
            "name": "hit",
            "prob": {"lit": probability}
        }],
        "stateVars": [],
        "probRules": [],
        "transitions": [],
        "triggers": [],
        "run": {
            "maxTrials": max_trials,
            "trackJoint": ["hit"],
            "numeric": numeric,
            "condition": {"ge": [{"var": "nHit"}, {"lit": "1"}]}
        }
    }))
    .expect("first-hit IR must deserialize")
}

#[test]
fn exact_first_hit_matches_closed_form_and_scaled_dp() {
    let exact_model = compile(&first_hit_model("exact", "1/2", 3))
        .expect("exact first-hit model must compile");
    let exact = run_exact(
        &exact_model,
        ExactOptions { reduce_layers: true, ..Default::default() },
        |_, _| true,
    )
    .expect("exact first-hit run");
    let first_hit = exact.first_hit.as_ref().expect("exact first-hit result");
    let expected = ["0", "4", "2", "1"];
    assert_eq!(exact.denominator, "8");
    assert_eq!(
        first_hit.pmf.iter().map(|value| value.numerator.as_str()).collect::<Vec<_>>(),
        expected,
    );
    assert!(first_hit.pmf.iter().all(|value| value.denominator == "8"));
    assert_eq!(first_hit.failure_reachable.numerator, "1");
    assert_eq!(first_hit.failure_reachable.denominator, "8");
    assert_eq!(exact.joint.len(), 1);
    assert_eq!(exact.joint[0].counts, vec![0]);
    assert_eq!(exact.joint[0].numerator, "1");

    let scaled_model = compile(&first_hit_model("scaled", "1/2", 3))
        .expect("scaled first-hit model must compile");
    let DpRunResult::Approximate(scaled) = run_dp(
        &scaled_model,
        DpOptions { prune_log10: None },
        |_, _| true,
    )
    .expect("scaled first-hit run")
    else {
        panic!("scaled model must use approximate DP");
    };
    let scaled_first_hit = scaled.first_hit.expect("scaled first-hit result");
    for (exact_value, scaled_value) in first_hit.pmf.iter().zip(scaled_first_hit.pmf) {
        assert!((exact_value.probability - scaled_value).abs() <= 1e-12);
    }
    assert!(
        (first_hit.failure_reachable.probability - scaled_first_hit.failure_reachable).abs()
            <= 1e-12
    );
}

#[test]
fn exact_first_hit_uses_consumed_grant_trial() {
    let mut ir = first_hit_model("exact", "0", 3);
    ir.triggers = serde_json::from_value(json!([{
        "at": {"trialCount": 1},
        "grant": {
            "leaf": "hit",
            "amount": 1,
            "consumesTrial": true,
            "appliesTransitions": false
        }
    }]))
    .expect("grant must deserialize");
    let model = compile(&ir).expect("grant first-hit model must compile");
    let exact = run_exact(&model, ExactOptions::default(), |_, _| true)
        .expect("grant first-hit exact run");
    let first_hit = exact.first_hit.expect("grant first-hit result");

    assert_eq!(first_hit.pmf[1].numerator, "0");
    assert_eq!(first_hit.pmf[2].numerator, first_hit.pmf[2].denominator);
    assert_eq!(first_hit.failure_reachable.numerator, "0");
    assert!(exact.joint.is_empty(), "all mass must be absorbed by the grant");
}
