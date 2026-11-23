# CMNI blind causal-attribution benchmark

**Status:** research protocol / implementation proposal  
**Scope:** synthetic qualification before any physical-mechanism attribution; not an experimental operating procedure and not evidence that a CMNI mechanism has been observed.

## 1. Objective

CMNI must distinguish a change in the physical process from a change in the measurement process. A flexible physical model can absorb detector, calibration, reference-spectrum, background, beam-characterization, or campaign effects and still fit the observations. A bias model can make the converse error by explaining a real physical change as an instrument artifact.

This benchmark makes that failure mode measurable. It evaluates whether a model can:

1. recover planted physical effects when measurement conditions are stable;
2. recover planted measurement effects when the physical process is stable;
3. separate both when they coexist;
4. abstain and report non-identifiability when their observable signatures are confounded;
5. preserve calibrated uncertainty under shared-reference covariance, response uncertainty, missing metadata, and model misspecification;
6. select an additional observation that resolves the ambiguity in a blinded follow-up.

The benchmark is a prerequisite for causal language. It is not a substitute for independent experimental replication.

## 2. Generative model contract

For experiment/run e and observation channel k, define

\[
Y_{e} \mid \theta,\eta, R_e \sim
\operatorname{Poisson}\left(
 t_e\, R_e\,\lambda_e(\theta,\eta)
 + b_e
\right),
\]

where:

- \(\theta\) are physical parameters and mechanism indicators;
- \(\eta\) are measurement, calibration, and campaign nuisance parameters;
- \(R_e\) is a detector-response realization, fixed only in the fixed-response control;
- \(t_e\) is exposure / live-time metadata;
- \(b_e\) is background, with a separate uncertainty model where applicable.

For continuous PFNS-style observations, use a separate raw-scale multivariate likelihood with the declared relative covariance convention. Do not silently substitute a log-Gaussian approximation for the raw-scale model.

Shared reference uncertainty must be represented once as a common latent quantity or as the corresponding joint covariance. It must not be duplicated as independent per-experiment noise.

The data generator must retain the true latent decomposition separately from the observations. It must support exact synthetic truth, controlled misspecification, and immutable seeds.

## 2A. Deterministic generator primitive now implemented

The standalone Rust crate now contains `blind_attribution.rs` and exports a deterministic generator. It produces a positive baseline spectrum and separate model-facing design columns for a global physical effect and a feature-conditioned measurement effect, with seeded lognormal noise.

Three basis geometries are implemented:
- **Orthogonal:** physical and measurement energy signatures are orthogonal.
- **Partial:** the measurement signature has a configured correlation with the physical signature.
- **Exact alias:** signatures are identical and experiment feature values are fixed to one, so only the sum of physical and measurement amplitudes is observable.

The API returns separate `BlindAttributionPublicData` and `BlindAttributionTruth` structures. This is a type-level separation only: the blind benchmark harness must serialize these to separate artifacts, hash/freeze the model-facing data, protect the truth file, and ensure fitting/model-selection code cannot access it.

Authored unit tests cover deterministic replay, positive observations, orthogonal basis geometry, requested partial correlation, exact alias equality and rejection of invalid exact-alias feature metadata. The generator is not yet the complete factorial challenge harness, lineage-aware split engine, scoring report, or blinded release process. Its Rust tests await exact-head CI execution.

## 3. Factorial challenge matrix

Generate a preregistered factorial suite crossing the following dimensions:

| Dimension | Required levels |
|---|---|
| Physical effect | null; single mechanism; two simultaneous mechanisms; energy-dependent effect |
| Measurement effect | null; detector-specific; calibration drift; shared-reference shift; background error |
| Confounding | orthogonal signatures; partial collinearity; exact observational equivalence |
| Covariance | diagonal; within-spectrum correlation; shared-reference cross-experiment correlation |
| Response | fixed known response; uncertain response ensemble; deliberately misspecified response |
| Metadata | complete; partially missing; noisy labels; campaign-level feature imbalance |
| Data quality | nominal counts; low counts; outliers; one influential campaign |
| Generalization | held-out energy region; detector; material; campaign; reference lineage |

Include negative controls in which the physical effect is exactly zero and the measurement effect is exactly zero. Include positive controls where each mechanism is independently identifiable. Include a hard abstention set where two explanations are observationally equivalent under the supplied measurement design.

The exact-equivalence cases are essential: the correct answer is “not identifiable from these observations,” not a forced attribution.

## 4. Frozen model ladder

Compare all models on identical training observations and identical held-out observations:

- **B0 — physical-only:** no extra measurement-bias layer.
- **B1 — independent-run nuisance:** run-specific effects with no metadata pooling.
- **B2 — feature-conditioned sparse bias:** metadata-conditioned energy basis and shrinkage.
- **B3 — hierarchical causal measurement model:** physical effects, detector/calibration/reference nuisance terms, and shared covariance represented separately.
- **B4 — oracle decomposition:** uses the known synthetic latent labels; diagnostic ceiling only, never a deployable competitor.

Where meaningful, include a fixed-response and response-uncertainty version of each eligible model. Freeze priors, exclusions, feature encoding, energy basis, optimization/sampling settings, and model-selection rules before revealing blind labels.

A model must not receive credit for recovering a feature effect if the feature is perfectly confounded with campaign or detector identity. Such a case belongs in the non-identifiability stratum.

## 5. Holdout and blinding protocol

- Split by complete experimental lineage, not randomly by individual energy bins.
- Keep every observation derived from a shared reference in the same leakage group unless the split explicitly tests reference transfer.
- Reserve detector-, campaign-, material-, and reference-lineage holdouts.
- Freeze the generator version, scenario manifest, seeds, model configurations, and scoring code before unblinding.
- Keep latent truth and blind scenario labels outside the model input path.
- Score the exact commit SHA and archive all inputs, outputs, environment metadata, and hashes.
- Repeat with multiple independently generated seeds; do not report a best-seed result as the benchmark result.

## 6. Metrics

Report metrics by scenario stratum and as aggregate distributions. Do not collapse all performance into one score.

### Attribution

- Physical-mechanism precision, recall, and false-attribution rate.
- Measurement-feature precision, recall, and false-attribution rate.
- Joint attribution accuracy when both effects are present.
- Abstention precision/recall on non-identifiable cases.
- False-discovery rate under the all-null negative controls.

### Prediction and uncertainty

- Held-out log predictive density.
- CRPS or a proper score appropriate to the observation likelihood.
- 50%, 80%, 90%, and 95% interval coverage with binomial uncertainty intervals.
- Interval width / sharpness conditional on achieved coverage.
- Simulation-based calibration rank diagnostics on tractable generated posteriors.
- Posterior predictive residual checks by detector, campaign, energy, and reference lineage.

### Identifiability and design

- Correct detection of rank deficiency / weakly identified directions.
- Discrete-hypothesis discrimination accuracy and calibration.
- Expected information gain error against a tractable oracle.
- Design regret relative to the oracle-best candidate.
- Cost-normalized information gain and wall-clock cost.
- Fraction of ambiguous cases for which the selected follow-up resolves the ambiguity in an independent simulation.

### Computational efficiency

- ESS per second and ESS per CPU-hour for posterior quantities of interest.
- Gradient evaluations, likelihood evaluations, memory, and wall-clock time.
- Failure / divergence / invalid-draw rates.
- Reproducibility across independent seeds and supported platforms.

## 7. Qualification gates

A result is not qualified by a single passing threshold. The release report must include the complete stratified scorecard and all failure cases.

Minimum gates:

1. **Truth integrity:** all blind labels, seeds, and latent truth are inaccessible to model fitting and feature engineering.
2. **Negative controls:** false attribution is reported with uncertainty; no unexplained systematic false discovery is accepted.
3. **Positive controls:** independently identifiable planted effects are recovered with calibrated uncertainty.
4. **Abstention:** exact-equivalence cases are explicitly marked non-identifiable; confident attribution is a failure.
5. **Coverage:** nominal interval coverage is assessed with binomial uncertainty and stratified by scenario; aggregate coverage cannot hide subgroup undercoverage.
6. **Leakage:** reference-derived observations and related energy bins cannot cross lineage holdouts accidentally.
7. **Robustness:** conclusions are reported across prior, covariance, response, metadata, and outlier perturbations.
8. **Reproducibility:** frozen inputs, code SHA, configuration, seeds, environment, and machine-readable receipts are archived.
9. **Independent check:** at least one posterior / likelihood path is independently implemented or compared against a mature reference implementation on matched synthetic cases.

Numerical acceptance tolerances must be selected from the number of replicates and the decision consequences before unblinding. Do not choose a convenient threshold after seeing the scores.

## 8. Required adversarial cases

- A physical effect and a detector bias with nearly identical energy dependence.
- A reference shift shared by multiple experiments, with one experiment incorrectly treated as independent.
- A detector-response perturbation that moves counts between channels while preserving total counts.
- A missing feature correlated with campaign, followed by a detector-held-out evaluation.
- A sparse-bias prior that suppresses a real but diffuse physical effect.
- A flexible physical model that absorbs a planted instrument artifact.
- A feature-conditioned bias model that explains away a real physical difference.
- A low-fidelity forward model whose discrepancy is correlated with the candidate mechanism.
- A prior-sensitive posterior with apparently good R-hat but poor scientific identifiability.
- An exact-equivalence case where no available candidate measurement distinguishes the hypotheses.

## 9. Relationship to existing CMNI work

This protocol operationalizes the existing research threads rather than replacing them:

- Symtropy #6538: mechanism identifiability and measurement-aware design.
- Symtropy #6569: multifidelity expected-information calculations and robust design.
- Symtropy #6572: sparse feature-conditioned measurement-bias inference.
- Symtropy #6596: faithful AIACHNE raw-scale horseshoe/BMC reproduction.
- Symtropy #6597: sampler qualification and diagnostics.
- Symtropy #6571: versioned PFNS and NMF-90 source-data ingestion.
- Symthaea #13064: evidence graph and identifiable measurement designs.
- Symthaea #13074: systematic measurement-bias attribution.

## 10. Implementation sequence

1. Add a deterministic synthetic generator with known physical and nuisance truth.
2. Add orthogonal positive controls, null controls, and exact-equivalence abstention cases.
3. Add grouped lineage splitting and blind-label isolation.
4. Run B0–B3 on identical frozen scenarios.
5. Add response uncertainty and shared-reference covariance.
6. Add calibration / SBC and held-out predictive scoring.
7. Add candidate follow-up measurements and oracle design regret.
8. Publish the scorecard and raw receipts before any real-data mechanism claim.

**Current status:** this is a benchmark contract. The existing CMNI kernel contains useful building blocks, but this complete blind benchmark, its generator, scoring harness, and qualification results have not yet been implemented or run.
