/// <reference lib="webworker" />

import type { EngineWorkerRequest, EngineWorkerResponse } from "./engineWorkerProtocol";

interface WasmEngine {
  default?: () => Promise<void>;
  run_dp_json: (source: string) => string;
  run_exact_json: (source: string) => string;
  run_mc_json: (source: string, runs: number, seed: number) => string;
}

let wasmPromise: Promise<WasmEngine> | undefined;

function loadWasm(): Promise<WasmEngine> {
  if (!wasmPromise) {
    const wasmPath = "/wasm/gacha_wasm.js";
    wasmPromise = import(/* @vite-ignore */ wasmPath).then(async (wasm: WasmEngine) => {
      await wasm.default?.();
      return wasm;
    });
  }
  return wasmPromise;
}

self.onmessage = async (event: MessageEvent<EngineWorkerRequest>) => {
  const request = event.data;
  let response: EngineWorkerResponse;
  try {
    const wasm = await loadWasm();
    const json = request.method === "dp"
      ? wasm.run_dp_json(request.source)
      : request.method === "exact"
        ? wasm.run_exact_json(request.source)
        : wasm.run_mc_json(request.source, request.runs ?? 0, request.seed ?? 0);
    response = { id: request.id, ok: true, json };
  } catch (error) {
    response = { id: request.id, ok: false, error: error instanceof Error ? error.message : String(error) };
  }
  self.postMessage(response);
};

export {};
