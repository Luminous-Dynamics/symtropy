# EHD PIV conformal and functional uncertainty

This crate adds two reference kernels that complete important missing layers between per-vector covariance and engineering decisions.

## 1. Cluster-wise conformal field calibration

Each calibration unit is a complete acquisition/field cluster with a frozen protocol fingerprint, fixed vector count and all four physical regions. The nonconformity score for a vector is its squared Mahalanobis residual. The score for a field is the maximum across every vector in that field.

For n exchangeable calibration fields and target coverage 1-alpha, the finite-sample rank is:

`k = ceil((n + 1) * (1 - alpha))`

If k is greater than n, the correct conformal set is unbounded. The code returns no finite radius rather than silently using the largest observed residual. At 95% target coverage, 19 complete calibration fields are the mathematical minimum for a finite split-conformal radius.

The guarantee is marginal over calibration and future field clusters under exchangeability. The protocol fingerprint and an external attestation ID are required, but the software cannot prove exchangeability. A new geometry, flow family, tracer, session, temporal regime or grid may violate it.

## 2. Spatially correlated field-functional uncertainty

For oriented face-area vector dA and velocity u:

- mass flux: `rho * (u dot dA)`
- momentum flux component k: `rho * u[k] * (u dot dA)`

Every face sample has a local 3x3 velocity covariance. Cross-location common causes are carried as explicit shared latent modes with one-sigma sensitivities for velocity, density and oriented area. Local errors are treated as independent across samples; common modes induce full cross-sample covariance.

The output is the 4x4 covariance of mass flow and three momentum-flux components, including their cross-covariances.

## Run

```sh
cargo test --manifest-path Cargo.toml
cargo clippy --manifest-path Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path Cargo.toml -- --check
```

## Claim boundary

No raw PIV images, correlation planes, LPT trajectories, pressure/viscous traction, control-volume storage or force-balance readings are ingested. Density and surface-area uncertainty are included only when supplied as shared modes. Cross-location error dependence not represented by a mode is omitted. This is a first-order uncertainty-propagation reference, not an experimental certification.
