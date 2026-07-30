import { describe, expect, it } from "vitest";
import { blueArchive, presets } from "./preset";
import { parseExactLiteral, validateLocally } from "./validator";
import type { ModelIr } from "./types";

describe("exact literal parser", () => {
  it("does not round-trip through Number", () => {
    expect(parseExactLiteral("0.007")).toEqual({ numerator: 7n, denominator: 1000n });
    expect(parseExactLiteral("3e-5")).toEqual({ numerator: 3n, denominator: 100000n });
    expect(parseExactLiteral("1/3")).toEqual({ numerator: 1n, denominator: 3n });
  });
});

describe("blue archive preset", () => {
  it("splits the entity tree into exclusive leaves", () => {
    const result = validateLocally(blueArchive);
    expect(result.leaves.map((leaf) => [leaf.id, leaf.probability])).toEqual([
      ["pickup", 0.007],
      ["star3__self", 0.023],
      ["__other__", 0.97],
    ]);
  });
});

describe("accumulator table preflight", () => {
  it("warns in the safe range and reports only E010 above the hard limit", () => {
    const warning = validateLocally(largeAccumulatorModel(2_000, 200)).diagnostics;
    expect(warning.find((item) => item.code === "W009")?.message).toContain("800,400");
    expect(warning.some((item) => item.code === "E010")).toBe(false);

    const error = validateLocally(largeAccumulatorModel(60_000, 500)).diagnostics;
    expect(error.find((item) => item.code === "E010")?.message).toContain("60,001,000");
    expect(error.some((item) => item.code === "W009")).toBe(false);
  });
});

describe("DP state-space preflight", () => {
  it("warns above the runtime layer limit without blocking execution", () => {
    const diagnostics = validateLocally(largeDpModel(180)).diagnostics;
    expect(diagnostics.find((item) => item.code === "W004")).toEqual(expect.objectContaining({
      severity: "warning",
      message: expect.stringContaining("1,000,000"),
    }));
    expect(diagnostics.some((item) => item.code === "E011")).toBe(false);
  });

  it("blocks execution above the core estimated-state hard limit", () => {
    const diagnostics = validateLocally(largeDpModel(500)).diagnostics;
    expect(diagnostics.find((item) => item.code === "E011")).toEqual(expect.objectContaining({
      severity: "error",
      message: expect.stringContaining("50,000,000"),
    }));
    expect(diagnostics.some((item) => item.code === "W004")).toBe(false);
  });
});

describe("preset DP preflight", () => {
  it("keeps every bundled preset below the W004 warning threshold", () => {
    for (const preset of presets) {
      const diagnostics = validateLocally(preset.model).diagnostics;
      expect(
        diagnostics.some((item) => item.code === "W004" || item.code === "E011"),
        preset.id,
      ).toBe(false);
    }
  });
});

function largeDpModel(maxTrials: number): ModelIr {
  return {
    irVersion: 2,
    name: "large DP",
    entities: [
      { id: "a", name: "A", prob: { lit: "1/4" } },
      { id: "b", name: "B", prob: { lit: "1/4" } },
      { id: "c", name: "C", prob: { lit: "1/4" } },
      { id: "d", name: "D", prob: { lit: "1/4" } },
    ],
    nestingPolicy: "clampChildren",
    stateVars: [],
    probRules: [],
    transitions: [],
    triggers: [],
    run: {
      maxTrials,
      trackJoint: ["a", "b", "c", "d"],
      numeric: "scaled",
      trialSeries: "none",
    },
  };
}

function largeAccumulatorModel(max: number, maxTrials: number): ModelIr {
  return {
    irVersion: 2,
    name: "large accumulator table",
    entities: [{ id: "hit", name: "당첨", prob: { lit: "1/2" } }],
    nestingPolicy: "clampChildren",
    stateVars: [{
      id: "seen",
      init: 0,
      max,
      role: "accumulator",
      update: [{ when: { leafOf: "hit" }, set: { trial: true } }],
    }],
    probRules: [],
    transitions: [],
    triggers: [],
    run: { maxTrials, trackJoint: ["seen"], numeric: "scaled", trialSeries: "none" },
  };
}

