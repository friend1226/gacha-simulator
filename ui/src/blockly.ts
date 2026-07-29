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
    message0: "%1 확률: %2가 %3 이상이면 %4 %5 / 기본 %6 / 증가량 %7",
    args0: [
      { type: "field_input", name: "TARGET", text: "star3" },
      { type: "field_input", name: "VAR", text: "pity" },
      { type: "field_number", name: "THRESHOLD", value: 65, min: 0, precision: 1 },
      {
        type: "field_dropdown",
        name: "MODE",
        options: [["선형 증가", "ramp"], ["확정값", "literal"]],
      },
      { type: "field_input", name: "THEN", text: "1" },
      { type: "field_input", name: "BASE", text: "0.03" },
      { type: "field_input", name: "STEP", text: "0.03" },
    ],
    previousStatement: null,
    nextStatement: null,
    colour: 268,
  },
  {
    type: "trial_state_prob_rule",
    message0: "%1 확률: %2회이고 %3가 %4이면 %5 / 기본 %6",
    args0: [
      { type: "field_input", name: "TARGET", text: "star5" },
      { type: "field_number", name: "TRIAL", value: 10, min: 1, precision: 1 },
      { type: "field_input", name: "VAR", text: "highSeen" },
      { type: "field_input", name: "STATE", text: "0" },
      { type: "field_input", name: "THEN", text: "0.98" },
      { type: "field_input", name: "ELSE", text: "0.08" },
    ],
    previousStatement: null,
    nextStatement: null,
    colour: 268,
    tooltip: "지정한 시행 횟수와 상태 변수 값이 모두 맞을 때 확률을 바꿉니다.",
  },
  {
    type: "transition_set",
    message0: "%1가 %2 %3를 %4 %5",
    args0: [
      { type: "field_input", name: "ENTITY", text: "star3" },
      {
        type: "field_dropdown",
        name: "PREDICATE",
        options: [["나오면", "leafOf"], ["나오지 않으면", "notLeafOf"]],
      },
      { type: "field_input", name: "VAR", text: "pity" },
      {
        type: "field_dropdown",
        name: "SET_MODE",
        options: [["대입", "literal"], ["현재값 +", "add"]],
      },
      { type: "field_input", name: "VALUE", text: "0" },
    ],
    previousStatement: null,
    nextStatement: null,
    colour: 155,
  },
  {
    type: "transition_or_set",
    message0: "%1 또는 %2가 나오면 %3를 %4 %5",
    args0: [
      { type: "field_input", name: "ENTITY_LEFT", text: "star6" },
      { type: "field_input", name: "ENTITY_RIGHT", text: "star5" },
      { type: "field_input", name: "VAR", text: "highSeen" },
      {
        type: "field_dropdown",
        name: "SET_MODE",
        options: [["대입", "literal"], ["현재값 +", "add"]],
      },
      { type: "field_input", name: "VALUE", text: "1" },
    ],
    previousStatement: null,
    nextStatement: null,
    colour: 155,
    tooltip: "두 결과 중 하나가 나오면 상태 변수를 갱신합니다.",
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
    message1: "시행 1회 소모 %1 · 전이 적용 %2",
    args1: [
      { type: "field_checkbox", name: "CONSUMES", checked: false },
      { type: "field_checkbox", name: "APPLIES", checked: true },
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
    {
      kind: "category",
      name: "확률 규칙",
      colour: "#7359c7",
      contents: [
        { kind: "block", type: "soft_pity_rule" },
        { kind: "block", type: "trial_state_prob_rule" },
      ],
    },
    {
      kind: "category",
      name: "결과 변화",
      colour: "#3d9b79",
      contents: [
        { kind: "block", type: "transition_set" },
        { kind: "block", type: "transition_or_set" },
      ],
    },
    { kind: "category", name: "시행 이벤트", colour: "#c88a3a", contents: [{ kind: "block", type: "grant_trigger" }] },
    { kind: "category", name: "조건", colour: "#b69837", contents: [{ kind: "block", type: "first_hit_condition" }] },
  ],
};

export interface UnsupportedBlockItem {
  path: string;
  description: string;
}

interface PreservedItem<T> {
  index: number;
  value: T;
}

interface WorkspaceRoundTripState {
  entities: PreservedItem<Entity>[];
  stateVars: PreservedItem<ModelIr["stateVars"][number]>[];
  probRules: PreservedItem<unknown>[];
  transitions: PreservedItem<unknown>[];
  triggers: PreservedItem<unknown>[];
  condition?: Expr;
  unsupported: UnsupportedBlockItem[];
}

interface ProbRuleBlockData {
  target: string;
  variable: string;
  threshold: string;
  base: string;
  mode: "ramp" | "literal";
  then: string;
  step: string;
}

interface TrialStateProbRuleBlockData {
  target: string;
  trial: string;
  variable: string;
  state: string;
  then: string;
  base: string;
}

interface TransitionBlockData {
  entity: string;
  predicate: "leafOf" | "notLeafOf";
  variable: string;
  mode: "literal" | "add";
  value: string;
}

interface TransitionOrBlockData {
  entities: [string, string];
  variable: string;
  mode: "literal" | "add";
  value: string;
}

interface TriggerBlockData {
  trial: number;
  leaf: string;
  amount: number;
  consumesTrial: boolean;
  appliesTransitions: boolean;
}

const roundTripState = new WeakMap<Blockly.Workspace, WorkspaceRoundTripState>();

function mergePreserved<T>(supported: T[], preserved: PreservedItem<T>[]): T[] {
  const result = [...supported];
  for (const item of [...preserved].sort((left, right) => left.index - right.index)) {
    result.splice(Math.min(item.index, result.length), 0, structuredClone(item.value));
  }
  return result;
}

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
  const children = chain(block.getInputTargetBlock("CHILDREN")).map(readEntity);
  return {
    id: block.getFieldValue("ID"),
    name: block.getFieldValue("NAME"),
    prob: { lit: String(block.getFieldValue("PROB")) },
    blockId: block.id,
    ...(children.length ? { children } : {}),
  };
}

export function workspaceToIr(workspace: Blockly.Workspace, previous: ModelIr): ModelIr {
  const root = workspace.getTopBlocks(true).find((block) => block.type === "model_container");
  if (!root) return previous;
  const preserved = roundTripState.get(workspace) ?? {
    entities: [],
    stateVars: [],
    probRules: [],
    transitions: [],
    triggers: [],
    unsupported: [],
  };
  const entities = mergePreserved(
    chain(root.getInputTargetBlock("ENTITIES")).map(readEntity),
    preserved.entities,
  );
  const stateVars = mergePreserved(chain(root.getInputTargetBlock("STATE")).map((block) => {
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
  }), preserved.stateVars);
  const probRules = mergePreserved(chain(root.getInputTargetBlock("PROB_RULES")).map((block) => {
    if (block.type === "trial_state_prob_rule") {
      return {
        target: block.getFieldValue("TARGET"),
        expr: {
          if: {
            and: [
              {
                eq: [
                  { trial: true },
                  { lit: String(block.getFieldValue("TRIAL")) },
                ],
              },
              {
                eq: [
                  { var: block.getFieldValue("VAR") },
                  { lit: String(block.getFieldValue("STATE")) },
                ],
              },
            ],
          },
          then: { lit: String(block.getFieldValue("THEN")) },
          else: { lit: String(block.getFieldValue("ELSE")) },
        },
        blockId: block.id,
      };
    }
    const target = block.getFieldValue("TARGET");
    const variable = block.getFieldValue("VAR");
    const threshold = String(block.getFieldValue("THRESHOLD"));
    const base = String(block.getFieldValue("BASE"));
    const step = String(block.getFieldValue("STEP"));
    const mode = block.getFieldValue("MODE");
    return {
      target,
      expr: {
        if: { ge: [{ var: variable }, { lit: threshold }] },
        then: mode === "literal"
          ? { lit: String(block.getFieldValue("THEN")) }
          : { add: [{ lit: base }, { mul: [{ lit: step }, { sub: [{ var: variable }, { lit: threshold }] }] }] },
        else: { lit: base },
      },
      blockId: block.id,
    };
  }), preserved.probRules);
  const transitions = mergePreserved(chain(root.getInputTargetBlock("TRANSITIONS")).map((block) => {
    const variable = block.getFieldValue("VAR");
    const value = String(block.getFieldValue("VALUE"));
    const set = {
      [variable]: block.getFieldValue("SET_MODE") === "add"
        ? { add: [{ var: variable }, { lit: value }] }
        : { lit: value },
    };
    if (block.type === "transition_or_set") {
      return {
        when: {
          or: [
            { leafOf: block.getFieldValue("ENTITY_LEFT") },
            { leafOf: block.getFieldValue("ENTITY_RIGHT") },
          ],
        },
        set,
        blockId: block.id,
      };
    }
    const entity = block.getFieldValue("ENTITY");
    return {
      when: block.getFieldValue("PREDICATE") === "notLeafOf"
        ? { not: { leafOf: entity } }
        : { leafOf: entity },
      set,
      blockId: block.id,
    };
  }), preserved.transitions);
  const triggers = mergePreserved(chain(root.getInputTargetBlock("TRIGGERS")).map((block) => ({
    at: { trialCount: Number(block.getFieldValue("TRIAL")) },
    grant: {
      leaf: block.getFieldValue("LEAF"),
      amount: Number(block.getFieldValue("AMOUNT")),
      consumesTrial: block.getFieldValue("CONSUMES") === "TRUE",
      appliesTransitions: block.getFieldValue("APPLIES") === "TRUE",
    },
    blockId: block.id,
  })), preserved.triggers);
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
    if (preserved.condition) {
      delete preserved.condition;
      preserved.unsupported = preserved.unsupported.filter((item) => item.path !== "run.condition");
    }
  }
  const run = { ...previous.run };
  if (condition) run.condition = condition;
  else if (preserved.condition) run.condition = structuredClone(preserved.condition);
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

export function getUnsupportedBlockItems(workspace: Blockly.Workspace): UnsupportedBlockItem[] {
  return structuredClone(roundTripState.get(workspace)?.unsupported ?? []);
}

function createBlock(workspace: Blockly.Workspace, type: string): Blockly.Block {
  const block = workspace.newBlock(type);
  const svg = block as Blockly.BlockSvg;
  if (typeof svg.initSvg === "function") svg.initSvg();
  if (typeof svg.render === "function") svg.render();
  return block;
}

function append(input: Blockly.Input | null, blocks: Blockly.Block[]) {
  let previous: Blockly.Block | undefined;
  for (const block of blocks) {
    if (!previous) input?.connection?.connect(block.previousConnection!);
    else previous.nextConnection?.connect(block.previousConnection!);
    previous = block;
  }
}

function makeEntity(workspace: Blockly.Workspace, entity: Entity): Blockly.Block {
  const block = createBlock(workspace, "entity_definition");
  block.setFieldValue(entity.name, "NAME");
  block.setFieldValue(entity.id, "ID");
  block.setFieldValue(String(entity.prob.lit ?? "0"), "PROB");
  append(block.getInput("CHILDREN"), (entity.children ?? []).map((child) => makeEntity(workspace, child)));
  return block;
}

function literalValue(value: unknown): string | undefined {
  if (!value || typeof value !== "object") return undefined;
  const literal = (value as { lit?: unknown }).lit;
  return typeof literal === "string" ? literal : undefined;
}

function supportedEntity(entity: Entity): boolean {
  return literalValue(entity.prob) !== undefined
    && (entity.children ?? []).every(supportedEntity);
}

function accumulatorBlockData(variable: ModelIr["stateVars"][number]) {
  if (variable.role !== "accumulator"
    || variable.clampPolicy === "error"
    || variable.update?.length !== 1) return undefined;
  const update = variable.update[0];
  const target = typeof update.when.leafOf === "string" ? update.when.leafOf : undefined;
  const add = update.set.add;
  if (!target || !Array.isArray(add) || add.length !== 2) return undefined;
  const isSelf = (value: unknown) => Boolean(value && typeof value === "object"
    && (value as Record<string, unknown>).var === variable.id);
  const amount = isSelf(add[0]) ? literalValue(add[1])
    : isSelf(add[1]) ? literalValue(add[0]) : undefined;
  if (amount === undefined || !Number.isFinite(Number(amount)) || String(Number(amount)) !== amount) return undefined;
  return { target, amount };
}

function probRuleBlockData(rule: unknown): ProbRuleBlockData | undefined {
  if (!rule || typeof rule !== "object") return undefined;
  const record = rule as Record<string, unknown>;
  const target = typeof record.target === "string" ? record.target : undefined;
  const expr = record.expr as Record<string, unknown> | undefined;
  const condition = expr?.if as Record<string, unknown> | undefined;
  const ge = condition?.ge;
  if (!target || !Array.isArray(ge) || ge.length !== 2) return undefined;
  const variable = ge[0] && typeof ge[0] === "object"
    ? (ge[0] as Record<string, unknown>).var : undefined;
  const threshold = literalValue(ge[1]);
  const base = literalValue(expr?.else);
  if (typeof variable !== "string" || threshold === undefined || base === undefined) return undefined;

  const literalThen = literalValue(expr?.then);
  if (literalThen !== undefined) {
    return { target, variable, threshold, base, mode: "literal", then: literalThen, step: "0" };
  }
  const then = expr?.then as Record<string, unknown> | undefined;
  const add = then?.add;
  if (!Array.isArray(add) || add.length !== 2 || literalValue(add[0]) !== base) return undefined;
  const mul = add[1] && typeof add[1] === "object"
    ? (add[1] as Record<string, unknown>).mul : undefined;
  if (!Array.isArray(mul) || mul.length !== 2) return undefined;
  const step = literalValue(mul[0]);
  const sub = mul[1] && typeof mul[1] === "object"
    ? (mul[1] as Record<string, unknown>).sub : undefined;
  if (!step || !Array.isArray(sub) || sub.length !== 2) return undefined;
  const subVariable = sub[0] && typeof sub[0] === "object"
    ? (sub[0] as Record<string, unknown>).var : undefined;
  if (subVariable !== variable || literalValue(sub[1]) !== threshold) return undefined;
  return { target, variable, threshold, base, mode: "ramp", then: base, step };
}

function trialStateProbRuleBlockData(rule: unknown): TrialStateProbRuleBlockData | undefined {
  if (!rule || typeof rule !== "object") return undefined;
  const record = rule as Record<string, unknown>;
  const target = typeof record.target === "string" ? record.target : undefined;
  const expr = record.expr as Record<string, unknown> | undefined;
  const condition = expr?.if as Record<string, unknown> | undefined;
  const and = condition?.and;
  if (!target || !Array.isArray(and) || and.length !== 2) return undefined;

  const trialCondition = and[0] as Record<string, unknown> | undefined;
  const trialEq = trialCondition?.eq;
  const stateCondition = and[1] as Record<string, unknown> | undefined;
  const stateEq = stateCondition?.eq;
  if (!Array.isArray(trialEq) || trialEq.length !== 2
    || !Array.isArray(stateEq) || stateEq.length !== 2) return undefined;

  const trialOperand = trialEq[0] as Record<string, unknown> | undefined;
  const trial = literalValue(trialEq[1]);
  const stateOperand = stateEq[0] as Record<string, unknown> | undefined;
  const variable = stateOperand?.var;
  const state = literalValue(stateEq[1]);
  const then = literalValue(expr?.then);
  const base = literalValue(expr?.else);
  if (trialOperand?.trial !== true
    || trial === undefined
    || typeof variable !== "string"
    || state === undefined
    || then === undefined
    || base === undefined) return undefined;
  return { target, trial, variable, state, then, base };
}

function transitionBlockData(transition: unknown): TransitionBlockData | undefined {
  if (!transition || typeof transition !== "object") return undefined;
  const record = transition as Record<string, unknown>;
  const when = record.when as Record<string, unknown> | undefined;
  let entity: unknown = when?.leafOf;
  let predicate: TransitionBlockData["predicate"] = "leafOf";
  if (typeof entity !== "string") {
    const not = when?.not as Record<string, unknown> | undefined;
    entity = not?.leafOf;
    predicate = "notLeafOf";
  }
  const set = record.set as Record<string, unknown> | undefined;
  const variables = set ? Object.keys(set) : [];
  if (typeof entity !== "string" || variables.length !== 1) return undefined;
  const variable = variables[0];
  const expression = set![variable];
  const literal = literalValue(expression);
  if (literal !== undefined) return { entity, predicate, variable, mode: "literal", value: literal };
  const add = expression && typeof expression === "object"
    ? (expression as Record<string, unknown>).add : undefined;
  if (!Array.isArray(add) || add.length !== 2) return undefined;
  const isSelf = (value: unknown) => Boolean(value && typeof value === "object"
    && (value as Record<string, unknown>).var === variable);
  const amount = isSelf(add[0]) ? literalValue(add[1])
    : isSelf(add[1]) ? literalValue(add[0]) : undefined;
  return amount === undefined
    ? undefined
    : { entity, predicate, variable, mode: "add", value: amount };
}

function transitionOrBlockData(transition: unknown): TransitionOrBlockData | undefined {
  if (!transition || typeof transition !== "object") return undefined;
  const record = transition as Record<string, unknown>;
  const when = record.when as Record<string, unknown> | undefined;
  const or = when?.or;
  if (!Array.isArray(or) || or.length !== 2) return undefined;
  const left = or[0] as Record<string, unknown> | undefined;
  const right = or[1] as Record<string, unknown> | undefined;
  const leftEntity = left?.leafOf;
  const rightEntity = right?.leafOf;
  const set = record.set as Record<string, unknown> | undefined;
  const variables = set ? Object.keys(set) : [];
  if (typeof leftEntity !== "string"
    || typeof rightEntity !== "string"
    || variables.length !== 1) return undefined;

  const variable = variables[0];
  const expression = set![variable];
  const literal = literalValue(expression);
  if (literal !== undefined) {
    return {
      entities: [leftEntity, rightEntity],
      variable,
      mode: "literal",
      value: literal,
    };
  }
  const add = expression && typeof expression === "object"
    ? (expression as Record<string, unknown>).add : undefined;
  if (!Array.isArray(add) || add.length !== 2) return undefined;
  const isSelf = (value: unknown) => Boolean(value && typeof value === "object"
    && (value as Record<string, unknown>).var === variable);
  const amount = isSelf(add[0]) ? literalValue(add[1])
    : isSelf(add[1]) ? literalValue(add[0]) : undefined;
  return amount === undefined
    ? undefined
    : {
        entities: [leftEntity, rightEntity],
        variable,
        mode: "add",
        value: amount,
      };
}

function triggerBlockData(trigger: unknown): TriggerBlockData | undefined {
  if (!trigger || typeof trigger !== "object") return undefined;
  const record = trigger as Record<string, unknown>;
  const at = record.at as Record<string, unknown> | undefined;
  const grant = record.grant as Record<string, unknown> | undefined;
  const set = record.set as Record<string, unknown> | undefined;
  if (typeof at?.trialCount !== "number"
    || typeof grant?.leaf !== "string"
    || (set && Object.keys(set).length > 0)) return undefined;
  return {
    trial: at.trialCount,
    leaf: grant.leaf,
    amount: typeof grant.amount === "number" ? grant.amount : 1,
    consumesTrial: grant.consumesTrial === true,
    appliesTransitions: grant.appliesTransitions !== false,
  };
}

function conditionBlockData(condition: Expr | undefined) {
  const ge = condition?.ge;
  if (!Array.isArray(ge) || ge.length !== 2) return undefined;
  const variable = ge[0] && typeof ge[0] === "object"
    ? (ge[0] as Record<string, unknown>).var : undefined;
  const count = literalValue(ge[1]);
  if (typeof variable !== "string" || !variable.startsWith("n") || count === undefined) return undefined;
  const entity = variable.slice(1);
  return {
    entity: entity.charAt(0).toLowerCase() + entity.slice(1),
    count,
  };
}

export function loadIr(workspace: Blockly.Workspace, ir: ModelIr): UnsupportedBlockItem[] {
  const unsupported: UnsupportedBlockItem[] = [];
  const preserved: WorkspaceRoundTripState = {
    entities: [],
    stateVars: [],
    probRules: [],
    transitions: [],
    triggers: [],
    unsupported,
  };
  Blockly.Events.disable();
  try {
    workspace.clear();
    const root = createBlock(workspace, "model_container");
    root.moveBy(32, 28);
    append(root.getInput("ENTITIES"), ir.entities.flatMap((entity, index) => {
      if (supportedEntity(entity)) return [makeEntity(workspace, entity)];
      preserved.entities.push({ index, value: structuredClone(entity) });
      unsupported.push({ path: `entities[${index}]`, description: `뽑기 결과 '${entity.id}'의 일반 확률식` });
      return [];
    }));
    append(root.getInput("STATE"), ir.stateVars.flatMap((variable, index) => {
      const accumulator = accumulatorBlockData(variable);
      const supportedControl = variable.role === "control"
        && !variable.name
        && !variable.clampPolicy
        && (!variable.update || variable.update.length === 0);
      if (!supportedControl && !accumulator) {
        preserved.stateVars.push({ index, value: structuredClone(variable) });
        const description = variable.role === "stat"
          ? `통계 변수 '${variable.id}'`
          : variable.role === "accumulator"
            ? `집계 변수 '${variable.id}'의 복합 갱신식`
            : `가챠 규칙 변수 '${variable.id}'의 추가 속성`;
        unsupported.push({ path: `stateVars[${index}]`, description });
        return [];
      }
      const block = createBlock(workspace, variable.role === "accumulator" ? "accumulator_variable" : "control_variable");
      block.setFieldValue(variable.id, "ID");
      block.setFieldValue(variable.init, "INIT");
      block.setFieldValue(variable.max ?? 0, "MAX");
      if (accumulator) {
        block.setFieldValue(variable.name ?? variable.id, "NAME");
        block.setFieldValue(accumulator.target, "TARGET");
        block.setFieldValue(Number(accumulator.amount), "AMOUNT");
      }
      return [block];
    }));
    append(root.getInput("PROB_RULES"), ir.probRules.flatMap((rule, index) => {
      const softPity = probRuleBlockData(rule);
      if (softPity) {
        const block = createBlock(workspace, "soft_pity_rule");
        block.setFieldValue(softPity.target, "TARGET");
        block.setFieldValue(softPity.variable, "VAR");
        block.setFieldValue(Number(softPity.threshold), "THRESHOLD");
        block.setFieldValue(softPity.mode, "MODE");
        block.setFieldValue(softPity.then, "THEN");
        block.setFieldValue(softPity.base, "BASE");
        block.setFieldValue(softPity.step, "STEP");
        return [block];
      }
      const trialState = trialStateProbRuleBlockData(rule);
      if (trialState) {
        const block = createBlock(workspace, "trial_state_prob_rule");
        block.setFieldValue(trialState.target, "TARGET");
        block.setFieldValue(Number(trialState.trial), "TRIAL");
        block.setFieldValue(trialState.variable, "VAR");
        block.setFieldValue(trialState.state, "STATE");
        block.setFieldValue(trialState.then, "THEN");
        block.setFieldValue(trialState.base, "ELSE");
        return [block];
      }
      preserved.probRules.push({ index, value: structuredClone(rule) });
      unsupported.push({ path: `probRules[${index}]`, description: `확률 규칙 ${index + 1}의 일반 표현식` });
      return [];
    }));
    append(root.getInput("TRANSITIONS"), ir.transitions.flatMap((transition, index) => {
      const data = transitionBlockData(transition);
      if (data) {
        const block = createBlock(workspace, "transition_set");
        block.setFieldValue(data.entity, "ENTITY");
        block.setFieldValue(data.predicate, "PREDICATE");
        block.setFieldValue(data.variable, "VAR");
        block.setFieldValue(data.mode, "SET_MODE");
        block.setFieldValue(data.value, "VALUE");
        return [block];
      }
      const orData = transitionOrBlockData(transition);
      if (orData) {
        const block = createBlock(workspace, "transition_or_set");
        block.setFieldValue(orData.entities[0], "ENTITY_LEFT");
        block.setFieldValue(orData.entities[1], "ENTITY_RIGHT");
        block.setFieldValue(orData.variable, "VAR");
        block.setFieldValue(orData.mode, "SET_MODE");
        block.setFieldValue(orData.value, "VALUE");
        return [block];
      }
      preserved.transitions.push({ index, value: structuredClone(transition) });
      unsupported.push({ path: `transitions[${index}]`, description: `결과 변화 ${index + 1}의 일반 술어 또는 표현식` });
      return [];
    }));
    append(root.getInput("TRIGGERS"), ir.triggers.flatMap((trigger, index) => {
      const data = triggerBlockData(trigger);
      if (!data) {
        preserved.triggers.push({ index, value: structuredClone(trigger) });
        unsupported.push({ path: `triggers[${index}]`, description: `시행 이벤트 ${index + 1}의 지급 외 동작` });
        return [];
      }
      const block = createBlock(workspace, "grant_trigger");
      block.setFieldValue(data.trial, "TRIAL");
      block.setFieldValue(data.leaf, "LEAF");
      block.setFieldValue(data.amount, "AMOUNT");
      block.setFieldValue(data.consumesTrial ? "TRUE" : "FALSE", "CONSUMES");
      block.setFieldValue(data.appliesTransitions ? "TRUE" : "FALSE", "APPLIES");
      return [block];
    }));
    const condition = conditionBlockData(ir.run.condition);
    if (condition) {
      const block = createBlock(workspace, "first_hit_condition");
      block.setFieldValue(condition.entity, "ENTITY");
      block.setFieldValue(Number(condition.count), "COUNT");
      append(root.getInput("CONDITION"), [block]);
    } else if (ir.run.condition) {
      preserved.condition = structuredClone(ir.run.condition);
      unsupported.push({ path: "run.condition", description: "일반 최초 달성 조건식" });
    }
    roundTripState.set(workspace, preserved);
  } finally {
    Blockly.Events.enable();
  }
  return unsupported;
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
