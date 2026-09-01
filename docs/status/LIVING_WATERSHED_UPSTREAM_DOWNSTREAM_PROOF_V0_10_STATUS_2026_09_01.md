# Living Watershed Upstream/Downstream Proof v0.10 — Status — 2026-09-01

## State

Draft / not yet qualified.

## Branch

`world/cuf-v0.10-upstream-downstream-proof`

## Base

`world/cuf-v0.9-watershed-connectivity`

## Authored proof

- three-cell one-way watershed A → B → C;
- deterministic downstream reachability;
- upstream Hydrology disturbance at A;
- unchanged benign C observation after relevance propagation;
- explicit proof that connectivity alone does not change the C policy result;
- fresh later Hydrology-authority observation at C;
- exact-time Terrain/Hydrology/Climate evidence at C;
- deterministic Living Watershed floodplain-reroute proposal;
- Basin-owner before/after causal-state identities;
- Basin-owner intervention execution;
- v0.7 environmental-ingest receipt;
- topology and upstream disturbance bound as ordered causal parents;
- repeated full-chain receipt digest determinism.

## Not claimed

This tranche does not claim to propagate water state, estimate routing time, model attenuation, or solve watershed hydrodynamics.

No Rust/Nix compile, test, clippy, or formatting result is asserted from the connected authoring environment.

## Qualification command

`nix develop --command bash scripts/qualify-cuf-v0.10-stack.sh`

## Exit gate

v0.10 may be called qualified only when the full/private workspace passes the v0.10 gate with the exact qualified head recorded.

## Next technical milestone

Move from an integration fixture to a Hydrology-owned response surface that can publish downstream observations from explicit authoritative inputs. World orchestration should continue to consume those observations rather than computing water state itself.
