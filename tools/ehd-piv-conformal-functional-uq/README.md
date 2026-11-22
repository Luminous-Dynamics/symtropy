# EHD PIV conformal and functional uncertainty

This crate adds five typed reference paths that complete important missing layers between per-vector covariance and engineering decisions: planar conformal calibration, 3D conformal calibration, field-functional propagation, conformal calibration of integrated outputs against independent functional references, and compact-support spatial covariance for overlapping PIV windows.

## 1. Cluster-wise conformal field calibration

Each calibration unit is a complete acquisition/field cluster with a frozen protocol fingerprint, fixed vector count and all four physical regions. The nonconformity score for a vector is its squared Mahalanobis residual. The score for a field is the maximum across every vector in that field.

For n exchangeable calibration fields and target coverage 1-alpha, the finite-sample rank is:

`k = ceil((n + 1) * (1 - alpha))`

If k is greater than n, the correct conformal set is unbounded. The code returns no finite radius rather than silently using the largest observed residual. At 95% target coverage, 19 complete calibration fields are the mathematical minimum for a finite split-conformal radius.

This is the whole-field maximum-score construction. Dunn, Wasserman and Ramdas (2023) provide related two-layer hierarchical conformal methods (CDF pooling and repeated subsampling) for new-group prediction; benchmark both because their estimand and efficiency trade-offs differ.

The guarantee is marginal over calibration and future field clusters under exchangeability. The protocol fingerprint and an external attestation ID are required, but the software cannot prove exchangeability. The 2D scorer and 3D scorer are separate APIs; do not use a 2D radius to claim out-of-plane coverage. A new geometry, flow family, tracer, session, temporal regime or grid may violate it.

## 2. Spatially correlated field-functional uncertainty

For oriented face-area vector dA and velocity u:

- mass flux: `rho * (u dot dA)`
- momentum flux component k: `rho * u[k] * (u dot dA)`

Every face sample has a local 3x3 velocity covariance. Cross-location common causes are carried as explicit shared latent modes with one-sigma sensitivities for velocity, density and oriented area. Local errors are treated as independent across samples; common modes induce full cross-sample covariance.

The output is the 4x4 covariance of mass flow and three momentum-flux components, including their cross-covariances. The package includes both 2D and 3D conformal scorers. The 3D path is dimension-compatible with the 3D field-functional covariance model, provided the same protocol, target field size, and full 3-component covariance model are used.

## 3. Integrated functional conformal calibration

The `functional_conformal` module includes an adapter from `FieldFunctionalReport` to a calibration observation, preserving [mass, momentum-x, momentum-y, momentum-z] order, reference-minus-prediction residual sign, source run ID and the exact 4×4 covariance. It then calibrates the complete four-component vector [mass flow, momentum-x, momentum-y, momentum-z] against independent reference functional evidence. Each lab-session cluster is scored by its maximum 4D Mahalanobis residual. The same exact conformal rank and whole-session exchangeability assumptions apply.

Covariance scoring first standardizes by component standard deviations to a dimensionless correlation matrix, then uses Cholesky factorization. This is necessary because mass flow (kg/s) and momentum flux (N) have different units. The calibration requires the exact same functional-contract identity, protocol fingerprint, and number of functional observations per session.

It does not qualify total thrust unless all control-volume terms and an independent force-balance reference are separately complete.

## 4. Spatially correlated PIV error

The `spatial_correlation` module adds an anisotropic Wendland C2 compact-support covariance between neighboring vector locations. It composes with, but is distinct from, independent local covariance and shared global nuisance modes. Each kernel requires a calibration receipt carrying a matching protocol fingerprint, at least 20 independent acquisition clusters, a declared correlation-fit error below its frozen limit, and no resolved negative correlation lobe.

The current Wendland model is nonnegative and stationary. It must be rejected if empirical error correlation has a negative lobe or if the fit fails. Compare it against empirical covariance, Gaussian/Matérn and oscillatory positive-definite alternatives. The reference implementation costs O(N²); production needs compact neighbor lists or a matrix-free implementation.

## Run

```sh
cargo test --manifest-path Cargo.toml
cargo clippy --manifest-path Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path Cargo.toml -- --check
```

An authored finite-difference test checks every mass/momentum Jacobian component on a non-axis-aligned face against central numerical derivatives.

## Claim boundary

No raw PIV images, correlation planes, LPT trajectories, pressure/viscous traction, control-volume storage or force-balance readings are ingested. Density and surface-area uncertainty are included only when supplied as shared modes. Cross-location error dependence not represented by a mode is omitted. This is a first-order uncertainty-propagation reference, not an experimental certification.
