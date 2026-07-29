import * as Blockly from "blockly/core";
import "blockly/blocks";
import type { Entity, Expr, ModelIr } from "./types";

Blockly.common.defineBlocksWithJsonArray([
  {
    type: "model_container",
    message0: "가챠 모델",
    message1: "뽑기 결과 %1",
    args1: [{ type: "input_statement", name: "ENTITIES" }],
    message2: "상태와 집계 %1",
    args2: [{ type: "input_statement", name: "STATE" }],
    message3: "확률 규칙 %1",
    args3: [{ type: "input_statement", name: "PROB_RULES" }],
    message4: "결과에 따른 변화 %1",
    args4: [{ type: "input_statement", name: "TRANSITIONS" }],
    message5: "시행 횟수 이벤트 %1",
    args5: [{ type: "input_statement", name: "TRIGGERS" }],
    message6: "최초 달성 조건 %1",
    args6: [{ type: "input_statement", name: "CONDITION" }],
    colour: 225,
    tooltip: "각 종류의 선언과 규칙을 의미별 슬롯에 넣습니다.",
  },
  {
    type: "entity_definition",
    message0: "뽑기 결과 %1 ID %2 확률 %3",
    args0: [
      { type: "field_input", name: "NAME", text: "픽업" },
      { type: "field_input", name: "ID", text: "pickup" },
      { type: "field_input", name: "PROB", text: "0.007" },
    ],
    message1: "하위 결과 %1",
    args1: [{ type: "input_statement", name: "CHILDREN" }],
    previousStatement: null,
    nextStatement: null,
    colour: 342,
    tooltip: "확률은 0.007, 1/3, 3e-5 같은 문자열로 정확히 보존됩니다.",
  },
  {
    type: "control_variable",
    message0: "가챠 규칙 변수 %1 초기 %2 상한 %3",
    args0: [
      { type: "field_input", name: "ID", text: "pity" },
      { type: "field_number", name: "INIT", value: 0, min: 0, precision: 1 },
      { type: "field_number", name: "MAX", value: 89, min: 0, precision: 1 },
    ],
    previousStatement: null,
    nextStatement: null,
    colour: 205,
  },
  {
    type: "accumulator_variable",
    message0: "집계 변수 %1 표시명 %2 초기 %3 상한 %4",
    args0: [
      { type: "field_input", name: "ID", text: "spent" },
      { type: "field_input", name: "NAME", text: "소모 재화" },
      { type: "field_number", name: "INIT", value: 0, min: 0, precision: 1 },
      { type: "field_number", name: "MAX", value: 60000, min: 0, precision: 1 },
    ],
    message1: "%1 결과마다 %2 증가",
    args1: [
      { type: "field_input", name: "TARGET", text: "__other__" },
      { type: "field_number", name: "AMOUNT", value: 120, precision: 1 },
    ],
    previousStatement: null,
    nextStatement: null,
    colour: 185,
    tooltip: "확률에는 영향을 주지 않고 결과 집계에만 쓰입니다.",
  },
  {
    type: "soft_pity_rule",
    message0: "%1 확률: %2가 %3 이상이면 기본 %4 + 초과당 %5",
    args0: [
      { type: "field_input", name: "TARGET", text: "star3" },
      { type: "field_input", name: "VAR", text: "pity" },
      { type: "field_number", name: "THRESHOLD", value: 65, min: 0, precision: 1 },
      { type: "field_input", name: "BASE", text: "0.03" },
      { type: "field_input", name: "STEP", text: "0.03" },
    ],
    previousStatement: null,
    nextStatement: null,
    colour: 268,
  },
  {
    type: "transition_set",
    message0: "%1가 나오면 %2 = %3",
    args0: [
      { type: "field_input", name: "ENTITY", text: "star3" },
      { type: "field_input", name: "VAR", text: "pity" },
      { type: "field_input", name: "VALUE", text: "0" },
    ],
    previousStatement: null,
    nextStatement: null,
    colour: 155,
  },
  {
    type: "first_hit_condition",
    message0: "%1 개수가 %2 이상일 때 최초 달성",
    args0: [
      { type: "field_input", name: "ENTITY", text: "pickup" },
      { type: "field_number", name: "COUNT", value: 2, min: 1, precision: 1 },
    ],
    previousStatement: null,
    nextStatement: null,
    colour: 62,
  },
  {
    type: "grant_trigger",
    message0: "%1회 직후 최종 항목 %2 을(를) %3개 확정 지급",
    args0: [
      { type: "field_number", name: "TRIAL", value: 200, min: 1, precision: 1 },
      { type: "field_input", name: "LEAF", text: "pickup" },
      { type: "field_number", name: "AMOUNT", value: 1, min: 1, precision: 1 },
    ],
    previousStatement: null,
    nextStatement: null,
    colour: 35,
  },
]);

export const toolbox: Blockly.utils.toolbox.ToolboxDefinition = {
  kind: "categoryToolbox",
  contents: [
    { kind: "category", name: "모델", colour: "#6372d9", contents: [{ kind: "block", type: "model_container" }] },
    { kind: "category", name: "뽑기 결과", colour: "#d05a86", contents: [{ kind: "block", type: "entity_definition" }] },
    {
      kind: "category",
      name: "상태",
      colour: "#498dba",
      contents: [
        { kind: "block", type: "control_variable" },
        { kind: "block", type: "accumulator_variable" },
      ],
    },
    { kind: "category", name: "확률 규칙", colour: "#7359c7", contents: [{ kind: "block", type: "soft_pity_rule" }] },
    { kind: "category", name: "결과 변화", colour: "#3d9b79", contents: [{ kind: "block", type: "transition_set" }] },
    { kind: "category", name: "시행 이벤트", colour: "#c88a3a", contents: [{ kind: "block", type: "grant_trigger" }] },
    { kind: "category", name: "조건", colour: "#b69837", contents: [{ kind: "block", type: "first_hit_condition" }] },
  ],
};

function chain(first: Blockly.Block | null): Blockly.Block[] {
  const result: Blockly.Block[] = [];
  let block = first;
  while (block) {
    result.push(block);
    block = block.getNextBlock();
  }
  return result;
}

function readEntity(block: Blockly.Block): Entity {
  return {
    id: block.getFieldValue("ID"),
    name: block.getFieldValue("NAME"),
    prob: { lit: String(block.getFieldValue("PROB")) },
    blockId: block.id,
    children: chain(block.getInputTargetBlock("CHILDREN")).map(readEntity),
  };
}

export function workspaceToIr(workspace: Blockly.WorkspaceSvg, previous: ModelIr): ModelIr {
  const root = workspace.getTopBlocks(true).find((block) => block.type === "model_container");
  if (!root) return previous;
  const entities = chain(root.getInputTargetBlock("ENTITIES")).map(readEntity);
  const stateVars: ModelIr["stateVars"] = chain(root.getInputTargetBlock("STATE")).map((block) => {
    if (block.type === "accumulator_variable") {
      const id = String(block.getFieldValue("ID"));
      return {
        id,
        name: String(block.getFieldValue("NAME")),
        init: Number(block.getFieldValue("INIT")),
        max: Number(block.getFieldValue("MAX")),
        role: "accumulator" as const,
        update: [{
          when: { leafOf: String(block.getFieldValue("TARGET")) },
          set: { add: [{ var: id }, { lit: String(block.getFieldValue("AMOUNT")) }] },
        }],
        clampPolicy: "saturate" as const,
        blockId: block.id,
      };
    }
    return {
      id: String(block.getFieldValue("ID")),
      init: Number(block.getFieldValue("INIT")),
      max: Number(block.getFieldValue("MAX")),
      role: "control" as const,
      blockId: block.id,
    };
  });
  const probRules = chain(root.getInputTargetBlock("PROB_RULES")).map((block) => {
    const target = block.getFieldValue("TARGET");
    const variable = block.getFieldValue("VAR");
    const threshold = String(block.getFieldValue("THRESHOLD"));
    const base = String(block.getFieldValue("BASE"));
    const step = String(block.getFieldValue("STEP"));
    return {
      target,
      expr: {
        if: { ge: [{ var: variable }, { lit: threshold }] },
        then: { add: [{ lit: base }, { mul: [{ lit: step }, { sub: [{ var: variable }, { lit: threshold }] }] }] },
        else: { lit: base },
      },
      blockId: block.id,
    };
  });
  const transitions = chain(root.getInputTargetBlock("TRANSITIONS")).map((block) => ({
    when: { leafOf: block.getFieldValue("ENTITY") },
    set: { [block.getFieldValue("VAR")]: { lit: String(block.getFieldValue("VALUE")) } },
    blockId: block.id,
  }));
  const triggers = chain(root.getInputTargetBlock("TRIGGERS")).map((block) => ({
    at: { trialCount: Number(block.getFieldValue("TRIAL")) },
    grant: {
      leaf: block.getFieldValue("LEAF"),
      amount: Number(block.getFieldValue("AMOUNT")),
      consumesTrial: false,
      appliesTransitions: true,
    },
    blockId: block.id,
  }));
  const conditionBlock = root.getInputTargetBlock("CONDITION");
  let condition: Expr | undefined;
  if (conditionBlock) {
    const entity = String(conditionBlock.getFieldValue("ENTITY"));
    condition = {
      ge: [
        { var: `n${entity.charAt(0).toUpperCase()}${entity.slice(1)}` },
        { lit: String(conditionBlock.getFieldValue("COUNT")) },
      ],
    };
  }
  const run = { ...previous.run };
  if (condition) run.condition = condition;
  else delete run.condition;
  return {
    ...previous,
    irVersion: 2,
    entities,
    stateVars,
    probRules,
    transitions,
    triggers,
    run,
  };
}

function createBlock(workspace: Blockly.WorkspaceSvg, type: string): Blockly.BlockSvg {
  const block = workspace.newBlock(type) as Blockly.BlockSvg;
  block.initSvg();
  block.render();
  return block;
}

function append(input: Blockly.Input | null, blocks: Blockly.BlockSvg[]) {
  let previous: Blockly.BlockSvg | undefined;
  for (const block of blocks) {
    if (!previous) input?.connection?.connect(block.previousConnection!);
    else previous.nextConnection?.connect(block.previousConnection!);
    previous = block;
  }
}

function makeEntity(workspace: Blockly.WorkspaceSvg, entity: Entity): Blockly.BlockSvg {
  const block = createBlock(workspace, "entity_definition");
  block.setFieldValue(entity.name, "NAME");
  block.setFieldValue(entity.id, "ID");
  block.setFieldValue(String(entity.prob.lit ?? "0"), "PROB");
  append(block.getInput("CHILDREN"), (entity.children ?? []).map((child) => makeEntity(workspace, child)));
  return block;
}

function simpleLiteral(value: unknown, fallback = "0"): string {
  if (!value || typeof value !== "object") return fallback;
  const literal = (value as { lit?: unknown }).lit;
  return typeof literal === "string" ? literal : fallback;
}

export function loadIr(workspace: Blockly.WorkspaceSvg, ir: ModelIr) {
  Blockly.Events.disable();
  try {
    workspace.clear();
    const root = createBlock(workspace, "model_container");
    root.moveBy(32, 28);
    append(root.getInput("ENTITIES"), ir.entities.map((entity) => makeEntity(workspace, entity)));
    append(root.getInput("STATE"), ir.stateVars.flatMap((variable) => {
      if (variable.role === "stat") return [];
      const block = createBlock(workspace, variable.role === "accumulator" ? "accumulator_variable" : "control_variable");
      block.setFieldValue(variable.id, "ID");
      block.setFieldValue(variable.init, "INIT");
      block.setFieldValue(variable.max ?? 0, "MAX");
      if (variable.role === "accumulator") {
        block.setFieldValue(variable.name ?? variable.id, "NAME");
        const update = variable.update?.[0];
        const target = update?.when && "leafOf" in update.when ? update.when.leafOf : "__other__";
        block.setFieldValue(String(target), "TARGET");
        const add = update?.set?.add;
        const amount = Array.isArray(add) ? simpleLiteral(add[1], "1") : "1";
        block.setFieldValue(Number(amount), "AMOUNT");
      }
      return [block];
    }));
    append(root.getInput("PROB_RULES"), (ir.probRules as Array<Record<string, unknown>>).flatMap((rule) => {
      const expr = rule.expr as Record<string, unknown> | undefined;
      const branch = expr?.if as Record<string, unknown> | undefined;
      const ge = branch?.ge;
      if (!Array.isArray(ge)) return [];
      const block = createBlock(workspace, "soft_pity_rule");
      block.setFieldValue(String(rule.target ?? ""), "TARGET");
      block.setFieldValue(String((ge[0] as { var?: string })?.var ?? ""), "VAR");
      block.setFieldValue(Number(simpleLiteral(ge[1], "0")), "THRESHOLD");
      block.setFieldValue(simpleLiteral((expr?.else as Expr | undefined), "0"), "BASE");
      return [block];
    }));
    append(root.getInput("TRANSITIONS"), (ir.transitions as Array<Record<string, unknown>>).flatMap((transition) => {
      const when = transition.when as { leafOf?: string } | undefined;
      const set = transition.set as Record<string, unknown> | undefined;
      const variable = set ? Object.keys(set)[0] : undefined;
      if (!when?.leafOf || !set || !variable) return [];
      const block = createBlock(workspace, "transition_set");
      block.setFieldValue(when.leafOf, "ENTITY");
      block.setFieldValue(variable, "VAR");
      block.setFieldValue(simpleLiteral(set[variable]), "VALUE");
      return [block];
    }));
    append(root.getInput("TRIGGERS"), (ir.triggers as Array<Record<string, unknown>>).flatMap((trigger) => {
      const at = trigger.at as { trialCount?: number } | undefined;
      const grant = trigger.grant as { leaf?: string; amount?: number } | undefined;
      if (!at?.trialCount || !grant?.leaf) return [];
      const block = createBlock(workspace, "grant_trigger");
      block.setFieldValue(at.trialCount, "TRIAL");
      block.setFieldValue(grant.leaf, "LEAF");
      block.setFieldValue(grant.amount ?? 1, "AMOUNT");
      return [block];
    }));
    const condition = ir.run.condition as { ge?: unknown[] } | undefined;
    if (Array.isArray(condition?.ge)) {
      const variable = (condition.ge[0] as { var?: string })?.var;
      if (variable?.startsWith("n")) {
        const block = createBlock(workspace, "first_hit_condition");
        const entity = variable.slice(1);
        block.setFieldValue(entity.charAt(0).toLowerCase() + entity.slice(1), "ENTITY");
        block.setFieldValue(Number(simpleLiteral(condition.ge[1], "1")), "COUNT");
        append(root.getInput("CONDITION"), [block]);
      }
    }
  } finally {
    Blockly.Events.enable();
  }
}

export function installWorkspaceVolume(
  workspace: Blockly.WorkspaceSvg,
  getVolume: () => number,
) {
  const audio = workspace.getAudioManager();
  const original = audio.play.bind(audio);
  audio.play = (name: string, volume = 1) => original(name, volume * getVolume());
}

export { Blockly };
