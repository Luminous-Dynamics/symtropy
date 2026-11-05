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
- The mean Winkler interval score penalizes misses in addition to width; lower is better. For nominal coverage `1 - alpha`, a miss below/above the interval incurs `2 / alpha` times its distance beyond the relevant bound.
- Reports stratify by regime. Aggregate coverage must never conceal a weak regime. Reports also count unique trajectory groups so repeated points from one trajectory are visible rather than silently treated as independent trials.
- One report is restricted to one model digest, calibration-set digest, test-set digest, physical quantity kind and unit. Mixed experiments must be evaluated separately.
- Model/calibration/test digest fields must be 64-character lowercase SHA-256 hex strings. The evaluator checks digest syntax only; it does not retrieve or authenticate the referenced bytes.
- This v1 evaluator requires one nominal coverage level per report.
- It rejects calibration records, duplicate IDs, mixed model/calibration/test/quantity/unit metadata, invalid intervals and non-finite values.

## Interpretation limits

The output is descriptive empirical held-out performance, not a distribution-free guarantee. Conformal prediction coverage depends on assumptions such as exchangeability; time-correlated trajectories, hardware changes and distribution shift require dedicated evaluation. OOD detection is not a safety certificate. An unsupported regime must be rejected or handled by a separately qualified fallback.

The initial fixture is synthetic known-truth data only. It is not experimental WPT evidence, a standards conformance result, or permission to energize hardware.
