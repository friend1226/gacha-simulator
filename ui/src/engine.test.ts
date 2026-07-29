import { describe, expect, it, vi } from "vitest";
import { EngineCancelledError, runDpJson, WebWorkerEngineBackend } from "./engine";
import type { EngineWorkerRequest, EngineWorkerResponse } from "./engineWorkerProtocol";
import { blueArchive } from "./preset";

describe("DP backend selection", () => {
  it("calls the exact engine for exact numeric mode", async () => {
    const runDpJsonBackend = vi.fn(async () => "scaled");
    const runExactJson = vi.fn(async () => "exact");
    const model = {
      ...blueArchive,
      run: { ...blueArchive.run, numeric: "exact" as const },
    };

    await expect(runDpJson({
      runDpJson: runDpJsonBackend,
      runExactJson,
    }, model)).resolves.toEqual({
      engine: "EXACT",
      json: "exact",
    });
    expect(runExactJson).toHaveBeenCalledOnce();
    expect(runDpJsonBackend).not.toHaveBeenCalled();
  });

  it("keeps approximate modes on the generic DP engine", async () => {
    const runDpJsonBackend = vi.fn(async () => "scaled");
    const runExactJson = vi.fn(async () => "exact");

    await expect(runDpJson({
      runDpJson: runDpJsonBackend,
      runExactJson,
    }, blueArchive)).resolves.toEqual({
      engine: "DP",
      json: "scaled",
    });
    expect(runDpJsonBackend).toHaveBeenCalledOnce();
    expect(runExactJson).not.toHaveBeenCalled();
  });
});

describe("web worker backend", () => {
  it("routes requests and resolves matching worker responses", async () => {
    const worker = fakeWorker();
    const backend = new WebWorkerEngineBackend(() => worker);
    const onProgress = vi.fn();
    const result = backend.runMcJson("model", 100_000, 42, onProgress);

    expect(worker.messages).toEqual([{
      id: 1,
      method: "mc",
      source: "model",
      runs: 100_000,
      seed: 42,
    }]);
    worker.respond({ id: 1, progress: { completed: 10_000, total: 100_000 } });
    expect(onProgress).toHaveBeenCalledWith({ completed: 10_000, total: 100_000 });
    worker.respond({ id: 1, ok: true, json: "result" });
    await expect(result).resolves.toBe("result");
  });

  it("terminates on cancel and creates a fresh worker for the next run", async () => {
    const firstWorker = fakeWorker();
    const secondWorker = fakeWorker();
    const workers = [firstWorker, secondWorker];
    let workerIndex = 0;
    const backend = new WebWorkerEngineBackend(() => workers[workerIndex++]);
    const cancelled = backend.runDpJson("first");
    const cancelledAssertion = expect(cancelled).rejects.toBeInstanceOf(EngineCancelledError);

    backend.cancel();
    await cancelledAssertion;
    expect(firstWorker.terminated).toBe(true);
    expect(workerIndex).toBe(1);

    const rerun = backend.runExactJson("second");
    expect(workerIndex).toBe(2);
    secondWorker.respond({ id: 2, ok: true, json: "rerun" });
    await expect(rerun).resolves.toBe("rerun");
  });
});

function fakeWorker() {
  return {
    onmessage: null as ((event: MessageEvent<EngineWorkerResponse>) => void) | null,
    onerror: null as ((event: ErrorEvent) => void) | null,
    messages: [] as EngineWorkerRequest[],
    terminated: false,
    postMessage(message: EngineWorkerRequest) {
      this.messages.push(message);
    },
    terminate() {
      this.terminated = true;
    },
    respond(response: EngineWorkerResponse) {
      this.onmessage?.({ data: response } as MessageEvent<EngineWorkerResponse>);
    },
  };
}
