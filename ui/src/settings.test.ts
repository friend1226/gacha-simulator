import { describe, expect, it } from "vitest";
import { defaultSettings, normalizeSettings } from "./settings";

describe("settings", () => {
  it("rejects unknown schema versions", () => {
    expect(normalizeSettings({ version: 99, mcRuns: 1 })).toEqual(defaultSettings);
  });

  it("clamps unsafe persisted values", () => {
    const settings = normalizeSettings({
      ...defaultSettings,
      soundVolume: 4,
      mcRuns: 0,
      maxRows: 999_999,
    });
    expect(settings.soundVolume).toBe(1);
    expect(settings.mcRuns).toBe(1);
    expect(settings.maxRows).toBe(10_000);
  });
});
