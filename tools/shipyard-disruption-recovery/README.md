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



## Sequential disruptions: replay the plan as a state trajectory

\`evaluate_recovery_sequence\` evaluates plan updates in chronological order. Each accepted stage's candidate, with its observed occupancy extensions and newly known exclusion zones applied, becomes the next stage's baseline. Prior exclusion zones are carried forward; vehicle outages remain binding while their intervals have not ended. A stage may change future reservations but cannot silently alter already completed or in-progress operations. Stage IDs must be unique and observation ticks monotonic.

The sequence stops at the first failed recovery rather than pretending the next step is grounded in a valid physical state. It reports accepted/total stages, cumulative changed-reservation counts and time/geometry shifts, plus worst per-stage recovery horizon and unaffected-work churn. These cumulative shift values count repeated revisions separately, which makes schedule thrashing visible; they are not unique-block counts.

A test trajectory first extends a departing block's occupancy from tick 5 to tick 7, then applies a transfer-corridor closure and a tug outage. The next plan must preserve the confirmed outgoing footprint through tick 7 and reroute the not-yet-started transfer to an available tug. A separate test verifies an earlier outage stays binding in a later stage. This is deterministic sequence replay, not stochastic forecasting; scenario probabilities still require calibrated evidence.

## Pareto selection without hidden weights

`pareto_recovery_candidates` compares only robustly accepted candidates. A missing recovery horizon/makespan is treated as not-applicable: when one candidate has a metric and another does not, neither is allowed to dominate solely because an `Option` sorts before the other. It publishes separate objectives for worst-case recovery horizon, makespan, changed placement/transfer count, unaffected placement/transfer churn, time shift and rectangle-edge displacement. One candidate dominates another only when it is no worse on every published objective and strictly better on at least one. A candidate failing any scenario is excluded from the frontier; empty or duplicate candidate IDs invalidate the comparison. Equal-scoring alternatives may both remain on the frontier.

This intentionally avoids inventing a single weighted score. Yard managers can review the frontier and state explicit business preferences rather than letting an undocumented coefficient trade away resilience, stability or schedule time. This is not a proof that the current objective set is complete: cost, emissions, lift energy, risk and downstream vessel-delivery penalties need properly sourced models before they can be added.


## Tests authored in V0

Twenty-three core recovery tests cover:
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
15. Pareto frontier selection preserving a real recovery-time versus geometry-shift trade-off;
16. rejection of candidates that fail any member of a robust disturbance envelope;
17. rejection of duplicate candidate identities;
18. prevention of moving an active block before clearance;
19. prevention of reassigning/rerouting an active transfer mid-operation;
20. treatment of not-applicable Pareto metrics as incomparable, not automatically better.
21. sequential recovery from a delayed clearance followed by a vehicle and corridor disruption;
22. persistence of an earlier vehicle outage across later planning stages, with fail-closed stop on an invalid recovery;
23. rejection of empty, duplicate-ID and time-reversed disruption sequences.

A separate integration test exhaustively enumerates every integer placement for one disturbed 2×2 block in a 6×4 yard. Its cell-set oracle finds exactly five feasible recovery layouts and identifies the minimum-edge-shift recovery while leaving unaffected reservations unchanged. It does not call the production candidate-coordinate generator and pins the expected recovery horizon.

These are authored unit scenarios. The exact-head workflow must pass before they are treated as verified results. The integration oracle's five-layout count is also an expected result until verified on the exact commit. The evaluator models rectangle-zone intersections, exclusive vehicle outages and explicit occupied-until extensions for known placements; it does not infer crane/lift clearance from sensor signals, calculate the physical move, compute a placement/route, model workstation capacity, infer an affected set for every possible disruption type, or prove that a recovery is globally optimal.

## Finite-resource constraints and joint acceptance

The public \`resources\` module now provides an independent unary-resource schedule checker for resource IDs representing a workstation, crane, tug, lift team, inspection station or another explicitly named resource. Each reservation has a stable reservation ID, operation ID, resource ID/kind and half-open time interval. The verifier rejects overlapping reservations on one unary resource, profile-kind drift, malformed/duplicate IDs, unknown outage targets, resource outages that overlap candidate work, and attempts to rewrite completed or move active work. It measures changed reservations, unaffected resource churn, absolute time shift, makespan and recovery horizon.

\`evaluate_joint_recovery\` composes the spatial recovery report and resource recovery report. Joint acceptance is true only when both reports accept. A plan cannot earn a pass by packing blocks neatly while double-booking a crane or ignoring an unavailable workstation. This is still a narrow unary-resource gate: capacity greater than one, two-crane tandem lifts, shared rigging constraints, crane interference geometry, swept paths, operator qualifications, load charts, work precedence and setup/changeover times are not yet represented. Resource IDs must refer to authoritative equipment/workstation profiles.

There are six additional resource-level tests for half-open endpoint behavior, future-operation recovery after an outage, in-progress immutability, completed-history immutability, unknown-resource rejection, and joint spatial/resource rejection. These are authored tests awaiting exact-head CI verification.

Current literature supports this next direction. An August 2026 multi-fidelity ant-colony study models multi-crane scheduling with safety distance, interference, precedence and tandem scheduling, and reports 7–13% shorter crane travel distance than its compared methods on its own real-world data ([Applied Soft Computing, DOI 10.1016/j.asoc.2026.115384](https://doi.org/10.1016/j.asoc.2026.115384)). Our unary resource module does not reproduce that model and makes no percentage-improvement claim. It adds a small, independently testable feasibility layer before attempting richer crane-coupling models.

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
