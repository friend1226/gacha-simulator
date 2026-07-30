/// <reference types="vite/client" />

import { describe, expect, it } from "vitest";
import {
  Blockly,
  blockToExpr,
  exprToBlock,
  getUnsupportedBlockItems,
  loadIr,
  toolbox,
  workspaceToIr,
} from "./blockly";
import type {
  BooleanExpr,
  Expr,
  ModelIr,
  NumberExpr,
  ProbabilityRule,
} from "./types";

const presetModules = import.meta.glob("../../presets/*.json", {
  eager: true,
  import: "default",
}) as Record<string, ModelIr>;
const blueArchivePreset = presetByFile("blue-archive-pickup.json");
const arknightsPreset = presetByFile("arknights-first-ten-guarantee.json");

describe("Blockly IR round trip", () => {
  for (const [path, preset] of Object.entries(presetModules)) {
    it(`preserves every rule in ${path}`, () => {
      const workspace = new Blockly.Workspace();
      try {
        expect(loadIr(workspace, preset)).toEqual([]);
        expect(normalizeForRoundTrip(workspaceToIr(workspace, preset))).toEqual(normalizeForRoundTrip(preset));
      } finally {
        workspace.dispose();
      }
    });
  }

  it("keeps unsupported rules while supported blocks are edited", () => {
    const model = structuredClone(blueArchivePreset);
    model.stateVars.push({ id: "legacyStat", init: 0, max: 5, role: "stat" });
    model.run.condition = { not: { ge: [{ var: "nPickup" }, { lit: "1" }] } };
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

  it("preserves a future probability expression that has no block representation", () => {
    const model = structuredClone(blueArchivePreset);
    const futureRule = {
      target: model.entities[0].id,
      expr: { mod: [{ trial: true }, { lit: "10" }] },
    } as unknown as ProbabilityRule;
    model.probRules = [futureRule];
    const workspace = new Blockly.Workspace();
    try {
      expect(loadIr(workspace, model).map((item) => item.path)).toEqual(["probRules[0]"]);
      expect(normalizeForRoundTrip(workspaceToIr(workspace, model))).toEqual(normalizeForRoundTrip(model));
    } finally {
      workspace.dispose();
    }
  });

  it("round-trips a general entity probability through the value socket", () => {
    const model = structuredClone(blueArchivePreset);
    model.stateVars.push({ id: "rateUp", init: 0, max: 1, role: "control" });
    model.entities[0].prob = {
      if: { eq: [{ var: "rateUp" }, { lit: "1" }] },
      then: { min: [{ lit: "1/10" }, { lit: "0.03" }] },
      else: { lit: "3e-2" },
    };
    const workspace = new Blockly.Workspace();
    try {
      expect(loadIr(workspace, model)).toEqual([]);
      expect(workspace.getBlocksByType("expr_if", false)).toHaveLength(1);
      expect(normalizeForRoundTrip(workspaceToIr(workspace, model)))
        .toEqual(normalizeForRoundTrip(model));
    } finally {
      workspace.dispose();
    }
  });

  it("preserves an entity with a future probability expression", () => {
    const model = structuredClone(blueArchivePreset);
    model.entities[0].prob = {
      mod: [{ trial: true }, { lit: "10" }],
    } as unknown as ModelIr["entities"][number]["prob"];
    const workspace = new Blockly.Workspace();
    try {
      expect(loadIr(workspace, model).map((item) => item.path)).toEqual(["entities[0]"]);
      expect(normalizeForRoundTrip(workspaceToIr(workspace, model)))
        .toEqual(normalizeForRoundTrip(model));
    } finally {
      workspace.dispose();
    }
  });

  it("represents the Arknights first-ten guarantee without unsupported items", () => {
    const workspace = new Blockly.Workspace();
    try {
      expect(loadIr(workspace, arknightsPreset)).toEqual([]);
      expect(workspace.getBlocksByType("probability_rule", false)).toHaveLength(4);
      expect(workspace.getBlocksByType("transition_or_set", false)).toHaveLength(1);
      expect(normalizeForRoundTrip(workspaceToIr(workspace, arknightsPreset)))
        .toEqual(normalizeForRoundTrip(arknightsPreset));
    } finally {
      workspace.dispose();
    }
  });

  it("removes a preserved condition warning after a block condition replaces it", () => {
    const model = structuredClone(blueArchivePreset);
    model.run.condition = { not: { ge: [{ var: "nPickup" }, { lit: "1" }] } };
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

describe("Blockly expression mapping", () => {
  const one = (): NumberExpr => ({ lit: "1" });
  const two = (): NumberExpr => ({ lit: "2" });
  const truth = (): BooleanExpr => ({ ge: [one(), two()] });
  const falsity = (): BooleanExpr => ({ lt: [one(), two()] });
  const expressions: Expr[] = [
    { lit: "0.007" },
    { lit: "1/3" },
    { lit: "3e-5" },
    { var: "pity" },
    { trial: true },
    { add: [one(), two()] },
    { sub: [one(), two()] },
    { mul: [one(), two()] },
    { div: [one(), two()] },
    { neg: one() },
    { abs: one() },
    { floor: one() },
    { ceil: one() },
    { round: one() },
    { min: [one(), two()] },
    { max: [one(), two()] },
    { clamp: [one(), { lit: "0" }, two()] },
    { pow: [two(), { lit: "3" }] },
    { eq: [one(), two()] },
    { ne: [one(), two()] },
    { lt: [one(), two()] },
    { le: [one(), two()] },
    { gt: [one(), two()] },
    { ge: [one(), two()] },
    { and: [truth(), falsity()] },
    { or: [truth(), falsity()] },
    { not: truth() },
    { xor: [truth(), falsity()] },
    {
      if: { and: [truth(), { not: falsity() }] },
      then: {
        if: { eq: [{ var: "pity" }, { lit: "10" }] },
        then: { lit: "1/3" },
        else: { trial: true },
      },
      else: { clamp: [{ var: "pity" }, { lit: "0" }, { lit: "89" }] },
    },
  ];

  for (const expression of expressions) {
    const name = Object.keys(expression)[0];
    it(`round-trips ${name}`, () => {
      const workspace = new Blockly.Workspace();
      try {
        const block = exprToBlock(workspace, expression);
        expect(block, JSON.stringify(expression)).toBeDefined();
        expect(blockToExpr(block!)).toEqual(expression);
      } finally {
        workspace.dispose();
      }
    });
  }

  it("covers every operator accepted by expr.rs", () => {
    const atomicKeys = new Set(["lit", "var", "trial", "if"]);
    const covered = expressions
      .map((expression) => Object.keys(expression)[0])
      .filter((key) => !atomicKeys.has(key))
      .sort();
    expect(covered).toEqual([
      "abs", "add", "and", "ceil", "clamp", "div", "eq", "floor", "ge", "gt",
      "le", "lt", "max", "min", "mul", "ne", "neg", "not", "or", "pow", "round",
      "sub", "xor",
    ]);
  });

  it("uses typed Number and Boolean connections", () => {
    const workspace = new Blockly.Workspace();
    try {
      const arithmetic = workspace.newBlock("expr_arithmetic");
      const comparison = workspace.newBlock("expr_compare");
      const conditional = workspace.newBlock("expr_if");
      expect(arithmetic.outputConnection?.getCheck()).toEqual(["Number"]);
      expect(arithmetic.getInput("LEFT")?.connection?.getCheck()).toEqual(["Number"]);
      expect(comparison.outputConnection?.getCheck()).toEqual(["Boolean"]);
      expect(conditional.getInput("IF")?.connection?.getCheck()).toEqual(["Boolean"]);
      expect(conditional.getInput("THEN")?.connection?.getCheck()).toEqual(["Number"]);
    } finally {
      workspace.dispose();
    }
  });

  it("keeps pow exponent as an integer field instead of an expression socket", () => {
    const workspace = new Blockly.Workspace();
    try {
      const power = workspace.newBlock("expr_pow");
      expect(power.getInput("BASE")).toBeDefined();
      expect(power.getInput("EXPONENT")).toBeNull();
      expect(power.getField("EXPONENT")).toBeDefined();
      expect(exprToBlock(
        workspace,
        { pow: [{ lit: "2" }, { var: "exponent" }] } as unknown as Expr,
      )).toBeUndefined();
    } finally {
      workspace.dispose();
    }
  });

  it("replaces fixed probability templates with general blocks and toolbox prefills", () => {
    const contents = (toolbox as Blockly.utils.toolbox.ToolboxInfo).contents;
    const probabilityCategory = contents.find((item) => (
      "name" in item && item.name === "확률 규칙"
    ));
    expect(probabilityCategory && "contents" in probabilityCategory).toBe(true);
    const probabilityBlocks = probabilityCategory && "contents" in probabilityCategory
      ? probabilityCategory.contents
        .filter((entry): entry is Blockly.utils.toolbox.BlockInfo => "type" in entry)
      : [];
    const blockTypes = contents.flatMap((item) => (
      "contents" in item
        ? item.contents
          .filter((entry): entry is Blockly.utils.toolbox.BlockInfo => "type" in entry)
          .map((entry) => entry.type)
        : []
    ));
    expect(blockTypes.filter((type) => type === "probability_rule")).toHaveLength(3);
    expect(blockTypes).not.toContain("soft_pity_rule");
    expect(blockTypes).not.toContain("trial_state_prob_rule");

    const workspace = new Blockly.Workspace();
    try {
      const prefilledExpressions = probabilityBlocks.slice(1).map(({ kind: _kind, ...state }) => {
        const block = Blockly.serialization.blocks.append(
          state as Blockly.serialization.blocks.State,
          workspace,
        );
        return blockToExpr(block.getInputTargetBlock("EXPR")!);
      });
      expect(prefilledExpressions).toEqual([
        {
          if: { ge: [{ var: "pity" }, { lit: "65" }] },
          then: {
            add: [
              { lit: "0.03" },
              {
                mul: [
                  { lit: "0.03" },
                  { sub: [{ var: "pity" }, { lit: "65" }] },
                ],
              },
            ],
          },
          else: { lit: "0.03" },
        },
        {
          if: {
            and: [
              { eq: [{ trial: true }, { lit: "10" }] },
              { eq: [{ var: "highSeen" }, { lit: "0" }] },
            ],
          },
          then: { lit: "0.98" },
          else: { lit: "0.08" },
        },
      ]);
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

function presetByFile(filename: string): ModelIr {
  const preset = Object.entries(presetModules)
    .find(([path]) => path.endsWith(`/${filename}`))
    ?.[1];
  if (!preset) throw new Error(`missing preset fixture: ${filename}`);
  return preset;
}
