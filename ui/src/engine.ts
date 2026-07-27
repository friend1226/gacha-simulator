import type { ModelIr } from "./types";

export interface WasmEngine {
  default?: () => Promise<void>;
  run_dp_json: (source: string) => string;
  run_exact_json: (source: string) => string;
  run_mc_json: (source: string, runs: number, seed: number) => string;
}

export function runDpJson(
  wasm: Pick<WasmEngine, "run_dp_json" | "run_exact_json">,
  model: ModelIr,
): { engine: "DP" | "EXACT"; json: string } {
  const source = JSON.stringify(model);
  return model.run.numeric === "exact"
    ? { engine: "EXACT", json: wasm.run_exact_json(source) }
    : { engine: "DP", json: wasm.run_dp_json(source) };
}
