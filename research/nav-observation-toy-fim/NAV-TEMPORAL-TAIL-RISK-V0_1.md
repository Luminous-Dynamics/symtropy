# NAV-TEMPORAL-TAIL-RISK v0.1 — Decision-risk-sensitive active sensing

**Status:** research specification and benchmark contract. The executable static mission-path CVaR evaluator/optimizer is separately implemented in open draft PR #7343; the belief-state CVaR primitive and static-replanning counterexample are in #7344; the two-level ICVaR evaluator is in #7345. Those are separate, unmerged increments. This document itself contains no benchmark score, admitted real payload, or physical campaign.

## Parentage

Composes with, but does not replace:

- NAV-TEMPORAL-ROBUST-VOI-001 #7298 and benchmark PR #7300.
- Time-aware evidence policy PR #7284.
- Temporal mission-global minimax oracle PR #7301 and runnable fixture PR #7302.
- Transition-set qualification issue #7304.
- Risk-aware active acquisition issue #6853.
- Time-validity / epoch applicability issue #7139.
- Existing observation integrity, common-cause, sequential VOI and OOSM work.

This is a separate objective track. It must not silently change the meaning or results of the existing expected-risk and minimax-regret scoreboards.

## 1. Research question

Can an active-navigation policy reduce the probability and severity of rare, high-consequence false-safe decisions at the actual evidence-use epoch, without hiding the resulting availability, energy, latency or false-degraded costs?

The current mission-global minimax objective compares expected terminal loss under each persistent candidate mission model. Expected loss is necessary, but it can conceal a policy that performs well on most trajectories and fails severely on a small tail of trajectories.

The new target is **mission-persistent robust tail-risk active sensing**.

The policy still sees only observable history. The evaluator may know the complete model and future trajectory for scoring, but those privileged values must not enter the controller.

## 2. Paradigm shift

### From information gain to consequence-weighted evidence

An action is valuable when it changes the distribution of the downstream decision loss, not merely when it reduces entropy or parameter variance.

A sensor can reveal a great deal about an irrelevant parameter while leaving the unsafe tail of the navigation decision unchanged. Conversely, a modest observation can eliminate a rare but consequential false-safe branch.

The active policy should therefore optimize a declared decision-loss distribution at the use epoch. Expected information gain, mutual information, posterior covariance, and sensor accuracy remain useful diagnostic baselines, not substitutes for decision risk.

This follows the direction of risk-aware goal-oriented Bayesian experimental design, which separates parameter information from uncertainty in downstream quantities of interest and allows rare or high-consequence outcomes to receive more weight. Its published examples concern inverse problems and sensor placement, not UWB navigation; transfer to this application is a research hypothesis, not an established result.

### Separate epistemic model uncertainty from trajectory-tail risk

Use three levels explicitly:

1. **Persistent mission model:** a complete observation/dynamics/environment model identity that remains fixed for the entire mission.
2. **Persistent common-cause regime:** a shared clock, obstruction, anchor, interference, calibration or platform fault regime that is sampled or selected once for a mission and is never independently resampled at each observation.
3. **Mission outcome path:** the stochastic sequence of observations, dropouts and navigation-state transitions conditional on that persistent mission model and regime.

Do not collapse these levels into one confidence score. Do not average over a common-cause regime unless its prior is independently justified. Where its probability is not qualified, include it in the outer plausible-model set and report the resulting robust bound.

## 3. Mathematical objective

Let:

- \(\Pi_{\mathrm{obs}}\) be the feasible non-anticipative policy trees whose decisions depend only on observable history;
- \(m \in \mathcal{M}\) be a complete candidate mission model held fixed throughout the mission;
- \(\omega\) be a complete stochastic mission outcome path under \(m\) and policy \(\pi\);
- \(L(\pi,m,\omega)\) be the declared mission loss, evaluated at the actual evidence-use epochs.

For a loss random variable \(Z\), define upper-tail conditional value at risk:

\[
\operatorname{CVaR}_{\alpha}(Z)
=
\inf_{\eta \in \mathbb{R}}
\left[
\eta + \frac{1}{1-\alpha}\,
\mathbb{E}\left[(Z-\eta)_+\right]
\right].
\]

The proposed primary tail objective is:

\[
\min_{\pi \in \Pi_{\mathrm{obs}}}
\max_{m \in \mathcal{M}}
\operatorname{CVaR}_{\alpha}
\left(L(\pi,m,\omega)\mid m,\pi\right).
\]

The outer maximum is over complete, mission-persistent models. CVaR is computed over stochastic mission paths within each fixed model. This is not CVaR over a probability distribution on epistemic models.

Keep the existing expected-risk objective alongside it:

\[
\min_{\pi \in \Pi_{\mathrm{obs}}}
\max_{m \in \mathcal{M}}
\mathbb{E}[L(\pi,m,\omega)].
\]

Also retain mission-global minimax regret as a distinct objective. A policy that minimizes expected worst-case loss need not minimize worst-case CVaR or regret.

### Risk-constrained alternative

For missions with a predeclared risk budget \(B_\alpha\), evaluate:

\[
\text{find }\pi\in\Pi_{\mathrm{obs}}
\quad\text{such that}\quad
\max_{m\in\mathcal{M}}\operatorname{CVaR}_{\alpha}(L\mid m,\pi)
\le B_\alpha.
\]

Latency, energy and bandwidth remain hard feasibility constraints or separately reported Pareto dimensions. Do not invent exchange rates that convert false-safe risk, joules, bytes and milliseconds into one opaque weighted score.

If no policy satisfies the risk and resource constraints, the result is abstention / no qualified action—not an automatically relaxed threshold.

## 4. Why the objective changes policy selection

The following is a deliberately synthetic one-stage counterexample.

| Policy | Mission loss distribution | Expected loss | CVaR at 95% |
|---|---|---:|---:|
| A | Loss 1 with probability 1%; loss 0 otherwise | 0.01 | 0.20 |
| B | Loss 0.02 on every mission | 0.02 | 0.02 |

Expected-loss optimization selects A. A 95%-CVaR objective selects B because A concentrates a materially worse outcome in its upper tail.

At 99% CVaR, A's tail loss is 1 while B remains 0.02. These values demonstrate an objective-ranking difference only. They are not estimated navigation failure rates, sensor measurements, or a field result.

A useful implementation must produce such counterexamples as exact fixtures, then extend them to contingent two-stage policies, persistent common-cause regimes, model disagreement and evidence-age transitions.

## 5. Loss definition and anti-gaming rules

The benchmark must declare the loss before running the policy comparison. At minimum, keep the following channels separate:

- false-safe decision loss and duration/exposure;
- false-degraded decision loss and duration/exposure;
- mission interruption / loss of service;
- missed deadline or stale-evidence use;
- resource consumption and recovery cost.

Do not allow a policy to improve CVaR by making every decision DEGRADED unless false-degraded exposure, valid coverage, availability and mission completion are reported alongside it.

The loss is attached to the actual decision/use epoch. Capture-time position error is a different metric and cannot replace use-epoch loss.

A learned future-epoch position remains a prediction with a declared target epoch; it is not relabelled as a physical observation at that epoch.

## 6. Required comparators

All policies must receive the same candidate actions, observations, model set, resource limits, timestamp semantics and information cutoff.

1. Mission-global minimax expected loss (existing objective).
2. Mission-global minimax regret (existing separate objective).
3. Mission-global robust CVaR at alpha = 0.95.
4. Mission-global robust CVaR at alpha = 0.99; alpha = 0.999 only with adequate tail support.
5. Bayesian-mixture CVaR, clearly labelled as prior-dependent rather than worst-case robust.
6. Rectangular / branchwise model-switching tail adversary, diagnostic only.
7. One-step greedy decision value and mutual-information / expected-information-gain policies.
8. Periodic, fixed-duty-cycle and random acquisition.
9. Time-validity rejection without decision revaluation.
10. Constant-velocity / kinematic delay compensation.
11. CNN–Transformer CIR plus motion-history delay compensation, reproduced under its published target epoch, dataset and split.
12. Clairvoyant future-state evaluator, privileged and never deployable.
13. NLOS reject-only ranging, where classified observations are excluded rather than corrected.
14. Learned NLOS range correction / raw-CIR Transformer, with exact target-domain supervision and environment holdout recorded.
15. A three-way accept-as-measured / correct-with-qualified-model / reject-or-abstain disposition policy.
16. Per-step split-conformal calibration (negative-control baseline), whole-episode maximum-score calibration, alpha-spending risk control, and trajectory-adaptive calibration.
17. Static mission-path CVaR, two-level ICVaR, and immediate belief-cost CVaR as distinct risk formulations, with their different semantics reported rather than pooled.

Report both each method's result under every objective and the policy selected by each objective. Do not score a CVaR policy only with CVaR or a minimax policy only with expected loss.

## 7. Numerical contract for the exact oracle

The first implementation should remain a small finite reference.

- Enumerate the complete outcome distribution for tractable horizon-two problems.
- Preserve exact outcome probabilities and loss values; do not estimate a tiny synthetic tail with Monte Carlo when exhaustive enumeration is possible.
- Define the discrete CVaR boundary rule, including fractional probability mass at the quantile.
- Use checked integer/rational arithmetic only after documenting the maximum probability and loss denominator. If the bound does not fit the selected integer type, reduce the fixture scale or use a verified wider representation.
- Deterministic sorting and tie-breaking must make results invariant to candidate-action order and outcome insertion order.
- The policy must be one observable tree shared across every persistent mission model.
- Model and common-cause identities stay fixed through all branches of a mission path.
- The rectangular adversary remains a separate diagnostic and cannot be substituted for the persistent-model objective.

For empirical campaigns, calculate tail estimates over independent mission/episode units, not packet rows. Report the effective number of independent clusters, quantile uncertainty and the confidence procedure. A large number of highly correlated packets does not create a large number of independent tail trials.

## 8. Required synthetic challenges and negative controls

| ID | Challenge | Required invariant |
|---|---|---|
| T01 | Rare catastrophic false-safe versus steady small loss | Expected-loss and CVaR policies can rank candidates differently |
| T02 | Persistent shared clock / anchor / obstruction fault | Common-cause identity is not resampled per action |
| T03 | Opposing persistent observation and dynamics models | One observable policy is evaluated under each fixed model |
| T04 | Evidence ages before use | Loss is evaluated at use epoch, not capture epoch |
| T05 | State-dependent dropout | No-observation is a real branch and still propagates state |
| T06 | Model-set expansion | Worst-case CVaR for a fixed policy cannot decrease when plausible models are added |
| T07 | Alpha monotonicity | CVaR must be nondecreasing as alpha increases for a fixed loss distribution |
| T08 | Constant-loss distribution | CVaR equals the constant loss and equals expected loss |
| T09 | Translation and positive scaling | CVaR obeys the corresponding loss translation/scaling identities |
| T10 | No risk-feasible action | Planner abstains; it does not silently relax the risk bound |
| T11 | Branchwise model switching | Diagnostic is separated from mission-persistent tail risk |
| T12 | Candidate order / label permutation | Exact objective values are invariant under harmless permutations |
| T13 | Resource and action reuse limits | Every observable branch respects budgets and no action is reused |
| T14 | Hidden state / future leakage | Controller cannot read privileged model identity or future state |

## 9. Dataset and industry comparison

### Dynamic UWB delay compensation

Modaberi and Far's 2026 study reports a CNN–Transformer combining channel impulse response and motion history, evaluated against Transformer, LSTM and constant-velocity baselines on 491,040 measurements from four indoor environments. It reports 0.26 m mean position error and 85% success within 0.5 m.

That result is a required tracking-accuracy reproduction target, not a false-safe probability or tail-integrity guarantee. Reproduce the exact target epoch, prediction horizon, latency distribution, split, inputs and test conditions before making a numerical comparison.

### MILUV

MILUV provides 217 minutes across 36 experiments and three quadcopters, including UWB ranges, CIR, passive listening, received signal power, clock-skew observations, vision, IMU and Vicon pose. The paper says the onboard computer clocks are synchronized using NTP at experiment start, while the separate UWB radio clocks remain unsynchronized and are addressed by the ranging protocol. CIR and range sampling rates differ substantially; CIR collection can reduce ranging to about 5 Hz while range-only experiments can reach about 200 Hz.

This is a strong source for dynamic replay and passive-evidence experiments, but only after the existing time/frame/provenance admission gates pass. Preserve passive-listening timestamps separately. Do not row-align CIR and range streams or infer radio-clock truth from host synchronization.

### HYMN

Use HYMN for spatial generalization and cross-technology transfer, not dynamic delay or transition calibration: its published v1 records stationary measurements at 48 reference points, with no sensor logging during transit. Hold out reference points and sites; random sample splits conceal the generalization penalty reported in the 2026 benchmark.

### NLOS evidence disposition: accept, correct, or reject

A raw UWB range classified as NLOS should not be forced into a binary “trust / correct” decision. The system should choose among three observable dispositions:

1. **Accept as measured** when the raw observation satisfies the declared measurement-integrity and applicability contract.
2. **Correct** only when a qualified correction model is valid for the current environment, device, CIR regime and target use epoch, with a bounded residual / uncertainty commitment.
3. **Reject or abstain** when neither raw acceptance nor the correction model is supportable.

This is a direct research priority from Kovaci et al., *Ultra-wideband indoor positioning: a critical survey of ranging, AI-based error mitigation, sensor fusion, and 6G integration* (published 23 November 2026; DOI 10.1186/s13638-026-02703-9). The survey reports method-specific results that must not be mixed into one ranking: REMnet reduces NLOS range MAE from 12.42 to 6.87 cm in a held-out medium room; neural reliability weighting reduces mean position error from 3.41 to 2.45 m on a five-anchor corridor after office training; calibrated anchor optimization reduces 3D RMSE from 29.1 to 7.1 cm in a cluttered-space experiment. It also highlights a cross-scenario study where rejecting NLOS-classified observations improved position while learned range correction increased error. These are different tasks and conditions, not directly comparable SOTA numbers.

Required matched experiment:
- raw range;
- NLOS rejection only;
- learned correction;
- reliability-weighted fusion;
- correction plus uncertainty envelope;
- decision-focused accept/correct/reject policy.

Report ranging and position P50/P95/P99, false acceptance of harmful NLOS, false rejection of valid LOS, accepted-observation coverage, protection-region size, trajectory availability, use-epoch calibration, latency, energy and adaptation-label privilege. Hold out entire rooms/sites/hardware/anchor layouts. Record target-domain supervision; do not call a within-site cross-trajectory result cross-building generalization.

### Whole-trajectory calibration is not per-step calibration

A multi-step mission-level event is often “at least one decision epoch violates its valid position / protection region.” Marginal coverage at each timestamp does not automatically control this union event.

Ning et al., *Per-Step Conformal Calibration Does Not Transfer to Agent Trajectories* (EMNLP 2026; DOI 10.18653/2026.emnlp-main.1808), report that nominal 10% per-step split-conformal miscoverage yields 83.5% trajectory score-threshold miscoverage on a 15-step web-navigation agent. The paper studies iid resampling, memoryless covariate shift and state-dependent Markov transitions, and evaluates alpha-spending schedules. This is not a navigation experiment, but it is a strong warning against assuming per-frame calibration composes into mission-level calibration.

Add these distinct comparators:
- per-step split conformal, explicitly labelled as a negative control for trajectory-union coverage;
- maximum-score whole-episode conformal calibration;
- alpha-spending / time-uniform risk control over decision epochs;
- trajectory-adaptive calibration such as TRACE-CRC;
- dependence-aware mission-block calibration where complete episodes, flights, devices or sites are the independent units.

Measure both marginal per-step coverage and the mission union event: any false-safe or any true-state-outside-protection-region event during an episode. Also report region size / volume, valid availability, abstention, time degraded, effective independent cluster count and uncertainty bounds. Do not infer mission PHMI from per-frame conformal coverage.

### Risk semantics remain separate

The benchmark must distinguish:
- immediate belief-state CVaR over hidden-state costs at the current decision epoch;
- static CVaR over complete mission-path loss;
- iterated / nested conditional CVaR for sequential replanning;
- outer worst-case aggregation over complete persistent mission models.

Do not sum or interchange these measures without a justified composition result. The R03 counterexample in #7344 demonstrates why static trajectory CVaR is not automatically preserved by fresh conditional replanning. The two-level evaluator in #7345 is not yet a full robust ICVaR Bellman policy optimizer.

### Standards and product certification

- ISO/IEC 21134:2026 supplies a benchmark framework for integrated indoor localization/tracking using dead reckoning. Align applicable reporting; it is not an end-to-end safety certificate.
- FiRa Core 4.1 expands UWB untracked-navigation and subsystem security/interoperability capabilities. Its core certification coverage is the UWB subsystem PHY, MAC and Link layers; it does not certify this policy, its dynamics models, or its use-epoch integrity.

## 10. Scoreboards

Keep independent, simultaneously reported scoreboards:

| Scoreboard | Required measurements |
|---|---|
| Localization / tracking | P50/P95/P99, mean error, ADE/FDE where applicable, trajectory drift, orientation, availability |
| Decision integrity | False-safe probability and exposure, false-degraded probability and exposure, CVaR95/CVaR99, use-epoch calibration, risk–coverage, abstention |
| Robust policy | Per-model expected loss, per-model CVaR, worst-case expected loss, worst-case CVaR, minimax regret, exact-oracle gap |
| Tail evidence | Number of independent mission clusters, tail sample count, quantile confidence bounds, coverage shortfall, scenario support |
| Operations | p50/p95/p99 latency, energy, airtime, bytes, compute, memory, degraded duration, recovery time |

A lower mean error is not a win if CVaR, false-safe exposure or valid coverage worsens. A narrower transition set is not a win if held-out coverage falls.

For rare-event claims, state the trial unit and confidence bound. For example, the approximate rule-of-three requires roughly 300,000 independent zero-failure trials to put a one-sided 95% upper bound near 10^-5. Correlated packets or repeated windows from one mission do not meet that independence assumption.

## 11. Proposed targets (not achieved)

1. Exact small-case oracle: 100% agreement with an independently implemented exhaustive enumerator.
2. Negative controls: 100% rejection / correct disposition for seeded chronology, leakage, identity, persistence and resource violations.
3. Risk sensitivity: demonstrate at least one exact fixture where minimax expected loss and robust CVaR choose different policies for the correct mathematical reason.
4. Tail quality: on a predeclared synthetic and then held-out trajectory suite, target at least 20% lower CVaR95 than the expected-risk-selected policy at matched resource limits and matched valid coverage, while reporting any increase in expected loss and false-degraded exposure.
5. SOTA reproduction: reproduce the 0.26 m / 85% delay-compensation result under its exact protocol before claiming any accuracy improvement.
6. Generalization: separate episode, vehicle, site, anchor-layout, motion-regime and NLOS holdouts.
7. Physical claim ceiling: no integrity, PHMI, certification or operational-safety claim from synthetic CVaR, fitted kernels, or nominal coverage alone.

The 20% target is a research ambition, not a forecast or achieved result. It must be revised if a matched baseline makes it inappropriate.

## 12. Implementation sequence

1. Keep the current time-aware and mission-global expected-risk solvers unchanged.
2. Add a separate exact discrete tail-risk evaluator for a fully enumerated path-loss distribution.
3. Cross-check it against a second independent implementation and hand-derived distributions, including fractional quantile mass.
4. Add robust CVaR policy selection over the same observable policy trees and fixed mission model set.
5. Add the synthetic challenges for tail-vs-expectation, persistent common cause, model-set expansion, NLOS accept/correct/reject, and whole-trajectory coverage; verify all metamorphic properties.
6. Keep common-cause prior-weighted and common-cause outer-worst-case semantics as separate declared protocols.
7. Connect only evidence-qualified transition model sets from #7304.
8. Reproduce tracking baselines on their own admitted datasets before running active-policy comparisons.
9. Run episode-level replay and then independently reviewed physical trials.

## References

- Jakeman, White, van Bloemen Waanders, Kouri and Alexanderian (2026), *Risk-Aware Goal-Oriented Bayesian Optimal Experimental Design*, arXiv:2609.08023. https://arxiv.org/abs/2609.08023
- Modaberi and Far (2026), *Real-time delay-compensated UWB localization for dynamic agents via deep trajectory prediction*, Expert Systems with Applications. https://doi.org/10.1016/j.eswa.2026.132248
- Shalaby et al. (2026), *MILUV: A Multi-UAV Indoor Localization dataset with UWB and Vision*, International Journal of Robotics Research. https://doi.org/10.1177/02783649251405898
- *From Least Squares to Deep Learning: Benchmarking Indoor Positioning on the HYMN Multi-Technology Dataset* (IPIN 2026). https://ieeexplore.ieee.org/abstract/document/11772473
- ISO/IEC 21134:2026. https://www.iso.org/standard/86651.html
- FiRa Core 4.1 announcement (20 October 2026). https://www.firaconsortium.org/news/press-releases/2026/10/fira-consortium-launches-core-4-1-specifications-and-enhanced
- Kovaci et al. (2026), *Ultra-wideband indoor positioning: a critical survey of ranging, AI-based error mitigation, sensor fusion, and 6G integration*. https://doi.org/10.1186/s13638-026-02703-9
- Ning et al. (2026), *Per-Step Conformal Calibration Does Not Transfer to Agent Trajectories*, EMNLP 2026. https://doi.org/10.18653/2026.emnlp-main.1808
- Rezaei et al. (2026), *TRACE-CRC: Trajectory-Adaptive Conformal Risk Control for Multi-Step Channel State Information Prediction*. https://proceedings.mlr.press/v329/rezaei26a.html

## Claim boundary

This specification adds a research objective and benchmark protocol. Related executable finite mathematical software is tracked separately in open draft PRs #7343–#7345 and is not merged into this specification branch. No measured benchmark score, admitted real payload, calibrated physical risk estimate, transition-model coverage guarantee, PHMI estimate, safety approval, FiRa certification or operational authority is claimed.
