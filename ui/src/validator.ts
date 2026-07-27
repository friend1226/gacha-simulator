import type { Diagnostic, Entity, LeafView, ModelIr, ValidationView } from "./types";

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
    return Math.max(probability, ir.nestingPolicy === "expandParent" ? childTotal : probability);
  }

  const topTotal = ir.entities.reduce((sum, entity) => sum + walk(entity, []), 0);
  if (topTotal > 1 + 1e-15) diagnostics.push({ code: "E003", severity: "error", message: "최상위 확률 합이 1을 초과합니다" });
  leaves.push({ id: "__other__", name: "그외", probability: Math.max(0, 1 - topTotal) });
  diagnostics.push({ code: "W002", severity: "info", message: "남는 확률은 ‘그외’ 리프로 자동 편입됩니다" });

  let controlStates = 1;
  for (const variable of ir.stateVars) {
    if (!Number.isInteger(variable.max) || variable.max < 0) {
      diagnostics.push({ code: "E004", severity: "error", message: `${variable.id}의 상한이 필요합니다`, blockId: variable.blockId });
    } else controlStates *= variable.max + 1;
  }
  const estimatedStates = controlStates * (ir.run.maxTrials + 1) ** Math.max(0, ir.run.trackJoint.length - 1);
  if (estimatedStates > 50_000_000) diagnostics.push({ code: "W004", severity: "warning", message: "예상 상태 공간이 DP 권장 한계를 초과합니다" });
  return { diagnostics, leaves, controlStates, estimatedStates, exactAvailable: true };
}
