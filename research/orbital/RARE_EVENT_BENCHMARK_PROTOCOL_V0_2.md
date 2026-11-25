# Orbital Rare-Event and Decision-Grade Assurance Benchmark Protocol v0.2

**Date:** 2026-11-25  
**Status:** research benchmark contract; targets below are prospective unless explicitly labelled as existing software tests  
**Scope:** conjunction event probability, adaptive rare-event sampling, multi-encounter trajectory risk, mitigation portfolio assessment, and evidence-backed decision support  
**Claim ceiling:** benchmark protocol and synthetic/known-truth qualification only; no flight certification or maneuver authority

## 1. Executive thesis

The benchmark target is not "produce a more accurate Pc number." It is:

> Given a versioned orbital state estimate, an independently challenged event universe, and a declared operating horizon, produce a reproducible, time-uniformly qualified risk statement and show how every proposed mitigation changes the entire future conjunction and consequence portfolio.

The unit of analysis should be one shared trajectory realization over a complete horizon, not one TCA/window in isolation. The unit of evidence should be a replayable sample and its complete proposal density, not a reported importance weight. The unit of decision should be a qualified action portfolio, not a scalar score that can compensate for a new high-risk conjunction or missing evidence.

A benchmark win requires matched event semantics, physical-model fidelity, support coverage, confidence level, decision threshold, resource budget and output claim ceiling. Lower sample count alone is not a win.

## 2. Paradigm shifts to evaluate

### 2.1 From one-TCA probability to pathwise horizon risk

For a joint uncertain trajectory X and a complete set of encounter windows W_j, define one event:

E_any = union_j E_j, where every E_j is evaluated on the same X.

Estimate P(E_any), the disjoint first-contact decomposition, and survival through the full horizon as separate estimands. Do not add marginal per-window Pc values or multiply independent no-contact probabilities unless the dependence/factorization is independently proved. Overlapping windows and repeated encounters must count one pathwise collision event once.

Required evidence: the event definition, frame/time scale, HBR, complete-window claim, trajectory realization ID, all window dispositions, unresolved intervals, and first-contact enclosure when justified.

### 2.2 From proposal fitting to coverage-first, proof-carrying sampling

Proposal construction has two distinct obligations:
1. **Coverage:** identify and bound probability mass in every relevant event mode, including modes not found by the proposal optimizer.
2. **Fitting:** reduce variance and computational cost after coverage is established.

For each adaptive draw, carry the selected residual proposal identity and the complete defensive-mixture q_t identity. Verify the actual branch/atom transform and reconstruct p/q_t independently. A digest pin is not proof that a particular binary or random generator executed; a finite categorical ticket oracle is not a continuous Gaussian/GMM domination proof.

The end state should be a continuous proposal execution receipt that binds:
- immutable event/target measure and support-closure evidence;
- mixture components, weights, density evaluator and version;
- sampler binary/build identity and random stream/counter identities;
- input state and proposal selected from prior, not future, observations;
- generated trajectory identity and complete q_t(X) / p(X) densities;
- exact or outward-rounded likelihood ratio and bounded-observation proof;
- deterministic replay receipt plus independent negative controls.

### 2.3 From point estimates to decision-grade time-uniform evidence

Use confidence sequences/e-processes whose guarantees survive adaptive sampling and stopping. Evaluate coverage over every time point on each path, not just a handful of selected stopping rules. Stop policies (threshold, interval width, budget, fixed horizon) are secondary diagnostics.

The statistical guarantee is conditional on the proposal actually generating the draws and on its predictable bounded-mean assumptions. It does not prove the event universe, orbital model or covariance is complete.

### 2.4 From per-action deltas to coupled portfolio outcomes

A maneuver changes an entire future catalog. A set of individually safe actions can interact, introduce a new conjunction, move risk into another time window, alter debris exposure or change maneuverability/coordination. The post-action evaluation must use the complete maneuver ephemeris against the full catalog and evaluate joint action sets, not add single-maneuver benefit estimates.

Keep these layers separate:
- physical risk gates (hard, non-compensatory);
- action feasibility and coordination;
- consequence-model uncertainty;
- resource/fuel/time cost;
- signed consequence delta and robust regret;
- evidence completeness and explicit abstention.

Do not clamp a negative consequence delta to zero. No-action is exactly zero; negative means a robustly adverse consequence change, not missing data.

### 2.5 From self-consistency to independently falsifiable evidence

A digest proves that data match a digest, not that the data are complete or true. Every producer inventory needs an independent challenger: mode grammar, world/model applicability, catalog completeness, source revision checks, planted common-mode omissions, independent arithmetic or a separate implementation. Unknown/unbounded is an abstention state, never an empty set or zero risk.

## 3. Competitor and standards matrix

| Family | Required baseline | What it measures | Transfer boundary |
|---|---|---|---|
| NASA operational practice | NASA CARA assessment and mitigation workflow; full catalog screening and coordination | Operational process parity, catalog screening, event trend/decision workflow | CARA parity is not independent physical truth or approval for deployment |
| NASA analytical Pc | Foster/Elrod 2D, analytic 3D, 3D-Nc / Pc3D_Hall, covariance realism | Numerical and implementation parity across established conjunction methods | Report each method's assumptions and supported regime |
| Monte Carlo | SDMC / brute-force Monte Carlo with independent high-fidelity propagation | Reference sampling error and expensive truth approximation | BFMC is not an oracle at rare probabilities without uncertainty bounds |
| Orbital rare-event | Ketzner & Elgohary cross-entropy Gaussian/GMM orbital conjunction method | Orbital-specific rare-event acceleration, including repeated encounters | Reproduce exact published cases; do not overstate transfer to other regimes |
| Orbital surrogate IS | Sztamfater-Garcia et al. approximate dynamics plus importance sampling | Propagation-cost reduction under the stated surrogate correction | Must separately measure surrogate/model bias and proposal variance |
| Early-orbit / large uncertainty | AMOS 2026 large poorly characterized uncertainty cases; GMM/unscented-transform comparisons | Weakly observed, non-Gaussian early-orbit stress | Do not treat a screening result as calibrated Pc |
| Constrained GMM | Lorek et al. coverage/fitting separation and finite-variance constrained proposal | Multimodal coverage, weight tails, estimator variance and cost | Published experiments are general rare-event problems, not orbital truth cases |
| Geometry-informed IS | GeoIS boundary/instanton-informed component plus residual coverage component | Event-boundary geometry and residual mass robustness | Published examples are not spacecraft conjunctions |
| Anisotropic / control-variate IS | Li et al. beyond exponential tilting | Tail geometry, coefficient of variation, proposal shape and control variates | Published examples include stochastic KdV, not orbital conjunctions |
| Adaptive IS + anytime confidence sequence | PLANS 2027 BO-guided AIS, defensive mixture and anytime empirical-Bernstein bound | Adaptive black-box failure-probability certification | Runway-detection results are not orbital results; generic architecture is prior art |
| Symbolic/statistical hybrid | DiSE (ATVA 2026) symbolic closure + per-leaf confidence sequences + joint gain/cost scheduler | Structural residual plus statistical uncertainty under optional stopping | Generic hybrid is prior art; orbital geometry/model/evidence obligations must be distinct |

References:
- NASA CARA: https://www.nasa.gov/cara/
- NASA CA Handbook Volume 1, August 2026: https://www.nasa.gov/wp-content/uploads/2026/08/ca-handbook-volume1-nasa-sp-20205011318-rev2-vol1.pdf
- NASA mitigation workflow: https://www.nasa.gov/cara/step-3-close-approach-risk-mitigation/
- NASA unique conjunction screening volumes: https://www.nasa.gov/cara/unique-conjunction-events/
- ISO 23705:2026: https://committee.iso.org/standard/87556.html?browse=ics
- Ketzner & Elgohary, 2026 cross-entropy orbital conjunction analysis, DOI 10.2514/1.G009204: https://doi.org/10.2514/1.G009204
- Sztamfater-Garcia et al., 2025 surrogate-dynamics IS, DOI 10.1016/j.asr.2024.12.074: https://doi.org/10.1016/j.asr.2024.12.074
- Lorek et al., constrained GMM, arXiv:2610.07485: https://arxiv.org/abs/2610.07485
- Wang, Katsevich & Tong, GeoIS, arXiv:2610.27442: https://arxiv.org/abs/2610.27442
- Li et al., anisotropic / control-variate IS, arXiv:2610.27434: https://arxiv.org/abs/2610.27434
- Kujur, Khanafseh & Pervan, PLANS 2027 abstract 17518: https://www.ion.org/plans/abstracts.cfm?paperID=17518
- Ray & Thakkar, DiSE / "How Often Does Your Program Fail?", ATVA 2026: https://conf.researchr.org/details/aplas-atva-2026/atva-2026/10/How-Often-Does-Your-Program-Fail-

## 4. Benchmark corpus and independent truth

### A. Exact synthetic and finite-oracle cases

- Isotropic/anisotropic Gaussian hard-body disk and sphere cases with high-precision independent integration.
- Multimodal and disjoint collision regions, including a late mode absent from a fitted proposal.
- Correlated and perfectly correlated repeated encounter windows.
- Overlapping, nested, touching and duplicate windows; event-union invariance under window subdivision.
- Exact first contact, tangency, endpoint contact, near miss, multiple minima and multiple entries/exits.
- Threshold-adjacent values above, below and exactly on the threshold, with strict/inclusive semantics tested.
- Defensive mixture, zero target-density atoms, residual support gaps and exact q_t density ratios.
- Exhaustively enumerable adaptive sampling paths with exact path probabilities and all-times interval coverage.

### B. Orbital truth and model stress

- Independently implemented high-fidelity force-model propagation and independent covariance propagation.
- Long-duration and repeating conjunctions with shared initial-state and force-model uncertainty.
- Correlated cross-object errors, process-noise temporal dependence and model-family ambiguity.
- Covariance under-dispersion, over-dispersion, singularity, asymmetry and covariance-realism mismatch.
- HBR/shape uncertainty and plausible object geometry.
- Early-orbit, sparse observation, weakly observed and large/non-Gaussian uncertainty cases.
- Maneuver ephemerides that create, move or elevate secondary conjunctions.
- Multi-operator and maneuverable-secondary coordination cases.
- Missing model, missing world/model pair, common-mode inventory omissions and post-hoc model selection.

### C. Public screening and interoperability

- NASA CARA's current screening-volume page states that its revised, orbit-regime-specific screening volumes (implemented since March 2026) are designed to capture 99% of conjunctions with Pc > 1e-4 in the current orbital environment. Treat this as a published operational screening-design target for its stated environment, not a universal recall guarantee or a known-truth answer key: https://www.nasa.gov/cara/unique-conjunction-events/
- NASA's August 2026 handbook recommends a mitigation threshold of 1e-4 and a goal at least 1.5 orders of magnitude below it (approximately 3e-6). Its guidance also calls for full-catalog screening of the maneuver ephemeris, no additional conjunctions above threshold, and coordination when the secondary is maneuverable: https://www.nasa.gov/wp-content/uploads/2026/08/ca-handbook-volume1-nasa-sp-20205011318-rev2-vol1.pdf

Use TraCSS / public CDM datasets for schema, replay, trend and screening-pipeline compatibility, not as complete ground truth for physical collision probability. Pin exact CDM versions, source revisions, frames, time scales and source timestamps.

## 5. Metrics and acceptance gates

### Safety / event completeness

| Metric | Synthetic target | Operational/physical claim |
|---|---:|---|
| False-safe on finite adversarial event/catalog corpus | 0 | No false-safe claim without independent physical event/catalog validation |
| Missed finite target-support atoms vs frozen manifest | 0 | Manifest completeness remains independently qualified |
| Double count across overlap/repeated windows | 0 | Shared trajectory and full horizon required |
| Newly created above-threshold post-maneuver conjunctions | 0 accepted | Full catalog screening required |
| Missing coordination when secondary is maneuverable | 0 accepted | Operator coordination evidence required |
| Missing or ambiguous coalition outcome rows | 0 silently accepted | Global abstention |

### Statistical quality

- Simultaneous all-times coverage: on every exact finite path, count a miss if the true probability is outside the confidence sequence at any checkpoint. For each nominal 90%, 95% and 99% sequence, exact finite-law miss probability must be no greater than alpha.
- Report threshold-triggered stopping, lower-bound stopping, width stopping, budget stopping and fixed-horizon stopping, but do not replace the all-times criterion with these selected policies.
- On preregistered Monte Carlo suites, report coverage calibration and confidence intervals for the miss rate. Simulation is evidence, not proof of the theorem or draw-generation assumptions.
- Report false-safe threshold decisions separately from interval coverage and from estimator bias.

### Proposal quality and efficiency

Report, separately for each proposal and adaptive state:
- target-support and discovered-event-mode recall;
- residual proposal support gaps and independently bounded undiscovered event mass;
- complete-mixture max p/q, max normalized weight, second moment, variance and ESS fraction;
- fraction of draws in defensive vs fitted component;
- proposal fitting and mode-discovery evaluations;
- expensive propagation counts;
- total compute (CPU/GPU hours), wall time, memory and replay cost;
- interval width and first correct decision time at fixed alpha.

No proposal wins by ESS alone if it misses modes. Finite variance is necessary but does not guarantee useful fixed-budget performance.

### Decision / portfolio quality

- exact joint-action enumeration parity on small candidate universes;
- zero new threshold-exceeding conjunctions after mitigation;
- signed worst-case consequence delta over all admitted joint scenarios;
- worst-case loss, minimax regret, nominal expected consequence, nominal CVaR and Wasserstein-robust CVaR as separate outputs;
- resource/fuel/time cost, coordination latency and time-to-decision;
- abstention quality: detect every planted missing source/model/coalition outcome and do not emit an optimum.

## 6. Matched-comparison protocol

Every reported comparison must freeze:
1. event-definition digest and time horizon;
2. joint state/covariance/force-model assumptions and their source revisions;
3. collision-radius and event geometry;
4. trajectory propagator and fidelity level;
5. candidate proposal families, seed policy and proposal-fitting budget;
6. confidence level, stopping rule and threshold semantics;
7. full catalog and action/coordination universe;
8. compute budget and measurement methodology.

Report two tournaments:

**Fixed-resource tournament:** equal total wall-clock / propagation budget. Report coverage, bias, interval width, false-safe and decision quality.

**Matched-assurance tournament:** methods must achieve the same nominal time-uniform coverage, mode-coverage evidence, physical-model fidelity and decision threshold; compare resources required to attain that assurance.

Include crude MC, BFMC/SDMC, CE-GMM, constrained GMM, GeoIS, surrogate-dynamics IS, anisotropic/control-variate methods, fixed defensive IS, BO-guided adaptive IS, and Symtropy's hybrid structural/statistical pipeline. Preserve each baseline's author-faithful settings.

## 7. Prospective improvement targets

These are research targets, not achieved results:

- 100% finite-oracle event and support-manifest conformance; zero false-safe in the preregistered adversarial corpus.
- 90/95/99% simultaneous finite-path coverage at or above nominal confidence for each method tested.
- At matched physical fidelity and assurance, aim for at least 10x fewer expensive trajectory evaluations than BFMC on selected rare-event families; report cases where this target is not reached.
- On multimodal cases, require 100% recall of planted modes in the finite truth set and an independently bounded residual mass for continuous cases.
- Full post-action catalog sweep for every candidate portfolio; zero accepted new above-threshold conjunctions.
- End-to-end replay of every reported decision certificate from frozen inputs, exact source commits and evidence receipts.
- Zero unqualified authority: the benchmark and optimizer emit evidence/decision support only, never maneuver authorization.

## 8. Current software work mapped to this protocol

- PR #7407: finite defensive proposal execution conformance, complete-mixture identity and verified-draw bridge to fixed-stake, PrPl-EB and Hedged.
- PR #7438: exact finite-support proposal coverage, max likelihood ratio, second moment, variance and ESS diagnostics.
- PR #7439: exact adaptive receipt-path coverage under all-times and selected stopping-policy diagnostics.
- PR #7443: pathwise multi-encounter event reducer composed into finite proposal atoms, including exact support-manifest binding.
- PR #7412: exact joint-action catalogue with capacity/coordination and signed consequence delta; source consequence calculation remains caller-supplied.
- Issue #7466: bind coalition consequences to the full joint physical-world/loss-model receipt and independent loss-model-universe result.

These are draft software/reference increments. Their exact-head CI, continuous proposal qualification, physical model validation and external operations review remain separate gates.

## 9. Claim and reporting rules

- Separate implemented, structurally checked, exact-finite-oracle-passed, CI-passed, independently validated, physically validated, and operationally qualified.
- A queued or skipped job is not a pass. A test written is not a test passed. A hash is not an authenticity or completeness proof.
- Use "SOTA comparison" only after author-faithful reproduction on the same frozen inputs and matched assurance. Do not compare sample counts across unrelated benchmarks.
- Publish raw fixtures, hashes, versions, scripts, full logs, negative controls, result summaries and failure cases.
- Preserve abstention as a valid outcome when any critical evidence is missing, stale, incomplete or ambiguous.
- No single weighted score may override a hard collision-risk gate, full-catalog requirement, coordination requirement or rights/safety constraint.
