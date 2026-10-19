# MINE-FLEX-EXEC-001 — physical execution feasibility v0.1

## Purpose

A schedule can be resource-feasible without being physically executable. The existing exact ledger checks production, safety electricity, grid-import capacity and protected water. This suite adds a separate, independently testable execution envelope and compares a resource-only robust cost minimizer with a minimizer constrained by that envelope.

This is a falsifiable reference contract—not an operational controller or a calibrated equipment model.

## Coarse v0.1 execution envelope

The benchmark represents process throughput using integer batches per period. It applies these deliberately simple constraints:

- Ramp up by at most one batch between adjacent periods.
- Ramp down by at most one batch between adjacent periods.
- After startup, remain on for at least two periods.
- Use no more than one startup in the three-period horizon.
- Reject a startup too close to the horizon boundary to prove the minimum operating duration.

Structured failure codes distinguish ramp-up, ramp-down, minimum-up-time, horizon-boundary and startup-count violations. This validator is separate from the optimizer's resource ledger.

These values are synthetic test settings. One process batch per period is not equivalent to a calibrated MW/min or rpm/s equipment ramp. The integer batch model also does not yet express separate ventilation, pumping, haulage, crushing, conveyor or stockpile assets.

## Robust-selection contract

For each of the 64 deterministic generated cases:

1. Enumerate schedules that satisfy the production target.
2. Retain only schedules that satisfy the existing electrical and protected-water ledger in every world in the published uncertainty envelope.
3. Independently reconstruct the resource ledger for each retained candidate.
4. Pick the resource-only minimum-worst-case-cost schedule as a baseline.
5. Run the separate execution audit; report whether the resource-only optimum violates it.
6. Select the minimum-worst-case-cost schedule from only the candidates that also pass the execution envelope.
7. Evaluate both choices on the same five declared worlds per case, reporting resource feasibility, execution violations, cost, carbon and cost regret separately.

Candidate ties are broken deterministically by worst-case carbon and then the batch vector. The actual-world oracle is used only for retrospective regret, never during selection.

## Reproduction

From the repository root:

```sh
cargo fmt --manifest-path tools/mine-flex-scheduling-bench/Cargo.toml -- --check
cargo test --manifest-path tools/mine-flex-scheduling-bench/Cargo.toml
cargo run --quiet --manifest-path tools/mine-flex-scheduling-bench/Cargo.toml -- \
  --executability --source-commit "$(git rev-parse HEAD)"
```

## Acceptance gates

- 32 validation cases and 32 disjoint generated holdout cases.
- 320 paired world evaluations, with both policy decisions evaluated against the same realized world.
- A regression control that is resource-feasible in every declared world but violates the independent ramp envelope.
- At least one resource-only optimum rejected by execution feasibility; otherwise the benchmark has failed to demonstrate its distinguishing control.
- A robust execution-constrained plan for every case, with zero execution-envelope violations and zero independent resource-accounting mismatches.
- An exact-head receipt bound to the 40-character source commit and SHA-256 of the generated case set.
- Failed decisions and unsupported conditions must not be marked feasible.

A successful synthetic result is only evidence that the stated contract holds in the finite generated suite. It is not a safety case, an estimate of real equipment capability, or a real-mine cost, carbon or throughput result.

## Research grounding and extension path

A process-coupled distributionally robust mine-energy framework models temporal and spatial correlations between renewables, ventilation, dewatering and blasting, and retains safety-critical energy constraints ([Scientific Reports, 2025](https://www.nature.com/articles/s41598-025-11013-x)). A 2026 mine-load-flexibility study discusses varying response rates and time granularities and the disaggregation of aggregate commands into feasible setpoints ([Energy, 2026](https://www.sciencedirect.com/science/article/pii/S0360544226019791)). Research on underground mine ventilation also highlights system inertia and the need for a pre-defined sequence of admissible control actions ([Journal of Mining Institute, 2025](https://pmi.spmi.ru/pmi/article/view/16568)).

The next tier should replace the batch proxy with separate asset envelopes: ventilation airflow and fan speed, pump/flow/head constraints, haulage and conveyor occupancy, crusher throughput, thermal load, stockpile inventory and process-quality constraints. Add minimum-on/off times, startup energy, ramp response delay, equipment availability, and lagged process state. Calibrate only from rights-cleared site data, hold out sites and time periods, and require independent blind evaluation before any field-control claims.
