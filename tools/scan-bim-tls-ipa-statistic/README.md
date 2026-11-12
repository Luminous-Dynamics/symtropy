# TLS IPA Part II Robust Omnibus Consumer

Standalone Rust 1.96 consumer for the O'Brien Robust Omnibus covariance-equality test described by Gregg, Muralikrishnan and Shilling (2026), Journal of Forensic Sciences, DOI 10.1111/1556-4029.70256.

## Scope boundary

This crate consumes already-produced, centralized-frame spherical residuals. It does not register scans, perform bundle adjustment, estimate transforms, generate residuals from raw Cartesian observations, calibrate an instrument, or determine accuracy/task fitness. The residual producer supplies exact period, sensor/profile/setup, target/position, and artifact provenance.

The simpler session-standard-deviation permutation sentinel in #4795 is a separate operational monitor. It is not this IPA Part II method.

## Statistical procedure

For each period, estimate the ordinary sample covariance of the spherical residual vectors (theta, phi, range) and report all six unique covariance components, standard deviations, and correlations.

For the robust omnibus comparison, center each residual coordinate by its period-specific median. For every row and each of the six coordinate pairs (theta,theta), (phi,phi), (range,range), (theta,phi), (theta,range), and (phi,range), form:

    Z[j,k,k'] = (e[j,k] - median[k]) * (e[j,k'] - median[k'])
    W[j,k,k'] = sign(Z[j,k,k']) * sqrt(abs(Z[j,k,k']))

The two groups of six-dimensional transformed vectors are compared with two-group Hotelling T-squared, expressed as an equivalent one-way MANOVA F statistic with df1=6 and df2=n_baseline+n_testing-7. The p-value is the F upper-tail probability.

An invertible diagonal scale transformation is applied to the six W components before matrix inversion. Hotelling T-squared is invariant under this nonsingular change of coordinates; it improves numerical conditioning across angular and range components. The pooled covariance must still be positive definite. No jitter, shrinkage, diagonal loading, or singular-matrix repair is applied.

## Input

- Closed JSON contract: `schema/ipa-input.schema.json` (Draft 2020-12).
- Exact instrument identity, firmware, sensor profile, calibration reference, and acquisition configuration.
- Exact centralization-method and residual-producer identities/digests.
- Two timepoints with residual-artifact, centralization-receipt, target-layout, and station-manifest digests.
- At least 20 target identities and four scan positions per timepoint, with one row for every declared target-position pair.
- Angular residuals in radians; range residuals in metres or millimetres, using the same unit in both periods.
- Baseline and testing must use the same instrument/profile/setup and target lineage. Target-layout and station manifests may be different across timepoints.

## Output semantics

- `SIGNIFICANT_PRECISION_CHANGE_DETECTED`: reject equal covariance at the declared alpha. Investigate the instrument; this does not prove inaccuracy or identify a failed component.
- `NO_SIGNIFICANT_PRECISION_CHANGE_DETECTED`: insufficient evidence to reject equal covariance. This is not proof that precision is unchanged.
- `INDETERMINATE`: invalid scope/data, inadequate design, non-finite residuals, or singular/non-positive-definite pooled transformed covariance.

Accuracy and task fitness are always `NOT_EVALUATED`. Output includes six covariance components, standard deviations, correlations, three one-sigma descriptive ellipse parameters, T-squared, F, degrees of freedom, p-value, and exact input digest.

The residual-artifact digest is a referenced producer identity. P0 checks its shape and binds the exact JSON input digest; it does not authenticate external artifact bytes or prove the residual producer's algorithm.

## Reproduction and benchmark path

- `fixtures/no-change.json`: independent synthetic residual samples from a shared covariance. At alpha 0.01, the independently recalculated p-value is approximately 0.2318.
- `fixtures/covariance-change.json`: synthetic multivariate change affecting angular spread, range spread, and cross-coordinate correlation. At alpha 0.01, the independently recalculated p-value is approximately 1.72e-17.
- Both fixtures contain 20 target identities, four scan positions, and 80 residual rows per period.
- Unit tests exercise both known-answer controls, covariance/ellipse outputs, profile/target-lineage mismatch, duplicate/missing target-position pairs, non-finite data, singular transformed covariance, row-order invariance, and the no-accuracy/no-task-fitness boundary.

P1 must reproduce the paper's instrument examples from source data or source-data-derived summary statistics under an independent implementation. It must also measure false-alarm rate and power under Gaussian, heavy-tailed, outlier-contaminated, unequal-size, target/position-dependent, and temporally correlated residuals. Rows sharing targets or stations are not automatically independent; the paper-faithful pointwise test and dependence-aware extensions must be reported separately.

## Claim ceiling

P0 is algorithm/contract behavior on synthetic residuals. It is not a sensor-performance certificate, calibration, absolute-accuracy result, task-fitness pass, forensic conclusion, or return-to-service authorization.
