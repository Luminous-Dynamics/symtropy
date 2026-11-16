# lunar-plasma-isru-evaluator

Standalone, offline Rust evaluator for the synthetic fixture in the Lunar Plasma ISRU process-twin benchmark.

## Run from the Symtropy repository root

```sh
cargo test --locked --manifest-path crates/tools/lunar-plasma-isru-evaluator/Cargo.toml
cargo run --locked --manifest-path crates/tools/lunar-plasma-isru-evaluator/Cargo.toml -- benchmarks/plasma/lunar-plasma-isru-process-v0.json
cargo clippy --locked --manifest-path crates/tools/lunar-plasma-isru-evaluator/Cargo.toml --all-targets -- -D warnings
```

The package has its own Cargo workspace boundary. It does not join the root workspace and does not launch plasma solvers or use network access.

## Checks

- Exact schema and synthetic-only top-level claim boundary.
- Unique scenario, feedstock, material-stream, observation, coproduct and meter identities.
- Feedstock material mass closure, including reconciliation to the declared dry batch mass.
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


## Species-to-product metrology gate

The measurement receipt is intentionally local to the synthetic plasma process fixture and does not replace the generic measurement-system owner. A quantitative O2 product is scoreable only when:
- the calibration identity matches the product record and covers O2 plus the exact measurement method;
- a normalized UTC validity interval contains the observation timestamp;
- expanded uncertainty is no greater than one-third of the declared decision margin;
- sampling recovery has an independent check identity and the corrected integrated mass matches the product mass;
- cross-sensitivity qualification passed for the declared species;
- a sampling interval and raw trace artifact are identified.

The expanded fixture contains 16 scenarios and 27 Rust tests: 1 synthetic pass, 8 blocked, 6 rejected and 1 not comparable. UTC timestamps are checked for normalized form and real calendar/time ranges; impossible dates are refused. No calibration or measurement values in the fixture represent an actual instrument or experiment.


## Evidence-reference boundary

The synthetic evaluator checks the presence and internal consistency of calibration, recovery, cross-sensitivity, interval and raw-artifact identifiers plus their declared metadata. It does **not** fetch, authenticate or independently inspect any real calibration certificate, reference-gas certificate, raw trace, or recovery study. A real physical qualification must resolve and review those artifacts under the existing measurement-system and evidence authorities; non-empty IDs are not proof of traceability.
