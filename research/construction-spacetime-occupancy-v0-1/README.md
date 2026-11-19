# Construction Space-Time Occupancy Core — v0.1

**Status:** synthetic, conservative AABB overlap oracle with an end-to-end accepted-floor integration test. Not a collision-avoidance system, safety controller, site clearance authority, lift plan or physical qualification.

Parent: [CONSTRUCTION-SPACETIME-OCCUPANCY-001 #4974](https://github.com/Luminous-Dynamics/symtropy/issues/4974).

Stacked dependency order: [construction interop #3644](https://github.com/Luminous-Dynamics/symtropy/pull/3644) → [accepted-floor ledger #4960](https://github.com/Luminous-Dynamics/symtropy/pull/4960) → this occupancy integration.

## Purpose

The existing construction interop core owns task lifecycle and named-resource time reservations. This crate adds one narrow layer: conservative three-dimensional occupancy overlap over the time windows already declared by those reservations.

It asks whether two work envelopes, access regions, staging footprints or transit/swept outer bounds may overlap in the same site-frame volume at the same time.

## V0.1 semantics

- Exact floor, site epoch, design revision digest and target frame binding.
- Integer millimetre coordinates and fixed axis-aligned 3D outer bounds.
- Separate geometry-uncertainty and project-clearance margins.
- Half-open time windows and clock uncertainty taken from the existing #3644 `ReservationRequest`.
- Closed spatial-boundary policy: touching after expansion is a potential conflict.
- Idempotent repeated occupancy IDs; conflicting reuse is rejected.
- A non-mutating `conflicts_for` query returns the complete conservative conflict set for a candidate, so planners can compare multiple blockers before choosing a schedule repair.
- A deterministic 1 m uniform-grid broad phase narrows spatial candidate pairs. Expanded AABBs are indexed into every grid cell they touch; boundaries are inclusive for broad-phase purposes.
- Envelopes spanning more than 4,096 cells enter a global fallback bucket and are checked against every candidate, preventing large platforms/swept outer bounds from being silently dropped.
- The exact uncertainty/clearance-expanded AABB intersection remains authoritative; the grid cannot declare a conflict or clear a conflict by itself.
- `candidate_count_for` exposes the broad-phase candidate count for benchmarking; `conflicts_for` then applies exact space/time tests.
- The benchmark now separates sparse distributed workfaces from a dense cluster of 1,000 disjoint small envelopes packed into one 10 m cube. The dense workload is deliberately adversarial for uniform grids and quantifies local over-candidate behaviour. Direct spatial-index insertion is benchmarked separately from the full reservation owner; the latter currently scans existing reservations for capacity checks and is a separate scaling limit owned by #3644.
- A frozen grid-completeness unit test checks every pair in a 216-box corpus spanning negative coordinates, zero and 1 m cell boundaries, with two envelope sizes; if exact closed AABBs overlap/touch, the broad-phase cell sets must intersect or one side must use the global fallback. A full i64-coordinate-span fixture must take the fallback.
- `insert` remains fail-closed and rejects admission if any conflict exists.
- A combined admission helper preflights the spatial occupancy and conflict set, asks the #3644 resource owner to reserve, then commits a prepared spatial record without further typed validation failures.
- It does not clone either entire book per admission. A spatial conflict does not create a candidate resource booking; a resource rejection does not create a candidate occupancy. The resource owner may still perform its own lazy expiration housekeeping during a reserve attempt.
- The cross-layer integration test uses the #4960 accepted-floor ledger: an independently accepted task is linked to workface/hoist occupancy, a second overlapping envelope is rejected without leaving a reservation, and task-attributed plus shared labour/cost reconcile into the same floor report.
- Typed stale-context, malformed-geometry, overflow, clock and task-binding failures.
- Conflict records include both occupancy/task IDs, occupancy classes, expanded overlap bounds, overlap time and clock domain.

## V0.2 space-time candidate index experiment

The existing 1 m spatial grid now has a bounded temporal broad phase alongside it. Each occupancy is indexed at exactly one of four time resolutions (1 s, 10 s, 60 s, 10 min), using the already uncertainty-expanded half-open interval from its existing ReservationRequest. The existing reservation book remains the resource-capacity/expiry owner; this temporal index exists only to prune geometric occupancy candidates.

An occupancy is assigned to the finest level whose bucket span fits within 4,096 buckets. Longer spans use a global-time fallback. Query intervals search every resolution. If a query is too wide to enumerate at a given level, the query conservatively includes every occupancy assigned to that level. This makes the index a candidate superset: it can add false positives but cannot drop a true interval overlap.

The query uses an adaptive broad-phase strategy. It estimates candidate pressure from the spatial and temporal bucket metadata and starts from the smaller side:
- spatial-first: enumerate spatial candidates, then filter by exact expanded half-open interval overlap;
- time-first: enumerate temporal candidates, then probe the spatial grid for each ID.

This matters for a reused workface: a query for the next shift should not scan every historical occupancy that shares the same spatial cell. Exact AABB intersection and half-open time overlap remain the final conflict predicate. The old candidate_count_for API remains the spatial-only baseline; candidate_counts_for exposes spatial-only, temporal-only and intersected candidate counts.

New adversarial tests cover half-open adjacency, bucket boundaries, coarse-level intervals, global-time fallback, wide-query fallback, uncertainty-created overlap, combined spatial/time fallback and a deterministic 512-record / 1,024-query full-scan differential oracle. The oracle recomputes exact conflicts from the stored expanded bounds/intervals without using either index.

The scale harness now includes a repeated-workface distribution at 10/100/1,000/10,000 records. Every record has the same physical workface bounds but a disjoint 1-second time slot. It reports median build time, p50/p95/p99 query latency and spatial/temporal/combined candidate counts. A separate positive control queries an actually occupied interval and requires the true overlap to remain visible.

**Interpretation limits:** the candidate-count diagnostic materializes both index candidate sets for observability and is not the timed adaptive conflict path. The repeated-workface benchmark is a synthetic single-host software experiment; it does not model all real site distributions, reservation book throughput, durable storage or distributed concurrency. No physical or SOTA claim follows from it.

## Local commands

```sh
cargo fmt --manifest-path research/construction-spacetime-occupancy-v0-1/Cargo.toml -- --check
cargo test --manifest-path research/construction-spacetime-occupancy-v0-1/Cargo.toml --locked
cargo clippy --manifest-path research/construction-spacetime-occupancy-v0-1/Cargo.toml --locked --all-targets -- -D warnings
cargo bench --manifest-path research/construction-spacetime-occupancy-v0-1/Cargo.toml --locked --bench scale
```

The scale harness reports insertion and full-conflict-query timings for 10, 100, 1,000 and 10,000 synthetic sparse occupancy records. These are measurements of this implementation and runner only; they are not deployment latency thresholds or field performance claims.

## First synthetic scale baseline

Dedicated Rust 1.96 run [#43381165686](https://github.com/Luminous-Dynamics/symtropy/actions/runs/43381165686), Ubuntu runner, exact head `855db67fa91473dc4977b850ec42fcf565430f01`:

| Sparse occupancy records | Insert all records (ms) | One candidate query (ms) | Conflicts |
|---:|---:|---:|---:|
| 10 | 0.015 | 0.000 | 0 |
| 100 | 0.164 | 0.001 | 0 |
| 1,000 | 4.741 | 0.008 | 0 |
| 10,000 | 524.295 | 0.105 | 0 |

This is one synthetic run, not a repeated distribution, benchmark leaderboard, deployment SLO or physical-site result. The fixture uses spatially separated AABBs with overlapping time windows. The implementation checks each new insert against the existing occupancy map, so batch insertion exhibits quadratic aggregate work; one candidate conflict query is a linear scan. At 10,000 sparse records, the reference implementation remains sub-second on this CI runner, but it needs a spatial/time broad-phase accelerator before being considered for high-rate replanning or much larger site models.

Next performance work should add warm-up and repeated runs, sparse/dense/adversarial overlap distributions, p50/p95/p99, memory and candidate-pair counts, plus an independently validated indexed alternative. Do not improve latency by shrinking uncertainty/clearance margins or by omitting conflict pairs.

## Required caller contract

The caller must first use the existing #3644 `ReservationBook` to admit the corresponding time/resource reservation and pass the same `ReservationRequest` into this occupancy index. This crate checks its task, epoch, clock and interval fields but cannot prove a reservation was granted or cryptographically authenticate its issuer.

The geometry producer must supply a conservative outer bound in the exact frame. This crate does not parse IFC, transform coordinates, infer a frame, construct a crane/robot swept volume or validate that an outer bound contains the physical system.

## Claim boundary

A reported overlap is a potential conflict under the declared conservative bounds, not proof of physical collision or unsafe operation. No reported overlap is not proof of physical safety or clearance. Unknown, stale or unobserved geometry must be held/not evaluated by the upstream observation and authority owners; it is never treated as an empty envelope or a clear zone.

The combined admission helper is a single-process, typed-error-atomic in-memory composition pattern. It is not crash-atomic durable commit, a distributed transaction or multi-controller serialization protocol; allocation failure or process termination is outside its guarantee. The cross-layer test uses synthetic fixtures and structural receipt references; it is not physical inspection evidence or an authenticated acceptance authority.

The v0.1 AABB model is a deterministic broad-phase research reference only. The grid is an acceleration index; 1 m cell size and 4,096-cell fallback threshold are implementation parameters, not construction tolerances. False positives are allowed and must remain visible. A later narrow-phase geometry implementation needs an independent reference oracle and independently measured/as-built envelopes before any physical claim.



## Stacked dependency note (13 November 2026)

This branch includes the exact accepted-floor production crate snapshot from #4960 so the standalone push workflow can resolve all local Cargo paths. The stacked PR base remains the accepted-floor branch. The floor crate is a dependency, not a second implementation of floor accounting in this package.
