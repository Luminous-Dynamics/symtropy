# WPT predictive uncertainty benchmark v1

This benchmark evaluates interval coverage and sharpness from frozen held-out observations. It intentionally accepts only `split: "test"` records; calibration-set construction is upstream and must be frozen independently.

## Run

```sh
cargo test --manifest-path tools/wpt-predictive-uncertainty/Cargo.toml
cargo run --manifest-path tools/wpt-predictive-uncertainty/Cargo.toml -- benchmarks/wpt-predictive-uncertainty/v1/known-truth.jsonl
```

## Semantics

- A record is one prediction/observation pair for one declared physical quantity, unit, model, calibration set, test set, trajectory group and operating regime.
- Coverage is inclusive: lower <= observed <= upper.
- Sharpness is mean interval width in the record's declared unit. Do not compare widths across unlike quantities or units.
- Reports stratify by regime. Aggregate coverage must never conceal a weak regime.
- This v1 evaluator requires one nominal coverage level per report.
- It rejects calibration records, duplicate IDs, invalid intervals and non-finite values.

## Interpretation limits

The output is descriptive empirical held-out performance, not a distribution-free guarantee. Conformal prediction coverage depends on assumptions such as exchangeability; time-correlated trajectories, hardware changes and distribution shift require dedicated evaluation. OOD detection is not a safety certificate. An unsupported regime must be rejected or handled by a separately qualified fallback.

The initial fixture is synthetic known-truth data only. It is not experimental WPT evidence, a standards conformance result, or permission to energize hardware.
