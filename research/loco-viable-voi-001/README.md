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

V0 uses integer fixed-point joint probability mass and loss units, both on scale S = 10^9, with checked wide-integer arithmetic. Each unnormalized expected-loss numerator is sum(J_ppb * L_ppb), on scale S²; divide by S to obtain expected loss in ppb units. The difference between passive and post-probe numerators is the exact viable-VOI numerator; divide by S² for normalized loss units. No floating-point normalization or rounding may enter the reference decision. EIG/Fisher comparators are computed and reported separately; they do not enter this result.

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
7. Robust viability-only selection without information objective.
8. EIG-only without post-probe viability as a negative control only; never a deployment controller.

Selectors cannot access hidden truth, evaluator-only fields, true branch IDs or future observations.

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


## 11. Executable finite Rust V0 reference

The standalone dependency-free Rust crate in this directory consumes:
- an upstream probe-admission receipt and status from the robust probe-continuation layer;
- complete physical branch/outcome possibility pairs and hypothesis/branch support pairs;
- one complete joint mass cube over hypothesis × branch × outcome;
- upstream outcome-specific robust continuation action sets and their evidence receipts;
- passive/no-probe action losses and post-probe action/hypothesis/branch/outcome losses;
- explicit energy, time, actuator and risk resource usage/limits.

It returns one of three gate dispositions before any probability ranking:
- physical rejected (with upstream witness);
- physical Unknown (with reason);
- resource rejected (with the hard-limit dimension).

Only when those gates pass does it evaluate the passive baseline and the outcome-specific admitted continuation choices. The joint mass cube must be complete and sum exactly to the fixed-point scale. A positive joint cell must be supported by both the declared hypothesis/branch support and physical branch/outcome possibility relation. Zero-mass physical branches remain in the upstream robust gate.

Unknown outcomes require an explicit fallback, and the fallback must be the only action exposed for that unknown outcome. Decision losses are evaluated only for the upstream admitted action IDs. No fallback or action set is synthesized by this crate.

The V0 Rust tests cover:
- exact passive risk 0.5, post-probe risk 0.1 and gross viability-conditioned VOI 0.4;
- exact probe-receipt branch inventory binding and a fresh passive/no-probe receipt;
- rejection of loss rows for actions not admitted by upstream continuation evidence;
- physical rejection short-circuit before decision ranking;
- retention of Unknown outcome mass and explicit fallback;
- energy hard-limit rejection before ranking;
- positive mass assigned to a physically impossible cell;
- missing outcome continuation evidence;
- invalid Unknown fallback action set;
- permutation invariance;
- stale continuation receipt rejection.

Qualification commands:

    cargo fmt --manifest-path research/loco-viable-voi-001/Cargo.toml -- --check
    cargo test --manifest-path research/loco-viable-voi-001/Cargo.toml --locked
    cargo clippy --manifest-path research/loco-viable-voi-001/Cargo.toml --locked --all-targets -- -D warnings

The exact-head workflow is authoritative for Rust 1.96. A green result qualifies finite arithmetic and supplied evidence-composition semantics only; it does not authenticate upstream receipts, prove robust viability or validate physical likelihoods, losses or resource measurements.


### Additional evidence and mass invariants added after first CI inspection

The Rust reference now additionally requires:
- a separate fresh, state/model/domain/horizon-bound receipt for the passive/no-probe comparator;
- exact equality between the upstream probe receipt's branch inventory and the branch inventory being evaluated;
- the hypothesis marginal of the full joint mass cube to equal the declared passive prior for every hypothesis (the probe cannot silently change the prior);
- every passive and continuation loss row to reference a known key and an admitted action, with duplicate and foreign keys rejected;
- a physically rejected probe to short-circuit before mass/decision computation even when a physical branch has zero probability mass; the branch remains in the upstream physical possibility inventory.

For joint mass and loss both scaled by (S=10^9), the unnormalized risk numerator and VOI numerator are on scale (S^2), not (S^3). Normalized loss/VOI is obtained by division by (S^2). The reference uses checked integer arithmetic.
