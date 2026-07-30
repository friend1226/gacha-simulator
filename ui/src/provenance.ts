import type { ModelIr } from "./types";

export type ModelProvenance = "pristine" | "dirty" | "none";
export type ProvenanceEvent =
  | "loadPreset"
  | "blockEdit"
  | "applyJson"
  | "updateModel"
  | "openModel"
  | "restore";

export function initialProvenance(hasStoredModel: boolean): ModelProvenance {
  return hasStoredModel ? "none" : "pristine";
}

export function nextProvenance(
  current: ModelProvenance,
  event: ProvenanceEvent,
): ModelProvenance {
  if (event === "loadPreset") return "pristine";
  if (event === "openModel" || event === "restore") return "none";
  return current === "none" ? "none" : "dirty";
}

export function serializeModelForExport(
  model: ModelIr,
  provenance: ModelProvenance,
): string {
  const exported = structuredClone(model) as ModelIr & { $preset?: unknown };
  if (provenance !== "pristine") delete exported.$preset;
  return JSON.stringify(exported, null, 2);
}
