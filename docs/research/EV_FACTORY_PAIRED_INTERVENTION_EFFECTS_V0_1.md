# EV factory paired intervention effect-of-effects v0.1

**Related work:** [EV-FACTORY-INTERVENTION-BENCH-001 #2419](https://github.com/Luminous-Dynamics/symtropy/issues/2419), [EV-FACTORY-INTERVENTION-BENCH-002 #2421](https://github.com/Luminous-Dynamics/symtropy/issues/2421)  
**Status:** repeated synthetic DES counterfactuals; not a calibrated plant twin  
**Implementation:** `ev_factory_simulation` per-seed outcome rows + `ev_factory_intervention_benchmark` paired comparisons

## Why marginal confidence intervals are not enough

Suppose interventions A and B share the same simulated line and supplier arrivals. Comparing A's marginal 95% interval with B's marginal 95% interval throws away their pairing. The relevant sample is the seed-by-seed difference in intervention gains.

For each seed (s), define:

```text
gain_A(s) = output_A(s) - baseline_output(s)
gain_B(s) = output_B(s) - baseline_output(s)
paired_effect_A_minus_B(s) = gain_A(s) - gain_B(s)
```

The baseline cancels algebraically in the last expression, but keeping the defined paired deltas in the trace makes the scenario boundary auditable. Calculate mean, sample standard deviation, empirical P05/P50/P95 and a 95% Student-`t` interval across `paired_effect_A_minus_B(s)`. Do not form this interval by subtracting the endpoints of two marginal confidence intervals.

## Per-seed report contract

The simulation report now exposes one `replication_outcomes` row for each seed × scenario, with:
- seed,
- scenario ID,
- intervention station ID (null only for baseline),
- good units/hour,
- same-seed baseline good units/hour, and
- paired good-output delta vs that seed's baseline (null only for baseline).

For the current five-station design, this means 20 seeds × 6 scenarios = 120 rows. The evaluator verifies exact count, known unique seeds/scenarios, one baseline plus each intervention per seed, unique (seed, scenario) tuples and arithmetic consistency of every delta with that seed's baseline. Missing or mismatched seeds fail closed.

## Intervention-vs-intervention comparisons

For each unordered pair of station additions (10 pairs in the five-station fixture), the evaluator records:

- `mean_paired_effect_a_minus_b_units_per_hour`,
- the full paired distribution summary and 95% confidence interval,
- empirical fraction of seeds where A beats B by more than the practical margin, and vice versa,
- declared practical-equivalence margin, and
- decision classification.

The initial internal practical-equivalence margin is **0.05 good units/hour** for this synthetic fixture. The rule is explicit:
- **A meaningfully better** when the whole 95% paired interval is above +0.05.
- **B meaningfully better** when the whole 95% paired interval is below −0.05.
- **Practically equivalent** when the whole interval lies inside [−0.05, +0.05].
- **Inconclusive** otherwise.

The threshold is a project-defined test knob—not a universal manufacturing threshold. A real line would need a margin tied to its takt/demand/cost boundary and data quality. For five station choices, a unique best intervention is claimed by the evaluator only when the point-estimate top candidate's paired comparison is meaningfully better than each alternative. If not, `robust_best_station_id` is null and policy disposition is abstention.

## Statistical limitations

These intervals quantify stochastic variation under this simulator and its declared seed pairing. They do not include model-form uncertainty, uncertainty in cycle/yield/failure assumptions, measurement bias, or omitted failure modes. Student-`t` intervals assume the per-seed deltas are suitable replication observations under the synthetic input model. The current test has 20 seeds; if the interval remains wide or rankings are close, add replications according to a predeclared precision/sequential-stopping policy rather than select a favorable seed.

Empirical fractions of seeds above/below the margin are descriptive sample frequencies, not posterior probabilities and not guaranteed calibrated probabilities of production improvement.

## Why this matters for the benchmark

The 2026 automotive-parts study's ABAP F1=0.89 and utilization F1=0.73 measure bottleneck classification against expert labels, not intervention-vs-intervention value: https://doi.org/10.3233/ATDE260693. They cannot be compared directly to these paired effect intervals. Keep three tasks separate:

1. **Constraint classification** against independently labeled station windows.
2. **Intervention-value ranking** against counterfactuals inside a declared simulator.
3. **Real production effect** against measured controlled results or an independent temporal holdout.

NIST's manufacturing digital-twin guidance emphasizes that verification, validation and uncertainty quantification remain distinct from model reproducibility: https://doi.org/10.6028/NIST.IR.8620.

## Run and acceptance

```sh
cargo fmt --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml -- --check
cargo test --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml
# CI generates the simulation report, computes its SHA-256 from exact bytes,
# evaluates the ranking, runs the deterministic repeat and mutation controls.
```

Required checks: 120 seed/scenario outcomes, valid seed alignment, 10 pairwise comparisons, 20 samples per paired interval, deterministic report, rejection of unknown/duplicate seeds and failed source invariants, all release gates false. Hand-check tests cover a known paired advantage and an effectively equal case.

No real MES/PLC or factory trace was collected. No supplier, plant, manufacturer, integrator, landlord or agency was contacted; no quotes requested. The result cannot authorize machinery purchases or factory/product release.
