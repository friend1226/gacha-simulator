import { invoke, isTauri } from "@tauri-apps/api/core";
import type { ModelIr } from "./types";

interface WasmEngine {
  default?: () => Promise<void>;
  run_dp_json: (source: string) => string;
  run_exact_json: (source: string) => string;
  run_mc_json: (source: string, runs: number, seed: number) => string;
}

export interface EngineBackend {
  platform: "web" | "tauri";
  runDpJson: (source: string) => Promise<string>;
  runExactJson: (source: string) => Promise<string>;
  runMcJson: (source: string, runs: number, seed: number) => Promise<string>;
}

export async function loadEngineBackend(): Promise<EngineBackend> {
  if (isTauri()) {
    return {
      platform: "tauri",
      runDpJson: (source) => invoke<string>("run_dp_json", { source }),
      runExactJson: (source) => invoke<string>("run_exact_json", { source }),
      runMcJson: (source, runs, seed) =>
        invoke<string>("run_mc_json", { source, runs, seed }),
    };
  }

  const wasmPath = "/wasm/gacha_wasm.js";
  const wasm = (await import(/* @vite-ignore */ wasmPath)) as WasmEngine;
  await wasm.default?.();
  return {
    platform: "web",
    runDpJson: async (source) => wasm.run_dp_json(source),
    runExactJson: async (source) => wasm.run_exact_json(source),
    runMcJson: async (source, runs, seed) => wasm.run_mc_json(source, runs, seed),
  };
}

export async function runDpJson(
  backend: Pick<EngineBackend, "runDpJson" | "runExactJson">,
  model: ModelIr,
): Promise<{ engine: "DP" | "EXACT"; json: string }> {
  const source = JSON.stringify(model);
  return model.run.numeric === "exact"
    ? { engine: "EXACT", json: await backend.runExactJson(source) }
    : { engine: "DP", json: await backend.runDpJson(source) };
}
