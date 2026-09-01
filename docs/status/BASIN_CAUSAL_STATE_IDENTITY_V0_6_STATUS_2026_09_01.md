# Basin Causal State Identity v0.6 — Status — 2026-09-01

## State

Draft / not yet qualified in the full private monorepo toolchain.

## Branch

`domain/cuf-v0.6-basin-state-identity`

Stacked on `core/cuf-v0.5-observation-evidence` / draft PR #65.

## Implemented

- read-only `BasinCausalStateIdentity` extension trait;
- typed `symtropy.basin.state.v1` SHA-256 digest;
- canonical y-major/x-minor cell traversal;
- all current `BasinCell` stored fields;
- all fourteen current LifeSim field layers with frozen explicit order;
- metabolic flux;
- trophic memory;
- viability;
- signals and signal-reader sequences;
- civic claims, evidence, and opposition sequences;
- fixed-width integer encoding;
- canonical `f32` encoding for signed zero and NaN payloads;
- explicit stable enum codes rather than Rust discriminants;
- field-grid shape rejection;
- deterministic identity tests and future-state sensitivity tests;
- canonical v0.6 contract.

## Authority note

The implementation lives downstream in `symtropy-world` because the connected GitHub authoring surface cannot safely splice a module declaration into the large single-file `symtropy-basin` source without wholesale replacement. `symtropy-basin` itself remains unchanged and remains the authority.

The trait reads only public Basin state. It does not mutate, sanitize, repair, interpolate, or redefine Basin state.

This placement can later be moved into the Basin package without changing the v1 digest stream, provided byte-for-byte digest parity is demonstrated.

## Qualification

Preferred full/private-monorepo gate:

`nix develop --command bash scripts/qualify-cuf-v0.6-stack.sh`

No compile, test, clippy, or Nix qualification result is asserted from the connected authoring environment.

## Next tranche

After qualification, implement an environmental ingest receipt binding exact-time observation evidence to prior/resulting Basin causal-state digests and an explicit versioned transformation policy. Do not mutate Basin from generic world code merely to make the receipt convenient.
