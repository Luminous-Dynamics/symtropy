# CUF v0.11 Native Authority Binding — Status

**Date:** 2026-09-01  
**Status:** design frozen; implementation blocked on qualification

## Frozen findings

- Universal Matter v4.8 exposes concrete Matter, sparse groundwater Hydrology, SurfaceWater, Ecosystem, deterministic weather, watershed, and hydrogeology APIs suitable for CUF adapters.
- SurfaceWater uses explicit physical units and must receive a V2 observation shape rather than the ambiguous legacy CUF hydrology summary.
- native surface domains use `(x,z)` geometry while Matter/groundwater use `(x,y,z)`; v0.11 therefore requires typed spatial bindings and explicit vertical sampling policies.
- `sample_surface_weather(...)` is deterministic forcing, not climate authority state.
- `sample_local_watershed(...)` is derived from Matter topography and therefore represents drainage potential, not observed water flow.
- procedural hydrogeology is Matter-backed derived state, not persistent HydrologyAuthority state.
- a resolved sparse-groundwater sample may depend on both Matter and Hydrology provenance.
- `HydrologyAuthority::digest()` currently omits the persisted active frontier; issue #76 tracks a complete causal authority digest before production groundwater integration.
- `apply_surface_runoff(...)` reports infiltration but only commits the runoff portion to `SurfaceWaterAuthority`; issue #77 tracks later conserved infiltration into groundwater.

## Preserved boundary

v0.11 remains read-only. It does not mutate Universal Matter or Basin and does not create replacement domain solvers.

## Required parent

Production implementation starts only from an exact combined Universal Matter v4.8 + CUF v0.10.1 tree that has passed the full local qualification/evidence gate from PR #74.

Tier A public CI for `symtropy-sim-contracts` is supplementary only; it does not qualify the private/full authority integration.

## Blocking issues

- #72 — v0.11 native Universal Matter observation adapters
- #76 — complete Hydrology authority causal digest
- #77 — later conserved surface infiltration into groundwater

## Next executable step

Run the guarded Universal Matter v4.8 replay on the combined PR #74 lineage and capture the exact PASS/FAIL evidence capsule. Do not implement production adapters before that result.