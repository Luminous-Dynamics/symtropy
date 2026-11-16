# hybrid-appj-methane-evaluator

Standalone, offline Rust evaluator for the hybrid APPJ methane-conversion source claims and proposed plasma/thermal factorial-design contract.

## Commands

```sh
cargo test --locked --manifest-path crates/tools/hybrid-appj-methane-evaluator/Cargo.toml
cargo run --locked --manifest-path crates/tools/hybrid-appj-methane-evaluator/Cargo.toml -- benchmarks/plasma/appj-methane-process-v0.json
cargo clippy --locked --manifest-path crates/tools/hybrid-appj-methane-evaluator/Cargo.toml --all-targets -- -D warnings
```

The crate has its own Cargo workspace boundary and does not join the Symtropy root workspace. It performs no network calls and launches no plasma solver.

## Contract

- Keeps the source's >5-fold methane-conversion improvement as a relative claim; absolute conversion remains unknown.
- Keeps approximately 36% carbon deposition separate from carbon capture, recovered feed-carbon fraction and commercial product grade.
- Keeps H2, C2-C3 hydrocarbons and solid carbon as separate streams.
- Requires calibration/measurement/raw-artifact references before quantitative product claims can be scoreable.
- Models the proposed plasma/thermal 2x2 factorial as DESIGN_ONLY, not as a source-reported experiment or synthetic physical measurement.
- Returns NOT_COMPARABLE where product contracts or energy boundaries differ.

All data in the proposed factorial are intentionally unmeasured. Source values are transcribed as source claims, not Luminous measurements.

## Claim ceiling

A SOURCE_CLAIM_ONLY or DESIGN_ONLY disposition validates metadata semantics and refusal behavior only. It is not a plasma simulation, source reproduction, physical measurement, process yield, energy-performance result, carbon-grade qualification, industrial scale-up, or experimental authorization.
