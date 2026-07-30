import { describe, expect, it } from "vitest";
import {
  defaultSettings,
  loadSettings,
  normalizeSettings,
  saveSettings,
} from "./settings";

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

  it("falls back when stored settings cannot be read or parsed", () => {
    expect(loadSettings({
      getItem: () => {
        throw new DOMException("blocked", "SecurityError");
      },
    })).toEqual(defaultSettings);
    expect(loadSettings({ getItem: () => "{broken" })).toEqual(defaultSettings);
  });

  it("keeps working when stored settings cannot be written", () => {
    expect(() => saveSettings(defaultSettings, {
      setItem: () => {
        throw new DOMException("full", "QuotaExceededError");
      },
    })).not.toThrow();
  });
});
