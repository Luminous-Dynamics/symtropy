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

The verifier reconstructs violations from concrete reservations; it does not trust a supplied `feasible=true` flag or try to repair the schedule. An empty reservation set emits `NoOperationalReservations`, so vacuous truth cannot be reported as a qualified yard plan.

V0 supports:
- axis-aligned rectangular footprints on an integer coordinate lattice;
- half-open integer-tick reservation intervals `[start, end)`;
- block placements/staging occupancy;
- declared rectangular transfer/access corridors;
- exclusion zones active over declared time ranges;
- explicit move predecessors and exclusive vehicle reservations.

It detects overlapping block footprints in overlapping time, one block booked in two places at once, a corridor blocked by another block, a move intersecting an exclusion zone, a block placed inside an active exclusion, overlapping uses of the same vehicle, missing predecessors and predecessor timing violations. Output violations are machine-readable and sorted deterministically. Touching at a shared footprint edge or time endpoint is allowed under the declared half-open contract.

Unsupported geometry is first-class and **not feasible**. A polygon profile, unknown footprint or invalid rectangle is not interpreted as empty space.

## Independent checks and frozen corpus

The frozen fixture at `tests/fixtures/space-time-v0.tsv` has raw-byte SHA-256:

`fb3021523403f88f9573ce563f71778074a441ca817d220edc6fd1c8075617a9`

It contains 14 cases spanning feasible edge-touch/time-separated controls and negative controls for overlaps, double booking, blocked transfers, vehicle conflicts, precedence, missing predecessors, exclusions, unsupported geometry and invalid inputs. The integration oracle parses the fixture and compares **per-code violation counts** with separately listed expected outcomes. Two unit tests independently compare rectangle intersection to integer-cell rasterization and half-open interval overlap to explicit tick scanning; an additional test checks input-order invariance.

These checks provide bounded correctness evidence, not proof of arbitrary polygon geometry, general optimizer correctness, production shipyard safety or real-world throughput. The test suite must pass on the exact PR head before the milestone can be called verified.

## Explicit limits

V0 does **not** model arbitrary polygons, rotation of footprints, non-rectangular swept paths, vertical clearance, turning radius, lift/load/unload dynamics, cranes/rigging, stack/support rules, soil/floor bearing capacity, structural loads, human exclusion/safety envelopes, physical vehicle acceleration, or transport capacity greater than one exclusive vehicle. The corridor is supplied by the scenario; V0 does not compute a route. Unsupported dimensions need a distinct qualified model, not an assumption that clearance exists.

The existing [industrial workcell validator PR #1967](https://github.com/Luminous-Dynamics/symtropy/pull/1967) checks timed task/resource capacity and precedence in its own model. This crate focuses on the spatial question its exclusive-resource abstraction cannot answer: whether block footprints and required access corridors are simultaneously compatible with reservations over time. The intended later integration should compose independently checked reports, not duplicate or bypass either authority.

## Benchmark roadmap

First gate: 100% agreement with hand-enumerated fixtures, all spatial/resource negative controls rejected, unsupported cases preserved as non-feasible, and deterministic reports under input permutation.

Next add a tiny exhaustive placement/transfer oracle and disruption-recovery cases, then compare first-fit and transparent greedy baselines against a bounded candidate planner. Report yard area-time utilization, block re-handles, vehicle moves, makespan/delay, access blockage, recovery time, schedule churn, feasibility rate, compute time and memory separately. Freeze the scenario generator, model switches, seeds and raw outputs.

For generic scheduling, use pinned [JSPLib](https://scheduleopt.github.io/benchmarks/jsplib/) and [FJSPLib](https://scheduleopt.github.io/benchmarks/fjsplib/) versions and report feasible/optimal counts and per-instance bounds/gaps. ScheduleOpt describes a standardized 10-minute comparison against CP Optimizer, CP-SAT, OptalCP and CPLEX and publishes best-known bounds. Those job-shop results are a generic solver baseline, not shipyard-space validation.

Shipyard-specific research includes Wang et al. (first published 11 July 2026), which studies irregular block spatial scheduling and disruption recovery with a mixed-integer model: [IET Collaborative Intelligent Manufacturing](https://doi.org/10.1049/cim2.70066). The paper provides a baseline to reconstruct—not evidence that this crate reproduces its model or exceeds its results. Its stated simplifying assumptions motivate explicit profile switches and adversarial tests around transfer and handling constraints.

No percentage improvement target is declared before the baseline implementation and comparison corpus are frozen. Any industrial productivity claim requires permissioned, longitudinal build data. Physical lift/crane and statutory safety decisions require their own qualified models and approvals.

## Cross-project responsibility

- **Symtropy:** space-time reservations, access/resource feasibility and schedule/disruption comparison.
- **Symthaea:** vessel requirements, physical-model applicability and separately qualified engineering evidence.
- **Mycelix:** authorized source/custody/inspection provenance, preserving event time and the strength/limitations of source claims.
- **Sol Atlas:** read-only site geometry and spatial/source freshness context.

A deterministic receipt proves what the verifier computed from stated inputs; it does not prove that the inputs describe the physical yard accurately.
