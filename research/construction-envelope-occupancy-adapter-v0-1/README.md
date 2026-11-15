# Construction Envelope → Occupancy Adapter v0.1

**Status:** pure typed software projection; synthetic fixture tests only.

## Ownership boundary

This crate composes two existing owners:

- `construction-motion-envelope` creates time-indexed physical, stopping and coordination outer bounds and retains context/margin/evidence lineage.
- `construction-spacetime-occupancy-core` owns spatial indexing, broad-phase candidate generation, exact AABB/time overlap evaluation and composition with the existing reservation owner.
- `construction-robot-interop-core` owns the ReservationRequest contract.

This adapter owns none of those subsystems. It only validates the binding between a source report slice and caller-supplied site/reservation identities.

## Critical mapping rule: expand exactly once

The producer's `coordination_outer` already includes:
- body radius and nominal swept geometry;
- path deviation, localization, tracking, model and numerical bounds;
- stopping/recovery reachability;
- project policy clearance.

The adapter maps that final AABB into `SpatialOccupancy.bounds` and sets both occupancy-level uncertainty and clearance margins to `[0, 0, 0]`. A second expansion would silently widen the envelope and distort conflict/tightness benchmarks.

The full source report canonical bytes, source context, primitive/slice identity, centerline, radius, all three outer bounds, seven margin channels and seven evidence references are preserved in `EnvelopeLineage`.

Canonical bytes are retained verbatim; this crate does not compute or authenticate a cryptographic digest. The consuming evidence owner may digest those bytes using the established receipt/digest path.

## Projection contract

`project_slice` requires the slice to be a member of the supplied report and validates:
- task ID against the caller's reservation;
- site epoch across report, SiteBinding and ReservationRequest;
- design revision and target frame against SiteBinding;
- clock domain against ReservationRequest;
- exact slice and reservation half-open interval equality;
- primitive manifest membership and unique expected primitive IDs;
- non-empty source evidence references and caller identities;
- digest shape and containment of physical ⊆ stopping ⊆ coordination bounds.

The caller supplies floor ID, occupancy ID, OccupancyKind and ReservationRequest. The adapter does not invent them.

## Non-goals and claim ceiling

Projection does not mutate an occupancy book or reserve a resource. It does not grant admission, motion authority, a lift permit or safety authorization. An occupancy conflict remains a conservative broad-phase coordination conflict, not a physical collision verdict.

No physical swept-volume truth, localization/stopping accuracy, collision freedom, construction safety, field productivity or standards-conformity claim is made.

## Commands

- `cargo metadata --manifest-path research/construction-envelope-occupancy-adapter-v0-1/Cargo.toml --locked --format-version 1`
- `cargo fmt --manifest-path research/construction-envelope-occupancy-adapter-v0-1/Cargo.toml -- --check`
- `cargo test --manifest-path research/construction-envelope-occupancy-adapter-v0-1/Cargo.toml --locked`
- `cargo clippy --manifest-path research/construction-envelope-occupancy-adapter-v0-1/Cargo.toml --locked --all-targets -- -D warnings`
- `cargo bench --manifest-path research/construction-envelope-occupancy-adapter-v0-1/Cargo.toml --bench projection`

The benchmark reports projection, read-only occupancy candidate and exact conflict query latencies separately. It is not a deployed robot or construction productivity benchmark.

## Benchmark interpretation

Batch-insert figures are amortized per record over a fresh index build; query figures are repeated read-only calls against the populated synthetic index. The current fixture deliberately spaces entries apart to isolate indexing behavior. A separate dense/adversarial workload is required before generalizing query performance. Allocation counts are not currently instrumented.
