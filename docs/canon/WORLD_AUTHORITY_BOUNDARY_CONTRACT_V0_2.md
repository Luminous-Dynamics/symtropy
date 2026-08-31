# World Authority Boundary Contract v0.2

**Status:** canonical migration contract  
**Date:** 2026-08-31  
**Scope:** `symtropy-world`, planetary cell views, domain ownership, multiscale orchestration

## 1. Governing rule

`symtropy-world` coordinates scopes, simulation scales, caches, and presentation. It is not permitted to become a competing source of truth for terrain, hydrology, climate, ecology, settlement, or other domain-owned state.

A cached value is trustworthy only when its provenance is explicit enough to identify the authority and authoritative state from which it was derived.

## 2. Legacy `PlanetCell` boundary

The existing `PlanetCell` contains useful body-cell identity and several historical convenience fields such as elevation, biome, hydrology, temperature, atmosphere, resources, and history tags.

During migration these fields remain available for compatibility, but their mere presence does not prove authority.

`PlanetCellAuthorityView::identity_only_from_legacy` therefore imports only:

- body/cell identity;
- grid identity and resolution;
- center latitude/longitude;
- nominal cell area.

It deliberately does **not** promote legacy terrain, hydrology, climate, or biome values into domain claims.

## 3. Derived-domain view contract

A `DerivedDomainView<T>` binds one cached value to:

1. `AuthorityId`;
2. `ScopeId`;
3. `ReferenceFrameId`;
4. `RepresentationId`;
5. `SimInstant`;
6. typed authoritative state digest;
7. the derived value.

A derived view remains a read model. The digest is provenance, not ownership transfer.

## 4. Exact scope attachment

A domain view may attach to a `PlanetCellAuthorityView` only when its scope exactly equals the canonical body-cell scope derived from that cell identity.

The world layer must reject cross-cell, cross-body, or otherwise mismatched claims rather than silently resampling or relabeling them.

Explicit aggregation/resampling belongs in a later representation-transfer operation with its own evidence.

## 5. Biomes are derived descriptions

`BiomeKind` remains available as a compatibility-facing summary, but a bare biome tag is not authoritative ecology.

A biome classification becomes a valid world-layer claim only when it arrives inside a digest-bound ecology view. Later ecology work may replace or enrich the classifier without changing this authority boundary.

## 6. No new persistence authority

`PlanetCellAuthorityView` and `DerivedDomainView` are caches/read models. They do not create a new save ledger.

Authoritative domain checkpoints remain with their owning systems. `symtropy-persistence` remains the save/journal layer, and Reality Ledger remains the lifecycle/evidence layer around authoritative artifacts.

## 7. Compatibility policy

v0.2 is intentionally additive:

- legacy `PlanetCell` fields are not deleted;
- existing callers need not migrate atomically;
- new authority-aware consumers should prefer digest-bound views;
- legacy fields should be deprecated only after their authoritative producers and consumers have migrated;
- no legacy value may be silently minted into authoritative evidence during that migration.

## 8. Acceptance gates

v0.2 is qualified when deterministic tests prove:

1. legacy conversion creates identity only and no domain claims;
2. a matching scope can attach a digest-bound domain view;
3. a mismatched body/cell scope is rejected;
4. a biome classification requires digest-bound ecology provenance to become a world-layer claim;
5. `symtropy-world` compiles against `symtropy-sim-contracts` in the full/private workspace;
6. formatting and `clippy -D warnings` pass.

## 9. Next tranche

After qualification, v0.3 may add adaptive-fidelity demand selection and explicit causal backpressure. The scheduler may choose representations; it must never mutate domain truth or bypass these authority boundaries.
