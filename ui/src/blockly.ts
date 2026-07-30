import * as Blockly from "blockly/core";
import "blockly/blocks";
import type {
  BooleanExpr,
  Entity,
  Expr,
  ModelIr,
  NumberExpr,
  ProbabilityRule,
} from "./types";

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
    message0: "뽑기 결과 %1 ID %2",
    args0: [
      { type: "field_input", name: "NAME", text: "픽업" },
      { type: "field_input", name: "ID", text: "pickup" },
    ],
    message1: "확률 %1",
    args1: [{ type: "input_value", name: "PROB", check: "Number" }],
    message2: "하위 결과 %1",
    args2: [{ type: "input_statement", name: "CHILDREN" }],
    previousStatement: null,
    nextStatement: null,
    colour: 342,
    tooltip: "확률 소켓에 숫자 또는 숫자를 만드는 식을 연결합니다.",
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
    type: "probability_rule",
    message0: "%1 확률을 %2 로 설정",
    args0: [
      { type: "field_input", name: "TARGET", text: "star3" },
      { type: "input_value", name: "EXPR", check: "Number" },
    ],
    previousStatement: null,
    nextStatement: null,
    colour: 268,
    tooltip: "대상 ID와 새 확률을 계산할 숫자 식을 연결합니다.",
  },
  {
    type: "expr_literal",
    message0: "숫자 %1",
    args0: [
      { type: "field_input", name: "VALUE", text: "0.007" },
    ],
    output: "Number",
    colour: 230,
    tooltip: "0.007, 1/3, 3e-5처럼 입력한 문자열을 f64 변환 없이 보존합니다.",
  },
  {
    type: "expr_variable",
    message0: "상태 변수 %1",
    args0: [{ type: "field_input", name: "VAR", text: "pity" }],
    output: "Number",
    colour: 205,
    tooltip: "확률이나 상태 갱신에서 읽을 상태 변수 ID를 입력합니다.",
  },
  {
    type: "expr_trial",
    message0: "시행 횟수",
    output: "Number",
    colour: 205,
    tooltip: "현재 뽑기의 1부터 시작하는 시행 번호입니다.",
  },
  {
    type: "expr_arithmetic",
    message0: "%1 %2 %3",
    args0: [
      { type: "input_value", name: "LEFT", check: "Number" },
      {
        type: "field_dropdown",
        name: "OP",
        options: [["+", "add"], ["−", "sub"], ["×", "mul"], ["÷", "div"]],
      },
      { type: "input_value", name: "RIGHT", check: "Number" },
    ],
    inputsInline: true,
    output: "Number",
    colour: 230,
    tooltip: "양쪽 숫자 소켓에 계산할 값을 연결합니다.",
  },
  {
    type: "expr_unary",
    message0: "%1 %2",
    args0: [
      {
        type: "field_dropdown",
        name: "OP",
        options: [
          ["부호 반전", "neg"],
          ["절댓값", "abs"],
          ["내림", "floor"],
          ["올림", "ceil"],
          ["반올림", "round"],
        ],
      },
      { type: "input_value", name: "VALUE", check: "Number" },
    ],
    output: "Number",
    colour: 230,
    tooltip: "숫자 소켓에 단항 연산을 적용할 값을 연결합니다.",
  },
  {
    type: "expr_minmax",
    message0: "%1 (%2, %3)",
    args0: [
      {
        type: "field_dropdown",
        name: "OP",
        options: [["최솟값", "min"], ["최댓값", "max"]],
      },
      { type: "input_value", name: "LEFT", check: "Number" },
      { type: "input_value", name: "RIGHT", check: "Number" },
    ],
    inputsInline: true,
    output: "Number",
    colour: 230,
    tooltip: "두 숫자 중 작은 값 또는 큰 값을 선택합니다.",
  },
  {
    type: "expr_clamp",
    message0: "%1 을(를) %2 이상 %3 이하로 제한",
    args0: [
      { type: "input_value", name: "VALUE", check: "Number" },
      { type: "input_value", name: "LOW", check: "Number" },
      { type: "input_value", name: "HIGH", check: "Number" },
    ],
    inputsInline: true,
    output: "Number",
    colour: 230,
    tooltip: "값, 최솟값, 최댓값 순서로 숫자를 연결합니다.",
  },
  {
    type: "expr_pow",
    message0: "%1 의 %2 제곱",
    args0: [
      { type: "input_value", name: "BASE", check: "Number" },
      {
        type: "field_number",
        name: "EXPONENT",
        value: 2,
        min: -2147483648,
        max: 2147483647,
        precision: 1,
      },
    ],
    output: "Number",
    colour: 230,
    tooltip: "밑에는 숫자 식을 연결하고 지수에는 정수만 입력합니다.",
  },
  {
    type: "expr_compare",
    message0: "%1 %2 %3",
    args0: [
      { type: "input_value", name: "LEFT", check: "Number" },
      {
        type: "field_dropdown",
        name: "OP",
        options: [
          ["=", "eq"],
          ["≠", "ne"],
          ["<", "lt"],
          ["≤", "le"],
          [">", "gt"],
          ["≥", "ge"],
        ],
      },
      { type: "input_value", name: "RIGHT", check: "Number" },
    ],
    inputsInline: true,
    output: "Boolean",
    colour: 120,
    tooltip: "두 숫자를 비교해 참 또는 거짓을 만듭니다.",
  },
  {
    type: "expr_logic",
    message0: "%1 %2 %3",
    args0: [
      { type: "input_value", name: "LEFT", check: "Boolean" },
      {
        type: "field_dropdown",
        name: "OP",
        options: [["그리고", "and"], ["또는", "or"], ["서로 다름", "xor"]],
      },
      { type: "input_value", name: "RIGHT", check: "Boolean" },
    ],
    inputsInline: true,
    output: "Boolean",
    colour: 120,
    tooltip: "양쪽에 참/거짓 식을 연결합니다.",
  },
  {
    type: "expr_not",
    message0: "아님 %1",
    args0: [{ type: "input_value", name: "VALUE", check: "Boolean" }],
    output: "Boolean",
    colour: 120,
    tooltip: "연결한 참/거짓 식의 결과를 뒤집습니다.",
  },
  {
    type: "expr_if",
    message0: "만약 %1",
    args0: [{ type: "input_value", name: "IF", check: "Boolean" }],
    message1: "이면 %1",
    args1: [{ type: "input_value", name: "THEN", check: "Number" }],
    message2: "아니면 %1",
    args2: [{ type: "input_value", name: "ELSE", check: "Number" }],
    output: "Number",
    colour: 268,
    tooltip: "조건에는 참/거짓 식을, 두 결과에는 숫자 식을 연결합니다.",
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
    {
      kind: "category",
      name: "뽑기 결과",
      colour: "#d05a86",
      contents: [{
        kind: "block",
        type: "entity_definition",
        inputs: {
          PROB: {
            block: {
              type: "expr_literal",
              fields: { VALUE: "0.007" },
            },
          },
        },
      }],
    },
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
        {
          kind: "block",
          type: "probability_rule",
          inputs: {
            EXPR: {
              block: {
                type: "expr_literal",
                fields: { VALUE: "0.03" },
              },
            },
          },
        },
        {
          kind: "block",
          type: "probability_rule",
          fields: { TARGET: "star3" },
          inputs: {
            EXPR: {
              block: {
                type: "expr_if",
                inputs: {
                  IF: {
                    block: {
                      type: "expr_compare",
                      fields: { OP: "ge" },
                      inputs: {
                        LEFT: { block: { type: "expr_variable", fields: { VAR: "pity" } } },
                        RIGHT: { block: { type: "expr_literal", fields: { VALUE: "65" } } },
                      },
                    },
                  },
                  THEN: {
                    block: {
                      type: "expr_arithmetic",
                      fields: { OP: "add" },
                      inputs: {
                        LEFT: { block: { type: "expr_literal", fields: { VALUE: "0.03" } } },
                        RIGHT: {
                          block: {
                            type: "expr_arithmetic",
                            fields: { OP: "mul" },
                            inputs: {
                              LEFT: { block: { type: "expr_literal", fields: { VALUE: "0.03" } } },
                              RIGHT: {
                                block: {
                                  type: "expr_arithmetic",
                                  fields: { OP: "sub" },
                                  inputs: {
                                    LEFT: { block: { type: "expr_variable", fields: { VAR: "pity" } } },
                                    RIGHT: { block: { type: "expr_literal", fields: { VALUE: "65" } } },
                                  },
                                },
                              },
                            },
                          },
                        },
                      },
                    },
                  },
                  ELSE: { block: { type: "expr_literal", fields: { VALUE: "0.03" } } },
                },
              },
            },
          },
        },
        {
          kind: "block",
          type: "probability_rule",
          fields: { TARGET: "star5" },
          inputs: {
            EXPR: {
              block: {
                type: "expr_if",
                inputs: {
                  IF: {
                    block: {
                      type: "expr_logic",
                      fields: { OP: "and" },
                      inputs: {
                        LEFT: {
                          block: {
                            type: "expr_compare",
                            fields: { OP: "eq" },
                            inputs: {
                              LEFT: { block: { type: "expr_trial" } },
                              RIGHT: { block: { type: "expr_literal", fields: { VALUE: "10" } } },
                            },
                          },
                        },
                        RIGHT: {
                          block: {
                            type: "expr_compare",
                            fields: { OP: "eq" },
                            inputs: {
                              LEFT: { block: { type: "expr_variable", fields: { VAR: "highSeen" } } },
                              RIGHT: { block: { type: "expr_literal", fields: { VALUE: "0" } } },
                            },
                          },
                        },
                      },
                    },
                  },
                  THEN: { block: { type: "expr_literal", fields: { VALUE: "0.98" } } },
                  ELSE: { block: { type: "expr_literal", fields: { VALUE: "0.08" } } },
                },
              },
            },
          },
        },
      ],
    },
    {
      kind: "category",
      name: "값",
      colour: "#5b78c7",
      contents: [
        { kind: "block", type: "expr_literal" },
        { kind: "block", type: "expr_variable" },
        { kind: "block", type: "expr_trial" },
        { kind: "block", type: "expr_arithmetic" },
        { kind: "block", type: "expr_unary" },
        { kind: "block", type: "expr_minmax" },
        { kind: "block", type: "expr_clamp" },
        { kind: "block", type: "expr_pow" },
        { kind: "block", type: "expr_compare" },
        { kind: "block", type: "expr_logic" },
        { kind: "block", type: "expr_not" },
        { kind: "block", type: "expr_if" },
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
  probRules: PreservedItem<ProbabilityRule>[];
  transitions: PreservedItem<unknown>[];
  triggers: PreservedItem<unknown>[];
  condition?: BooleanExpr;
  unsupported: UnsupportedBlockItem[];
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

type SerializedBlock = Blockly.serialization.blocks.State;

const arithmeticOperators = ["add", "sub", "mul", "div"] as const;
const unaryOperators = ["neg", "abs", "floor", "ceil", "round"] as const;
const minMaxOperators = ["min", "max"] as const;
const comparisonOperators = ["eq", "ne", "lt", "le", "gt", "ge"] as const;
const logicOperators = ["and", "or", "xor"] as const;

function recordValue(value: unknown): Record<string, unknown> | undefined {
  return value && typeof value === "object" && !Array.isArray(value)
    ? value as Record<string, unknown>
    : undefined;
}

function binaryArguments(value: unknown): [unknown, unknown] | undefined {
  return Array.isArray(value) && value.length === 2
    ? [value[0], value[1]]
    : undefined;
}

function ternaryArguments(value: unknown): [unknown, unknown, unknown] | undefined {
  return Array.isArray(value) && value.length === 3
    ? [value[0], value[1], value[2]]
    : undefined;
}

function unaryArgument(value: unknown): unknown {
  if (Array.isArray(value)) return value.length === 1 ? value[0] : undefined;
  return value;
}

function blockInput(block: SerializedBlock): { block: SerializedBlock } {
  return { block };
}

function numberExprState(expression: unknown): SerializedBlock | undefined {
  const record = recordValue(expression);
  if (!record) return undefined;
  if (typeof record.lit === "string") {
    return { type: "expr_literal", fields: { VALUE: record.lit } };
  }
  if (typeof record.var === "string") {
    return { type: "expr_variable", fields: { VAR: record.var } };
  }
  if (record.trial === true) return { type: "expr_trial" };

  if ("if" in record) {
    const condition = booleanExprState(record.if);
    const thenBranch = numberExprState(record.then);
    const elseBranch = numberExprState(record.else);
    if (!condition || !thenBranch || !elseBranch) return undefined;
    return {
      type: "expr_if",
      inputs: {
        IF: blockInput(condition),
        THEN: blockInput(thenBranch),
        ELSE: blockInput(elseBranch),
      },
    };
  }

  for (const operator of arithmeticOperators) {
    if (!(operator in record)) continue;
    const args = binaryArguments(record[operator]);
    const left = args && numberExprState(args[0]);
    const right = args && numberExprState(args[1]);
    if (!left || !right) return undefined;
    return {
      type: "expr_arithmetic",
      fields: { OP: operator },
      inputs: { LEFT: blockInput(left), RIGHT: blockInput(right) },
    };
  }
  for (const operator of unaryOperators) {
    if (!(operator in record)) continue;
    const value = numberExprState(unaryArgument(record[operator]));
    if (!value) return undefined;
    return {
      type: "expr_unary",
      fields: { OP: operator },
      inputs: { VALUE: blockInput(value) },
    };
  }
  for (const operator of minMaxOperators) {
    if (!(operator in record)) continue;
    const args = binaryArguments(record[operator]);
    const left = args && numberExprState(args[0]);
    const right = args && numberExprState(args[1]);
    if (!left || !right) return undefined;
    return {
      type: "expr_minmax",
      fields: { OP: operator },
      inputs: { LEFT: blockInput(left), RIGHT: blockInput(right) },
    };
  }
  if ("clamp" in record) {
    const args = ternaryArguments(record.clamp);
    const value = args && numberExprState(args[0]);
    const low = args && numberExprState(args[1]);
    const high = args && numberExprState(args[2]);
    if (!value || !low || !high) return undefined;
    return {
      type: "expr_clamp",
      inputs: {
        VALUE: blockInput(value),
        LOW: blockInput(low),
        HIGH: blockInput(high),
      },
    };
  }
  if ("pow" in record) {
    const args = binaryArguments(record.pow);
    const base = args && numberExprState(args[0]);
    const exponent = args && recordValue(args[1])?.lit;
    const parsedExponent = typeof exponent === "string" && /^-?\d+$/.test(exponent)
      ? Number(exponent)
      : Number.NaN;
    if (!base
      || !Number.isInteger(parsedExponent)
      || parsedExponent < -2_147_483_648
      || parsedExponent > 2_147_483_647) return undefined;
    return {
      type: "expr_pow",
      fields: { EXPONENT: parsedExponent },
      inputs: { BASE: blockInput(base) },
    };
  }
  return undefined;
}

function booleanExprState(expression: unknown): SerializedBlock | undefined {
  const record = recordValue(expression);
  if (!record) return undefined;
  for (const operator of comparisonOperators) {
    if (!(operator in record)) continue;
    const args = binaryArguments(record[operator]);
    const left = args && numberExprState(args[0]);
    const right = args && numberExprState(args[1]);
    if (!left || !right) return undefined;
    return {
      type: "expr_compare",
      fields: { OP: operator },
      inputs: { LEFT: blockInput(left), RIGHT: blockInput(right) },
    };
  }
  for (const operator of logicOperators) {
    if (!(operator in record)) continue;
    const args = binaryArguments(record[operator]);
    const left = args && booleanExprState(args[0]);
    const right = args && booleanExprState(args[1]);
    if (!left || !right) return undefined;
    return {
      type: "expr_logic",
      fields: { OP: operator },
      inputs: { LEFT: blockInput(left), RIGHT: blockInput(right) },
    };
  }
  if ("not" in record) {
    const value = booleanExprState(unaryArgument(record.not));
    if (!value) return undefined;
    return {
      type: "expr_not",
      inputs: { VALUE: blockInput(value) },
    };
  }
  return undefined;
}

export function exprToBlock(
  workspace: Blockly.Workspace,
  expression: Expr,
): Blockly.Block | undefined {
  const state = numberExprState(expression) ?? booleanExprState(expression);
  return state
    ? Blockly.serialization.blocks.append(state, workspace)
    : undefined;
}

function numberExprToBlock(
  workspace: Blockly.Workspace,
  expression: unknown,
): Blockly.Block | undefined {
  const state = numberExprState(expression);
  return state
    ? Blockly.serialization.blocks.append(state, workspace)
    : undefined;
}

function inputBlock(block: Blockly.Block, name: string): Blockly.Block | undefined {
  return block.getInputTargetBlock(name) ?? undefined;
}

function blockToNumberExpr(block: Blockly.Block | undefined): NumberExpr | undefined {
  if (!block) return undefined;
  if (block.type === "expr_literal") {
    return { lit: String(block.getFieldValue("VALUE")) };
  }
  if (block.type === "expr_variable") {
    return { var: String(block.getFieldValue("VAR")) };
  }
  if (block.type === "expr_trial") return { trial: true };
  if (block.type === "expr_arithmetic") {
    const operator = String(block.getFieldValue("OP"));
    const left = blockToNumberExpr(inputBlock(block, "LEFT"));
    const right = blockToNumberExpr(inputBlock(block, "RIGHT"));
    if (!arithmeticOperators.includes(operator as typeof arithmeticOperators[number])
      || !left
      || !right) return undefined;
    return { [operator]: [left, right] } as NumberExpr;
  }
  if (block.type === "expr_unary") {
    const operator = String(block.getFieldValue("OP"));
    const value = blockToNumberExpr(inputBlock(block, "VALUE"));
    if (!unaryOperators.includes(operator as typeof unaryOperators[number]) || !value) {
      return undefined;
    }
    return { [operator]: value } as NumberExpr;
  }
  if (block.type === "expr_minmax") {
    const operator = String(block.getFieldValue("OP"));
    const left = blockToNumberExpr(inputBlock(block, "LEFT"));
    const right = blockToNumberExpr(inputBlock(block, "RIGHT"));
    if (!minMaxOperators.includes(operator as typeof minMaxOperators[number])
      || !left
      || !right) return undefined;
    return { [operator]: [left, right] } as NumberExpr;
  }
  if (block.type === "expr_clamp") {
    const value = blockToNumberExpr(inputBlock(block, "VALUE"));
    const low = blockToNumberExpr(inputBlock(block, "LOW"));
    const high = blockToNumberExpr(inputBlock(block, "HIGH"));
    return value && low && high ? { clamp: [value, low, high] } : undefined;
  }
  if (block.type === "expr_pow") {
    const base = blockToNumberExpr(inputBlock(block, "BASE"));
    const exponent = Number(block.getFieldValue("EXPONENT"));
    return base
      && Number.isInteger(exponent)
      && exponent >= -2_147_483_648
      && exponent <= 2_147_483_647
      ? { pow: [base, { lit: String(exponent) }] }
      : undefined;
  }
  if (block.type === "expr_if") {
    const condition = blockToBooleanExpr(inputBlock(block, "IF"));
    const thenBranch = blockToNumberExpr(inputBlock(block, "THEN"));
    const elseBranch = blockToNumberExpr(inputBlock(block, "ELSE"));
    return condition && thenBranch && elseBranch
      ? { if: condition, then: thenBranch, else: elseBranch }
      : undefined;
  }
  return undefined;
}

function blockToBooleanExpr(block: Blockly.Block | undefined): BooleanExpr | undefined {
  if (!block) return undefined;
  if (block.type === "expr_compare") {
    const operator = String(block.getFieldValue("OP"));
    const left = blockToNumberExpr(inputBlock(block, "LEFT"));
    const right = blockToNumberExpr(inputBlock(block, "RIGHT"));
    if (!comparisonOperators.includes(operator as typeof comparisonOperators[number])
      || !left
      || !right) return undefined;
    return { [operator]: [left, right] } as BooleanExpr;
  }
  if (block.type === "expr_logic") {
    const operator = String(block.getFieldValue("OP"));
    const left = blockToBooleanExpr(inputBlock(block, "LEFT"));
    const right = blockToBooleanExpr(inputBlock(block, "RIGHT"));
    if (!logicOperators.includes(operator as typeof logicOperators[number])
      || !left
      || !right) return undefined;
    return { [operator]: [left, right] } as BooleanExpr;
  }
  if (block.type === "expr_not") {
    const value = blockToBooleanExpr(inputBlock(block, "VALUE"));
    return value ? { not: value } : undefined;
  }
  return undefined;
}

export function blockToExpr(block: Blockly.Block): Expr | undefined {
  return blockToNumberExpr(block) ?? blockToBooleanExpr(block);
}

function readEntity(block: Blockly.Block): Entity {
  const children = chain(block.getInputTargetBlock("CHILDREN")).map(readEntity);
  return {
    id: block.getFieldValue("ID"),
    name: block.getFieldValue("NAME"),
    prob: blockToNumberExpr(inputBlock(block, "PROB")) ?? { lit: "0" },
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
  const stateVars = mergePreserved<ModelIr["stateVars"][number]>(
    chain(root.getInputTargetBlock("STATE")).map((block): ModelIr["stateVars"][number] => {
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
    }),
    preserved.stateVars,
  );
  const probRules = mergePreserved<ProbabilityRule>(
    chain(root.getInputTargetBlock("PROB_RULES")).map((block) => ({
      target: String(block.getFieldValue("TARGET")),
      expr: blockToNumberExpr(inputBlock(block, "EXPR")) ?? { lit: "0" },
      blockId: block.id,
    })),
    preserved.probRules,
  );
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
  let condition: BooleanExpr | undefined;
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
  const probability = numberExprToBlock(workspace, entity.prob);
  if (probability) {
    block.getInput("PROB")?.connection?.connect(probability.outputConnection!);
  }
  append(block.getInput("CHILDREN"), (entity.children ?? []).map((child) => makeEntity(workspace, child)));
  return block;
}

function literalValue(value: unknown): string | undefined {
  if (!value || typeof value !== "object") return undefined;
  const literal = (value as { lit?: unknown }).lit;
  return typeof literal === "string" ? literal : undefined;
}

function supportedEntity(entity: Entity): boolean {
  return numberExprState(entity.prob) !== undefined
    && (entity.children ?? []).every(supportedEntity);
}

function accumulatorBlockData(variable: ModelIr["stateVars"][number]) {
  if (variable.role !== "accumulator"
    || variable.clampPolicy === "error"
    || variable.update?.length !== 1) return undefined;
  const update = variable.update[0];
  const target = typeof update.when.leafOf === "string" ? update.when.leafOf : undefined;
  const add = recordValue(update.set)?.add;
  if (!target || !Array.isArray(add) || add.length !== 2) return undefined;
  const isSelf = (value: unknown) => Boolean(value && typeof value === "object"
    && (value as Record<string, unknown>).var === variable.id);
  const amount = isSelf(add[0]) ? literalValue(add[1])
    : isSelf(add[1]) ? literalValue(add[0]) : undefined;
  if (amount === undefined || !Number.isFinite(Number(amount)) || String(Number(amount)) !== amount) return undefined;
  return { target, amount };
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

function conditionBlockData(condition: BooleanExpr | undefined) {
  const ge = condition && "ge" in condition ? condition.ge : undefined;
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
      const expression = numberExprToBlock(workspace, rule.expr);
      if (expression) {
        const block = createBlock(workspace, "probability_rule");
        block.setFieldValue(rule.target, "TARGET");
        block.getInput("EXPR")?.connection?.connect(expression.outputConnection!);
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
