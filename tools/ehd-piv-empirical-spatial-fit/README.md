# EHD PIV empirical spatial-error fitting

This crate advances the spatial-covariance layer from an externally supplied kernel receipt to a reproducible, cluster-weighted fit plus untouched-acquisition validation.

## Data contract

Each `ResidualField` is one complete acquisition/dependence cluster. It contains a frozen protocol fingerprint, a unique acquisition ID, a unique independent-reference evidence ID, one ordered spatial grid, and the 3-component residual field (measured PIV velocity minus independently established reference velocity). Residuals are in m/s and coordinates in metres.

Calibration and validation cohorts must contain at least 20 independent acquisitions each. Acquisition IDs and reference-evidence IDs must be disjoint between cohorts. Every field must use the same ordered grid and protocol. SHA-256-shaped manifest identities are required for both cohorts; the caller remains responsible for hashing and preserving the actual data and manifest.

## Fit

The initial reference estimator fits one stationary, isotropic 3D Wendland C2 covariance range from a predeclared candidate grid:

`K(r) = (1-r)^4(4r+1)` for `0 <= r < 1`, and zero otherwise.

The model is `Cov(e_i,e_j) = K(||x_i-x_j||/ell) S` for distinct locations. The per-location covariance is decomposed as `C(0) = S + N`, where `S` is the correlated spatial component and `N` is the local nugget. Both must be positive semidefinite. The mean residual profile is estimated from calibration acquisitions and reported separately; it is not silently removed from engineering results.

The fitter gives each complete acquisition one vote. It does not count all vectors or overlapping windows as independent experiments. Candidate range values and acceptance thresholds must be frozen in the fit configuration before opening the validation cohort.

## Held-out validation and UQ-005 adapter

Validation uses only untouched complete acquisition clusters. It bins pairs by physical lag (median nearest-neighbour spacing), computes cluster-level correlations, reports the maximum absolute correlation discrepancy, and tests for statistically resolved negative same-component correlation lobes. The maximum error gate and negative-lobe z threshold are frozen in the fit record. A failed fit or a negative lobe cannot be exported as a UQ-005 kernel.

A qualified report can be converted to the existing `SpatialErrorKernel`. Its `local_nugget_covariance_m2_s2` must be supplied as the per-vector local covariance to the UQ-005 functional reducer. The reducer adds `S` through cross-location terms. Supplying the full `C(0)` as local covariance would double-count `S`.

## Current limits

- One stationary, isotropic Wendland C2 family only. Anisotropic, nonstationary, regional, temporal and oscillatory covariance models are not fitted here.
- A resolved negative lobe is rejected; this crate does not yet fit a different positive-definite oscillatory family.
- The fit uses a fixed ordered grid and complete independent acquisitions. It is not a raw-image PIV processor or a universal correction factor.
- The reference implementation is O(A*N^2) in acquisition count A and vector count N; production acceleration must preserve the same fit and validation contract.
- A unique reference ID, reference-uncertainty assessment ID, protocol fingerprint and manifest digest are assertions/identities, not cryptographic proof of reference independence, exchangeability or raw-data integrity.
- The fitted residual covariance combines PIV and reference uncertainty unless the external reference assessment establishes a negligible reference contribution or a separately qualified correction has already been applied.
- No raw experimental PIV cohort has been ingested or processed by this crate. The synthetic tests are manufactured software tests, not laboratory validation.

## Run

```sh
cargo test --manifest-path Cargo.toml
cargo clippy --manifest-path Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path Cargo.toml -- --check
```

The package depends on the sibling `ehd-piv-conformal-functional-uq` crate for the UQ-005 compatibility type.

## CI

The shared workflow now captures formatter output before the final strict check, so any rustfmt changes can be committed rather than hidden in an automatically mutating CI step. The repository pull-request matrix runs formatting, unit tests and strict Clippy for this crate and its UQ-005 dependency on Ubuntu, Windows and macOS. Qualification claims must cite the exact commit head; queued, skipped or cancelled checks are not passes.


## Separate covariance shape and amplitude gates

Lag-correlation shape is evaluated using the frozen calibration scale C(0)=S+N. The validation cohort's own zero-lag covariance is scored separately as a componentwise validation-to-calibration variance ratio. The initial relative variance-shift gate is 0.10, frozen in FitConfig. A zero calibration variance that becomes non-zero in held-out acquisitions is rejected explicitly. No held-out variance is used to recalibrate the model.
