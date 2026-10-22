# Shipyard disruption-recovery evaluator (V0)

A dependency-free Rust evaluator for comparing a pre-disruption schedule with a proposed recovery plan, built on the independent verifier in [space-time PR #2548](https://github.com/Luminous-Dynamics/symtropy/pull/2548).

## Run

```sh
cargo fmt --all --manifest-path tools/shipyard-disruption-recovery/Cargo.toml -- --check
cargo test --locked --offline --manifest-path tools/shipyard-disruption-recovery/Cargo.toml
cargo clippy --locked --offline --manifest-path tools/shipyard-disruption-recovery/Cargo.toml --all-targets -- -D warnings
```

The dedicated workflow pins Rust 1.96.0 and runs formatting, tests and Clippy offline.

## Paradigm shift: measure recourse, not just static layout quality

A disruption evaluator must answer more than whether the new plan passes a feasibility check. It should also report how much work changed, what changed outside the disturbance's scope, how much time/space was shifted, and how long affected work takes to finish. V0 deliberately emits separate metrics rather than hiding trade-offs in an arbitrary weighted scalar.

The evaluator applies the same declared exclusion-zone, vehicle-outage, and delayed-clearance evidence to both the baseline and proposed candidate. A `PlacementOccupancyExtension` records that a block's old footprint cannot be assumed clear before an authoritative tick; the extension is applied to both plans, automatically marking the outgoing reservation and any intersecting future placement/corridor as affected. The candidate must preserve the same block, starting time and physical footprint for that extended reservation. An extension must be reported before the old scheduled end; V0 rejects retrospective extension of work already considered complete. New-zone intersections, corridor-zone intersections and vehicle-outage overlap also derive affected IDs. The evaluator runs the candidate through the independent space-time verifier; no planner-supplied feasibility assertion is accepted.

The report separates:
- baseline feasible before/after the disruption;
- candidate feasible after the disruption, including vehicle-outage checks;
- placement and transfer reservations added, removed or changed;
- changed reservations outside the affected set (schedule churn);
- summed absolute start/end tick shifts;
- summed rectangle-edge displacement;
- candidate makespan and a declared recovery horizon (latest end of affected/changed reservations minus disruption observation tick);
- immutable-history violations.

Already-completed reservations (end tick at or before the disruption observation tick) may not be rewritten. An infeasible candidate is rejected; a deterministic report or a lower churn count alone does not make it accepted.

## Robust envelope across plausible disruptions

`evaluate_recovery_envelope` evaluates the same candidate across a caller-supplied finite set of hard disturbance scenarios. V0 assigns no probabilities: **every listed scenario must pass**. Empty scenario sets, empty IDs and duplicate scenario IDs fail closed. The report records accepted scenarios and worst-case recovery horizon, unaffected placement/transfer churn, time shift and rectangle-edge displacement. The scenario report ordering is deterministic.

This is a finite robustness envelope, not a probabilistic guarantee over disturbances that were never enumerated. A candidate that only works if a block clears at the optimistic tick is rejected when another supplied scenario keeps the same footprint occupied longer. The benchmark should construct these scenario sets from source-grounded lower/upper timing bounds and then report both per-scenario outcomes and worst-case metrics; do not invent probabilities from a point forecast.



## Tests authored in V0

Fourteen unit tests cover:
1. a newly active zone invalidating a future reservation and the candidate recovering it without changing unaffected work;
2. a vehicle outage requiring a transfer to be reassigned to another vehicle;
3. failure when the candidate still uses an unavailable vehicle;
4. prevention of historical reservation mutation;
5. detection of a transfer corridor invalidated by a new exclusion zone, with recovery that avoids unaffected churn;
6. measurement of churn on work outside the affected set;
7. a delayed block handoff that keeps the departing footprint occupied and requires relocation of the incoming block;
8. rejection of clearance extensions received after the scheduled reservation is considered complete;
9. rejection of unknown extension targets;
10. rejection when a candidate removes or moves the physical footprint that is still occupied;
11. computing makespan and recovery horizon from the clearance-extended effective state, not the stale planned end time.
12. robust acceptance across three clearance-time scenarios;
13. rejection when a candidate survives the optimistic clearance time but fails a later-clearance scenario;
14. fail-closed empty and duplicate scenario-set identities.

A separate integration test exhaustively enumerates every integer placement for one disturbed 2×2 block in a 6×4 yard. Its cell-set oracle finds exactly five feasible recovery layouts and identifies the minimum-edge-shift recovery while leaving unaffected reservations unchanged. It does not call the production candidate-coordinate generator and pins the expected recovery horizon.

These are authored unit scenarios. The exact-head workflow must pass before they are treated as verified results. The integration oracle's five-layout count is also an expected result until verified on the exact commit. The evaluator models rectangle-zone intersections, exclusive vehicle outages and explicit occupied-until extensions for known placements; it does not infer crane/lift clearance from sensor signals, calculate the physical move, compute a placement/route, model workstation capacity, infer an affected set for every possible disruption type, or prove that a recovery is globally optimal.

## Benchmark protocol and targets

Freeze each base plan, disturbance time, outage/exclusion evidence, candidate plan, model profile, and expected output before comparing policies. Include:
- vehicle outages;
- late workstation completion;
- newly occupied destinations or access routes;
- new exclusion zones;
- late-arriving blocks and inspection holds (not yet natively modelled by this V0 interface);
- explicitly impossible-to-recover controls.

For each case and policy, report independent feasibility, post-disruption baseline feasibility, changed affected reservations, unaffected churn, start/end shift, re-handle count when transfers are modelled, makespan/tardiness, recovery horizon, candidate rejection, runtime and peak memory. Compare policies as a Pareto frontier across recovery feasibility, nominal objective degradation, unaffected churn and runtime. Do not collapse these into one score without publishing the weights and sensitivity analysis.

A useful research baseline is Wang et al. (first published 11 July 2026), which examines irregular-block shipyard spatial scheduling under workstation/vehicle disruption via mixed-integer programming and a hybrid artificial-bee-colony / adaptive-large-neighbourhood-search recovery method ([DOI](https://doi.org/10.1049/cim2.70066)). This crate does not reproduce that model and does not claim to beat it. First reproduce comparable assumptions and instances, then compare recovery quality against transparent first-fit/greedy baselines and an exact solver on tiny cases.

For the generic scheduling subproblem, pin the instance revision and bounds from [JSPLib](https://scheduleopt.github.io/benchmarks/jsplib/) and [FJSPLib](https://scheduleopt.github.io/benchmarks/fjsplib/); those suites compare reference engines under standardized 10-minute budgets and track optimality gaps. They do not test irregular shipyard geometry, and a win there is not evidence of yard-wide superiority.

## Claim boundary

This is an evaluator, not a disruption-aware optimizer or production planner. It has no authenticated MES/ERP integration, no complete workstation model, no arbitrary polygon or swept-volume geometry, and no lift/crane/rigging or floor-bearing safety model. Physical claims require qualified engineering evidence; industrial cost/time savings require permissioned, longitudinal yard data and comparable baselines.
