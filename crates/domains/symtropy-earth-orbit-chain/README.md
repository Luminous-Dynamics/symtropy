# Earth-to-Orbit Chain Reference

Package: `symtropy-earth-orbit-chain`

This package reconciles an ordered terrestrial/orbital payload path, consumes time-bounded resource qualification and specialist orbit-compatibility evidence, and emits per-cargo delivery results. It does not calculate transport capacity, launch azimuth, trajectory, orbital insertion or flight dynamics.

## Run the reference campaign

```sh
cargo fmt --manifest-path Cargo.toml --package symtropy-earth-orbit-chain -- --check
cargo test --locked --manifest-path Cargo.toml --package symtropy-earth-orbit-chain --all-targets
cargo clippy --locked --manifest-path Cargo.toml --package symtropy-earth-orbit-chain --all-targets -- -D warnings
cargo run --locked --manifest-path Cargo.toml --package symtropy-earth-orbit-chain --bin earth-orbit-chain-reference -- \
  crates/domains/symtropy-earth-orbit-chain/fixtures/v1/nominal.json \
  --source-commit "$(git rev-parse HEAD)"
```

The JSON receipt binds the exact campaign input bytes by SHA-256 and the source commit supplied by the caller. It contains per-cargo delivery records with cargo identity/mass, target orbit, destination, terminal node, arrival epoch and status. A non-ready campaign emits `indeterminate` or `fail` and returns a nonzero process code.

## Claim boundary

The fixture is synthetic. Its resource and orbit-compatibility SHA-256 values are test placeholders, not fetched, signed or authenticated qualification receipts. The CLI verifies only digest shape in the model, not source issuer/signature/truth. The receipt proves only deterministic execution of this declared software contract on the named input bytes.

A passing synthetic receipt does not prove a booking, resource reservation, payload integration, launch, orbit insertion, custody transfer, mission success, service acceptance, regulatory compliance or flight readiness. The output is a downstream evidence input, not an authorization.
