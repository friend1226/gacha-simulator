export type Expr = Record<string, unknown>;

export interface Entity {
  id: string;
  name: string;
  prob: Expr;
  children?: Entity[];
  blockId?: string;
}

export interface ModelIr {
  irVersion: 1;
  name: string;
  entities: Entity[];
  nestingPolicy: "clampChildren" | "expandParent" | "scaleSiblings" | "error";
  stateVars: Array<{ id: string; init: number; max: number; role: "control"; blockId?: string }>;
  probRules: unknown[];
  transitions: unknown[];
  triggers: unknown[];
  run: {
    maxTrials: number;
    trackJoint: string[];
    numeric: "f64" | "scaled" | "exact";
    condition?: Expr;
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

