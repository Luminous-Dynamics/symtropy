# Causal Simulation Contracts v0.1 — Status

**Date:** 2026-08-31  
**Branch:** `core/cuf-v0.1-sim-contracts`  
**Base:** `14f17d194dbb90db0f9b8a3ca3f2133ba1f1d7fd` (`world/inhabited-epistemic-v0.2-lifecycle`)  
**Status:** authored, workspace-wired, **not yet locally qualified**

## Landed on the branch

- new dependency-light `symtropy-sim-contracts` core crate;
- portable `AuthorityId`, `ScopeId`, `ReferenceFrameId`, and `RepresentationId` identities;
- signed wide-range `SimInstant` with canonical nanoseconds;
- domain-separated `TypedDigest32` evidence;
- domain-owned conservation/equivalence proof binding;
- serializer-independent `RepresentationTransferReceipt` digest;
- deterministic unit tests for identity validation, negative time, digest domain separation, same-representation rejection, causal-parent ordering, and JSON round-trip identity;
- canonical Causal Simulation Contract v0.1 document;
- workspace membership registration.

## Deliberately not included

- no Terrain, Hydrology, Basin, Ecology, Bevy, Rapier, Mycelix, Symthaea, networking, or persistence dependency;
- no adaptive-fidelity scheduler yet;
- no causal-backpressure policy yet;
- no migration of `symtropy-world` state yet;
- no claim that a generic receipt can verify a domain's physical conservation law by itself.

## Qualification required

Run from the repository root in the intended Nix/Rust environment:

```bash
nix develop --command cargo fmt --all -- --check
nix develop --command cargo test -p symtropy-sim-contracts
nix develop --command cargo clippy -p symtropy-sim-contracts --all-targets -- -D warnings
nix develop --command cargo check --workspace --all-targets
nix develop --command bash scripts/check-workspace.sh
nix develop --command bash scripts/check-licenses.sh
```

Because the crate is a new workspace package, the local Cargo invocation may regenerate `Cargo.lock`. If it changes, inspect and commit that lockfile delta before qualification is frozen.

## Qualification rule

Do not mark this tranche qualified merely because the source looks correct. Qualification requires the commands above to pass against the exact branch head and the resulting tree/lock state to be recorded.

## Next tranche after qualification

`world/cuf-v0.2-authority-boundaries` should consume these contracts in `symtropy-world` and convert duplicated biome/hydrology fields into digest-bound derived views without changing Universal Matter authority semantics.
