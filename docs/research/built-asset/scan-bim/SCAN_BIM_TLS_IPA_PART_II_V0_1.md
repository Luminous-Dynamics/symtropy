# TLS IPA Part II — Robust Omnibus Precision-Change Consumer v0.1

**Owner:** #4807 — SCAN-BIM-TLS-IPA-REPRODUCTION-001  
**Implementation:** `tools/scan-bim-tls-ipa-statistic/`  
**Status:** P0 synthetic residual consumer and known-answer fixtures implemented; exact-head Rust/Actions qualification pending.

## 1. Purpose and boundary

Reproduce the statistical consumer in the 2026 NIST/OSAC Interim Performance Assessment (IPA) Part II procedure for detecting a change in a terrestrial laser scanner's spherical-coordinate precision between two assessment timepoints.

This is deliberately separate from the lightweight session-SD permutation sentinel in #4795. That monitor is an operational repeatability signal; it is not a reproduction of the published IPA Part II test.

Part II estimates a covariance matrix over azimuth, elevation and range residuals, then compares the baseline and testing covariance structures. The test detects evidence of a precision change; it does not establish absolute accuracy, calibration validity, task fitness, or authority to return a scanner to service.

## 2. Published procedure

The NIST/OSAC workflow acquires a stationary target array from multiple TLS positions, converts the position-specific target coordinates to a centralized frame using the prescribed simplified bundle-adjustment/composite-coordinate procedure, back-transforms the composite target coordinates to each scan position, and computes spherical residuals (theta, phi, range). The paper's statistical consumer estimates the three-by-three covariance matrix for each timepoint.

For every row j in timepoint i, each spherical coordinate k is centered by its timepoint-specific median M[i,k]. For each of the six unique covariance components, the O'Brien robust transform is:

    Z[i,j,k,k'] = (e[i,j,k] - M[i,k]) * (e[i,j,k'] - M[i,k'])
    W[i,j,k,k'] = sign(Z[i,j,k,k']) * sqrt(abs(Z[i,j,k,k']))

The six transformed variables correspond to theta-theta, phi-phi, range-range, theta-phi, theta-range and phi-range. A one-way MANOVA compares the two timepoint groups; for two groups this reduces to Hotelling T-squared. The equivalent F test has numerator degrees of freedom p=6 and denominator degrees of freedom n_baseline+n_testing-p-1.

Research source: Gregg, Muralikrishnan & Shilling (2026), *Toward a documentary standard for performance testing of terrestrial laser scanners used in forensic practice: A statistical procedure to assess change in instrument precision*, Journal of Forensic Sciences 71(2), 1023–1031. [Full paper, DOI 10.1111/1556-4029.70256](https://doi.org/10.1111/1556-4029.70256). The paper describes the centralized-frame residual-generation procedure and O'Brien/Hotelling test; P0 here consumes residuals from that upstream process rather than reimplementing it.

## 3. Existing executable

The standalone Rust 1.96 crate includes:

- Closed serde input types and Draft 2020-12 JSON Schema.
- Exact instrument/profile, firmware, calibration-reference, acquisition configuration, residual-producer and centralization-method identity/digests.
- Two timepoints with exact target/position manifests and residual/centralization artifact references.
- Complete target-position grids, unique target/position/row identifiers, explicit angle/range units, chronological order and non-finite-value rejection.
- Raw three-by-three covariance, standard deviations, correlations and descriptive pairwise one-sigma ellipse parameters.
- O'Brien's six signed-square-root transformed components and the two-group Hotelling T-squared / F test.
- Positive-definiteness checks on the pooled transformed covariance. Singular or near-singular cases become `INDETERMINATE`; no jitter, shrinkage or diagonal loading is used.
- `accuracy_disposition` and `task_fitness_disposition` remain `NOT_EVALUATED` for every result.

The implementation applies a common invertible diagonal scaling to the six transformed coordinates before matrix inversion. Hotelling T-squared is invariant under a nonsingular linear coordinate transformation; the scaling is for numeric conditioning between angular and range components, not a change to the test statistic.

**Evidence limit:** P0 validates the consumer contract and calculation over synthetic residuals. Digest-shaped residual/centralization identifiers are bound into the input but their referenced bytes and producer method are not authenticated or reproduced by this crate.

## 4. Synthetic known-answer controls

The fixtures use deterministic synthetic samples, 20 target identities and four scan positions per timepoint (80 rows each). Significance is predeclared at alpha=0.01.

| Fixture | Statistical condition | Independent recalculation | Expected result |
|---|---|---:|---|
| `no-change.json` | Same generating covariance in both periods | p ≈ 0.2318236 | `NO_SIGNIFICANT_PRECISION_CHANGE_DETECTED` |
| `covariance-change.json` | Changed angular/range spread and cross-coordinate correlations | p ≈ 1.7214e-17 | `SIGNIFICANT_PRECISION_CHANGE_DETECTED` |

The p-values were independently recalculated from the committed JSON fixtures using a separate implementation of the median-centering, six-component transform, pooled covariance, Hotelling statistic and F upper-tail probability. This is a fixture-level numerical cross-check, not Rust execution or physical scanner evidence.

## 5. Statistical assumptions and limits

The P0 Hotelling test assumes the six-dimensional transformed row vectors are independent within and across timepoint groups and that the two-group multivariate test approximation is appropriate. Multiple residual rows can share physical targets, scanner stations, environmental conditions and session-level errors. The current consumer validates unique target-position rows, but uniqueness does not prove statistical independence.

Accordingly:

- The P0 method result is the paper-faithful pointwise test over the supplied residual rows; do not relabel it a cluster-robust or dependence-adjusted test.
- P1 must simulate and characterize type-I error and power under target/position dependence, session clustering, temporal autocorrelation, unequal sample sizes, outliers and heavy-tailed residuals.
- Any dependence-aware extension must have its own method ID, assumptions, calibration and comparison to the paper-faithful result. Do not silently replace the published test.
- `NO_SIGNIFICANT_PRECISION_CHANGE_DETECTED` means failure to reject equality at the chosen alpha. It is not proof that the scanner's precision is unchanged.
- `SIGNIFICANT_PRECISION_CHANGE_DETECTED` triggers investigation and re-verification; it does not prove absolute inaccuracy or identify a failed component.

## 6. Benchmark ladder

**P0 — synthetic algorithm reproduction**

- 100% expected disposition agreement for no-change and seeded covariance-change controls.
- Known-answer covariance entries and six-component ordering checked against an independent implementation.
- 100% rejection/indeterminate outcomes for malformed scope, duplicate or incomplete target-position grids, non-finite residuals, changed instrument/profile/setup and singular transformed covariance.
- Zero implicit covariance repair and zero promotion of a non-rejection to accuracy or task-fitness PASS.
- Deterministic semantic results under row-order permutations; exact input digest remains byte-sensitive.

**P1 — independent reproduction and instrument study**

- Reproduce the paper's published instrument examples or source-derived summary statistics with explicit tolerances and provenance.
- Preserve raw coordinate artifacts, centralized-frame producer, back-transformation and residual calculation as separate versioned outputs owned by #1607 or another independently qualified producer.
- Preregister alpha, minimum targets/positions/sessions, exclusions, practical change magnitudes and false-alarm policy.
- Measure type-I error and power across Gaussian, heavy-tailed and contaminated cases; angular-only, range-only, correlation-only and combined covariance changes; unequal group sizes; and realistic dependence structures.
- Compare the O'Brien test with the simpler #4795 session-SD sentinel, Box's M as a deliberately fragile reference comparator, and a predeclared bootstrap covariance test. Keep method assumptions and estimands distinct.
- Report false alarms, power by change family, p-value calibration, effect sizes across all six covariance components, confidence/ellipse interpretation, minimum usable design, runtime and indeterminate rate.

**P2 — operational TLS health pilot**

- Use a named physical TLS unit and independently characterized check field over repeated interim assessment sessions.
- Keep the unit, firmware, profile, acquisition configuration, target lineage and reference chain explicit.
- Investigate significant changes; do not automatically modify calibration, survey evidence or building state.
- Keep Part I length accuracy and any independent calibration/verification procedure separate from Part II precision monitoring.

## 7. Standards and industry context

The NIST/OSAC TLS IPA has two distinct components: Part I evaluates length accuracy against reference values, Part II assesses overall spherical-coordinate precision, with an additional inside/outside zero-error appendix. NIST describes it as an interim in-house QA procedure between manufacturer calibrations, not a replacement for calibration. [NIST TLS IPA project portal](https://pages.nist.gov/tls-web-portal/about.html).

ISO 17123-9:2018 remains the published field-repeatability standard. ISO's current ISO/CD 17123-9 Edition 2 is under committee development and is intended to replace the 2018 edition. Its stated scope is field verification of TLS precision/repeatability for an immediate task, not a comprehensive instrument acceptance or performance evaluation. It is a complementary field-check comparator, not a substitute for reproducing the IPA Part II statistical test. [ISO/CD 17123-9 status and scope](https://www.iso.org/standard/91344.html) · [Published ISO 17123-9:2018](https://www.iso.org/standard/68382.html).

## 8. Ownership and claim ceiling

- #1607 owns survey sessions, frames, registration, centralization/change-estimate provenance and calibration evidence.
- #4589 owns measurement detectability and task-specific metrology sufficiency; it can consume this precision-health result as one input.
- #4795 owns the lighter longitudinal session-SD sentinel.
- #4807 owns faithful IPA Part II consumer reproduction and its benchmark.
- #4116 owns blind external replay; #4692 owns paired nuisance/physical-change attribution; #4088 owns building-state admission.

P0 establishes statistical-code and contract behavior on synthetic residuals only. It does not validate a real TLS, absolute accuracy, calibration traceability, legal surveying practice, task fitness, forensic reliability, building condition or return-to-service authority. P1 requires actual instrument data, independent residual production, adequate independent sessions, and competent metrology review.
