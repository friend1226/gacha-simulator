import { describe, expect, it } from "vitest";
import { parseEngineError } from "./engineDiagnostics";
import { confidenceLabel, confidenceLabels, diagnosticHelp } from "./labels";

const presetModules = import.meta.glob("../../presets/*.json", {
  eager: true,
  import: "default",
}) as Record<string, { $preset: { confidence: string } }>;

describe("diagnostic labels", () => {
  it("has Korean guidance for every core diagnostic code", () => {
    const coreCodes = [
      "E000", "E001", "E002", "E003", "E004", "E005", "E006", "E007", "E008", "E009", "E010", "E011", "E012",
      "W001", "W002", "W003", "W004", "W005", "W006", "W007", "W008", "W009", "W010",
    ];
    for (const code of coreCodes) {
      expect(diagnosticHelp[code]?.title, code).toBeTruthy();
      expect(diagnosticHelp[code]?.fix, code).toBeTruthy();
    }
    const parsed = parseEngineError(coreCodes.map((code) => `${code}: engine detail`).join("\n"));
    expect(parsed.diagnostics.map((item) => item.code)).toEqual(coreCodes);
    expect(parsed.diagnostics.every((item) => item.title && item.fix)).toBe(true);
  });

  it("explains every probability-table axis in W010 and E012 guidance", () => {
    for (const code of ["W010", "E012"]) {
      expect(diagnosticHelp[code].fix).toContain("시행 횟수");
      expect(diagnosticHelp[code].fix).toContain("뽑기 결과 수");
      expect(diagnosticHelp[code].fix).toContain("도달 가능한 제어 상태");
    }
  });

  it("explains both sides of E008 assignments", () => {
    expect(diagnosticHelp.E008.fix).toContain("대입 대상");
    expect(diagnosticHelp.E008.fix).toContain("우변 참조");
    expect(diagnosticHelp.E008.fix).toContain("제어 변수");
  });
});

describe("confidence labels", () => {
  it("has a Korean label for every confidence value used by presets", () => {
    for (const [path, preset] of Object.entries(presetModules)) {
      const confidence = preset.$preset.confidence;
      expect(confidenceLabels[confidence], `${path}: ${confidence}`).toBeTruthy();
    }
  });

  it("covers every confidence value defined by DESIGN 10.4", () => {
    expect(confidenceLabels).toEqual({
      official: "공식 공시",
      datamined: "데이터마이닝",
      "community-estimate": "커뮤니티 추정",
    });
    expect(confidenceLabel("future-source")).toBe("future-source");
  });
});
