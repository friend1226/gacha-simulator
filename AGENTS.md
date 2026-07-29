# Gacha simulator implementation rules

The numbered list below contains the 9 absolute rules from `CLAUDE.md`'s "절대 규칙",
mirrored 1:1 in English. Keep that numbered list in sync with `CLAUDE.md`. Source spec:
`docs/DESIGN.md` §12.

1. Never put `Fraction`/`BigRational` in the DP inner loop. Exact mode uses one common
   denominator per layer with BigInt numerators only. A GCD call inside the inner loop
   is a design violation (reduction happens at most once per layer, via `reduce_layers`).
2. Never parse probability literals as f64. Decimal strings must be parsed directly into
   rationals; `"0.007".parse::<f64>()` followed by rational conversion is forbidden.
3. Never duplicate engines per backend. Keep `run_generic::<P: Prob>` as the single
   generic implementation shared by `engine_mc`/`engine_dp`.
4. Never store entity counts in state. Only leaf counts are stored; entity counts are
   derived sums. A grant increments only its target leaf's counter — incrementing an
   ancestor separately double-counts.
5. Never evaluate probability expressions inside an engine's inner loop. Always go
   through the precomputed `prob_table`.
6. Monte Carlo output must always include a Wilson confidence interval and the seed.
7. The default snapshot mode is `aggregate`. `full` must never run without explicit
   confirmation.
8. Never silently discard pruned mass or probability-clamp events. Accumulate and
   report them in the result.
9. Do not perform performance optimization until the §13.3 core validation tests pass;
   keep those tests passing whenever performance work is performed. Correctness
   validation comes first.

## Branch and merge process

- `main` is the production deployment branch. Do not commit directly to it.
- Branch work from `dev`, then merge the completed work back into `dev`.
- Only the owner decides when to merge `dev` into `main`; agents must not do this on
  their own.
- Automatic production deployment runs only on pushes to `main`.
