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

PIVPostProcess is a strong classifier comparator, not a displacement-UQ or end-to-end PIV solver comparator. The 2026 paper reports test F1 0.992, ROC-AUC 0.998 and log loss 0.037 on a 50,000-vector stratified test sample, and separately reports mean F1 0.990 across six additional experimental datasets. Its methods describe combining approximately 203 million vectors from twelve selected datasets, random shuffling and stratified vector-level sampling for the 250,000/50,000 train/test subsets. That point-level split is not equivalent to a pre-registered holdout of complete raw-image sessions, geometries, calibration batches and flow families. The six additional experimental datasets are stronger external-transfer evidence and should be preserved as a separate comparator cohort rather than ignored. The study's labels are expert manual filtering; that is a valuable annotation reference, not independent velocity truth. The classifier does not emit a calibrated full 2-D predictive covariance.

We therefore keep four scoreboards separate:
1. vector classifier quality against frozen expert labels;
2. displacement/velocity error against known synthetic or independent reference truth;
3. covariance calibration and proper score on independent session/geometry holdouts;
4. physically integrated functionals (near-wall gradients, mass/momentum flux, force) with their own complete uncertainty and control-volume boundaries.

## Dataset and comparator plan

A useful public raw-image starting point is the Five-Frame-Sequence Synthetic PIV dataset (Zenodo DOI 10.5281/zenodo.21732072). The record describes 12,466 five-frame 256×256 synthetic sequences with known displacement fields, nine flow scenarios, and changes in particle properties, sensor noise, laser-sheet thickness, out-of-plane motion, visibility and illumination. Its separate test archive is a natural frozen evaluation cohort; test scenarios must remain untouched during tuning.

A second high-value comparator is the 2026 PIVtools platform and its turbulent-channel validation cohort. The public dataset card describes 4,000 noisy 2048×2048 synthetic image pairs at Reτ≈1000, SNR≈8, planar and ±45° stereo geometries, and DNS-derived ground-truth statistics including mean velocity, Reynolds stresses and 95% confidence intervals. It provides a matched end-to-end planar, stereo and ensemble PIV processing benchmark, rather than a covariance-calibration ground truth by itself. The PIVtools paper describes a unified GUI/CLI, image preprocessing, calibration, ensemble correlation with direct Reynolds-stress extraction, hybrid Python/C + Dask processing, and 77% parallel efficiency on 128 cores. Its earlier public preprint reports a 10.5× speedup over leading commercial software on a 20-core 4 MP four-pass workstation benchmark, and mean-profile recovery to y+≈1 using ensemble correlation under the specified sparse/noisy setup. Treat these as source-reported performance figures to reproduce under matched hardware, settings and dataset versions—not portable universal speed claims. The dataset card is CC-BY-4.0; the software repository is GPL-3.0-or-later, so use it as an isolated external comparator and do not vendor its code into this Rust crate.

Classical uncertainty comparators must also be included in the same image cohorts: correlation-statistics uncertainty (Wieneke, 2015), image-matching uncertainty (Sciacchitano, Wieneke & Scarano, 2013), and particle-disparity / peak-ratio methods. These answer a different question from PIVPostProcess's binary valid/invalid classification. They must be scored on displacement/velocity error and covariance calibration, not ranked by F1. The PIVPostProcess experimental source data are available from the authors on request; its code is publicly available via Zenodo. Confirm the code's license and the raw-data access terms before redistribution.

## Verification, validation and uncertainty framework

The programme should align its evidence structure with established CFD V&V practice, while extending it to the measurement pipeline itself. ASME V&V 20 (2009, reaffirmed 2021) frames validation as quantifying accuracy inferred from comparisons between solution and data for specified variables at specified validation points, explicitly considering uncertainty in both solution and experimental data. NISTIR 8298 separates sources of CFD inaccuracy including physical modelling, discretization, iterative convergence/roundoff and programming/user error, and discusses numerical, input-parameter and experimental-result uncertainty. The EHD programme should therefore preserve at least four non-interchangeable terms: numerical/solver error, physical-model discrepancy, input/calibration uncertainty, and measurement/diagnostic uncertainty. The covariance kernel here addresses only one part of the measurement layer; it must not be promoted into a complete V&V procedure.

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

## Competitive benchmark scorecard

The comparison must be lane-specific. Do not combine one classifier score, one solver residual and one hardware runtime into a single leaderboard.

| Lane | Required comparators | Primary scorecard |
|---|---|---|
| Vector validity | PIVPostProcess, normalized median / universal outlier detection, adaptive Gaussian window | per-region precision/recall/F1, false-valid removal, valid retention, coherent-structure preservation |
| Displacement uncertainty | correlation statistics, image matching, particle disparity, peak ratio / SNR | displacement RMSE and bias against known truth, interval coverage/width, calibration curves |
| Full PIV processing | PIVtools, OpenPIV/PIVlab or other reproducible matched pipelines | same-image velocity/statistics error, near-wall reach, Reynolds-stress error, runtime, memory and energy |
| Covariance calibration | each method that emits covariance or predictive intervals | marginal + joint ellipse coverage, proper Gaussian score, Mahalanobis statistics, sharpness, session-cluster generalization |
| EHD physical functionals | all-surface PIV momentum, LPT/STB differential momentum, direct force balance | mass/charge/energy/momentum boundary completeness and reconciliation within predeclared combined uncertainty |

Prospective gates should be set before evaluation. The first campaign should retain the existing vector-cleaning targets (invalid F1 ≥0.97, recall ≥0.90 and false removal of reference-valid vectors ≤1% in each relevant region), then add: nominal 95% joint ellipse coverage in the 90–98% band for each region; at least 200 reference vectors and 20 truly independent high-level acquisition/dependence clusters per region; mean squared Mahalanobis distance near its 2-D Gaussian expectation of 2; and lower Gaussian negative log likelihood than matched covariance baselines without inflating ellipse area. The 20-cluster / 200-vector floors are screening thresholds only; a pre-registered power analysis must determine the final sample plan. For end-to-end flow functionals, separately target <5% bias in predeclared integrated outputs (for example mass flux or mean-profile regions) only where a traceable reference exists; boundary-layer and wake performance must be reported separately. No numerical target can replace a complete control-volume force balance.

For speed, report image pairs/s, valid vectors/s, wall-clock time, peak memory and energy per processed megapixel on the same workstation, image cohort, preprocessing, pass schedule, vector spacing and output quality. PIVtools' 10.5× commercial comparison and 77% efficiency on 128 cores are useful reference points, but the benchmark must reproduce them with matched settings before claiming a competitive result.

## Claim boundary and remaining work

The current code tests evaluator mathematics on manufactured data. It does not generate correlation planes, estimate Hessians, calibrate image-processing methods, process raw image files, establish empirical coverage, or validate any EHD device. It does not infer independence from arbitrary IDs; the caller owns the scientific grouping decision.

Pending: local cargo fmt/test/Clippy, exact-head hosted CI, external review, frozen dataset manifests and hashes, matched baseline runs, raw image-level provenance, pre-registered cluster/split plan, empirical covariance calibration against known or independent reference truth, and independent laboratory measurements.

## References and comparator records

- Ghazi Nezami, A., Johnson, B.A. & Vantassel, J.P. (2026). PIVPostProcess. *Experiments in Fluids* 67, 176. DOI: 10.1007/s00348-026-04319-7; software DOI: 10.5281/zenodo.18794853.
- Wieneke, B. (2015). PIV uncertainty quantification from correlation statistics. *Measurement Science and Technology* 26, 074002. DOI: 10.1088/0957-0233/26/7/074002.
- Sciacchitano, A., Wieneke, B. & Scarano, F. (2013). PIV uncertainty quantification by image matching. *Measurement Science and Technology* 24, 045302. DOI: 10.1088/0957-0233/24/4/045302.
- Taylor, M.T., Lawson, J.M. & Ganapathisubramani, B. (2026). PIVtools: A comprehensive open source software for particle image velocimetry. *SoftwareX* 35, 102967. DOI: 10.1016/j.softx.2026.102967; validation dataset DOI: 10.5258/SOTON/D3981.
- Five-Frame-Sequence Synthetic PIV dataset. Zenodo DOI: 10.5281/zenodo.21732072.
- ASME V&V 20-2009 (R2021), Standard for Verification and Validation in Computational Fluid Dynamics and Heat Transfer.
- Yeo, D. (2020). NISTIR 8298, A Summary of Industrial Verification, Validation, and Uncertainty Quantification Procedures in Computational Fluid Dynamics. DOI: 10.6028/NIST.IR.8298.
