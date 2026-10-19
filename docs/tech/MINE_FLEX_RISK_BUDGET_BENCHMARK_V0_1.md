# MINE-FLEX-PARETO-001 — risk-budgeted robust dispatch frontier v0.1

## Purpose

MINE-FLEX-RECOURSE-001 made the conflict between cost-first and carbon-first dispatch measurable. This increment avoids forcing operators to select one of those two lexicographic extremes. It implements an **epsilon-constraint** policy: minimize worst-case cost while requiring worst-case carbon to remain below a declared cap.

The cap is not converted to a weighted sum. Safety electricity, grid import limits, protected water reserves, and the production target remain hard constraints and are never traded away to improve cost or carbon.

## Decision contract

The chooser receives only the forecast and the published finite uncertainty envelope for that case. It does not receive the realized-world index, hidden realization ID, or evaluator-only oracle schedule when selecting a schedule.

For every integer schedule that achieves the target production, the reference evaluator:

1. checks feasibility against every world in the envelope;
2. independently reconstructs each feasible candidate's cost, carbon and hard-resource balances;
3. records the maximum cost and maximum carbon across the envelope;
4. removes dominated robust schedules to publish the nondominated cost-carbon frontier; and
5. for each carbon budget, chooses the schedule with the lowest worst-case cost, then lowest worst-case carbon, then lexicographically smallest batch vector.

The six budget values are relative headroom above the minimum achievable worst-case carbon among robust-feasible schedules: **0%, 2.5%, 5%, 10%, 25%, and 100%**. The reference chooses by the declared envelope; realized-world oracle costs are evaluator-only and are used strictly for retrospective regret measurement.

This is a static robust policy, not adaptive control. It does not assign probabilities to the worlds or claim a chance constraint. The percentages are a reproducible sweep, not a recommendation for a real mine's carbon budget.

## Reproduction

From the repository root:

```sh
cargo fmt --manifest-path tools/mine-flex-scheduling-bench/Cargo.toml -- --check
cargo test --manifest-path tools/mine-flex-scheduling-bench/Cargo.toml
cargo run --quiet --manifest-path tools/mine-flex-scheduling-bench/Cargo.toml -- \
  --risk-budget --source-commit "$(git rev-parse HEAD)"
```

The emitted JSON receipt binds the source commit and the deterministic generated-case-set digest. The CI gate checks all 64 separated validation/held-out cases, all 384 case-budget decisions, all 1,920 paired envelope-world evaluations, zero hard-constraint violations, zero cap violations, zero decision failures, zero budget-monotonicity violations, and nonempty nondominated frontiers.

The key monotonicity property is structural: widening a cap must not increase the minimum attainable worst-case cost. The selected schedule's worst-case carbon must never exceed its own cap. Neither property implies that costs or carbon will improve monotonically on every individual realized world.

## Current scope and claim ceiling

The evaluation uses an exhaustive finite integer oracle on authored **synthetic** resource schedules. Unit values are arbitrary integers, not currency, metered electricity, water volumes, verified carbon emissions, production tonnes, or safety measurements. Passing means the contract holds on the generated corpus and exact source head only. It does not establish actual mine savings, environmental performance, physical executability, real-world reliability, or general out-of-distribution robustness.

The envelope is finite and public. Candidate schedules must work in every world declared in it; worlds not represented in the envelope are not covered by that constraint.

## Research grounding and next steps

Mine energy management studies increasingly couple renewable supply, ventilation, dewatering, blasting energy and hydraulic/thermal processes, rather than scheduling each load as independent demand. A recent process-coupled distributionally robust framework explicitly models those cross-process and temporal correlations: [Scientific Reports (2025)](https://www.nature.com/articles/s41598-025-11013-x). Distributionally robust dispatch research also formalizes the reliability-versus-cost trade-off when the uncertainty distribution is not known exactly: [European Journal of Operational Research](https://www.sciencedirect.com/science/article/pii/S0377221721003271). Recent mine-flexibility work emphasizes that heterogeneous loads have different ramp limits, delays and response times, so aggregate commands must ultimately be converted into executable setpoints: [Energy (2026)](https://www.sciencedirect.com/science/article/pii/S0360544226019791).

The next evidence tier should add correlated multi-period supply shocks and coupled water/power failures, physical response envelopes for ventilation and dewatering, explicitly modeled production delay / stockpile / equipment constraints, blinded holdout distributions, and finally site-separated rights-cleared telemetry. Report tail cost/regret and feasibility coverage alongside the entire Pareto frontier; never collapse safety, protected water, production, cost and carbon into one opaque scalar.
