# EHD PIV covariance calibration reference

This standalone Rust crate evaluates whether estimated two-component PIV velocity covariance predicts actual vector error on a frozen reference cohort.

## What it measures

- 95% streamwise marginal interval coverage.
- 95% wall-normal marginal interval coverage.
- 95% joint two-dimensional covariance-ellipse coverage using the chi-square threshold for two degrees of freedom.
- Pooled vector-count coverage and equal-weight coverage over declared independent dependence clusters.
- An approximate cluster-level 95% confidence interval, available only with at least 20 independent clusters.
- Mean squared Mahalanobis distance, bivariate Gaussian negative log likelihood, covariance-ellipse area and marginal interval width.

The joint ellipse matters: two 95% marginal intervals do not imply a 95% joint region. A vector can fall inside both component intervals yet outside the predicted covariance ellipse.

## Run

From this directory:

```sh
cargo test --manifest-path Cargo.toml
cargo clippy --manifest-path Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path Cargo.toml -- --check
```

These commands are authored instructions. The Rust tests have not been run in the current implementation environment because cargo/rustc/rustfmt are unavailable.

## Evidence and claim boundaries

This is a reference-data evaluator, not a PIV image processor. A caller must provide independent reference velocities, estimated velocities, and a positive-definite predictive covariance for every vector.

The independence cluster must represent the highest known shared-cause group, not merely a unique file or processed run. Repeated/overlapping interrogation windows, same-session frames, camera calibration, illumination and tracer batches can correlate errors.

The cluster-level confidence interval is approximate and relies on correctly defined independent clusters. It uses a t critical value of 2.093 for at least 20 clusters; this is conservative for larger sample counts under the t approximation. It is not a standards-compliant uncertainty certificate.

No raw images, independent measurements, experimental EHD results, solver comparisons or SOTA wins are included. Synthetic unit-test fixtures only test evaluator behavior.
