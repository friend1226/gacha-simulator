use gacha_core::{compile, run_dp, DpOptions, DpRunResult, ModelIr};
use serde_json::json;
use std::env;
use std::error::Error;

const PICKUP_TARGET_MS: u64 = 300;
const JOINT_TARGET_MS: u64 = 8_000;

fn hard_pity_model(track_joint: &[&str]) -> Result<gacha_core::CompiledModel, Box<dyn Error>> {
    let ir: ModelIr = serde_json::from_value(json!({
        "irVersion": 1,
        "name": "M8 hard-pity stress benchmark",
        "entities": [{
            "id": "star3",
            "name": "star3",
            "prob": {"lit": "0.03"},
            "children": [{
                "id": "pickup",
                "name": "pickup",
                "prob": {"lit": "0.007"}
            }]
        }],
        "stateVars": [{"id": "pity", "init": 0, "max": 179, "role": "control"}],
        "probRules": [{
            "target": "star3",
            "expr": {
                "if": {"ge": [{"var": "pity"}, {"lit": "179"}]},
                "then": {"lit": "1"},
                "else": {"lit": "0.03"}
            }
        }],
        "transitions": [
            {"when": {"leafOf": "star3"}, "set": {"pity": {"lit": "0"}}},
            {
                "when": {"not": {"leafOf": "star3"}},
                "set": {"pity": {"add": [{"var": "pity"}, {"lit": "1"}]}}
            }
        ],
        "triggers": [],
        "run": {
            "maxTrials": 1000,
            "trackJoint": track_joint,
            "numeric": "scaled"
        }
    }))?;
    let model = compile(&ir)?;
    assert!(
        !model.prob_table.control_invariant,
        "benchmark must retain all 180 pity control states"
    );
    Ok(model)
}

fn measure(track_joint: &[&str], runs: usize) -> Result<(Vec<u64>, u64), Box<dyn Error>> {
    let model = hard_pity_model(track_joint)?;
    let mut samples = Vec::with_capacity(runs);
    for _ in 0..runs {
        let result = run_dp(&model, DpOptions::default(), |_, _| true)?;
        let DpRunResult::Approximate(result) = result else {
            return Err("hard-pity benchmark unexpectedly selected exact DP".into());
        };
        samples.push(result.elapsed_ms);
    }
    samples.sort_unstable();
    let median = samples[samples.len() / 2];
    Ok((samples, median))
}

fn main() -> Result<(), Box<dyn Error>> {
    let runs = env::var("M8_BENCH_RUNS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|runs| *runs > 0)
        .unwrap_or(3);
    let (pickup_samples, pickup_median) = measure(&["pickup"], runs)?;
    let (joint_samples, joint_median) = measure(&["pickup", "star3__self"], runs)?;

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "scenario": "N=1000, control states=180, hard-pity probability depends on control",
            "rayonThreads": env::var("RAYON_NUM_THREADS").ok(),
            "runs": runs,
            "pickup": {
                "samplesMs": pickup_samples,
                "medianMs": pickup_median,
                "designTargetMs": PICKUP_TARGET_MS,
                "meetsTarget": pickup_median < PICKUP_TARGET_MS
            },
            "pickupByStar3Self": {
                "samplesMs": joint_samples,
                "medianMs": joint_median,
                "designTargetMs": JOINT_TARGET_MS,
                "meetsTarget": joint_median < JOINT_TARGET_MS
            }
        }))?
    );
    Ok(())
}
