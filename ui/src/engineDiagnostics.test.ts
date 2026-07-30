import { describe, expect, it } from "vitest";
import { parseEngineError } from "./engineDiagnostics";

describe("engine diagnostic presentation", () => {
  it("maps multiple engine diagnostics to Korean guidance and preserves the original", () => {
    const original = [
      "Error: W009: accumulator precompute table requires 800400 entries",
      "E010: accumulator precompute table requires 60001000 entries",
      "E011: approximate DP layer state count 1000001 exceeds limit 1000000",
    ].join("\n");
    const parsed = parseEngineError(original);
    expect(parsed.original).toBe(original);
    expect(parsed.diagnostics).toEqual([
      expect.objectContaining({ code: "W009", title: "큰 집계 변수 테이블" }),
      expect.objectContaining({ code: "E010", title: "집계 변수 테이블 한도 초과" }),
      expect.objectContaining({ code: "E011", title: "DP 실행 상태 한도 초과" }),
    ]);
  });

  it("keeps unknown diagnostic codes usable", () => {
    expect(parseEngineError("Error: E999: future engine detail").diagnostics).toEqual([{
      code: "E999",
      original: "future engine detail",
      title: undefined,
      fix: undefined,
    }]);
  });
});
