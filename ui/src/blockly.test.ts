/// <reference types="vite/client" />

import { describe, expect, it } from "vitest";
import {
  ACCUMULATOR_VARIABLE_TYPE,
  Blockly,
  CONTROL_VARIABLE_TYPE,
  GachaConnectionChecker,
  blockToExpr,
  encodeEntityCountReference,
  exprToBlock,
  getUnsupportedBlockItems,
  listWorkspaceVariables,
  loadIr,
  refreshVariableToolbox,
  saveWorkspaceVariable,
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
    model.run.condition = { not: { ge: [{ var: "nMissing" }, { lit: "1" }] } };
    const workspace = new Blockly.Workspace();
    try {
      expect(loadIr(workspace, model).map((item) => item.path)).toEqual(["run.condition"]);
      const root = workspace.getBlocksByType("model_container", false)[0];
      const condition = workspace.newBlock("condition_expression");
      const comparison = workspace.newBlock("expr_compare");
      const count = workspace.newBlock("expr_entity_count");
      const threshold = workspace.newBlock("expr_literal");
      comparison.setFieldValue("ge", "OP");
      count.setFieldValue("pickup", "ENTITY");
      threshold.setFieldValue("2", "VALUE");
      comparison.getInput("LEFT")?.connection?.connect(count.outputConnection!);
      comparison.getInput("RIGHT")?.connection?.connect(threshold.outputConnection!);
      condition.getInput("EXPR")?.connection?.connect(comparison.outputConnection!);
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

describe("Blockly variable menu and expression contexts", () => {
  it("uses Blockly variables for ID rename and keeps metadata in sync", () => {
    const workspace = checkedWorkspace();
    try {
      const created = saveWorkspaceVariable(workspace, {
        id: "rate",
        role: "control",
        init: 1,
        max: 3,
      });
      const reference = exprToBlock(workspace, { var: "rate" });
      expect(reference).toBeDefined();

      const renamed = saveWorkspaceVariable(workspace, {
        id: "renamedRate",
        name: "표시 확률",
        role: "control",
        init: 1,
        max: 3,
        clampPolicy: "error",
      }, created.variableId);

      expect(blockToExpr(reference!)).toEqual({ var: "renamedRate" });
      expect(listWorkspaceVariables(workspace)).toEqual([renamed]);
    } finally {
      workspace.dispose();
    }
  });

  it("removes declaration blocks and exposes a dynamic variable category", () => {
    const contents = (toolbox as Blockly.utils.toolbox.ToolboxInfo).contents;
    const variableCategory = contents.find((item) => "name" in item && item.name === "변수");
    expect(variableCategory && "custom" in variableCategory && variableCategory.custom).toBe("GACHA_VARIABLES");
    const blockTypes = contents.flatMap((item) => (
      "contents" in item
        ? item.contents.flatMap((entry) => "type" in entry ? [entry.type] : [])
        : []
    ));
    expect(blockTypes).not.toContain("control_variable");
    expect(blockTypes).not.toContain("accumulator_variable");
  });

  it("enables variable-dependent toolbox blocks only after a control exists", () => {
    const workspace = checkedWorkspace();
    let updated: Blockly.utils.toolbox.ToolboxInfo | undefined;
    (workspace as Blockly.WorkspaceSvg).updateToolbox = (definition) => {
      updated = definition as Blockly.utils.toolbox.ToolboxInfo;
    };
    try {
      refreshVariableToolbox(workspace as Blockly.WorkspaceSvg);
      expect(toolboxBlocks(updated!, "결과 변화").every((block) => block.enabled === false))
        .toBe(true);
      expect(toolboxBlocks(updated!, "시행 이벤트")
        .find((block) => block.type === "set_trigger")?.enabled).toBe(false);
      expect(toolboxBlocks(updated!, "확률 규칙").slice(1)
        .every((block) => block.enabled === false)).toBe(true);

      const control = saveWorkspaceVariable(workspace, {
        id: "rate", role: "control", init: 0, max: 2,
      });
      refreshVariableToolbox(workspace as Blockly.WorkspaceSvg);
      const transitions = toolboxBlocks(updated!, "결과 변화");
      expect(transitions.every((transition) => transition.enabled === true)).toBe(true);
      expect(transitions.map((transition) => (
        transition.fields?.VAR as { id: string }
      ).id)).toEqual([control.variableId, control.variableId]);
    } finally {
      workspace.dispose();
    }
  });

  it("round-trips accumulator self + control + trial expressions and clampPolicy error", () => {
    const model = structuredClone(blueArchivePreset);
    model.stateVars = [
      { id: "boost", init: 1, max: 1, role: "control" },
      {
        id: "spent",
        name: "소모",
        init: 0,
        max: 999,
        role: "accumulator",
        clampPolicy: "error",
        update: [{
          when: { leafOf: "pickup" },
          set: {
            add: [
              { var: "spent" },
              {
                if: { ge: [{ trial: true }, { lit: "10" }] },
                then: { var: "boost" },
                else: { lit: "1" },
              },
            ],
          },
        }],
      },
    ];
    model.probRules = [];
    model.transitions = [];
    const workspace = checkedWorkspace();
    try {
      expect(loadIr(workspace, model)).toEqual([]);
      expect(workspace.getBlocksByType("accumulator_update", false)).toHaveLength(1);
      expect(normalizeForRoundTrip(workspaceToIr(workspace, model)))
        .toEqual(normalizeForRoundTrip(model));
    } finally {
      workspace.dispose();
    }
  });

  it("does not leak field defaults into stateVars while loading setter blocks", () => {
    const model = minimalModel("hit");
    model.stateVars = [
      { id: "rate", init: 0, max: 2, role: "control" },
      {
        id: "total", init: 0, max: 10, role: "accumulator",
        update: [{ when: { leafOf: "hit" }, set: { add: [{ var: "total" }, { lit: "1" }] } }],
      },
    ];
    model.transitions = [{ when: { leafOf: "hit" }, set: { rate: { lit: "1" } } }];
    model.triggers = [{ at: { trialCount: 2 }, set: { rate: { lit: "2" } } }];
    const workspace = checkedWorkspace();
    try {
      expect(loadIr(workspace, model)).toEqual([]);
      expect(listWorkspaceVariables(workspace).map((variable) => variable.id)).toEqual([
        "rate",
        "total",
      ]);
      expect(normalizeForRoundTrip(workspaceToIr(workspace, model)))
        .toEqual(normalizeForRoundTrip(model));
    } finally {
      workspace.dispose();
    }
  });

  it("rejects accumulator references in probability, transition, and trigger contexts", () => {
    const workspace = checkedWorkspace();
    try {
      const control = saveWorkspaceVariable(workspace, {
        id: "pity", role: "control", init: 0, max: 2,
      });
      const accumulator = saveWorkspaceVariable(workspace, {
        id: "spent", role: "accumulator", init: 0, max: 10,
      });
      const probability = workspace.newBlock("entity_definition");
      const transition = workspace.newBlock("transition_set");
      transition.setFieldValue(control.variableId, "VAR");
      const trigger = workspace.newBlock("set_trigger");
      trigger.setFieldValue(control.variableId, "VAR");

      for (const host of [probability, transition, trigger]) {
        const reference = variableReference(workspace, accumulator.variableId);
        const input = host.getInput(host.type === "entity_definition" ? "PROB" : "VALUE")!.connection!;
        expect(workspace.connectionChecker.canConnect(
          reference.outputConnection,
          input,
          false,
        ), host.type).toBe(false);
      }
    } finally {
      workspace.dispose();
    }
  });

  it("allows only the current accumulator plus controls in accumulator updates", () => {
    const workspace = checkedWorkspace();
    try {
      const control = saveWorkspaceVariable(workspace, {
        id: "boost", role: "control", init: 0, max: 1,
      });
      const spent = saveWorkspaceVariable(workspace, {
        id: "spent", role: "accumulator", init: 0, max: 10,
      });
      const currency = saveWorkspaceVariable(workspace, {
        id: "currency", role: "accumulator", init: 0, max: 10,
      });
      const host = workspace.newBlock("accumulator_update");
      host.setFieldValue(spent.variableId, "VAR");
      const input = host.getInput("VALUE")!.connection!;

      expect(canConnect(workspace, variableReference(workspace, spent.variableId), input)).toBe(true);
      expect(canConnect(workspace, variableReference(workspace, control.variableId), input)).toBe(true);
      expect(canConnect(workspace, variableReference(workspace, currency.variableId), input)).toBe(false);

      const selfReference = variableReference(workspace, spent.variableId);
      input.connect(selfReference.outputConnection!);
      host.setFieldValue(currency.variableId, "VAR");
      expect(host.getFieldValue("VAR")).toBe(spent.variableId);
    } finally {
      workspace.dispose();
    }
  });

  it("does not let a connected variable dropdown bypass its expression context", () => {
    const workspace = checkedWorkspace();
    try {
      const control = saveWorkspaceVariable(workspace, {
        id: "pity", role: "control", init: 0, max: 2,
      });
      const accumulator = saveWorkspaceVariable(workspace, {
        id: "spent", role: "accumulator", init: 0, max: 10,
      });
      const entity = workspace.newBlock("entity_definition");
      const reference = variableReference(workspace, control.variableId);
      entity.getInput("PROB")!.connection!.connect(reference.outputConnection!);

      reference.setFieldValue(accumulator.variableId, "VAR");
      expect(reference.getFieldValue("VAR")).toBe(control.variableId);
      expect(() => saveWorkspaceVariable(workspace, {
        id: "pity", role: "accumulator", init: 0, max: 2,
      }, control.variableId)).toThrow("기존 변수의 역할은 바꿀 수 없습니다");
    } finally {
      workspace.dispose();
    }
  });

  it("rejects forbidden references for direct, nested, moved, and rooted-tree additions", () => {
    const workspace = checkedWorkspace();
    try {
      const control = saveWorkspaceVariable(workspace, {
        id: "pity", role: "control", init: 0, max: 2,
      });
      const accumulator = saveWorkspaceVariable(workspace, {
        id: "spent", role: "accumulator", init: 0, max: 10,
      });
      const transition = workspace.newBlock("transition_set");
      transition.setFieldValue(control.variableId, "VAR");
      const transitionInput = transition.getInput("VALUE")!.connection!;

      const direct = variableReference(workspace, accumulator.variableId);
      expect(canConnect(workspace, direct, transitionInput)).toBe(false);

      const nested = workspace.newBlock("expr_arithmetic");
      nested.getInput("LEFT")!.connection!.connect(
        variableReference(workspace, accumulator.variableId).outputConnection!,
      );
      expect(canConnect(workspace, nested, transitionInput)).toBe(false);

      const completed = workspace.newBlock("expr_arithmetic");
      completed.getInput("RIGHT")!.connection!.connect(
        variableReference(workspace, accumulator.variableId).outputConnection!,
      );
      expect(canConnect(workspace, completed, transitionInput)).toBe(false);

      const rooted = workspace.newBlock("expr_arithmetic");
      transitionInput.connect(rooted.outputConnection!);
      const insertion = variableReference(workspace, accumulator.variableId);
      expect(canConnect(workspace, insertion, rooted.getInput("LEFT")!.connection!)).toBe(false);
    } finally {
      workspace.dispose();
    }
  });

  it("separates condition entity counts from state-variable contexts", () => {
    const workspace = checkedWorkspace();
    try {
      const control = saveWorkspaceVariable(workspace, {
        id: "pity", role: "control", init: 0, max: 2,
      });
      const condition = workspace.newBlock("condition_expression");
      const comparison = workspace.newBlock("expr_compare");
      comparison.getInput("LEFT")!.connection!.connect(
        variableReference(workspace, control.variableId).outputConnection!,
      );
      expect(canConnect(workspace, comparison, condition.getInput("EXPR")!.connection!)).toBe(false);

      const probability = workspace.newBlock("entity_definition");
      const entityCount = workspace.newBlock("expr_entity_count");
      expect(canConnect(workspace, entityCount, probability.getInput("PROB")!.connection!)).toBe(false);
    } finally {
      workspace.dispose();
    }
  });

  it("preserves the silent-transition reproduction model instead of connecting it", () => {
    const model = minimalModel("hit");
    model.stateVars = [
      { id: "pity", init: 0, max: 2, role: "control" },
      { id: "spent", init: 0, max: 10, role: "accumulator" },
    ];
    model.entities[0].prob = {
      if: { ge: [{ var: "pity" }, { lit: "2" }] },
      then: { lit: "1" },
      else: { lit: "0" },
    };
    model.transitions = [{
      when: { leafOf: "__other__" },
      set: { pity: { add: [{ var: "spent" }, { lit: "1" }] } },
    }];
    const workspace = checkedWorkspace();
    try {
      expect(loadIr(workspace, model).map((item) => item.path)).toEqual(["transitions[0]"]);
      expect(workspace.getBlocksByType("transition_set", false)).toHaveLength(0);
      expect(normalizeForRoundTrip(workspaceToIr(workspace, model)))
        .toEqual(normalizeForRoundTrip(model));
    } finally {
      workspace.dispose();
    }
  });

  it("round-trips uppercase entity IDs without lossy n-prefix encoding", () => {
    const model = minimalModel("Pickup");
    model.run.condition = { ge: [{ var: "Pickup" }, { lit: "1" }] };
    const workspace = checkedWorkspace();
    try {
      expect(encodeEntityCountReference("Pickup")).toEqual({
        variable: "Pickup",
        canonical: false,
      });
      expect(loadIr(workspace, model)).toEqual([]);
      expect(workspace.getBlocksByType("expr_entity_count", false)[0].getFieldValue("ENTITY"))
        .toBe("Pickup");
      expect(normalizeForRoundTrip(workspaceToIr(workspace, model)))
        .toEqual(normalizeForRoundTrip(model));
    } finally {
      workspace.dispose();
    }
  });

  it("loads forbidden legacy expressions without throwing and keeps pasted subtrees safe", () => {
    const model = minimalModel("hit");
    model.stateVars = [
      { id: "pity", init: 0, max: 2, role: "control" },
      { id: "spent", init: 0, max: 10, role: "accumulator" },
    ];
    model.probRules = [{ target: "hit", expr: { var: "spent" } }];
    model.run.condition = { ge: [{ var: "pity" }, { lit: "1" }] };
    const workspace = checkedWorkspace();
    try {
      expect(() => loadIr(workspace, model)).not.toThrow();
      expect(getUnsupportedBlockItems(workspace).map((item) => item.path)).toEqual([
        "probRules[0]",
        "run.condition",
      ]);
      expect(normalizeForRoundTrip(workspaceToIr(workspace, model)))
        .toEqual(normalizeForRoundTrip(model));

      const accumulator = workspace.getVariableMap()
        .getVariable("spent", ACCUMULATOR_VARIABLE_TYPE)!;
      const pasted = Blockly.serialization.blocks.append({
        type: "expr_arithmetic",
        inputs: {
          LEFT: {
            block: {
              type: "expr_variable",
              fields: {
                VAR: {
                  id: accumulator.getId(),
                  name: accumulator.getName(),
                  type: accumulator.getType(),
                },
              },
            },
          },
        },
      }, workspace);
      const entity = workspace.getBlocksByType("entity_definition", false)[0];
      expect(canConnect(workspace, pasted, entity.getInput("PROB")!.connection!)).toBe(false);
    } finally {
      workspace.dispose();
    }
  });

  it("keeps a valid variable connection safe across undo and redo", async () => {
    const workspace = checkedWorkspace();
    try {
      const control = saveWorkspaceVariable(workspace, {
        id: "pity", role: "control", init: 0, max: 2,
      });
      const transition = workspace.newBlock("transition_set");
      transition.setFieldValue(control.variableId, "VAR");
      const reference = variableReference(workspace, control.variableId);
      const input = transition.getInput("VALUE")!.connection!;

      Blockly.Events.setGroup(true);
      input.connect(reference.outputConnection!);
      Blockly.Events.setGroup(false);
      await new Promise((resolve) => setTimeout(resolve, 0));

      expect(input.targetBlock()).toBe(reference);
      workspace.undo(false);
      expect(input.targetBlock()).toBeNull();
      workspace.undo(true);
      expect(input.targetBlock()).toBe(reference);
    } finally {
      Blockly.Events.setGroup(false);
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

function checkedWorkspace() {
  return new Blockly.Workspace(new Blockly.Options({
    plugins: { connectionChecker: GachaConnectionChecker },
  }));
}

function variableReference(workspace: Blockly.Workspace, variableId: string) {
  const block = workspace.newBlock("expr_variable");
  block.setFieldValue(variableId, "VAR");
  return block;
}

function canConnect(
  workspace: Blockly.Workspace,
  block: Blockly.Block,
  input: Blockly.Connection,
) {
  return workspace.connectionChecker.canConnect(block.outputConnection, input, false);
}

function minimalModel(entityId: string): ModelIr {
  return {
    irVersion: 2,
    name: "blockly context fixture",
    entities: [{ id: entityId, name: entityId, prob: { lit: "0.5" } }],
    nestingPolicy: "error",
    stateVars: [],
    probRules: [],
    transitions: [],
    triggers: [],
    run: {
      maxTrials: 2,
      trackJoint: [entityId],
      numeric: "exact",
      trialSeries: "none",
    },
  };
}

function toolboxBlocks(
  definition: Blockly.utils.toolbox.ToolboxInfo,
  categoryName: string,
): Blockly.utils.toolbox.BlockInfo[] {
  const category = definition.contents.find((item) => (
    "name" in item && item.name === categoryName
  ));
  return category && "contents" in category
    ? category.contents.filter(
      (entry): entry is Blockly.utils.toolbox.BlockInfo => "type" in entry,
    )
    : [];
}
