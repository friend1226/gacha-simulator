# Gacha simulator implementation rules

These rules capture the non-negotiable invariants from the implementation specification
supplied with the project.

- Keep probability literals as exact decimal/fraction strings until parsed as rationals.
- Store counts for mutually exclusive leaves only; entity counts are derived sums.
- Precompute probability tables before running either engine.
- Keep DP generic over `Prob`; do not duplicate floating-point engines.
- Exact DP uses one common denominator per layer and BigInt numerators per cell.
- A grant is applied after the normal draw and changes only its target leaf count.
- Monte Carlo output must include Wilson confidence intervals and the seed.
- Never silently discard pruned mass or probability-clamp events.
