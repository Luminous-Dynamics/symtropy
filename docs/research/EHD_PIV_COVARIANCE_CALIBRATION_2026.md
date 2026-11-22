# EHD-PIV-UQ-003 — two-dimensional covariance calibration across independent sessions

**Status:** standalone Rust reference authored; 10 unit tests authored; local Rust execution unavailable; exact-head CI pending.  
**Branch:** research/ehd-piv-covariance-calibration-v0-1  
**Parent:** research/ehd-piv-displacement-uncertainty-v0-1

## Why this increment exists

EHD-PIV-UQ-002 creates a local 2×2 displacement covariance from a fitted correlation-peak Hessian, then propagates it into a 2×2 velocity covariance. EHD-PIV-UQ-001 evaluates regional coverage counts supplied by an upstream pipeline. Neither increment yet evaluates whether the *full predicted covariance matrix* correctly predicts the observed two-component vector error across independent acquisition/dependence clusters.

Two failure modes matter:

1. **Marginal-versus-joint calibration.** Separate 95% intervals for streamwise and wall-normal velocity do not establish 95% coverage of the joint two-dimensional vector. A sample can be inside both marginal intervals and outside the covariance ellipse.
2. **Pseudoreplication.** PIV vectors from overlapping interrogation windows and adjacent frames are correlated. Thousands of vectors from one run must not be counted as thousands of independent calibration experiments.

## Executable kernel

The new crate evaluates reference/estimate pairs and their predicted covariance matrices. For residual vector e and covariance Σ, it computes:

- streamwise and wall-normal marginal coverage using the 95% Gaussian z threshold;
- joint 2-D ellipse coverage using eᵀΣ⁻¹e ≤ χ²(2, 0.95) = 5.991464547…;
- mean squared Mahalanobis distance (ideal expectation 2 under a correctly specified 2-D Gaussian);
- bivariate Gaussian negative log likelihood;
- mean 95% ellipse area and component interval widths.

Coverage is reported two ways: vector-count-weighted pooled coverage, and equal-weight mean coverage across declared independent dependence clusters. The latter prevents a very large, clean session from masking a small but failed session.

A cluster-level approximate 95% Student-t interval is emitted only when at least 20 independent clusters exist. It uses t=2.093 (the df=19 two-sided 95% critical value), which is conservative for larger cluster counts under the t approximation. The interval is based on independent cluster-level coverage fractions, not the raw count of overlapping vectors. Its assumptions and limitations are explicit.

## Prospective qualification gates

These are benchmark targets, not achieved results:

| Metric | Initial target |
|---|---:|
| Nominal component marginal coverage | 95% |
| Nominal 2-D joint ellipse coverage | 95% |
| Acceptable observed coverage band | 90–98% |
| Minimum reference vectors per physical region | 200 |
| Minimum independent dependence clusters per region | 20 |
| All regions | bulk, near-wall, shear layer, wake |
| Covariance validity | finite, symmetric 2×2, positive definite |
| Proper score | report Gaussian NLL; compare only on identical held-out observations |
| Sharpness | report ellipse area and interval widths alongside coverage |

The 20-cluster criterion is an initial evidence floor, not a universal sample-size theorem. A future benchmark should use power analysis based on run-to-run variability and a pre-registered confidence level.

## Relation to the PIVPostProcess baseline

PIVPostProcess reports F1 0.992, ROC-AUC 0.998 and log loss 0.037 on a 50,000-vector stratified test sample, plus mean F1 0.990 on six additional experimental datasets. The paper describes pooling approximately 203 million vectors from twelve source datasets and randomly shuffling/stratifying at vector level. Its external labels use expert manual vector filtering. This is a strong classifier result, but does not itself establish full-geometry/session-held-out transfer, independent physical truth, or calibrated 2-D velocity covariance.

We therefore keep four scoreboards separate:
1. vector classifier quality against frozen expert labels;
2. displacement/velocity error against known synthetic or independent reference truth;
3. covariance calibration and proper score on independent session/geometry holdouts;
4. physically integrated functionals (near-wall gradients, mass/momentum flux, force) with their own complete uncertainty and control-volume boundaries.

## Dataset and comparator plan

A useful public raw-image starting point is the Five-Frame-Sequence Synthetic PIV dataset (Zenodo DOI 10.5281/zenodo.21732072). The record describes 12,466 five-frame 256×256 synthetic sequences with known displacement fields, nine flow scenarios, and changes in particle properties, sensor noise, laser-sheet thickness, out-of-plane motion, visibility and illumination. Its separate test archive is a natural frozen evaluation cohort; test scenarios must remain untouched during tuning.

A second high-value comparator is the PIVtools turbulent-channel validation cohort. The public dataset card describes 4,000 synthetic 2048×2048 image pairs at Reτ≈1000, noisy imaging (SNR≈8), planar and ±45° stereo geometry, and DNS-derived ground-truth statistics. Taylor, Lawson and Ganapathisubramani's 2026 PIVtools paper reports end-to-end planar/stereo/ensemble pipelines, with instantaneous and ensemble mean-profile accuracy to different near-wall y+ limits. This is an end-to-end pipeline comparator, not a direct uncertainty-calibration win.

The PIVPostProcess paper's own experimental data are available from the authors on request; its software is public in Zenodo. The exact code/data license and access terms must be checked before redistributing or incorporating any comparator code.

## Benchmark execution sequence

1. Freeze image cohorts, hashes, metadata and split policy before running methods.
2. Keep the public FFS-S-PIV test archive sealed; use scenario-level train/test partitions and report the MHD, backward-step, cylinder, channel and wake families separately.
3. Run a conventional multi-pass cross-correlation baseline and an end-to-end PIV platform comparator on the same images and masks.
4. Run the PIVPostProcess validity classifier only on the vector fields it is designed to consume; compare classification against separately frozen labels.
5. For each method, emit predicted 2×2 covariance and evaluate it against ground-truth displacement/velocity references, not expert-validity labels.
6. Hold out complete geometry, flow family, acquisition sequence, tracer, pressure and session groups. Never randomly split adjacent vectors from the same image fields across train and test.
7. Report false removal of valid vectors and retention of coherent structures separately in near-wall, shear-layer and wake regions.
8. Propagate the calibrated covariance through gradients and surface integrals; evaluate functional errors, not just per-vector metrics.
9. For real EHD experiments, retain observer-passivity qualification and independent PIV/LPT/force-balance evidence. Synthetic ground truth does not validate the tracer's physical passivity or ion-wind thrust.

## Adversarial tests authored

- marginal intervals pass while the joint covariance ellipse fails;
- off-diagonal covariance changes Mahalanobis distance;
- one high-volume session cannot hide a failed low-volume session;
- 2,000 vectors from one session remain one independent cluster;
- singular covariance is rejected;
- duplicate sample identities are rejected;
- a missing physical region is rejected;
- non-finite velocity and invalid gates are rejected;
- Gaussian NLL and ellipse area are exposed together so coverage inflation is visible;
- empty cohorts are rejected.

## Claim boundary and remaining work

The current code tests evaluator mathematics on manufactured data. It does not generate correlation planes, estimate Hessians, calibrate image-processing methods, process raw image files, establish empirical coverage, or validate any EHD device. It does not infer independence from arbitrary IDs; the caller owns the scientific grouping decision.

Pending: local cargo fmt/test/Clippy, exact-head hosted CI, external review, ingestion of the public image cohorts, matched baseline runs, raw image-level provenance, and independent laboratory measurements.
