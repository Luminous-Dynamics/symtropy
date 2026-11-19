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


## Evidence class and claim ceiling

Every campaign input requires an explicit `evidence_class` and `claim_ceiling`; these are echoed in the serialized campaign report and exact-input receipt. Missing values produce `NeedsEvidence`, not `Ready`. The frozen v1 fixture is explicitly synthetic.

The resource and orbit compatibility digests in that fixture are shape-valid test placeholders. The package does not retrieve, authenticate or independently qualify them. The CLI binds exact campaign bytes and the caller-supplied source commit, but it is not a signed hardware/flight receipt. A synthetic pass remains synthetic and cannot be promoted by changing a label.
