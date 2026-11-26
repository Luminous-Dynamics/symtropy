# Paired Counterfactual Mission Risk v0.1

**Status:** research design / benchmark preregistration proposal  
**Tracking:** [#7525](https://github.com/Luminous-Dynamics/symtropy/issues/7525)  
**Claim ceiling:** no implementation or physical qualification is claimed by this document.  
**Authority:** none; this research layer must never grant manoeuvre authority.

## 1. Research question

Can a mission-risk evaluator compare manoeuvre portfolios more efficiently and more reliably by evaluating each portfolio against the same exogenous uncertainty realizations, while independently propagating the post-action trajectory for each portfolio?

The proposed unit is a **paired trajectory counterfactual**. One latent uncertainty draw is shared across the no-action baseline and candidate action arms; each arm receives its own exact action set, dynamics propagation, complete mission event signature and direct consequence evaluation.

This extends, rather than replaces, the current research increments:

- [#7501 — source-bound coalition consequences](https://github.com/Luminous-Dynamics/symtropy/pull/7501)
- [#7503 — mission-level joint encounter risk](https://github.com/Luminous-Dynamics/symtropy/pull/7503)
- [#7443 — pathwise proposal bridge](https://github.com/Luminous-Dynamics/symtropy/pull/7443)
- [#7412 — joint-action outcome catalogue](https://github.com/Luminous-Dynamics/symtropy/pull/7412)
- [#7161 — joint physical-world / loss-model bridge](https://github.com/Luminous-Dynamics/symtropy/pull/7161)
- [#7220 — independent loss-model-universe adequacy](https://github.com/Luminous-Dynamics/symtropy/pull/7220)

Mission-sequence risk aggregation itself is not a novelty claim. NASA CARA documents 2D/3D expected-collision-rate methods for single and multiple encounters, plus high-fidelity Monte Carlo from the orbit-determination epoch for repeated conjunctions. Symtropy's proposed differentiation is the auditable pairing of **complete mission paths across counterfactual action portfolios**, with source-bound consequences and decision-level inference.

## 2. Formal model

For a frozen epistemic scenario \(s\), define a target distribution \(P_s\) over exogenous uncertainty \(U\), including applicable initial-state perturbations, environmental/model inputs and other stochastic drivers.

For each portfolio \(a\), the arm-specific state and trajectory are:

\[
X_i(a,s)=\Phi(a,U_i;s).
\]

Each arm is propagated independently under its own action set. Sharing \(U_i\) does not mean sharing the post-action trajectory or reusing the no-action event outcome.

For the complete event inventory \(\mathcal E_s=\{E_1,\ldots,E_n\}\), derive the full path signature:

\[
H_i(a,s)=(E_{i1}(a,s),\ldots,E_{in}(a,s)).
\]

The mission any-event indicator is computed pathwise:

\[
Y_i(a,s)=\mathbf 1\left\{\bigcup_{k=1}^{n} E_{ik}(a,s)\right\}.
\]

Portfolio consequence is evaluated directly, without singleton-action additivity:

\[
L_i(a,s)=L(a,U_i;s).
\]

The paired baseline-relative consequence is:

\[
D_i(a,s)=L_i(a_0,s)-L_i(a,s),
\]

where \(a_0\) is the explicit no-action arm. A positive value denotes avoided consequence; a negative value denotes an adverse result. No negative value is clamped to zero.

For a fixed, correctly sampled finite target measure, the empirical paired contrast is:

\[
\widehat{\Delta}(a,s)=\frac{1}{N}\sum_{i=1}^{N}D_i(a,s).
\]

The statistical unit is the latent mission draw \(U_i\), represented by one joint row across all arms and events. Portfolio arms, encounter windows and conjunction pairs are not additional independent samples.

## 3. Why pairing is a hypothesis, not a guarantee

Common random numbers can reduce the variance of an action difference when baseline and action outcomes are positively correlated under the chosen coupling:

\[
\operatorname{Var}(L(a_0,U)-L(a,U))
=
\operatorname{Var}(L(a_0,U))
+
\operatorname{Var}(L(a,U))
-
2\operatorname{Cov}(L(a_0,U),L(a,U)).
\]

The covariance term can help or hurt. Pairing is therefore an empirical, preregistered variance-reduction hypothesis, not an automatic performance improvement.

Common random numbers (CRN) are established simulation methodology, not a new method claimed by Symtropy. Glasserman and Yao (1992) analyse conditions under which CRN helps and when it may not. Yang and Nelson (1991) and Nelson and Matejcik (1995) study CRN with simultaneous multiple-comparison procedures and best-system selection. Symtropy's contribution must be the domain-specific evidence contract and its orbital, mission-wide validation—not the invention of CRN.

A seed value alone is not an adequate coupling description. The implementation should bind semantic exogenous drivers (for example, primary-state perturbation, secondary-state perturbation, shared density-model error and force-model draw) to stable identities, so that action-dependent execution paths cannot silently shift which random variate represents which physical uncertainty. The exact coupling map must be replayable and preserve each arm's target marginal law.

An exact four-atom control should make the non-guarantee visible. Let (U) be uniform on four atoms and baseline loss be (L_0=[0,1,2,3]). Its population variance is 1.25.

| Action-arm loss on the same four atoms | Covariance with baseline | Paired-difference variance | Independent-arm difference variance |
|---|---:|---:|---:|
| (L_A=[1,2,3,4]) | (+1.25) | (0) | (2.5) |
| (L_B=[0,1,1,0]) | (0) | (1.5) | (1.5) |
| (L_C=[3,2,1,0]) | (-1.25) | (5.0) | (2.5) |

This exact arithmetic control must be included in the first finite fixture. It demonstrates when pairing can help, do nothing or hurt. It proves only the statistical mechanism on a toy finite measure; it does not predict orbital covariance or establish a physical variance reduction.

The coupling must preserve each arm's target marginal distribution. The same exogenous uncertainty identity may be applied to every arm, but each arm must recompute its state transition, propagated trajectory, encounter inventory and consequence. No arm may inherit another arm's collision classification or loss.

A paired row is one multivariate observation. It must not be split into fictitious independent samples. Correlation across arms and events must be preserved in the variance estimator and in simultaneous portfolio comparisons.

NASA CARA's 2026 handbook also highlights a related pitfall: reusing perturbations across pairwise combinations can undermine the conditions behind conventional Monte Carlo confidence intervals. Symtropy must distinguish a valid paired multivariate sample from reuse that inflates nominal sample size or invalidates uncertainty statements.

## 4. Three ledgers that must remain distinct

| Ledger | Contents | What it must not become |
|---|---|---|
| Aleatory | Draw IDs, target/proposal densities, weights, trajectory samples | A list of independent encounter probabilities |
| Epistemic | Physical worlds, covariance/force-model alternatives, loss-model families, event-universe alternatives | An unqualified nominal probability distribution |
| Interventional | Exact action portfolios, constraints, post-action propagations and effects | An additive sum of single-action deltas |

A probability mixture over epistemic scenarios is permitted only with a separately qualified weighting source. Otherwise, evaluate each admitted scenario separately and preserve robust / worst-case comparisons.

## 5. Proposed evidence contract

### 5.1 Campaign identity

Freeze and bind:

- campaign and baseline identity;
- exact action / portfolio universe and resource constraints;
- physical-world and consequence-model scenario set;
- target-measure identity and source receipt;
- propagator, force-model, ephemeris epoch, covariance and hard-body-radius inputs;
- event-definition and full mission event-inventory identities;
- consequence-model version, calibration scope and loss units;
- coupling method/version and seed-schedule identity;
- proposal distribution and likelihood-ratio contract where importance sampling is used.

A digest demonstrates integrity relative to its preimage. It does not authenticate the issuer, prove model calibration or establish real-world inventory completeness.

### 5.2 Paired atom

For each latent draw, record:

- nonzero canonical latent atom ID and scenario ID;
- target/proposal density identity and exact weight provenance;
- one arm record for the no-action baseline and every required portfolio;
- exact action-set digest and arm-specific propagation/trajectory identity;
- complete event/window outcomes for that arm;
- pathwise horizon reducer result and full event signature;
- direct consequence result and model receipt;
- typed outcome state: definite, ambiguous, unavailable, invalid or unresolved.

A missing/ambiguous arm may invalidate the affected paired contrast. It must never be silently interpreted as no-contact, zero loss or safety. Partial results remain visible, but a global portfolio ranking must abstain if required arms or scenarios are not covered.

### 5.3 Adaptive importance sampling

For target density \(p_s(U)\) and actual proposal \(q_t(U)\), each draw must carry the exact source-bound likelihood ratio \(p_s(U_i)/q_t(U_i)\). Proposal support must include the target event support. Weight clipping, post-hoc removal of inconvenient samples and unrecorded proposal changes are prohibited; any approximation needs an explicit bias bound.

When proposals adapt, adaptation must be predictable from prior information. An anytime-valid confidence sequence is valid only under its actual sampling and weighting assumptions. Pairing across action arms does not independently establish sequential validity.

For multiple candidate portfolios, the system must account for simultaneous comparisons and post-selection bias. Per-arm marginal intervals do not automatically give a valid interval for the selected winner.

The proposal must cover the union of rare-event regions across the no-action arm, every candidate portfolio, every encounter and every admitted scenario. A proposal adapted to one manoeuvre can miss a collision mode introduced by another. One defensive construction to benchmark is:

[
q_t(U)=epsilon p_s(U)+(1-epsilon)sum_{j=1}^{J}alpha_{j,t}q_{j,t}(U),quad epsilon>0,
]

where components target distinct portfolio/event/scenario modes. Use the exact density of the full mixture in the likelihood ratio—not merely the selected component density—and bind the mixture weights and adaptation history in the receipt. Arm-specific proposals require a separately justified multiple-importance-sampling estimator and a valid cross-arm coupling; their draws must not be pooled as if they shared one proposal density.

## 6. Integration plan

1. Reuse #7443's shared-trajectory pathwise event reduction and #7503's exact mission union. Do not create another event reducer.
2. Reuse #7412's exact joint-action catalogue and #7501's direct coalition consequence map. Do not infer a coalition outcome from singleton action outcomes.
3. Introduce versioned campaign, latent-atom, portfolio-arm, coupling-receipt and paired-outcome types in the isolated Rust research kernel.
4. Define an independent challenger that reconstructs pairing identity, target/proposal weights, pathwise unions, signed paired deltas and variance statistics.
5. Add a selection layer only after the paired estimator and simultaneous uncertainty contract are qualified.
6. Extend from static portfolios to non-anticipative mission policies (pi_t(mathcal H_t)) that depend only on observations available by decision time (t). Add twin-world tests: two worlds with identical observable history through (t) must induce the same action at (t), even when their hidden future trajectories differ. This prevents hindsight leakage in mission replanning.

## 7. Adversarial known-answer corpus

At minimum:

- identical baseline/action outcomes (zero paired delta);
- positive, zero and negative cross-arm correlation;
- a case where CRN improves variance and a case where it worsens variance;
- a manoeuvre pair with synergy, cancellation and adverse interaction;
- a new post-action conjunction absent from the no-action catalogue;
- repeated and overlapping encounter windows on one trajectory;
- missing, duplicated, ambiguous and mismatched latent atoms;
- missing portfolio arm and mismatched action digest;
- target/proposal support mismatch and invalid likelihood ratio;
- reused seed with duplicate sample identity;
- a planted common-mode loss-model omission;
- input-order permutations that must produce identical canonical outputs;
- a portfolio that is best under one epistemic scenario but harmful in another;
- a portfolio ranking that changes under post-selection if only marginal intervals are used.

The exact finite corpus qualifies arithmetic and structure only. It is not an orbital probability measurement.

## 8. Baseline matrix

These methods solve different subproblems and must not be collapsed into one speed leaderboard.

| Baseline | Primary role | Comparison discipline |
|---|---|---|
| NASA CARA 2D Foster / Elrod Pc | Single-conjunction analytic Pc | Use only within method assumptions |
| NASA CARA 3D Pc | Single-conjunction analytic Pc | Compare against same geometry/covariance and high-fidelity reference |
| NASA 2D-Nc / 3D-Nc | Expected collision rates and multi-encounter bounds | Compare mission-sequence risk, not only isolated Pc |
| NASA high-fidelity brute-force MC from OD epoch | Reference for long/repeated encounters | Match dynamics, covariance and full horizon |
| Monte Carlo from TCA | Local single-event reference | Use only where short-event assumptions apply |
| Cross-entropy Gaussian / GMM | Rare-event proposal adaptation | Include multimodal and repeating-encounter cases |
| Orbital importance sampling with surrogate dynamics | Reduce expensive propagation count | Independently validate surrogate error and weight correctness |
| Subset simulation / line sampling | Alternative rare-event estimators | Apply only where event geometry and assumptions fit |
| Safe-margin screening | Fast conjunction screening | Measure throughput and false negatives separately; it is not a Pc estimator |

Known research baselines include Ketzner & Elgohary (2026), “Monte Carlo Orbital Conjunction Analysis Using the Cross-Entropy Method,” DOI [10.2514/1.G009204](https://doi.org/10.2514/1.G009204), and Sztamfater-Garcia et al. (2025), “An approximate model for the computation of in-orbit collision probabilities using importance sampling,” DOI [10.1016/j.asr.2024.12.074](https://doi.org/10.1016/j.asr.2024.12.074).

All comparisons must freeze or report ephemeris epoch, force model, covariance and cross-correlation treatment, hard-body radius, event geometry, full mission horizon, event inventory, proposal support, hardware, expensive trajectory-call budget and stopping rule.

## 9. Multi-fidelity extension to investigate

Do not make paired high-fidelity Monte Carlo the only route to fewer expensive propagations. Evaluate a multi-fidelity control-variate extension where (F(a,U)) is the expensive trajectory/consequence calculation and (G(a,U)) is an inexpensive, estimand-aligned model:

[
mathbb E[F]=mathbb E[G]+mathbb E[F-G].
]

The low-fidelity term may be evaluated broadly, with the correction estimated on a smaller high-fidelity sample. The estimator must preserve the exact target expectation; any fitting or calibration of (G) must be separated or cross-fitted, and discrepancy/bias must be reported independently from sampling variance. A CARA Pc/Nc calculation can be a risk comparator or control variate only for a matched risk estimand; it is not automatically a proxy for arbitrary consequence loss. A surrogate must not turn model error into an apparently narrow confidence interval.

Measure correction variance, high-fidelity call reduction, held-out residual coverage and bias bounds against independent propagated cases. Keep this separate from the base paired estimator until it has its own evidence contract.

## 10. Metrics and prospective targets

Report at least:

- relative bias and log-scale error of Pc / mission union against an independent high-fidelity reference;
- marginal and simultaneous interval coverage;
- false-safe, false-positive and abstention rates;
- paired-difference variance / unpaired-difference variance;
- paired versus unpaired high-fidelity calls at matched simultaneous coverage and correct portfolio-selection rate;
- low-/high-fidelity correction bias and variance, cross-fit calibration error and held-out residual coverage;
- high-fidelity trajectory calls to reach preregistered error and coverage;
- probability of selecting the wrong portfolio and regret against the exhaustive finite oracle;
- full event-signature and support coverage;
- unknown/ambiguous mass, maximum importance weight and effective sample size;
- p50/p95/p99 runtime and memory;
- seeded omission detection and independent-oracle mismatch counts.

**Prospective targets, not achieved results:**

- 100% exact agreement on finite known-answer controls;
- zero false-safe acceptance on seeded missing-event, missing-arm, stale-model and adverse-interaction controls;
- zero unsupported independence claims and zero duplicate counting of latent samples;
- exact signed paired deltas and finite union arithmetic;
- complete declared action × scenario × latent-atom coverage, or typed abstention;
- at least 2× median paired-variance reduction on preregistered positively correlated cases, with no loss of accuracy/coverage; fallback when pairing is not beneficial;
- at least 10× fewer high-fidelity trajectory calls than brute-force Monte Carlo on selected rare-event cases at matched error and valid coverage;
- correct portfolio ranking on the finite exhaustive oracle;
- zero automatic operational authority.

Targets must be measured on held-out cases. No speedup, estimator superiority or physical qualification is claimed by this design proposal.

## 11. Claim and authority boundary

This is a research/reference decision layer. It is not an orbit propagator, conjunction screening service, calibrated probability source, manoeuvre planner, operator notification service or operational authority. It does not replace NASA CARA, ISO 23705:2026, mission-specific risk governance or human/operator review.

A matching digest, complete finite manifest or source-supplied BELOW_THRESHOLD status does not prove issuer authenticity, physical correctness, real-world universe completeness or safety. Unknown means unknown. No automatic manoeuvre authority is introduced.

## 12. Selected prior art

- Paul Glasserman and David D. Yao (1992), [Some Guidelines and Guarantees for Common Random Numbers](https://doi.org/10.1287/mnsc.38.6.884), *Management Science* 38(6).
- Wei-Ning Yang and Barry L. Nelson (1991), [Using Common Random Numbers and Control Variates in Multiple-Comparison Procedures](https://doi.org/10.1287/opre.39.4.583), *Operations Research* 39(4).
- Barry L. Nelson and Frank J. Matejcik (1995), [Using Common Random Numbers for Indifference-Zone Selection and Multiple Comparisons in Simulation](https://doi.org/10.1287/mnsc.41.12.1935), *Management Science* 41(12).
- NASA, [Spacecraft Conjunction Assessment and Collision Avoidance Best Practices Handbook, Volume 2 (updated August 2026)](https://ntrs.nasa.gov/citations/20260000457).
- Ryan Ketzner and Tarek A. Elgohary (2026), [Monte Carlo Orbital Conjunction Analysis Using the Cross-Entropy Method](https://doi.org/10.2514/1.G009204), *Journal of Guidance, Control, and Dynamics*.
- Yannick Sztamfater-Garcia et al. (2025), [An approximate model for the computation of in-orbit collision probabilities using importance sampling](https://doi.org/10.1016/j.asr.2024.12.074), *Advances in Space Research*.

These references establish prior methods to compare against. Their published gains must be reproduced under matched conditions before any Symtropy comparison is claimed.
