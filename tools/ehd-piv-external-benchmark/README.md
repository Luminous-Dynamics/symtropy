# EHD-PIV external benchmark profile kernel

Dependency-free Rust profile-comparison kernel for independently sourced PIV benchmarks.

## Purpose

Compare estimated and reference mean/stress profiles on an explicit common y+ grid. Report pointwise error and trapezoid-weighted error separately. Preserve missing estimates as missing; report both point coverage and physical y+ span coverage.

## Supported profile semantics

Each call compares one scalar profile channel at a time, for example:
- mean streamwise velocity (U^+);
- Reynolds stress components (u'u'^+), (v'v'^+), (w'w'^+), (u'v'^+), (u'w'^+), (v'w'^+);
- derived TKE/trace only when the exact component convention is recorded;
- hot-wire mean or streamwise variance profiles.

Input values must already be converted to the same units and aligned on one explicit strictly increasing y+ grid. The kernel does not interpolate between grids. Any resampling must happen upstream under a separately recorded method/configuration.

## Metrics

Per full profile and each named y+ band:
- point coverage;
- pointwise bias, RMSE and NRMSE normalized by reference RMS;
- reference-interval agreement (estimate lies in reference interval);
- prediction-interval coverage (reference value lies in prediction interval);
- span coverage;
- trapezoidal bias, RMSE, normalized RMSE and integrated absolute error.

The interval metrics are diagnostics only. They do not establish calibrated uncertainty or finite-sample coverage guarantees. Trapezoidal metrics integrate over nonuniform y+ spacing and omit adjacent segments where either estimate is missing.

## Full Reynolds-stress tensor

The same crate also provides `compare_reynolds_stress_profile` for the six independent entries of a symmetric 3D Reynolds-stress covariance tensor: `uu, vv, ww, uv, uw, vw`. It reports per-component RMSE, TKE bias/RMSE from half the tensor trace, and full-tensor Frobenius RMSE/NRMSE (off-diagonal entries count twice). It checks positive-semidefinite realizability and reports reference/prediction violation counts separately. A perfect TKE trace cannot hide offsetting errors in `uu` and `vv`.

This full 3D realizability diagnostic applies only when all six components are actually available. Planar two-component measurements and one-dimensional hot-wire profiles must not be silently promoted to full 3D tensor evidence.

## Run

    cargo fmt --manifest-path tools/ehd-piv-external-benchmark/Cargo.toml -- --check
    cargo test --manifest-path tools/ehd-piv-external-benchmark/Cargo.toml
    cargo clippy --manifest-path tools/ehd-piv-external-benchmark/Cargo.toml --all-targets -- -D warnings

The crate does not read TIFF, MAT or HDF5 files. A later importer must preserve units, calibration, coordinate orientation, case identity, data revision and conversion receipt.
