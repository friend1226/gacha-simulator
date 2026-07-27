import * as Blockly from "blockly/core";
import "blockly/blocks";
import type { Entity, ModelIr } from "./types";

Blockly.common.defineBlocksWithJsonArray([
  {
    type: "entity_definition",
    message0: "엔티티 %1 ID %2 확률 %3",
    args0: [
      { type: "field_input", name: "NAME", text: "픽업" },
      { type: "field_input", name: "ID", text: "pickup" },
      { type: "field_input", name: "PROB", text: "0.007" },
    ],
    message1: "하위 엔티티 %1",
    args1: [{ type: "input_statement", name: "CHILDREN" }],
    previousStatement: null,
    nextStatement: null,
    colour: 342,
    tooltip: "확률은 0.007, 1/3, 3e-5 같은 문자열로 보존됩니다.",
  },
  {
    type: "control_variable",
    message0: "제어 변수 %1 초기 %2 상한 %3",
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
    message0: "%1회 직후 리프 %2 을(를) %3개 확정 지급",
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
    { kind: "category", name: "엔티티", colour: "#d05a86", contents: [{ kind: "block", type: "entity_definition" }] },
    { kind: "category", name: "상태", colour: "#498dba", contents: [{ kind: "block", type: "control_variable" }] },
    { kind: "category", name: "확률 규칙", colour: "#7359c7", contents: [{ kind: "block", type: "soft_pity_rule" }] },
    { kind: "category", name: "전이", colour: "#3d9b79", contents: [{ kind: "block", type: "transition_set" }] },
    { kind: "category", name: "트리거", colour: "#c88a3a", contents: [{ kind: "block", type: "grant_trigger" }] },
    { kind: "category", name: "조건", colour: "#b69837", contents: [{ kind: "block", type: "first_hit_condition" }] },
  ],
};

export function workspaceToIr(workspace: Blockly.WorkspaceSvg, previous: ModelIr): ModelIr {
  const entities: Entity[] = [];
  const stateVars: ModelIr["stateVars"] = [];
  const triggers: unknown[] = [];
  const probRules: unknown[] = [];
  const transitions: unknown[] = [];
  let condition: ModelIr["run"]["condition"] | undefined;

  function readChain(first: Blockly.Block | null, entityTarget: Entity[]) {
    let block = first;
    while (block) {
      if (block.type === "entity_definition") {
        const entity: Entity = {
          id: block.getFieldValue("ID"),
          name: block.getFieldValue("NAME"),
          prob: { lit: String(block.getFieldValue("PROB")) },
          blockId: block.id,
          children: [],
        };
        readChain(block.getInputTargetBlock("CHILDREN"), entity.children!);
        entityTarget.push(entity);
      } else if (block.type === "control_variable") {
        stateVars.push({
          id: block.getFieldValue("ID"),
          init: Number(block.getFieldValue("INIT")),
          max: Number(block.getFieldValue("MAX")),
          role: "control",
          blockId: block.id,
        });
      } else if (block.type === "grant_trigger") {
        triggers.push({
          at: { trialCount: Number(block.getFieldValue("TRIAL")) },
          grant: {
            leaf: block.getFieldValue("LEAF"),
            amount: Number(block.getFieldValue("AMOUNT")),
            consumesTrial: false,
            appliesTransitions: true,
          },
          blockId: block.id,
        });
      } else if (block.type === "soft_pity_rule") {
        const target = block.getFieldValue("TARGET");
        const variable = block.getFieldValue("VAR");
        const threshold = String(block.getFieldValue("THRESHOLD"));
        const base = String(block.getFieldValue("BASE"));
        const step = String(block.getFieldValue("STEP"));
        probRules.push({
          target,
          expr: {
            if: { ge: [{ var: variable }, { lit: threshold }] },
            then: { add: [{ lit: base }, { mul: [{ lit: step }, { sub: [{ var: variable }, { lit: threshold }] }] }] },
            else: { lit: base },
          },
          blockId: block.id,
        });
      } else if (block.type === "transition_set") {
        transitions.push({
          when: { leafOf: block.getFieldValue("ENTITY") },
          set: { [block.getFieldValue("VAR")]: { lit: String(block.getFieldValue("VALUE")) } },
          blockId: block.id,
        });
      } else if (block.type === "first_hit_condition") {
        const entity = String(block.getFieldValue("ENTITY"));
        const variable = `n${entity.charAt(0).toUpperCase()}${entity.slice(1)}`;
        condition = { ge: [{ var: variable }, { lit: String(block.getFieldValue("COUNT")) }] };
      }
      block = block.getNextBlock();
    }
  }
  for (const block of workspace.getTopBlocks(true)) {
    if (!block.getPreviousBlock()) readChain(block, entities);
  }
  return {
    ...previous,
    entities,
    stateVars,
    probRules,
    transitions,
    triggers,
    run: { ...previous.run, ...(condition ? { condition } : {}) },
  };
}

export function loadIr(workspace: Blockly.WorkspaceSvg, ir: ModelIr) {
  workspace.clear();
  let y = 28;
  for (const entity of ir.entities) {
    const block = makeEntity(workspace, entity);
    block.moveBy(24, y);
    y += block.getHeightWidth().height + 28;
  }
  for (const variable of ir.stateVars) {
    const block = workspace.newBlock("control_variable");
    block.setFieldValue(variable.id, "ID");
    block.setFieldValue(variable.init, "INIT");
    block.setFieldValue(variable.max, "MAX");
    block.initSvg();
    block.render();
    block.moveBy(320, y);
    y += 76;
  }
}

function makeEntity(workspace: Blockly.WorkspaceSvg, entity: Entity): Blockly.BlockSvg {
  const block = workspace.newBlock("entity_definition") as Blockly.BlockSvg;
  block.setFieldValue(entity.name, "NAME");
  block.setFieldValue(entity.id, "ID");
  block.setFieldValue(String(entity.prob.lit ?? "0"), "PROB");
  block.initSvg();
  block.render();
  let previous: Blockly.BlockSvg | null = null;
  for (const child of entity.children ?? []) {
    const childBlock = makeEntity(workspace, child);
    if (!previous) block.getInput("CHILDREN")?.connection?.connect(childBlock.previousConnection!);
    else previous.nextConnection?.connect(childBlock.previousConnection!);
    previous = childBlock;
  }
  return block;
}

export { Blockly };
