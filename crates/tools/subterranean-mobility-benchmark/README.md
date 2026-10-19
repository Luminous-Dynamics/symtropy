# Subterranean mobility benchmark v0.1

A standard-library-only Rust reference kernel for fail-closed passage admissibility. It supports mine, cave, tunnel, and unknown-void research scenarios but makes no claim of physical route safety, equipment qualification, or rescue certification.

## Run

From a Rust toolchain compatible with edition 2024:

~~~sh
cargo test --offline --manifest-path crates/tools/subterranean-mobility-benchmark/Cargo.toml
cargo run --offline --manifest-path crates/tools/subterranean-mobility-benchmark/Cargo.toml --bin subterranean-mobility-benchmark
~~~

## Contract

- Evaluates a declared person/PPE/load/aid profile against supplied passage geometry and evidence.
- Uses integer units and conservative uncertainty bounds for clear width, height, turning radius, grade, steps, and traction.
- Requires freshness-bearing hazard evidence; a known blocking hazard rejects the passage.
- Supports direction-specific grade and step limits; reverse travel requires an explicit bidirectional declaration.
- Qualified support evidence is required only when the traversal profile demands it for a vertical transition. Software cannot certify an anchor or rigging system.
- Returns one of three outcomes: admissible within the declared envelope, rejected by a known hard constraint, or unknown because required evidence is missing/stale.
- Preserves reason codes. A known hard violation remains a rejection even if another required observation is missing.
- Searches only edges passing all hard checks. Unknown edges do not become safe because they are part of the shortest path.
- Uses stable lexical ordering and an explicit expansion, pending-frontier, and hop cap. Hitting any cap returns SearchLimitExceeded without returning a partial route or a definitive no-route claim.

## Limits

This is an evidence filter over supplied data, not a perception system or physical simulation. It does not assess rock stability, geotechnical support, atmosphere, medical suitability, real rope/anchor strength, current water dynamics, human biomechanics, device snagging, braking in a real environment, or rescue-team competence. A status of admissible means only that the provided fresh measurements satisfy the provided profile. It must not be used as an operational authorization.

See SUBTERRANEAN-HUMAN-MOBILITY-001 in the docs/research directory, implementation task #2039, and FLOOD-EGRESS-001 #1963.


## Communications-loss / egress-coordination contract

The egress coordination module separates coordination support from route admissibility:

- Live coordination requires a current location fix, a fresh local plan matching the declared route-graph version, and a live acknowledged communication path.
- Store/carry-forward messages are deferred, not reported as delivered.
- With no current path, fresh local location and a version-matched offline plan may remain locally available; that does not mean the route is still physically clear.
- Missing/stale location or plan evidence yields an unknown status. Explicitly unavailable location or missing/unverified plan evidence prevents a local-inputs-available claim.
- The route-search disposition is returned unchanged. No admissible route, a bounded/incomplete search, and coordination loss remain distinct outcomes.

This is a deterministic interface contract, not an end-to-end network simulator, localization algorithm, live closure detector, or operational egress authorization.


## Timed mission-trace replay

The `trace` module replays authored state-change events against a simulated clock. It is a mission-state replay harness, not a radio simulator or physical route-safety model.

- Event arrival time and evidence observation time are distinct, so delayed localization/plan evidence can already be stale at receipt.
- Checkpoint events advance simulated time and re-evaluate freshness without inventing new sensor readings or network acknowledgements.
- Communications, localization, local-plan availability and route-search disposition remain separate axes.
- Event IDs must be non-empty and unique; timestamps must be non-decreasing. Events sharing a timestamp preserve fixture order as the explicit causal tie-break.
- Event-cap overflow returns `EventLimitExceeded` with zero processed events and no partial samples.
- Replaying the same event sequence yields the same ordered report and reason codes.

The trace is only as accurate as its declared events and evidence. It does not model per-hop queues, airtime, RF/TTE propagation, actual delivery probability, localization error dynamics, physical hazard evolution, or rescue authorization. See [MINE-MISSION-TRACE-001 #2097](https://github.com/Luminous-Dynamics/symtropy/issues/2097) and [Symthaea MINE-COMMS-TRACE-001 #8066](https://github.com/Luminous-Dynamics/symthaea/issues/8066).


### Version-bound local plans

Every local plan in timed traces declares the exact graph version it was prepared against. A graph-version change invalidates a plan from an earlier version immediately, even if its observation timestamp is still fresh. A matching replacement plan is required before the local-plan input can be reported available again. Route-search disposition remains a separate value: neither a new graph version nor a successful communications path independently establishes a physically safe route.
