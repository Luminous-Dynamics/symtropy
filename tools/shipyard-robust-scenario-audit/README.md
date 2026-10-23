# Shipyard finite-scenario robust plan auditor (experimental V0)

This crate implements a scenario-complete audit layer for finite disruption recovery plans. It is intentionally separate from the placement optimizer and does not change the current production verifier.

## Paradigm shift: robust means every declared case

A nominal plan that passes is not a robust plan. A robust acceptance requires:
1. an immutable, explicitly enumerated scenario set;
2. exactly one candidate plan for each scenario, with no missing or duplicate cases;
3. every required block and operation still represented in each applicable plan;
4. no space-time conflicts under the fixed-footprint model;
5. no overlapping use of the same unary resource;
6. no use of a resource during a source-declared outage.

Outage definitions are supplied separately from candidate plans, so a candidate cannot make a failed scenario disappear by changing the outage window. Omitting future block or operation commitments is incomplete evidence, not a successful recovery.

## Result semantics

- `RobustlyFeasible`: scenario coverage is complete, required commitments are present, and all listed constraints pass for every declared scenario.
- `NotRobust`: the scenario set and required inputs are complete, but at least one specific scenario violates spatial or unary-resource constraints. The failing scenario IDs and per-scenario violations are returned.
- `Incomplete`: the scenario set is missing, duplicated, or includes unknown IDs; required reservations/operations are absent; or structural constraints prevent a complete claim. It must never be relabeled robustly feasible.

## Explicit model boundaries

The checker supports static polygon footprint occupancy over half-open integer tick intervals and unary resource windows/outages. It does not model transfer trajectories, moving swept volume, crane kinematics, tandem lifts, resource capacity greater than one, setup/changeover, operator qualifications, load charts, stacking/support, floor bearing, or probabilistic scenario likelihoods. Passing is only a certificate for this finite, supplied model and scenario set; omitted scenarios remain uncovered risk.

## Frozen conformance targets

- A delayed-clearance scenario with overlapping same-yard occupancy is a counterexample even when the nominal scenario passes.
- An omitted declared scenario yields Incomplete.
- Resource use overlapping a declared crane outage fails.
- Two overlapping operations reserving the same unary crane fail.
- Missing required blocks or operations yields Incomplete.
- Half-open endpoint reuse passes when no spatial/resource conflict remains.

All cases are authored conformance fixtures, not industrial validation results. Rust 1.96 formatting, offline tests, and Clippy with warnings denied must pass on the exact head before any robustness claim is reported.
