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

### 2.4 Generative neural operators and conditional diffusion (March 2026)

Oommen et al., *Learning turbulent flows with generative models for super resolution and sparse flow reconstruction*, Nature Communications 17, 3707 (9 March 2026), DOI 10.1038/s41467-026-70145-4, adds two important SOTA comparator families:

**Forward operator / spatio-temporal super-resolution and forecasting:** adversarially-trained neural operator (adv-NO). On the paper's 2D Schlieren-jet super-resolution case, the vanilla neural operator has field NRMSE 0.0542 and energy-spectrum NRMSE 0.1601; adv-NO has field NRMSE 0.0662 but energy-spectrum NRMSE 0.0109 at the same reported 0.010 s single-sample inference cost on an H100. This is approximately 14.7× lower spectrum error but slightly worse pointwise field error. The comparison makes the key point: a method can be worse by field NRMSE but much better in multiscale/spectral fidelity. These numbers are task-specific, not universal.
 
For 3D homogeneous-isotropic-turbulence forecasting at (Re_\lambda=90), the same paper reports field NRMSE 0.0326 / spectrum NRMSE 0.0865 for vanilla NO and 0.0265 / 0.0235 for adv-NO. Its reported per-sample inference time is 0.082 s for both on an H100; the diffusion comparator GenCFD is 7.804 s per sample at the same listed parameter count, under the paper's matched setting. The authors also report phase alignment and Q–R velocity-gradient invariant statistics. The paper notes that divergence-penalising a conventional L2-trained NO did not correct spectral bias.

**Inverse sparse-observation reconstruction:** conditional score-based diffusion (EDM) and conditional GAN. The paper uses 3D DNS cylinder-wake data at (Re=11{,}000), with 150 training, 18 validation and 17 test snapshots. Its “zero-shot” result is transfer to unseen masks/layouts within the same geometry, not generalisation to unseen geometries. Inputs are sparse PTV-like velocity observations generated from DNS fields, not raw PIV images. On its full-domain masking test, the conditional diffusion model remains effective at 99% missingness, whereas its conditional GAN baseline fails at the extreme mask. The diffusion model is a strong inverse-problem comparator, but iterative sampling cost, posterior calibration and experiment-to-DNS domain shift must be measured separately.

These methods belong on different lanes: adv-NO is a forward operator surrogate for super-resolution/forecasting; conditional diffusion samples an inverse conditional posterior from partial observations. Do not rank them in one leaderboard unless the estimand, observed information, data, compute budget and inference semantics are matched.

## 2.5 Probabilistic turbulence priors, statistical computation and temporal reconstruction (2026)

The field is moving beyond deterministic super-resolution toward models that estimate *conditional distributions* of flow realizations. These methods answer different questions and need separate benchmark tracks.

### GenCFD — statistical forward computation / forward UQ

Raonić et al., *Generative AI for efficient statistical computation of fluids*, Nature Communications 17, 9846 (17 August 2026), DOI 10.1038/s41467-026-76390-x, introduces GenCFD, a conditional score-based diffusion approach for statistical computation of 3D turbulent flow.

The paper evaluates five challenging flow benchmarks, including canonical turbulence, engineering flows and atmospheric science. Its target is the conditional distribution, not only its conditional mean: results include conditional mean/variance, point PDFs, energy spectra, structure functions, higher central moments through order five and Wasserstein comparison of conditional distributions. It reports approximately one to five orders of magnitude speedup versus conventional CFD solvers, with benchmark samples generated in under four seconds on an RTX 4090. These are paper-specific benchmark/runtime claims and must be reproduced under their original settings before direct comparison.

The model code is public under Apache-2.0 at [camlab-ethz/GenCFD](https://github.com/camlab-ethz/GenCFD); representative trajectories and dataset references are described in the paper's data availability. It is a strong comparator for **forward statistical prediction / conditional forward UQ**, not automatically a sparse-PIV inverse solver. Do not compare a statistical sample generator against a deterministic field reconstruction using only one RMSE.

### Sparse-observation inverse diffusion

Two 2026 studies provide complementary baselines:

- *Guiding diffusion models to reconstruct flow fields from sparse data*, Physics of Fluids 38, 015112 (8 January 2026), DOI 10.1063/5.0304492, introduces masked/guided diffusion for sparse observations. In its 2D Kolmogorov-flow 5% observation example, the reported masked-diffusion RMSE is 0.13±0.02, sparse-location P-RMSE 0.10±0.01, physical-residual metric 0.02±0.03, structural similarity loss 0.05±0.01 and energy-spectrum error 0.26±0.05. The paper reports better performance than its listed supervised and prior diffusion baselines on that task. These values are not comparable to another dataset or EHD flow without exact reproduction.
- Oommen et al. (2026), described above, use conditional EDM diffusion for sparse PTV-like observation reconstruction in a 3D cylinder-wake DNS case. Their unseen-mask/layout result is same-geometry generalization, not unseen-geometry transfer or raw-image PIV.

These should be measured using both field/structure metrics and **measurement consistency at observed locations**, while separately reporting posterior sample count, sampling steps, sampling latency, conditional coverage and sharpness. Sparse observed values cannot be treated as error-free if they themselves carry PIV covariance.

### Temporal super-resolution and autoregressive conditional diffusion

- *Physics-regularized residual diffusion model for temporal super-resolution reconstruction of turbulent flows*, Computers & Fluids 320 (15 November 2026), 107277, DOI 10.1016/j.compfluid.2026.107277, conditions on two temporally sparse endpoint velocity fields. It constructs a linear interpolation base and learns the normalized DNS residual at intermediate times. Add this as an endpoint-conditioned temporal-super-resolution baseline, with phase/timing, interpolation residual, energy transfer and temporal coherence metrics.
- Kohl, Chen & Thuerey, *Benchmarking autoregressive conditional diffusion models for turbulent flow simulation*, Neural Networks 199 (2026), 108641, DOI 10.1016/j.neunet.2026.108641, benchmarks 2D incompressible, transonic and isotropic-turbulence scenarios. It explicitly evaluates accuracy, posterior sampling, spectral behavior, temporal stability and flow-parameter generalization. Diffusion methods show accuracy/stability advantages on the declared cases, while established deterministic architectures are faster at inference. This is a useful long-rollout comparator, not direct PIV reconstruction.

### General research direction: reusable conditional priors

Wang, *Generative priors for spatiotemporal turbulence: Toward conditionable and scalable flow foundation models*, Physical Review Fluids 11, 090502 (21 September 2026), DOI 10.1103/qbw5-qqmg, is a Perspective advocating diffusion/flow-matching priors reusable for generation, inverse conditioning, assimilation, super-resolution, restoration and solver-constrained inference. It explicitly identifies distributional fidelity, non-memorization, data consistency, posterior calibration, physical admissibility and large-domain coherence as validation criteria. This is a strategic research agenda, not itself evidence of one model's performance.

### Separate these tracks in all leaderboards

1. **Forward conditional statistics / forward UQ:** GenCFD, conditional distribution of future/unknown flow given declared conditions. Metrics: mean/variance/PDF/higher moments, Wasserstein distribution distance, spectra, structure functions, conditional coverage/sharpness, calibrated functionals and rollout cost.
2. **Inverse sparse observation reconstruction:** masked/guided diffusion, conditional EDM, conditional GAN, GP/LSE/SAMM-RR. Metrics: measurement consistency, field/structure/spectral error, posterior calibration and cost.
3. **Temporal super-resolution / interpolation:** endpoint-conditioned residual diffusion, temporal operators and deterministic interpolation. Metrics: between-observation error, timing/phase, temporal coherence, spectrum/structure-function transfer and event detection.
4. **Autoregressive rollout / parameter generalization:** conditional diffusion vs neural operators and CFD baselines. Metrics: long-horizon stability, drift, spectra, posterior statistics and out-of-training-range parameter performance.

A common metric suite may be shared, but the estimand, information available to the model, data-generating regime, inference semantics and compute budget must be frozen per track.

### 2.6 Synthetic image generation is a separate benchmark from reconstruction

**SynthPix** (Terpin et al., *SoftwareX* 34, 102642, June 2026; DOI 10.1016/j.softx.2026.102642) is the current high-throughput open-source generator comparator. Its MIT-licensed public repository is v0.4.0 at pinned commit `0effe6e70b77d0d5c97348d8220e74d31bfaee2e`. It uses JAX batching/accelerators, streams image pairs and ground-truth flow fields, and supports HDF5/MAT/NPY flow sources. It exposes seeding density, particle diameter/intensity ranges, particle shape correlation `rho`, per-frame particle appearance variability, particle hide probabilities, noise, masks and histogram matching. The paper reports orders-of-magnitude throughput improvements over selected alternative generators. In its own synthetic benchmark, after tuning rendering parameters to reproduce the relevant image conditions, mean/max relative EPE discrepancy was 0.001/0.005. Treat this as an external reproduction target, not a general accuracy guarantee.

The paper describes its v0.1 geometry/sensor model as simplified and orthogonal, with non-orthogonal camera projection and finite pixel fill factor deferred; it also describes multi-camera volumetric synthesis as future work. Check the exact pinned repository release before assuming any such limitation has changed in v0.4.0.

**Division of labour:** SynthPix is the high-throughput synthetic generation engine for large nuisance sweeps, hard-example mining and training. The new Rust observation operator in [PR #7259](https://github.com/Luminous-Dynamics/symtropy/pull/7259) is a transparent deterministic reference kernel for small analytic controls (finite-exposure RK2 motion, explicit Gaussian sheet attenuation, PSF, fixed per-pixel noise, gain/offset, quantization and fail-closed bounds). Do not compare their throughput as though they were equivalent implementations. Compare semantic coverage and known-output behavior first; compare speed only on matched hardware/configurations.

Keep three tracks separate:
1. **Generator fidelity:** synthetic image distributions and parameters vs analytic or measured camera data.
2. **Estimator performance:** PIV reconstruction accuracy on a given frozen image corpus.
3. **End-to-end predictive validity:** whether benchmark performance on generated data predicts held-out performance on real image pairs.

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

Nineteen independent calibration units is the mathematical finite-rank minimum for a finite split-conformal 95% order statistic; it is not an adequate general-purpose benchmark sample. Coverage confidence intervals must use independent evaluation clusters, not pixels or vectors. Under a simple binomial approximation, approximately 457 independent test clusters give a 95% confidence half-width near two percentage points when true coverage is 95%; the actual plan must account for dependence, heterogeneity and exact intervals.

## 6. Baselines and matched privileges

Required baselines, where the data modality supports them:

- persistence and nearest-time / linear interpolation;
- POD with linear regression;
- LSE;
- SAMM-RR;
- linear Kalman filter and smoother;
- SVGP-KAN, or a source-faithful reproduction once source code/data are available;
- covariance-aware GP or reduced-rank GP reconstruction;
- adversarially trained neural operator (adv-NO) for forward super-resolution/forecasting;
- conditional score-based diffusion (EDM) and conditional GAN for sparse-observation inverse reconstruction;
- any Symthaea/Symtropy/HDC/CfC challenger.

All methods must receive the same observations, sensor history, calibration data, train/test partitions, flow metadata, compute budget policy and inference cutoff. Report any additional information privilege (for example, phase, forcing frequency, geometry, auxiliary sensors or future measurements) explicitly. Methods with different privileges belong on separate tracks.

HDC/CfC is an experimental challenger, not a privileged baseline. It earns credit only by demonstrating incremental value against the strongest conventional baseline under matched observations, compute, parameter budget and calibration protocol.

## 7. Scoreboards

### A. Field reconstruction

- relative L2 and normalized RMSE;
- pointwise median, P90, P95 and maximum errors by declared region;
- profile error and wake/shear-layer position/amplitude;
- vorticity, divergence, band-wise energy-spectrum error, high-wavenumber tail-energy retention, structure-function error and Q–R velocity-gradient invariant PDF error when 3D gradients are available;
- temporal phase, event timing and lag correlation for time-resolved tasks;
- between-observation temporal super-resolution error, rollout drift and long-horizon stability;
- mean, Reynolds stress, skewness and flatness error only when these are the declared estimands.

### B. Uncertainty quality

- pointwise 50/80/90/95/99% coverage;
- simultaneous whole-field coverage;
- functional coverage;
- calibration slope/intercept and reliability curves;
- uncertainty growth between observation times;
- CRPS or weighted interval score for scalar predictive marginals;
- energy score and variogram score for spatial/multivariate predictive distributions;
- Wasserstein distance and conditional point-PDF scores for statistical forward generators;
- mean/variance/high-order-moment (through declared order) fidelity and structure-function error;
- posterior sample diversity, conditional measurement consistency and posterior calibration for inverse diffusion;
- conditional measurement-consistency residual and phase alignment near observed regions for generative posterior samples;
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

## 7.1 Periodic-flow acquisition: phase coverage × independent replication

Periodic and quasi-periodic flows need an acquisition-design scoreboard that does not collapse temporal sampling to one percentage.

Ju (2026), using a synthetic pulsed impinging jet with 160 forcing cycles, identifies a consequential interaction between phase diversity and per-phase replication (DOI: 10.1016/j.aitf.2026.100034). Its reported configurations include:

| Sampling | Distinct forcing phases | Samples per phase | SVGP-KAN/LSE interpolation L2 |
|---|---:|---:|---:|
| 0.50%, non-coprime | 1 | 10 | 0.089 |
| 0.55%, coprime | 11 | 1 | 0.237 |
| 1.0%, non-coprime | 1 | 20 | 0.068 |
| 8.1%, coprime | 81 | 2 | 0.047 |
| 10.1%, coprime | 101 | 2 | source reports best regime; reproduce exact table/config before use |

The publication reports that 0.55% coprime sampling with 11 distinct phases and only one observation per phase performs substantially worse than 1.0% non-coprime sampling with 20 repetitions at one phase. At 8.1%, 81 phases with two observations per phase performs substantially better. This does not mean phase diversity is harmful; it means phase diversity without sufficient replication can be unstable. The paper identifies two samples per phase as a useful threshold **for its tested synthetic impinging-jet configuration**, not a universal fluid law or general guarantee.

### Design variables and diagnostics

For predeclared forcing period and phase origin, convert each measurement timestamp to φ ∈ [0, 1). Partition the cycle into fixed phase bins before candidate evaluation. Report at least:

- nominal sample fraction and total snapshots;
- occupied phase bins and phase coverage fraction;
- occupancy histogram, singleton-bin fraction and empty-bin fraction;
- minimum and median **independent-cycle** replication per occupied phase bin;
- maximum cyclic phase gap;
- phase-occupancy entropy (normalised by log of total bins);
- number of independent cycles/session clusters;
- phase-estimation error and clock/forcing uncertainty when phase is inferred rather than supplied.

Multiple frames from one forcing cycle do not automatically count as independent replication. Dependence-cluster IDs must be retained; use distinct cycles/acquisition clusters for independent-replication counts. Evaluate phase coverage separately within model-fit, model-selection, calibration and final-test roles so no cycle crosses partitions.

### Matched design tournament

At a fixed total acquisition and compute budget compare:
1. uniform phase-locked sampling;
2. random phase sampling;
3. coprime / phase-diverse sampling;
4. non-coprime / repeated-phase sampling;
5. stratified phase coverage with a minimum independent-cycle repeat count;
6. adaptive phase selection driven by a frozen predictor's expected reduction in downstream functional risk.

Do not let a policy inspect future reference fields when choosing a phase or acquisition time. Known forcing phase and unknown/estimated phase are different information-privilege tracks. In the unknown-phase track, phase estimation must be fitted using training/selection data only and its error included in the budget and uncertainty report.

### Reconstruction and uncertainty metrics

Report observed-time and between-observation-time errors separately, plus worst-phase and phase-bin stratified error. For the declared estimand, include field/functional relative error, structure preservation, pointwise and simultaneous whole-field uncertainty coverage, sharpness, phase-wise calibration, and generalization gap (interpolation error minus observed-time error). A model cannot claim success by fitting observed snapshots while failing between them.

### Initial prospective controls

- Reproduce the paper's seven sampling schedules and its low/high-noise conditions after obtaining its source code/data; report phase bins, replication and interpolation-time error.
- Include an adversarial one-sample-per-phase coprime schedule and repeated-phase non-coprime schedule at matched total snapshots.
- Include exact phase-clock offsets, jitter, drift and forcing-frequency mismatch; invalid phase metadata must produce abstention or a separate estimated-phase track.
- Use two independent cycles per occupied bin as a first **candidate design constraint** for the paper-reproduction lane, not an acceptance rule for other flows.
- Search the Pareto frontier between phase coverage, independent replication, acquisition count and functional uncertainty instead of optimising the count of unique phases alone.

### Executable V0 reference kernel

The Rust crate `tools/ehd-piv-phase-design/` implements the phase-bin scorecard. Each observation carries a phase fraction and a dependence-cluster ID. The evaluator reports occupied/empty/singleton bins, independent-cycle replication, median/minimum replication, normalized occupancy entropy, maximum cyclic phase gap and explicit failure reasons. Repeated frames from one cycle increase raw observation count but cannot increase independent-cycle replication. The threshold is supplied in the benchmark config; the code does not encode a universal two-repeat rule. The crate contains eight manufactured unit tests, including the source-inspired one-observation-per-phase, repeat-at-one-phase and 81-phase/two-cycle designs.

**Claim boundary:** these source results are synthetic periodic impinging-jet evidence. They do not establish an EHD threshold, and no phase-known reconstruction result may be promoted to a physical EHD conclusion without matched acquisition and independent validation.

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
- **Spectrum-aware and distribution-aware objectives:** evaluate low- and high-wavenumber energy, gradient tails, phase alignment and Q–R statistics separately; never allow adversarial realism to substitute for pointwise measurement consistency or calibrated posterior uncertainty.
- **Multi-fidelity assimilation:** combine sparse PIV, pressure probes, LPT, hot-wire/LDA, DNS priors and physics models with explicit privilege and covariance accounting.
- **Observer-aware active learning:** avoid designs where the tracer or sensor perturbs the EHD state enough to invalidate the target regime.
- **Model-falsification:** include plausible but wrong dynamics, phase drift, unseen frequencies, geometry changes, missing sensors, timing shifts, common-mode calibration errors and nonstationary turbulence.
- **Conditional-distribution qualification:** test non-memorization, measurement consistency, posterior calibration, physical admissibility and large-domain coherence; producing plausible samples is not sufficient.
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
