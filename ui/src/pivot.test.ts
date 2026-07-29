import { describe, expect, it } from "vitest";
import { pivot } from "./pivot";

describe("pivot", () => {
  it("preserves probability mass while marginalizing", () => {
    const table = pivot([
      { counts: [0, 0, 1], probability: 0.1 },
      { counts: [0, 1, 2], probability: 0.2 },
      { counts: [1, 0, 3], probability: 0.3 },
      { counts: [1, 1, 4], probability: 0.4 },
    ], [
      { key: "a", label: "A", role: "row" },
      { key: "b", label: "B", role: "sum" },
      { key: "c", label: "C", role: "sum" },
    ]);
    expect(table.total).toBeCloseTo(1);
    expect(table.cells).toEqual([
      { row: 0, col: 0, probability: 0.30000000000000004, display: "3.00000000000e-1" },
      { row: 1, col: 0, probability: 0.7, display: "7.00000000000e-1" },
    ]);
  });

  it("filters a slice before aggregating", () => {
    const table = pivot([
      { counts: [0, 1], probability: 0.25 },
      { counts: [1, 1], probability: 0.75 },
    ], [
      { key: "a", label: "A", role: "filter", filterValue: 1 },
      { key: "b", label: "B", role: "row" },
    ]);
    expect(table.total).toBe(0.75);
  });

  it("preserves and aggregates probabilities below the f64 range", () => {
    const single = pivot([
      { counts: [200], probability: 0, display: "1.04618382913e-431" },
    ], [{ key: "hit", label: "Hit", role: "row" }]);
    expect(single.cells[0].probability).toBe(0);
    expect(single.cells[0].display).toBe("1.04618382913e-431");

    const aggregated = pivot([
      { counts: [200, 0], probability: 0, display: "1.04618382913e-431" },
      { counts: [200, 1], probability: 0, display: "2.00000000000e-431" },
    ], [
      { key: "hit", label: "Hit", role: "row" },
      { key: "extra", label: "Extra", role: "sum" },
    ]);
    expect(aggregated.cells[0].display).toBe("3.04618382913e-431");

    const exact = pivot([{
      counts: [200],
      probability: 0,
      numerator: (7n ** 200n).toString(),
      denominator: (1000n ** 200n).toString(),
    }], [{ key: "hit", label: "Hit", role: "row" }]);
    expect(exact.cells[0].display).toBe("1.04618382913e-431");
  });
});
