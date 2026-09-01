# Living Watershed Reference Policy v0.8 — Status — 2026-09-01

## State

Draft / not yet qualified in the full private monorepo toolchain.

## Branch

`world/cuf-v0.8-living-watershed-policy`

Stacked on `world/cuf-v0.7-environmental-ingest-receipts` / draft PR #67.

## Implemented

- deterministic `LivingWatershedPolicyV1`;
- stable namespaced policy digest binding rule order and thresholds;
- exact Terrain + Hydrology evidence requirement;
- optional Climate evidence with fail-closed planting behavior when absent;
- finite/range validation for current summary values;
- floodplain reroute proposal;
- temperate low-salinity riparian planting proposal;
- explicit observe/no-intervention outcome;
- production policy API with no mutable Basin access;
- receipt helper that accepts already-produced before/after Basin identities;
- end-to-end tests where the test harness/owner applies a proposed existing Basin intervention and then mints a v0.7 receipt;
- no-action receipt test proving ecological stability does not require fake mutation;
- canonical v0.8 reference-policy contract.

## Scientific scope

This is intentionally an architecture-validation reference policy, not a global watershed model. The current authority summaries do not yet expose precipitation, ET, hydraulic conductivity, sediment, channel geometry, nutrient state, or species suitability, so v1 does not pretend to reason from them.

The policy also does not use hard biome classification as intervention truth.

## Authority boundary

The production policy only proposes `Option<BasinIntervention>`. It never accepts `&mut BasinWorld`.

Execution remains an owning-domain decision. Before/after state identities and v0.7 receipts are produced around that external execution/evaluation.

## Qualification

Preferred full/private-monorepo gate:

`nix develop --command bash scripts/qualify-cuf-v0.8-stack.sh`

No compile, test, clippy, or Nix qualification result is asserted from the connected authoring environment.

## Next experiment

Build a multi-cell upstream/downstream Living Watershed experiment with an explicit connectivity contract. Demonstrate that one upstream hydrology change can deterministically alter downstream evidence/proposals/Basin consequences while preserving scope, time, authority, and receipt provenance.
