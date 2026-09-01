# CUF Deterministic Forcing Evidence v0.10.1 — Status

**Date:** 2026-09-01  
**Status:** Authored; local Rust/Nix qualification required

## Landed

- `ForcingModelId` as a distinct identity class from `AuthorityId`;
- `DeterministicForcingEvidence` with scope, frame, simulation instant, frozen model-contract digest, deterministic input digest, and exact output digest;
- serializer-independent, domain-separated evidence digest;
- tests for deterministic identity, input sensitivity, output sensitivity, JSON round-trip, and invalid model IDs;
- root re-exports from `symtropy-sim-contracts`;
- canonical observation-vs-forcing authority boundary.

## Preserved invariants

- existing `ObservationEvidence` fields and digest algorithm are unchanged;
- forcing evidence carries no `AuthorityId`;
- forcing evidence owns no world state;
- forcing evaluation cannot itself assert a Terrain/Hydrology/SurfaceWater/Ecology change;
- only an owning authority can convert forcing into authoritative state through a domain transition.

## Motivation discovered during Universal Matter v4.8 audit

The retained v4.8 Terrain lineage contains a deterministic gameplay-scale `weather_field`, but that module explicitly describes itself as forcing rather than persistent authority.

This tranche gives CUF a first-class way to preserve the model/input/output provenance of that forcing without mislabelling it as Climate observation evidence.

The same mechanism is intentionally suitable for later stellar irradiance, orbital ephemeris, tide, radiation, and other deterministic external drivers.

## Qualification still required

Run from the full/private workspace:

```bash
nix develop --command bash scripts/qualify-cuf-v0.10.1-forcing.sh
```

No compile/test/clippy result is asserted by this authored status document.

## Relationship to Universal Matter v4.8 / CUF v0.11

This tranche is independent of replaying Universal Matter v4.8 and may be reviewed separately.

After a v4.8 + CUF head is qualified, CUF v0.11 should use:

- `ObservationEvidence` for native authority-backed Matter/Hydrology/SurfaceWater/Ecosystem state;
- `DeterministicForcingEvidence` for deterministic weather or other non-authoritative forcing;
- explicit domain-owned transition receipts when forcing changes authoritative state.
