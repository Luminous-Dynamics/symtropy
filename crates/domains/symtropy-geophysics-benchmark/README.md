# Symtropy Geophysics Known-Truth H0

A dependency-free, deterministic fixture package for GEO-SIM-001. Its purpose is to test observation operators and observability claims before coupling to a larger geodynamo/MHD simulator.

## Scope

- Two dimensionless state coordinates and two observation channels.
- Complementary sensors, a weakly observed high-mode proxy, an exact null mode, and a correlated duplicate measurement.
- Explicit model-relative times, covariance checks, stable fixture IDs, provenance and claim ceiling.
- A frozen public manifest records the four operators, full measurement covariances, observations, and expected covariance-whitened singular values/ranks; hidden state truth is deliberately absent.
- A 2x2 covariance-whitened local observability oracle reports singular values, rank, condition number only when full rank, and weak/null directions. It is an H0 reference, not a large-system solver.
- Estimator-facing API does not contain ground-truth values. Truth fixtures are only compiled for unit tests or when the explicit `benchmark-scoring` feature is enabled.

These are algebraic benchmark controls, not physical geodynamo outputs, observations of Earth, or a validated Earth-interior forward model. The state coordinates are not calibrated physical units.

## Run

```sh
cargo test -p symtropy-geophysics-benchmark --lib --locked
cargo clippy -p symtropy-geophysics-benchmark --all-targets --locked -- -D warnings
cargo fmt --package symtropy-geophysics-benchmark -- --check
```

## Acceptance properties

- Same fixture ID yields identical public observations and manifest metadata.
- Complementary channels can separately expose both toy state coordinates.
- The weak high-mode fixture records sensitivity 0.1 and higher measurement variance, rather than implying resolution from the parameterization.
- Two states with very different hidden-mode values produce exactly identical observations in the null-mode case.
- Correlated duplicate covariance is preserved and positive definite; it is not treated as an independent complementary observation.
- Non-finite values, asymmetric covariance and singular covariance fail closed.

No empirical performance benchmark has been run yet. The source is a small H0 control surface, not a geodynamo solver.
