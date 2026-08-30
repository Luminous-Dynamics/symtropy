# Inhabited Epistemic Worlds v0.2 — Lifecycle Qualification Gate

Status: **fresh mechanical and empirical qualification required**.

The v0.1 inhabitation hardening and the Reality Ledger v1.3 lifecycle source are
parents of this work, but no previous PASS transfers automatically to this new
HEAD/TREE.

## 1. Freeze source and environment

Retain:

- exact Symtropy HEAD/TREE;
- exact pinned `symthaea-reality-ledger` HEAD/TREE;
- Cargo.lock SHA-256;
- flake.lock SHA-256 when present;
- rust-toolchain.toml SHA-256 when present;
- `rustc -vV` and `cargo -Vv`;
- architecture, OS and Nix shell identity;
- `RUSTFLAGS`, `CARGO_BUILD_TARGET` and relevant features;
- Bevy version;
- persistence adapter identity + serialization schema;
- world genesis, physics profile and asset-manifest digests;
- timebase policy;
- GPU/driver/WGPU backend whenever live sensor evidence is included.

## 2. Qualify host-neutral Reality Ledger v1.3 first

From the exact pinned Symthaea source, execute
`crates/domains/symthaea-reality-ledger/QUALIFICATION_V1_3.md`.

Do not continue if the public lifecycle source needs correction. Commit the
correction, update the Symtropy pin, freeze new HEAD/TREE values, and restart.

## 3. Symtropy mechanical gates

Run under `nix develop`:

```bash
cargo fmt --all -- --check

cargo check -p symthaea-bevy-brain --lib --tests
cargo test -p symthaea-bevy-brain --lib --tests
cargo clippy -p symthaea-bevy-brain --lib --tests -- -D warnings

cargo check -p symthaea-bevy-brain --features reality-ledger-adapter --lib --tests
cargo test -p symthaea-bevy-brain --features reality-ledger-adapter --lib --tests
cargo clippy -p symthaea-bevy-brain --features reality-ledger-adapter --lib --tests -- -D warnings

cargo run -p symthaea-bevy-brain \
  --features reality-ledger-adapter \
  --bin inhabited_epistemic_v02_lifecycle_smoke

cargo check -p symthaea-bevy-brain \
  --features realtime-art-render,realtime-art-object-id,reality-ledger-adapter \
  --lib --tests
cargo test -p symthaea-bevy-brain \
  --features realtime-art-render,realtime-art-object-id,reality-ledger-adapter \
  --lib --tests
cargo clippy -p symthaea-bevy-brain \
  --features realtime-art-render,realtime-art-object-id,reality-ledger-adapter \
  --lib --tests -- -D warnings
```

## 4. Structural lifecycle gates

Mechanical PASS must demonstrate:

- closed episode -> snapshot copies the exact exit typed state;
- persisted artifact bytes are BLAKE3-bound independently from the FNV-1a64
  semantic state identity;
- changed persisted bytes change the snapshot digest;
- successor snapshots reference the exact previous snapshot and cannot regress;
- lifecycle timelines enforce `Active -> Suspended -> Active` or
  `Active -> Suspended -> Archived`;
- Resume before Suspend fails;
- Resume after Archive fails;
- Revisit while Suspended or Archived fails;
- lifecycle transitions require external authority receipts;
- restored typed state must equal snapshot typed state;
- prior presence must be closed;
- resumed presence must be a distinct open session for the same world + agent;
- prior exit == snapshot == resumed entry state;
- a continued inhabited episode reuses the original world genesis rather than
  inventing a new genesis at the restored state;
- the continued episode ledger begins with an exact snapshot continuation
  anchor and verifies as a new append-only segment;
- changing the original genesis while retaining the snapshot fails closed;
- ephemeral counterfactual fork begins from exact snapshot state and remains
  authority-poor;
- persisted committed fork is a distinct `SpawnedFrom` child and requires
  external persistence authority.

## 5. Host persistence qualification

The new Rust contracts do **not** prove actual Bevy serialization/restoration.
Before `VART-LIFECYCLE-001`, qualify the selected host persistence adapter with
known-state fixtures.

Required evidence:

1. serialize known committed world to bytes;
2. hash exact bytes used for restoration;
3. eliminate the original live state as the restore source;
4. deserialize the persisted bytes into a new host instance;
5. recompute canonical semantic scene state independently;
6. require typed restored state == snapshot state;
7. verify required assets, parent relations and stable object IDs;
8. verify the frozen timebase/restart policy;
9. when physics/runtime continuation is claimed, separately verify the frozen
   dynamic-state surface rather than inferring it from semantic-scene equality.

## 6. Execute VART-LIFECYCLE-001

Populate `VART_LIFECYCLE_001_PLAN.template.json` prospectively. Keep
`execution_authorized=false` until all mechanical and persistence gates pass and
all thresholds/policies are frozen.

The primary confirmatory study is leave -> persist -> suspend -> destroy live
state -> restore -> recompute -> resume -> revisit. The exploratory fork check
must remain analytically separate from the primary PASS.

## 7. Interpretation boundary

A full PASS supports:

> One provenance-identified Symtropy digital world can be persisted and later
> re-entered under a new explicit presence session while preserving the frozen
> world/state lineage and lifecycle evidence contracts.

It does not establish subjective continuity, consciousness during suspension,
physical identity, unrestricted host determinism, or autonomous persistence
and lifecycle authority.
