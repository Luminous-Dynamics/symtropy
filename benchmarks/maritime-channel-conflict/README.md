# Maritime channel direction and conflict contract v0

Dependency-light Rust verifier for one-way channel direction windows, same-segment headway and spatial interaction constraints over fixed fleet-oracle operation placements.

## Composition boundary

This crate consumes the existing `OperationPlacement` records from [MARITIME-FLEET-001 #3717 / PR #3724](https://github.com/Luminous-Dynamics/symtropy/pull/3724). It does not create a second fleet scheduler, route optimizer, VTS, hydrographic database or vessel-physics engine.

It verifies:
- movements follow explicit, route-revision-bound segment direction windows;
- an explicitly closed window or wrong one-way direction is infeasible;
- unrepresented time windows remain unknown, not implicitly open;
- same-segment operations obey evidence-bound headway;
- every unordered pair of distinct segments has an explicit interaction declaration;
- a pair can be declared Independent only with a current, subject-bound topology receipt;
- a conflict pair meets its declared minimum separation;
- source evidence is bound to exact segment/segment-pair identity and route revision.

## Closed-world topology rule

For a workload with N distinct channel segments, the interaction matrix must declare all N(N-1)/2 unordered segment pairs. Each pair says either:
- Independent: the reviewed topology says these two segments do not share a traffic conflict; or
- Conflict: they share a conflict area and a minimum empty separation interval is required.

A missing pair declaration is NeedsEvidence. It is never interpreted as no conflict. This is intentionally stricter than a graph representation in which omitted edges are silently non-interacting.

Direction windows are likewise explicit. A movement must fit entirely inside one declared ForwardOnly, ReverseOnly or Closed interval. Gaps remain unknown. The initial model does not infer a direction schedule from a static "one-way channel" label.

## Time semantics

All movement operations use the exact fleet placement and one caller-declared integer-tick basis. Operation intervals are half-open: an operation ending at tick 20 and another starting at tick 20 do not overlap. A configured minimum separation of zero permits that boundary handoff; positive separation requires an actual empty interval of at least that many ticks.

This is a discrete interval verifier only. It does not calculate physical vessel separation, collision probability, swept path, bank effects, visibility, current, tide, under-keel clearance or navigability.

## Evidence semantics

Each segment direction window and same-segment headway receipt binds:
- exact segment ID;
- exact route revision;
- source and source revision;
- evidence status;
- valid-time bounds and claim ceiling.

Each pair interaction receipt binds the canonical pair subject `lower-segment-id|higher-segment-id` and route revision. Evidence strings are references/review metadata; this crate does not authenticate publishers, fetch artifacts or prove licensing.

## Run

~~~sh
cargo fmt --manifest-path benchmarks/maritime-channel-conflict/Cargo.toml -- --check
cargo test --locked --manifest-path benchmarks/maritime-channel-conflict/Cargo.toml
cargo clippy --locked --manifest-path benchmarks/maritime-channel-conflict/Cargo.toml --all-targets -- -D warnings
~~~

## Tests

The current synthetic corpus covers:
- forward, reverse and closed-window movement;
- missing direction coverage, stale and revoked evidence;
- route revision mismatch and exact subject binding;
- same-segment overlap, configured headway, exact-boundary and half-open handoff;
- cross-segment conflict overlap and required clearance;
- explicit independent pairs, missing interaction declarations, stale pair evidence and evidence validity;
- canonical pair identity, structural duplicates, invalid endpoints and input-order determinism;
- an independent test-only direct headway oracle over **512 frozen timing/clearance combinations**.

The tests validate the discrete verifier only. They do not establish that the supplied channel graph, interaction matrix or time windows represent a real port correctly.

## Promotion gates

Before connection to a real port plan:
1. The complete segment-pair matrix must derive from rights-reviewed, versioned channel/basin geometry and conflict-zone studies.
2. Direction windows must be sourced from the applicable traffic-service/authority rules and valid-time period; no inferred opening or permission.
3. Separation values need a source-defined model and uncertainty; they are not universally valid safety distances.
4. Route and topology revisions must invalidate prior movement mappings.
5. Compare against hand-checked 2–8 movement exhaustive cases and a matched multi-basin/one-way-channel MILP/heuristic baseline.
6. Physical trafficability and navigation clearance remain with the qualified harbour authority and validated ship-handling process.

The output is an evidence-qualified comparison disposition, not a VTS order, traffic clearance, collision-avoidance certificate or operational authority.
