# World Continuation Golden Vectors v0.1

**Status:** canonical test vectors for the dependency-light v0.1 implementation  
**Contract:** `WORLD_CONTINUATION_MANIFEST_CONTRACT_V0_1` / `SIMULATION_TIMEBASE_CONTINUATION_CONTRACT_V0_1`

The fixture serializer is not canonical. The digests below are over the explicit binary field encoding defined by the Rust contract and frozen documentation.

## Vector T1 — fixed timebase

Semantic input:

- schema: `1`;
- timebase ID: `gameplay.fixed.test.v1`;
- epoch digest: SHA-256 typed digest of `world-genesis` in domain `symtropy.test.genesis.v1`, schema `1`;
- origin tick: `0`;
- origin `SimInstant`: `0s + 0ns`;
- step: `50_000_000ns`.

Expected typed digest domain:

`symtropy.fixed-timebase.identity.v1`

Expected SHA-256 value:

`43608bcf139d9222c356a03530991e3581c72b853f90b4205f923bc5874a0a30`

## Vector M1 — minimal genesis world continuation root

Semantic input:

- schema: `1`;
- world instance: `world:golden`;
- sequence: `0`;
- lifecycle: `Genesis`;
- no parent manifest;
- instant: `20s + 0ns`;
- timebase: vector T1;
- frame: `sol:earth:surface-fixed`;
- inactive-time policy: SHA-256 typed digest of bytes `paused` in domain `symtropy.inactive-time-policy.v1`, schema `1`;
- no forcing context;
- no causal journal head;
- no distributed-authority context;
- zero domain entries;
- zero child manifests.

Expected typed digest domain:

`symtropy.world-continuation-manifest.identity.v1`

Expected SHA-256 value:

`40d8dd14ec5e0786cdba702937b027224d53827dd975fcc002ac0c2bacad0186`

## Required behavior

Implementations must reproduce these values exactly.

Changing the fixture JSON/TOML presentation without changing the semantic fields must not change the digest. Changing any canonical field must change the applicable digest unless a future contract version explicitly states otherwise.

The Rust integration test `tests/continuation_golden.rs` is the first executable consumer of these vectors. Future WASM/server/tooling implementations should consume the same semantic vectors independently.
