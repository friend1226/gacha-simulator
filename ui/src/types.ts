export type NumberExpr =
  | { lit: string }
  | { var: string }
  | { trial: true }
  | { add: [NumberExpr, NumberExpr] }
  | { sub: [NumberExpr, NumberExpr] }
  | { mul: [NumberExpr, NumberExpr] }
  | { div: [NumberExpr, NumberExpr] }
  | { neg: NumberExpr }
  | { abs: NumberExpr }
  | { floor: NumberExpr }
  | { ceil: NumberExpr }
  | { round: NumberExpr }
  | { min: [NumberExpr, NumberExpr] }
  | { max: [NumberExpr, NumberExpr] }
  | { clamp: [NumberExpr, NumberExpr, NumberExpr] }
  | { pow: [NumberExpr, { lit: string }] }
  | { if: BooleanExpr; then: NumberExpr; else: NumberExpr };

export type BooleanExpr =
  | { eq: [NumberExpr, NumberExpr] }
  | { ne: [NumberExpr, NumberExpr] }
  | { lt: [NumberExpr, NumberExpr] }
  | { le: [NumberExpr, NumberExpr] }
  | { gt: [NumberExpr, NumberExpr] }
  | { ge: [NumberExpr, NumberExpr] }
  | { and: [BooleanExpr, BooleanExpr] }
  | { or: [BooleanExpr, BooleanExpr] }
  | { not: BooleanExpr }
  | { xor: [BooleanExpr, BooleanExpr] };

export type Expr = NumberExpr | BooleanExpr;

export interface ProbabilityRule {
  target: string;
  expr: NumberExpr;
  blockId?: string;
}

export interface Entity {
  id: string;
  name: string;
  prob: NumberExpr;
  children?: Entity[];
  blockId?: string;
}

export interface ModelIr {
  irVersion: 1 | 2;
  name: string;
  entities: Entity[];
  nestingPolicy: "clampChildren" | "expandParent" | "scaleSiblings" | "error";
  stateVars: Array<{
    id: string;
    init: number;
    max?: number;
    role: "control" | "stat" | "accumulator";
    name?: string;
    update?: Array<{ when: Record<string, unknown>; set: Expr }>;
    clampPolicy?: "saturate" | "error";
    blockId?: string;
  }>;
  probRules: ProbabilityRule[];
  transitions: unknown[];
  triggers: unknown[];
  run: {
    maxTrials: number;
    trackJoint: string[];
    numeric: "f64" | "scaled" | "exact";
    condition?: BooleanExpr;
    trialSeries?: "none" | "marginal" | "checkpoints";
    seriesCheckpoints?: number[];
  };
}

export interface WilsonInterval {
  estimate: number;
  lower: number;
  upper: number;
}

export interface ResultCell {
  counts: number[];
  probability?: number;
  display?: string;
  numerator?: string;
  denominator?: string;
  occurrences?: number;
  interval?: WilsonInterval;
}

export interface FirstHitResult {
  pmf: Array<number | { probability: number }>;
  cdf: Array<number | { probability: number }>;
  failureReachable: number | { probability: number };
  mean?: number;
  percentiles: Array<[number, number]>;
}

export interface EngineResult {
  engine: "DP" | "Exact" | "MC";
  numeric: string;
  trials: number;
  peakStates?: number;
  runs?: number;
  seed?: number;
  trackedLeafIds: string[];
  joint: ResultCell[];
  firstHit?: FirstHitResult;
  prunedMass?: number;
  elapsedMs: number;
  clampEvents: number;
  accumulatorClampEvents: number;
  modelHash?: string;
  trialSeries?: {
    mode: string;
    marginal?: Array<{
      trial: number;
      axes: Array<{
        id: string;
        cells: Array<{
          value: number;
          probability?: number;
          display?: string;
          occurrences?: number;
          interval?: WilsonInterval;
        }>;
      }>;
    }>;
    checkpoints?: Array<{ trial: number; joint: ResultCell[] }>;
  };
}

export interface Diagnostic {
  code: string;
  severity: "error" | "warning" | "info";
  message: string;
  blockId?: string;
}

export interface LeafView {
  id: string;
  name: string;
  probability: number;
}

export interface ValidationView {
  diagnostics: Diagnostic[];
  leaves: LeafView[];
  controlStates: number;
  estimatedStates: number;
  exactAvailable: boolean;
}

