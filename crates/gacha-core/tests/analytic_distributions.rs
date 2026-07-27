use gacha_core::engine_dp::{DpOptions, DpRunResult, FirstHitResult};
use gacha_core::{compile, run_dp, ModelIr};
use serde_json::json;

fn first_hit_result(successes: u32, max_trials: u32, probability: &str) -> FirstHitResult {
    let ir: ModelIr = serde_json::from_value(json!({
        "irVersion": 1,
        "name": format!("{successes}-success first hit"),
        "entities": [{"id": "hit", "name": "hit", "prob": {"lit": probability}}],
        "stateVars": [],
        "probRules": [],
        "transitions": [],
        "triggers": [],
        "run": {
            "maxTrials": max_trials,
            "trackJoint": ["hit"],
            "numeric": "scaled",
            "condition": {
                "ge": [{"var": "nHit"}, {"lit": successes.to_string()}]
            }
        }
    }))
    .expect("analytic distribution IR must deserialize");
    let model = compile(&ir).expect("analytic distribution IR must compile");
    let result =
        run_dp(&model, DpOptions { prune_log10: None }, |_, _| true).expect("scaled DP must run");
    let DpRunResult::Approximate(result) = result else {
        panic!("scaled model unexpectedly used exact DP");
    };
    result
        .first_hit
        .expect("condition must produce first-hit output")
}

fn choose(n: u32, k: u32) -> f64 {
    if k > n {
        return 0.0;
    }
    let k = k.min(n - k);
    (0..k).fold(1.0, |value, i| value * (n - i) as f64 / (i + 1) as f64)
}

#[test]
fn first_success_matches_geometric_distribution() {
    let p = 0.25_f64;
    let max_trials = 12u32;
    let result = first_hit_result(1, max_trials, "1/4");

    assert_eq!(result.pmf.len(), max_trials as usize + 1);
    assert_eq!(result.pmf[0], 0.0);
    for trial in 1..=max_trials {
        let expected = (1.0 - p).powi(trial as i32 - 1) * p;
        assert!(
            (result.pmf[trial as usize] - expected).abs() <= 1e-12,
            "trial {trial}: DP={} geometric={expected}",
            result.pmf[trial as usize],
        );
    }
    let expected_failure = (1.0 - p).powi(max_trials as i32);
    assert!((result.failure_reachable - expected_failure).abs() <= 1e-12);
}

#[test]
fn kth_success_matches_negative_binomial_distribution() {
    let p = 0.3_f64;
    let successes = 3u32;
    let max_trials = 15u32;
    let result = first_hit_result(successes, max_trials, "3/10");

    let mut success_mass = 0.0;
    for trial in 1..=max_trials {
        let expected = if trial < successes {
            0.0
        } else {
            choose(trial - 1, successes - 1)
                * p.powi(successes as i32)
                * (1.0 - p).powi((trial - successes) as i32)
        };
        success_mass += expected;
        assert!(
            (result.pmf[trial as usize] - expected).abs() <= 1e-12,
            "trial {trial}: DP={} negative-binomial={expected}",
            result.pmf[trial as usize],
        );
    }
    assert!((result.failure_reachable - (1.0 - success_mass)).abs() <= 1e-12);
}
