import { describe, expect, it } from "vitest";
import { blueArchive } from "./preset";
import { parseExactLiteral, validateLocally } from "./validator";

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

