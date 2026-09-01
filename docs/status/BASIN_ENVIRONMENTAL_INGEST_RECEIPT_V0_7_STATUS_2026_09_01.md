# Basin Environmental Ingest Receipt v0.7 — Status — 2026-09-01

## State

Draft / not yet qualified in the full private monorepo toolchain.

## Branch

`world/cuf-v0.7-environmental-ingest-receipts`

Stacked on `domain/cuf-v0.6-basin-state-identity` / draft PR #66.

## Implemented

- non-mutating `BasinEnvironmentalIngestReceipt`;
- explicit Basin authority/scope/reference-frame/time binding;
- role-tagged environmental source observations;
- frozen Terrain → Hydrology → Climate → Ecology role order;
- rejection of duplicate/non-canonical role order;
- exact source scope/frame/time validation;
- strict `symtropy.basin.state.v1` prior/resulting digest validation;
- explicit Basin environmental policy digest binding;
- causal-parent binding and shared maximum enforcement;
- deterministic serializer-independent receipt digest;
- explicit `StateChanged` vs `StateUnchanged` outcome derived from before/after identity;
- tests showing observation, policy, Basin state domain, role ordering, and causal-parent ordering are receipt-significant;
- canonical v0.7 contract.

## Authority boundary

The receipt has no Basin mutation API. It records evidence only after an owning domain has produced prior/resulting state identities around an evaluation/transformation.

No authority-to-role registry is introduced in this tranche. Roles are explicit identity-bearing claims; later capability/authority certification may decide which authority IDs are authorized to assert each role.

## Qualification

Preferred full/private-monorepo gate:

`nix develop --command bash scripts/qualify-cuf-v0.7-stack.sh`

No compile, test, clippy, or Nix qualification result is asserted from the connected authoring environment.

## Next tranche

Implement a deliberately narrow deterministic `symtropy.basin.environment-policy.living-watershed.v1` reference policy and a Living Watershed vertical slice. Prove that a small set of physically meaningful environmental inputs can produce reproducible Basin consequences and v0.7 receipts without creating a second state authority.
