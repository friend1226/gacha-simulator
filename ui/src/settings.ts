export type ProbabilityFormat = "scientific" | "decimal" | "percent" | "reciprocal";

export interface AppSettings {
  version: 1;
  soundVolume: number;
  numeric: "f64" | "scaled" | "exact";
  mcRuns: number;
  mcSeed: number;
  probabilityFormat: ProbabilityFormat;
  maxRows: number;
  theme: "dark";
}

export const defaultSettings: AppSettings = {
  version: 1,
  soundVolume: 0.5,
  numeric: "scaled",
  mcRuns: 100_000,
  mcSeed: 42,
  probabilityFormat: "scientific",
  maxRows: 500,
  theme: "dark",
};

const STORAGE_KEY = "gacha-lab.settings.v1";

export function normalizeSettings(value: unknown): AppSettings {
  if (!value || typeof value !== "object" || (value as { version?: number }).version !== 1) {
    return { ...defaultSettings };
  }
  const candidate = value as Partial<AppSettings>;
  return {
    ...defaultSettings,
    ...candidate,
    version: 1,
    soundVolume: Math.max(0, Math.min(1, Number(candidate.soundVolume ?? 0.5))),
    mcRuns: Math.max(1, Math.floor(Number(candidate.mcRuns ?? 100_000))),
    mcSeed: Math.max(0, Math.floor(Number(candidate.mcSeed ?? 42))),
    maxRows: Math.max(10, Math.min(10_000, Math.floor(Number(candidate.maxRows ?? 500)))),
  };
}

export function loadSettings(storage: Pick<Storage, "getItem"> = localStorage): AppSettings {
  try {
    const source = storage.getItem(STORAGE_KEY);
    return source ? normalizeSettings(JSON.parse(source)) : { ...defaultSettings };
  } catch {
    return { ...defaultSettings };
  }
}

export function saveSettings(
  settings: AppSettings,
  storage: Pick<Storage, "setItem"> = localStorage,
) {
  try {
    storage.setItem(STORAGE_KEY, JSON.stringify(normalizeSettings(settings)));
  } catch {
    // Keep in-memory settings usable when persistent storage is unavailable.
  }
}

export function formatProbability(value: number, format: ProbabilityFormat): string {
  if (!Number.isFinite(value)) return "—";
  if (value === 0) return "0";
  if (format === "decimal") return value.toPrecision(12);
  if (format === "percent") return `${(value * 100).toPrecision(10)}%`;
  if (format === "reciprocal") return value > 0 ? `1 / ${(1 / value).toPrecision(8)}` : "—";
  return value.toExponential(10);
}
