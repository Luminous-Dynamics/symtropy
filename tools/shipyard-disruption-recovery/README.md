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

The evaluator applies the same declared exclusion-zone and vehicle-outage evidence to both the baseline and proposed candidate. It derives affected placement IDs from new-zone intersections and affected transfer IDs from outage overlap, then adds explicitly declared affected IDs. It runs the candidate through the independent space-time verifier; no planner-supplied feasibility assertion is accepted.

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

## Tests authored in V0

Five unit tests cover:
1. a newly active zone invalidating a future reservation and the candidate recovering it without changing unaffected work;
2. a vehicle outage requiring a transfer to be reassigned to another vehicle;
3. failure when the candidate still uses an unavailable vehicle;
4. prevention of historical reservation mutation;
5. measurement of churn on work outside the affected set.

A separate integration test exhaustively enumerates every integer placement for one disturbed 2×2 block in a 6×4 yard. Its cell-set oracle finds exactly five feasible recovery layouts and identifies the minimum-edge-shift recovery while leaving unaffected reservations unchanged. It does not call the production candidate-coordinate generator and pins the expected recovery horizon.

These are authored unit scenarios. The exact-head workflow must pass before they are treated as verified results. The integration oracle's five-layout count is also an expected result until verified on the exact commit. The evaluator currently models rectangle-zone intersections and exclusive vehicle outages only; it does not compute a placement/route, model workstation capacity, infer an affected set for every possible disruption type, or prove that a recovery is globally optimal.

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
