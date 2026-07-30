use gacha_core::engine_dp::{DpOptions, DpResult, DpRunResult};
use gacha_core::{compile, run_dp, run_exact, run_mc, ExactOptions, McOptions, ModelIr};
use serde_json::json;

fn grant_model(
    numeric: &str,
    consumes_trial: bool,
    applies_transitions: bool,
    trigger_trial: u32,
    condition: bool,
) -> gacha_core::CompiledModel {
    let condition = condition.then(|| {
        json!({
            "ge": [{"var": "nGift"}, {"lit": "1"}]
        })
    });
    let ir: ModelIr = serde_json::from_value(json!({
        "irVersion": 1,
        "name": format!(
            "grant semantics consumes={consumes_trial} transitions={applies_transitions}"
        ),
        "entities": [
            {"id": "draw", "name": "draw", "prob": {"lit": "1"}},
            {"id": "gift", "name": "gift", "prob": {"lit": "0"}}
        ],
        "stateVars": [{"id": "enabled", "init": 1, "max": 1, "role": "control"}],
        "probRules": [{
            "target": "draw",
            "expr": {
                "if": {"eq": [{"var": "enabled"}, {"lit": "1"}]},
                "then": {"lit": "1"},
                "else": {"lit": "0"}
            }
        }],
        "transitions": [{
            "when": {"leafIs": "gift"},
            "set": {"enabled": {"lit": "0"}}
        }],
        "triggers": [{
            "at": {"trialCount": trigger_trial},
            "grant": {
                "leaf": "gift",
                "amount": 1,
                "consumesTrial": consumes_trial,
                "appliesTransitions": applies_transitions
            }
        }],
        "run": {
            "maxTrials": 3,
            "trackJoint": ["draw", "gift", "__other__"],
            "numeric": numeric,
            "condition": condition
        }
    }))
    .expect("grant semantics IR must deserialize");
    compile(&ir).expect("grant semantics IR must compile")
}

fn run_scaled(model: &gacha_core::CompiledModel) -> DpResult {
    match run_dp(
        model,
        DpOptions {
            prune_log10: None,
            ..Default::default()
        },
        |_, _| true,
    )
    .expect("scaled DP must run")
    {
        DpRunResult::Approximate(result) => result,
        DpRunResult::Exact(_) => panic!("scaled model unexpectedly used exact DP"),
    }
}

#[test]
fn grant_semantics_four_combinations_match_mc_dp_and_exact() {
    let cases = [
        (false, false, vec![3, 1, 0]),
        (false, true, vec![1, 1, 2]),
        (true, false, vec![2, 1, 0]),
        (true, true, vec![1, 1, 1]),
    ];

    for (case, (consumes_trial, applies_transitions, expected_counts)) in
        cases.into_iter().enumerate()
    {
        let scaled_model = grant_model("scaled", consumes_trial, applies_transitions, 1, false);
        let dp = run_scaled(&scaled_model);
        assert_eq!(dp.trials, 3);
        assert_eq!(dp.joint.len(), 1);
        assert_eq!(dp.joint[0].counts, expected_counts);
        assert_eq!(dp.joint[0].probability, 1.0);
        assert_eq!(
            dp.joint[0].counts.iter().sum::<u32>(),
            if consumes_trial { 3 } else { 4 },
            "a consuming grant must replace one normal draw in the trial budget",
        );

        let mc = run_mc(
            &scaled_model,
            McOptions {
                runs: 10_000,
                seed: 0x6a17_u64 + case as u64,
                confidence_z: 1.959963984540054,
                batch_size: 1_000,
            },
            |_, _| true,
        );
        assert_eq!(mc.tracked_leaf_ids, dp.tracked_leaf_ids);
        assert_eq!(mc.joint.len(), 1);
        assert_eq!(mc.joint[0].counts, expected_counts);
        assert_eq!(mc.joint[0].occurrences, mc.runs);
        assert!(
            mc.joint[0].interval.lower <= dp.joint[0].probability
                && dp.joint[0].probability <= mc.joint[0].interval.upper
        );

        let exact_model = grant_model("exact", consumes_trial, applies_transitions, 1, false);
        let exact = run_exact(&exact_model, ExactOptions::default(), |_, _| true)
            .expect("exact DP must run");
        assert_eq!(exact.trials, 3);
        assert_eq!(exact.joint.len(), 1);
        assert_eq!(exact.joint[0].counts, expected_counts);
        assert_eq!(exact.joint[0].numerator, exact.joint[0].denominator);
    }
}

#[test]
fn consuming_grant_records_first_hit_on_its_consumed_trial() {
    let model = grant_model("scaled", true, false, 1, true);
    let dp = run_scaled(&model);
    let first_hit = dp
        .first_hit
        .expect("condition must produce first-hit output");
    assert_eq!(first_hit.pmf[1], 0.0);
    assert_eq!(first_hit.pmf[2], 1.0);

    let mc = run_mc(
        &model,
        McOptions {
            runs: 1_000,
            seed: 42,
            confidence_z: 1.959963984540054,
            batch_size: 100,
        },
        |_, _| true,
    );
    let first_hit = mc
        .first_hit
        .expect("condition must produce MC first-hit output");
    assert_eq!(first_hit[1], 0);
    assert_eq!(first_hit[2], mc.runs);
}

#[test]
fn consuming_grant_is_not_applied_without_remaining_trial_budget() {
    let model = grant_model("scaled", true, true, 3, false);
    assert!(
        model
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "W007"),
        "compiler must report the statically known dropped grant",
    );
    let dp = run_scaled(&model);
    assert_eq!(dp.trials, 3);
    assert_eq!(dp.joint.len(), 1);
    assert_eq!(dp.joint[0].counts, vec![3, 0, 0]);
}

#[test]
fn compiler_warns_for_each_consuming_grant_beyond_the_remaining_budget() {
    let ir: ModelIr = serde_json::from_value(json!({
        "irVersion": 1,
        "name": "multiple consuming grants",
        "entities": [{"id": "gift", "name": "gift", "prob": {"lit": "1"}}],
        "stateVars": [],
        "probRules": [],
        "transitions": [],
        "triggers": [
            {
                "blockId": "fits",
                "at": {"trialCount": 1},
                "grant": {"leaf": "gift", "consumesTrial": true}
            },
            {
                "blockId": "overflow-1",
                "at": {"trialCount": 1},
                "grant": {"leaf": "gift", "consumesTrial": true}
            },
            {
                "blockId": "overflow-2",
                "at": {"trialCount": 1},
                "grant": {"leaf": "gift", "consumesTrial": true}
            }
        ],
        "run": {
            "maxTrials": 2,
            "trackJoint": ["gift"],
            "numeric": "scaled"
        }
    }))
    .expect("warning regression IR must deserialize");
    let model = compile(&ir).expect("warning regression IR must compile");
    let warnings: Vec<_> = model
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "W007")
        .collect();

    assert_eq!(warnings.len(), 2);
    assert_eq!(warnings[0].block_id.as_deref(), Some("overflow-1"));
    assert_eq!(warnings[1].block_id.as_deref(), Some("overflow-2"));
}
