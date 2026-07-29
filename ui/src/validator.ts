import type { Diagnostic, Entity, LeafView, ModelIr, ValidationView } from "./types";

// Keep these thresholds in sync with crates/gacha-core/src/compile.rs.
export const ACCUMULATOR_TABLE_WARNING_ENTRIES = 500_000;
export const ACCUMULATOR_TABLE_MAX_ENTRIES = 10_000_000;

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

export function validateLocally(ir: ModelIr): ValidationView {
  const diagnostics: Diagnostic[] = [];
  const ids = new Set<string>();
  const leaves: LeafView[] = [];
  const leafAncestors = new Map<string, string[]>();

  function walk(entity: Entity, ancestors: string[]): number {
    if (ids.has(entity.id)) {
      diagnostics.push({ code: "E001", severity: "error", message: `중복 엔티티 ID: ${entity.id}`, blockId: entity.blockId });
    }
    ids.add(entity.id);
    let probability = 0;
    try {
      const literal = entity.prob.lit;
      if (typeof literal !== "string") throw new Error("UI 미리보기는 리터럴 확률만 즉시 계산합니다");
      probability = decimal(parseExactLiteral(literal));
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
  if (estimatedStates > 50_000_000) diagnostics.push({ code: "W004", severity: "warning", message: "예상 상태 공간이 DP 권장 한계를 초과합니다" });
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
  const add = update.set.add;
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
