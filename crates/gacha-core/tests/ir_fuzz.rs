use gacha_core::engine_dp::{DpOptions, DpRunResult};
use gacha_core::{compile, run_dp, run_mc, McOptions, ModelIr};
use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;
use serde_json::{json, Value};
use std::panic::{catch_unwind, AssertUnwindSafe};

fn choose<T: Copy>(rng: &mut Xoshiro256PlusPlus, values: &[T]) -> T {
    values[rng.gen_range(0..values.len())]
}

fn random_expr(rng: &mut Xoshiro256PlusPlus) -> Value {
    let literal = choose(
        rng,
        &[
            "0", "1", "1/2", "0.007", "-1/3", "2", "1/0", "", "NaN", "1e-400",
        ],
    );
    match rng.gen_range(0..10) {
        0 => json!({ "lit": literal }),
        1 => json!({ "var": choose(rng, &["pity", "stat", "missing", ""]) }),
        2 => json!({ "add": [{ "lit": literal }, { "lit": "1" }] }),
        3 => json!({ "sub": [{ "var": "pity" }, { "lit": literal }] }),
        4 => json!({ "div": [{ "lit": "1" }, { "lit": literal }] }),
        5 => json!({
            "if": { "ge": [{ "var": "pity" }, { "lit": "1" }] },
            "then": { "lit": literal },
            "else": { "lit": "0" }
        }),
        6 => json!({ "add": [] }),
        7 => json!({ "unknown": [{ "lit": literal }] }),
        8 => Value::Null,
        _ => json!([literal]),
    }
}

fn random_schema_valid_ir(rng: &mut Xoshiro256PlusPlus, case: usize) -> ModelIr {
    let entity_count = rng.gen_range(0..=3);
    let mut entities = Vec::with_capacity(entity_count);
    for index in 0..entity_count {
        let id = if rng.gen_bool(0.2) {
            choose(rng, &["", "item", "__other__"]).to_owned()
        } else {
            format!("item{}", if rng.gen_bool(0.2) { 0 } else { index })
        };
        let children = if rng.gen_bool(0.35) {
            vec![json!({
                "id": if rng.gen_bool(0.2) { id.clone() } else { format!("{id}_child") },
                "name": "child",
                "prob": random_expr(rng)
            })]
        } else {
            Vec::new()
        };
        entities.push(json!({
            "id": id,
            "name": format!("entity {index}"),
            "prob": random_expr(rng),
            "children": children
        }));
    }

    let transitions = if rng.gen_bool(0.5) {
        vec![json!({
            "when": if rng.gen_bool(0.5) {
                json!({ "leafIs": choose(rng, &["item0", "missing", "__other__"]) })
            } else {
                json!({ "not": { "leafOf": choose(rng, &["item0", "missing", ""]) } })
            },
            "set": {
                choose(rng, &["pity", "stat", "missing", ""]): random_expr(rng)
            }
        })]
    } else {
        Vec::new()
    };

    let triggers = if rng.gen_bool(0.5) {
        vec![json!({
            "at": { "trialCount": choose(rng, &[0_u32, 1, 2, u32::MAX]) },
            "grant": {
                "leaf": choose(rng, &["item0", "item0_child", "missing", "__other__"]),
                "amount": choose(rng, &[0_u32, 1, 2, u32::MAX]),
                "consumesTrial": rng.gen_bool(0.5),
                "appliesTransitions": rng.gen_bool(0.5)
            },
            "set": {
                choose(rng, &["pity", "stat", "missing"]): random_expr(rng)
            }
        })]
    } else {
        Vec::new()
    };

    serde_json::from_value(json!({
        "irVersion": choose(rng, &[0_u32, 1, u32::MAX]),
        "name": format!("fuzz case {case}"),
        "entities": entities,
        "nestingPolicy": choose(
            rng,
            &["clampChildren", "expandParent", "scaleSiblings", "error"],
        ),
        "stateVars": [
            {
                "id": "pity",
                "init": choose(rng, &[-1_i64, 0, 1, i64::MAX]),
                "max": choose(rng, &[0_u32, 1, 3]),
                "role": "control"
            },
            {
                "id": if rng.gen_bool(0.2) { "pity" } else { "stat" },
                "init": choose(rng, &[-1_i64, 0, 1]),
                "max": Value::Null,
                "role": "stat"
            }
        ],
        "probRules": [{
            "target": choose(rng, &["item0", "item0_child", "missing", ""]),
            "expr": random_expr(rng)
        }],
        "transitions": transitions,
        "triggers": triggers,
        "run": {
            "maxTrials": choose(rng, &[0_u32, 1, 2, 5, u32::MAX]),
            "trackJoint": [choose(rng, &["item0", "item0_child", "missing", "__other__"])],
            "numeric": choose(rng, &["f64", "scaled", "exact"]),
            "condition": if rng.gen_bool(0.5) { random_expr(rng) } else { Value::Null }
        }
    }))
    .expect("fuzzer must produce schema-valid IR")
}

#[test]
fn randomized_ir_compilation_never_panics_and_errors_are_structured() {
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(0x13_03_0007);

    for case in 0..512 {
        let ir = random_schema_valid_ir(&mut rng, case);
        let outcome = catch_unwind(AssertUnwindSafe(|| compile(&ir)));
        let compile_result = outcome.unwrap_or_else(|payload| {
            panic!("compiler panicked for deterministic fuzz case {case}: {payload:?}")
        });
        if let Err(error) = compile_result {
            assert!(
                !error.diagnostics.is_empty(),
                "invalid fuzz case {case} returned an empty CompileError"
            );
            assert!(
                error.diagnostics.iter().all(|diagnostic| {
                    !diagnostic.code.is_empty() && !diagnostic.message.is_empty()
                }),
                "fuzz case {case} returned a malformed diagnostic: {:?}",
                error.diagnostics
            );
        }
    }
}

#[test]
fn randomized_small_valid_models_finish_in_dp_and_mc() {
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(0x13_03_0007_0002);
    let probabilities = ["0", "1/100", "1/2", "99/100", "1"];

    for case in 0..128 {
        let probability = probabilities[rng.gen_range(0..probabilities.len())];
        let max_trials = rng.gen_range(0..=8_u32);
        let trigger_trial = rng.gen_range(0..=10_u32);
        let consumes_trial = rng.gen_bool(0.5);
        let applies_transitions = rng.gen_bool(0.5);
        let ir: ModelIr = serde_json::from_value(json!({
            "irVersion": 1,
            "name": format!("engine fuzz case {case}"),
            "entities": [{
                "id": "hit",
                "name": "hit",
                "prob": { "lit": probability }
            }],
            "stateVars": [{
                "id": "pity",
                "init": 0,
                "max": 2,
                "role": "control"
            }],
            "probRules": [],
            "transitions": [{
                "when": { "leafIs": "hit" },
                "set": { "pity": { "lit": "0" } }
            }],
            "triggers": [{
                "at": { "trialCount": trigger_trial },
                "grant": {
                    "leaf": "hit",
                    "amount": rng.gen_range(0..=2_u32),
                    "consumesTrial": consumes_trial,
                    "appliesTransitions": applies_transitions
                },
                "set": {}
            }],
            "run": {
                "maxTrials": max_trials,
                "trackJoint": ["hit"],
                "numeric": "scaled"
            }
        }))
        .expect("engine fuzzer must produce schema-valid IR");
        let model = compile(&ir).expect("engine fuzz model must compile");

        let outcome = catch_unwind(AssertUnwindSafe(|| {
            let dp = run_dp(
                &model,
                DpOptions {
                    prune_log10: None,
                    ..Default::default()
                },
                |_, _| true,
            )
            .expect("DP fuzz run");
            assert!(matches!(dp, DpRunResult::Approximate(_)));
            let mc = run_mc(
                &model,
                McOptions {
                    runs: 64,
                    seed: case as u64,
                    confidence_z: 1.96,
                    batch_size: 16,
                },
                |_, _| true,
            );
            assert_eq!(mc.runs, 64);
        }));
        assert!(
            outcome.is_ok(),
            "engine panicked for deterministic fuzz case {case}"
        );
    }
}
