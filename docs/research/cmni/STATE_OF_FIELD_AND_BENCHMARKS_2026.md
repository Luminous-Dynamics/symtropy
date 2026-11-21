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

The 2026 AIACHNE process and 2025 PARADIGM portfolio are published external baselines. The original LANL source repository is public. CMNI has research primitives for Poisson Fisher information, model KL separation, response-mixture moments, sparse bias inference, and a scalar approximate-control-variate estimator. The nuisance-profiled Fisher routine and tests have just been authored.

The AIACHNE reproduction, full MF-EIG implementation, blind generator, independent experiment-lineage benchmark, NMF-90 parity, and physical validation are not yet complete. The latest standalone CI run failed Clippy before the test step; new code is therefore unverified until the exact-head checks pass.
