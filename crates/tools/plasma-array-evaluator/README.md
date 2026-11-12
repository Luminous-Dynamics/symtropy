# plasma-array-evaluator

A standalone, dependency-light Rust evaluator for the frozen synthetic scenario set in the Symtropy plasma-array scale-out benchmark.

## Run

From the Symtropy repository root:

```sh
cargo test --manifest-path crates/tools/plasma-array-evaluator/Cargo.toml
cargo run --manifest-path crates/tools/plasma-array-evaluator/Cargo.toml -- benchmarks/plasma/plasma-array-scaleout-v0.json
```

The package has its own Cargo workspace boundary and does not join the root workspace. It uses Serde only for JSON input/output.

## Scope

The evaluator checks manifest identity, synthetic-only evidence, unique cell/scenario IDs, stale configurations, per-cell service reconciliation, active-cell sample CV, linear scale-out efficiency, cross-talk, failure isolation, attempt denominators and complete-system energy/gas boundaries. It compares computed terminal dispositions with each frozen expected disposition.

All values in the fixture are artificial. A successful audit means only that the benchmark contract and arithmetic are internally consistent. It is not a plasma solver, physical observation, device-performance prediction, safety case or authorization to operate hardware.

## Validation status

Rust tests are authored. Run them with a supported Rust toolchain and report exact-head CI separately. No result should be inferred from branch creation or queued/skipped workflows.
