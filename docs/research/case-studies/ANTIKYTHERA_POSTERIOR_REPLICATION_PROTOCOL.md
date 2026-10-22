# Antikythera calendar-ring posterior replication protocol

**Status:** Research protocol; no posterior computation has been run by this project  
**Goal:** turn the literature-only sensitivity seed into a reproducible scientific experiment  
**Primary question:** can an independent Rust implementation reproduce the published calendar-ring hole-count inference under the stated data selection, likelihood, priors, and parameterization?

## 1. Why the next step is raw-data reproduction

The metadata seed now represents the published results and their sensitivity to data preprocessing. It does not yet contain the hole coordinates, likelihood implementation, sampler output, or posterior diagnostics. Therefore it cannot establish an independent scientific result.

Woan and Bayley (2024) state that their analysis is based entirely on the hole-position measurements released with Budiselic et al. and describe two independently written samplers: an emcee ensemble-MCMC analysis and a dynesty nested-sampling analysis. Their paper reports 68% credible estimates of 355.24 (+1.39/-1.36) using all data and 354.08 (+1.47/-1.41) for a trimmed dataset. Table 2 provides discrete integer-hole-count probabilities under both data variants.

**Raw input candidate:** Thoeni, Budiselic, and Ramsey, *Replication Data for: Antikythera Mechanism Shows Evidence of Lunar Calendar*, Harvard Dataverse, DOI [10.7910/DVN/VJGLVS](https://doi.org/10.7910/DVN/VJGLVS). The BHI source links to this dataset and the 2024 paper says it used these measurements. The dataset's current file inventory, exact license/terms, byte count and hashes have **not** been verified in this project. Do not redistribute or embed the data until the terms and the actual files are reviewed.

## 1.1 Current access/verification state

The 2024 article cites version V3 of the 2019 Harvard Dataverse replication dataset (DOI `10.7910/DVN/VJGLVS`). The DOI and dataset citation are corroborated by the paper's bibliography and library metadata, but automated access to the repository's file listing was blocked by its verification page in this research pass. Therefore, the specific data files, file-level licenses/terms, version-specific metadata, byte sizes, and SHA-256 values remain **unverified**. Do not infer CC0 merely because Harvard Dataverse generally encourages CC0; its policy explicitly permits depositors to set other terms and file-specific restrictions.

The current research seed has an `AnalysisRun` with status `planned`, no input or output artifacts, and explicit limitations. That is intentional. Change it to `inputs_verified` only after downloading the exact data version, confirming the license/terms, computing hashes from actual bytes, and reviewing the inclusion/exclusion manifest. Never relabel a planned run as executed because its protocol or expected values exist.

## 2. Freeze input provenance before analyzing it

The first deliverable is an input manifest, not a sampler:
- retain the dataset DOI and the Dataverse version identifier;
- list exact filenames, file IDs, byte sizes, format, source license/terms, and SHA-256 digests;
- keep untouched source bytes separately from normalized working data;
- preserve source column names, coordinates, units, fragment/section labels, and row identities;
- include transformation code and a report of rows/holes included or excluded;
- fail closed if the downloaded file version, digest, coordinate units, or source terms are unclear.

The paper describes 81 measured hole-centre points, with two one-point sections omitted because they provide no parameter constraints. Its base model uses 79 points from sections S1, S2, S3, S5, S6, and S7. For the trimmed analysis, it excludes the short S5/S6 segments and the first and last holes from S1/S2/S3/S7: hole IDs 2, 23, 24, 32, 33, 69, 76 and 81. Preserve both dataset variants as explicit named manifests; never implement “trimmed” as an undocumented filter.

## 3. The published model to reproduce

The 2024 paper describes the following model. This is a transcription of its stated assumptions, not a claim that the assumptions are uniquely correct.

### Parameters

For six usable fragment sections, each section has an unknown two-dimensional centre `(x0, y0)` and rotation `alpha`. The global parameters are:
- `N`: inferred total number of holes;
- `r`: circle radius;
- `sigma_r`: radial error scale;
- `sigma_t`: tangential error scale.

That produces 22 unknowns: 3 × 6 section parameters plus 4 global parameters.

### Geometric and noise assumptions

- Hole centres lie on a circular ring with regularly spaced nominal angles parameterized by `N`.
- The six usable fragments are coplanar; each section can have its own translation and rotation.
- Radial and tangential errors are modeled separately as independent zero-mean Gaussian components.
- The positional-noise scales are shared across the modeled holes.
- The baseline continuous model treats `N` as continuous, allowing a possible spacing discontinuity at the gap.
- The source uses uniform priors for `N`, radius, fragment-centre positions, and rotations; Jeffreys priors are used for the radial and tangential noise scales.
- In the integer-count analysis, the source evaluates integer N against the continuous marginal posterior and also reports a discrete nested-sampling cross-check.

All parameter ranges, angle conventions, coordinate units, normalization constants, data exclusions, and any implementation-specific constraints must be made explicit in a versioned model specification before sampling begins.

### Reference computation

The paper reports 100 ensemble walkers for 35,000 steps, with the first 15,000 discarded as burn-in and the remaining samples thinned by 10. Its nested-sampling cross-check used 2,000 live points and stopped when the remaining evidence was below 1% (`delta ln Z < 0.01`). These are reference settings to reproduce and assess, not universal settings that automatically guarantee convergence.

## 4. Rust-first implementation lane

Use the existing `humanity-history-core` to preserve the input and analysis manifests. Put numeric computation into an isolated research crate only after the data contract has been reviewed, for example `antikythera-inference`; keep it independent of Bevy and the rendering layer.

Candidate sampler research found Rust MCMC packages such as [general-mcmc](https://docs.rs/general-mcmc/latest/general_mcmc/), which advertises Metropolis–Hastings, HMC, NUTS, and Gibbs algorithms. This is only a candidate dependency, not an approved choice. Audit its license, maintenance, algorithm contracts, deterministic seeding, diagnostics, and suitability for this posterior before adopting it. No equivalent nested-sampling dependency has been approved in this protocol.

Proposed architecture:
1. `measurement-ingest`: parse the source data into typed rows with row IDs, section IDs, and millimetre coordinates; emit an ingest audit.
2. `calendar-ring-model`: pure likelihood/prior functions over a typed 22-parameter vector; no sampler code in the likelihood.
3. `sampling-adapter`: seedable proposal/sampling engine with iteration trace, acceptance diagnostics, and serializable configuration.
4. `posterior-summary`: credible bounds, marginal summaries, integer-N probability table, posterior predictive checks, chain diagnostics, and reproducible artifacts.
5. `compare-reference`: compare source-reported values against locally computed results without overwriting the original estimates in the evidence graph.

A later, separately audited nested-sampling implementation or a clearly isolated reference harness should cross-check posterior mass and model evidence. A sampler name or a successful compile is not validation of statistical correctness.

## 5. Benchmarks and acceptance gates

The first gates are data and model correctness, not runtime speed.

### Gate A — data fidelity

- 100% of parsed rows retain a link to the original file and row/record locator.
- Included/excluded row IDs for the full and trimmed analysis match a versioned manifest exactly.
- Units and coordinate frames are explicit; no silently inferred conversions.
- SHA-256 hashes are calculated from the downloaded bytes by the ingest command, then independently checked from the stored manifest.
- A changed source version or changed data digest invalidates the prior run manifest rather than silently updating historical results.

### Gate B — likelihood correctness

- Check the likelihood on synthetic coordinate sets with known circle radius, hole count, rigid fragment transforms, and independently controlled radial/tangential noise.
- Use rotation/translation metamorphic tests: transforming a fragment and applying the corresponding nuisance parameters should preserve the relevant likelihood.
- Use limiting cases (zero noise, equal isotropic noise, one-fragment subproblems) to test expected invariants.
- Compare an independently coded reference likelihood at at least 100 fixed parameter vectors; the allowed numerical tolerance must be set before comparing results.

### Gate C — parameter recovery and calibration

- Run reproducible synthetic-recovery experiments at multiple N values and error scales.
- For 100 seeded synthetic datasets, report bias, interval coverage, convergence failures, and runtime; do not report only the successful runs.
- Provisional target: for a calibrated 90% interval, empirical coverage between 85% and 95% across a sufficiently large simulation suite, with binomial confidence intervals. Increase trials if intervals are too wide to assess this target.
- Verify sensitivity to initialization and random seed; all seeds and sampler configurations are recorded.

### Gate D — reproduction of the published results

Targets must be preregistered in a run manifest before comparing computed output to the paper:
- reproduce the full-data and trimmed-data continuous summaries;
- compare 68% bounds to the published bounds with provisional absolute tolerance 0.15 holes, while also publishing posterior plots and convergence diagnostics;
- compare the source-reported discrete probabilities for every N=349–361 under each dataset with provisional absolute tolerance 0.02 per probability, or explain the discrepancy before relaxing this bound;
- reproduce the qualitative direction of the sensitivity result: trimming raises the reported N=354 probability and lowers the N=355 and N=356 probabilities;
- compare independent samplers or implementations. Do not claim model evidence reproducibility merely because two chains converge to similar medians.

These tolerances are recorded as the comparison rule on the seed's planned analysis-run manifest. They are initial engineering targets, not published confidence bounds or already achieved outcomes. Refine them using simulation-based calibration and algorithmic diagnostics in a preregistered protocol, never retroactively to make a failed reproduction appear to pass.

### Gate E — diagnostics and criticism

Every run must report:
- convergence diagnostics and effective sample size for all reported parameters;
- chain/walker trace plots and autocorrelation diagnostics;
- posterior predictive residuals in radial/tangential coordinates;
- priors and posterior marginals, including correlations such as radius versus N;
- sensitivity to the coplanarity assumption, segment boundaries, noise family, and endpoint-hole handling;
- failed initializations, divergences (for gradient methods), sampler acceptance, and computational cost.

A proposed initial MCMC target is split-chain \(R\hat_\rm{hat}\) below 1.01 and effective sample size above 1,000 for key global parameters, subject to sampler-specific diagnostic validity. These targets alone do not establish correctness.

## 5.1 Model criticism ladder: test assumptions, not just the published parameters

The publication itself identifies possible extensions: relax the coplanarity assumption by allowing out-of-plane section displacement/orientation (adding 18 parameters), and infer segment-division locations rather than treating the original segmentation choices as fixed. These are not automatic upgrades: extra degrees of freedom can widen uncertainty and may overfit.

Run these as sequential, preregistered model variants:
1. **Baseline reproduction:** fixed six-section segmentation, coplanar geometry, source-stated radial/tangential error model.
2. **Posterior predictive checks:** compare observed and simulated hole positions in radial/tangential coordinates, inspect residual autocorrelation and segment-specific patterns, and publish discrepancy summaries.
3. **Noise-model sensitivity:** test a small, justified set of alternative residual models only if diagnostics show tail/heavy-error or heteroscedastic behavior.
4. **Segmentation uncertainty:** introduce uncertain segment boundaries only after encoding the competing boundary hypotheses and a prior/penalty that prevents unrestricted boundary search from trivially improving fit.
5. **3D/coplanarity sensitivity:** allow out-of-plane degrees of freedom only where the source geometry or systematic residuals justify the added complexity.
6. **Out-of-sample/model-check comparison:** compare predictive checks and, where the likelihood/model permits it, held-out predictive scores. Report the complexity and uncertainty cost; do not choose solely by in-sample likelihood.

For every variant, retain a separate model-specification version and analysis-run manifest. Report all failed convergence and posterior predictive checks. A more flexible model is not automatically a better explanation; the key result may be that current evidence cannot discriminate between variants.

## 6. What not to conclude

- A source-reported posterior table is not our posterior run.
- A trimmed dataset is a sensitivity condition, not an independent replication.
- A 68% credible interval does not equal a 68% probability that a particular mechanical reconstruction is historically true.
- Matching a ring count does not prove a calendar interpretation: inscriptions, use, function, gearing, and cultural context are additional evidence channels.
- Posterior probabilities are conditional on the likelihood, priors, measured points, fragment segmentation, noise model, and coplanarity assumptions.
- The mechanism is incomplete; model adequacy and missing-data uncertainty must be reported.

## 7. Local execution sequence

Once the data and licensing gates are complete:
1. run the structural validation and ingest audit locally;
2. run synthetic likelihood tests before connecting a sampler;
3. validate the sampler on known synthetic cases;
4. run a smoke-size reference fit, inspect diagnostics, then scale toward reference settings;
5. run full and trimmed analyses with recorded data digests and random seeds;
6. compare results against the preregistered tolerances;
7. publish the model, data manifest, sampler config, logs, and analysis report through a reviewable PR.

Do not wait for CI to develop the likelihood tests, data schema, or synthetic fixtures. But do not call the paper reproduced until the data, likelihood, priors, sampler diagnostics, and comparison tolerances have all been checked locally.
