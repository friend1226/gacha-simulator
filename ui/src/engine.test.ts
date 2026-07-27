import { describe, expect, it, vi } from "vitest";
import { runDpJson } from "./engine";
import { blueArchive } from "./preset";

describe("DP backend selection", () => {
  it("calls the exact WASM engine for exact numeric mode", () => {
    const run_dp_json = vi.fn(() => "scaled");
    const run_exact_json = vi.fn(() => "exact");
    const model = {
      ...blueArchive,
      run: { ...blueArchive.run, numeric: "exact" as const },
    };

    expect(runDpJson({ run_dp_json, run_exact_json }, model)).toEqual({
      engine: "EXACT",
      json: "exact",
    });
    expect(run_exact_json).toHaveBeenCalledOnce();
    expect(run_dp_json).not.toHaveBeenCalled();
  });

  it("keeps approximate modes on the generic DP WASM engine", () => {
    const run_dp_json = vi.fn(() => "scaled");
    const run_exact_json = vi.fn(() => "exact");

    expect(runDpJson({ run_dp_json, run_exact_json }, blueArchive)).toEqual({
      engine: "DP",
      json: "scaled",
    });
    expect(run_dp_json).toHaveBeenCalledOnce();
    expect(run_exact_json).not.toHaveBeenCalled();
  });
});
