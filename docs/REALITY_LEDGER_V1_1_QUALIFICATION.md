# Reality Ledger v1.1 Symtropy Adapter — qualification

Run under the project Nix development shell.

## Qualified-base preservation

This branch was constructed from qualified v1D source head
`a00fc96245a06509a8c8e387ac19280b145b91ac`. Re-run the default v1D surface to
prove the feature-gated adapter did not alter the existing path:

```bash
cargo fmt --all -- --check
cargo check -p symthaea-bevy-brain --lib --tests
cargo test -p symthaea-bevy-brain --lib
cargo clippy -p symthaea-bevy-brain --lib --tests -- -D warnings
```

## Reality adapter feature

```bash
cargo check -p symthaea-bevy-brain --features reality-ledger-adapter --lib --tests
cargo test -p symthaea-bevy-brain --features reality-ledger-adapter --lib --tests
cargo clippy -p symthaea-bevy-brain --features reality-ledger-adapter --lib --tests -- -D warnings
```

The Cargo lock must resolve `symthaea-reality-ledger` to exact pinned commit
`ffa27ea1a0fa2bac69df6008adfdd2167b8e29c0` unless the integrated monorepo
explicitly replaces that source with a byte-equivalent locally qualified tree.
Record the resolved package source in the receipt.

## Required semantic gates

- exactly one four-ghost candidate maps to `DigitalCommitted`;
- exactly three map to distinct `Counterfactual` child worlds;
- counterfactual parents point to the committed world ID + lineage;
- wrong candidate membership fails;
- capture without artifact digest fails;
- candidate state digest binds the actual rendered semantic scene hash;
- revision/frame/camera/fidelity remain equal to the four-ghost evidence plane;
- typed digest domain mismatch fails materialization even when raw values match;
- selected materialization requires external authority evidence;
- adapter construction never mutates ArtPort/world state.

## Live VART-REALITY-SYMTROPY-001

After the mechanical gates pass, execute the preregistered study in
`docs/VART_REALITY_SYMTROPY_001.md` using real capture receipts.

No claim of subjective presence, aesthetic value, physical grounding or general
world-creation authority follows from a PASS.
