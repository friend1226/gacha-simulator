import { describe, expect, it } from "vitest";
import { normalizeFirstHit } from "./firstHit";

describe("first-hit result normalization", () => {
  it("turns MC occurrence counts into PMF, CDF, percentiles, and failure mass", () => {
    const result = normalizeFirstHit([0, 20, 30, 0], 100)!;
    expect(result.pmf).toEqual([0, 0.2, 0.3, 0]);
    expect(result.cdf).toEqual([0, 0.2, 0.5, 0.5]);
    expect(result.failureReachable).toBe(0.5);
    expect(result.mean).toBeCloseTo(1.6);
    expect(result.percentiles[0]).toEqual([0.5, 2]);
  });
});
