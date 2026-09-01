# Observation Evidence v0.5 — Status

**Date:** 2026-09-01  
**Branch:** `core/cuf-v0.5-observation-evidence`  
**Stacked on:** `world/cuf-v0.4-representation-residency` / draft PR #64  
**Status:** authored, portable provenance + exact environmental evidence layer, **not yet locally qualified**

## Landed on the branch

- dependency-light `ObservationEvidence` in `symtropy-sim-contracts`;
- serializer-independent observation evidence digest;
- JSON round-trip and identity-sensitivity tests;
- `DerivedDomainView<T>::observation_evidence()` bridge;
- provenance-only `EnvironmentalEvidenceBundle`;
- exact scope/reference-frame/simulation-time coherence checks;
- rejection of identity-only environmental bundles;
- rejection of asynchronous observations as a single exact snapshot;
- deterministic source-digest ordering;
- canonical Observation Evidence Contract v0.5;
- fail-closed `scripts/qualify-cuf-v0.5-stack.sh`.

## Authority boundary

No Terrain, Fluid, Basin, LifeSim, Climate, Ecology, or settlement value is moved into `symtropy-sim-contracts`.

`ObservationEvidence` identifies source provenance only. `EnvironmentalEvidenceBundle` composes provenance only. Existing domain owners remain authoritative.

## Dependency boundary

The portable evidence primitive lives below `symtropy-world`, so Basin and other domains can eventually consume provenance without depending on world orchestration.

The v0.5 world bridge merely converts an already digest-bound `DerivedDomainView<T>` into the same core evidence envelope.

## Temporal boundary

The exact environmental bundle refuses to combine observations from different simulation instants. No default staleness window, interpolation rule, or extrapolation policy is invented by the common layer.

## Qualification boundary

The connected authoring environment does not provide the repository Rust/Nix toolchain, so no compile/test result is asserted here.

Preferred full/private-monorepo gate:

```bash
nix develop --command bash scripts/qualify-cuf-v0.5-stack.sh
```

The gate fails closed if the private `../mycelix-multiworld-sim` sibling required by `symtropy-world` is absent.

## Recommended next tranche

Do not add a generic environmental transformation receipt yet. First give `symtropy-basin` a canonical deterministic state digest. Then implement Basin environmental ingest as a real causal transformation whose receipt binds source `ObservationEvidence`, prior Basin digest, resulting Basin digest, and transformation-policy evidence.
