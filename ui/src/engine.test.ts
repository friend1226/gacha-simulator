import { describe, expect, it, vi } from "vitest";
import { runDpJson } from "./engine";
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
