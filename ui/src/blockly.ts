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

export const CONTROL_VARIABLE_TYPE = "gacha-control";
export const ACCUMULATOR_VARIABLE_TYPE = "gacha-accumulator";
const VARIABLE_TYPES = [CONTROL_VARIABLE_TYPE, ACCUMULATOR_VARIABLE_TYPE];
const VARIABLE_TOOLBOX_CATEGORY = "GACHA_VARIABLES";
const CREATE_CONTROL_VARIABLE = "CREATE_GACHA_CONTROL_VARIABLE";
const CREATE_ACCUMULATOR_VARIABLE = "CREATE_GACHA_ACCUMULATOR_VARIABLE";
const MANAGE_VARIABLES = "MANAGE_GACHA_VARIABLES";

export type ExpressionContext =
  | "probability"
  | "transition"
  | "trigger"
  | "accumulator"
  | "condition";

type ContextConnection = Blockly.Connection & {
  gachaExpressionContext?: ExpressionContext;
  gachaVariableField?: string;
};

function installContextExtension(
  extension: string,
  inputName: string,
  context: ExpressionContext,
  variableField?: string,
) {
  if (Blockly.Extensions.isRegistered(extension)) return;
  Blockly.Extensions.register(extension, function (this: Blockly.Block) {
    const connection = this.getInput(inputName)?.connection as ContextConnection | undefined;
    if (!connection) return;
    connection.gachaExpressionContext = context;
    connection.gachaVariableField = variableField;
  });
}

installContextExtension("gacha_probability_context", "PROB", "probability");
installContextExtension("gacha_probability_rule_context", "EXPR", "probability");
installContextExtension("gacha_transition_context", "VALUE", "transition");
installContextExtension("gacha_trigger_context", "VALUE", "trigger");
installContextExtension("gacha_accumulator_context", "VALUE", "accumulator", "VAR");
installContextExtension("gacha_condition_context", "EXPR", "condition");

function variableFromBlock(
  block: Blockly.Block,
): Blockly.IVariableModel<Blockly.IVariableState> | null {
  const field = block.getField("VAR");
  return field instanceof Blockly.FieldVariable ? field.getVariable() : null;
}

function contextOnConnection(connection: Blockly.Connection | null | undefined) {
  const contextual = connection as ContextConnection | undefined;
  if (!contextual?.gachaExpressionContext) return undefined;
  const block = contextual.getSourceBlock();
  const target = contextual.gachaVariableField
    ? variableFromBlock(block)
    : null;
  return {
    context: contextual.gachaExpressionContext,
    accumulatorId: target?.getId(),
  };
}

function contextAboveBlock(block: Blockly.Block) {
  let current: Blockly.Block | null = block;
  while (current) {
    const parent = current.getParent();
    if (!parent) return undefined;
    for (const input of parent.inputList) {
      if (input.connection?.targetBlock() !== current) continue;
      const context = contextOnConnection(input.connection);
      if (context) return context;
    }
    current = parent;
  }
  return undefined;
}

export function expressionTreeAllowed(
  root: Blockly.Block,
  context: ExpressionContext,
  accumulatorId?: string,
): boolean {
  for (const block of root.getDescendants(false)) {
    if (block.type === "expr_entity_count" && context !== "condition") return false;
    if (block.type !== "expr_variable") continue;
    if (context === "condition") return false;
    const variable = variableFromBlock(block);
    if (!variable) return false;
    if (variable.getType() === CONTROL_VARIABLE_TYPE) continue;
    if (context === "accumulator"
      && variable.getType() === ACCUMULATOR_VARIABLE_TYPE
      && variable.getId() === accumulatorId) continue;
    return false;
  }
  return true;
}

if (!Blockly.Extensions.isRegistered("gacha_variable_reference_guard")) {
  Blockly.Extensions.register("gacha_variable_reference_guard", function (this: Blockly.Block) {
    const field = this.getField("VAR");
    if (!(field instanceof Blockly.FieldVariable)) return;
    field.setValidator((variableId) => {
      const context = contextAboveBlock(this);
      if (!context) return variableId;
      const variable = this.workspace.getVariableMap().getVariableById(variableId);
      if (!variable || context.context === "condition") return null;
      if (variable.getType() === CONTROL_VARIABLE_TYPE) return variableId;
      return context.context === "accumulator"
        && variable.getType() === ACCUMULATOR_VARIABLE_TYPE
        && variable.getId() === context.accumulatorId
        ? variableId
        : null;
    });
  });
}

if (!Blockly.Extensions.isRegistered("gacha_accumulator_target_guard")) {
  Blockly.Extensions.register("gacha_accumulator_target_guard", function (this: Blockly.Block) {
    const field = this.getField("VAR");
    if (!(field instanceof Blockly.FieldVariable)) return;
    field.setValidator((variableId) => {
      const expression = this.getInputTargetBlock("VALUE");
      return !expression || expressionTreeAllowed(expression, "accumulator", variableId)
        ? variableId
        : null;
    });
  });
}

export class GachaConnectionChecker extends Blockly.ConnectionChecker {
  override doTypeChecks(a: Blockly.Connection, b: Blockly.Connection): boolean {
    if (!super.doTypeChecks(a, b)) return false;
    const directContext = contextOnConnection(a) ?? contextOnConnection(b);
    const aBlock = a.getSourceBlock();
    const bBlock = b.getSourceBlock();
    const inheritedContext = directContext ?? contextAboveBlock(aBlock) ?? contextAboveBlock(bBlock);
    if (!inheritedContext) return true;
    const expressionRoot = aBlock.outputConnection ? aBlock : bBlock.outputConnection ? bBlock : undefined;
    return !expressionRoot || expressionTreeAllowed(
      expressionRoot,
      inheritedContext.context,
      inheritedContext.accumulatorId,
    );
  }
}

Blockly.common.defineBlocksWithJsonArray([
  {
    type: "model_container",
    message0: "가챠 모델",
    message1: "뽑기 결과 %1",
    args1: [{ type: "input_statement", name: "ENTITIES" }],
    message2: "집계 갱신 %1",
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
    extensions: ["gacha_probability_context"],
  },
  {
    type: "leaf_predicate",
    message0: "%1가 %2",
    args0: [
      { type: "field_input", name: "ENTITY", text: "__other__" },
      {
        type: "field_dropdown",
        name: "PREDICATE",
        options: [["나오면", "leafOf"], ["나오지 않으면", "notLeafOf"]],
      },
    ],
    output: "LeafPredicate",
    colour: 185,
    tooltip: "집계 값을 바꿀 결과 ID와 해당 결과의 출현 여부를 고릅니다.",
  },
  {
    type: "accumulator_update",
    message0: "집계 변수 %1 갱신",
    args0: [
      {
        type: "field_variable",
        name: "VAR",
        variable: "spent",
        variableTypes: [ACCUMULATOR_VARIABLE_TYPE],
        defaultType: ACCUMULATOR_VARIABLE_TYPE,
      },
    ],
    message1: "조건 %1",
    args1: [{ type: "input_value", name: "WHEN", check: "LeafPredicate" }],
    message2: "새 값 %1",
    args2: [{ type: "input_value", name: "VALUE", check: "Number" }],
    previousStatement: null,
    nextStatement: null,
    colour: 185,
    tooltip: "집계 변수, 결과 조건, 새 값을 계산할 숫자 식을 차례로 연결합니다.",
    extensions: ["gacha_accumulator_context", "gacha_accumulator_target_guard"],
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
    extensions: ["gacha_probability_rule_context"],
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
    args0: [{
      type: "field_variable",
      name: "VAR",
      variable: "pity",
      variableTypes: VARIABLE_TYPES,
      defaultType: CONTROL_VARIABLE_TYPE,
    }],
    output: "Number",
    colour: 205,
    tooltip: "변수 메뉴에서 만든 상태 변수를 고릅니다. 허용되지 않는 문맥에는 연결되지 않습니다.",
    extensions: ["gacha_variable_reference_guard"],
  },
  {
    type: "expr_entity_count",
    message0: "결과 개수 %1",
    args0: [{ type: "field_input", name: "ENTITY", text: "pickup" }],
    output: "Number",
    colour: 62,
    tooltip: "최초 달성 조건에서 누적 개수를 읽을 결과 ID를 입력합니다.",
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
    message0: "%1가 %2 상태 변수 %3를",
    args0: [
      { type: "field_input", name: "ENTITY", text: "star3" },
      {
        type: "field_dropdown",
        name: "PREDICATE",
        options: [["나오면", "leafOf"], ["나오지 않으면", "notLeafOf"]],
      },
      {
        type: "field_variable",
        name: "VAR",
        variable: "pity",
        variableTypes: [CONTROL_VARIABLE_TYPE],
        defaultType: CONTROL_VARIABLE_TYPE,
      },
    ],
    message1: "%1 로 설정",
    args1: [{ type: "input_value", name: "VALUE", check: "Number" }],
    previousStatement: null,
    nextStatement: null,
    colour: 155,
    tooltip: "결과 조건, 제어 변수, 새 값을 계산할 숫자 식을 차례로 연결합니다.",
    extensions: ["gacha_transition_context"],
  },
  {
    type: "transition_or_set",
    message0: "%1 또는 %2가 나오면 상태 변수 %3를",
    args0: [
      { type: "field_input", name: "ENTITY_LEFT", text: "star6" },
      { type: "field_input", name: "ENTITY_RIGHT", text: "star5" },
      {
        type: "field_variable",
        name: "VAR",
        variable: "highSeen",
        variableTypes: [CONTROL_VARIABLE_TYPE],
        defaultType: CONTROL_VARIABLE_TYPE,
      },
    ],
    message1: "%1 로 설정",
    args1: [{ type: "input_value", name: "VALUE", check: "Number" }],
    previousStatement: null,
    nextStatement: null,
    colour: 155,
    tooltip: "두 결과 중 하나가 나오면 제어 변수를 연결한 숫자 식의 값으로 바꿉니다.",
    extensions: ["gacha_transition_context"],
  },
  {
    type: "condition_expression",
    message0: "%1 일 때 최초 달성",
    args0: [{ type: "input_value", name: "EXPR", check: "Boolean" }],
    previousStatement: null,
    nextStatement: null,
    colour: 62,
    tooltip: "조건 전용 결과 개수와 비교·논리 블록을 연결합니다.",
    extensions: ["gacha_condition_context"],
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
  {
    type: "set_trigger",
    message0: "%1회 직후 상태 변수 %2를",
    args0: [
      { type: "field_number", name: "TRIAL", value: 10, min: 1, precision: 1 },
      {
        type: "field_variable",
        name: "VAR",
        variable: "guarantee",
        variableTypes: [CONTROL_VARIABLE_TYPE],
        defaultType: CONTROL_VARIABLE_TYPE,
      },
    ],
    message1: "%1 로 설정",
    args1: [{ type: "input_value", name: "VALUE", check: "Number" }],
    previousStatement: null,
    nextStatement: null,
    colour: 35,
    tooltip: "시행 번호, 제어 변수, 새 값을 계산할 숫자 식을 차례로 연결합니다.",
    extensions: ["gacha_trigger_context"],
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
      name: "변수",
      colour: "#498dba",
      custom: VARIABLE_TOOLBOX_CATEGORY,
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
                        LEFT: {
                          block: {
                            type: "expr_variable",
                            fields: { VAR: { name: "pity", type: CONTROL_VARIABLE_TYPE } },
                          },
                        },
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
                                    LEFT: {
                                      block: {
                                        type: "expr_variable",
                                        fields: { VAR: { name: "pity", type: CONTROL_VARIABLE_TYPE } },
                                      },
                                    },
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
                              LEFT: {
                                block: {
                                  type: "expr_variable",
                                  fields: { VAR: { name: "highSeen", type: CONTROL_VARIABLE_TYPE } },
                                },
                              },
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
        {
          kind: "block",
          type: "transition_set",
          fields: {
            VAR: { name: "pity", type: CONTROL_VARIABLE_TYPE },
          },
          inputs: { VALUE: { block: { type: "expr_literal", fields: { VALUE: "0" } } } },
        },
        {
          kind: "block",
          type: "transition_or_set",
          fields: {
            VAR: { name: "highSeen", type: CONTROL_VARIABLE_TYPE },
          },
          inputs: { VALUE: { block: { type: "expr_literal", fields: { VALUE: "1" } } } },
        },
      ],
    },
    {
      kind: "category",
      name: "시행 이벤트",
      colour: "#c88a3a",
      contents: [
        { kind: "block", type: "grant_trigger" },
        {
          kind: "block",
          type: "set_trigger",
          fields: {
            VAR: { name: "guarantee", type: CONTROL_VARIABLE_TYPE },
          },
          inputs: { VALUE: { block: { type: "expr_literal", fields: { VALUE: "1" } } } },
        },
      ],
    },
    {
      kind: "category",
      name: "조건",
      colour: "#b69837",
      contents: [{
        kind: "block",
        type: "condition_expression",
        inputs: {
          EXPR: {
            block: {
              type: "expr_compare",
              fields: { OP: "ge" },
              inputs: {
                LEFT: { block: { type: "expr_entity_count", fields: { ENTITY: "pickup" } } },
                RIGHT: { block: { type: "expr_literal", fields: { VALUE: "1" } } },
              },
            },
          },
        },
      }],
    },
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
  expression: NumberExpr;
}

interface TransitionOrBlockData {
  entities: [string, string];
  variable: string;
  expression: NumberExpr;
}

interface TriggerBlockData {
  trial: number;
  leaf: string;
  amount: number;
  consumesTrial: boolean;
  appliesTransitions: boolean;
}

interface SetTriggerBlockData {
  trial: number;
  variable: string;
  expression: NumberExpr;
}

interface PreservedUpdate {
  index: number;
  value: NonNullable<ModelIr["stateVars"][number]["update"]>[number];
}

export interface WorkspaceVariableDefinition {
  variableId: string;
  id: string;
  role: "control" | "accumulator";
  init: number;
  max?: number;
  name?: string;
  clampPolicy?: "saturate" | "error";
  blockId?: string;
}

interface VariableMetadata extends Omit<WorkspaceVariableDefinition, "variableId" | "id" | "role"> {
  preservedUpdates: PreservedUpdate[];
}

const roundTripState = new WeakMap<Blockly.Workspace, WorkspaceRoundTripState>();
const variableMetadata = new WeakMap<Blockly.Workspace, Map<string, VariableMetadata>>();

function roleForType(type: string): "control" | "accumulator" | undefined {
  if (type === CONTROL_VARIABLE_TYPE) return "control";
  if (type === ACCUMULATOR_VARIABLE_TYPE) return "accumulator";
  return undefined;
}

function typeForRole(role: "control" | "accumulator") {
  return role === "control" ? CONTROL_VARIABLE_TYPE : ACCUMULATOR_VARIABLE_TYPE;
}

function metadataMap(workspace: Blockly.Workspace) {
  let map = variableMetadata.get(workspace);
  if (!map) {
    map = new Map();
    variableMetadata.set(workspace, map);
  }
  return map;
}

function defaultMetadata(role: "control" | "accumulator"): VariableMetadata {
  return {
    init: 0,
    max: role === "control" ? 89 : 60_000,
    ...(role === "accumulator" ? { clampPolicy: "saturate" as const } : {}),
    preservedUpdates: [],
  };
}

export function listWorkspaceVariables(workspace: Blockly.Workspace): WorkspaceVariableDefinition[] {
  const metadata = metadataMap(workspace);
  return workspace.getVariableMap().getAllVariables().flatMap((variable) => {
    const role = roleForType(variable.getType());
    if (!role) return [];
    const definition = metadata.get(variable.getId()) ?? defaultMetadata(role);
    return [{
      variableId: variable.getId(),
      id: variable.getName(),
      role,
      init: definition.init,
      max: definition.max,
      ...(definition.name ? { name: definition.name } : {}),
      ...(definition.clampPolicy ? { clampPolicy: definition.clampPolicy } : {}),
      ...(definition.blockId ? { blockId: definition.blockId } : {}),
    }];
  });
}

export function saveWorkspaceVariable(
  workspace: Blockly.Workspace,
  draft: Omit<WorkspaceVariableDefinition, "variableId">,
  variableId?: string,
): WorkspaceVariableDefinition {
  const id = draft.id.trim();
  if (!id) throw new Error("변수 ID를 입력하세요.");
  if (!Number.isInteger(draft.init) || draft.init < 0) {
    throw new Error("초기값은 0 이상의 정수여야 합니다.");
  }
  if (draft.max !== undefined && (!Number.isInteger(draft.max) || draft.max < 0)) {
    throw new Error("상한은 0 이상의 정수여야 합니다.");
  }
  if (draft.max !== undefined && draft.init > draft.max) {
    throw new Error("초기값은 상한보다 클 수 없습니다.");
  }
  const variables = workspace.getVariableMap();
  const conflict = variables.getAllVariables()
    .find((variable) => variable.getName() === id && variable.getId() !== variableId);
  if (conflict) throw new Error(`이미 '${id}' 변수가 있습니다.`);

  let variable = variableId ? variables.getVariableById(variableId) : null;
  const previousMetadata = variable ? metadataMap(workspace).get(variable.getId()) : undefined;
  if (!variable) {
    variable = variables.createVariable(id, typeForRole(draft.role));
  } else {
    if (variable.getType() !== typeForRole(draft.role)) {
      throw new Error("기존 변수의 역할은 바꿀 수 없습니다. 새 변수를 만든 뒤 참조를 옮기세요.");
    }
    if (variable.getName() !== id) variable = variables.renameVariable(variable, id);
  }
  const nextMetadata: VariableMetadata = {
    init: draft.init,
    max: draft.max,
    ...(draft.name?.trim() ? { name: draft.name.trim() } : {}),
    ...(draft.clampPolicy ? { clampPolicy: draft.clampPolicy } : {}),
    ...(draft.blockId ? { blockId: draft.blockId } : {}),
    preservedUpdates: previousMetadata?.preservedUpdates ?? [],
  };
  metadataMap(workspace).set(variable.getId(), nextMetadata);
  if (variableId && variableId !== variable.getId()) metadataMap(workspace).delete(variableId);
  return {
    variableId: variable.getId(),
    id: variable.getName(),
    role: draft.role,
    init: nextMetadata.init,
    max: nextMetadata.max,
    ...(nextMetadata.name ? { name: nextMetadata.name } : {}),
    ...(nextMetadata.clampPolicy ? { clampPolicy: nextMetadata.clampPolicy } : {}),
    ...(nextMetadata.blockId ? { blockId: nextMetadata.blockId } : {}),
  };
}

export function deleteWorkspaceVariable(workspace: Blockly.Workspace, variableId: string) {
  const variable = workspace.getVariableMap().getVariableById(variableId);
  if (!variable) return;
  workspace.getVariableMap().deleteVariable(variable);
  metadataMap(workspace).delete(variableId);
}

export function installVariableToolbox(
  workspace: Blockly.WorkspaceSvg,
  handlers: {
    create: (role: "control" | "accumulator") => void;
    manage: () => void;
  },
) {
  workspace.registerButtonCallback(CREATE_CONTROL_VARIABLE, () => handlers.create("control"));
  workspace.registerButtonCallback(CREATE_ACCUMULATOR_VARIABLE, () => handlers.create("accumulator"));
  workspace.registerButtonCallback(MANAGE_VARIABLES, handlers.manage);
  workspace.registerToolboxCategoryCallback(VARIABLE_TOOLBOX_CATEGORY, () => {
    const variables = workspace.getVariableMap().getAllVariables()
      .filter((variable) => roleForType(variable.getType()));
    const accumulator = variables.find((variable) => variable.getType() === ACCUMULATOR_VARIABLE_TYPE);
    const contents: Blockly.utils.toolbox.FlyoutItemInfoArray = [
      { kind: "label", text: "시행 횟수 · 고정" },
      { kind: "block", type: "expr_trial" },
      { kind: "button", text: "제어 변수 만들기", callbackkey: CREATE_CONTROL_VARIABLE },
      { kind: "button", text: "집계 변수 만들기", callbackkey: CREATE_ACCUMULATOR_VARIABLE },
      { kind: "button", text: "변수 관리…", callbackkey: MANAGE_VARIABLES },
      ...variables.map((variable): Blockly.utils.toolbox.BlockInfo => ({
        kind: "block",
        type: "expr_variable",
        fields: {
          VAR: {
            id: variable.getId(),
            name: variable.getName(),
            type: variable.getType(),
          },
        },
      })),
    ];
    if (accumulator) {
      contents.push({
        kind: "block",
        type: "accumulator_update",
        fields: {
          VAR: {
            id: accumulator.getId(),
            name: accumulator.getName(),
            type: accumulator.getType(),
          },
        },
        inputs: {
          WHEN: {
            block: {
              type: "leaf_predicate",
              fields: { ENTITY: "__other__", PREDICATE: "leafOf" },
            },
          },
          VALUE: {
            block: {
              type: "expr_arithmetic",
              fields: { OP: "add" },
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
                RIGHT: { block: { type: "expr_literal", fields: { VALUE: "1" } } },
              },
            },
          },
        },
      });
    }
    return contents;
  });
  refreshVariableToolbox(workspace);
}

function replaceToolboxVariable(
  value: unknown,
  variable: Blockly.IVariableModel<Blockly.IVariableState>,
) {
  if (!value || typeof value !== "object") return;
  const record = value as Record<string, unknown>;
  const fields = record.fields as Record<string, unknown> | undefined;
  if (fields && "VAR" in fields) {
    fields.VAR = {
      id: variable.getId(),
      name: variable.getName(),
      type: variable.getType(),
    };
  }
  for (const child of Object.values(record)) replaceToolboxVariable(child, variable);
}

export function refreshVariableToolbox(workspace: Blockly.WorkspaceSvg) {
  const definition = structuredClone(toolbox as Blockly.utils.toolbox.ToolboxInfo);
  const control = workspace.getVariableMap().getAllVariables()
    .find((variable) => variable.getType() === CONTROL_VARIABLE_TYPE);
  for (const item of definition.contents) {
    if (!("name" in item) || !("contents" in item)) continue;
    const variableDependent = item.name === "결과 변화"
      ? item.contents
      : item.name === "시행 이벤트"
        ? item.contents.filter((entry) => "type" in entry && entry.type === "set_trigger")
        : item.name === "확률 규칙"
          ? item.contents.filter((_, index) => index > 0)
          : [];
    for (const entry of variableDependent) {
      if (!("type" in entry)) continue;
      entry.enabled = Boolean(control);
      if (control) replaceToolboxVariable(entry, control);
    }
  }
  workspace.updateToolbox(definition);
}

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
  if (state) hydrateVariableFields(state, workspace, "probability");
  return state
    ? Blockly.serialization.blocks.append(state, workspace)
    : undefined;
}

function nestedSerializedBlocks(state: SerializedBlock): SerializedBlock[] {
  const children: SerializedBlock[] = [];
  for (const input of Object.values(state.inputs ?? {})) {
    if (input.block) children.push(input.block);
    if (input.shadow) children.push(input.shadow);
  }
  return children;
}

function hydrateVariableFields(
  state: SerializedBlock,
  workspace: Blockly.Workspace,
  context: ExpressionContext,
  accumulatorName?: string,
) {
  if (state.type === "expr_variable") {
    const name = String(state.fields?.VAR ?? "");
    const expectedType = context === "accumulator" && name === accumulatorName
      ? ACCUMULATOR_VARIABLE_TYPE
      : CONTROL_VARIABLE_TYPE;
    const variableMap = workspace.getVariableMap();
    const variable = variableMap.getVariable(name, expectedType)
      ?? variableMap.createVariable(name, expectedType);
    state.fields = {
      ...state.fields,
      VAR: {
        id: variable.getId(),
        name: variable.getName(),
        type: variable.getType(),
      },
    };
  }
  for (const child of nestedSerializedBlocks(state)) {
    hydrateVariableFields(child, workspace, context, accumulatorName);
  }
}

function numberExprToBlock(
  workspace: Blockly.Workspace,
  expression: unknown,
  context: ExpressionContext = "probability",
  accumulatorName?: string,
): Blockly.Block | undefined {
  const state = numberExprState(expression);
  if (state) hydrateVariableFields(state, workspace, context, accumulatorName);
  return state
    ? Blockly.serialization.blocks.append(state, workspace)
    : undefined;
}

function decodeEntityCountReference(name: string): string {
  if (!name.startsWith("n")) return name;
  const stripped = name.slice(1);
  return stripped.charAt(0).toLowerCase() + stripped.slice(1);
}

export function encodeEntityCountReference(entityId: string): {
  variable: string;
  canonical: boolean;
} {
  const candidate = `n${entityId.charAt(0).toUpperCase()}${entityId.slice(1)}`;
  return decodeEntityCountReference(candidate) === entityId
    ? { variable: candidate, canonical: true }
    : { variable: entityId, canonical: false };
}

function conditionExprState(
  expression: BooleanExpr,
  entityIds: Set<string>,
): SerializedBlock | undefined {
  const state = booleanExprState(expression);
  if (!state) return undefined;
  let valid = true;
  const rewrite = (block: SerializedBlock) => {
    if (block.type === "expr_variable") {
      const variable = String(block.fields?.VAR ?? "");
      const entity = decodeEntityCountReference(variable);
      if (!entityIds.has(entity)) {
        valid = false;
        return;
      }
      block.type = "expr_entity_count";
      block.fields = { ENTITY: entity };
    }
    for (const child of nestedSerializedBlocks(block)) rewrite(child);
  };
  rewrite(state);
  return valid ? state : undefined;
}

function conditionExprToBlock(
  workspace: Blockly.Workspace,
  expression: BooleanExpr,
  entityIds: Set<string>,
) {
  const state = conditionExprState(expression, entityIds);
  return state ? Blockly.serialization.blocks.append(state, workspace) : undefined;
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
    const variable = variableFromBlock(block);
    return variable ? { var: variable.getName() } : undefined;
  }
  if (block.type === "expr_entity_count") {
    const entity = String(block.getFieldValue("ENTITY"));
    const encoded = encodeEntityCountReference(entity);
    block.setWarningText(encoded.canonical
      ? null
      : "대문자로 시작하는 ID는 n 접두 표기가 손실되므로 원본 ID로 저장합니다.");
    return { var: encoded.variable };
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
  const accumulatorUpdates = new Map<string, NonNullable<ModelIr["stateVars"][number]["update"]>>();
  for (const block of chain(root.getInputTargetBlock("STATE"))) {
    if (block.type !== "accumulator_update") continue;
    const variable = variableFromBlock(block);
    const condition = inputBlock(block, "WHEN");
    const expression = blockToNumberExpr(inputBlock(block, "VALUE"));
    if (!variable || !condition || !expression) continue;
    const entity = String(condition.getFieldValue("ENTITY"));
    const when = condition.getFieldValue("PREDICATE") === "notLeafOf"
      ? { not: { leafOf: entity } }
      : { leafOf: entity };
    const updates = accumulatorUpdates.get(variable.getId()) ?? [];
    updates.push({ when, set: expression });
    accumulatorUpdates.set(variable.getId(), updates);
  }
  const stateVars = mergePreserved<ModelIr["stateVars"][number]>(
    listWorkspaceVariables(workspace).map((definition): ModelIr["stateVars"][number] => {
      const metadata = metadataMap(workspace).get(definition.variableId)
        ?? defaultMetadata(definition.role);
      const updates = mergePreserved(
        accumulatorUpdates.get(definition.variableId) ?? [],
        metadata.preservedUpdates,
      );
      return {
        id: definition.id,
        init: definition.init,
        ...(definition.max !== undefined ? { max: definition.max } : {}),
        role: definition.role,
        ...(definition.name ? { name: definition.name } : {}),
        ...(updates.length ? { update: updates } : {}),
        ...(definition.clampPolicy ? { clampPolicy: definition.clampPolicy } : {}),
        ...(definition.blockId ? { blockId: definition.blockId } : {}),
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
    const variable = variableFromBlock(block);
    const expression = blockToNumberExpr(inputBlock(block, "VALUE")) ?? { lit: "0" };
    const set = { [variable?.getName() ?? ""]: expression };
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
  const triggers = mergePreserved(chain(root.getInputTargetBlock("TRIGGERS")).map((block) => {
    if (block.type === "set_trigger") {
      const variable = variableFromBlock(block);
      return {
        at: { trialCount: Number(block.getFieldValue("TRIAL")) },
        set: {
          [variable?.getName() ?? ""]: blockToNumberExpr(inputBlock(block, "VALUE")) ?? { lit: "0" },
        },
        blockId: block.id,
      };
    }
    return {
      at: { trialCount: Number(block.getFieldValue("TRIAL")) },
      grant: {
        leaf: block.getFieldValue("LEAF"),
        amount: Number(block.getFieldValue("AMOUNT")),
        consumesTrial: block.getFieldValue("CONSUMES") === "TRUE",
        appliesTransitions: block.getFieldValue("APPLIES") === "TRUE",
      },
      blockId: block.id,
    };
  }), preserved.triggers);
  const conditionBlock = root.getInputTargetBlock("CONDITION");
  const condition = conditionBlock?.type === "condition_expression"
    ? blockToBooleanExpr(inputBlock(conditionBlock, "EXPR"))
    : undefined;
  if (condition) {
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

function assignVariableField(
  block: Blockly.Block,
  fieldName: string,
  variable: Blockly.IVariableModel<Blockly.IVariableState>,
) {
  const field = block.getField(fieldName);
  const temporary = field instanceof Blockly.FieldVariable ? field.getVariable() : null;
  block.setFieldValue(variable.getId(), fieldName);
  if (temporary
    && temporary.getId() !== variable.getId()
    && !metadataMap(block.workspace).has(temporary.getId())
    && Blockly.Variables.getVariableUsesById(block.workspace, temporary.getId()).length === 0) {
    block.workspace.getVariableMap().deleteVariable(temporary);
  }
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

function expressionVariableNames(expression: unknown): string[] {
  if (!expression || typeof expression !== "object") return [];
  if (Array.isArray(expression)) return expression.flatMap(expressionVariableNames);
  const record = expression as Record<string, unknown>;
  return [
    ...(typeof record.var === "string" ? [record.var] : []),
    ...Object.values(record).flatMap(expressionVariableNames),
  ];
}

function expressionAllowedForIr(
  expression: unknown,
  context: ExpressionContext,
  stateVars: ModelIr["stateVars"],
  accumulatorId?: string,
  entityIds?: Set<string>,
) {
  const controls = new Set(stateVars
    .filter((variable) => variable.role === "control")
    .map((variable) => variable.id));
  return expressionVariableNames(expression).every((name) => {
    if (context === "condition") {
      return entityIds?.has(decodeEntityCountReference(name)) ?? false;
    }
    if (controls.has(name)) return true;
    return context === "accumulator" && name === accumulatorId;
  });
}

function supportedEntity(entity: Entity, stateVars: ModelIr["stateVars"]): boolean {
  return numberExprState(entity.prob) !== undefined
    && expressionAllowedForIr(entity.prob, "probability", stateVars)
    && (entity.children ?? []).every((child) => supportedEntity(child, stateVars));
}

function transitionBlockData(
  transition: unknown,
  stateVars: ModelIr["stateVars"],
): TransitionBlockData | undefined {
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
  const control = stateVars.some((state) => state.role === "control" && state.id === variable);
  return control
    && numberExprState(expression)
    && expressionAllowedForIr(expression, "transition", stateVars)
    ? { entity, predicate, variable, expression: expression as NumberExpr }
    : undefined;
}

function transitionOrBlockData(
  transition: unknown,
  stateVars: ModelIr["stateVars"],
): TransitionOrBlockData | undefined {
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
  const control = stateVars.some((state) => state.role === "control" && state.id === variable);
  return control
    && numberExprState(expression)
    && expressionAllowedForIr(expression, "transition", stateVars)
    ? {
        entities: [leftEntity, rightEntity],
        variable,
        expression: expression as NumberExpr,
      }
    : undefined;
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

function setTriggerBlockData(
  trigger: unknown,
  stateVars: ModelIr["stateVars"],
): SetTriggerBlockData | undefined {
  if (!trigger || typeof trigger !== "object") return undefined;
  const record = trigger as Record<string, unknown>;
  const at = record.at as Record<string, unknown> | undefined;
  const set = record.set as Record<string, unknown> | undefined;
  const variables = set ? Object.keys(set) : [];
  if (typeof at?.trialCount !== "number" || record.grant || variables.length !== 1) return undefined;
  const variable = variables[0];
  const expression = set![variable];
  const control = stateVars.some((state) => state.role === "control" && state.id === variable);
  return control
    && numberExprState(expression)
    && expressionAllowedForIr(expression, "trigger", stateVars)
    ? { trial: at.trialCount, variable, expression: expression as NumberExpr }
    : undefined;
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
    workspace.getVariableMap().clear();
    const metadata = new Map<string, VariableMetadata>();
    variableMetadata.set(workspace, metadata);
    for (const [index, variable] of ir.stateVars.entries()) {
      if (variable.role === "stat") {
        preserved.stateVars.push({ index, value: structuredClone(variable) });
        unsupported.push({ path: `stateVars[${index}]`, description: `통계 변수 '${variable.id}'` });
        continue;
      }
      const model = workspace.getVariableMap().createVariable(
        variable.id,
        typeForRole(variable.role),
      );
      metadata.set(model.getId(), {
        init: variable.init,
        max: variable.max,
        ...(variable.name ? { name: variable.name } : {}),
        ...(variable.clampPolicy ? { clampPolicy: variable.clampPolicy } : {}),
        ...(variable.blockId ? { blockId: variable.blockId } : {}),
        preservedUpdates: [],
      });
    }
    const root = createBlock(workspace, "model_container");
    root.moveBy(32, 28);
    append(root.getInput("ENTITIES"), ir.entities.flatMap((entity, index) => {
      if (supportedEntity(entity, ir.stateVars)) return [makeEntity(workspace, entity)];
      preserved.entities.push({ index, value: structuredClone(entity) });
      unsupported.push({ path: `entities[${index}]`, description: `뽑기 결과 '${entity.id}'의 미지원 또는 허용되지 않는 확률식` });
      return [];
    }));
    append(root.getInput("STATE"), ir.stateVars.flatMap((variable, stateIndex) => {
      if (variable.role !== "accumulator") {
        if (variable.role === "control" && variable.update?.length) {
          const model = workspace.getVariableMap().getVariable(variable.id, CONTROL_VARIABLE_TYPE);
          const variableState = model && metadata.get(model.getId());
          if (variableState) {
            variableState.preservedUpdates = variable.update.map((value, index) => ({
              index,
              value: structuredClone(value),
            }));
            unsupported.push({
              path: `stateVars[${stateIndex}].update`,
              description: `제어 변수 '${variable.id}'의 갱신식`,
            });
          }
        }
        return [];
      }
      const model = workspace.getVariableMap().getVariable(variable.id, ACCUMULATOR_VARIABLE_TYPE);
      if (!model) return [];
      return (variable.update ?? []).flatMap((update, updateIndex) => {
        const when = recordValue(update.when);
        let entity = when?.leafOf;
        let predicate = "leafOf";
        if (typeof entity !== "string") {
          const not = recordValue(when?.not);
          entity = not?.leafOf;
          predicate = "notLeafOf";
        }
        const supported = typeof entity === "string"
          && numberExprState(update.set)
          && expressionAllowedForIr(update.set, "accumulator", ir.stateVars, variable.id);
        if (!supported) {
          metadata.get(model.getId())?.preservedUpdates.push({
            index: updateIndex,
            value: structuredClone(update),
          });
          unsupported.push({
            path: `stateVars[${stateIndex}].update[${updateIndex}]`,
            description: `집계 변수 '${variable.id}'의 미지원 또는 허용되지 않는 갱신식`,
          });
          return [];
        }
        const block = createBlock(workspace, "accumulator_update");
        assignVariableField(block, "VAR", model);
        const condition = createBlock(workspace, "leaf_predicate");
        condition.setFieldValue(entity, "ENTITY");
        condition.setFieldValue(predicate, "PREDICATE");
        block.getInput("WHEN")?.connection?.connect(condition.outputConnection!);
        const expression = numberExprToBlock(workspace, update.set, "accumulator", variable.id);
        if (expression) block.getInput("VALUE")?.connection?.connect(expression.outputConnection!);
        return [block];
      });
    }));
    append(root.getInput("PROB_RULES"), ir.probRules.flatMap((rule, index) => {
      const supported = numberExprState(rule.expr)
        && expressionAllowedForIr(rule.expr, "probability", ir.stateVars);
      const expression = supported
        ? numberExprToBlock(workspace, rule.expr, "probability")
        : undefined;
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
      const data = transitionBlockData(transition, ir.stateVars);
      if (data) {
        const block = createBlock(workspace, "transition_set");
        block.setFieldValue(data.entity, "ENTITY");
        block.setFieldValue(data.predicate, "PREDICATE");
        const variable = workspace.getVariableMap().getVariable(data.variable, CONTROL_VARIABLE_TYPE);
        if (!variable) return [];
        assignVariableField(block, "VAR", variable);
        const expression = numberExprToBlock(workspace, data.expression, "transition");
        if (expression) block.getInput("VALUE")?.connection?.connect(expression.outputConnection!);
        return [block];
      }
      const orData = transitionOrBlockData(transition, ir.stateVars);
      if (orData) {
        const block = createBlock(workspace, "transition_or_set");
        block.setFieldValue(orData.entities[0], "ENTITY_LEFT");
        block.setFieldValue(orData.entities[1], "ENTITY_RIGHT");
        const variable = workspace.getVariableMap().getVariable(orData.variable, CONTROL_VARIABLE_TYPE);
        if (!variable) return [];
        assignVariableField(block, "VAR", variable);
        const expression = numberExprToBlock(workspace, orData.expression, "transition");
        if (expression) block.getInput("VALUE")?.connection?.connect(expression.outputConnection!);
        return [block];
      }
      preserved.transitions.push({ index, value: structuredClone(transition) });
      unsupported.push({ path: `transitions[${index}]`, description: `결과 변화 ${index + 1}의 일반 술어 또는 표현식` });
      return [];
    }));
    append(root.getInput("TRIGGERS"), ir.triggers.flatMap((trigger, index) => {
      const data = triggerBlockData(trigger);
      if (data) {
        const block = createBlock(workspace, "grant_trigger");
        block.setFieldValue(data.trial, "TRIAL");
        block.setFieldValue(data.leaf, "LEAF");
        block.setFieldValue(data.amount, "AMOUNT");
        block.setFieldValue(data.consumesTrial ? "TRUE" : "FALSE", "CONSUMES");
        block.setFieldValue(data.appliesTransitions ? "TRUE" : "FALSE", "APPLIES");
        return [block];
      }
      const setData = setTriggerBlockData(trigger, ir.stateVars);
      if (setData) {
        const block = createBlock(workspace, "set_trigger");
        block.setFieldValue(setData.trial, "TRIAL");
        const variable = workspace.getVariableMap().getVariable(setData.variable, CONTROL_VARIABLE_TYPE);
        if (!variable) return [];
        assignVariableField(block, "VAR", variable);
        const expression = numberExprToBlock(workspace, setData.expression, "trigger");
        if (expression) block.getInput("VALUE")?.connection?.connect(expression.outputConnection!);
        return [block];
      }
      preserved.triggers.push({ index, value: structuredClone(trigger) });
      unsupported.push({ path: `triggers[${index}]`, description: `시행 이벤트 ${index + 1}의 미지원 또는 허용되지 않는 동작` });
      return [];
    }));
    const entityIds = new Set<string>();
    const collectEntityIds = (entities: Entity[]) => {
      for (const entity of entities) {
        entityIds.add(entity.id);
        collectEntityIds(entity.children ?? []);
      }
    };
    collectEntityIds(ir.entities);
    const conditionExpression = ir.run.condition
      && expressionAllowedForIr(ir.run.condition, "condition", ir.stateVars, undefined, entityIds)
      ? conditionExprToBlock(workspace, ir.run.condition, entityIds)
      : undefined;
    if (conditionExpression) {
      const block = createBlock(workspace, "condition_expression");
      block.getInput("EXPR")?.connection?.connect(conditionExpression.outputConnection!);
      append(root.getInput("CONDITION"), [block]);
    } else if (ir.run.condition) {
      preserved.condition = structuredClone(ir.run.condition);
      unsupported.push({ path: "run.condition", description: "미지원 또는 존재하지 않는 결과를 참조하는 최초 달성 조건식" });
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
