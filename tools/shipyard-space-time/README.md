# Shipyard space-time reference verifier (V0)

A dependency-free Rust verifier for a deliberately bounded shipyard block-staging and transfer model. It is an independent child of [SHIPYARD-SPACE-001 #2050](https://github.com/Luminous-Dynamics/symtropy/issues/2050) and complements the schedule-to-cost work in [PR #2430](https://github.com/Luminous-Dynamics/symtropy/pull/2430).

## Run

```sh
cargo fmt --all --manifest-path tools/shipyard-space-time/Cargo.toml -- --check
cargo test --locked --offline --manifest-path tools/shipyard-space-time/Cargo.toml
cargo clippy --locked --offline --manifest-path tools/shipyard-space-time/Cargo.toml --all-targets -- -D warnings
```

This is a separate nested Cargo workspace with no external dependencies. CI checks the frozen scenario manifest SHA-256 before testing.

## Paradigm shift: verify space-time feasibility from evidence, not planner assertions

The verifier reconstructs violations from concrete reservations; it does not trust a supplied `feasible=true` flag or try to repair the schedule. An empty reservation set emits `NoOperationalReservations`, so vacuous truth cannot be reported as a qualified yard plan. Missing or invalid finite yard bounds are rejected; every supported block footprint and transfer/access corridor must fit entirely inside the declared yard rectangle.

V0 supports:
- one explicit finite yard boundary and axis-aligned rectangular footprints on an integer coordinate lattice;
- half-open integer-tick reservation intervals `[start, end)`;
- block placements/staging occupancy;
- declared rectangular transfer/access corridors;
- exclusion zones active over declared time ranges;
- explicit move predecessors and exclusive vehicle reservations.

It detects overlapping block footprints in overlapping time, one block booked in two places at once, a corridor blocked by another block, a move intersecting an exclusion zone, a block placed inside an active exclusion, overlapping uses of the same vehicle, missing predecessors and predecessor timing violations. Output violations are machine-readable and sorted deterministically. Touching at a shared footprint edge or time endpoint is allowed under the declared half-open contract.

Unsupported geometry is first-class and **not feasible**. A polygon profile, unknown footprint or invalid rectangle is not interpreted as empty space.

## Independent checks and frozen corpus

The frozen fixture at `tests/fixtures/space-time-v0.tsv` has raw-byte SHA-256:

`7a1b65b926ad7417902ef38f69040a7757dc89fd038801d93afaee881298df00`

It contains 17 cases spanning feasible edge-touch/time-separated controls and negative controls for overlaps, double booking, blocked transfers, vehicle conflicts, precedence, missing predecessors, exclusions, unsupported geometry, invalid time/rectangle, missing yard bounds, and out-of-bounds blocks/corridors. The integration oracle parses the fixture and compares **per-code violation counts** with separately listed expected outcomes. Two unit tests independently compare rectangle intersection to integer-cell rasterization and half-open interval overlap to explicit tick scanning; an additional test checks input-order invariance.

These checks provide bounded correctness evidence, not proof of arbitrary polygon geometry, general optimizer correctness, production shipyard safety or real-world throughput. The test suite must pass on the exact PR head before the milestone can be called verified.

## Explicit limits

V0 does **not** model arbitrary polygons, rotation of footprints, non-rectangular swept paths, vertical clearance, turning radius, lift/load/unload dynamics, cranes/rigging, stack/support rules, soil/floor bearing capacity, structural loads, human exclusion/safety envelopes, physical vehicle acceleration, or transport capacity greater than one exclusive vehicle. The corridor is supplied by the scenario; V0 does not compute a route. Unsupported dimensions need a distinct qualified model, not an assumption that clearance exists.

The existing [industrial workcell validator PR #1967](https://github.com/Luminous-Dynamics/symtropy/pull/1967) checks timed task/resource capacity and precedence in its own model. This crate focuses on the spatial question its exclusive-resource abstraction cannot answer: whether block footprints and required access corridors are simultaneously compatible with reservations over time. The intended later integration should compose independently checked reports, not duplicate or bypass either authority.

## Benchmark roadmap

Do not build an optimizer on top of a permissive or under-specified verifier. First gate on exact agreement with the frozen spatial/time fixture, all negative controls rejected, explicit finite yard bounds, zero false-feasible outputs, and deterministic reports. Then follow the baseline ladder below before claiming improvements. Yard-area utilization, block re-handles, travel time, makespan/delay, recovery time, schedule churn, compute time and memory are separate metrics—not one opaque score.

## Comparative benchmark plan: a measurable path toward state of the art

A recent directly relevant study, Wang et al. (first published 11 July 2026), formulates irregular-block spatial scheduling under workstation and vehicle disruptions as a mixed-integer model and proposes a hybrid artificial bee colony / adaptive large-neighbourhood-search / recovery procedure. Its objectives include space utilization and vehicle scheduling time. We do not claim to reproduce its model or beat its reported results; the present verifier covers a smaller contract—rectangular footprints and explicit space-time conflicts. See https://doi.org/10.1049/cim2.70066 and the prior irregular-block study https://doi.org/10.1016/j.cie.2020.106985.

Generic schedule optimization should be qualified separately against the pinned JSPLib/FJSPLib instance revisions and best-known bounds in https://scheduleopt.github.io/benchmarks/. The published protocol compares CPO, CP-SAT, OptalCP and CPLEX under a standardized 10-minute budget and records optimality gaps/bound deviation. Generic job-shop performance must not be presented as shipyard geometric validation.

### Baseline ladder

1. **Verifier gate:** frozen feasible and infeasible reservations; exact per-code expected violations; zero false-feasible results in negative controls; independent rectangle/time oracles.
2. **Trivial placement baselines:** first-fit and bottom-left/first-fit-decreasing using a frozen order and deterministic tie-breaks. Every placement is passed through the independent verifier.
3. **Transparent greedy baseline:** choose among currently feasible staging locations and move slots by earliest-ready/earliest-due or a declared space-time score. Freeze the score before test cases; include cases where the greedy baseline fails.
4. **Exact tiny-instance baseline:** MILP/CP-SAT or exhaustive search for a small, finite scenario family. Report objective bounds and gaps, not only the candidate's result. The verifier remains independent of all planners.
5. **Published-method comparison:** reconstruct the Wang et al. benchmark and ABC/ALNS/recovery assumptions only after its input definitions, objective weights, termination conditions, and instance data are available in an independently reproducible form. If reproduction is incomplete, report a paper-level conceptual comparison, not a numerical head-to-head.

### Robustness-first paradigm shift

Static yard-area utilization is not enough: a dense layout may make the next block impossible to move or make recovery after a truck outage much worse. Optimize the space-time plan and its recoverability together, and publish a Pareto frontier rather than hiding trade-offs in one score.

For each frozen baseline and candidate, measure separately:
- feasibility rate and false-feasible count from the independent verifier;
- usable area-time utilization (state the denominator, exclusions, and occupancy window);
- moves and re-handles per completed block;
- vehicle travel / occupied transfer time and vehicle conflict count;
- makespan, delayed operations and weighted tardiness;
- time to restore a feasible schedule after each named disruption;
- schedule churn (number and weighted cost of previously unaffected reservations changed);
- objective regret versus the best feasible plan known for the same post-disruption state;
- solver/verifier latency, peak memory, and timeout/failure rate.

Use three levels of test data: exhaustive tiny cases with provable answers; frozen medium/large synthetic cases generated from versioned manifests; and, only with permission, de-identified yard observations for external validity. Disruptions should include a vehicle becoming unavailable, workstation finishing late, destination remaining occupied, new exclusion zone, late block arrival, and inspection rejection. Include impossible-to-recover controls.

Keep two objectives distinct:
- **nominal efficiency**: quality before a disruption;
- **robust recourse**: feasibility and degradation after a declared disturbance set.

A proposed stretch goal, to be evaluated rather than assumed achieved, is **zero false-feasible outputs on all published negative controls and better recovery regret at no more than a pre-registered 5% nominal objective penalty** relative to the strongest feasible baseline. Revise or reject the target if the frozen pilot data show it is ill-posed. Do not report improvement percentages until baseline outputs, the metric definitions, and the test set are frozen before evaluating the candidate.

## Cross-project responsibility

- **Symtropy:** space-time reservations, access/resource feasibility and schedule/disruption comparison.
- **Symthaea:** vessel requirements, physical-model applicability and separately qualified engineering evidence.
- **Mycelix:** authorized source/custody/inspection provenance, preserving event time and the strength/limitations of source claims.
- **Sol Atlas:** read-only site geometry and spatial/source freshness context.

A deterministic receipt proves what the verifier computed from stated inputs; it does not prove that the inputs describe the physical yard accurately.
