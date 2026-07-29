export type Expr = Record<string, unknown>;

export interface Entity {
  id: string;
  name: string;
  prob: Expr;
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
  probRules: unknown[];
  transitions: unknown[];
  triggers: unknown[];
  run: {
    maxTrials: number;
    trackJoint: string[];
    numeric: "f64" | "scaled" | "exact";
    condition?: Expr;
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

