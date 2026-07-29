export type EngineWorkerMethod = "dp" | "exact" | "mc";

export interface EngineWorkerRequest {
  id: number;
  method: EngineWorkerMethod;
  source: string;
  runs?: number;
  seed?: number;
}

export type EngineWorkerResponse =
  | { id: number; progress: { completed: number; total: number } }
  | { id: number; ok: true; json: string }
  | { id: number; ok: false; error: string };
