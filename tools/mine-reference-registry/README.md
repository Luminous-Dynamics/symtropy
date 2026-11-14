# Mine reference registry validator

This crate validates source metadata; it does not download datasets or parse mine models. It prevents a public URL or academic citation from being mistaken for permission to rehost benchmark data.

Run the frozen contract:

```sh
cargo fmt --manifest-path tools/mine-reference-registry/Cargo.toml -- --check
cargo test --manifest-path tools/mine-reference-registry/Cargo.toml
cargo run --quiet --manifest-path tools/mine-reference-registry/Cargo.toml -- \
  docs/tech/mine-reference-registry-v0.1.json --source-commit "$(git rev-parse HEAD)"
```

The receipt binds exact registry bytes and source commit. Metadata-only entries have no claimed source file SHA. Entries with unverified or restricted rights cannot claim redistribution permission; downloaded bytes require verified rights and a lowercase SHA-256.

The verified-open state is a registry assertion, not an independent legal opinion. Before copying or packaging any third-party work, check the actual artifact's license and applicable terms. MineLib files are not bundled here.
