# Error estimator calibration reference v0.1

This dependency-free Rust harness measures held-out empirical calibration of local/step error estimates against observed global physical error. It implements the first executable slice of [PHYS-ERROR-ESTIMATOR-CALIBRATION-001 #5194](https://github.com/Luminous-Dynamics/symtropy/issues/5194).

## Data contract

The input is strict TSV v0.1 with an exact header. Every row binds simulator/version/build/configuration, solver/contact regime, estimator kind, trajectory family, calibration/evaluation split, timestep, dimensions/constraints, local estimate (or explicit `NA`), observed global error, tolerance, time/memory, reference build/configuration and units profile.

Trajectory families cannot occur in both calibration and evaluation splits. Groups include exact simulator and reference build/configuration identity, so mismatched runs do not silently pool.

## Calibration

For each estimator group, the harness calculates `ratio = observed_global_error / reported_local_error` on the calibration split. The nearest-rank quantile (default 99%) becomes a frozen multiplicative calibration factor. That factor is evaluated only on held-out trajectory families.

Zero reported estimate with nonzero observed calibration error is uncalibratable and causes rejection; it is never replaced by epsilon. Rows with `estimator_kind=none` must use `NA`; they remain in observed-error/runtime summaries but do not enter coverage or predicted-acceptance calculations.

Outputs include held-out coverage and Wilson 95% interval, estimator ratio p50/p95, predicted-vs-observed tolerance confusion matrix, false-accept/reject fractions, observed global error p50/p95 and runtime/memory summaries.

## Synthetic fixture

`fixtures/synthetic-error-samples-v0.1.tsv` is explicitly synthetic. It demonstrates a Drake CENIC-style half-step estimator calibration group and a MuJoCo 4.0.1 group with no supplied estimator. It is not measured simulator output.

Run:

    cargo fmt --manifest-path research/error-estimator-calibration-001/Cargo.toml --all -- --check
    cargo test --locked --manifest-path research/error-estimator-calibration-001/Cargo.toml
    cargo clippy --locked --manifest-path research/error-estimator-calibration-001/Cargo.toml --all-targets -- -D warnings
    cargo run --locked --manifest-path research/error-estimator-calibration-001/Cargo.toml -- research/error-estimator-calibration-001/fixtures/synthetic-error-samples-v0.1.tsv

## Claim boundary

This tool measures calibration on supplied datasets; it does not prove an error estimator is a mathematical global bound. It does not generate Drake, MuJoCo, Pinocchio or Symtropy simulation samples and does not infer missing estimates. External adapters and independent high-accuracy reference trajectories are separate obligations.