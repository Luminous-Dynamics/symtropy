# Universal Matter v4.8 Stateful Resource Audit

**Date:** 2026-09-01  
**Status:** static source audit; runtime/persistence qualification required  
**Companion:** `UNIVERSAL_MATTER_V4_8_AUTHORITY_IDENTITY_AUDIT_2026_09_01.md`

## Purpose

The authority audit cannot rely on names alone. Bevy `Resource` values that are not called `*Authority` can still contain:

- canonical state;
- continuation-significant integration residuals;
- derived indexes;
- cache invalidation queues;
- entity/runtime bindings;
- presentation state.

This pass classifies those categories so persistence/replay does not silently depend on naming conventions.

## Resource inventory

The retained v4.8 patch defines 34 Bevy `Resource` structs in the reviewed Terrain additions. Twenty-two are named authorities and are covered by the companion audit.

The remaining resources fall into four categories:

1. canonical/support state;
2. continuation-significant integration state;
3. derived/rebuildable runtime/index state;
4. configuration/presentation/statistics.

## Canonical/support state: SupportRegistry

`SupportRegistry` is not named `Authority`, but it is persistent domain state that affects structural support and Hydrology drainage behavior.

Owned canonical fields:

- `next_id`;
- support elements.

It has:

- `SupportRegistrySnapshot` including `next_id` and elements;
- validation on restore;
- `SupportStateDigest` binding `next_id`, IDs, anchors, type/material, influence radius, nominal capacity, drainage capacity, and condition.

Static classification: **canonical state with an explicit complete-looking digest/snapshot contract**.

No continuation-significant omitted field was identified in this pass.

This demonstrates why persistence review should search by semantics and `Resource`, not only by names ending in `Authority`.

## Continuation-significant integration state: LandscapeEnvironmentAccumulator

`LandscapeEnvironmentAccumulator` stores:

- `BTreeMap<PromotedLandscapeObjectId, [f32; 2]>` residuals.

The two residual channels accumulate fractional environmental exposure for waterlogging and decay.

Each frame:

1. water/terrain exposure adds a fractional amount to the residual;
2. the integral floor is extracted;
3. the remaining fraction stays in the accumulator;
4. the integer delta mutates `LandscapePhysicalStateAuthority`.

Therefore the residual controls when future canonical waterlogging/decay state changes.

The accumulator currently has no snapshot/digest in the reviewed source.

Static classification: **continuation-significant integration state**.

Resetting it to zero on save/reload can alter later canonical state even if `LandscapePhysicalStateAuthority` itself was restored exactly.

Tracking: issue #81.

Preferred repair direction:

- canonical fixed-point residuals rather than floating continuation state;
- either move them into the physical-state authority or give the accumulator an explicit snapshot + continuation digest;
- remove residuals when the associated promoted object is no longer live;
- test interrupted versus uninterrupted evolution.

## Derived indexes: LandscapePromotionRegistry

`LandscapePromotionRegistry` owns:

- canonical promotion records;
- `tile_index`;
- `object_id_index`.

The source explicitly documents the two indexes as derived. Snapshots persist canonical records only and `from_snapshot(...)` deterministically rebuilds both indexes while validating object IDs, duplicate keys, and duplicate object IDs.

Static classification: **canonical records + derived indexes**.

The indexes should not become separate persistence truth.

Recommended qualification:

- snapshot/reload preserves registry digest and all lookup behavior;
- shuffled snapshot input, if accepted, reconstructs the same canonical indexes/digest;
- duplicate/invalid inputs fail closed.

## Cache-invalidation continuation: DirtyMatterRegions

`DirtyMatterRegions` stores a canonical-order `BTreeSet<EarthChunkCoord>` of chunks requiring derived rebuild.

It is drained by `take_sorted()` and exists to schedule reconstruction of derived Terrain consumers rather than to represent physical matter.

Static classification: **derived cache-invalidation work queue**, not physical authority state.

It should not be folded into `MatterStateDigest`.

However, losing it across lifecycle transitions is safe only if the load/restart path guarantees an equivalent conservative reconstruction strategy, for example:

- all loaded derived chunks are rebuilt from Matter authority state;
- or all relevant chunks are marked dirty after authority restore;
- or pending dirty work is explicitly persisted as runtime continuation state.

Qualification should prove that restoring Matter cannot leave stale collision/render/derived state merely because the pre-save dirty queue was discarded.

## Derived entity bindings

The following reviewed resources are mappings between canonical IDs and live Bevy entities/runtime instances:

- `MatterFragmentRuntimeBindings`;
- `LandscapePromotionRuntimeBindings`.

Static classification: **runtime bindings / rebuildable indexes**.

They must be reconstructed from canonical authority/registry state after load rather than persisted as entity identity truth.

Entity IDs are process/runtime identities and must not enter domain-state digests.

## Presentation/runtime resources

The following reviewed resources are presentation/runtime state rather than canonical physical state:

- `HydrodynamicWakeRuntime`;
- `ProductionWaterRenderRuntime`;
- `TerrainGpuMaterialRuntime`;
- `TerrainLodSeamRuntime`.

Their reconstruction may affect visual continuity but must not change domain authority state.

Tests should enforce one-way dependency:

`authority state -> runtime/presentation reconstruction`

not:

`runtime entity/material identity -> authority truth`.

## Configuration resources

Reviewed configuration resources include:

- `HydrodynamicRuntimeConfig`;
- `LandscapeAssetRuntimeConfig`;
- `TerrainGpuMaterialConfig`;
- `TerrainRuntimeConfig`;
- `TerrainGenesisProfile`.

These require a separate policy decision depending on semantics:

- presentation-only configuration need not be authority state;
- simulation-affecting configuration must be frozen/bound in model or continuation provenance;
- generation profiles that determine procedural Matter must already be represented in the authoritative state/generator identity used by Matter.

A runtime config must not silently alter deterministic physics while remaining absent from replay evidence.

## Statistics resources

Reviewed statistics resources include:

- `TerrainCollisionConsumerStats`;
- `TerrainLodSeamStats`;
- `TerrainRuntimeBridgeStats`.

Static classification: **diagnostic telemetry**, unless future code begins using a statistic as a decision input.

The moment a counter affects scheduling/physics/policy, it ceases to be merely diagnostic and must be reclassified.

## General invariant

Persistence/replay review should scan at least:

- `Resource`;
- `Authority`;
- `Registry`;
- `Accumulator`;
- `Queue`;
- `Chronicle`;
- `Session`;
- `Runtime`;
- `State`;
- checkpoint/snapshot structures.

Names are hints, not semantics.

For every stateful object ask:

1. Can this value alter future authoritative physical state?
2. Is it already represented in an authority digest?
3. Is it deterministically reconstructible from canonical state?
4. Does save/reload reset it?
5. Is it a bounded integration remainder?
6. Is it merely presentation or diagnostics?
7. Does its configuration alter simulation results?

## Current actionable result

The newly confirmed non-Authority continuation issue is `LandscapeEnvironmentAccumulator` (#81).

No separate persistence issue is currently required for `SupportRegistry`, because its reviewed snapshot/digest includes the canonical fields that affect support/drainage semantics.

`LandscapePromotionRegistry` indexes and runtime entity bindings are intentionally derived.

`DirtyMatterRegions` remains a derived invalidation queue but needs lifecycle/reconstruction tests to prove discarding the queue cannot leave stale derived world state after reload.

## Artifact integrity

As with the authority audit, do not edit the retained v4.8 patch artifact to address these findings.

Preserve pristine authored replay first, then layer focused repairs and requalify the cumulative lineage.