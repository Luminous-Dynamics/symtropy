# Watershed Causal Connectivity v0.9 — Status — 2026-09-01

## State

Draft / not yet qualified in the full private monorepo toolchain.

## Branch

`world/cuf-v0.9-watershed-connectivity`

Stacked on `world/cuf-v0.8-living-watershed-policy` / draft PR #68.

## Implemented

- Hydrology-owned directed `WatershedConnectionEvidence`;
- exact authority/reference-frame/simulation-time topology coherence;
- Hydrology-namespaced relation-digest validation;
- canonical edge sorting independent of arrival order;
- duplicate/self-edge rejection;
- v1 acyclic one-way drainage validation;
- stable typed `symtropy.watershed.topology.v1` digest;
- deterministic downstream causal reachability;
- minimum graph-hop distance only, explicitly not physical travel time or flux;
- tests for arrival-order independence, three-cell reachability, converging paths, duplicate edges, cycles, and relation namespace;
- canonical v0.9 contract.

## Authority boundary

The topology can propagate causal relevance only. It contains no API or state field that can synthesize downstream water level, discharge, groundwater, salinity, sediment, or travel time.

Fresh downstream physical state must still come from Hydrology authority.

## Qualification

Preferred full/private-monorepo gate:

`nix develop --command bash scripts/qualify-cuf-v0.9-stack.sh`

No compile, test, clippy, or Nix qualification result is asserted from the connected authoring environment.

## Next proof

Add an integration-only three-cell proof:

- A → B → C topology;
- upstream disturbance evidence at A;
- C becomes causally relevant through topology;
- a separately supplied Hydrology-authority observation changes C from benign to floodplain conditions;
- Living Watershed v1 changes its proposal only after that real downstream observation arrives;
- Basin owner executes the proposal;
- v0.7 receipt binds the downstream evidence and includes upstream/topology evidence as causal parents.

This should remain a proof harness, not a world-owned hydrology solver.
