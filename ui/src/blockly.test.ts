/// <reference types="vite/client" />

import { describe, expect, it } from "vitest";
import { Blockly, getUnsupportedBlockItems, loadIr, workspaceToIr } from "./blockly";
import type { ModelIr } from "./types";

const presetModules = import.meta.glob("../../presets/*.json", {
  eager: true,
  import: "default",
}) as Record<string, ModelIr>;

describe("Blockly IR round trip", () => {
  for (const [path, preset] of Object.entries(presetModules)) {
    it(`preserves every rule in ${path}`, () => {
      const workspace = new Blockly.Workspace();
      try {
        loadIr(workspace, preset);
        expect(normalizeForRoundTrip(workspaceToIr(workspace, preset))).toEqual(normalizeForRoundTrip(preset));
      } finally {
        workspace.dispose();
      }
    });
  }

  it("keeps unsupported rules while supported blocks are edited", () => {
    const model = structuredClone(Object.values(presetModules)[0]);
    model.stateVars.push({ id: "legacyStat", init: 0, max: 5, role: "stat" });
    model.run.condition = { or: [{ ge: [{ var: "nPickup" }, { lit: "1" }] }] };
    const workspace = new Blockly.Workspace();
    try {
      expect(loadIr(workspace, model).map((item) => item.path)).toEqual([
        `stateVars[${model.stateVars.length - 1}]`,
        "run.condition",
      ]);
      const entity = workspace.getBlocksByType("entity_definition", false)[0];
      entity.setFieldValue("수정된 이름", "NAME");
      const roundTrip = workspaceToIr(workspace, model);
      expect(roundTrip.entities[0].name).toBe("수정된 이름");
      expect(roundTrip.stateVars).toContainEqual(model.stateVars.at(-1));
      expect(roundTrip.run.condition).toEqual(model.run.condition);
    } finally {
      workspace.dispose();
    }
  });

  it("preserves a general probability rule that has no block representation", () => {
    const model = structuredClone(Object.values(presetModules)[0]);
    model.probRules = [{
      target: model.entities[0].id,
      expr: {
        if: { eq: [{ trial: true }, { lit: "10" }] },
        then: { lit: "1" },
        else: { lit: "0.1" },
      },
    }];
    const workspace = new Blockly.Workspace();
    try {
      expect(loadIr(workspace, model).map((item) => item.path)).toEqual(["probRules[0]"]);
      expect(normalizeForRoundTrip(workspaceToIr(workspace, model))).toEqual(normalizeForRoundTrip(model));
    } finally {
      workspace.dispose();
    }
  });

  it("removes a preserved condition warning after a block condition replaces it", () => {
    const model = structuredClone(Object.values(presetModules)[0]);
    model.run.condition = { or: [{ ge: [{ var: "nPickup" }, { lit: "1" }] }] };
    const workspace = new Blockly.Workspace();
    try {
      expect(loadIr(workspace, model).map((item) => item.path)).toEqual(["run.condition"]);
      const root = workspace.getBlocksByType("model_container", false)[0];
      const condition = workspace.newBlock("first_hit_condition");
      condition.setFieldValue("pickup", "ENTITY");
      condition.setFieldValue(2, "COUNT");
      root.getInput("CONDITION")?.connection?.connect(condition.previousConnection!);

      const roundTrip = workspaceToIr(workspace, model);

      expect(roundTrip.run.condition).toEqual({
        ge: [{ var: "nPickup" }, { lit: "2" }],
      });
      expect(getUnsupportedBlockItems(workspace)).toEqual([]);
    } finally {
      workspace.dispose();
    }
  });

  it("normalizes omitted trigger grant defaults for structural comparison", () => {
    const preset = Object.values(presetModules).find((candidate) => candidate.triggers.length > 0);
    expect(preset).toBeDefined();
    const model = structuredClone(preset!);
    const grant = (model.triggers[0] as {
      grant: { consumesTrial?: boolean; appliesTransitions?: boolean };
    }).grant;
    delete grant.consumesTrial;
    delete grant.appliesTransitions;
    const workspace = new Blockly.Workspace();
    try {
      loadIr(workspace, model);
      expect(normalizeForRoundTrip(workspaceToIr(workspace, model))).toEqual(normalizeForRoundTrip(model));
    } finally {
      workspace.dispose();
    }
  });

  it("round-trips non-default trigger grant flags", () => {
    const preset = Object.values(presetModules).find((candidate) => candidate.triggers.length > 0);
    expect(preset).toBeDefined();
    const model = structuredClone(preset!);
    const trigger = model.triggers[0] as {
      grant: { consumesTrial: boolean; appliesTransitions: boolean };
    };
    trigger.grant.consumesTrial = true;
    trigger.grant.appliesTransitions = false;
    const workspace = new Blockly.Workspace();
    try {
      expect(loadIr(workspace, model)).toEqual([]);
      expect(stripBlockIds(workspaceToIr(workspace, model))).toEqual(stripBlockIds(model));
    } finally {
      workspace.dispose();
    }
  });
});

function stripBlockIds(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(stripBlockIds);
  if (!value || typeof value !== "object") return value;
  return Object.fromEntries(Object.entries(value)
    .filter(([key]) => key !== "blockId")
    .map(([key, item]) => [key, stripBlockIds(item)]));
}

function normalizeForRoundTrip(model: ModelIr): unknown {
  const normalized = structuredClone(model);
  for (const trigger of normalized.triggers) {
    if (!trigger || typeof trigger !== "object") continue;
    const grant = (trigger as { grant?: Record<string, unknown> }).grant;
    if (!grant) continue;
    grant.amount ??= 1;
    grant.consumesTrial ??= false;
    grant.appliesTransitions ??= true;
  }
  return stripBlockIds(normalized);
}
