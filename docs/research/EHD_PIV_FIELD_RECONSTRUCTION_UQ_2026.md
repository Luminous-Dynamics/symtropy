# EHD-PIV-UQ-010 — field reconstruction, independent uncertainty calibration and selective abstention

**Status:** prospective benchmark contract; no external dataset has been processed by this contract.  
**Issue:** #6963.  
**Related:** #6928 (empirical spatial covariance), #6929 (covariance model comparison), #6936 (acquisition/reference evidence), #6937 (observer effects), #6681 (EHD model discrepancy), #6641/#6648 (force and control-volume observables).

## 1. Purpose and claim boundary

This workstream evaluates reconstruction of a velocity field from sparse PIV observations and/or complementary sensors. It is not another PIV image-correlation processor, vector classifier, covariance fitter, or EHD solver.

The research question is:

> At a declared observation rate, sensor configuration, flow family and compute budget, can a method reconstruct the declared physical field or functional, provide useful uncertainty calibrated on independent acquisitions, and abstain when its evidence does not support the requested prediction?

A reconstruction method is not successful merely because it achieves low average vector error. It must preserve relevant structures, be calibrated at the correct unit of inference, and communicate when it is outside its supported domain.

No physical EHD conclusion follows from success on synthetic images, DNS, cylinder wakes, periodic impinging jets, or another fluid regime.

## 2. Current comparators and precise scope

### 2.1 SVGP-KAN reconstruction (June 2026)

Ju, Y. S., *Uncertainty-aware flow field reconstruction using SVGP Kolmogorov–Arnold networks*, AI Thermal Fluids 6 (2026), 100034, DOI: 10.1016/j.aitf.2026.100034.

The publication compares SVGP-KAN with LSE, SAMM-RR and Kalman filtering for synthetic pulsed impinging jets, across seven PIV sampling configurations from 0.5% to 10.1%. It reports relative L2 error around 0.047 at 8.1% sampling for SVGP-KAN and SAMM-RR; reported training plus inference is approximately 50–52 seconds for SVGP-KAN versus roughly 1.5 seconds for SAMM-RR in the stated M2 MacBook Air, 8 GB case. The publication states that data and complete analysis code are available from the author on request.

The article's uncertainty scaling uses PIV training data. Our tournament must add a stricter, separate calibration cohort that is not used to fit weights, select an architecture, choose features, or tune hyperparameters. The published dataset/code must be obtained and version-pinned before claiming a faithful independent reproduction.

This paper is a strong baseline for periodic coherent-flow reconstruction. It is not a turbulent EHD result and its epistemic uncertainty is not a substitute for spatially correlated PIV measurement uncertainty or physical aleatoric variability.

### 2.2 PIVPostProcess vector validation (October 2026)

Ghazi Nezami, Johnson & Vantassel, *An open-source, physics-aware, data-driven tool for automated particle image velocimetry post-processing*, Experiments in Fluids 67 (2026), 176, DOI: 10.1007/s00348-026-04319-7.

The publication reports test-set F1 of 0.992 and ROC-AUC of 0.998, and mean F1 of 0.990 on six additional experimental datasets. Its synthetic corruption analysis also shows that subtle corruption is materially harder: at 150% of field RMS, average precision is 0.50, recall 0.78 and F1 0.62; at 2,000%, precision and recall are each about 0.91 and F1 is about 0.91.

Therefore the vector-validation lane must measure false removal of valid vectors, not only detection of bad vectors. It must include subtle errors, weak shear/vorticity structures, intermittent events, low seeding, out-of-plane loss, and class imbalance. Manual labels are a declared comparator, not infallible physical truth.

### 2.3 PIVtools processing and throughput (September 2026)

Taylor, Lawson & Ganapathisubramani, *PIVtools: A comprehensive open source software for particle image velocimetry*, SoftwareX 35 (2026), 102967, DOI: 10.1016/j.softx.2026.102967.

PIVtools is the processor/throughput comparator. Its published peak of 117 four-megapixel image pairs per second is tied to a 128-core EPYC 9654 benchmark (64 workers, two cores per worker) and a stated multipass configuration. It must not be compared with a different image size, algorithm settings, output definition, core count or storage setup.

UQ-010 compares field reconstruction and calibrated uncertainty; it does not claim to replace PIVtools' processing throughput.

## 3. Declare the estimand before scoring

The target must be explicit and immutable for each benchmark lane:

1. **Instantaneous physical field:** the velocity field at a particular unobserved time.
2. **Conditional expected field:** the expected field given the available sensor history and declared model.
3. **Ensemble/time mean:** a statistical average over a declared stationary or conditionally stationary cohort.
4. **Derived functional/statistic:** vorticity, Reynolds stress, mass/volume flow, momentum flux, or another named quantity.

Do not score a conditional-mean prediction against an instantaneous turbulent field as though the entire residual were measurement error. Do not treat the ensemble spread of a genuinely fluctuating flow as instrument noise.

## 4. Uncertainty ontology

Keep these sources distinct in the report:

- **Physical aleatoric variability:** unresolved or intrinsically variable flow behavior under the declared estimand.
- **PIV measurement uncertainty:** image/correlation/vector uncertainty, including spatial and temporal dependence.
- **Reference uncertainty:** uncertainty in hot-wire, LDA, LPT, pressure, force-balance, DNS or other declared reference path.
- **Shared/common-mode uncertainty:** calibration, timing, registration, geometry or environmental effects shared across paths.
- **Reconstruction epistemic uncertainty:** uncertainty induced by finite training data and model parameters.
- **Model-form discrepancy:** inadequacy of the reconstruction model or its assumed physics.
- **Numerical approximation error:** truncation, solver, quadrature and finite-precision effects.
- **Observer/intervention effect:** physical change induced by tracer, seeding, illumination, probe or diagnostic configuration.

These terms may be dependent. Do not combine by root-sum-square without a justified covariance model. A calibrated surrogate confidence interval does not establish PIV measurement accuracy. A calibrated field residual does not establish a complete engineering control-volume balance.

## 5. Split, calibration and test design

### 5.1 Dependence-aware grouping

Use the UQ-008 dependency graph. Group by the highest shared-cause unit relevant to the claim: campaign, session, geometry, sensor/reference calibration, synchronization source, tracer batch, operating regime and acquisition cluster.

No random vector, image-pair, frame, spatial-point, or overlapping temporal-window split may allow a shared dependence component to cross model-fit, model-selection, uncertainty-calibration, and final-test partitions.

### 5.2 Four disjoint roles

Freeze these partitions before training:

- **Training:** fit model weights, POD basis, GP inducing variables, feature transforms and preprocessing statistics.
- **Model selection:** choose architecture, features, regularization, sampling strategy and hyperparameters.
- **Uncertainty calibration:** calibrate intervals/sets after the predictor is frozen; it must not fit predictor weights or choose the winning model.
- **Sealed evaluation:** used once for final scoring, never for choosing a model, threshold, feature, covariance family or interval scaling factor.

Out-of-domain challenge groups are separately frozen and must not be used to set the in-domain calibration threshold.

### 5.3 Field-level coverage

For a field with reference `u_ref(x)`, prediction `u_hat(x)`, and pre-calibration scale `s(x)`, one candidate simultaneous nonconformity score is:

`R_a = max_{x in declared evaluation domain} ||u_ref,a(x)-u_hat,a(x)|| / max(s_a(x), s_floor)`.

One complete independent acquisition/dependence cluster contributes one score. A split-conformal quantile then produces a field-wide envelope, subject to the declared exchangeability assumption. Pointwise marginal coverage and simultaneous whole-field coverage are separate metrics.

The domain, mask policy, scale floor, vector norm, normalization, region treatment, confidence level and calibration cohort must be frozen. Missing vectors cannot silently disappear from the maximum.

Nineteen independent calibration units is the mathematical finite-rank minimum for a finite split-conformal 95% order statistic; it is not an adequate general-purpose benchmark sample. Coverage confidence intervals must use independent evaluation clusters, not pixels or vectors. Under a simple binomial approximation, approximately 456 independent test clusters give a 95% confidence half-width near two percentage points when true coverage is 95%; the actual plan must account for dependence, heterogeneity and exact intervals.

## 6. Baselines and matched privileges

Required baselines, where the data modality supports them:

- persistence and nearest-time / linear interpolation;
- POD with linear regression;
- LSE;
- SAMM-RR;
- linear Kalman filter and smoother;
- SVGP-KAN, or a source-faithful reproduction once source code/data are available;
- covariance-aware GP or reduced-rank GP reconstruction;
- any Symthaea/Symtropy/HDC/CfC challenger.

All methods must receive the same observations, sensor history, calibration data, train/test partitions, flow metadata, compute budget policy and inference cutoff. Report any additional information privilege (for example, phase, forcing frequency, geometry, auxiliary sensors or future measurements) explicitly. Methods with different privileges belong on separate tracks.

HDC/CfC is an experimental challenger, not a privileged baseline. It earns credit only by demonstrating incremental value against the strongest conventional baseline under matched observations, compute, parameter budget and calibration protocol.

## 7. Scoreboards

### A. Field reconstruction

- relative L2 and normalized RMSE;
- pointwise median, P90, P95 and maximum errors by declared region;
- profile error and wake/shear-layer position/amplitude;
- vorticity, divergence, spectra and structure-function error;
- temporal phase, event timing and lag correlation for time-resolved tasks;
- mean, Reynolds stress, skewness and flatness error only when these are the declared estimands.

### B. Uncertainty quality

- pointwise 50/80/90/95/99% coverage;
- simultaneous whole-field coverage;
- functional coverage;
- calibration slope/intercept and reliability curves;
- uncertainty growth between observation times;
- CRPS or weighted interval score for scalar predictive marginals;
- energy score and variogram score for spatial/multivariate predictive distributions;
- interval width or confidence-region volume;
- cluster-bootstrap confidence intervals and worst-slice reporting.

Coverage without sharpness is insufficient. Overly wide intervals must not be rewarded.

### C. Selective prediction and abstention

- risk-versus-coverage curves;
- false-safe rate on out-of-domain and adversarial controls;
- abstention precision/recall and detection delay;
- recovery only after a fresh, supported evidence state;
- performance by flow family, region, SNR, sampling ratio, sensor dropout and protocol revision;
- explicit `OutOfEnvelope`, `NotIdentifiable` and `InsufficientEvidence` outcomes.

### D. Vector validity (separate from reconstruction)

- exact matched-cohort reproduction of PIVPostProcess F1 and ROC-AUC;
- precision/recall for valid and invalid classes;
- valid-vector retention and false-removal rate;
- performance on subtle/low-amplitude errors;
- preservation of shear layers, vortices, wakes and intermittent coherent structures;
- calibration of rejection probabilities and risk at each chosen rejection rate.

### E. Engineering functionals

- mass/volume flow, momentum flux, vorticity, Reynolds stresses and other declared quantities;
- coverage, interval width and bias for each functional;
- geometry, density, pressure, boundary, mask and missing-region uncertainty;
- agreement with separately qualified physical reference or force-balance evidence.

### F. Systems performance

- p50/p95/p99 latency and throughput;
- peak memory, energy, bytes moved and model size;
- end-to-end cost including image preprocessing, feature extraction, model fitting, uncertainty calibration, fallback, I/O and receipt generation;
- quality-versus-compute Pareto frontiers at matched hardware and output semantics.

## 8. Dataset admission register

Dataset rights, version and scientific role are separate admission gates.

| Dataset | Role | Current admission state | Claim boundary |
|---|---|---|---|
| FFS-S-PIV, DOI 10.5281/zenodo.21732072 | Synthetic five-frame images and known displacement | Candidate; verify exact payload licence/version before redistribution | Software/image processing verification, not physical flow validation |
| INRAE Re=3900 2026 experimental PIV, DOI 10.57745/DHJXM6 | 10,000 non-time-resolved 2D2C fields plus ensemble statistics | Rights review required; public file access is not a licence | Experimental distribution/structure corpus; no known truth residuals |
| INRAE Re=3900 Eulerian/Lagrangian DNS, DOI 10.15454/GLNRHK | DNS velocity/pressure and particle trajectories | Candidate under Etalab Open License 2.0; pin version/files/hashes | DNS/particle-response verification, not experimental PIV or EHD evidence |
| SVGP-KAN paper cohort, DOI 10.1016/j.aitf.2026.100034 | Matched reconstruction comparator | Author request required for code/data; not yet reproduced | Synthetic periodic impinging-jet result only until admitted |
| EHD tracer/observer experiments | Physical EHD validation | Separate future physical cohort | Only supports tested device, regime and observer conditions |

No payload may enter a public scoreboard until rights, exact version, file inventory, byte counts, hashes, terms of use and admissible claim scope are recorded. Do not mirror or redistribute rights-pending files.

## 9. Prospective acceptance criteria

These are research targets, not achieved results:

1. Faithfully reproduce the SVGP-KAN/SAMM-RR/LSE result on the source's exact cohort before claiming improvement.
2. Keep model-fit, model-selection, uncertainty-calibration and final evaluation groups disjoint at the dependency-graph level.
3. Achieve nominal 95% whole-field and functional coverage within predeclared confidence bounds, with useful sharpness.
4. Report performance by region and flow family; no aggregate score may conceal a materially failing subgroup.
5. Zero false qualification on the frozen out-of-domain, reference-mutation and observer-shift controls.
6. At matched observation and compute budgets, demonstrate at least one of:
   - materially lower reconstruction error without worsening uncertainty calibration;
   - materially smaller uncertainty regions at matched empirical coverage;
   - lower selective risk at matched useful coverage and availability.
7. Reproduce the PIVPostProcess classifier baseline on the exact matched data/labels before claiming classifier superiority; additionally show valid-vector retention and low-corruption performance.
8. No synthetic, DNS, cylinder-wake or periodic-jet success is promoted to a physical EHD result.

## 10. Research opportunities beyond current baselines

- **Decision-aligned sensing:** choose the next PIV sampling time or sensor placement to minimize expected uncertainty in the engineering quantity, not only field RMSE.
- **Coverage-aware reconstruction:** jointly evaluate spatial dependence, temporal dependence and whole-field uncertainty rather than attaching independent pixel intervals.
- **Structure-preserving abstention:** preserve uncertain coherent structures for review rather than deleting them as outliers.
- **Multi-fidelity assimilation:** combine sparse PIV, pressure probes, LPT, hot-wire/LDA, DNS priors and physics models with explicit privilege and covariance accounting.
- **Observer-aware active learning:** avoid designs where the tracer or sensor perturbs the EHD state enough to invalidate the target regime.
- **Model-falsification:** include plausible but wrong dynamics, phase drift, unseen frequencies, geometry changes, missing sensors, timing shifts, common-mode calibration errors and nonstationary turbulence.
- **Human review value:** rank field regions by expected reduction in decision risk if manually reviewed, not by generic uncertainty magnitude.

## References

- Ju (2026), *Uncertainty-aware flow field reconstruction using SVGP Kolmogorov–Arnold networks*, DOI: 10.1016/j.aitf.2026.100034.
- Ghazi Nezami, Johnson & Vantassel (2026), *An open-source, physics-aware, data-driven tool for automated PIV post-processing*, DOI: 10.1007/s00348-026-04319-7.
- Taylor, Lawson & Ganapathisubramani (2026), *PIVtools*, DOI: 10.1016/j.softx.2026.102967.
- FFS-S-PIV dataset, DOI: 10.5281/zenodo.21732072.
- INRAE experimental Re=3900 PIV data (2026), DOI: 10.57745/DHJXM6.
- INRAE Re=3900 Eulerian/Lagrangian dataset (2021), DOI: 10.15454/GLNRHK.
- Sciacchitano & Wieneke (2016), *PIV uncertainty propagation*, DOI: 10.1088/0957-0233/27/8/084006.
- NISTIR 8298 (2020), DOI: 10.6028/NIST.IR.8298.
