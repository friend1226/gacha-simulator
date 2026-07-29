use gacha_core::{
    compile, run_dp, run_exact, run_mc, DpOptions, DpRunResult, ExactOptions, McOptions, ModelIr,
};
use serde_json::json;

fn large_accumulator_table_model(maximum: u32, max_trials: u32) -> ModelIr {
    serde_json::from_value(json!({
        "irVersion": 2,
        "name": "accumulator table preflight",
        "entities": [{"id": "hit", "name": "hit", "prob": {"lit": "1/2"}}],
        "stateVars": [{
            "id": "score",
            "init": 0,
            "max": maximum,
            "role": "accumulator",
            "update": [
                {
                    "when": {"leafOf": "hit"},
                    "set": {"add": [{"var": "score"}, {"trial": true}]}
                },
                {
                    "when": {"not": {"leafOf": "hit"}},
                    "set": {"add": [{"var": "score"}, {"trial": true}]}
                }
            ]
        }],
        "probRules": [],
        "transitions": [],
        "triggers": [],
        "run": {
            "maxTrials": max_trials,
            "trackJoint": ["score"],
            "numeric": "scaled",
            "trialSeries": "none"
        }
    }))
    .unwrap()
}

fn accumulator_model(numeric: &str) -> ModelIr {
    serde_json::from_value(json!({
        "irVersion": 2,
        "name": "accumulator saturation",
        "entities": [{"id": "hit", "name": "hit", "prob": {"lit": "1/2"}}],
        "stateVars": [{
            "id": "spent",
            "name": "spent currency",
            "init": 0,
            "max": 3,
            "role": "accumulator",
            "clampPolicy": "saturate",
            "update": [
                {
                    "when": {"leafOf": "hit"},
                    "set": {"add": [{"var": "spent"}, {"lit": "2"}]}
                },
                {
                    "when": {"not": {"leafOf": "hit"}},
                    "set": {"add": [{"var": "spent"}, {"lit": "2"}]}
                }
            ]
        }],
        "probRules": [],
        "transitions": [],
        "triggers": [],
        "run": {
            "maxTrials": 2,
            "trackJoint": ["hit", "spent"],
            "numeric": numeric,
            "trialSeries": "marginal"
        }
    }))
    .unwrap()
}

#[test]
fn accumulator_axis_matches_across_dp_exact_and_mc_and_reports_clamps() {
    let scaled_model = compile(&accumulator_model("scaled")).unwrap();
    let DpRunResult::Approximate(dp) =
        run_dp(&scaled_model, DpOptions { prune_log10: None }, |_, _| true).unwrap()
    else {
        panic!("scaled model must use approximate DP");
    };
    assert_eq!(dp.tracked_leaf_ids, ["hit", "spent"]);
    assert!(dp.joint.iter().all(|cell| cell.counts[1] == 3));
    assert_eq!(dp.accumulator_clamp_events, 4);

    let exact_model = compile(&accumulator_model("exact")).unwrap();
    let exact = run_exact(&exact_model, ExactOptions::default(), |_, _| true).unwrap();
    assert_eq!(exact.accumulator_clamp_events, dp.accumulator_clamp_events);
    assert_eq!(
        exact
            .joint
            .iter()
            .map(|cell| (cell.counts.clone(), cell.numerator.clone()))
            .collect::<Vec<_>>(),
        vec![
            (vec![0, 3], "1".to_owned()),
            (vec![1, 3], "2".to_owned()),
            (vec![2, 3], "1".to_owned()),
        ],
    );

    let mc = run_mc(
        &scaled_model,
        McOptions {
            runs: 20_000,
            seed: 17,
            ..Default::default()
        },
        |_, _| true,
    );
    assert_eq!(mc.seed, 17);
    assert_eq!(mc.accumulator_clamp_events, mc.runs);
    assert!(mc.joint.iter().all(|cell| cell.counts[1] == 3));
    for cell in &dp.joint {
        let interval = mc
            .joint
            .iter()
            .find(|candidate| candidate.counts == cell.counts)
            .unwrap()
            .interval;
        assert!(interval.lower <= cell.probability && cell.probability <= interval.upper);
    }
}

#[test]
fn redundant_leaf_counter_is_derived_without_an_accumulator_state_axis() {
    let ir: ModelIr = serde_json::from_value(json!({
        "irVersion": 2,
        "name": "derived counter",
        "entities": [{"id": "hit", "name": "hit", "prob": {"lit": "1/2"}}],
        "stateVars": [{
            "id": "hitAgain",
            "init": 0,
            "max": 100,
            "role": "accumulator",
            "update": [{
                "when": {"leafOf": "hit"},
                "set": {"add": [{"var": "hitAgain"}, {"lit": "1"}]}
            }]
        }],
        "probRules": [],
        "transitions": [],
        "triggers": [],
        "run": {
            "maxTrials": 2,
            "trackJoint": ["hitAgain"],
            "numeric": "scaled",
            "trialSeries": "none"
        }
    }))
    .unwrap();
    let model = compile(&ir).unwrap();
    assert!(model.accumulator_max.is_empty());
    assert_eq!(model.analysis.stat_states, 3);
    assert!(model
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "W008"));
    let DpRunResult::Approximate(result) =
        run_dp(&model, DpOptions { prune_log10: None }, |_, _| true).unwrap()
    else {
        panic!("scaled model must use approximate DP");
    };
    assert_eq!(
        result
            .joint
            .iter()
            .map(|cell| (&cell.counts, cell.probability))
            .collect::<Vec<_>>(),
        vec![(&vec![0], 0.25), (&vec![1], 0.5), (&vec![2], 0.25)],
    );
}

#[test]
fn marginal_series_preserves_mass_and_last_checkpoint_matches_joint() {
    let model = compile(&accumulator_model("scaled")).unwrap();
    let DpRunResult::Approximate(result) =
        run_dp(&model, DpOptions { prune_log10: None }, |_, _| true).unwrap()
    else {
        panic!("scaled model must use approximate DP");
    };
    assert_eq!(result.trial_series.marginal.len(), 2);
    for point in &result.trial_series.marginal {
        for axis in &point.axes {
            let total: f64 = axis.cells.iter().map(|cell| cell.probability).sum();
            assert!(
                (total - 1.0).abs() <= 1e-12,
                "trial {} axis {}",
                point.trial,
                axis.id
            );
        }
    }

    let mut ir = accumulator_model("scaled");
    ir.run.trial_series = Some(gacha_core::ir::TrialSeriesMode::Checkpoints);
    ir.run.series_checkpoints = vec![1, 2];
    let checkpoint_model = compile(&ir).unwrap();
    let DpRunResult::Approximate(checkpoints) = run_dp(
        &checkpoint_model,
        DpOptions { prune_log10: None },
        |_, _| true,
    )
    .unwrap() else {
        panic!("scaled model must use approximate DP");
    };
    let checkpoint = &checkpoints.trial_series.checkpoints.last().unwrap().joint;
    assert_eq!(
        checkpoint
            .iter()
            .map(|cell| (&cell.counts, &cell.display))
            .collect::<Vec<_>>(),
        checkpoints
            .joint
            .iter()
            .map(|cell| (&cell.counts, &cell.display))
            .collect::<Vec<_>>(),
    );
}

#[test]
fn control_dependent_accumulator_prevents_control_state_canonicalization() {
    let ir: ModelIr = serde_json::from_value(json!({
        "irVersion": 2,
        "name": "control dependent accumulator",
        "entities": [{"id": "hit", "name": "hit", "prob": {"lit": "1/2"}}],
        "stateVars": [
            {"id": "pity", "init": 0, "max": 2, "role": "control"},
            {
                "id": "seenPity",
                "init": 0,
                "max": 2,
                "role": "accumulator",
                "update": [
                    {"when": {"leafOf": "hit"}, "set": {"var": "pity"}},
                    {"when": {"not": {"leafOf": "hit"}}, "set": {"var": "pity"}}
                ]
            }
        ],
        "probRules": [],
        "transitions": [
            {
                "when": {"leafOf": "hit"},
                "set": {"pity": {"add": [{"var": "pity"}, {"lit": "1"}]}}
            },
            {
                "when": {"not": {"leafOf": "hit"}},
                "set": {"pity": {"add": [{"var": "pity"}, {"lit": "1"}]}}
            }
        ],
        "triggers": [],
        "run": {
            "maxTrials": 2,
            "trackJoint": ["seenPity"],
            "numeric": "scaled",
            "trialSeries": "none"
        }
    }))
    .unwrap();
    let model = compile(&ir).unwrap();
    assert!(!model.prob_table.control_invariant);
    let DpRunResult::Approximate(result) =
        run_dp(&model, DpOptions { prune_log10: None }, |_, _| true).unwrap()
    else {
        panic!("scaled model must use approximate DP");
    };
    assert_eq!(result.joint.len(), 1);
    assert_eq!(result.joint[0].counts, [2]);
    assert!((result.joint[0].probability - 1.0).abs() <= 1e-12);
}

#[test]
fn mc_marginal_series_matches_dp_absorption_with_wilson_intervals() {
    let ir: ModelIr = serde_json::from_value(json!({
        "irVersion": 2,
        "name": "absorbed series",
        "entities": [{"id": "hit", "name": "hit", "prob": {"lit": "1/2"}}],
        "stateVars": [],
        "probRules": [],
        "transitions": [],
        "triggers": [],
        "run": {
            "maxTrials": 3,
            "trackJoint": ["hit"],
            "numeric": "scaled",
            "condition": {"ge": [{"var": "nHit"}, {"lit": "1"}]},
            "trialSeries": "marginal"
        }
    }))
    .unwrap();
    let model = compile(&ir).unwrap();
    let DpRunResult::Approximate(dp) =
        run_dp(&model, DpOptions { prune_log10: None }, |_, _| true).unwrap()
    else {
        panic!("scaled model must use approximate DP");
    };
    let mc = run_mc(
        &model,
        McOptions {
            runs: 100_000,
            seed: 1234,
            ..Default::default()
        },
        |_, _| true,
    );
    assert_eq!(mc.runs, 100_000);
    assert!(mc.joint.iter().map(|cell| cell.occurrences).sum::<u64>() < mc.runs);
    for (dp_point, mc_point) in dp
        .trial_series
        .marginal
        .iter()
        .zip(&mc.trial_series.marginal)
    {
        assert_eq!(dp_point.trial, mc_point.trial);
        let expected = dp_point.axes[0].cells[0].probability;
        let interval = mc_point.axes[0].cells[0].interval;
        assert!(
            interval.lower <= expected && expected <= interval.upper,
            "trial {} DP={expected} Wilson=[{}, {}]",
            dp_point.trial,
            interval.lower,
            interval.upper,
        );
    }
}

#[test]
fn accumulator_table_preflight_warns_and_rejects_before_unsafe_expansion() {
    for (maximum, max_trials, expected_entries) in
        [(2_000, 200, 800_400u64), (20_000, 200, 8_000_400u64)]
    {
        let model = compile(&large_accumulator_table_model(maximum, max_trials))
            .expect("tables below the hard limit must still compile");
        let warning = model
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "W009")
            .expect("large safe tables must warn");
        assert!(warning.message.contains(&expected_entries.to_string()));
        assert!(warning.message.contains("trials=200"));
        assert!(warning
            .message
            .contains(&format!("current=max+1={}", maximum + 1)));
    }

    let error = compile(&large_accumulator_table_model(60_000, 500))
        .expect_err("60M entries must be rejected before allocation");
    let diagnostic = error
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E010")
        .expect("the hard table guard must have a dedicated error");
    assert!(diagnostic.message.contains("60001000"));
    assert!(diagnostic.message.contains("control=1"));
    assert!(diagnostic.message.contains("trials=500"));
    assert!(diagnostic.message.contains("current=max+1=60001"));
}
