import { invoke, isTauri } from "@tauri-apps/api/core";
import type { ModelIr } from "./types";
import type { EngineWorkerMethod, EngineWorkerRequest, EngineWorkerResponse } from "./engineWorkerProtocol";

export interface EngineBackend {
  platform: "web" | "tauri";
  runDpJson: (source: string) => Promise<string>;
  runExactJson: (source: string) => Promise<string>;
  runMcJson: (source: string, runs: number, seed: number) => Promise<string>;
  cancel: () => void;
}

export class EngineCancelledError extends Error {
  constructor() {
    super("Engine execution cancelled");
    this.name = "EngineCancelledError";
  }
}

interface WorkerLike {
  onmessage: ((event: MessageEvent<EngineWorkerResponse>) => void) | null;
  onerror: ((event: ErrorEvent) => void) | null;
  postMessage: (message: EngineWorkerRequest) => void;
  terminate: () => void;
}

export class WebWorkerEngineBackend implements EngineBackend {
  readonly platform = "web";
  private worker?: WorkerLike;
  private nextId = 1;
  private pending = new Map<number, {
    resolve: (json: string) => void;
    reject: (error: Error) => void;
  }>();

  constructor(private readonly createWorker: () => WorkerLike = () =>
    new Worker(new URL("./engine.worker.ts", import.meta.url), { type: "module" })) {}

  runDpJson(source: string) {
    return this.request("dp", source);
  }

  runExactJson(source: string) {
    return this.request("exact", source);
  }

  runMcJson(source: string, runs: number, seed: number) {
    return this.request("mc", source, runs, seed);
  }

  cancel() {
    this.worker?.terminate();
    this.worker = undefined;
    for (const request of this.pending.values()) request.reject(new EngineCancelledError());
    this.pending.clear();
  }

  private request(method: EngineWorkerMethod, source: string, runs?: number, seed?: number) {
    const worker = this.ensureWorker();
    const id = this.nextId++;
    return new Promise<string>((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      worker.postMessage({ id, method, source, runs, seed });
    });
  }

  private ensureWorker(): WorkerLike {
    if (this.worker) return this.worker;
    const worker = this.createWorker();
    worker.onmessage = (event) => {
      const response = event.data;
      const request = this.pending.get(response.id);
      if (!request) return;
      this.pending.delete(response.id);
      if (response.ok) request.resolve(response.json);
      else request.reject(new Error(response.error));
    };
    worker.onerror = (event) => {
      const error = new Error(event.message || "Web Worker failed");
      worker.terminate();
      this.worker = undefined;
      for (const request of this.pending.values()) request.reject(error);
      this.pending.clear();
    };
    this.worker = worker;
    return worker;
  }
}

let webBackend: WebWorkerEngineBackend | undefined;

export async function loadEngineBackend(): Promise<EngineBackend> {
  if (isTauri()) {
    return {
      platform: "tauri",
      runDpJson: (source) => invoke<string>("run_dp_json", { source }),
      runExactJson: (source) => invoke<string>("run_exact_json", { source }),
      runMcJson: (source, runs, seed) =>
        invoke<string>("run_mc_json", { source, runs, seed }),
      cancel: () => {},
    };
  }

  webBackend ??= new WebWorkerEngineBackend();
  return webBackend;
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
