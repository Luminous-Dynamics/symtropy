# MINE-FLEX-ROBUST-001 — paired generated mine-dispatch robustness tournament v0.1

## Purpose

Extend the MINE-FLEX exact accounting oracle with uncertainty and policy comparison. The key question is not "which schedule minimizes cost in one forecast?" but "which schedule remains feasible and keeps tail regret bounded when the forecast is wrong?"

This benchmark is authored synthetic data. Its generator, output and arbitrary integer units are not real-mine measurements, cost estimates, carbon accounting or a site safety model.

## Information boundary and policies

Each seed creates one public point forecast and a five-member uncertainty envelope. The nominal policy sees only the point forecast. The robustness policy sees the full declared envelope but is not given the evaluator's current realization index. The evaluator alone scores each policy in each realization and computes a separate perfect-information lower-cost bound.

- **Fixed baseline:** frozen schedule [2,1,1].
- **Nominal:** exact cost-minimizing schedule under the point forecast.
- **Bounded minimax:** select a schedule feasible under every member of the declared envelope; minimize worst-case grid cost, then worst-case carbon, then use lexicographic schedule order.
- **Realization oracle:** the minimum-cost feasible schedule for the realized case, available to the evaluator only. It is a lower-cost bound, not a deployable policy.

The same five possible realizations are scored against all policies within a seed, providing paired comparisons. Costs and carbon remain separate outcomes; the robust objective is explicitly cost-first rather than a disguised multi-objective weighted sum.

## Generated split

Generator version: mine-flex-generated-disturbances-v0.1.0.

- Validation seeds: 1 through 32.
- Held-out seeds: 10001 through 10032.
- Five realizations per seed: nominal, P0 import-cap tight, P1 renewable shortfall, P1 water shortfall, and a correlated P0/P1 stress with a P2 price/carbon spike.
- Each seed deterministically varies period costs, carbon intensities and water availability inside the public generator.
- Total evaluation: 64 generated cases × 5 paired realizations = 320 world-policy comparison rows shared across the three selectable policy types.

The two seed ranges do not overlap. Held-out numbers are generated with the same published program and thus are **reproducible out-of-sample seeds, not a blinded independent dataset**. Their results must be reported after the evaluation code/version is frozen; do not tune thresholds to make the held-out score pass.

## Metrics

Per validation/held-out split and policy, the receipt reports:
- number of evaluated, feasible and infeasible policy-world outcomes, plus feasibility rate in basis points;
- mean grid cost and mean grid-carbon units for feasible outcomes;
- cost regret versus the realized-world oracle, including mean regret, maximum regret, nearest-rank 95th percentile, and the arithmetic mean of the worst ceil(5% × n) regret observations (CVaR-style upper-tail metric);
- paired mean cost difference between the nominal and bounded-minimax policies among worlds where both are feasible;
- worlds where one policy stays feasible and the other does not;
- independent reconstruction/hard-constraint violations.

Regret is only calculated for feasible schedules. An infeasible schedule is never assigned an invented huge cost or silently counted as feasible; its feasibility failure is reported separately. The per-realization oracle is evaluator-only. This avoids turning hard constraints into a tunable penalty.

## Acceptance gates

- Exact source commit and digest of the generated case set appear in the receipt.
- Same version and seed set produce byte-identical serialized receipts for the same source commit.
- 32 validation seeds and 32 non-overlapping held-out seeds each produce 160 world outcomes.
- The robust schedule is feasible in all 320 declared generated worlds, and every reported feasible schedule agrees with independent balance reconstruction.
- Zero hard-constraint violations in any schedule reported as feasible.
- The held-out split includes tail regret and feasibility metrics; mean-only reporting is insufficient.
- Failure is a legitimate outcome. The acceptance target is not a claim that robust scheduling always wins on cost or carbon.

These are synthetic implementation controls; zero failures here does not demonstrate operational safety at a mine.

## Research motivation

- [Distributionally robust hybrid energy management in smart mining](https://www.nature.com/articles/s41598-025-11013-x) explicitly discusses cross-process, time-correlated uncertainty and safety-critical constraints.
- [Li et al., 2026, Unlocking lithium mine load flexibility](https://doi.org/10.1016/j.energy.2026.141872) studies time/granularity constraints of flexible loads in a real isolated lithium-mine microgrid.
- [Frequency-constrained autonomous microgrid planning for mining industry applications](https://doi.org/10.1016/j.apenergy.2025.126201) considers power-system frequency stability alongside design and scheduling.
- [Optimal scheduling of coal mine integrated energy systems under multi-energy-flow uncertainty](https://doi.org/10.1016/j.csite.2025.107229) couples flexible electric/thermal loads with uncertain supply and demand.

The published numerical case results are not imported as targets. They motivate dimensions to test and would need independent reproduction with aligned system boundaries before comparison.

## Known omissions and next tier

v0.1 has only three periods, integer batch loads, a bounded scenario set and simple per-period resource accounting. It omits power-flow equations, equipment ramp/start times, storage, forecast calibration, stochastic arrival/delay of assays, water quality, process kinetics and site geometry. The robust policy protects the declared uncertainty envelope, not all imaginable conditions.

Next: pre-register broader disturbance families, add forecast misspecification and withheld shock types, compare distributionally robust and chance-constrained baselines only under aligned assumptions, report paired confidence intervals across many independent seeds, and add site evidence only after rights, provenance, calibration and operations/safety review.

## Ownership

Composes the accounting primitives of MINE-FLEX-001 #2068, adaptive-policy semantics #2051, feed blending #2002, and campaign evaluation #1933. It does not create a new authoritative mine ledger, assay model, mine-water model, safety function or production optimizer.
