# LOCO-DECISION-VOI-001 — Decision value of safe contact information

**Status:** protocol and exact synthetic known-answer fixtures; no locomotion performance result or robot safety claim.

Parent work:
- [LOCO-CONTACT-INFO-001 / #4004](https://github.com/Luminous-Dynamics/symtropy/issues/4004) owns action-conditioned contact inference and active haptic exploration.
- [LOCO-BELIEF-001 / #4100](https://github.com/Luminous-Dynamics/symtropy/issues/4100) owns dual robust/probabilistic belief-space assurance.
- This workstream is tracked in [#4268](https://github.com/Luminous-Dynamics/symtropy/issues/4268).

## 1. Research question

When is a contact observation worth acquiring because it changes a consequential locomotion decision?

Entropy reduction and Fisher information quantify aspects of uncertainty reduction or parameter identifiability. Neither, by itself, establishes that a new observation changes the best available action. A measurement may reveal a great deal about a terrain variable that is irrelevant to all currently admitted decisions. A much smaller observation may resolve a friction/support ambiguity that changes which admitted foothold or support sequence has lower expected loss.

The benchmark therefore measures **decision value**, not just information volume.

This is a domain adapter and evaluation layer. It is not a new general POMDP solver, a replacement contact estimator, an active-control policy, or a physical safety authority.

## 2. Ownership and composition

| Capability | Existing owner | Contract consumed here |
|---|---|---|
| Contact evidence, action-conditioned likelihoods and EIG | #4004 / #4008 | Prior, calibrated observation likelihoods and probe identity |
| Joint correlated contact fusion | #4017 / #4230 | No independent-channel double counting |
| Belief-space and uncertainty semantics | #4100 | Epistemic/aleatoric/health separation |
| Robust physical viability | #4058 / #4061 and #4083 | Set of already-admitted probe and decision actions |
| Certificate validity | #4117 / #4119 | Current evidence/model/actuator/timing status |
| One-step calibrated chance filter | #4010 / #4231 | Optional operating-mode admission evidence |
| Non-recycling horizon risk ledger | #4232 / #4245 | Required current allocation where chance-risk mode is used |
| Decision-aware measurement selection outside locomotion | #4041 | Methodological precedent; no parallel generic engine |

The decision-value evaluator must not recalculate or replace any of these upstream responsibilities.

## 3. V0.1 decision model

Let:
- (H=\{h_1,\ldots,h_n\}) be finite contact/terrain hypotheses;
- (p_i=P(h_i)) be prior mass;
- (A_R) be the set of decision alternatives already admitted by the independent robust physical-assurance layer;
- (L(a,h_i)) be a preregistered task/regret loss;
- (u) be a candidate probe;
- (q_{io}=P(y_o\mid h_i,u)) be its calibrated likelihood for observation outcome (o).

The probe's likelihood rows must include every declared outcome. Probability mass cannot be dropped and silently renormalised.

Prior Bayes risk:

\[
R_0=\min_{a\in A_R}\sum_i p_i L(a,h_i).
\]

Expected posterior decision risk:

\[
R_1(u)=\sum_o P(y_o\mid u)\min_{a\in A_R}
\mathbb E[L(a,h)\mid y_o,u].
\]

Expected value of sample information:

\[
\operatorname{VOI}(u)=R_0-R_1(u).
\]

VOI is measured in the declared decision-loss units. It is not a probability of safety.

### Exact fixed-point form

Use integer probability and loss units with scale (S=10^9):
- prior masses (p_i\in[0,S]), with \(\sum_i p_i=S\);
- each likelihood row (q_{i\cdot}\in[0,S]^m), with \(\sum_o q_{io}=S\);
- losses \(L_{ai}\in[0,S]\) for the normalized synthetic fixtures.

Define:

\[
N_0=\min_a\sum_i p_iL_{ai}
\]

and

\[
N_1=\sum_o\min_a\sum_i p_iq_{io}L_{ai}.
\]

Then:

\[
\boxed{\operatorname{VOI}(u)=\frac{S N_0-N_1}{S^3}}
\]

This form is exact for the fixed-point inputs, provided checked wide-integer arithmetic is used. Reject malformed or unnormalised inputs; do not clip or renormalise them. The eventual Rust reference should use checked \(u128\) operations and explicit cardinality bounds.

Compute EIG separately for comparator reporting:

\[
\operatorname{EIG}(u)=
\sum_{i,o}\frac{p_iq_{io}}{S^2}
\ln\left(\frac{q_{io}S}{\sum_jp_jq_{jo}}\right),
\]

with zero-mass terms omitted. EIG must not enter the exact VOI arithmetic.

## 4. Physical admission is an upstream hard gate

The candidate set is not “all actions with a high score”. A probe may be evaluated only when a current independent admission receipt confirms, for the same state, domain and model:
- robust viability/recoverability admission;
- assurance/certificate validity;
- action/actuator identity and measured delivery;
- current calibration and observation-model identity;
- a valid finite-horizon chance allocation when the selected operating mode requires one.

Decision alternatives are likewise supplied with independent admission receipts. V0.1 evaluates regret only among this admitted action set.

Important consequence: a posterior probability update cannot promote an action that was not already robustly admitted, and a low-probability hypothesis cannot simply be discarded to manufacture a robust-safe action.

This evaluator checks declared receipt identity, status and freshness. It does not cryptographically authenticate the receipt or prove the upstream physical abstraction sound.

## 5. Unknown observations and abstention

An explicit unknown/out-of-model outcome is required. It represents evidence that cannot safely be interpreted under the declared likelihood family.

On this outcome:
- do not force a known-hypothesis posterior update;
- do not silently redistribute unknown probability mass across known outcomes;
- select only a separately admitted stop/hold/recovery fallback, or return Unknown/abstain;
- record unknown-outcome frequency and resulting task cost.

A finite hypothesis set does not prove that it contains the true physical regime.

## 6. Exact known-answer fixtures

The machine-readable fixtures are in [fixtures/known_answers_v0_1.json](fixtures/known_answers_v0_1.json).

### Fixture A — high information, zero decision value

Two equally likely hypotheses are perfectly distinguished by the probe (EIG = \(\ln 2\) nats). However, the same admitted action has lower loss under both hypotheses. Expected decision risk remains 0.10 before and after observation. Therefore:

- EIG = 0.6931471805599453 nats;
- VOI = exactly 0;
- optimal decision does not change.

This falsifies the assumption that more information necessarily improves the current task decision.

### Fixture B — low information, positive decision value

Prior hypothesis probabilities are 0.51 and 0.49. Two admitted actions have complementary unit losses. The observation likelihoods are 0.60/0.40 under the first hypothesis and 0.40/0.60 under the second.

- prior optimal action: A;
- outcome probabilities: 0.502 and 0.498;
- posterior optimal actions: A and B respectively;
- prior expected loss: 0.49;
- expected post-observation loss: 0.40;
- exact VOI: 0.09;
- probability of switching from the unique prior-optimal action: 0.498;
- EIG: approximately 0.0201275135293554 nats.

This is a lower-information but decision-relevant observation.

### Fixture C — uninformative observation

Both hypotheses induce the same outcome distribution. EIG and VOI are exactly zero, and the optimal action remains unchanged.

### Fixture D — explicit unknown outcome

A declared unknown observation outcome has non-zero mass. It must select a separately admitted fallback with its declared loss; it must not be treated as a known outcome or silently renormalised away.

## 7. Required baselines

All selectors receive the same belief, observation likelihoods, admitted probe set, admitted decision alternatives, hidden outcomes, energy/time budget and compute budget.

1. Passive/no-probe decision.
2. Random selection among admitted probes.
3. EIG-only selection after identical admission filtering.
4. Fisher-information selection after identical admission filtering.
5. Heuristic contact-probe selection.
6. VOI selection using the same declared loss matrix and likelihoods.
7. EIG-only without physical admission as a negative control only; it is never a deployment policy.

Use the same likelihood calibration and same robust-admitted action set for every selector. Report results by terrain, individual, contact mode and sensor-health stratum.

## 8. Qualification scorecard

Report independently:
- exact prior and expected posterior decision risk;
- exact VOI in declared loss units;
- held-out realised regret and worst-case regret;
- probability of changing the unique prior-optimal action;
- optimal-action-set refinement probability;
- EIG in nats and Fisher-information comparator;
- probes per episode and decision-value per Joule;
- energy, elapsed time, actuator margin and physical viability reserve consumed;
- unknown/OOD outcome rate and abstention/fallback frequency;
- false-safe outcomes from the independent physical evaluator;
- falls, slips, task completion and recovery success;
- model/calibration coverage and performance under held-out individuals/terrains;
- worst-case latency, memory and deadline misses.

Energy, time, decision loss, safety constraints and recoverability reserve remain distinct. Do not collapse them into a weighted universal score.

## 9. Proposed qualification targets (not achieved)

- 100% exact agreement with independently enumerated finite known-answer fixtures.
- The irrelevant-perfect-information fixture returns exactly zero VOI.
- The low-information decision-relevant fixture returns exactly 0.09 VOI and 0.498 action-switch probability.
- Zero high-EIG bypasses of physical admission.
- Zero silent unknown-mass loss or likelihood renormalisation.
- No decision-value result from stale, mismatched, uncalibrated or OOD likelihood evidence.
- At matched admissibility and resource budgets, non-inferior held-out regret to the best simple baseline.
- Stretch target: at least 10% lower held-out regret, or at least 10% fewer probes at equal regret, without worsening hard physical constraints.

Targets are research hypotheses. They are not achieved performance.

## 10. Matched external comparison

| Reference | Demonstrated scope | Symtropy comparison |
|---|---|---|
| ISO 18646-5:2026 | Standardized whole-machine locomotion performance tests; not safety verification | Reproducible speed, stopping, stairs, slope, obstacle and clearance reporting |
| Zhao et al. (2026) | Continuous contact-reliability/residual KF; physical indoor walking/turning and onboard latency | Matched estimation baselines; compare calibration and decision regret separately |
| SPI-Active (CoRL 2025) | Fisher-information active dynamics identification and sim-to-real transfer | Same action/observation budget; compare dynamics-identification value with contact-decision value |
| STEP-CBF+ (2026) | Reduced-order foothold/posture screening in randomized simulation | Matched candidate footholds and constraints; distinguish simulation from hardware |
| Impact-aware robust convex MPC (RA-L 2026) | Bounded-impact robust MPC with Unitree Go2 simulation/hardware experiments | Matched impact, terrain and tracking trials; retain whole-body vs reduced-order distinctions |
| RAIBO2 (Nature 2026) | Hardware/software co-design, 42.195 km marathon, reported total cost of transport 0.25 | Long-horizon energy boundary and route/task comparison; do not attribute its hardware advantage to a controller alone |

Sources:
- ISO 18646-5:2026: https://committee.iso.org/cms/live/live/en/sites/isoorg/contents/data/standard/08/68/86850.html
- Jia et al. (2026): https://doi.org/10.1007/s10462-026-11644-0
- Zhao et al. (2026): https://doi.org/10.3390/s26196041
- SPI-Active (CoRL 2025): https://proceedings.mlr.press/v305/sobanbabu25a.html
- Krishna et al. (ICLR 2025): https://mlanthology.org/iclr/2025/krishna2025iclr-value/
- STEP-CBF+ (2026): https://www.nature.com/articles/s41598-026-72507-w
- Impact-aware robust MPC (RA-L 2026): https://doi.org/10.1109/LRA.2026.3692086
- RAIBO2 (Nature 2026): https://doi.org/10.1038/s41586-026-11102-5

## 11. Execution sequence

1. Freeze this protocol and exact fixture file.
2. Implement the finite-hypothesis evaluator as a dependency-free Rust research crate.
3. Independently enumerate tiny fixtures; test integer overflow, malformed priors/likelihoods, permutation invariance and unknown outcomes.
4. Connect current admission receipts from the robust-viability, certificate-lifecycle, chance-gate and risk-ledger references.
5. Evaluate matched EIG/Fisher/random/VOI selectors on FlyGym/SPINDER episodes using the source-pinned observation and locomotion benchmark adapters.
6. Freeze a held-out terrain/individual/sensor-health campaign before unblinding outcomes.
7. Proceed to physical qualification only after independent review of the model, receipts, timing and risk accounting.

## 12. Claim ceiling

This protocol and its synthetic fixtures qualify finite arithmetic and declared decision-model semantics only. They do not validate physical likelihoods, loss functions, terrain hypotheses, receipt authenticity, continuous-time viability, robot safety, hardware energy or real-world locomotion superiority. Simulation, biological and hardware evidence must remain separate.
