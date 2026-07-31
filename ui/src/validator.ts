import type {
  BooleanExpr,
  Diagnostic,
  Entity,
  LeafView,
  ModelIr,
  NumberExpr,
  ValidationView,
} from "./types";

// Keep these thresholds in sync with crates/gacha-core/src/compile.rs,
// DEFAULT_DP_LAYER_STATE_LIMIT ↔ engine_dp::DEFAULT_DP_MAX_LAYER_STATES and
// DP_ESTIMATED_STATE_HARD_LIMIT ↔ compile::DP_ESTIMATED_STATE_LIMIT.
export const ACCUMULATOR_TABLE_WARNING_ENTRIES = 500_000;
export const ACCUMULATOR_TABLE_MAX_ENTRIES = 10_000_000;
export const DEFAULT_DP_LAYER_STATE_LIMIT = 1_000_000;
export const DP_ESTIMATED_STATE_HARD_LIMIT = 50_000_000;

export function parseExactLiteral(value: string): { numerator: bigint; denominator: bigint } {
  const source = value.trim();
  if (source.includes("/")) {
    const pieces = source.split("/");
    if (pieces.length !== 2) throw new Error("잘못된 분수 리터럴");
    const denominator = BigInt(pieces[1]);
    if (denominator === 0n) throw new Error("분모는 0일 수 없습니다");
    return reduce(BigInt(pieces[0]), denominator);
  }
  const match = /^([+-]?)(\d*)(?:\.(\d*))?(?:[eE]([+-]?\d+))?$/.exec(source);
  if (!match || (!match[2] && !match[3])) throw new Error("잘못된 확률 리터럴");
  const sign = match[1] === "-" ? -1n : 1n;
  const whole = match[2] || "0";
  const fraction = match[3] || "";
  const exponent = Number(match[4] || 0);
  let numerator = sign * BigInt(whole + fraction);
  const scale = fraction.length - exponent;
  if (scale >= 0) return reduce(numerator, 10n ** BigInt(scale));
  numerator *= 10n ** BigInt(-scale);
  return reduce(numerator, 1n);
}

function reduce(numerator: bigint, denominator: bigint) {
  if (denominator < 0n) {
    numerator = -numerator;
    denominator = -denominator;
  }
  let a = numerator < 0n ? -numerator : numerator;
  let b = denominator;
  while (b) [a, b] = [b, a % b];
  return { numerator: numerator / a, denominator: denominator / a };
}

const decimal = (r: { numerator: bigint; denominator: bigint }) =>
  Number(r.numerator) / Number(r.denominator);

function evaluatePreviewNumber(
  expression: NumberExpr,
  variables: Map<string, number>,
  trial: number,
): number {
  if ("lit" in expression) return decimal(parseExactLiteral(expression.lit));
  if ("var" in expression) {
    const value = variables.get(expression.var);
    if (value === undefined) throw new Error(`알 수 없는 상태 변수: ${expression.var}`);
    return value;
  }
  if ("trial" in expression) return trial;
  if ("if" in expression) {
    return evaluatePreviewBoolean(expression.if, variables, trial)
      ? evaluatePreviewNumber(expression.then, variables, trial)
      : evaluatePreviewNumber(expression.else, variables, trial);
  }
  if ("add" in expression) {
    return evaluatePreviewNumber(expression.add[0], variables, trial)
      + evaluatePreviewNumber(expression.add[1], variables, trial);
  }
  if ("sub" in expression) {
    return evaluatePreviewNumber(expression.sub[0], variables, trial)
      - evaluatePreviewNumber(expression.sub[1], variables, trial);
  }
  if ("mul" in expression) {
    return evaluatePreviewNumber(expression.mul[0], variables, trial)
      * evaluatePreviewNumber(expression.mul[1], variables, trial);
  }
  if ("div" in expression) {
    const denominator = evaluatePreviewNumber(expression.div[1], variables, trial);
    if (denominator === 0) throw new Error("0으로 나눌 수 없습니다");
    return evaluatePreviewNumber(expression.div[0], variables, trial) / denominator;
  }
  if ("neg" in expression) return -evaluatePreviewNumber(expression.neg, variables, trial);
  if ("abs" in expression) return Math.abs(evaluatePreviewNumber(expression.abs, variables, trial));
  if ("floor" in expression) return Math.floor(evaluatePreviewNumber(expression.floor, variables, trial));
  if ("ceil" in expression) return Math.ceil(evaluatePreviewNumber(expression.ceil, variables, trial));
  if ("round" in expression) {
    return Math.floor(evaluatePreviewNumber(expression.round, variables, trial) + 0.5);
  }
  if ("min" in expression) {
    return Math.min(
      evaluatePreviewNumber(expression.min[0], variables, trial),
      evaluatePreviewNumber(expression.min[1], variables, trial),
    );
  }
  if ("max" in expression) {
    return Math.max(
      evaluatePreviewNumber(expression.max[0], variables, trial),
      evaluatePreviewNumber(expression.max[1], variables, trial),
    );
  }
  if ("clamp" in expression) {
    const value = evaluatePreviewNumber(expression.clamp[0], variables, trial);
    const low = evaluatePreviewNumber(expression.clamp[1], variables, trial);
    const high = evaluatePreviewNumber(expression.clamp[2], variables, trial);
    return Math.min(Math.max(value, low), high);
  }
  const exponent = parseExactLiteral(expression.pow[1].lit);
  if (exponent.denominator !== 1n) throw new Error("거듭제곱 지수는 정수여야 합니다");
  return evaluatePreviewNumber(expression.pow[0], variables, trial)
    ** Number(exponent.numerator);
}

function evaluatePreviewBoolean(
  expression: BooleanExpr,
  variables: Map<string, number>,
  trial: number,
): boolean {
  if ("and" in expression) {
    return evaluatePreviewBoolean(expression.and[0], variables, trial)
      && evaluatePreviewBoolean(expression.and[1], variables, trial);
  }
  if ("or" in expression) {
    return evaluatePreviewBoolean(expression.or[0], variables, trial)
      || evaluatePreviewBoolean(expression.or[1], variables, trial);
  }
  if ("not" in expression) return !evaluatePreviewBoolean(expression.not, variables, trial);
  if ("xor" in expression) {
    return evaluatePreviewBoolean(expression.xor[0], variables, trial)
      !== evaluatePreviewBoolean(expression.xor[1], variables, trial);
  }
  const operands = "eq" in expression ? expression.eq
    : "ne" in expression ? expression.ne
      : "lt" in expression ? expression.lt
        : "le" in expression ? expression.le
          : "gt" in expression ? expression.gt
            : expression.ge;
  const left = evaluatePreviewNumber(operands[0], variables, trial);
  const right = evaluatePreviewNumber(operands[1], variables, trial);
  if ("eq" in expression) return left === right;
  if ("ne" in expression) return left !== right;
  if ("lt" in expression) return left < right;
  if ("le" in expression) return left <= right;
  if ("gt" in expression) return left > right;
  return left >= right;
}

export function validateLocally(ir: ModelIr): ValidationView {
  const diagnostics: Diagnostic[] = [];
  const ids = new Set<string>();
  const leaves: LeafView[] = [];
  const leafAncestors = new Map<string, string[]>();
  const previewVariables = new Map(ir.stateVars
    .filter((variable) => variable.role === "control")
    .map((variable) => [variable.id, variable.init]));
  const previewRules = new Map(ir.probRules.map((rule) => [rule.target, rule.expr]));

  function walk(entity: Entity, ancestors: string[]): number {
    if (ids.has(entity.id)) {
      diagnostics.push({ code: "E001", severity: "error", message: `중복 엔티티 ID: ${entity.id}`, blockId: entity.blockId });
    }
    ids.add(entity.id);
    let probability = 0;
    try {
      probability = evaluatePreviewNumber(
        previewRules.get(entity.id) ?? entity.prob,
        previewVariables,
        1,
      );
      if (probability < 0) diagnostics.push({ code: "E002", severity: "error", message: `${entity.name} 확률이 음수입니다`, blockId: entity.blockId });
    } catch (error) {
      diagnostics.push({ code: "E006", severity: "error", message: String(error), blockId: entity.blockId });
    }
    const children = entity.children ?? [];
    if (!children.length) {
      leaves.push({ id: entity.id, name: entity.name, probability });
      leafAncestors.set(entity.id, ancestors);
      return probability;
    }
    const childStart = leaves.length;
    const childTotal: number = children.reduce((sum, child) => sum + walk(child, [...ancestors, entity.id]), 0);
    if (childTotal > probability + 1e-15) {
      diagnostics.push({ code: "W001", severity: "warning", message: `${entity.name}의 자식 확률 합이 부모보다 큽니다`, blockId: entity.blockId });
      if (ir.nestingPolicy === "clampChildren" && childTotal > 0) {
        const scale = probability / childTotal;
        for (let i = childStart; i < leaves.length; i += 1) leaves[i].probability *= scale;
      }
    }
    leaves.push({
      id: `${entity.id}__self`,
      name: `${entity.name}(전용)`,
      probability: Math.max(0, probability - Math.min(childTotal, probability)),
    });
    leafAncestors.set(`${entity.id}__self`, [...ancestors, entity.id]);
    return Math.max(probability, ir.nestingPolicy === "expandParent" ? childTotal : probability);
  }

  const topTotal = ir.entities.reduce((sum, entity) => sum + walk(entity, []), 0);
  if (topTotal > 1 + 1e-15) diagnostics.push({ code: "E003", severity: "error", message: "최상위 확률 합이 1을 초과합니다" });
  leaves.push({ id: "__other__", name: "그외", probability: Math.max(0, 1 - topTotal) });
  leafAncestors.set("__other__", []);
  diagnostics.push({ code: "W002", severity: "info", message: "남는 확률은 ‘그외’ 리프로 자동 편입됩니다" });

  let controlStates = 1;
  let accumulatorStates = 1;
  for (const variable of ir.stateVars) {
    if (!Number.isInteger(variable.max) || (variable.max ?? -1) < 0) {
      diagnostics.push({ code: "E004", severity: "error", message: `${variable.id}의 상한이 필요합니다`, blockId: variable.blockId });
    } else if (variable.role === "control") {
      controlStates *= (variable.max ?? 0) + 1;
    } else if (variable.role === "accumulator") {
      accumulatorStates *= (variable.max ?? 0) + 1;
    } else {
      diagnostics.push({ code: "E009", severity: "error", message: `${variable.id}: stat 변수는 직접 선언할 수 없습니다`, blockId: variable.blockId });
    }
  }
  const estimatedStates = controlStates * accumulatorStates
    * (ir.run.maxTrials + 1) ** Math.max(0, ir.run.trackJoint.length - 1);
  if (estimatedStates > DP_ESTIMATED_STATE_HARD_LIMIT) {
    diagnostics.push({
      code: "E011",
      severity: "error",
      message: `예상 상태 공간이 DP 한도 ${DP_ESTIMATED_STATE_HARD_LIMIT.toLocaleString()}개를 초과합니다`,
    });
  } else if (estimatedStates > DEFAULT_DP_LAYER_STATE_LIMIT) {
    diagnostics.push({
      code: "W004",
      severity: "warning",
      message: `예상 상태 공간이 기본 DP 레이어 상한 ${DEFAULT_DP_LAYER_STATE_LIMIT.toLocaleString()}개를 넘을 수 있습니다`,
    });
  }
  validateAccumulatorTable(ir, leaves, leafAncestors, controlStates, diagnostics);
  return { diagnostics, leaves, controlStates, estimatedStates, exactAvailable: true };
}

function validateAccumulatorTable(
  ir: ModelIr,
  leaves: LeafView[],
  leafAncestors: Map<string, string[]>,
  controlStates: number,
  diagnostics: Diagnostic[],
) {
  const controlIds = new Set(ir.stateVars.filter((variable) => variable.role === "control").map((variable) => variable.id));
  const specs = ir.stateVars
    .filter((variable) => variable.role === "accumulator"
      && Number.isInteger(variable.max)
      && variable.max !== undefined
      && variable.max >= 0
      && !isDerivedLeafCounter(variable, ir, leaves, leafAncestors))
    .map((variable) => {
      const expressions = variable.update?.map((update) => update.set) ?? [];
      const dependsOnTrial = expressions.some((expression) => expressionHasTrial(expression));
      const dependsOnControl = expressions.some((expression) => expressionUsesControl(expression, controlIds));
      const controls = dependsOnControl ? controlStates : 1;
      const trials = dependsOnTrial ? Math.max(1, ir.run.maxTrials) : 1;
      const entries = controls * trials * leaves.length * ((variable.max ?? 0) + 1);
      return { variable, controls, trials, entries };
    });
  const totalEntries = specs.reduce((total, spec) => total + spec.entries, 0);
  if (totalEntries < ACCUMULATOR_TABLE_WARNING_ENTRIES) return;

  const axes = specs.map(({ variable, controls, trials, entries }) =>
    `${variable.id}(제어=${controls}, 시행=${trials}, 리프=${leaves.length}, 현재값=max+1=${(variable.max ?? 0) + 1}, 엔트리=${entries})`
  ).join("; ");
  const blockId = specs[0]?.variable.blockId;
  if (totalEntries > ACCUMULATOR_TABLE_MAX_ENTRIES) {
    diagnostics.push({
      code: "E010",
      severity: "error",
      message: `집계 변수 사전계산 테이블 ${totalEntries.toLocaleString()}개가 한도 ${ACCUMULATOR_TABLE_MAX_ENTRIES.toLocaleString()}개를 초과합니다: ${axes}`,
      blockId,
    });
  } else {
    diagnostics.push({
      code: "W009",
      severity: "warning",
      message: `집계 변수 사전계산 테이블에 ${totalEntries.toLocaleString()}개 엔트리가 필요합니다: ${axes}`,
      blockId,
    });
  }
}

function expressionHasTrial(value: unknown): boolean {
  if (!value || typeof value !== "object") return false;
  if (Array.isArray(value)) return value.some(expressionHasTrial);
  const object = value as Record<string, unknown>;
  return Object.prototype.hasOwnProperty.call(object, "trial")
    || Object.values(object).some(expressionHasTrial);
}

function expressionUsesControl(value: unknown, controlIds: Set<string>): boolean {
  if (!value || typeof value !== "object") return false;
  if (Array.isArray(value)) return value.some((item) => expressionUsesControl(item, controlIds));
  const object = value as Record<string, unknown>;
  return (typeof object.var === "string" && controlIds.has(object.var))
    || Object.values(object).some((item) => expressionUsesControl(item, controlIds));
}

function isDerivedLeafCounter(
  variable: ModelIr["stateVars"][number],
  ir: ModelIr,
  leaves: LeafView[],
  leafAncestors: Map<string, string[]>,
): boolean {
  if (variable.init !== 0 || variable.update?.length !== 1) return false;
  const update = variable.update[0];
  const target = typeof update.when.leafOf === "string"
    ? update.when.leafOf
    : typeof update.when.leafIs === "string" ? update.when.leafIs : undefined;
  const add = update.set && typeof update.set === "object" && "add" in update.set
    ? update.set.add
    : undefined;
  if (!target || !Array.isArray(add) || add.length !== 2) return false;
  const isSelf = (value: unknown) => Boolean(value && typeof value === "object"
    && (value as Record<string, unknown>).var === variable.id);
  const isOne = (value: unknown) => Boolean(value && typeof value === "object"
    && (value as Record<string, unknown>).lit === "1");
  if (!((isSelf(add[0]) && isOne(add[1])) || (isOne(add[0]) && isSelf(add[1])))) return false;

  const matching = new Set(leaves
    .filter((leaf) => leaf.id === target || leafAncestors.get(leaf.id)?.includes(target))
    .map((leaf) => leaf.id));
  if (!matching.size) return false;
  const granted = ir.triggers.reduce<number>((total, trigger) => {
    if (!trigger || typeof trigger !== "object") return total;
    const grant = (trigger as Record<string, unknown>).grant;
    if (!grant || typeof grant !== "object") return total;
    const record = grant as Record<string, unknown>;
    return total + (typeof record.leaf === "string" && matching.has(record.leaf) && typeof record.amount === "number"
      ? record.amount
      : 0);
  }, 0);
  return (variable.max ?? -1) >= ir.run.maxTrials + granted;
}
