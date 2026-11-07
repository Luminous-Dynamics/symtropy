# Maritime robust traffic envelope contract v0

A dependency-light Rust verifier for the selected operation placements of the existing multi-demand fleet oracle.

## Compose; do not duplicate

The upstream owner remains [MARITIME-FLEET-001 #3717 / PR #3724](https://github.com/Luminous-Dynamics/symtropy/pull/3724). That oracle chooses exact integer-tick placements under declared shared channel, lock, berth, terminal, water, shore-power and inland-transfer capacity.

This crate consumes a selected `FleetPlan`. It verifies a different layer: whether each vessel operation's uncertainty-expanded occupancy fits the declared one-way direction windows and exclusive crossing/merge resources, and whether any pair of operations conflict. It is not another scheduler, route optimizer, spatial engine or Pareto search.

## Paradigm shift: verify a robust envelope, not a nominal timestamp

Each occupancy preserves the fleet plan's exact operation and adds explicit:
- earliest entry offset;
- latest clearance offset;
- route revision and direction;
- source-bound validity evidence.

Each traffic resource contributes:
- versioned resource identity;
- typed semantics: one-way channel, exclusive conflict zone or exclusive merge zone;
- a sorted, non-overlapping availability/direction-window schedule;
- clearance guards before and after occupancy.

The conservative occupancy is the half-open interval:
`[scheduled_start - early_entry - clearance_before, scheduled_end + late_clearance + clearance_after)`.

The verifier checks that this whole interval fits one declared resource window. For a one-way channel, the declared window must permit the exact direction. All occupancies of a resource have capacity one in v0; guarded intervals may not overlap. This naturally catches a crossing/merge conflict even when the vessels use different route-edge IDs, provided the independently reviewed graph maps both operations to the same conflict-zone ID.

Evidence subject IDs are versioned and length-prefixed. Resource receipts bind the exact resource profile, including all windows, directions and clearance guards. Occupancy receipts bind the exact selected fleet operation, its route revision, direction, uncertainty bounds and time basis. Changing a plan timestamp, uncertainty, resource window or guard invalidates the relevant receipt.

## Run

~~~sh
cargo fmt --manifest-path benchmarks/maritime-traffic-envelope/Cargo.toml -- --check
cargo test --locked --manifest-path benchmarks/maritime-traffic-envelope/Cargo.toml
cargo clippy --locked --manifest-path benchmarks/maritime-traffic-envelope/Cargo.toml --all-targets -- -D warnings
~~~

## Test scope

The synthetic unit corpus covers:
- safe half-open handoff and nominal separation;
- nominally separate movements that conflict under timing uncertainty;
- resource clearance guards;
- alternating one-way direction windows and wrong-direction rejection;
- uncertainty envelopes crossing a direction-window boundary;
- stale/revoked evidence;
- exact resource-profile and occupancy-envelope subject binding;
- exact FleetPlan operation and participant/alternative selection binding;
- evidence expiry, deterministic permutation and invalid window sets;
- pre-decision occupancy rejection.

A separate test-only direct interval oracle checks **256 frozen timing combinations** using arithmetic interval overlap. It does not call the production effective-occupancy or conflict-detection functions.

These tests establish software behavior for declared abstract resources only. They do not validate a port's traffic graph, safe headway, channel direction policy, vessel dynamics, manoeuvring room, under-keel clearance or collision risk.

## Explicit limits

- The conflict graph and resource mapping are supplied inputs; the crate does not infer them from GIS or vessel geometry.
- Timing uncertainty and clearance guards are supplied inputs; no predictor is trained or calibrated here.
- One resource has capacity one. Parallel traffic, passing bays and validated multi-lane capacity require a separately declared resource model.
- The checker validates an existing schedule; it does not optimize, reschedule or create alternatives.
- No VTS, pilot, master, port authority or operator approval is represented.
- A passing synthetic contract is not navigability clearance, collision-avoidance certification, MASS approval or permission to operate.

## Promotion path

1. Reproduce small abstract conflict-graph cases and compare against an independent exhaustive pairwise oracle.
2. Intake route/traffic-service semantics from exact IHO S-127 and applicable local port rules, with edition, jurisdiction, update, validity, source rights and transformation provenance.
3. Build a rights-reviewed port benchmark with observed movement events, stated headway policy, channel direction windows and conflict-zone mapping.
4. Compare the joint fleet + mobile assistance + traffic verifier against FCFS and the strongest reproducible literature baseline under identical inputs and hardware budgets.
5. Add robust/stochastic recourse only after operation-time error distributions are estimated from held-out historical movements and calibrated by regime.

No better-than-SOTA claim is made by this crate.
