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
