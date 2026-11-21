# Full schema and semantic validation qualification gate

This child change is intentionally stacked on the Navigation Transition benchmark draft. Its CI run exercises the exact parent tree, including the current JSON Schema Draft 2020-12 validator and semantic cross-record validator.

The command under test is:

```sh
cargo test --all-targets --manifest-path research/nav-transition-validator/Cargo.toml
```

The targeted workflow is expected to compile both the library tests and CLI target. The schema unit tests validate the committed 14-case fixture against the committed Draft 2020-12 schema; mutation tests must reject an undeclared fault kind and missing case identity.

No claim of physical navigation validation or SOTA performance is implied by this gate.
