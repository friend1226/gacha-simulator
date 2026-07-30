import { describe, expect, it } from "vitest";
import blueArchiveSource from "../../presets/blue-archive-pickup.json";
import type { ModelIr } from "./types";
import {
  initialProvenance,
  nextProvenance,
  serializeModelForExport,
  type ModelProvenance,
} from "./provenance";

describe("model provenance", () => {
  it("tracks every model entry and edit path", () => {
    expect(initialProvenance(false)).toBe("pristine");
    expect(initialProvenance(true)).toBe("none");

    let provenance: ModelProvenance = "none";
    provenance = nextProvenance(provenance, "loadPreset");
    expect(provenance).toBe("pristine");
    provenance = nextProvenance(provenance, "blockEdit");
    expect(provenance).toBe("dirty");

    provenance = nextProvenance(provenance, "loadPreset");
    provenance = nextProvenance(provenance, "applyJson");
    expect(provenance).toBe("dirty");

    provenance = nextProvenance(provenance, "loadPreset");
    provenance = nextProvenance(provenance, "updateModel");
    expect(provenance).toBe("dirty");

    provenance = nextProvenance(provenance, "openModel");
    expect(provenance).toBe("none");
    expect(nextProvenance(provenance, "blockEdit")).toBe("none");
    expect(nextProvenance("pristine", "restore")).toBe("none");
  });

  it("keeps preset metadata only for pristine exports", () => {
    const model = structuredClone(blueArchiveSource) as ModelIr;
    expect(JSON.parse(serializeModelForExport(model, "pristine")).$preset).toBeTruthy();
    expect(JSON.parse(serializeModelForExport(model, "dirty")).$preset).toBeUndefined();
    expect(JSON.parse(serializeModelForExport(model, "none")).$preset).toBeUndefined();
    expect((model as ModelIr & { $preset?: unknown }).$preset).toBeTruthy();
  });
});
