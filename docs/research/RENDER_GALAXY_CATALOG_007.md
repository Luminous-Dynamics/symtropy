# GALAXY-CATALOG-007 — Evidence-separated hierarchical tile summaries

**Status:** Rust contract added; build and performance validation pending.

## Model

Every summary is bound to five non-zero digests: source catalog snapshot, frame/epoch transformation, source selection, payload schema, and license/attribution context. The evidence layer is also part of context identity:

- Observed catalog records.
- Inferred population-model records.
- Synthetic visual-presentation records.

The layers cannot be merged. The code counts data-product payload records; it does not reinterpret those rows as the number of physical stars.

## Parent/child accounting

An aggregate accepts only direct octree children, once per octant. Record totals use checked integer addition and remain unchanged on failed merges. Coverage is complete only when all eight direct children each certify completeness within the same declared selection; otherwise partial/unknown status is preserved. A zero-row complete tile is not the same as a tile whose content is unknown.

The merge checks spatial child identity and lineage compatibility. It does not independently deduplicate billions of source IDs; the immutable ingestion/partition manifest must guarantee source uniqueness and disjoint spatial assignment before leaf summaries are admitted.

## Scientific boundary

A galactocentric rendering coordinate is not a direct measurement. Catalog input must already bind the transform, reference frame, epoch/time scale, selection function and uncertainty schema. An inferred population field and a synthetic presentation layer remain separately queryable and separately visible; neither fills an observational hole as though it were measured. This initial summary model carries payload row accounting, not photometric flux or stellar-population estimates. Those require explicit units/model metadata and their own conservation contract.

## Benchmarks

The GALAXY-CATALOG-01 fixture covers full child rollup, missing child, empty versus unknown, duplicate child, catalog/frame lineage mismatch, evidence-layer mismatch and overflow atomicity. Next stages add flux-conserving aggregates with explicit band/unit schemas, covariance summaries, on-disk manifests, and real source ingestion.

No performance result is claimed until exact-head CI and a checked-in release-mode benchmark receipt exist.
