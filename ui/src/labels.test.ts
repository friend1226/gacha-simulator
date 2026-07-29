import { describe, expect, it } from "vitest";
import { diagnosticHelp } from "./labels";

describe("diagnostic labels", () => {
  it("has Korean guidance for every core diagnostic code", () => {
    for (const code of [
      "E000", "E001", "E002", "E003", "E004", "E005", "E006", "E007", "E008", "E009", "E010",
      "W001", "W002", "W003", "W004", "W005", "W006", "W007", "W008", "W009",
    ]) {
      expect(diagnosticHelp[code]?.title, code).toBeTruthy();
      expect(diagnosticHelp[code]?.fix, code).toBeTruthy();
    }
  });
});
