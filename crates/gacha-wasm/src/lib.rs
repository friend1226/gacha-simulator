use gacha_core::{compile, run_dp, run_exact, run_mc, DpOptions, ExactOptions, McOptions, ModelIr};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn validate_model(source: &str) -> Result<String, JsValue> {
    let ir: ModelIr = serde_json::from_str(source).map_err(js_error)?;
    match compile(&ir) {
        Ok(model) => serde_json::to_string(&serde_json::json!({
            "ok": true,
            "leaves": model.leaves,
            "diagnostics": model.diagnostics,
            "analysis": model.analysis,
            "exactCommonDenominator": model.exact_lcm.to_string(),
        }))
        .map_err(js_error),
        Err(error) => serde_json::to_string(&serde_json::json!({
            "ok": false,
            "diagnostics": error.diagnostics,
        }))
        .map_err(js_error),
    }
}

#[wasm_bindgen]
pub fn run_dp_json(source: &str) -> Result<String, JsValue> {
    let model = compile_source(source)?;
    let result = run_dp(&model, DpOptions::default(), |_, _| true).map_err(js_error)?;
    serde_json::to_string(&result).map_err(js_error)
}

#[wasm_bindgen]
pub fn run_exact_json(source: &str) -> Result<String, JsValue> {
    let model = compile_source(source)?;
    let result = run_exact(&model, ExactOptions::default(), |_, _| true).map_err(js_error)?;
    serde_json::to_string(&result).map_err(js_error)
}

#[wasm_bindgen]
pub fn run_mc_json(source: &str, runs: u32, seed: u32) -> Result<String, JsValue> {
    let model = compile_source(source)?;
    let result = run_mc(
        &model,
        McOptions {
            runs: runs as u64,
            seed: seed as u64,
            ..Default::default()
        },
        |_, _| true,
    );
    serde_json::to_string(&result).map_err(js_error)
}

fn compile_source(source: &str) -> Result<gacha_core::CompiledModel, JsValue> {
    let ir: ModelIr = serde_json::from_str(source).map_err(js_error)?;
    compile(&ir).map_err(|e| {
        JsValue::from_str(
            &e.diagnostics
                .iter()
                .map(|d| format!("{}: {}", d.code, d.message))
                .collect::<Vec<_>>()
                .join("\n"),
        )
    })
}

fn js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}
