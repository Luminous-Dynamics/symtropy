# Pair Lifecycle Ledger v0.1

A dependency-free Rust research oracle for deterministic accounting of collision interactions across a physics pipeline.

## What it establishes

For a frozen set of canonical collider pairs, every stage input must receive exactly one disposition. Forwarded identities and deterministic child identities become the exact input set of the next stage. A missing, duplicate, orphaned, cross-pair or stale-scope identity is rejected.

The initial stage sequence is eligibility, temporal bounds, broadphase, narrowphase, candidate generation, manifold reduction, solver mapping and solve outcome.

Unsupported or invalid temporal bounds cannot be recorded as successful prunes: they must be forwarded with an explicit conservative-fallback reason. Terminal reason codes are restricted by stage.

## Run

```sh
cargo fmt --manifest-path research/pair-lifecycle-ledger/v0.1/Cargo.toml -- --check
cargo test --manifest-path research/pair-lifecycle-ledger/v0.1/Cargo.toml --locked
cargo clippy --manifest-path research/pair-lifecycle-ledger/v0.1/Cargo.toml --locked --all-targets -- -D warnings
```

## Identity and receipt

Pair identity includes world revision, step index, body handle plus generation, collider ID and compound subshape path. Pair endpoints are canonicalized so A/B swaps preserve identity. Child lineage IDs extend a parent path by exactly one deterministic ordinal.

Receipts provide per-stage input/forward/spawn/terminal counts, stage-scoped reason histograms and a deterministic 64-bit FNV-1a fingerprint. The fingerprint is not cryptographic; a signed external evidence envelope must use a collision-resistant digest.

## Claim boundary

This crate validates accounting and lineage, not collision correctness. It does not independently prove temporal enclosures, broadphase recall, narrowphase geometry, wrench preservation, solver convergence or task safety. Those remain owned by the independent oracles and qualification programmes in Symtropy. It does not yet model many-to-one merge ancestry; that requires an explicit aggregate lineage node. Production integration and instrumentation overhead are not measured.

License: AGPL-3.0-or-later. See the repository commercial licensing notice for alternative licensing.
