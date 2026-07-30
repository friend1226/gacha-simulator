use gacha_core::{compile, run_dp, DpOptions, DpRunResult, ModelIr, Severity};
use serde_json::{json, Value};

fn base_model(probability: Value, state_vars: Vec<Value>) -> ModelIr {
    serde_json::from_value(json!({
        "irVersion": 1,
        "name": "core diagnostic model",
        "entities": [{
            "id": "hit",
            "name": "hit",
            "prob": probability,
            "blockId": "hit-probability"
        }],
        "stateVars": state_vars,
        "probRules": [],
        "transitions": [],
        "triggers": [],
        "run": {
            "maxTrials": 5,
            "trackJoint": ["hit"],
            "numeric": "scaled"
        }
    }))
    .expect("diagnostic test IR must deserialize")
}

#[test]
fn e002_rejects_negative_literal_and_preserves_block_id() {
    let ir = base_model(json!({"lit": "-0.01"}), Vec::new());
    let error = compile(&ir).expect_err("negative probability must fail compilation");
    let diagnostic = error
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E002")
        .expect("E002 diagnostic");

    assert_eq!(diagnostic.severity, Severity::Error);
    assert_eq!(diagnostic.block_id.as_deref(), Some("hit-probability"));
}

#[test]
fn e005_rejects_probability_rule_for_unknown_entity() {
    let mut ir = base_model(json!({"lit": "0.5"}), Vec::new());
    ir.prob_rules = serde_json::from_value(json!([{
        "target": "missing",
        "expr": {"lit": "0.25"},
        "blockId": "missing-rule"
    }]))
    .expect("probability rule must deserialize");

    let error = compile(&ir).expect_err("unknown probability rule target must fail compilation");
    let diagnostic = error
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E005")
        .expect("E005 diagnostic");

    assert_eq!(diagnostic.severity, Severity::Error);
    assert_eq!(diagnostic.block_id.as_deref(), Some("missing-rule"));
    assert!(diagnostic.message.contains("missing"));
}

#[test]
fn e002_uses_control_bounds_without_false_positive_for_correlated_terms() {
    let state_vars = vec![json!({
        "id": "pity",
        "init": 0,
        "max": 5,
        "role": "control"
    })];
    let negative = base_model(
        json!({"sub": [{"var": "pity"}, {"lit": "2"}]}),
        state_vars.clone(),
    );
    let error = compile(&negative).expect_err("bounded affine expression can be negative");
    assert!(error
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "E002"));

    let correlated = base_model(
        json!({"sub": [{"var": "pity"}, {"var": "pity"}]}),
        state_vars,
    );
    let model = compile(&correlated).expect("identical correlated terms cancel to zero");
    assert!(!model
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "E002"));

    let conditional = base_model(
        json!({
            "if": {"eq": [{"var": "pity"}, {"lit": "5"}]},
            "then": {"lit": "-1/10"},
            "else": {"lit": "1/10"}
        }),
        vec![json!({
            "id": "pity",
            "init": 0,
            "max": 5,
            "role": "control"
        })],
    );
    let error =
        compile(&conditional).expect_err("reachable negative conditional branch must be rejected");
    assert!(error
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "E002"));
}

fn condition_model(condition: Value, pickup_probability: &str, triggers: Vec<Value>) -> ModelIr {
    serde_json::from_value(json!({
        "irVersion": 1,
        "name": "condition diagnostic model",
        "entities": [{
            "id": "star3",
            "name": "star3",
            "prob": {"lit": "0.03"},
            "children": [{
                "id": "pickup",
                "name": "pickup",
                "prob": {"lit": pickup_probability}
            }]
        }],
        "stateVars": [],
        "probRules": [],
        "transitions": [],
        "triggers": triggers,
        "run": {
            "maxTrials": 5,
            "trackJoint": ["star3"],
            "numeric": "scaled",
            "condition": condition
        }
    }))
    .expect("condition diagnostic IR must deserialize")
}

#[test]
fn w003_detects_inclusion_range_and_zero_probability_contradictions() {
    let impossible_conditions = [
        json!({"gt": [{"var": "nPickup"}, {"var": "nStar3"}]}),
        json!({"gt": [{"var": "nStar3"}, {"lit": "5"}]}),
        json!({"ge": [{"var": "nPickup"}, {"lit": "1"}]}),
    ];
    for (index, condition) in impossible_conditions.into_iter().enumerate() {
        let pickup_probability = if index == 2 { "0" } else { "0.007" };
        let model = compile(&condition_model(condition, pickup_probability, Vec::new()))
            .expect("impossible condition is a warning, not an error");
        assert!(
            model
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "W003"),
            "condition case {index} must be diagnosed",
        );
    }
}

#[test]
fn w003_does_not_warn_when_a_grant_can_satisfy_a_zero_probability_condition() {
    let model = compile(&condition_model(
        json!({"ge": [{"var": "nPickup"}, {"lit": "1"}]}),
        "0",
        vec![json!({
            "at": {"trialCount": 1},
            "grant": {
                "leaf": "pickup",
                "amount": 1,
                "consumesTrial": false,
                "appliesTransitions": true
            }
        })],
    ))
    .expect("grant-reachable condition must compile");

    assert!(!model
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "W003"));
}

fn probability_table_model(max_a: u32, max_b: u32, probability: Value) -> ModelIr {
    serde_json::from_value(json!({
        "irVersion": 1,
        "name": "probability table diagnostic model",
        "entities": [{"id": "hit", "name": "hit", "prob": probability}],
        "stateVars": [
            {"id": "a", "init": 0, "max": max_a, "role": "control"},
            {"id": "b", "init": 0, "max": max_b, "role": "control"}
        ],
        "probRules": [],
        "transitions": [],
        "triggers": [],
        "run": {
            "maxTrials": 2,
            "trackJoint": ["hit"],
            "numeric": "scaled",
            "trialSeries": "none"
        }
    }))
    .expect("probability table diagnostic IR must deserialize")
}

fn two_control_probability() -> Value {
    json!({
        "add": [
            {"lit": "1/2"},
            {"mul": [
                {"lit": "0"},
                {"add": [{"var": "a"}, {"var": "b"}]}
            ]}
        ]
    })
}

#[test]
fn probability_table_preflight_rejects_trial_expansion_before_allocation() {
    let mut ir = probability_table_model(
        3_000,
        4_000,
        json!({
            "if": {"eq": [{"trial": true}, {"lit": "1"}]},
            "then": {"lit": "1/2"},
            "else": {"lit": "1/2"}
        }),
    );
    ir.run.max_trials = 5_000_001;
    let error =
        compile(&ir).expect_err("10M probability entries must be rejected before allocation");
    let diagnostic = error
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E012")
        .expect("the hard probability-table guard must have a dedicated error");
    assert!(
        error
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != "W010"),
        "E012 must replace the lower-severity W010 signal",
    );
    assert!(diagnostic.message.contains("10000002"));
    assert!(diagnostic.message.contains("control=1"));
    assert!(diagnostic.message.contains("trials=5000001"));
}

#[test]
fn unreachable_declared_controls_are_folded_and_the_model_runs() {
    let model = compile(&probability_table_model(
        3_000,
        4_000,
        two_control_probability(),
    ))
    .expect("only the initial control state is reachable without transitions");
    assert_eq!(model.prob_table.entries.len(), 1);
    assert!(model.prob_table.entry_control_invariant);
    assert_eq!(model.analysis.control_states, 1);
    assert!(model.analysis.dp_available);
    assert!(!model
        .diagnostics
        .iter()
        .any(|diagnostic| matches!(diagnostic.code.as_str(), "W010" | "E012")));

    let result = run_dp(&model, DpOptions::default(), |_, _| true)
        .expect("the reachable-state model must execute");
    let DpRunResult::Approximate(result) = result else {
        panic!("scaled model must use the approximate DP result");
    };
    assert_eq!(result.trials, 2);
    assert_eq!(result.peak_states, 3);
    assert_eq!(result.joint.len(), 3);
}

#[test]
fn reachable_control_table_uses_dense_indices_for_sparse_states() {
    let ir: ModelIr = serde_json::from_value(json!({
        "irVersion": 1,
        "name": "sparse reachable controls",
        "entities": [{
            "id": "hit",
            "name": "hit",
            "prob": {
                "add": [
                    {"lit": "1/4"},
                    {"mul": [{"lit": "1/20"}, {"var": "pity"}]}
                ]
            }
        }],
        "stateVars": [
            {"id": "pity", "init": 0, "max": 10, "role": "control"}
        ],
        "probRules": [],
        "transitions": [{
            "when": {"leafOf": "hit"},
            "set": {"pity": {"add": [{"var": "pity"}, {"lit": "2"}]}}
        }],
        "triggers": [],
        "run": {
            "maxTrials": 2,
            "trackJoint": ["hit"],
            "numeric": "scaled",
            "trialSeries": "none"
        }
    }))
    .expect("sparse reachable-control model must deserialize");

    let model = compile(&ir).expect("sparse reachable-control model must compile");
    assert_eq!(model.analysis.control_states, 3);
    assert_eq!(model.prob_table.entries.len(), 3);
    assert_eq!(
        model
            .prob_table
            .control_entry_indices
            .keys()
            .copied()
            .collect::<Vec<_>>(),
        vec![0, 2, 4],
    );
    assert_eq!(model.transition_table.entries.len(), 3);
    assert_eq!(model.transition_control_index(2, 0, 2), 4);

    let result = run_dp(&model, DpOptions::default(), |_, _| true)
        .expect("dense probability and transition indices must execute");
    let DpRunResult::Approximate(result) = result else {
        panic!("scaled model must use the approximate DP result");
    };
    assert_eq!(result.trials, 2);
    assert_eq!(result.joint.len(), 3);
}

#[test]
fn reachable_controls_include_trigger_assignments() {
    let ir: ModelIr = serde_json::from_value(json!({
        "irVersion": 1,
        "name": "trigger reachable controls",
        "entities": [{
            "id": "hit",
            "name": "hit",
            "prob": {
                "add": [
                    {"lit": "1/4"},
                    {"mul": [{"lit": "1/20"}, {"var": "pity"}]}
                ]
            }
        }],
        "stateVars": [
            {"id": "pity", "init": 0, "max": 10, "role": "control"}
        ],
        "probRules": [],
        "transitions": [],
        "triggers": [{
            "at": {"trialCount": 1},
            "set": {"pity": {"lit": "5"}}
        }],
        "run": {
            "maxTrials": 2,
            "trackJoint": ["hit"],
            "numeric": "scaled",
            "trialSeries": "none"
        }
    }))
    .expect("trigger reachable-control model must deserialize");

    let model = compile(&ir).expect("trigger reachable-control model must compile");
    assert_eq!(
        model
            .prob_table
            .control_entry_indices
            .keys()
            .copied()
            .collect::<Vec<_>>(),
        vec![0, 5],
    );
    let result = run_dp(&model, DpOptions::default(), |_, _| true)
        .expect("trigger-assigned control state must be present in the probability table");
    let DpRunResult::Approximate(result) = result else {
        panic!("scaled model must use the approximate DP result");
    };
    assert_eq!(result.trials, 2);
    assert_eq!(result.joint.len(), 3);
}

#[test]
fn cyclic_transition_frontier_keeps_late_trigger_states_reachable() {
    let ir: ModelIr = serde_json::from_value(json!({
        "irVersion": 2,
        "name": "parity cycle with late trigger",
        "entities": [
            {
                "id": "a",
                "name": "a",
                "prob": {
                    "if": {"eq": [{"var": "parity"}, {"lit": "0"}]},
                    "then": {"lit": "0.5"},
                    "else": {"lit": "0.25"}
                }
            },
            {
                "id": "b",
                "name": "b",
                "prob": {
                    "if": {"eq": [{"var": "parity"}, {"lit": "0"}]},
                    "then": {"lit": "0.5"},
                    "else": {"lit": "0.75"}
                }
            }
        ],
        "nestingPolicy": "error",
        "stateVars": [
            {"id": "parity", "init": 0, "max": 1, "role": "control"},
            {"id": "flag", "init": 0, "max": 1, "role": "control"}
        ],
        "probRules": [],
        "transitions": [{
            "when": {
                "or": [
                    {"leafIs": "a"},
                    {"leafIs": "b"},
                    {"leafIs": "__other__"}
                ]
            },
            "set": {
                "parity": {"sub": [{"lit": "1"}, {"var": "parity"}]}
            }
        }],
        "triggers": [{
            "at": {"trialCount": 6},
            "set": {
                "flag": {"sub": [{"lit": "1"}, {"var": "parity"}]}
            }
        }],
        "run": {
            "maxTrials": 7,
            "trackJoint": ["a"],
            "numeric": "f64",
            "trialSeries": "none"
        }
    }))
    .expect("cyclic trigger regression IR must deserialize");

    let model = compile(&ir).expect("cyclic trigger model must compile");
    let result = run_dp(&model, DpOptions::default(), |_, _| true)
        .expect("late-trigger control states must be present in the probability table");
    assert_eq!(model.analysis.control_states, 4);
    assert_eq!(
        model
            .prob_table
            .control_entry_indices
            .keys()
            .copied()
            .collect::<Vec<_>>(),
        vec![0, 1, 2, 3],
    );

    let DpRunResult::Approximate(result) = result else {
        panic!("f64 model must use the approximate DP result");
    };
    let probability = |count| {
        result
            .joint
            .iter()
            .find(|cell| cell.counts == [count])
            .map(|cell| cell.probability)
            .unwrap_or_default()
    };
    assert!((probability(0) - 0.026_367_187_5).abs() <= 1e-12);
    assert!((probability(7) - 0.000_976_562_5).abs() <= 1e-12);
}
