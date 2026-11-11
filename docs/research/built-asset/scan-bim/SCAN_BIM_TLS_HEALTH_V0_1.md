# TLS Longitudinal Precision Health Monitor v0.1

**Issue:** [#4748 — SCAN-BIM-TLS-HEALTH-001](https://github.com/Luminous-Dynamics/symtropy/issues/4748)  
**Parent programme:** [#4589 — Measurement Observability](https://github.com/Luminous-Dynamics/symtropy/pull/4589)  
**Status:** P0 synthetic Rust monitor implemented; exact-head Rust/CI qualification pending.

## 1. Paradigm: monitor change in the measuring instrument, not only the measured building

Most Scan-to-BIM change workflows focus on differences between two building observations. But the measuring instrument itself can change state between manufacturer calibrations, for example after transport or mechanical disturbance. If this is not detected, instrument degradation can masquerade as changes in the asset.

The new monitor establishes a longitudinal baseline of the scanner's spherical-coordinate precision and compares a later interim assessment with it. It is a health signal about the measurement process; it is not a new survey adjustment or registration engine.

## 2. Method reproduced

Gregg, Muralikrishnan & Shilling (2026) describe the NIST/OSAC Interim Performance Assessment (IPA) Part II procedure. At each timepoint, a stationary target array is observed from multiple scanner positions. After centralized-frame processing and target averaging, spherical residuals are calculated. Their 3×3 covariance matrix captures variance in azimuth, elevation and range, together with cross-coordinate covariance.

The comparison tests:

`H_0: \Sigma_{baseline}=\Sigma_{testing}`

The robust omnibus construction is:
1. Subtract the median of each spherical coordinate at its timepoint.
2. Form the six upper-triangle pair products (three variances and three cross-covariances).
3. Apply the signed square-root transform to each product.
4. Compare the two six-dimensional transformed groups using the two-group Hotelling T² / one-way MANOVA equivalent.
5. Convert T² to an F statistic and calculate the p-value at a preregistered significance threshold.

The monitor reports covariance components and standard-deviation ratios alongside the omnibus result. It does not use a single standard-deviation scalar to conceal which coordinate or correlation changed.

**Important interpretation:** failure to reject equal covariance is not evidence that the instrument is unchanged. A significant covariance change does not by itself prove an absolute accuracy loss or identify its mechanical cause. Pair the statistical test with covariance/correlation visualization and a separate length-accuracy assessment.

## 3. P0 Rust implementation

Package: `tools/scan-bim-tls-health-monitor/`

- Strict serde input objects with unknown fields rejected.
- Exact instrument identity, serial, firmware, sensor profile, configuration and calibration certificate are content-bound; each timepoint must bind to the same canonical instrument identity digest.
- Baseline/testing UTC time order is checked.
- Spherical units are explicit: arcseconds, arcseconds, millimetres.
- Target and scanner-position IDs are unique. Every target-position pair must occur exactly once per timepoint.
- Source scan manifest, target array and residual-generation receipt are SHA-256 bound.
- Baseline/testing covariance matrices, standard deviations, correlations, standard-deviation ratios, Hotelling statistic, degrees of freedom and p-value are reported.
- The F survival probability is computed using a bounded continued-fraction implementation of the regularized incomplete beta function; no third-party statistics runtime is used.
- Singular/undersampled pooled covariance is `INDETERMINATE`; no implicit diagonal jitter or covariance repair is performed.
- Reports include `accuracy_disposition=NOT_EVALUATED` and `task_fitness_disposition=NOT_EVALUATED`.

## 4. Dispositions

| Disposition | Meaning | Next interpretation |
|---|---|---|
| `SIGNIFICANT_PRECISION_CHANGE_DETECTED` | Reject equal-covariance null at declared α | Investigate, review covariance changes, consider requalification |
| `NO_SIGNIFICANT_PRECISION_CHANGE_DETECTED` | Insufficient evidence to reject equal-covariance null | Does **not** prove unchanged precision, accuracy or task fitness |
| `INDETERMINATE` | Invalid/incomplete evidence or unsupported test conditions | Repair input, establish a new baseline or obtain competent review |

This tool does not impose an operational stop policy. The responsible instrument owner, metrologist and authority determine the consequence of a significant result.

## 5. Benchmarks and next steps

### P0 synthetic

- Same residual covariance: no significant change detected.
- Range-only precision degradation.
- Elevation-only precision degradation.
- Cross-coordinate covariance/correlation change.
- Singular covariance and insufficient/invalid data.
- Instrument/profile/configuration/calibration identity mismatch.
- Duplicate/missing target-position pair, invalid timestamp and unsupported units.
- Residual ordering permutation and exact-byte digest binding.

### P1 published-method reproduction

Reproduce the NIST/OSAC examples from source data where available, including reported test statistic/p-value and covariance matrices, within declared numeric tolerance. If data are not redistributable, fetch them locally and publish only source metadata/digests and the reproducible analysis receipt. Do not claim a published-method reproduction from synthetic fixtures.

### P2 instrument campaign

For several actual TLS units, collect repeated baseline/interim sessions with instrument IDs and firmware/configuration pinned. Include known interventions (for example controlled profile/variance degradation in a lab setting under competent supervision) and stable controls. Report:
- false alarm probability under stable conditions;
- detection power by degradation magnitude and covariance component;
- sample-size sensitivity;
- baseline and test covariance/standard-deviation/correlation changes;
- session/operator/room/target-array variability;
- agreement with separately qualified length-accuracy checks.

Do not count 80 target-position residuals as 80 independent scanner instruments. The scanner is the longitudinal experimental subject; repeated target measurements are within-session observations.

## 6. Relationship to standards and industry

NIST's TLS Interim Performance Assessment separates Part I length accuracy, Part II spherical-coordinate precision, and an inside/outside test for zero error. Its project page states that the procedure supplements regular manufacturer calibration and is being developed toward an OSAC documentary standard expected in early 2027. This monitor adapts the published Part II statistical method; it does not implement Part I or the appendix, and it does not claim OSAC/ISO conformity.

NIST's 2026 inter-target network calibration study demonstrates very low uncertainty in a specific 20-target/four-position lab arrangement using a laser-tracker reference. It is an important comparator for target-network design and distance accuracy, not a transferable accuracy claim for every scanner or building site.

Industry practice typically relies on manufacturer calibration/service intervals and product-specific performance checks. Luminous should not claim to replace these. The differentiator is a reproducible, content-bound longitudinal evidence record that can be composed with task-specific qualification and change attribution across the built-asset lifecycle.

## 7. Ownership and claim boundary

- #1607 owns survey sessions, calibration, coordinate frames, registration and checkpoint/change evidence.
- #4589 owns measurement detectability, metrology and task-sufficiency gates.
- #4116 owns blind external replay and evaluator isolation.
- #4088/#4692 own building change admission and counterfactual attribution.
- This monitor owns only the longitudinal precision comparison between two instrument timepoints.

P0 establishes deterministic statistical-code behaviour on synthetic residuals. It does not establish actual instrument accuracy, calibration traceability, legal survey fitness, building safety, code compliance or the authority to mutate asset state.
