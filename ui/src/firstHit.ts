import type { FirstHitResult } from "./types";

export function normalizeFirstHit(value: unknown, runs?: number): FirstHitResult | undefined {
  if (!value) return undefined;
  if (!Array.isArray(value)) return value as FirstHitResult;
  const denominator = Math.max(1, runs ?? value.reduce((sum, count) => sum + Number(count), 0));
  const pmf = value.map((count) => Number(count) / denominator);
  let running = 0;
  const cdf = pmf.map((probability) => (running += probability));
  const success = running;
  const weighted = pmf.reduce((sum, probability, trial) => sum + trial * probability, 0);
  const levels = [0.5, 0.75, 0.9, 0.95, 0.99];
  return {
    pmf,
    cdf,
    failureReachable: Math.max(0, 1 - success),
    mean: success > 0 ? weighted / success : undefined,
    percentiles: levels.map((level) => {
      const target = success * level;
      const trial = cdf.findIndex((probability) => probability >= target);
      return [level, trial < 0 ? cdf.length - 1 : trial];
    }),
  };
}
