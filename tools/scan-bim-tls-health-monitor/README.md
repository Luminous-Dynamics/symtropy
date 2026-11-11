# Scan-to-BIM TLS Precision Health Monitor v0.1

A bounded Rust reference implementation for longitudinally comparing the spherical-coordinate precision of one terrestrial laser scanner between a baseline and a later interim assessment.

## What it does

- Validates exact instrument, firmware, profile, configuration, calibration, timepoint, target-array, scan-manifest and residual-generation bindings.
- Consumes upstream-generated spherical residuals: azimuth (arcseconds), elevation (arcseconds) and range (millimetres).
- Estimates each timepoint's 3×3 covariance matrix.
- Reproduces the O'Brien Robust Omnibus Test construction: median-centred pairwise products, signed square-root transform, and a two-group Hotelling T² / MANOVA comparison over six covariance components.
- Reports the statistic, degrees of freedom, p-value, covariance matrices, coordinate standard deviations/correlations and testing/baseline standard-deviation ratios.
- Binds the exact input bytes with SHA-256.

## What it does not do

It does not register scans, centralize target arrays, estimate transforms, calibrate a scanner, test absolute length accuracy, test zero error, or decide whether a scanner is fit for a named building task. It consumes residuals produced by a separately qualified upstream process.

A significant result means **the estimated spherical-coordinate covariance changed** under the declared test. It does not identify the mechanical cause or prove absolute inaccuracy.

A non-significant result means **the test did not reject equal covariance**. It is not proof that no change occurred, and it is not an accuracy or task-fitness certificate.

The approach adapts the Part II precision test described by Gregg, Muralikrishnan & Shilling (2026), which forms an interim part of the NIST/OSAC TLS Interim Performance Assessment. It is not itself an OSAC standard or a claim of standards conformance.

## Run

```sh
cargo fmt --manifest-path tools/scan-bim-tls-health-monitor/Cargo.toml -- --check
cargo test --locked --manifest-path tools/scan-bim-tls-health-monitor/Cargo.toml
cargo clippy --locked --manifest-path tools/scan-bim-tls-health-monitor/Cargo.toml --all-targets -- -D warnings

cargo run --locked --manifest-path tools/scan-bim-tls-health-monitor/Cargo.toml -- \
  --input tools/scan-bim-tls-health-monitor/fixtures/no-change.json

cargo run --locked --manifest-path tools/scan-bim-tls-health-monitor/Cargo.toml -- \
  --input tools/scan-bim-tls-health-monitor/fixtures/precision-change.json
```

## Statistical construction

For each residual row (e=(e_\theta,e_\phi,e_r)), subtract the coordinate-wise median from that timepoint. For the six upper-triangle coordinate pairs (k\leq l), compute:

`W_kl = sign((e_k - median_k)(e_l - median_l)) * sqrt(abs((e_k - median_k)(e_l - median_l)))`

The two timepoints' six-dimensional W vectors are compared with the two-sample Hotelling T² test. The p-value is calculated from the corresponding F distribution. Singular pooled covariance, insufficient sample size, changed instrument binding, invalid timestamps/units, incomplete target-position grids and malformed provenance return `INDETERMINATE`; the implementation does not add diagonal jitter or repair singular matrices.

## P0 profile and synthetic controls

This v0.1 benchmark profile requires at least 20 targets and four scanner positions at each timepoint. The NIST/OSAC paper describes those counts as a practical draft configuration, not an absolute requirement for every environment.

The frozen no-change fixture uses identical deterministic residual values at two timepoints to provide a simple exact invariance control. Unit tests also generate a range-only precision degradation, elevation degradation, covariance/correlation change, singular data, invalid units, stale instrument bindings and invalid residual grids. These tests establish code behaviour only; they do not qualify an actual scanner.

## Research sources

- Gregg, Muralikrishnan & Shilling (2026), *A statistical procedure to assess change in instrument precision*, Journal of Forensic Sciences, DOI [10.1111/1556-4029.70256](https://doi.org/10.1111/1556-4029.70256).
- NIST TLS Interim Performance Assessment project and OSAC-standard development status: https://www.nist.gov/programs-projects/terrestrial-laser-scanner-performance-evaluation-and-documentary-standard-forensic
- Muralikrishnan et al. (2026), inter-target distance network calibration using TLS, DOI [10.1016/j.precisioneng.2026.04.029](https://doi.org/10.1016/j.precisioneng.2026.04.029).

## Claim ceiling

Synthetic tests are not physical measurement evidence. This tool does not establish scanner accuracy, calibration traceability, legal survey fitness, structural adequacy, code compliance or safety. A significant precision-change result should trigger investigation and competent review; the result alone does not impose a manufacturer-calibration interval or an operational stop policy.
