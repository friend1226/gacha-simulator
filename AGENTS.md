# Gacha simulator agent guide

## Sources of truth

- Read `docs/DESIGN.md` before changing architecture or engine semantics. Its decisions
  are authoritative.
- Use `docs/DESIGN.md` §13 for known implementation differences, `docs/STATUS.md` for
  completed validation evidence, and `docs/USAGE.md` for supported build and runtime
  workflows.
- When implementation and specification intentionally diverge, update §13 together
  with the code. Do not leave the reason only in comments or a temporary plan.
- Keep this file and `CLAUDE.md` synchronized when shared implementation or workflow
  rules change.

## Absolute implementation rules

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

## Validation baseline

- Run Rust validation with `cargo test --workspace --exclude gacha-tauri`. The Tauri
  crate requires platform GUI libraries that are not present in Linux CI.
- Run UI validation from `ui/` with `npx tsc --noEmit` and `npm test`.
- The current regression line is 62 Rust core tests and 37 UI tests. Preserve all three
  preset `resultSha256` goldens.
- None of the three presets combines `transitions` and `triggers`. When changing
  reachable-control analysis, explicitly run
  `cyclic_transition_frontier_keeps_late_trigger_states_reachable` in
  `crates/gacha-core/tests/core_diagnostics.rs`; preset goldens alone do not cover that
  path.
- After changing `gacha-core` or `gacha-wasm` behavior used by the web app, rebuild the
  package with
  `wasm-pack build crates/gacha-wasm --target web --out-dir ../../ui/public/wasm`, then
  run `npm run build` and verify calculations through `npm run preview`.
  `npm run dev` cannot execute the WASM worker path because Vite does not allow the
  required dynamic import from `public/`.
- `ui/public/wasm` and `ui/dist` are generated build artifacts and must not be
  committed.

## Cross-file change rules

- Documentation prose is Korean; code identifiers and commit messages are English.
- A Model IR schema change must increment `irVersion` and update `presets/` and
  `ui/src/types.ts` together.
- A new diagnostic code must be added to the table in `docs/DESIGN.md` §3.4 and to the
  UI diagnostic labels/help coverage.
- Keep validation thresholds synchronized between `ui/src/validator.ts`,
  `crates/gacha-core/src/compile.rs`, and `crates/gacha-core/src/engine_dp.rs`.
- Record material validation results in `docs/STATUS.md`. Temporary plan and
  measurement files must not be committed.
- Game-specific mechanics belong in `presets/`, not hard-coded engine branches.

## Branch and merge process

- `main` is the production deployment branch. Do not commit directly to it.
- Branch work from `dev` and open a pull request.
- **Do not merge into `dev` before the work passes review.** Open the PR, confirm CI
  passes, report completion together with the branch, commit, PR number, and an explicit
  "awaiting merge", then stop. Merge only after approval. If changes are requested, push
  additional commits to the same branch and report again.
- Only the owner decides when to merge `dev` into `main`; agents must not do this on
  their own.
- Automatic production deployment runs only on pushes to `main`.
- Items marked as awaiting owner judgment in `CLAUDE.md` must not be implemented
  without explicit approval. In particular, a lazy probability table conflicts with
  absolute rule 5 and requires a separate design decision.
