# CMNI state of the field and comparative benchmark programme (2026)

**Status:** research strategy and benchmark contract  
**Scope:** public literature and code audit current to 2026-11-21. Numerical targets below are proposed CMNI qualification targets unless explicitly labelled as published results. This document does not claim a completed AIACHNE reproduction, a validated nuclear-data evaluation, or superiority over specialist codes.

## 1. The current frontier is already an end-to-end scientific process

CMNI must compare itself against the complete state of the art, not only against a bias-fitting algorithm.

### AIACHNE: discrepancy discovery, expert root-cause investigation, and an independent experiment

Neudecker et al., “Resolving root causes of experiment discrepancies guided by machine learning,” Nature Communications (published 2026-08-27), formalizes a process with these stages:

1. Bayesian ML relates discrepancies to metadata features that may encode measurement procedures.
2. Subject-matter experts investigate candidate causes using targeted simulations of historical experiments.
3. New measurements are designed to resolve remaining uncertainty.
4. Historical data are rejected, corrected/salvaged, or independently validated on scientific grounds.

The published demonstration uses 21 252Cf PFNS datasets and 41 metadata feature categories. The feature analysis is performed one category at a time using the collection of 21 datasets. The authors report an unweighted average reduction in between-dataset spread by a factor of 2.8 and a maximum of up to 6 when the most conspicuous outliers are excluded. They also report a targeted independent experiment that reduced evaluated uncertainties by up to 25% over an energy range containing 80% of the PFNS integral. Salvaging the Kornilov data is reported to reduce evaluated uncertainty by about 5%.

The independent experiment is as important as the ML result. It used elastic neutron scattering on 9Be and 12C to provide a response-determination pathway different from the historical 1H, 6Li, and 235U-based approaches. The result tests a possible common-mode explanation rather than merely adding another measurement with the same response lineage.

**CMNI implication:** reproducing sparse bias inference is necessary but insufficient. The competitive baseline is the entire ML → expert physics/simulation → independent measurement → evaluate/reject/correct process.

Primary sources:
- https://doi.org/10.1038/s41467-026-76798-5
- https://pubmed.ncbi.nlm.nih.gov/42802175/
- https://github.com/lanl/sparse_bias

### PARADIGM: application- and fundamental-science experiment portfolios

Neudecker et al., “Machine Learning to Select Experiments Driven by Fundamental Science and Applications for Targeted Nuclear Data Improvement,” Physical Review X 15, 021086 (published 2025-06-09), provides a second direct comparator.

The published study combines 168 historical experiments, 23 candidate differential experiments and generated integral criticality candidates. Its optimization addresses 12,200 nuclear-data values relevant to the integral experiments. Generalized least squares, Gaussian-process smoothing and D-optimality are used to select a portfolio of four experiments: two differential measurements and two integral criticality experiments.

**CMNI implication:** the design objective must cover a portfolio of complementary experiments and downstream application sensitivity, not merely choose the next point on one curve or maximize a local Fisher determinant.

Primary source:
- https://doi.org/10.1103/PhysRevX.15.021086

### Multifidelity Bayesian optimal design

Coons and Huan, “A Multifidelity Estimator of the Expected Information Gain for Bayesian Optimal Experimental Design,” SIAM/ASA Journal on Uncertainty Quantification 13(4), 1990–2021 (online 2025-11-25), introduce a multi-fidelity expected-information estimator under approximate control variates. In the demonstrated nonlinear and turbulent-flow examples, the estimator is unbiased with respect to the high-fidelity mean and obtains one-to-two orders of magnitude variance reduction compared with the single-fidelity estimators studied.

CMNI currently has a scalar nested approximate-control-variate primitive. That is not the published full MF-EIG method: it does not yet calculate EIG, optimize high/low sample allocation, estimate beta from an independent pilot, or certify robustness to fidelity discrepancy.

**CMNI implication:** implement the paper's complete benchmark and estimator before claiming a multifidelity advantage.

Primary sources:
- https://doi.org/10.1137/25M1731812
- https://arxiv.org/abs/2501.10845

### Batch Bayesian experiment design: optimize portfolios, not just the next point

Mäkinen, Duncan and Helin, *Batch-based Bayesian optimal experimental design in linear inverse problems*, Statistics and Computing 36, 266 (published 2026-11-11; DOI 10.1007/s11222-026-11022-4), give a Bayesian formulation for batch A-optimal design relaxed to finite positive measures with fixed mass. Their expected utility is concave in the relaxed measure, and they derive Wasserstein-gradient particle updates. Their practical lifted formulation uses an ensemble-variance penalty to concentrate each design distribution and an MMD-based repulsive term to discourage distinct design ensembles from collapsing onto the same point.

Important limitations in the paper are directly relevant to CMNI:
- the unregularized measure solution does not automatically guarantee exactly B deployable discrete measurement locations;
- the repulsive and variance regularizers generally remove concavity of the lifted objective, so local maxima may depend on initialization and regularization weights;
- the numerical examples are linear inverse problems, not nuclear-data campaign portfolios.

A complementary 2026 preprint by Sharrock, *Wasserstein Gradient Flows for Batch Bayesian Optimal Experimental Design* (arXiv:2603.12102), addresses batch expected-information-gain design through entropic regularization over design measures, with Gibbs design laws and mean-field / product approximations. This provides a promising bridge from A-optimal covariance reduction to direct EIG optimization, but must be compared on identical candidate sets and compute budgets.

**CMNI implication:** compare discrete greedy selection, exhaustive small-batch oracle, relaxed A-optimal measures, and entropic batch-EIG methods. Measure utility after converting the relaxed distributions into a real deployable batch; report the rounding regret, exact batch-size constraint, minimum separation/feasibility, and sensitivity to initialization. Do not claim a globally optimal batch merely because the unregularized measure utility is concave—the regularized lifted objective may not be.

Primary sources:
- https://doi.org/10.1007/s11222-026-11022-4
- https://arxiv.org/abs/2603.12102

### Independence is a lineage graph, not a detector label

The July 2026 AIACHNE (^{252}\mathrm{Cf}) experiment used an efficiency-calibration approach uncorrelated with the previous detector-efficiency measurements, using elastic neutron scattering on (^{12}\mathrm{C}) and (^{9}\mathrm{Be}). The new route is an important independent test. Yet the paper also reports deficiencies in angle-differential (^{12}\mathrm{C}) and (^{9}\mathrm{Be}(n,n)) evaluated nuclear data, which produce small unphysical structures in the derived PFNS.

This gives CMNI a more precise definition of independent evidence:
- a measurement can be independent of historical detector calibration;
- its response calibration may still rely on evaluated nuclear data;
- those evaluated data may carry their own shared uncertainty, version, and common-cause dependencies.

Therefore do not treat "different detector" or "new experiment" as equivalent to "independent evidence." Trace the complete uncertainty path through reaction standards, evaluated libraries, response simulations, software corrections, and reference spectra. The covariance model should include shared source contributions when sensitivity/Jacobian information is available, and explicitly mark unknown dependencies where it is not.

The standalone CMNI crate now has a research-only common-cause covariance assembler: each experiment contributes an independent observation covariance and Jacobians with respect to named shared uncertainty sources. For shared source (s), it adds the cross block (J_{i,s}C_sJ_{j,s}^T). It rejects inconsistent covariance definitions for the same source ID rather than silently assuming zero cross covariance. The tests are authored, not yet CI-verified.

Primary source:
- https://doi.org/10.1103/w7z1-l32n

### Mature posterior diagnostics and spectrum-adjustment references

- Stan's current posterior-analysis guidance uses rank-normalized split and folded R-hat, bulk ESS, tail ESS, and Monte Carlo standard errors for quantities of interest. Its warning guidance recommends bulk and tail ESS greater than 100 times the number of chains for final analysis, while also emphasizing that Monte Carlo error must be relevant to the actual scientific estimand.
- The IAEA NMF-90 resource is a quality-checked integrated neutron-spectrum adjustment package with four adjustment codes, six reactor benchmark input data sets, a 640-group dosimetry cross-section library, and covariance files. The IAEA catalogue lists the resource as static with last update 2006; it is a historical benchmark, not a newly current evaluation.
- A 2025 analytic neutron Boltzmann benchmark extends uncertainty-propagation verification to PFNS and average prompt-neutron multiplicity, with continuous-energy cross sections.

Primary sources:
- https://mc-stan.org/docs/reference-manual/analysis.html
- https://mc-stan.org/misc/warnings
- https://nucleus.iaea.org/Pages/nmf.aspx
- https://doi.org/10.1080/00295639.2025.2489769

## 2. Where CMNI can make a distinct contribution

The goal is not to claim that a new Rust kernel is intrinsically better than Stan, Geant4, GRAVEL, MAXED, or the AIACHNE process. The opportunity is to make the scientific decision pipeline more explicit, testable, and reproducible.

### A. Replace marginal feature sweeps with joint, lineage-aware causal attribution

The AIACHNE study's described workflow investigates one feature category at a time. This is useful for interpretable clues, but CMNI should explicitly test the cases where:
- two metadata categories are correlated (for example, detector type and response-correction method);
- a campaign effect is aliased with a detector or reference family;
- two features interact;
- a shared reference error appears as multiple apparently independent observations;
- normalization is confounded with a constant energy-bias component.

Do not assume a joint model is automatically superior. Compare one-feature-at-a-time, joint additive, hierarchical interaction, and sparse interaction models on identical blind holdouts. The winner is the model with better out-of-sample attribution and calibrated uncertainty, not the largest in-sample evidence or most elaborate latent structure.

### B. Treat normalization as a gauge freedom

For y(E) = s mu(E) exp(a + delta_shape(E)), a free experiment scale s and a constant bias offset a can be observationally equivalent. No sampler diagnostic can separate two parameters whose likelihood directions are identical.

CMNI should:
- expose the normalization/bias null direction;
- separate a group-wide offset from shape-only bias;
- profile or marginalize nuisance parameters when assessing physical identifiability;
- demand an external anchor or orthogonal measurement when the decomposition is not identified;
- abstain from causal attribution when competing explanations remain observationally equivalent.

The standalone kernel now contains a research implementation of nuisance-profiled Fisher information using a Schur complement. It is deliberately strict: a singular or ill-conditioned nuisance block is rejected rather than silently regularized. Its tests are authored, not yet executed.

### C. Optimize independence of evidence, not only information volume

D-optimality can select measurements that jointly constrain a parameter vector. A separate question is whether the new experiment is sufficiently independent of the existing evidence to falsify a shared systematic explanation.

Add an evidence-lineage/common-cause dimension to design:
- response-calibration lineage;
- reference reaction and evaluated data lineage;
- detector technology and analysis chain;
- shared software, nuclear data, and correction procedures;
- sample, campaign, laboratory, and operator dependencies where recorded.

A high-value candidate experiment should be scored for both information gain and independence of its dominant error pathways. A nominally new measurement that shares the suspected systematic root may provide little discriminating power.

### D. Make the benchmark decision-centric

The primary output is not a more precise fitted curve. It is a qualified action:
- retain a datum with stated uncertainty;
- correct it under an independently validated physical model;
- reject it with a traceable scientific reason;
- request a discriminating measurement;
- abstain because the available evidence cannot distinguish explanations.

Score the quality of those actions against blinded truth, not only pointwise prediction error.

### E. Connect raw detector observations to evaluated spectra

Where source data permit, fit raw counts through detector response, background, live time, calibration, and physical forward models. For already-published PFNS spectra, preserve the source likelihood and covariance. Do not claim that a post-unfolded spectrum is a raw-count model or that an ensemble of response matrices is a complete detector simulation.

### Current common-cause covariance substrate

The research crate includes `assemble_joint_experiment_covariance` and typed inputs for per-experiment independent covariance plus named shared-source covariance/Jacobian contributions. It builds all within- and cross-experiment blocks with (J_i C_s J_j^T). The same source ID must carry a consistent covariance definition across every experiment; inconsistent definitions fail closed.

Each experiment also reports whether its common-cause lineage coverage is complete. The joint result propagates that flag. A computed block-diagonal matrix with incomplete source metadata is therefore not labelled as proof of independence. This is a provenance qualification signal, not a way to infer missing covariance values; unrecorded common causes still require source evidence or conservative sensitivity bounds.

Current implementation is a linearized covariance assembler. It is not yet a lineage graph ingestion engine, full joint likelihood, batch optimizer, or posterior propagation layer. Its unit tests are authored and await exact-head CI.

## 3. Frozen benchmark ladder

All candidate methods must receive identical training data, validation data, held-out experimental lineages, feature availability, and computational budgets. Include oracle methods only as diagnostic ceilings.

| ID | Comparator | What it tests |
|---|---|---|
| A0 | Original LANL AIACHNE notebook + selected Stan model | Faithful reproduction of the actual published analysis path |
| A1 | Original correlated Stan variant | Sensitivity to full within-experiment covariance |
| A2 | Current CMNI sparse bias / horseshoe target | Mathematical and posterior parity |
| A3 | CMNI joint hierarchical physical + detector + reference model | Whether joint attribution improves blind generalization |
| A4 | CMNI normalization-aware and nuisance-profiled model | Whether confounded directions are correctly exposed |
| A5 | AIACHNE-style expert-informed decisions | End-to-end action baseline, with expert evidence recorded separately |
| A6 | Oracle latent decomposition | Synthetic-only upper bound |
| D0 | Single-fidelity nested Monte Carlo EIG | Experimental-design variance / cost baseline |
| D1 | Published MF-EIG / ACV estimator | Multifidelity estimator parity |
| D2 | CMNI robust and lineage-aware design | Incremental design value against matched compute budgets |
| N0 | NMF-90 adjustment codes / documented input suite | Neutron spectrum-adjustment parity on the same input library |

A0 and A1 must remain distinct. The public LANL notebook selects interpolation_horseshoe with tau_scale=1e-5, 2,000 warmup and 2,500 retained samples. The interpolation_horseshoe_corr Stan model is a separate correlated-residual variant and is not selected by that inspected notebook path. The parity study must establish which implementation corresponds to each published figure/result; do not silently substitute one for the other.

## 4. Required benchmark suites and metrics

### Suite 1 — AIACHNE reproduction

- Freeze the upstream LANL repository commit and every source-file identity.
- Verify byte-level SHA-256 for the 21 JSON inputs and the pointwise reference data after retrieval.
- Reproduce JSON parsing, zero-bin handling, experiment-specific normalization, energy interpolation, feature masks, bias basis ordering, priors, covariance interpretation, and reported outputs.
- Match the original notebook's default model and separately run the correlated Stan variant.
- Report numerical differences in design matrices, log posterior, gradient, posterior summaries, and published bias curves.
- Preserve the upstream licence and provenance; prefer a source manifest over copying third-party data unnecessarily.

### Suite 2 — blinded causal attribution

Cross:
- physical effect: null, isolated, multiple, energy-dependent;
- measurement effect: null, detector, calibration, response, background, reference, scale;
- feature geometry: independent, correlated, nested, interacting, exact alias;
- covariance: diagonal, correlated within dataset, shared across datasets;
- metadata: complete, noisy, missing, campaign-confounded;
- split: held-out energy, experiment, detector, campaign, material, and reference lineage.

Metrics:
- physical and measurement false-attribution rates;
- precision/recall by mechanism and feature group;
- correct abstention on exact-equivalence cases;
- held-out log predictive density and proper scoring rules;
- 50/80/90/95% uncertainty coverage with binomial intervals;
- feature-inclusion calibration and sensitivity to priors / feature regrouping;
- transfer across detector, campaign, and reference lineage.

### Suite 3 — posterior correctness

- Compare CMNI against Stan on identical small and medium posterior targets.
- Test analytic gradients with independent finite differences and a separately implemented derivative path.
- Run simulation-based calibration on tractable known-truth models.
- Report rank-normalized and folded R-hat, bulk/tail ESS, MCSE for all decision-relevant summaries, E-BFMI, divergences, and independent-seed reproducibility.
- Require the Monte Carlo error to be small relative to the uncertainty or decision tolerance being reported.

Proposed final sampler gates: at least four independent chains; rank/folded R-hat below 1.01; bulk and tail ESS at least 100 per chain; zero unexplained divergences; E-BFMI at least 0.3 per chain. These gates do not establish likelihood correctness or physical-model validity.

### Suite 4 — active and robust experiment design

- Reproduce the published Coons–Huan nonlinear MF-EIG benchmark before a CMNI physics adaptation.
- Compare single-fidelity nested Monte Carlo and MF-EIG on the same total wall-clock / high-fidelity evaluation budget.
- Report estimator bias against a tractable oracle, variance, RMSE, high/low model calls, wall time, and degradation as low/high fidelity correlation weakens.
- Compare nominal D-optimality, E-/A-optimality, expected information gain, robust maximin design, and lineage-independent design.
- Report design regret against an oracle candidate, cost-normalized information gain, and the fraction of ambiguous scenarios resolved by the selected follow-up.

A variance reduction of 10–100x is a research target informed by the published MF-EIG demonstrations, not a current CMNI result. It must be achieved without material estimator bias or a hidden increase in total compute.

### Suite 5 — spectrum adjustment and uncertainty propagation

- Use identical NMF-90 input data, cross-section libraries, covariance, prior spectra, and measured reaction rates for CMNI and reference codes.
- Compare unfolded spectrum deviation, reaction-rate reconstruction, covariance propagation, integral observables, and coverage.
- Add the analytic neutron Boltzmann benchmark for a known-truth uncertainty-propagation test.
- Keep PFNS standard evaluation, NMF-90 adjustment, and detector pulse-height unfolding as separate benchmark families.

## 5. Proposed acceptance goals (not achieved results)

| Dimension | Proposed goal |
|---|---|
| Faithful AIACHNE reproduction | Reproduce the selected reference implementation and published outputs within predeclared numerical tolerances |
| Blind positive controls | At least 90% detection for independently identifiable planted effects at the preregistered power |
| Null controls | False-attribution rate no greater than 5%, reported with confidence intervals |
| Exact aliases | 100% explicit abstention when the supplied likelihood makes mechanisms observationally equivalent |
| Uncertainty | Nominal coverage demonstrated by SBC and held-out coverage intervals; no aggregate score may hide subgroup undercoverage |
| Design | At most 5% regret to oracle on a predeclared tractable design suite |
| MF-EIG | Reproduce the paper's estimator behaviour before targeting 10x or greater variance reduction in CMNI cases |
| Runtime | Report ESS/second, ESS/CPU-hour, high-fidelity call count, wall time, and memory, not a single throughput figure |
| Reproducibility | Exact commit, input hashes, config, seeds, environment and receipts for every scored result |

Use enough independent replicates to make the requested confidence interval meaningful. Thresholds and analysis plans must be frozen before blind labels are revealed. Do not select acceptance tolerances after seeing results.

## 6. Immediate implementation order

1. Finish standalone kernel quality gates: formatting, Clippy, compilation, and tests. Preserve all lints; do not remove -D warnings.
2. Complete nuisance-profiled information tests and use the result in the CMNI design scorecard.
3. Freeze the upstream LANL sparse_bias source commit and inventory all 22 files in its AIACHNE data directory. Distinguish Git blob identities from SHA-256 file digests.
4. Build exact preprocessing / Stan-data parity tests and run both notebook-selected and correlated Stan reference models.
5. Implement a deterministic blind generator with scale/constant-bias gauge cases, feature interactions, shared references, and independent-control experiments.
6. Reproduce the published MF-EIG estimator before extending CMNI's scalar ACV primitive.
7. Compare experiment portfolios using both uncertainty reduction and independence of systematic error pathways.
8. Publish a stratified scorecard and failure cases before any claim of beating a specialist method.

## 7. Status boundary

The 2026 AIACHNE process and 2025 PARADIGM portfolio are published external baselines. The original LANL source repository is public. CMNI has research primitives for Poisson Fisher information, model KL separation, response-mixture moments, sparse bias inference, scalar approximate-control-variate estimation, shared-reference normalization, common-cause covariance, grouped horseshoe posterior evaluation, grouped fixed-step HMC and deterministic blind-attribution data generation.

### Status addendum — 2026-11-21

This addendum supersedes any earlier status wording in this document where it conflicts with the exact-head evidence below.

**Committed public increments on `research/cmni-symtropy-v0`:**

- The pinned LANL AIACHNE source manifest and normalization audit are committed. The manifest records 22 verified source objects; the audit covers 21 experiment records. The candidate scale-factor fixture is reconstructed but still requires independent SciPy/original-helper parity.
- Shared-reference normalization now includes observation/reference Jacobians, separate covariance contributions, cross-experiment covariance and AIACHNE overlap geometry.
- The grouped three-scale horseshoe posterior, prepared evaluator and deterministic grouped fixed-step HMC/multi-chain diagnostics are authored in the standalone research kernel. This is not NUTS and is not a reproduction of the original AIACHNE Bayesian Monte Carlo algorithm.
- The blind causal-attribution generator is committed. The sealed-label harness, benchmark scoring, lineage-aware holdouts and calibration report remain outstanding.
- Common-cause covariance assembly is committed, including repeated source-definition checks and an explicit lineage-completeness qualification bit.
- A closed-form common-cause batch-design adversarial oracle is committed in `COMMON_CAUSE_BATCH_DESIGN_ORACLE_V1.md` and `research/cmni/fixtures/common-cause-batch-design-oracle-v1.json`. It is a synthetic analytical fixture, not a validated portfolio optimizer or a PFNS result.

**Exact-head CI evidence:**

- At commit `4e219329595fb3678958257c0ee0eed929d11c8a`, the Ubuntu, macOS and Windows workflow jobs completed with failure because `cargo fmt --check` found formatting differences across nine research-kernel source files.
- The logged test steps themselves reported 212/212 passing on Ubuntu, 114/114 on macOS and 142 passing / 1 ignored on Windows. These results do not turn the failed workflow into a passing qualification.
- Commit `368c39ae7a204071c6bc3a0ab5b5fd436162dc88` applies the formatter's expected changes to those nine files without changing logic, tests or gates.
- The common-cause oracle documentation and fixture were then committed at `ad1c7aa87e54fbab45cae7031cd7a38a23ec258d`.
- At the time of this addendum, the three new-head workflow jobs for `ad1c7aa87e54fbab45cae7031cd7a38a23ec258d` were in progress. Their conclusions are therefore **pending**, not green. Clippy, exact-head test, formatting and all-target build qualification must be recorded against a final immutable head before claiming a pass.

The research branch is currently unprotected. A commit on this branch is a public research artifact, not a protected release.

**Still not scientifically qualified:**

- Independent SciPy execution of all 21 normalization scale factors and overlap arrays.
- Joint inference of the interpolation-mean coefficients and exact matched Stan posterior parity.
- Simulation-based calibration and independently qualified multi-chain sampling; no claim of NUTS or original BMC reproduction.
- Delta-method versus Monte Carlo shared-reference propagation on nonlinear and tail-sensitive cases.
- A sealed blind attribution harness with null/positive controls, exact aliases, and held-out detector, campaign and reference lineages.
- An executable batch optimizer compared to exhaustive small-set oracles, greedy selection and the published measure-relaxed design method.
- Full reproduction of the published Coons–Huan multifidelity EIG estimator; the scalar ACV primitive is not that estimator.
- NMF-90 parity, independent detector-response validation and real experimental validation.

**Immediate execution order:**

1. Finish format, Clippy, build and test qualification on one final exact commit; preserve every existing gate.
2. Independently reproduce the LANL SciPy normalization and Stan inputs/posterior targets.
3. Qualify grouped inference with SBC, Stan comparisons, independent chains and ESS/MCSE for decision-relevant quantities.
4. Compare shared-reference delta-method covariance with Monte Carlo propagation, including induced cross-experiment covariance and posterior-coverage impact.
5. Implement the common-cause batch oracle as executable tests, then a discrete exhaustive oracle and greedy/joint portfolio baselines.
6. Complete the blind-label harness and publish stratified abstention, false-attribution and coverage results.
7. Extend to NMF-90 and independent detector-response benchmarks only after their matched inputs and reference codes are frozen.

A research capability becomes a benchmark result only after the preregistered comparison is executed and its exact inputs, commit, configuration, seeds, software environment and outputs are preserved. No superiority claim is made by this status update.
