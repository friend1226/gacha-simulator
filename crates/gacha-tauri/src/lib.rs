use gacha_core::{
    compile, run_dp, run_exact, run_mc, CompiledModel, DpOptions, ExactOptions, McOptions, ModelIr,
};

fn compile_source(source: &str) -> Result<CompiledModel, String> {
    let ir: ModelIr = serde_json::from_str(source).map_err(|error| error.to_string())?;
    compile(&ir).map_err(|error| {
        error
            .diagnostics
            .iter()
            .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.message))
            .collect::<Vec<_>>()
            .join("\n")
    })
}

fn validate_model_impl(source: &str) -> Result<String, String> {
    let ir: ModelIr = serde_json::from_str(source).map_err(|error| error.to_string())?;
    let response = match compile(&ir) {
        Ok(model) => serde_json::json!({
            "ok": true,
            "leaves": model.leaves,
            "diagnostics": model.diagnostics,
            "analysis": model.analysis,
            "exactCommonDenominator": model.exact_lcm.to_string(),
        }),
        Err(error) => serde_json::json!({
            "ok": false,
            "diagnostics": error.diagnostics,
        }),
    };
    serde_json::to_string(&response).map_err(|error| error.to_string())
}

fn run_dp_json_impl(source: &str) -> Result<String, String> {
    let model = compile_source(source)?;
    let result =
        run_dp(&model, DpOptions::default(), |_, _| true).map_err(|error| error.to_string())?;
    serde_json::to_string(&result).map_err(|error| error.to_string())
}

fn run_exact_json_impl(source: &str) -> Result<String, String> {
    let model = compile_source(source)?;
    let result = run_exact(&model, ExactOptions::default(), |_, _| true)
        .map_err(|error| error.to_string())?;
    serde_json::to_string(&result).map_err(|error| error.to_string())
}

fn run_mc_json_impl(source: &str, runs: u32, seed: u32) -> Result<String, String> {
    let model = compile_source(source)?;
    let result = run_mc(
        &model,
        McOptions {
            runs: u64::from(runs),
            seed: u64::from(seed),
            ..Default::default()
        },
        |_, _| true,
    );
    serde_json::to_string(&result).map_err(|error| error.to_string())
}

#[tauri::command]
async fn validate_model(source: String) -> Result<String, String> {
    validate_model_impl(&source)
}

#[tauri::command]
async fn run_dp_json(source: String) -> Result<String, String> {
    run_dp_json_impl(&source)
}

#[tauri::command]
async fn run_exact_json(source: String) -> Result<String, String> {
    run_exact_json_impl(&source)
}

#[tauri::command]
async fn run_mc_json(source: String, runs: u32, seed: u32) -> Result<String, String> {
    run_mc_json_impl(&source, runs, seed)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            validate_model,
            run_dp_json,
            run_exact_json,
            run_mc_json
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Gacha Lab desktop application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn source(numeric: &str) -> String {
        json!({
            "irVersion": 1,
            "name": "Tauri command smoke model",
            "entities": [{"id": "hit", "name": "hit", "prob": {"lit": "0.5"}}],
            "stateVars": [],
            "probRules": [],
            "transitions": [],
            "triggers": [],
            "run": {"maxTrials": 2, "trackJoint": ["hit"], "numeric": numeric}
        })
        .to_string()
    }

    #[test]
    fn validation_command_uses_core_compiler() {
        let response: Value =
            serde_json::from_str(&validate_model_impl(&source("scaled")).unwrap()).unwrap();

        assert_eq!(response["ok"], true);
        assert_eq!(response["analysis"]["dpAvailable"], true);
    }

    #[test]
    fn dp_command_uses_native_core_backend() {
        let response: Value =
            serde_json::from_str(&run_dp_json_impl(&source("scaled")).unwrap()).unwrap();

        assert_eq!(response["numeric"], "scaled");
        assert_eq!(response["joint"].as_array().unwrap().len(), 3);
    }
}
