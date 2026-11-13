# LOCO-VIABLE-VOI-001 — Value contact information over robust post-probe continuations

**Status:** protocol and exact synthetic fixtures. No physical robot result or safety claim.

Issue: [#4310](https://github.com/Luminous-Dynamics/symtropy/issues/4310).

## 1. Objective

Compose the finite decision-value oracle (#4272) with the probe-induced state-transition and robust continuation oracle (#4293).

Question: after a physically admitted probe changes the robot's state and consumes time/resources, does its observation improve the best task decision **among the continuation actions that remain robustly admitted from every physically possible post-probe state**?

This layer owns only the composition semantics, joint branch/hypothesis/outcome bookkeeping, selector comparison and exact finite reference fixtures. It does not create another estimator, transition abstraction, viability kernel, risk ledger, energy ledger or general POMDP.

## 2. Why the composition is necessary

#4272 evaluates decision value over a supplied admitted action set. #4293 computes an observation-specific admitted continuation set over the physical successor-state set after a probe.

A probe can change:
- support configuration and body momentum;
- the successor states physically possible when a delayed observation arrives;
- the remaining horizon and available recovery actions;
- actuator, thermal and energy headroom;
- the available risk allocation.

A frozen pre-probe action set is therefore not the post-probe decision problem. An observation may be informative but have no admissible useful continuation.

## 3. Existing ownership boundaries

| Concern | Existing owner | Consumed evidence |
|---|---|---|
| Contact likelihood and correlated sensor evidence | #4004 / #4230 | Calibrated joint evidence and correlation identity |
| Fixed finite decision-risk/VOI arithmetic | #4268 / #4272 | Decision loss units and exact risk arithmetic |
| Probe physical branches and observation possibilities | #4287 / #4293 | Prefix viability, latency and post-probe state sets |
| Hybrid transition and robust viability | #4061 / #4083 | Complete physical successors and admitted continuation sets |
| Assurance validity | #4119 | State/model/actuator/timing identity and freshness |
| Horizon risk lifecycle | #4231 / #4245 | Current admission and non-recycling budget |
| Energy and actuator costs | #4044 / #4179 / #4206 | Measured/source-qualified resource boundaries |

This workstream must consume, not recalculate, the upstream admission results.

## 4. Gate ordering

1. Validate current state, model, abstraction, operating domain, actuator, sensor/calibration, clocks, horizon and receipt identities.
2. Require every physically possible probe trajectory branch and intermediate latency state to pass the independent robust viability/recovery gate (#4293 consuming #4061/#4083). A branch with zero calibrated probability remains physically possible.
3. For every observation outcome (y), take the complete post-probe physical successor set (S_y) and upstream robustly admitted continuation set (A_R(y)).
4. Require a separately admitted fallback for Unknown, delayed, missing, contradictory and out-of-model observations.
5. Only after those hard gates pass may calibrated joint probabilities rank the admitted continuations or probes.
6. Enforce energy, actuator/thermal, elapsed-time and non-recycling risk budgets as hard limits where declared.

A Bayesian update cannot remove a physically possible branch or promote an action into (A_R(y)).

## 5. Exact finite mathematical contract

Let:
- (H) be finite latent contact/terrain hypotheses;
- (B) be the complete set of physically possible probe branches;
- (Y) be all declared observation outcomes, including explicit Unknown;
- (J(h,b,y)) be calibrated **joint** integer mass for hypothesis, physical branch and observation;
- (A_R(y)) be the independently admitted continuation actions for the complete physical successor set compatible with (y);
- (L(a,h,b,y)) be preregistered non-negative integer task/decision loss;
- (S) be the probability-mass scale (V0 fixtures use (10^9)).

Joint mass must be supplied directly. Do not construct it by multiplying marginal probabilities unless an independently qualified factorization explicitly warrants that assumption. All joint cells normalize exactly:

\[
\sum_{h,b,y}J(h,b,y)=S.
\]

For each outcome (y), evaluate only admitted actions:

\[
R_y=\min_{a\in A_R(y)}
\sum_{h,b}J(h,b,y)L(a,h,b,y).
\]

Expected post-probe continuation risk:

\[
R_{\mathrm{after}}(u)=\sum_y R_y.
\]

The no-probe decision uses its own matched state/observation/continuation evidence and the same loss and task units:

\[
R_{\mathrm{passive}}=
\min_{a\in A_R(\varnothing)}
\sum_h J_0(h)L_0(a,h).
\]

Viability-conditioned decision value:

\[
\boxed{
\operatorname{VOI}_{\mathrm{viable}}(u)
=R_{\mathrm{passive}}-R_{\mathrm{after}}(u)
}
\]

V0 should use integer fixed-point probabilities and losses with checked wide-integer arithmetic. The exact numerator denominator is (S^2) for probability×loss when loss is in integer units; no floating-point normalization or rounding may enter the reference decision. EIG/Fisher comparators are computed and reported separately; they do not enter this result.

The probe is evaluated only when every physical branch and every intermediate latency state passes the upstream robust gate and all observation outcomes have an admitted continuation/fallback. If an outcome has no admitted action, the probe is rejected/Unknown before ranking; do not assign an arbitrarily huge loss to make an unsafe action merely unattractive.

### Joint-mass and physical-possibility separation

- The robust gate consumes the entire physical possibility relation, independently of probability mass.
- Joint mass is used only for expected-loss ranking among admitted choices.
- A zero-mass physically possible branch remains in the robust gate.
- Unknown outcome mass is never dropped or silently redistributed.
- Correlated observations remain joint; do not double-count shared evidence.
- Missing/invalid joint cells, identity mismatch, stale calibration or unqualified model adequacy returns Invalid/Unknown.

### Resource accounting

Report the probe cost vector separately:
- battery/source and actuator electrical energy;
- mechanical work / impact impulse where measured;
- elapsed time and observation latency;
- actuator torque/slew/thermal reserve;
- current horizon risk allocation consumed/pending/unknown.

Only compare policies under the same declared hard budgets. A task-specific net utility may be reported only with a preregistered unit conversion; physical safety is never a tradable penalty.

## 6. Required policy baselines

All methods get the same priors, physical branches, observations, calibration, loss matrices, action/receipt sets, sensor inputs, energy/time/risk budgets and compute budget.

1. Passive/no-probe continuation.
2. Random selection among independently admitted probes, then optimal admitted continuation.
3. EIG-only after identical physical admission and continuation gate.
4. Fisher-information ranking after identical hard gates.
5. Fixed-state VOI #4272 followed by the #4293 continuation gate (critical ablation).
6. Viability-conditioned VOI using outcome-specific (S_y) and (A_R(y)).
7. Short-rollout context inference / meta-RL adaptation (e.g. ARER-MetaRL-style fault-context embedding and policy reconfiguration), with identical probe duration, energy, actuator/risk budget and post-probe horizon; compare its adapted action only after the same upstream viability/continuation admission.
8. Robust viability-only selection without information objective.
9. EIG-only without post-probe viability as a negative control only; never a deployment controller.

Selectors cannot access hidden truth, evaluator-only fields, true branch IDs or future observations.



### Current SOTA comparators added in 2026

- **ARER-MetaRL under locked-joint faults** (Wu et al., PLOS One 21(10), e0361099, published 16 Oct 2026): context encoding from a short rollout, modular policy routing and regularized fast adaptation. In Isaac Sim only, 12-DoF quadruped, 1000 Hz physics/50 Hz control; meta-training covers 0–3 locked joints, severity tests up to six. Paper-reported MELV/MERP are 0.86/0.84 at one locked joint and 0.55/0.54 at six, versus single-task RL 0.21/0.18 and 0.05/0.03. This paper explicitly defers hardware, reduced-torque, intermittent-fault and freely-swinging-joint tests. Use it as a short-context adaptation baseline; its context inference does not certify that the chosen probe or adapted action remains viable. https://doi.org/10.1371/journal.pone.0361099
- **Quadruped UED curriculum benchmark** (Gilewicz et al., arXiv:2610.40364, 29 Oct 2026): 14 curricula across five quadruped morphologies with an eight-dimensional terrain space, matched PPO environment steps and five seeds. Its evaluation adds held-out structured terrain, cross-fitted CVaR25, per-feature 50%-success capability envelopes and level-wise domination. The in-distribution random-terrain leader differs from the held-out leader on Go1, Go2 and Barkour. Use its method matrix and held-out protocol when evaluating where probes/adaptive curricula should collect information, but do not conflate a success envelope with a robust viability/recovery certificate. https://arxiv.org/abs/2610.40364

## 7. Frozen known-answer and falsification corpus

The machine-readable fixtures are in [fixtures/known_answers_v0_1.json](fixtures/known_answers_v0_1.json).

- **A — low-EIG, useful viable decision:** prior risk 0.5; an admissible probe partitions two hypotheses and exposes different post-probe continuation sets; exact post-probe risk 0.1 and viable VOI 0.4.
- **B — high-EIG, no viable continuation:** perfect observation information does not rescue a probe whose physical branch lacks a robustly admitted post-probe continuation. Reject before EIG/VOI ranking.
- **C — observation aliases body states:** two branches share an observation; continuation must use the robust action intersection over both successor states.
- **D — unknown fallback:** non-zero unknown mass is retained and assigned only to the explicit admitted fallback.
- **E — zero-mass physical branch:** probability zero does not remove a physically possible dead-end branch from robust admission.
- **F — resource limit:** a high decision-value probe that exceeds a declared energy/time/actuator/risk hard budget is excluded.
- **G — correlated joint mass:** joint likelihood table differs from the product of marginals; exact answer uses supplied joint mass only.

## 8. Required scorecard

Report separately:
- probe admission and robust branch completeness;
- post-probe physical successor sets and viability/recovery classification;
- outcome-specific admitted continuation action sets and witnesses;
- exact passive and post-probe expected decision risk;
- viability-conditioned VOI and held-out realized regret;
- EIG/Fisher comparators and realized information calibration;
- Unknown/abstention/fallback rate;
- joint-mass normalization and correlation/provenance rejections;
- battery/source, actuator electrical and mechanical energy boundaries as available;
- probe duration, latency, impact, actuator/thermal utilization;
- current/pending/consumed/unknown risk allocations;
- task completion, stable-step fraction, slip, foot scuff, shank collision, falls and recovery time;
- worst/tail latency, memory and missed deadlines.

Do not collapse viability, recovery, calibration, decision value, energy and compute into one weighted score. Keep a complete adverse-outcome taxonomy.

## 9. Proposed targets (not achieved)

- 100% exact agreement with independent finite brute-force composition fixtures.
- Zero EIG/VOI bypasses of physical admission.
- Zero silent unknown probability mass or physical branch removal.
- Zero selection outside the outcome-specific admitted continuation set.
- Zero stale/mismatched receipt acceptance.
- At matched hard physical/resource gates, non-inferior held-out regret to the best simple baseline.
- Stretch: at least 10% lower held-out regret or 10% fewer probes at equal regret, without worsening any hard physical constraint or preregistered adverse-outcome stratum.
- Publish all trade-offs; no aggregate weighted-score superiority claim.

## 10. Claim ceiling

V0 qualifies exact composition arithmetic and evidence/admission semantics for supplied finite inputs. It does not validate physical likelihoods/losses, prove upstream viability soundness, authenticate receipts, establish continuous-time safety or demonstrate real-world locomotion superiority. Simulation, biological and hardware results remain separate evidence tiers.


### Exact joint possibility support

The finite relation must explicitly enumerate the allowed triples ((h,b,y)), separately from their calibrated mass. Its ((h,b)) projection must exactly equal the declared hypothesis/branch support; its ((b,y)) projection must exactly equal the physical branch/observation support. A cell can therefore be impossible even when both pairwise projections are individually allowed. Positive joint mass outside the declared triple relation is Invalid. Zero-mass but physically possible triples remain in the possibility relation and are not removed from the robust physical gate.
