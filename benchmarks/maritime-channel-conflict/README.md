# Maritime channel conflict contract v0

Dependency-light Rust verifier for abstract one-way channel, merge, crossing and basin-entry movement conflicts. It consumes the selected `FleetPlan` and checks the exact operation intervals against a caller-supplied, versioned edge/direction/conflict graph.

This is **not a scheduler or vessel dynamics model**. The shared-resource fleet oracle still chooses candidate operation placements; this crate checks whether the selected placements satisfy the declared graph policy.

## What is checked

- A graph maps fleet segment IDs to unique edge IDs and directed node pairs.
- Each edge declares non-overlapping direction windows and allowed direction IDs.
- Each channel movement is bound to an exact fleet operation placement, selected demand/participant/alternative, edge ID and route revision.
- Every fleet operation using a segment declared by the graph must have exactly one corresponding channel movement. Missing movement coverage yields `NeedsEvidence`.
- Every movement interval must fit wholly inside one direction window. A missing coverage window yields `NeedsEvidence`; a declared but disallowed direction yields `Infeasible`.
- Symmetric conflict rules can model opposing directions on one channel, shared merges, crossings, basin entrances or following traffic on the same edge/direction.
- Conflict rules enforce no overlap and a minimum separation/headway over half-open operation intervals.
- Report ordering is deterministic under input permutations. Identical conflict reports are produced independent of movement ordering.

## Evidence and claim boundary

`TopologyReviewState::Reviewed` is a caller-supplied intake/review disposition. It is not source authentication, a proof that the graph is physically complete, or validation of local navigation rules. The graph has an exact route revision and time basis; external intake must bind producer/source, edition/revision, jurisdiction, coverage, CRS/datum, update validity, uncertainty, terms and raw data digests before a real-world comparison.

The direction windows and conflict relations are abstract scheduling constraints. This crate does not calculate:
- vessel swept paths, collision probability, hydrodynamics, bank suction/cushion or passing-ship forces;
- bathymetry, under-keel clearance, tide/current, weather or wave response;
- pilot judgement, VTS permission, navigation clearance or legal right-of-way;
- traffic optimization, tug/pilot assignment, resource dispatch or vessel control.

A passing synthetic graph test is not a navigability decision, physical qualification, MASS approval or operational authority.

## Compose; do not duplicate

- [MARITIME-FLEET-001 #3717 / PR #3724](https://github.com/Luminous-Dynamics/symtropy/pull/3724) owns selected multi-demand operation placements and shared capacities.
- [MARITIME-MOBILE-OPS-001 #3798](https://github.com/Luminous-Dynamics/symtropy/issues/3798) owns the larger mobile-resource and spatial-traffic integration research.
- [MARITIME-MANOEUVRING-001 #3865 / PR #3878](https://github.com/Luminous-Dynamics/symtropy/pull/3878) owns trajectory metrics and manoeuvring evidence contracts.
- Symthaea supplies vessel/model applicability and physical manoeuvring envelopes; Sol-Atlas supplies versioned geography/hydrographic context; Mycelix remains authority for participant commitments and approvals.

## Run

~~~sh
cargo fmt --manifest-path benchmarks/maritime-channel-conflict/Cargo.toml -- --check
cargo test --locked --manifest-path benchmarks/maritime-channel-conflict/Cargo.toml
cargo clippy --locked --manifest-path benchmarks/maritime-channel-conflict/Cargo.toml --all-targets -- -D warnings
~~~

## Test corpus

Unit cases cover:
- opposing one-way traffic under time-bounded direction windows;
- insufficient clearance headway;
- merge and crossing conflicts;
- same-edge half-open handoff;
- prohibited direction and missing direction coverage;
- required movement coverage for every declared FleetPlan segment;
- exact FleetPlan operation and selected-alternative binding;
- Unknown/Unsupported topology review states;
- conflict graph structural errors and permutation invariance.

A separate direct tick-by-tick reference oracle sweeps **3,136** frozen tiny two-movement interval/headway combinations. It independently detects overlap by testing active occupancy at each tick and counts free ticks in the separation interval. This verifies the interval kernel for that synthetic range only; it does not validate graph completeness or real-world collision/traffic rules.

## Promotion path

1. Bind graph nodes/edges to source-qualified chart/port data and stable route revisions, with rights and update lineage.
2. Add vessel-specific operation geometry and direction/tide/under-keel envelopes from qualified upstream owners; do not infer them from this graph.
3. Add conflict-zone completeness review and held-out route/port topology cases.
4. Reproduce published dual one-way channel, multi-basin berth allocation and lock/port coupling benchmarks on matched problem instances.
5. Evaluate port-level and corridor-level tail service metrics against the strongest reproducible baselines.

No superiority claim is made by this v0.
