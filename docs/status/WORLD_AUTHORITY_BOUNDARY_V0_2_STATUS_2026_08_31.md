# World Authority Boundary v0.2 — Status

**Date:** 2026-08-31  
**Branch:** `world/cuf-v0.2-authority-boundaries`  
**Stacked on:** `core/cuf-v0.1-sim-contracts` / draft PR #61  
**Status:** authored, additive migration layer, **not yet locally qualified**

## Landed on the branch

- `symtropy-world` now depends on the dependency-light `symtropy-sim-contracts` crate;
- new `BodyCellIdentity` separates body/cell geometry from mutable domain claims;
- new generic `DerivedDomainView<T>` binds cached values to authority, scope, frame, representation, simulation time, and typed authoritative state digest;
- new terrain, hydrology, climate, and ecology cell summary read models;
- new `PlanetCellAuthorityView` composes those optional digest-bound views;
- legacy `PlanetCell` conversion is identity-only and deliberately refuses to promote bare biome/hydrology/climate/terrain values;
- mismatched domain-view scope is rejected;
- deterministic unit tests encode the migration boundary.

## Deliberately unchanged

- no legacy `PlanetCell` field has been removed;
- no Universal Matter authority semantics changed;
- no terrain/hydrology/ecology persistence moved into `symtropy-world`;
- no automatic resampling or cross-scope aggregation exists yet;
- no adaptive-fidelity scheduler exists yet.

## Qualification boundary

This connected authoring environment does not provide the repository's Rust/Nix toolchain, so no compile/test result is asserted here.

`symtropy-world` is excluded from the standalone workspace because it depends on private Mycelix workspace components. Qualify this branch in the full/private monorepo environment:

```bash
nix develop --command cargo fmt --all -- --check
nix develop --command cargo test -p symtropy-sim-contracts -p symtropy-world
nix develop --command cargo clippy -p symtropy-sim-contracts -p symtropy-world --all-targets -- -D warnings
```

Also run any normal private-workspace integration gates used for `symtropy-world` / Mycelix bridge changes.

## Stack rule

Do not merge v0.2 independently of v0.1. Its `symtropy-sim-contracts` dependency is intentional and the v0.1 qualification/lockfile state must be frozen first or together with the stack.

## Next tranche

`world/cuf-v0.3-adaptive-fidelity` should add a deterministic integer-scored representation-demand scheduler plus explicit `NeedsRefinement` causal backpressure. It must select representations only; it must never mutate domain truth.
