import arknightsFirstTenSource from "../../presets/arknights-first-ten-guarantee.json";
import blueArchiveSource from "../../presets/blue-archive-pickup.json";
import simplePitySource from "../../presets/simple-pity.json";
import type { ModelIr } from "./types";

export interface PresetEntry {
  id: string;
  model: ModelIr;
  meta: {
    game: string;
    banner: string;
    sourceUrl: string;
    verifiedDate: string;
    confidence: string;
    notes: string;
  };
}

function entry(id: string, source: unknown): PresetEntry {
  const value = source as ModelIr & { $preset: PresetEntry["meta"] };
  return { id, model: value, meta: value.$preset };
}

export const presets = [
  entry("blue-archive-pickup", blueArchiveSource),
  entry("arknights-first-ten-guarantee", arknightsFirstTenSource),
  entry("simple-pity", simplePitySource),
];

export const blueArchive = presets[0].model;
