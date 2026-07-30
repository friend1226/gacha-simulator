use gacha_core::engine_dp::{DpOptions, DpResult, DpRunResult};
use gacha_core::{compile, run_dp, run_exact, ExactOptions, ModelIr};
use num_bigint::BigInt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Golden {
    preset_sha256: String,
    result_sha256: String,
    numeric: String,
    trials: u32,
    tracked_leaf_ids: Vec<String>,
    joint_cells: usize,
    remaining_mass: String,
    has_first_hit: bool,
}

fn sha256(value: &[u8]) -> String {
    Sha256::digest(value)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn probability(value: f64) -> String {
    let normalized = if value == 0.0 { 0.0 } else { value };
    format!("{normalized:.11e}")
}

fn canonical_result(result: &DpResult) -> serde_json::Value {
    let joint: Vec<_> = result
        .joint
        .iter()
        .map(|cell| {
            json!({
                "counts": cell.counts,
                "display": cell.display,
            })
        })
        .collect();
    let first_hit = result.first_hit.as_ref().map(|hit| {
        json!({
            "pmf": hit.pmf.iter().map(|value| probability(*value)).collect::<Vec<_>>(),
            "cdf": hit.cdf.iter().map(|value| probability(*value)).collect::<Vec<_>>(),
            "failureReachable": probability(hit.failure_reachable),
            "mean": hit.mean.map(probability),
            "percentiles": hit.percentiles.iter()
                .map(|(level, trial)| (probability(*level), *trial))
                .collect::<Vec<_>>(),
        })
    });
    json!({
        "numeric": result.numeric,
        "trials": result.trials,
        "trackedLeafIds": result.tracked_leaf_ids,
        "joint": joint,
        "firstHit": first_hit,
        "prunedMass": probability(result.pruned_mass),
        "clampEvents": result.clamp_events,
    })
}

fn assert_scaled_golden(preset_source: &str, golden_source: &str) {
    let ir: ModelIr = serde_json::from_str(preset_source).expect("preset must deserialize");
    let model = compile(&ir).expect("preset must compile");
    let result = run_dp(
        &model,
        DpOptions {
            prune_log10: None,
            ..Default::default()
        },
        |_, _| true,
    )
    .expect("preset DP must run");
    let DpRunResult::Approximate(result) = result else {
        panic!("golden presets must use approximate DP");
    };

    let canonical =
        serde_json::to_vec(&canonical_result(&result)).expect("canonical result must serialize");
    let actual = Golden {
        preset_sha256: sha256(preset_source.as_bytes()),
        result_sha256: sha256(&canonical),
        numeric: result.numeric.clone(),
        trials: result.trials,
        tracked_leaf_ids: result.tracked_leaf_ids.clone(),
        joint_cells: result.joint.len(),
        remaining_mass: probability(result.joint.iter().map(|cell| cell.probability).sum()),
        has_first_hit: result.first_hit.is_some(),
    };
    let expected: Golden = serde_json::from_str(golden_source).expect("golden must deserialize");
    assert_eq!(
        actual,
        expected,
        "preset output changed; review and explicitly update the golden:\n{}",
        serde_json::to_string_pretty(&actual).unwrap(),
    );
}

fn assert_exact_golden(preset_source: &str, golden_source: &str) {
    let mut ir: ModelIr = serde_json::from_str(preset_source).expect("preset must deserialize");
    ir.run.numeric = gacha_core::ir::NumericBackend::Exact;
    let model = compile(&ir).expect("preset must compile");
    let result =
        run_exact(&model, ExactOptions::default(), |_, _| true).expect("preset exact DP must run");
    let canonical = json!({
        "numeric": result.numeric,
        "trials": result.trials,
        "trackedLeafIds": result.tracked_leaf_ids,
        "joint": result.joint.iter().map(|cell| json!({
            "counts": cell.counts,
            "numerator": cell.numerator,
            "denominator": cell.denominator,
        })).collect::<Vec<_>>(),
        "denominator": result.denominator,
        "clampEvents": result.clamp_events,
    });
    let canonical = serde_json::to_vec(&canonical).expect("canonical result must serialize");
    let numerator_sum: BigInt = result
        .joint
        .iter()
        .map(|cell| cell.numerator.parse::<BigInt>().expect("exact numerator"))
        .sum();
    let remaining_mass = if numerator_sum.to_string() == result.denominator {
        "1".to_owned()
    } else {
        format!("{numerator_sum}/{}", result.denominator)
    };
    let actual = Golden {
        preset_sha256: sha256(preset_source.as_bytes()),
        result_sha256: sha256(&canonical),
        numeric: result.numeric.clone(),
        trials: result.trials,
        tracked_leaf_ids: result.tracked_leaf_ids.clone(),
        joint_cells: result.joint.len(),
        remaining_mass,
        has_first_hit: false,
    };
    let expected: Golden = serde_json::from_str(golden_source).expect("golden must deserialize");
    assert_eq!(
        actual,
        expected,
        "preset output changed; review and explicitly update the golden:\n{}",
        serde_json::to_string_pretty(&actual).unwrap(),
    );
}

#[test]
fn blue_archive_pickup_matches_golden() {
    assert_exact_golden(
        include_str!("../../../presets/blue-archive-pickup.json"),
        include_str!("../../../presets/golden/blue-archive-pickup.json"),
    );
}

#[test]
fn simple_pity_matches_golden() {
    assert_scaled_golden(
        include_str!("../../../presets/simple-pity.json"),
        include_str!("../../../presets/golden/simple-pity.json"),
    );
}

#[test]
fn arknights_first_ten_guarantee_matches_golden() {
    assert_exact_golden(
        include_str!("../../../presets/arknights-first-ten-guarantee.json"),
        include_str!("../../../presets/golden/arknights-first-ten-guarantee.json"),
    );
}

#[test]
fn arknights_first_ten_guarantee_invariants_hold() {
    let source = include_str!("../../../presets/arknights-first-ten-guarantee.json");
    let ir: ModelIr = serde_json::from_str(source).expect("preset must deserialize");
    let model = compile(&ir).expect("preset must compile");
    let result =
        run_exact(&model, ExactOptions::default(), |_, _| true).expect("preset exact DP must run");
    let star6 = result
        .tracked_leaf_ids
        .iter()
        .position(|id| id == "star6")
        .expect("star6 must be tracked");
    let star5 = result
        .tracked_leaf_ids
        .iter()
        .position(|id| id == "star5")
        .expect("star5 must be tracked");

    let no_high_rarity_numerator: BigInt = result
        .joint
        .iter()
        .filter(|cell| cell.counts[star6] == 0 && cell.counts[star5] == 0)
        .map(|cell| cell.numerator.parse::<BigInt>().expect("exact numerator"))
        .sum();
    assert_eq!(
        no_high_rarity_numerator,
        BigInt::from(0),
        "Arknights first-ten guarantee broke: P(no 5-star or 6-star) must be exactly zero",
    );

    assert!(
        result
            .joint
            .iter()
            .all(|cell| cell.counts.iter().sum::<u32>() == 10),
        "Arknights first-ten guarantee broke: every result cell must contain exactly 10 pulls",
    );

    let expected_star6 = result
        .joint
        .iter()
        .map(|cell| cell.counts[star6] as f64 * cell.probability)
        .sum::<f64>();
    // The first-ten guarantee must not change the 6-star expectation: 10 pulls × base 2%.
    assert!(
        (expected_star6 - 0.2).abs() < 1e-12,
        "Arknights first-ten guarantee broke: E[6-star]={expected_star6:.14}, expected 0.2",
    );

    let expected_star5 = result
        .joint
        .iter()
        .map(|cell| cell.counts[star5] as f64 * cell.probability)
        .sum::<f64>();
    assert!(
        (expected_star5 - 1.148_678_440_1).abs() < 1e-12,
        "Arknights first-ten guarantee broke: E[5-star]={expected_star5:.14}, expected 1.1486784401",
    );
}
