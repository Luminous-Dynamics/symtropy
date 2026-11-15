# lunar-plasma-isru-evaluator

Standalone, offline Rust evaluator for the synthetic fixture in the Lunar Plasma ISRU process-twin benchmark.

## Run from the Symtropy repository root

```sh
cargo test --manifest-path crates/tools/lunar-plasma-isru-evaluator/Cargo.toml
cargo run --manifest-path crates/tools/lunar-plasma-isru-evaluator/Cargo.toml -- benchmarks/plasma/lunar-plasma-isru-process-v0.json
cargo clippy --manifest-path crates/tools/lunar-plasma-isru-evaluator/Cargo.toml --all-targets -- -D warnings
```

The package has its own Cargo workspace boundary. It does not join the root workspace and does not launch plasma solvers or use network access.

## Checks

- Exact schema and synthetic-only top-level claim boundary.
- Unique scenario, feedstock, material-stream, observation, coproduct and meter identities.
- Feedstock material mass closure.
- Separate reagent inventory closure (opening + makeup = ending + residue-bound + purge).
- Distinct O2 / CO / CO2 species identity.
- Quantitative gas measurement and calibration reference for any accepted O2 product.
- Product mass reconciliation against the feedstock output stream.
- Complete, non-overlapping grid-side energy meter roles.
- Coproduct product-grade and receiving-process gate.
- Explicit NOT_COMPARABLE and expected-disposition mismatch reporting.

## Claim ceiling

A `SYNTHETIC_ACCOUNTING_PASS` means only that the supplied artificial fixture satisfies its declared arithmetic and contract checks. It is not a plasma chemistry calculation, solver result, physical experiment, oxygen production yield, reagent-recycle result, energy-performance claim, product qualification, lunar feasibility result or operating authorization.

The synthetic intensity emitted for the positive control is a code-path exercise only; never use it as an engineering baseline.
