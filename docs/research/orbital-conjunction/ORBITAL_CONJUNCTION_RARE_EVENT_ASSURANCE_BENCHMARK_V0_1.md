# Orbital Conjunction Rare-Event Assurance Benchmark v0.1

**Status:** research charter / benchmark design; not an operational system  
**Parent work item:** [Symtropy #6746](https://github.com/Luminous-Dynamics/symtropy/issues/6746)  
**Upstream physical-model work:** [#4602 thermosphere drag and orbit uncertainty](https://github.com/Luminous-Dynamics/symtropy/issues/4602) · [#4622 POD-derived density observability](https://github.com/Luminous-Dynamics/symtropy/issues/4622)  
**Initial Symtropy baseline:** e864d8a96aadaa6183faf28884fdd3190d5e16a2  
**NASA CARA corpus snapshot inspected:** 43f0160f94d76a91805d86521c3e8fa0d09ecd68

---

## Executive decision

Build a composable rare-event assurance engine, not another monolithic probability estimator.

The proposal algorithm must be replaceable. The stable contract is the evidence chain:

**physical model + event definition → mode discovery → proposal fitting → frozen proposal → independent evaluation → uncertainty and coverage diagnostics → qualified estimate or abstention.**

The engine's first target domain is orbital conjunction risk. Its longer-term contribution is a way to qualify rare-event estimates when the event set is multimodal, curved, trajectory-defined, or sensitive to model and covariance uncertainty.

The differentiator is not a lower Monte Carlo count on an easy Gaussian case. It is a defensible estimate for a difficult trajectory-space event, with explicit evidence about what the method did not discover or cannot qualify.

## 1. Why the 2026 papers change the design

| Research | What it contributes | What it does not establish |
|---|---|---|
| Lorek et al., [Robust Importance Sampling for Rare Events via Constrained Gaussian Mixtures](https://arxiv.org/abs/2610.07485), 2026 | Separates event coverage from proposal fitting; constrains Gaussian component covariances to preserve a finite-variance guarantee under the paper's Gaussian assumptions | Does not guarantee discovery of every disconnected mode, low practical variance, or finite-budget precision |
| Wang, Katsevich & Tong, [Geometry-Informed Importance Sampling for Rare Events](https://arxiv.org/abs/2610.27442), 2026 | Uses local event-boundary geometry near an instanton and adds a component for event mass missed by the local approximation | Local regularity and one-sidedness assumptions do not automatically cover mode unions, switching minimizers, or arbitrary nonsmooth trajectory events |
| Chakroborty, Dhulipala & Shields, [Covariance-Free Bifidelity Control Variates](https://doi.org/10.1137/24M1658498), 2025 | Gives a bifidelity importance-sampling comparator that avoids directly estimating a high/low estimator covariance | Its random-ratio finite-sample behavior must be reproduced and measured; covariance-free does not mean variance-free or automatically unbiased |
| Sztamfater-Garcia et al., [An approximate model for computation of in-orbit collision probabilities using importance sampling](https://doi.org/10.1016/j.asr.2024.12.074), 2025 | A directly relevant orbital conjunction importance-sampling comparator, using a surrogate / full-dynamics correction strategy in LEO and GEO cases | Its reported cases do not establish performance on every trajectory geometry, multimodal uncertainty model, or operational collision-risk workflow |

**Design consequence:** retain constrained mixtures, geometric proposals, and bifidelity correction as plugins. Do not let any one paper's theorem become the assurance contract for the whole engine.

For the constrained-mixture theorem, preserve the paper's exact covariance parameterization and assumptions. A later normalizing-flow or nonlinear refinement must not inherit that finite-variance result without a separate proof.

For GeoIS, preserve which local boundary chart and regularity conditions support each claim. A local chart is evidence about that chart, not a global proof that the entire event set has been found.

For CVIS, reproduce the published estimator and its finite-sample behavior faithfully before modifying its coefficient, proposal, or control-variate construction.

## 2. The orbital problem is not just a TCA integral

A NASA CARA handbook example compares a 3D collision-rate treatment, Monte Carlo, and a conventional 2D conjunction-plane estimate for a non-rectilinear case:

- CARA 3D result: approximately 3.49528e-5.
- Monte Carlo: approximately 3.55e-5 from 3,553 hits in 1e8 trials, with a reported 95% interval approximately 3.44e-5 to 3.67e-5.
- Foster/Estes 2D estimate: approximately 2.33866e-6, about fifteen times lower in that example.
- The risk-rate peak occurred about nine seconds before TCA.

This is a specific handbook example, not a universal failure rate for 2D Pc. It demonstrates why a TCA-only estimate and a trajectory-level collision-rate calculation must not be treated as interchangeable.

NASA CARA's public EphemerisPc documentation makes a second distinction: integrated expected collision count Nc can differ from probability of at least one collision Pc when encounter segments blend; it reports PcMin / PcMax bounds under dependence assumptions rather than silently assuming one cumulative probability.

### 2.1 Four estimands, four labels

Every result must declare one of these target quantities:

1. **TCA snapshot Pc:** probability under a stated local conjunction-plane / TCA model.
2. **Pathwise first-contact probability:** probability that relative separation enters the hard-body region at any time over a declared window.
3. **Expected collision count / rate:** integrated expected count or rate over the declared trajectory interval.
4. **Cumulative Pc across encounters:** probability of at least one collision across a set of encounters, with dependence assumptions explicit.

These quantities can be related under specific assumptions, but they are not synonyms. No output may label Nc, a per-encounter Pc, and a cumulative Pc bound with the same unqualified field name.

## 3. Paradigm shifts

### A. From a single instanton to a multi-instanton event atlas

Model the rare-event set as a collection of candidate local boundary charts, not as one globally Gaussian blob.

Each chart should retain:
- stable mode / chart identity;
- how the mode was discovered;
- local boundary approximation and its validity domain;
- proposal component and normalized mixture weight;
- estimated contribution and uncertainty;
- overlap / double-counting treatment;
- residual search and omitted-mode challenge results.

A finite mode list never proves global completeness. Curated negative controls must plant a narrow or disconnected event mode that the discovery strategy is designed to miss. The assurance layer must report that miss or abstain.

### B. From a point at TCA to trajectory-space first passage

Define the high-fidelity event over a frozen time interval using the full relative trajectory and collision geometry. The active closest-approach time may change with the sampled initial state and model parameters. Multiple minima, encounter-segment merging, low relative velocity, and non-rectilinear motion can make the event boundary nonsmooth.

Use GeoIS-style local charts only where the local geometry assumptions are tested. Where the boundary is a union of charts or a nonsmooth envelope over time, make that an explicit extension and benchmark it separately.

### C. From one covariance matrix to a separated uncertainty budget

Do not collapse the following into one covariance without a defensible stochastic model:

- initial state covariance and cross-object cross-covariance;
- covariance realism / calibration error;
- force model and physical parameter uncertainty;
- atmospheric density, ballistic coefficient, attitude / area mode and drag uncertainty where applicable;
- hard-body radius / collision geometry uncertainty;
- epistemic model alternatives and non-probabilistic discrepancy.

Report model-conditional probabilities separately. A distribution over model alternatives, a worst-case envelope, and a calibrated posterior predictive probability are different claims.

### D. From proposal adaptation to frozen evaluation

Separate adaptation from measurement:
1. discover candidate modes and fit the proposal on a declared training stream;
2. freeze and digest the complete proposal;
3. draw an independent evaluation stream;
4. evaluate the full mixture density for every draw;
5. retain high-fidelity event labels and weighted contributions as evaluation evidence only.

All defensive components remain in the proposal denominator. No evaluation outcome may silently change the proposal whose performance it is used to measure.

### E. From lower variance to decision-relevant cost

Count all work required to achieve the result: mode discovery, fitting, low-fidelity training, high-fidelity trajectories, retries, and independent qualification. Compare at matched error, interval coverage, model fidelity, and compute budget.

### F. From one winner to an estimator portfolio

Support independent implementations of direct Monte Carlo, FORM/SORM, cross-entropy, subset simulation / sequential Monte Carlo, defensive and constrained Gaussian mixtures, geometry-informed proposals, residual correction, and CVIS. Select the estimator based on the declared event and its validated domain, not on a single leaderboard score.


### G. From proposal diagnostics to an independent residual-mass certificate

Add a separate verifier that bounds the probability mass remaining outside discovered event modes / local boundary charts. ESS, repeated-seed agreement, and stable estimates are valuable diagnostics, but none proves that a narrow disconnected mode was not missed.

For a Gaussian nominal model, whiten to X = mu + L Z with Z standard normal. Restrict to a finite cube [-b,b]^d and bound its complement conservatively by 2 d Phi(-b). Partition the cube into disjoint cells. An independent interval enclosure of the event function may exclude a cell only when it proves the event impossible throughout that cell. Every unresolved cell contributes its conservative Gaussian box mass to an upper bound.

The desired result is a residual upper bound U_uncovered, not a binary completeness assertion. Where a positive lower bound P_estimate_lower is available, relative coverage may be qualified only if U_uncovered <= epsilon_relative * P_estimate_lower. If P_estimate_lower is zero, only an absolute unresolved-mass claim is available; otherwise abstain.

This becomes a distinct child workstream: [#6766 — RARE-EVENT-RESIDUAL-MASS-CERTIFICATE-001](https://github.com/Luminous-Dynamics/symtropy/issues/6766). Its first B0 target is a static, low-dimensional Gaussian collision event. Extending it to trajectory-space events requires validated interval enclosures over both state cells and time, not sampled trajectories misrepresented as bounds.

The log-domain Gaussian tail/interval implementation in PR #6748 is useful numerical infrastructure but is not a rigorously outward-rounded probability enclosure and must not be labelled a certificate.
## 4. Mathematical and assurance contracts

Let p(x) be the declared nominal density and A the exact event definition. For a frozen proposal q(x) with support wherever p(x) times the event indicator is nonzero:

P(A) = E_q[1_A(X) p(X) / q(X)].

Implementation requirements:
- evaluate the full normalized mixture density, not the selected component density;
- compute log densities and log weights stably;
- declare proposal support and nominal support conditions;
- separate adaptation draws from evaluation draws;
- retain mode-wise weighted contributions;
- expose finite-sample diagnostics and uncertainty intervals;
- never interpret an observed absence of a mode as proof that the mode does not exist.

For low-fidelity event indicator I_L and high-fidelity indicator I_H, the residual identity is:

P_H = E_p[I_L] + E_q[(I_H - I_L) p/q].

It is unbiased only when the low-fidelity expectation is exact or separately estimated using a justified unbiased procedure, and the residual term uses valid samples and weights. Shared samples, adaptive fitting, random coefficients, or a biased low-fidelity estimator change the finite-sample analysis. CVIS must therefore be tested as published, not treated as a drop-in unbiased residual identity.

The Lorek covariance constraint Sigma_k greater than one-half Sigma may be applied only within the paper's Gaussian assumptions and precisely documented parameterization. It gives a finite-variance result, not practical precision or mode-discovery completeness. A flow or other refinement layer must not inherit that theorem without a separate proof.

GeoIS local-boundary guarantees apply only under the paper's stated smoothness / one-sidedness and approximation assumptions. Multi-mode unions, nonsmooth active-minimum switches, disconnected regions, and global event support require separate construction and evidence.

The qualification result is multi-axis. It must not compress all evidence into one confidence score.

Suggested dispositions:
- QualifiedForDeclaredModelAndEvent
- QualifiedWithRestrictedClaim
- NeedsIndependentReview
- UnqualifiedModeCoverage
- UnqualifiedCovarianceOrModel
- NumericallyUnstable
- InsufficientEvaluationBudget
- Abstain

## 5. Stable architecture

| Component | Owns | Must not own |
|---|---|---|
| Problem definition | Event semantics, window, HBR, units, frames, time system, intended use | Physical model truth |
| Nominal model manifest | Exact state/covariance, cross-covariance, epochs, force-model and parameter provenance | Proposal adaptation |
| Coverage planner | Candidate modes, charts, defensive support, residual-domain exploration | Global completeness claims without evidence |
| Proposal trainer | CFI / constrained GMM, cross-entropy, local geometry and other replaceable fitters | Evaluation truth |
| Frozen proposal | Normalized mixture, full density, seed policy, digest | Post-freeze adaptation |
| Independent evaluator | Evaluation samples, high-fidelity event labels, likelihood weights and raw contributions | Proposal tuning |
| Estimator portfolio | Direct IS, residual correction, CVIS, mode-stratified estimates | Model authority |
| Qualification engine | Intervals, coverage, ESS / weight tails, mode diagnostics, model sensitivity and abstention | Replacing source evidence |
| Evidence receipt | Exact inputs / heads / digests, proposal, samples, raw stats, costs, claim ceiling, replay | Operational maneuver authority |

The high-fidelity evaluator should be independently testable from the proposal fitter. A later process/capability boundary should prevent evaluator truth, hidden test outcomes, or future observations from leaking into adaptation.

### NASA CARA's existing multistep cascade is a required policy-level baseline

Do not treat the proposal portfolio itself as the novel contribution. NASA CARA already publishes a multistep algorithm that begins with 2D-Pc, performs data-quality and 2D-Pc usage-violation analysis, and escalates conditionally to 2D-Nc and 3D-Nc. Its extended wrapper can add SDMC. CARA presentations also describe an internal BFMC high-fidelity Monte Carlo tier; BFMC source is not publicly released.

The public usage-violation analysis explicitly considers:
- non-positive-definite covariance issues and their remediation;
- conjunctions that are temporally extended;
- collision-rate peaks offset from TCA;
- likely inaccuracy of the conjunction-plane approximation;
- convergence / quadrature limitations for more advanced methods;
- very low relative velocity and blended encounter interactions.

This is a mature industry design pattern: use cheaper methods by default and escalate only when declared usage conditions indicate that the assumptions may be violated. We must reproduce the full multistep policy, not only compare isolated algorithms.

**What we add beyond that baseline:**
1. An independent residual event-mass bound outside discovered rare-event modes / geometric charts.
2. Frozen-proposal, independent evaluation with log-weight / second-moment and repeated-run coverage evidence.
3. Explicit uncertainty decomposition for covariance realism, cross-covariance, force-model uncertainty and epistemic model discrepancy.
4. A pathwise first-contact target across the complete assessment window, separate from TCA Pc and CARA Nc.
5. A qualification disposition that can abstain when event support, model adequacy or statistical evidence remains insufficient.
6. A cost / risk escalation policy that can compare expected computation savings against false-safe probability and missed-mode exposure.

The benchmark should compare:
- CARA 2D-Pc alone;
- CARA's full PcMultiStep cascade (2D-Pc → usage violation / proxy correction → 2D-Nc → 3D-Nc);
- PcMultiStepWithSDMC where reproducible under the public release and rights permit;
- an independent high-fidelity reference. Internal BFMC can only be a comparator if independently accessible; it must not be assumed available from the public repository.

Use CARA's published usage-violation thresholds as pinned baseline configuration, then perform a separate sensitivity study. Do not tune CARA thresholds on the held-out evaluation set. Report escalation confusion matrices against independent reference, per-tier estimate bias / interval coverage, false-safe escalation misses, unnecessary escalations, total cost, and claim-coverage/abstention.

## 6. Industry baseline and fair comparison

NASA CARA's public toolset is the mandatory first external comparator, not a straw-man Monte Carlo baseline:

- PcCircle / Foster-Estes 2D Pc;
- Hall 2D-Nc and 3D-Nc;
- SDMC Pc and Keplerian two-body Monte Carlo;
- EphemerisPc on CCSDS OEM data, including encounter segments and Nc, PcMin, PcMax;
- covariance-realism assessment.

The 2025 orbital importance-sampling paper is another mandatory direct comparator. Generic rare-event methods (FORM/SORM, cross-entropy, subset simulation, defensive IS) remain useful baselines on synthetic truth fixtures.

**Do not claim to beat CARA on its intended simple Gaussian cases before reproducing them.** The credible differentiation target is more difficult: multimodal and trajectory-space probability estimation with independent mode-coverage challenges, model uncertainty, multi-fidelity correction, and explicit selective qualification.

For each comparison, freeze:
- event semantics, HBR, time window and dynamical fidelity;
- initial state, covariance, cross-covariance and frame/time conversion;
- baseline source commit / implementation and parameterization;
- number of high-fidelity model calls and total compute;
- random seeds / independent repetitions;
- precision, tolerance, confidence interval method and output units.

NASA CARA's CDM examples require care: its data README notes that CDM TCA can be rounded to the nearest millisecond and the states may need adjustment to actual TCA; the CARA multi-step wrapper performs relevant corrections. Its README also documents small output differences across MATLAB, BLAS/LAPACK, MKL and operating-system environments. Freeze the comparison environment and use a documented tolerance, not silent numerical pass/fail judgments.

## Operational boundary and industry-facing evaluation

A rare-event estimator is only one part of conjunction safety operations. Keep the stages separate:

1. **Screening:** discover candidate close approaches from ephemerides / catalogues.
2. **Risk assessment:** estimate collision likelihood, quantify its uncertainty, and assess consequences.
3. **Mitigation:** an accountable operator evaluates feasible actions, approval, execution timing, and secondary conjunction risk.

NASA explicitly distinguishes a CDM / screening alert from a collision warning: screening identifies proximity; a subsequent risk-assessment step is needed. NASA also describes risk as likelihood combined with consequence. This engine owns only the qualified probability-evidence contribution; it does not decide whether to manoeuvre.

NASA CARA published updated screening volumes implemented since March 2026, designed to capture 99% of conjunctions with Pc greater than 1e-4 under the stated orbit-regime screening design. Treat that as an **upstream screening-capture benchmark**, not a claim about the accuracy or calibration of any Pc estimator. Measure screening recall separately from Pc error, and never let a high-quality Pc estimate compensate for a missed candidate event.

ISO 23705:2026 (published June 2026) provides an international workflow and technical requirements for identifying, evaluating, and avoiding collisions among orbiting objects. Its scope spans data requirements, probability / consequence techniques, operator thresholds, and collision-avoidance manoeuvres. It also addresses cumulative survival and secondary collision risk. Use it as an interoperability and process benchmark: preserve epoch/time scale, reference frame, state/ephemeris, covariance, force-model / prediction metadata, object characteristics, operational status, and intended decision threshold. The standard's post-manoeuvre risk target and avoidance requirements belong to the responsible operator's approval process; they are not claims or authority granted by this research engine.

The benchmark therefore needs three distinct scorecards:
- **Screening capture:** candidate-event recall by orbit regime and declared screening volume.
- **Probability estimation:** matched event semantics, accuracy / interval coverage / model sensitivity / cost.
- **Decision evidence:** whether the probability and consequence packet supplies enough traceable information for a human/operator's declared policy, including secondary events. No autonomous CAM selection is in scope.

Useful primary references: [NASA Step 1 screening](https://www.nasa.gov/cara/step-1-conjunction-event-prediction/), [NASA Step 2 risk assessment](https://www.nasa.gov/cara/step-2-close-approach-risk-assessment/), [NASA unique conjunction events and March 2026 screening-volume update](https://www.nasa.gov/cara/unique-conjunction-events/), and [ISO 23705:2026](https://www.iso.org/standard/87556.html).

## 7. External benchmark corpus

Initial source inspected: [NASA/CARA_Analysis_Tools](https://github.com/nasa/CARA_Analysis_Tools), tree 43f0160f94d76a91805d86521c3e8fa0d09ecd68.

Candidate assets:
- DataFiles/PcTestCaseCDMs/ and CARA_PcMethod_Test_Conjunctions.xlsx;
- DataFiles/SampleCDMs/ with Alfano, Frisbee and Omitron cases, including high Pc, covariance-axis, minimum miss, minimum relative velocity, non-positive-definite covariance, and 3D-Nc examples;
- DataFiles/AlfanoInputData/Alfano_2009_Test_Cases/ with epoch and TCA data;
- DistributedMatlab/EphemerisPc/Example/HighRelVelLEO/ OEM ephemerides;
- CARA's released 2D / 3D / SDMC / EphemerisPc and covariance-realism implementations.

Pin every used file individually by upstream commit and cryptographic digest. Inspect the applicable NASA Open Source Software Agreement and Git LFS status before copying, building or redistributing corpus files. CARA outputs are reference implementations and comparison values, not universal ground truth.

For synthetic truth, use analytic fixtures first and independent high-budget Monte Carlo / quadrature where appropriate. Real conjunction data do not provide observed collision outcomes at the rare probabilities being estimated; agreement with another Pc calculation is not real-world frequency calibration.

## 8. Benchmark ladder

| Tier | Corpus | Main question |
|---|---|---|
| B0 — analytic kernel | 1D Gaussian tail; multivariate half-space; 2D / 3D Gaussian collision regions; coordinate / frame / unit / time transforms; singular covariance negatives | Is the estimator math and numeric kernel correct? |
| B1 — mode discovery | Smooth single mode; disconnected equal / unequal modes; narrow modes; mode separation sweeps; curved / near-tangent / switching boundaries; deliberate omitted-mode attacks | Does proposal coverage survive adversarial event geometry? |
| B2 — CARA reproduction | Alfano, Omitron, Frisbee, CDM, low / high relative velocity, non-rectilinear, cross-covariance, covariance pathology, actual-TCA correction | Can we reproduce accepted external methods and conventions? |
| B3 — trajectory events | Full-window first contact, multiple minima, risk peak away from TCA, merged encounter segments, full ephemeris, high-fidelity relative propagation | Does the estimator handle the actual time-dependent event? |
| B4 — model / multi-fidelity | Low/high fidelity ablations, residual correction, CVIS finite-sample study, covariance misspecification, heavy-tailed / mixture truth, alternative force models | Is the qualified estimate robust to modelling choices and efficient at matched fidelity? |

Data splits must be by complete conjunction / physical encounter and, where relevant, by satellite, event family or model regime. Adjacent or overlapping arcs from one event cannot be split across train and test and then counted as independent evidence.

## Direct Monte Carlo sample-budget floor

For a Bernoulli collision indicator with event probability p, the large-sample relative standard error is approximately 1/sqrt(N p). A normal-approximation 95% interval with relative half-width r therefore requires approximately:

`N ≈ (1.96 / r)^2 (1-p) / p`.

For r = 0.10 (a 10% relative 95% half-width), this is approximately 384/p when p is small:

| Nominal event probability | Approximate direct-MC draws |
|---:|---:|
| 1e-3 | 3.84e5 |
| 1e-5 | 3.84e7 |
| 1e-7 | 3.84e9 |
| 1e-9 | 3.84e11 |

This is a planning approximation, not a replacement for exact binomial intervals or repeated-run coverage measurements. At p = 1e-9, 1e8 direct draws have expected hit count 0.1 and approximately 90.5% probability of observing zero events.

This is why the importance-sampling and bifidelity portfolio matters. It is not a reason to accept a smaller sample blindly: methods must still meet matched error, interval coverage, proposal-support, model-fidelity, and independent-evaluation gates. The benchmark reports the actual number of high-fidelity physical-model evaluations and the full adaptation / training costs.

## 9. Metrics and proposed targets

Report by probability band, event geometry, physical regime, and mode:
- bias, relative error, RMSE, log-probability error;
- confidence interval width and empirical coverage across independent repetitions;
- effective sample size, max normalized weight, weight quantiles and second-moment diagnostics;
- mode recall, per-mode contribution, overlap, uncovered-domain findings;
- high-fidelity / low-fidelity calls, proposal-training cost, wall time P50/P95/P99, memory, and cost per target error;
- numerical failures, covariance pathologies, event-classifier disagreement and replay mismatch;
- selective coverage, false qualification and abstention rate;
- sensitivity to covariance, force model, atmosphere, HBR and model discrepancy.

**Probability / dimension sweep:** for analytic and synthetic truth fixtures, freeze strata at nominal event probabilities 1e-3, 1e-5, 1e-7 and 1e-9, and stochastic dimensions 1, 2, 3, 6 and 12. The 12D case represents joint sampling of two full 6D object states; where a validated relative-state Gaussian is formed with cross-covariance handled correctly, a 6D formulation is a distinct, separately identified stratum. Extend to nuisance / physical parameters only as separately declared dimensions, not by silently changing the base problem.

**Aspirational targets — none are claimed as achieved:**
1. 100% pass on analytic, geometry, frame, unit and time known-answer fixtures within pre-registered tolerances.
2. Zero false accepts on the finite adversarial corpus for deliberately omitted modes, invalid covariance, off-support proposals, stale model inputs and evaluation leakage.
3. 95% interval empirical coverage inside a predeclared 93–97% band for each sufficiently sampled, correctly specified test stratum. Sparse strata stay unqualified.
4. Reproduce chosen CARA outputs within documented platform-dependent tolerances before claiming improvement.
5. On selected multimodal / trajectory-space cases, at least 10x fewer high-fidelity evaluations than the strongest faithfully reproduced comparator at matched error and coverage. Investigate 100x only where the reference and compute budget make that claim testable.
6. Match the best admissible CARA method on the same assumptions before attempting to outperform it.
7. Any unresolved support, invalid covariance, out-of-domain physical model, or underpowered uncertainty estimate leads to abstention.

These are benchmark gates, not product guarantees. In particular, 95% confidence interval coverage requires many independent replications and cannot be demonstrated by one fixed deterministic run.

## 10. Integration boundary

- Symtropy #4602 supplies orbit/drag uncertainty and its model provenance.
- Symtropy #4622 supplies qualified POD-derived observations with arc, kernel, quality and uncertainty intact.
- This programme consumes frozen upstream products and evaluates declared conjunction events.
- Symthaea may reason over qualified estimates and their limitations, but does not become the physical model authority.
- No probability result from this work authorizes a spacecraft maneuver or an operational collision-avoidance decision.

## 11. Implementation sequence

**P0 — benchmark contract:** this charter, source inventory, ownership and claim ceiling.  
**P1 — analytic Rust kernel:** Gaussian tails / half-spaces, covariance validation, stable log densities, normalized mixture density, weighted indicator and deterministic known answers. Keep it dependency-light and independent of Bevy.  
**P2 — frozen proposal contract:** defensive mixture, full-density evaluation, sample provenance and negative controls.  
**P3 — CARA reproduction:** pin a small rights-cleared CDM / Alfano / Omitron subset; reproduce reference methods and platform tolerances.  
**P4 — proposal portfolio:** implement constrained GMM / CFI, geometry-informed charts, and CVIS replication as separately testable candidates.  
**P5 — trajectory-space event:** consume upstream trajectory products and implement pathwise first-contact evaluation with multiple-minimum fixtures.  
**P6 — qualification and model robustness:** independent mode challenges, covariance realism, misspecification, weighted uncertainty and selective abstention.

The first executable code slice should be P1. It can be developed and validated independently of the full Bevy workspace and does not need to wait for unrelated CI runners. No code or test-pass claim is made by this documentation-only increment.

## 12. References

- Lorek et al. (2026), [Robust Importance Sampling for Rare Events via Constrained Gaussian Mixtures](https://arxiv.org/abs/2610.07485).
- Wang, Katsevich & Tong (2026), [Geometry-Informed Importance Sampling for Rare Events](https://arxiv.org/abs/2610.27442).
- Chakroborty, Dhulipala & Shields (2025), [Covariance-Free Bifidelity Control Variates for Rare Event Simulation](https://doi.org/10.1137/24M1658498).
- Sztamfater-Garcia et al. (2025), [An approximate model for computation of in-orbit collision probabilities using importance sampling](https://doi.org/10.1016/j.asr.2024.12.074).
- NASA, [CARA Analysis Tools](https://github.com/nasa/CARA_Analysis_Tools).
- NASA, [Conjunction Assessment Handbook (2023)](https://ntrs.nasa.gov/citations/20230010583).
- ISO, [ISO 23705:2026 — Space systems, assessment of collision avoidance manoeuvre](https://www.iso.org/standard/84529.html).

## Claim ceiling

Analytic tests validate only the mathematical cases they cover. CARA reproduction establishes agreement with specific external methods under frozen conventions. Synthetic rare-event results do not prove operational orbit accuracy. A model-conditional Pc is not a calibrated real-world collision frequency. This programme is not flight qualification, collision-avoidance certification, or spacecraft maneuver authority.