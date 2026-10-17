# Symtropy Geophysics Known-Truth H0

A deterministic fixture package with no runtime dependencies (the manifest-consistency test uses test-only `serde_json`) for GEO-SIM-001. Its purpose is to test observation operators and observability claims before coupling to a larger geodynamo/MHD simulator.

## Scope

- Two dimensionless state coordinates and two observation channels.
- Complementary sensors, a weakly observed high-mode proxy, an exact null mode, and a correlated duplicate measurement.
- Explicit model-relative times, covariance checks, stable fixture IDs, provenance and claim ceiling; simultaneous channels may share a timestamp, and only decreasing time sequences are rejected.
- A frozen public manifest records the four operators, full measurement covariances, observations, and expected covariance-whitened singular values/ranks; hidden state truth is deliberately absent.
- A 2x2 covariance-whitened local observability oracle reports singular values, rank, condition number only when full rank, and weak/null directions. It is an H0 reference, not a large-system solver.
- An exact Gaussian expected-information-gain oracle evaluates `0.5 * ln(det(I + Sa Hᵀ R⁻¹ H))` in nats, using the full covariance matrices from the fixture.
- The EIG oracle also reports information gain per declared acquisition-cost unit; all four fixtures use two channels at unit cost, making the complementary-vs-duplicate comparison budget matched.
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
- The exact expected-information-gain reference distinguishes complementary observations (1.6094379124341003 nats) from the correlated duplicate pair (0.8067249769620305 nats). Treating the duplicate errors as independent incorrectly raises the reference to 1.0986122886681098 nats.
- Correlated duplicate covariance is preserved and positive definite; its expected information gain is 0.8067249769620305 nats, versus 1.0986122886681098 under a deliberately false independent-noise assumption. This is an analytic control, not an empirical performance result.
- Non-finite values, asymmetric covariance and singular covariance fail closed.

No empirical performance benchmark has been run yet. The source is a small H0 control surface, not a geodynamo solver.

Nineteen Rust tests are authored, including exact comparisons with the public fixture manifest. They are not yet executed; check the exact-head CI workflows before claiming qualification.
