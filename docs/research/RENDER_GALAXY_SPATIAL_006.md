# GALAXY-SPATIAL-006 — Deterministic galactic catalog partitioning

**Status:** Implemented as a bounded Rust primitive; exact-head validation and performance are pending.

## Goal

Give the renderer a stable spatial partition key for a Milky-Way-sized catalog and a deterministic way to admit tile residency within explicit memory/record budgets. The index composes with Symtropy's existing scale-continuum, time/frame, provenance, and residency authorities; it does not replace them.

## Coordinate/index contract

- A fixed cubic index domain spans ±50,000 parsecs on each Cartesian axis.
- Inputs are validated and quantized to integer microparsecs. One microparsec is approximately 0.206 AU.
- A 36-level octree uses a 3-bit-per-level Morton prefix (108 bits at maximum depth), stored in a 128-bit integer.
- The fixed cube boundary is inclusive. Non-finite and out-of-domain coordinates reject; nothing silently wraps or clamps.
- Cell identity is geometric only. Each catalog snapshot must separately preserve coordinate-frame identity, epoch/time scale, upstream transform receipt, source identifier, uncertainty/covariance, selection criteria, license, and ingest revision.
- The index accepts already transformed Cartesian coordinates. It does not convert angular measurements into 3D positions, estimate missing distances, model extinction, or infer stellar trajectories.

## Deterministic budget admission

The Rust planner orders candidates by pinned status, priority descending, then stable cell identity. It obeys independent maximum tile, record and resident-byte budgets. A pinned set that cannot fit fails closed; optional content is deferred with counts and admitted resource totals. Duplicate tile identities reject.

This is a host-side admission policy, not a background loader, cache, GPU culler, or proof of full spatial coverage. The host remains responsible for conservative tile bounds, frustum traversal, cancellation, asset verification, ancestor/descendant transition policy and source freshness.

## Benchmarks and claim ceiling

The frozen GALAXY-SCALE-01 fixture defines boundary, determinism, budget-admission and synthetic-catalog workloads. It distinguishes proposed performance goals from measurements. Exact source revision, fixture digest, build profile, CPU, OS and sample seed should accompany timing receipts.

A useful next target is repeatable million-source synthetic traversal and budget-planning runs, followed by a public licensed data sample before scaling ingestion toward billion-source catalogs. The Gaia DR3 count is a catalog-size reference, not a claim that Gaia is a complete census of the Milky Way. No speed or completeness advantage over Gaia Sky, HiPS clients, OGC 3D Tiles implementations, or commercial engines is claimed.

## Known gaps

- No on-disk B-tree/LSM manifest, mmap tile reader, network streaming, GPU indirect draw, density-preserving star shader or catalogue ingestion pipeline.
- No astrometry/frame integration and no propagation of covariance over time.
- No clustered multi-star aggregate rendering or catalog-derived Milky Way morphology.
- No measured performance results or hardware comparison.
