# Mobility Restart/Reconciliation Contract v0.1

This is a dependency-free Rust reference contract for checking restart evidence supplied by the existing persistence, physical-effect, mission-resolution, current-authority and controller-epoch owners.

Start with:

- [Protocol and benchmark](PROTOCOL_AND_BENCHMARK_V0.1.md)
- [Rust contract](src/lib.rs)
- [Public conformance corpus](conformance_cases_v0_1.json)
- [Corpus schema](conformance_cases.schema.json)

Run locally:

```sh
cargo fmt --manifest-path research/mobility-restart-reconciliation/Cargo.toml -- --check
cargo test --manifest-path research/mobility-restart-reconciliation/Cargo.toml --locked
cargo clippy --manifest-path research/mobility-restart-reconciliation/Cargo.toml --all-targets --locked -- -D warnings
```

The reference never calls a native sink. A FreshAttemptCandidate is not authority or dispatch permission. The production adapter must revalidate the exact ticket at the native application boundary. No durable, cryptographic, physical or certification claim is made.
