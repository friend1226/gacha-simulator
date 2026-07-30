use gacha_core::{compile, ModelIr, Severity};
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
fn probability_table_preflight_warns_and_rejects_before_expansion() {
    let warning_model = compile(&probability_table_model(
        700,
        400,
        two_control_probability(),
    ))
    .expect("562K probability entries must remain below the hard limit");
    let warning = warning_model
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "W010")
        .expect("large probability tables must warn");
    assert!(warning.message.contains("562202"));
    assert!(warning.message.contains("control=281101"));
    assert!(warning.message.contains("trials=1"));
    assert!(warning.message.contains("leaves=2"));

    let error = compile(&probability_table_model(
        3_000,
        4_000,
        two_control_probability(),
    ))
    .expect_err("24M probability entries must be rejected before allocation");
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
    assert!(diagnostic.message.contains("24014002"));
    assert!(diagnostic.message.contains("control=12007001"));
}

#[test]
fn control_invariant_probability_table_counts_the_folded_control_axis() {
    let model = compile(&probability_table_model(
        3_000,
        4_000,
        json!({"lit": "1/2"}),
    ))
    .expect("control-independent probability table must fold before the hard guard");
    assert_eq!(model.prob_table.entries.len(), 1);
    assert!(model.prob_table.entry_control_invariant);
    assert!(!model
        .diagnostics
        .iter()
        .any(|diagnostic| matches!(diagnostic.code.as_str(), "W010" | "E012")));
}
