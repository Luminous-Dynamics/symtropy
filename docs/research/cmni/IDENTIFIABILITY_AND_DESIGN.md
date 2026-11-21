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
| 4 | Standard spectrum unfolding | First reproduce published results on the exact source, detector and metric; then target ≤1.42% error on the cited EJ-301 / Cf best-case condition and ≤4.3% mean deviation only on the matching evaluation protocol, while maintaining calibrated uncertainty; separately compare EJ-309 GRAVEL / Bayesian benchmarks |
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

A spectrum reference is not automatically exact ground truth. The historical ²⁵²Cf prompt-fission neutron spectrum (PFNS) standard dates to 1987; a 2025 AIACHNE re-evaluation reports that original evaluation inputs and experimental covariances were lost. It could only reproduce the historical result within its stated one-sigma uncertainties, while the new evaluation incorporates reviewed datasets and covariance estimates, differs below 300 keV, and extends the evaluated range to approximately 500 eV–25 MeV. The publisher's supplementary-material description lists seven files: two pointwise spectra with correlation/covariance files for the new evaluation, two analogous files matching the historical evaluation, two JSON experimental-input databases, and a smoothed normalized spectrum. These are the preferred public reference artifacts to ingest rather than digitizing plots.

A separate 2026 AIACHNE measurement used ¹²C and ⁹Be elastic-neutron-scattering efficiency determinations, a method independent of the previous measurements, and reports a detailed covariance treatment. It supports the overall existing PFNS shape while pointing out possible deficiencies in angle-differential ¹²C and ⁹Be evaluated nuclear data that create structures in the inferred spectrum. Thus even an independent reference can carry nuclear-data model uncertainty.

The kernel now provides a full-covariance generalized-least-squares shape score with one nonnegative multiplicative scale profiled analytically. Its scale is a nuisance parameter removed to compare shapes; it is **not** an absolute fluence estimate. Report shape error and absolute normalization / fluence error separately. Use the full reference covariance, compare alternate evaluated versions, and document any reference-grid interpolation or binning. Never treat a sparse reference grid as permission for arbitrary linear interpolation.

A complementary public source is the IAEA Neutron Metrology File NMF-90: it includes four spectrum-adjustment codes, IRDF-90/NMF-G reaction cross sections with covariance files, and six consistent neutron-field benchmark datasets with calculated spectra, measured reaction rates and covariance information. It is an older static resource (last update listed as 2006) and is not a scintillator pulse-height benchmark; it is useful for validating covariance-aware adjustment, input/output reproducibility and nuclear-data uncertainty handling.

The best published EJ-301 result identified in this audit remains a distinct detector benchmark: a 1-inch/2-inch EJ-301 Geant4 + calibrated response + MLEM/GRAVEL study reports 0.35-MeV lower threshold, below-4.28% average deviation and 1.42% best case against ISO 8529-1. It uses a different detector from EJ-309, and its paper says supporting data are unavailable publicly due to commercial-contract restrictions. The reported response matrices have 80 neutron groups over 0–16 MeV and 50 gamma groups over 0–3 MeV. This should be listed as an external result pending author data access, not as a reproducible benchmark in the current repository.

### Layer G — feature-conditioned systematic-bias attribution

A residual that follows detector technology, analysis method, geometry or calibration feature should not be absorbed automatically into a new physical parameter. The AIACHNE PFNS work is a relevant precedent: it develops a sparse Bayesian strategy for relating energy-dependent multiplicative discrepancies to experiment metadata, and its 2025 re-evaluation explicitly audits detector/analysis features, background, scattering, attenuation, response, deadtime and angular corrections.

A candidate CMNI observation model is:

[
\log y_{e,k}=\log \mu_{e,k}(\theta)+x_e^T\beta_k+b_{e,k}+\epsilon_{e,k},
]

where e indexes experiments, k indexes energy / observation channels, x_e is a versioned experimental-feature vector, beta_k is a sparse energy-dependent feature-bias coefficient, b_e,k is a declared run-level random effect, and epsilon follows the measurement likelihood with its full covariance. A horseshoe or another predeclared shrinkage prior is a candidate for beta, not a default to be tuned after inspecting results.

The kernel now includes a deterministic forward primitive that applies caller-supplied feature coefficients to positive physical expected counts using exp(X beta) independently for each observation channel. It does not estimate beta, impose shrinkage, modify raw data, or infer causal effects. This is an interface boundary that keeps potential detector/analysis bias parameters distinct from the physical reaction model.

Qualification requirements:

- Raw observations and source artifacts are immutable. A corrected dataset is a separate derived layer with a complete provenance receipt.
- Physical parameters theta and feature-bias parameters beta remain distinct in the graph.
- Collinear physical and measurement-feature effects must be reported as non-identifiable, not separated by an arbitrary prior.
- Fit candidate bias features on a training set, then test the inferred correction on held-out experiments using predictive log score and interval coverage.
- Include no-bias negative controls, planted sparse-bias synthetic controls, correlated-feature stress tests, and feature-label permutation controls.
- Compare no-bias, run-specific bias and feature-conditioned sparse-bias models under identical covariance and holdout rules.
- Require independent detector / measurement-lineage evidence before any corrected observations influence a physical-mechanism claim.

Proposed synthetic qualification targets: at least 90% power for preregistered effect sizes, no more than 5% false discovery rate under no-bias controls, and calibrated 90/95% intervals. These are proposed thresholds, not achieved results. Symthaea tracking: https://github.com/Luminous-Dynamics/symthaea/issues/13074.

### Layer H — small-feature Bayesian bias inference

The research kernel now includes a bounded spike-and-slab comparator for a flattened set of observation channels:

[
y_i\sim\operatorname{Poisson}\left(\mu_{i,\mathrm{physical}}\exp(x_i^T\beta)\right).
]

It enumerates all feature subsets exactly for up to 12 preregistered predictors. Each included coefficient has a zero-centred Gaussian slab prior and each feature has an independent Bernoulli inclusion prior. Within each subset, Newton optimization finds the posterior mode; a Laplace approximation around that mode estimates the subset marginal likelihood and coefficient covariance. Posterior model probabilities are normalized across the enumerated subsets, then used to calculate feature inclusion probabilities and mixture coefficient uncertainty.

This is **not** the AIACHNE model reproduction. AIACHNE's WANDA 2025 presentation describes a sparse Bayesian, energy-dependent multiplicative-bias model with a horseshoe prior, with bias basis matrix B and coefficients gamma. Our implementation uses exact small-subset enumeration plus a Gaussian slab and Laplace integration as a transparent alternative. It should be benchmarked against a faithful horseshoe implementation on the actual AIACHNE/PFNS feature table before claiming parity.

The current algorithm is intended for a small, frozen feature dictionary. For energy-dependent bias, the caller can flatten experiment-by-energy observations and build columns from predeclared experiment-feature × energy-basis interactions. Standardize columns using training data only; do not leak holdout information into feature scaling. Keep physical means fixed in this first implementation, and do not add a free global intercept unless its confounding with absolute normalization is explicitly handled.

Qualification controls now include:
- a planted single-feature multiplicative bias with strong posterior inclusion;
- a no-bias negative control where the null model is preferred;
- perfectly collinear features where posterior inclusion remains symmetric and coefficient covariance exposes attribution ambiguity;
- prior and feature-scaling sensitivity, feature-label permutation, and campaign-level holdout tests as required next-stage qualification.

Posterior inclusion is not causal attribution. A feature can identify a predictive measurement association without identifying why the physical experiment produced it. Any proposed corrected dataset remains separate from immutable source observations and must carry its coefficient posterior and provenance.

### Layer I — direct energy-basis and correlated PFNS-style bias model

A closer audit of Walton et al. (LA-UR-24-29607; Computer Physics Communications, DOI 10.1016/j.cpc.2025.109698) clarifies the structure that our earlier generic feature operator lacked:

- Their baseline per-experiment PFNS model is μ = D σ s, where D maps the evaluated spectrum to the observation energy grid and s is a per-experiment scale.
- The baseline observation model is y = μ + ε with multivariate-normal error covariance diag(μ) Σ diag(μ), where Σ is the experiment's relative covariance.
- Experimental datasets are grouped by *levels* of a categorical feature. The feature categories include hardware, methods and corrections. The same datasets can appear in overlapping group levels under different features, so multiple redundant explanations must be assessed.
- A group-specific multiplicative bias is y_g = μ exp(δ_g) + ε, with δ_g(E) expanded in Gaussian functions over log10 outgoing energy.
- The reported basis uses 160 functions per feature level: widths 0.3, 0.15 and 0.05 in log10-energy coordinates, with 10, 50 and 100 logarithmically spaced centres respectively.
- Bias coefficients receive a horseshoe prior with global sparsity parameter τ. The paper examines fixed τ, half-Cauchy and half-normal hyperpriors; the half-normal was reported as more numerically stable in their comparison.

The research kernel now constructs the reported 160-column grouped energy basis and supports overlapping group memberships. This is basis construction only; it does not implement the horseshoe prior or the paper's Bayesian Monte Carlo sampler.

The kernel also adds an exact small-subset Bayesian model average for a *log-ratio Gaussian approximation*. It fits log(observed / baseline) residuals using a supplied full covariance matrix, always-included Gaussian-prior fixed effects (for example, per-experiment log normalization), and Bernoulli-inclusion / Gaussian-slab sparse bias features. The Gaussian subset marginal likelihood is integrated analytically, including covariance and model uncertainty. This is a useful correlated-data comparator, not the original AIACHNE raw-scale likelihood. Applying the delta method to their relative covariance Σ gives an approximate covariance in log-ratio space for modest relative errors; a faithful reproduction must retain their raw-scale multivariate-normal model, per-experiment scale and BMC/horseshoe inference.

Important limits:
- The 160-column-per-group basis can be built, but the current exact subset inference caps sparse columns at 12. It cannot fit the complete AIACHNE basis in one run.
- The separate Poisson spike-and-slab implementation is for count observations and must not be used as the PFNS likelihood.
- The log-Gaussian comparator requires a full covariance matrix already transformed / justified in log-ratio space; it does not automatically convert the AIACHNE raw relative covariance.
- Posterior inclusion describes association with a feature level, not a causal mechanism. Walton et al. explicitly report roughly 50 features for only 20 datasets and warn about underdetermination and expert interpretation.
- Their stable-bias screening required nonzero posterior bias samples to persist over a substantial sweep of τ_0; one isolated fit is insufficient.

### Layer J — raw-scale correlated observation likelihood

The kernel now evaluates the raw-scale observation likelihood for one experiment:

\[
y\\sim\\mathcal N\\!\\left(\\mu\\odot\\exp(\\delta),\\;D_\\mu\\Sigma_{\\mathrm{rel}}D_\\mu\\right),
\\qquad D_\\mu=\\operatorname{diag}(\\mu).
\]

Here mu is the un-biased prediction including the declared per-experiment normalization; Sigma_rel is the full covariance of relative experimental errors; and delta(E) is the grouped, energy-dependent log-bias field. The covariance is anchored to mu and stays fixed while the bias changes the mean. This matches the raw-scale relative-covariance convention described in the AIACHNE PFNS paper. It must not be replaced silently by the earlier log-ratio Gaussian approximation.

The new primitive is a per-experiment log-likelihood, with Cholesky-based full-covariance evaluation and fail-closed symmetry / positive-definiteness checks. It can be composed with the grouped 160-column basis and sparse feature coefficients. It does not yet implement the prior over grouped coefficients, global horseshoe sparsity parameter, posterior sampler, or across-experiment BMC chain. It is one essential layer of the faithful reproduction, not the complete inference algorithm.

### Layer K — horseshoe posterior target and analytic gradients

The raw-scale likelihood can now be combined with a global-local horseshoe hierarchy. In the current research target:

[
\gamma_j\mid\lambda_j,\tau\sim\mathcal N(0,\tau^2\lambda_j^2),\qquad
\lambda_j\sim\operatorname{HalfCauchy}(0,1),
]

with either a half-normal or half-Cauchy prior on the global scale tau. The sampler coordinates are gamma, log(lambda) and log(tau), and the returned gradients include the Jacobians from the log-scale parameterization. The half-normal global-scale option is included because the AIACHNE paper reports it as more numerically stable than the half-Cauchy alternative in its comparison.

Independent finite-difference calculations validate the analytic gradients for coefficient, local-scale and global-scale coordinates under both prior choices. Five Rust unit tests now encode these finite-difference checks and basic parameter validation.

This is still **not Bayesian Monte Carlo or HMC sampling**. It is the posterior target and gradient needed by a sampler. It does not include adaptation, multiple chains, effective sample size, R-hat, divergent-transition diagnostics, tau0 sweep analysis, or posterior bias-field screening. Do not report posterior inclusion / credible intervals from this target until a qualified sampler has been run and converged.

### Layer L — cached posterior target and fixed-step HMC comparator

The fixed observation covariance is now factored once in a PreparedRawScaleHorseshoeModel. Each posterior evaluation reuses that Cholesky factor while calculating the changing bias mean and horseshoe prior. This avoids repeating the cubic covariance factorization at every leapfrog step.

A seeded fixed-step HMC comparator is also present. It uses the analytic gradients, caller-selected step size and leapfrog count, momentum reversal, a Metropolis accept/reject step, discarded initial transitions, and records accepted proposals and energy-error divergences. A basic split-Rhat screening function can compare a coefficient across independent chains.

This is not NUTS and not yet a qualified sampler: there is no step-size or mass-matrix adaptation, no rank-normalized / folded Rhat, no ESS / MCSE, no posterior predictive check and no tau0-sweep stability analysis. The HMC implementation should be compared against an independent mature sampler on tractable low-dimensional oracles before any posterior claim is made. The AIACHNE BMC sampler reproduction remains a separate comparison target, not something this HMC implementation is claimed to equal.

### Layer M — fixed-step HMC comparator and sampler qualification

The prepared raw-scale horseshoe target caches the observation covariance factorization so it can be evaluated repeatedly without rebuilding the same matrix factorization. A seeded fixed-step HMC comparator now uses that target, with caller-supplied step size and leapfrog count, a Metropolis accept/reject step, momentum reversal, discarded initial transitions, and acceptance / divergence accounting.

The current sampler has important limitations: no step-size adaptation, no mass-matrix adaptation, no NUTS, no rank-normalized or folded Rhat, no ESS / MCSE, no E-BFMI, and no posterior predictive diagnostics. It is an exploratory HMC comparator, not the AIACHNE BMC implementation or a qualified posterior sampler.

A defensible sampler qualification gate should follow the current Stan diagnostic guidance as an external reference:
- at least four independently seeded chains with overdispersed initial states;
- rank-normalized split and folded Rhat < 1.01 before trusting final inferences;
- bulk-ESS preferably > 100 × number of chains and adequate tail-ESS for the reported tail quantities;
- inspect divergent transitions, tree-depth / integration limits and energy behavior (E-BFMI);
- report MCSE relative to the precision required by the actual inferential decision;
- run prior / tau0 sensitivity and posterior predictive checks;
- validate the sampler against analytic low-dimensional targets and an independently implemented mature sampler before fitting the full PFNS basis.

Stan cites E-BFMI below approximately 0.3 as a warning sign for poor energy exploration. These are proposed CMNI qualification gates based on published sampler diagnostics, not results achieved by the current fixed-step implementation.

### Layer M — rank-normalized multi-chain qualification diagnostics

The HMC research comparator now records a joint Hamiltonian-energy value for every retained transition. A separate diagnostics module calculates rank-normalized split-Rhat and folded rank-normalized split-Rhat; bulk ESS on rank-normalized draws; tail ESS from lower/upper 5% indicators; raw-scale mean ESS and mean MCSE in coefficient units; per-chain E-BFMI and total divergences.

The proposed qualification flag requires at least four chains, max(Rhat) < 1.01, bulk/tail ESS at least 100 times the number of chains, zero divergences, and E-BFMI >= 0.3 for every chain. These are conservative project gates based on public Stan guidance, not proof that this implementation exactly matches Stan's calculations. MCSE is computed on raw coefficient draws; bulk ESS remains rank-normalized.

References: https://mc-stan.org/misc/warnings.html and https://mc-stan.org/docs/2_40/reference-manual/analysis.html

The diagnostics require at least four unique chain seeds and equal retained draw counts; the seed is recorded on each HMC chain. Unique seeds are a reproducibility guard, not proof of independent posterior exploration. The diagnostics remain a research implementation requiring independent comparison against reference chains. Rank ties, heavy-tail stress cases, autocorrelation patterns, E-BFMI and ESS should be cross-checked against a mature implementation before the qualification flag is trusted. Mean MCSE is computed on raw coefficient draws and scaled in coefficient units; bulk ESS is computed on rank-normalized draws. An IID deterministic positive-control gives Rhat and folded Rhat near 1, bulk ESS about 7,896, tail ESS about 7,690 and minimum E-BFMI about 1.94 in an independent Python implementation of these equations. This is a numerical cross-check, not Rust execution or Stan parity. The diagnostics do not replace trace plots, posterior predictive checks or prior sensitivity.

### Layer N — source-archive admission gate

The public branch now contains an admission schema and manifest for the seven 2025 PFNS supplementary artifacts. The manifest identifies their scientific roles but deliberately records each as not accessible, with filename, byte length, retrieval timestamp and SHA-256 all null. The publisher page documents two pointwise evaluation spectra, two matching correlation/covariance files, two JSON experimental-input databases, and a smoothed normalized spectrum. The supplement endpoint and container download route could not be retrieved in this environment, and no provenance-verifiable mirror was found.

This is an evidence gate, not data ingestion. The manifest must not be promoted to hash-verified or content-validated until the original file bytes have been retrieved, exact publisher filenames and byte lengths recorded, SHA-256 computed, and the internal schema / covariance semantics checked. Preserve sparse energy grids and do not silently linearly interpolate the historical PFNS files. Tracking: https://github.com/Luminous-Dynamics/symtropy/issues/6571.

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
- covariance-aware reference-spectrum shape comparison with profiled scale;
- a deterministic log-linear feature-conditioned multiplicative-bias forward operator, with caller-supplied coefficients;
- a published-form 160-column grouped Gaussian log-energy bias-basis constructor with overlapping categorical feature-level memberships;
- a small-feature Poisson spike-and-slab comparator for count data;
- a full-covariance log-ratio Gaussian sparse model average with fixed normalization effects and exact Gaussian integration over subsets (up to 12 sparse columns);
- a raw-scale correlated Gaussian PFNS likelihood with mean mu * exp(delta) and covariance diag(mu) Sigma_relative diag(mu), preserving the un-biased covariance anchor;
- a global-local horseshoe posterior target with half-Cauchy local scales, half-normal or half-Cauchy global scale, and analytic gradients in log-scale sampler coordinates;
- a prepared raw-scale target with cached covariance factorization and seeded fixed-step HMC;
- rank-normalized / folded Rhat, bulk/tail ESS, raw-mean ESS / MCSE, E-BFMI and divergence aggregation as a separate research diagnostic.

Not implemented yet:

- automatic numerical differentiation of the full CMNI forward model;
- global profile-likelihood identifiability, Bayesian model comparison or expected information gain;
- a full Bayesian expected-information-gain estimator, multifidelity sample-allocation optimizer or ACV coefficient learner;
- a continuous robust-design optimizer, uncertainty-set calibration or Pareto frontier;
- a nuisance-integrated or posterior-predictive model-discrimination score;
- remaining sampler qualification: step-size / mass-matrix adaptation, NUTS or validated BMC comparison, independent reference-chain validation for the new diagnostics, posterior predictive checks and tau0 stability sweeps;
- inference over the complete 160-basis-per-group design and global τ hyperprior;
- τ_0 sensitivity sweeps, redundant feature-level grouping analysis, correlated-feature robustness, and campaign-held-out validation;
- a general nuisance-aware hierarchical likelihood for response, background, live time, PSD and other calibration dimensions;
- versioned PFNS reference evaluation ingestion, JSON experimental covariance database reader, interpolation audit and absolute-fluence comparison;
- Geant4, GRAVEL, or the cited 2026 Bayesian method;
- real 2025/2026 waveform or source-dataset reconstruction.

The finite response-mixture likelihood assumes equal weights over the supplied finite calibration ensemble and fixed background. The ACV coefficient is fixed independently of the production samples and the estimator does not bound model discrepancy. Finite maximin scoring only covers the caller-supplied nuisance scenarios. The code has authored tests only; they require a Rust-capable execution environment before being described as passing. These synthetic and algebraic primitives do not validate an underlying nuclear mechanism or provide experimental evidence.
