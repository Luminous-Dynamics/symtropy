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

The current research kernel accepts caller-supplied expected counts and Jacobians and computes this matrix. It combines independent run contributions and reports eigenvalues, effective rank, log determinant and condition number.

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

The implemented ensemble reference uses the average conditional Poisson variance plus the sample covariance of conditional means. Background is fixed within this first implementation; background calibration uncertainty, live-time/dead-time uncertainty, pileup, threshold and PSD uncertainty remain separate required model dimensions.

This is a forward uncertainty primitive, not detector calibration, Geant4, scintillation simulation, PSD reconstruction or a source-spectrum inversion.

## Layer C — experiment selection

Candidate interventions should be represented as observation-prediction alternatives, not narrative labels. Each design candidate declares:

- the parameterization and scaling;
- expected observation vector and Jacobian or full likelihood;
- observation covariance / nuisance model;
- cost, practical feasibility and sample constraints;
- a set of hypotheses that it is intended to separate;
- a predeclared holdout and falsifier.

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
| 4 | Standard spectrum unfolding | On blinded standard spectra and matched response matrices, aim for mean relative spectral deviation at or below 10% while maintaining calibrated 90–95% coverage; compare with GRAVEL and Bayesian baselines |
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

The 2026 Bayesian unfolding paper reports <15% spectral relative deviation and 92.3% coverage on its specified Cf and Am-Be test data. The 2026 EJ309/EJ276D Geant4 comparison uses GRAVEL and Qs, and finds detector-size and stopping-criterion choices matter. These are benchmarks to reproduce under matched conditions, not universal performance constants.

References:
- Wang et al. (2026), *A high-dimensional Bayesian approach for spectrum unfolding and uncertainty quantification in spectrometric measurements*, DOI: https://doi.org/10.1016/j.nima.2025.171041
- *Geant4 simulation of EJ309 and EJ276D scintillators: A comparative study for fast neutron spectroscopy* (2026), DOI: https://doi.org/10.1016/j.net.2026.104376
- López-Fidalgo & Wong (2026), *Optimal Designs for Correlated Data*, DOI: https://doi.org/10.1146/annurev-statistics-042324-012947
- NIST, *Assessment of measurement uncertainty via observation equations*: https://www.nist.gov/publications/assessment-measurement-uncertainty-observation-equations

## Current implementation and limits

Implemented in this research-only Rust crate:

- independent-Poisson Fisher information from caller-supplied expected counts and Jacobian;
- combination of Fisher contributions from independent candidate runs;
- symmetric Jacobi eigenvalue decomposition and local effective-rank / logdet / condition diagnostics;
- equal-weight detector response ensemble and predictive mean/count covariance using the law of total covariance.

Not implemented yet:

- automatic numerical differentiation of the full CMNI forward model;
- global profile-likelihood identifiability, Bayesian model comparison or expected information gain;
- a design-space optimizer, cost model or Pareto frontier;
- a correlated/hierarchical Poisson likelihood for uncertain response matrices;
- bootstrap/calibration-data ingestion and covariance provenance;
- Geant4, GRAVEL, or the cited 2026 Bayesian method;
- real 2025/2026 waveform or source-dataset reconstruction.

The code has authored tests only; they require a Rust-capable execution environment before being described as passing. These synthetic and algebraic primitives do not validate an underlying nuclear mechanism or provide experimental evidence.
