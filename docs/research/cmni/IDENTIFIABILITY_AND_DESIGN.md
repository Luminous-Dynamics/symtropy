# CMNI identifiability and measurement-aware experiment design

**Status:** research design, v0.1  
**Companion implementation:** \`research/cmni/reference-kernel/src/experiment_design.rs\` and \`response_uncertainty.rs\`  
**Scope:** abstract design qualification and analysis of published beam-driven D–D hydride experiments. This document is not an experimental operating procedure.

## Why this is the next architectural step

The central CMNI problem is not merely fitting a screening curve. Different combinations of beam composition, stopping, target occupancy, defects, transport, reaction model and detector response can produce similar observed count curves. A good fit therefore need not identify a physical mechanism.

The research system should answer, in order:

1. Can the chosen observation channels distinguish the candidate parameters or hypotheses at all?
2. Which candidate measurement or intervention adds information in a direction that is currently unobserved?
3. Does the prediction remain calibrated after detector, background, beam and material-state uncertainties are propagated?
4. Does the result transfer to a held-out energy, sample, material, detector or campaign?

A parameter estimate must not be reported as identified when the observation geometry has a null or nearly-null direction.

## Layer A — local identifiability

For independent Poisson count channels with expected counts \(\mu_i(\theta)\), the local Fisher information is:

\[
\mathcal I(\theta) = J(\theta)^T
\operatorname{diag}(1/\mu_i(\theta))J(\theta),
\qquad
J_{ij}=\partial\mu_i/\partial\theta_j.
\]

The current research kernel accepts caller-supplied expected counts and Jacobians and computes this matrix. It combines independent run contributions and reports eigenvalues, effective rank, log determinant and condition number. It also reports directed pairwise Poisson KL divergences between discrete candidate predictions, conditional on fixed response and independent count channels.

Interpretation rules:

- **Rank first:** a rank-deficient matrix means at least one local parameter combination is not identified by that design.
- **Scale parameters:** condition numbers and eigenvalue tolerances depend on parameter units. Use documented, dimensionless parameter scaling before comparing designs.
- **D-optimality second:** log determinant is useful only after rank is full and the same parameterization/scaling is used.
- **Local is not global:** Fisher information does not prove global identifiability, exclude multimodal likelihoods, or establish mechanism truth.
- **Covariance model matters:** the current Poisson Fisher helper is conditional on a fixed response and independent Poisson count channels. For response-calibration uncertainty or other shared nuisances, use a hierarchical likelihood or a covariance-aware model; do not feed an approximate diagonal model into this helper and call it fully uncertainty-aware.

## Layer B — response calibration as a random object

A detector response matrix should not be treated as exact merely because it came from a simulation or calibration run. Different calibration/bootstrap draws can move counts between pulse-height bins and create cross-bin covariance.

For response realization \(R\), expected count vector \(\lambda(R)\), and conditionally independent Poisson counts:

\[
E[Y]=E_R[\lambda(R)]
\]
\[
\operatorname{Cov}(Y)=
E_R[\operatorname{diag}(\lambda(R))]
+\operatorname{Cov}_R(\lambda(R)).
\]

The implemented ensemble reference uses the average conditional Poisson variance plus the sample covariance of conditional means. It also computes the finite-ensemble log predictive probability as a log-sum-exp over conditional Poisson likelihoods, with equal fixed weights for the supplied response realizations. This preserves more of the mixture structure than a Gaussian moment approximation, but it is not a general continuous hierarchical model. Background is fixed; background calibration uncertainty, live-time/dead-time uncertainty, pileup, threshold and PSD uncertainty remain separate required model dimensions.

This is a forward uncertainty primitive, not detector calibration, Geant4, scintillation simulation, PSD reconstruction or a source-spectrum inversion.

## Layer C — experiment selection

Candidate interventions should be represented as observation-prediction alternatives, not narrative labels. Each design candidate declares:

- the parameterization and scaling;
- expected observation vector and Jacobian or full likelihood;
- observation covariance / nuisance model;
- cost, practical feasibility and sample constraints;
- a set of hypotheses that it is intended to separate;
- a predeclared holdout and falsifier.

Discrete mechanism alternatives also need a model-discrimination score. For fixed-response, independent Poisson predictions, the directed KL divergence is the exact expected log-likelihood ratio under one candidate. The current kernel exposes a pairwise directed KL matrix; it does not turn that matrix into posterior model probabilities or account for nuisance uncertainty.

### Layer D — multifidelity expected-information calculations

CMNI naturally admits a fidelity hierarchy: closed-form screening and reduced transport references; calibrated surrogate or reduced-order models; specialist stopping/material solvers; and the full coupled high-fidelity forward model. A lower-fidelity model is useful only when its paired predictions correlate with the high-fidelity quantity under the same sampled inputs.

The current kernel implements a scalar nested approximate-control-variate (ACV) estimator:

[
\widehat{\mu}_{ACV}=\overline{H}_{n_H}+\beta(\overline{L}_{n_H}-\overline{L}_{n_L}),
\qquad n_L>n_H,
]

where the first n_H low-fidelity samples are paired with the high-fidelity samples and the coefficient beta is frozen independently (for example, from a pilot sample). The nested-sample variance identity is also reported. This is a reusable statistical primitive, **not** the full multifidelity expected-information-gain estimator from the literature. It does not choose beta, allocate compute budgets, estimate EIG, or bound high/low model discrepancy.

A future CMNI EIG estimator should compare a single-fidelity nested Monte Carlo baseline with multifidelity ACV under an identical total compute budget. It must report bias against a tractable oracle, estimator RMSE, variance reduction, high/low evaluation counts, wall-clock cost, and performance as correlation between fidelities degrades. The recent Coons–Huan estimator reports unbiased high-fidelity EIG estimation and 1–2 orders of magnitude variance reduction in its demonstrated nonlinear and turbulent-flow examples. This is an external research target, not a result of the current kernel.

### Layer E — robust finite-scenario design scoring

The current kernel also scores candidate designs over a finite, declared set of nuisance scenarios and selects the maximin design. The scenario set and utilities are caller-supplied; this is a transparent baseline for stress-testing candidate designs, not a guarantee over every possible nuisance realization. More sophisticated robust Bayesian design methods for nonlinear PDE inverse problems use worst-case formulations and uncertainty-aware expected information gain; CMNI should eventually compare its finite-scenario baseline with such methods.

A future design optimizer should compare multiple objectives:

- **Rank / E-optimality:** improve the smallest information eigenvalue and eliminate blind directions.
- **D-optimality:** maximize log determinant after full rank is obtained.
- **A-optimality:** reduce average marginal variance.
- **Model discrimination:** maximize expected separation between competing predictive distributions, for example expected log predictive density or a preregistered KL-based criterion.
- **Robust design:** optimize the worst or lower-quantile information score over plausible nuisance states, not one convenient nominal point.
- **Cost-adjusted value:** report information gain per unit of time, detector use, sample consumption and financial cost.

Do not optimize only the fitted screening parameter. The candidate model must include the competing nuisance mechanisms that can mimic it.

## High-information measurement families

The following are study-design concepts, not laboratory operating instructions.

| Family | Competing explanations separated | Key qualification |
|---|---|---|
| Species-resolved incident beam characterization | Nominal voltage / beam spread versus D+ / D2+ mixture and fragment-energy alternatives | Independent species and energy evidence; current and deuteron-flux bookkeeping |
| Spatial fluence map + independent total-current reference | Total current versus local flux-density / footprint effects | Map integral reconciles with independent Faraday-cup total within declared uncertainty |
| Pre/post depth-resolved D profile | Uniform loading versus near-surface occupancy / transport | Isotope-specific response, depth PSF and lateral PSF kept distinct |
| Defect-sensitive pre/post characterization | Defect/trap hypothesis versus occupancy-only explanation | Independent defect observable; response and detection limit stated |
| Time-resolved observations and sample history | Static electronic screening versus non-equilibrium transport / depletion / replenishment | Randomized order, clocks, sample history and time-dependent nuisance model |
| Independent neutron and charged-particle channels | True yield change versus detector-specific response artifact | Channel overlap and energy availability explicitly declared |
| Blinded source/calibration injections and negative controls | Physical effect versus analysis/measurement artifact | Independent labels, frozen code and hidden truth retained until scoring |
| Leave-one-material / leave-one-campaign holdout | General mechanism versus campaign-specific nuisance fit | Holdout split frozen before model comparison |

A measurement modality contributes only if its response operator is calibrated in the actual sample matrix and relevant geometry. In particular, lateral spatial resolution is not depth resolution, and a protium-sensitive nuclear reaction is not a deuterium measurement.

## Benchmark ladder and provisional targets

All numeric thresholds below are **proposed qualification targets**, not achieved performance claims. Freeze data, splits, metrics and software digests before evaluation.

| Level | Benchmark | Proposed target / pass criterion |
|---|---|---|
| 0 | Algebraic and synthetic oracle | Exact recovery in constructed full-rank cases; explicit null-space detection in rank-deficient cases; fail-closed malformed inputs |
| 1 | Fisher-information numerics | Relative eigenvalue / rank decisions agree with independent high-precision references across scale sweeps; no unit-dependent surprises after parameter normalization |
| 2 | Response ensemble moments | Mean and covariance agree with an independent implementation; covariance is symmetric positive semidefinite within tolerance; exact zero calibration spread reduces to Poisson covariance |
| 3 | Uncertainty calibration | Across preregistered repeated simulations, nominal 90% interval coverage falls within a binomially justified acceptance band around 90%; likewise for 95% intervals |
| 4 | Standard spectrum unfolding | First reproduce published results on the exact source, detector and metric; then target ≤1.4% error on the cited EJ-301 / Cf best-case condition and ≤4.3% mean deviation only on the matching evaluation protocol, while maintaining calibrated uncertainty; separately compare EJ-309 GRAVEL / Bayesian benchmarks |
| 5 | Detector/model mismatch | Quantify performance under response-matrix perturbation, energy-scale drift, threshold shifts, background misspecification and unobservable bins; no silent overconfident output |
| 6 | CMNI forward prediction | Predict held-out energies, sample histories and campaigns with preregistered log score, residual, coverage and calibration diagnostics |
| 7 | Mechanism discrimination | Identify which hypotheses are not separable under a given observation plan; select candidate designs that measurably reduce ambiguity on held-out synthetic truth |
| 8 | Reproducibility | 100% of published benchmark results carry source hashes, code / dependency digest, frozen split, solver identity, parameters and replay receipt |

For coverage, do not use a simplistic “observed coverage equals nominal” rule on a small number of replicates. Set the number of replicates and acceptance band using a binomial interval before running the benchmark.

For spectrum accuracy, report at least Qs or a clearly defined relative spectral error, peak/centroid error where relevant, total fluence bias, interval coverage, and sensitivity to the energy-grid and response-matrix condition number. Normalize shapes and absolute fluence separately; shape agreement does not prove absolute rate calibration.

## State-of-practice comparison

| Stack / method | Strong use | Limitation to test rather than assume |
|---|---|---|
| Geant4 detector transport | Build energy-dependent response matrices and geometry/material response | Geometry, scintillation/light conversion, calibration, nuclear data and detector-specific validation must be qualified |
| GRAVEL | Established nonnegative iterative neutron-spectrum unfolding | Iteration/stopping and response conditioning affect the solution; uncertainty coverage must be assessed |
| Bayesian unfolding | Posterior spectrum and uncertainty quantification; recent 2026 work reports under-15% relative deviation and 92.3% coverage on stated test sources | Performance is source-, detector-, prior- and benchmark-dependent; reproduce on matched datasets before comparison |
| SRIM/TRIM-class ion transport | Useful stopping / implantation baseline in appropriate material and energy domains | Does not by itself provide the complete dynamic hydride, defect, electronic or reaction model |
| Specialist materials diagnostics | Independent deuterium, defect, phase and depth-sensitive observables | Different modalities have different isotope selectivity, point-spread functions, depth kernels, sample constraints and uncertainty |
| Symtropy CMNI research twin | Connects source evidence, solver adapters, conservation, measurement operators, nuisance models, replay and experiment design | It is not yet a validated specialist nuclear/materials solver; current kernel is research-only and synthetic |

The February 2026 high-dimensional Bayesian unfolding paper reports <15% spectral relative deviation and 92.3% coverage on its specified IAEA-403 Cf and Am-Be test data. A separate calibrated EJ-301 study (published online 27 November 2025; Scientific Reports volume 16, 2026) reports a sub-0.35-MeV threshold, average deviation below 4.28%, and best-case 1.42% agreement with the ISO 8529-1 Cf reference. Those are not interchangeable benchmarks: the detector, calibration, source, energy grid and scoring protocol differ. In particular, EJ-301 is not EJ-309. The August 2026 EJ309/EJ276D comparison uses Geant4 response matrices and GRAVEL/Qs and finds detector size and stopping choices matter. Reproduce each study on its own data and metrics before comparing methods or claiming improvement.

References:
- Wang et al. (2026), *A high-dimensional Bayesian approach for spectrum unfolding and uncertainty quantification in spectrometric measurements*, DOI: https://doi.org/10.1016/j.nima.2025.171041
- *Geant4 simulation of EJ309 and EJ276D scintillators: A comparative study for fast neutron spectroscopy* (2026), DOI: https://doi.org/10.1016/j.net.2026.104376
- López-Fidalgo & Wong (2026), *Optimal Designs for Correlated Data*, DOI: https://doi.org/10.1146/annurev-statistics-042324-012947
- Zhang et al. (2025 online / 2026 volume), *High-fidelity unfolding of 252Cf neutron spectra in EJ-301 scintillators via calibrated response matrices and Monte Carlo simulations*, DOI: https://doi.org/10.1038/s41598-025-30154-7
- Neudecker et al. (2025), *Re-evaluating the prompt fission neutron spectrum of spontaneously fissioning 252Cf*, DOI: https://doi.org/10.1051/epjn/2025061
- AIACHNE (2026), independent 252Cf PFNS measurement using 12C and 9Be elastic scattering, DOI: https://doi.org/10.1103/w7z1-l32n
- IAEA Neutron Metrology File NMF-90: https://www-nds.iaea.org/public/libraries/nmf/
- Coons & Huan (2025), *A Multifidelity Estimator of the Expected Information Gain for Bayesian Optimal Experimental Design*, DOI: https://doi.org/10.1137/25M1731812
- Chowdhary, Attia & Alexanderian (2026), *Robust Optimal Experimental Design of Infinite-Dimensional Bayesian Nonlinear Inverse Problems*, DOI: https://doi.org/10.1137/24M1693921
- NIST, *Assessment of measurement uncertainty via observation equations*: https://www.nist.gov/publications/assessment-measurement-uncertainty-observation-equations

### Layer F — reference-spectrum uncertainty and shape-only scoring

A spectrum reference is not automatically exact ground truth. The historical ²⁵²Cf prompt-fission neutron spectrum (PFNS) standard dates to 1987; a 2025 AIACHNE re-evaluation reports that original evaluation inputs and experimental covariances were lost. It could only reproduce the historical result within its stated one-sigma uncertainties, while the new evaluation incorporates reviewed datasets and covariance estimates, differs below 300 keV, and extends the evaluated range to approximately 500 eV–25 MeV. The authors provide pointwise spectra, correlation/covariance files and JSON experimental databases as supplementary material.

A separate 2026 AIACHNE measurement used ¹²C and ⁹Be elastic-neutron-scattering efficiency determinations, a method independent of the previous measurements, and reports a detailed covariance treatment. It supports the overall existing PFNS shape while pointing out possible deficiencies in angle-differential ¹²C and ⁹Be evaluated nuclear data that create structures in the inferred spectrum. Thus even an independent reference can carry nuclear-data model uncertainty.

The kernel now provides a full-covariance generalized-least-squares shape score with one nonnegative multiplicative scale profiled analytically. Its scale is a nuisance parameter removed to compare shapes; it is **not** an absolute fluence estimate. Report shape error and absolute normalization / fluence error separately. Use the full reference covariance, compare alternate evaluated versions, and document any reference-grid interpolation or binning. Never treat a sparse reference grid as permission for arbitrary linear interpolation.

A complementary public source is the IAEA Neutron Metrology File NMF-90: it includes four spectrum-adjustment codes, IRDF-90/NMF-G reaction cross sections with covariance files, and six consistent neutron-field benchmark datasets with calculated spectra, measured reaction rates and covariance information. It is an older static resource (last update listed as 2006) and is not a scintillator pulse-height benchmark; it is useful for validating covariance-aware adjustment, input/output reproducibility and nuclear-data uncertainty handling.

The best published EJ-301 result identified in this audit remains a distinct detector benchmark: a 1-inch/2-inch EJ-301 Geant4 + calibrated response + MLEM/GRAVEL study reports 0.35-MeV lower threshold, below-4.28% average deviation and 1.42% best case against ISO 8529-1. It uses a different detector from EJ-309, and its paper says supporting data are unavailable publicly due to commercial-contract restrictions. The reported response matrices have 80 neutron groups over 0–16 MeV and 50 gamma groups over 0–3 MeV. This should be listed as an external result pending author data access, not as a reproducible benchmark in the current repository.

## Current implementation and limits

Implemented in this research-only Rust crate:

- independent-Poisson Fisher information from caller-supplied expected counts and Jacobian;
- combination of Fisher contributions from independent candidate runs;
- exact pairwise directed KL divergence for fixed independent-Poisson predictions from candidate discrete hypotheses;
- full-covariance spectrum-shape chi-square with an analytically profiled multiplicative normalization;
- symmetric Jacobi eigenvalue decomposition and local effective-rank / logdet / condition diagnostics;
- equal-weight detector response ensemble and predictive mean/count covariance using the law of total covariance;
- finite equal-weight response-ensemble Poisson mixture log predictive probability for observed integer counts, evaluated in log space;
- nested two-level scalar approximate-control-variate estimation with frozen beta and a nested-sample variance estimate;
- finite-scenario maximin candidate-design scoring with cost metadata and deterministic tie-breaking;
- covariance-aware reference-spectrum shape comparison with profiled scale.

Not implemented yet:

- automatic numerical differentiation of the full CMNI forward model;
- global profile-likelihood identifiability, Bayesian model comparison or expected information gain;
- a full Bayesian expected-information-gain estimator, multifidelity sample-allocation optimizer or ACV coefficient learner;
- a continuous robust-design optimizer, uncertainty-set calibration or Pareto frontier;
- a nuisance-integrated or posterior-predictive model-discrimination score;
- a general nuisance-aware hierarchical likelihood for response, background, live time, PSD and other calibration dimensions;
- versioned PFNS reference evaluation ingestion, JSON experimental covariance database reader, interpolation audit and absolute-fluence comparison;
- Geant4, GRAVEL, or the cited 2026 Bayesian method;
- real 2025/2026 waveform or source-dataset reconstruction.

The finite response-mixture likelihood assumes equal weights over the supplied finite calibration ensemble and fixed background. The ACV coefficient is fixed independently of the production samples and the estimator does not bound model discrepancy. Finite maximin scoring only covers the caller-supplied nuisance scenarios. The code has authored tests only; they require a Rust-capable execution environment before being described as passing. These synthetic and algebraic primitives do not validate an underlying nuclear mechanism or provide experimental evidence.
