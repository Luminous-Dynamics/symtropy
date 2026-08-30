# Inhabited Epistemic Worlds v0.1 — Qualification Gate

Status: **mechanical qualification required before empirical execution**.

This gate qualifies the integrated Reality Ledger + Artist Eye + explicit
presence surface. It must be run from one frozen source HEAD/TREE. Any source,
lockfile, toolchain, relevant feature, GPU driver/backend, shader, projection,
MSAA, or evidence-policy change starts a new evidence lineage.

## 1. Source and environment capsule

Retain before running tests:

- exact Symtropy HEAD and TREE;
- exact pinned `symthaea-reality-ledger` commit;
- `Cargo.lock` SHA-256;
- `flake.lock` SHA-256 when present;
- `rust-toolchain.toml` SHA-256 when present;
- `rustc -vV`;
- `cargo -Vv`;
- architecture and OS/Nix identity;
- `RUSTFLAGS`, `CARGO_BUILD_TARGET` and relevant build flags;
- GPU model, driver, WGPU backend and adapter identity for live-GPU runs;
- Bevy version;
- object-ID shader digest;
- projection convention and near/far parameters;
- MSAA state;
- capture resolution and fidelity profile.

## 2. Mechanical gates

Run under the repository's real `nix develop` environment.

```bash
cargo fmt --all -- --check

cargo check -p symthaea-bevy-brain --lib --tests
cargo test -p symthaea-bevy-brain --lib --tests
cargo clippy -p symthaea-bevy-brain --lib --tests -- -D warnings

cargo check -p symthaea-bevy-brain --features reality-ledger-adapter --lib --tests
cargo test -p symthaea-bevy-brain --features reality-ledger-adapter --lib --tests
cargo clippy -p symthaea-bevy-brain --features reality-ledger-adapter --lib --tests -- -D warnings

cargo check -p symthaea-bevy-brain --features realtime-art-render,reality-ledger-adapter --lib --tests
cargo test -p symthaea-bevy-brain --features realtime-art-render,reality-ledger-adapter --lib --tests
cargo clippy -p symthaea-bevy-brain --features realtime-art-render,reality-ledger-adapter --lib --tests -- -D warnings

cargo check -p symthaea-bevy-brain --features realtime-art-render,realtime-art-object-id,reality-ledger-adapter --lib --tests
cargo test -p symthaea-bevy-brain --features realtime-art-render,realtime-art-object-id,reality-ledger-adapter --lib --tests
cargo clippy -p symthaea-bevy-brain --features realtime-art-render,realtime-art-object-id,reality-ledger-adapter --lib --tests -- -D warnings
```

The integration test `tests/art_reality_inhabited_integrity.rs` is part of the
`reality-ledger-adapter` gate and must pass. It freezes these higher-order
invariants:

- semantic scene state is truthfully typed as FNV-1a64 rather than BLAKE3;
- genesis, presence and committed binding share one exact root descriptor;
- genesis initial state equals presence entry state as a typed digest;
- passive presence is exactly `Observe`, `Enter`, `Fork`, `Propose`;
- passive presence carries no authority receipt;
- a ghost population is either absent or exactly three children;
- every ghost is a direct `CounterfactualOf` child of the committed root;
- every ledger world descriptor exactly matches the registered graph descriptor;
- the Reality Ledger chain verifies.

## 3. Sensor mechanism qualification

Do not execute the inhabited-world confirmatory episode until both live sensor
mechanisms have their own clean receipts.

### Live metric depth

Use analytically known geometry. Require prospectively frozen tolerances, zero
unexpected readback loss, correct reverse-Z/orthographic reconstruction for the
chosen projection, exact capture identity and no queue drops.

### Live object-ID

Use a small known scene. Require exact `u32` identity recovery through the RGBA8
codec, zero unknown non-zero labels, correct visible support/centroids within
frozen tolerances, exact capture identity, no committed-scene mutation and no
queue drops.

## 4. Atomic object/depth gate

Before either GPU pass, freeze one `ObjectDepthCapturePlan`.

The admitted `WorldObservationBundle` must require exactly the expected object
ID and depth planes and both receipts must agree on:

- world and lineage;
- revision;
- `StudioFrame`;
- semantic scene-state digest;
- camera;
- fidelity;
- prospective plan identity.

Missing, substituted or cross-plan receipts fail closed.

## 5. Passive inhabited episode first

Execute `VART-INHABIT-001` first with **no materialization**:

1. freeze world genesis;
2. open one `WorldPresenceSession`;
3. admit committed-world sensor evidence;
4. register exactly three ghost worlds;
5. observe the ghost worlds;
6. choose `Abstain` or a non-mutating decision path;
7. close presence with explicit exit state/frame;
8. verify the complete ledger;
9. perform post-session memory-admission checks.

A passive PASS requires every proposal memory to remain hypothetical and unable
to claim that it happened in the committed parent world.

## 6. Authorized materialization is a separate evidence lineage

Only after the passive episode passes may a selected proposal be materialized.
That run requires an externally supplied authority receipt and exact typed
selected-state == committed-after-state equality. The other proposal worlds
remain `Counterfactual`.

Do not combine the passive and materialization studies into one post-hoc result.

## 7. Interpretation boundary

A complete PASS supports the following narrow claim:

> Symthaea can be explicitly present in one provenance-identified digital world,
> receive transactionally aligned sensor evidence, explore distinct
> counterfactual child worlds, leave those branches without provenance
> contamination, and exit with a verifiable world/ledger history.

It does **not** establish subjective experience, consciousness, metaphysical
reality, aesthetic competence, unrestricted object permanence, general causal
understanding, or autonomous mutation authority.
