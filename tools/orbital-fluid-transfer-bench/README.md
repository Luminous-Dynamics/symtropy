# Orbital Fluid Transfer Reference Benchmark v0.1

Standalone, Rust-first reference validator for Symtropy issue [#6178](https://github.com/Luminous-Dynamics/symtropy/issues/6178).

It tests strict fluid-class applicability, accepted receiver state, exact integer mass accounting and evidence/claim boundaries. It intentionally does **not** model CFD, phase change, heat transfer, pressure-vessel stress, cryogenic chilldown, boil-off dynamics, coupler mechanics or flight operations.

## Run

```sh
cargo fmt --manifest-path tools/orbital-fluid-transfer-bench/Cargo.toml -- --check
cargo test --manifest-path tools/orbital-fluid-transfer-bench/Cargo.toml
cargo clippy --manifest-path tools/orbital-fluid-transfer-bench/Cargo.toml --all-targets -- -D warnings
cargo run --quiet --manifest-path tools/orbital-fluid-transfer-bench/Cargo.toml -- \
  tools/orbital-fluid-transfer-bench/fixtures/v0.1/reference.json \
  --source-commit "$(git rev-parse HEAD)"
```

The receipt binds the exact fixture bytes by SHA-256 and the supplied 40-character source commit. It contains no wall-clock timestamp.

## V0 contracts

- Storable liquid, cryogenic liquid, and high-pressure/supercritical gas remain distinct profiles.
- A storage-only demonstration cannot be upgraded to a transfer claim.
- A transferred product is accepted only against declared final phase, temperature and pressure limits.
- Source inventory and every transfer/residual stream are reconciled in integer grams.
- Accepted receiver mass is never inferred from source tank mass loss alone.
- Unknown/contradictory evidence is Indeterminate or Rejected, never a default pass.
- Event identity and evidence references are required.

## Claim ceiling

A pass establishes only the software accounting/evidence contract for the synthetic fixture. It does not establish any real fluid transfer, cryogenic performance, heat leak/boil-off, coupler compatibility, pressure-vessel safety, materials compatibility, mission service, regulatory acceptance or flight qualification.
