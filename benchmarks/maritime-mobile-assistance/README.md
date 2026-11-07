# Maritime mobile assistance contract v0

Dependency-light Rust verifier for named tug and pilot assignments against fixed operation placements emitted by the existing shared-resource fleet oracle.

This crate **does not schedule vessels or choose assignments**. It verifies whether a caller-supplied assignment can satisfy:
- named tug count, accepted tug-class identifiers and aggregate bollard-pull capability in kN;
- exact pilot qualification identifiers;
- resource availability and operation-window feasibility;
- non-overlapping use of each tug and pilot;
- resource-specific waypoint-to-waypoint reposition timing against exact resource identity and route-revision-bound evidence legs; every leg has a globally unique opaque `leg_id`, must reference a known tug/pilot resource, and its evidence subject uses a versioned, length-prefixed canonical encoding of leg ID, resource, direction, route revision, duration and time basis;
- current, time-valid capability, position, qualification and reposition evidence. Evidence validity endpoints are inclusive while operation/travel windows are half-open; a receipt must cover through the final occupied tick (end minus one), not the exclusive end boundary.

## Compose; do not duplicate

The upstream owner remains [MARITIME-FLEET-001 #3717 / PR #3724](https://github.com/Luminous-Dynamics/symtropy/pull/3724), which chooses fixed integer-tick operation placements under shared channel, lock, berth, terminal, water, energy and inland capacities.

This mobile-assistance verifier consumes the fleet API's `OperationPlacement` and checks only the named-resource assignment layer. The `evaluate_mobile_assistance_for_plan` entry point additionally proves that every assistance task exactly matches an operation in the supplied `FleetPlan` and the demand's selected participant/alternative. A forged time window, unselected alternative, missing demand selection or non-member operation is rejected. This is structural plan binding, not cryptographic provenance/authentication of the `FleetPlan` value. It does not add another search, scheduler, route planner, Pareto optimizer or shared-capacity oracle.

## Model boundary

- One assistance task binds to the exact fleet demand, alternative, segment and time interval.
- Every tug and pilot has a stable identity, configuration revision, initial waypoint and declared availability windows.
- Tug bollard pull is supplied in kN by an upstream evidence-backed requirement/resource profile. It is never inferred from vessel length or class.
- Pilot qualifications are opaque identifiers supplied by the responsible source owner; this crate does not certify credentials.
- A reposition leg binds an exact tug or pilot ID, directed waypoint pair, route revision and evidence receipt. Tug and pilot legs for the same waypoints may have different durations. This crate verifies those supplied durations; it does not calculate the route, speed, hydrodynamic force, fuel or energy.
- Travel is scheduled just in time before the next operation. The travel window, subsequent operation window, resource availability and evidence validity are checked separately.
- Missing, stale, conflicting, unsupported, revoked or wrong-revision evidence cannot yield `ReadyForComparison`.
- A hard resource/capability/timing conflict is `Infeasible`; missing or unresolved evidence is `NeedsEvidence`; revoked/unsupported evidence is `Unsupported`.
- No result dispatches a tug, assigns a pilot, issues VTS clearance, books a berth or grants navigation authority.

All times are abstract integer ticks in a caller-declared common time basis. The initial model uses direct waypoint-pair travel legs rather than graph pathfinding. Multiple legs/path alternatives and time-dependent route-cost models are later extensions only when backed by a declared graph and reference evidence.

## Run

~~~sh
cargo fmt --manifest-path benchmarks/maritime-mobile-assistance/Cargo.toml -- --check
cargo test --locked --manifest-path benchmarks/maritime-mobile-assistance/Cargo.toml
cargo clippy --locked --manifest-path benchmarks/maritime-mobile-assistance/Cargo.toml --all-targets -- -D warnings
~~~

## Tests

The unit fixtures currently contain **32 tests** and cover:
- feasible two-task tug/pilot continuity;
- tug double-booking, wrong-class rejection and half-open handoff;
- insufficient reposition time;
- insufficient tug count and bollard pull;
- pilot qualification mismatch or omission;
- missing reposition evidence, resource-specific tug/pilot durations, rejection of tug-to-pilot evidence reuse, stale capability evidence, revoked position evidence, wrong route revision, duration-tampering rejection, orphan/duplicate leg rejection and delimiter-collision negative controls;
- availability limits, duplicate assignments and deterministic permutation replay.

A separate test-only reference path recomputes one-tug/two-task feasibility directly over **512 frozen timing combinations**. It does not call the production reposition or sequence helpers. This is a synthetic algorithmic cross-check only; it is not experimental validation of tug movement or capability.

## Promotion requirements

Before this can inform a real port decision:
1. A port/vessel engineering profile must define the assistance tasks and required tug/pilot capabilities for the exact vessel, operation and environmental context.
2. Resource identities, bollard-pull records, qualifications, positions, duty and availability evidence need source/rights/currentness review.
3. Reposition times must be derived from a declared route/network model with validated assumptions and uncertainty; this v0 contract accepts them as evidence-bound inputs only.
4. The assignment feasibility verifier must be independently compared with exhaustive tiny assignment cases and matched published scheduling baselines.
5. The result remains decision-support only. Mycelix/operator authority and real dispatch systems remain outside this crate.

Synthetic tests do not demonstrate safe tug operations, pilot qualification, navigability or regulatory compliance.
