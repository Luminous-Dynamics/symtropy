# LOCO-BELIEF-001 — Dual-channel belief-space viability and safe active contact sensing

Status: research specification only; no belief-space solver or physical safety certificate implemented.

Issue: https://github.com/Luminous-Dynamics/symtropy/issues/4100

## Purpose

Add an information-aware locomotion layer that can reduce uncertainty and improve future decisions without conflating probabilistic confidence with robust physical guarantees.

This workstream consumes:
- the sound hybrid abstraction contract in #4083;
- the finite robust viability/recovery oracle in #4058/#4061;
- individual dynamics from #3988;
- contact evidence and active sensing from #4004;
- gait transitions from #3964;
- energy accounting from #4044.

It must not duplicate those models, physics solvers or assurance oracles.

## Research basis and novelty boundary

The contact-sensing survey by Jia et al. (Artificial Intelligence Review, online 19 July 2026) organizes foot-ground evidence into interface fields, near-foot wrench/transient signals and proprioceptive inference. It also discusses controller-facing feasibility envelopes, active haptic exploration and information-health monitoring:
https://doi.org/10.1007/s10462-026-11644-0

Vahs, Verhagen and Tumova, arXiv:2603.10572v2 (5 May 2026, preprint), propose a layered certificate-based architecture for reach-avoid POMDPs, separating task progress, information gathering and safety with belief-space CLFs/CBFs and conformal prediction:
https://arxiv.org/abs/2603.10572

Impact-aware robust convex MPC for quadruped locomotion on uncertain terrain (IEEE RA-L, 2026) is a hardware-relevant robust-control comparator:
https://doi.org/10.1109/LRA.2026.3692086

POMDPs, active sensing, conformal prediction, CBFs and tactile sensing are not new contributions by themselves. The research hypothesis is the value of composing calibrated probabilistic belief updates with a separately maintained set-based recoverability channel, and making active sensing subordinate to explicit action admissibility.

## Separate uncertainty channels

Do not reduce all uncertainty to one covariance or scalar confidence.

1. Epistemic/model uncertainty: individual dynamics, terrain/contact properties and model parameters that may be reduced by informative observations.
2. Aleatoric/environmental uncertainty: disturbances that remain even with perfect identification.
3. Evidence/estimator health: stale timestamps, dropouts, saturation, calibration state, OOD indicators and data association ambiguity.

Observation may reduce epistemic uncertainty; it does not erase disturbance bounds or make an otherwise inadmissible action safe.

## Dual assurance contract

### Channel R — robust set-based
- Consume conservative transitions from #4083 and robust viability/recovery from #4061.
- Reject an action if any declared admissible outcome leaves the required viable/recovery region.
- Return Unknown when the model abstraction is incomplete.
- Never invent probabilities for unmodelled or merely bounded outcomes.

### Channel P — calibrated probabilistic
- Use a declared belief representation and explicit distributional assumptions.
- Report finite-horizon risk, calibration/coverage, sample size and uncertainty intervals.
- Identify calibration population, data split, dependence/exchangeability assumptions and shift limitations.
- Use held-out individuals and terrain, plus OOD/abstention reporting.
- Never promote a probabilistically low-risk action into a robust certificate.

Keep both outputs visible. A policy may require Channel R to pass, or use a separately declared probabilistic operating mode with an explicit risk budget and approval boundary. No silent fallback between modes.

## Belief and evidence record

Bind the following:
- physical-state set or posterior;
- terrain/contact/parameter posterior;
- prior, likelihood and update-algorithm identity;
- observation source, sensor identity, timestamp, latency and clock quality;
- sensor-health flags, saturation, clipping, dropout and stale-data status;
- correlation/dependence assumptions;
- calibration dataset digest, split and population;
- OOD/abstention state;
- epistemic/aleatoric separation where identifiable.

When evidence is stale, missing, contradictory or outside calibration support, widen/branch the belief or abstain. Do not preserve narrow confidence as if an unhealthy sensor were valid.

## Active sensing is a physical action

A contact probe, posture shift, foothold test or exploratory movement must be treated as an action with physical consequences.

A probe is admissible only when:
1. its physical consequences are included in the sound set-valued model;
2. robust viability establishes the required continuation, or a separately specified probabilistic operating mode explicitly applies;
3. energy, time, sensor wear and lost task opportunity are measured separately;
4. predicted information gain is tested on held-out conditions;
5. an explicit fallback exists for uninformative or contradictory observations.

Optimize information gain only among actions admitted by the applicable assurance mode. Information gain is not a safety argument.

## First implementation scope

Start with finite discrete latent contact/terrain hypotheses, not a high-dimensional general POMDP engine.

1. Typed Rust schema for epistemic, aleatoric and sensor-health evidence.
2. Small exact Bayesian reference updater for finite hypotheses.
3. Independent enumeration/reference calculation for tiny fixtures.
4. Robust action-admissibility adapter that consumes #4061 outputs.
5. Calibration and abstention scorecard (no hidden probability-to-certainty conversion).
6. Active probe selector that cannot bypass robust admissibility.
7. Canonical receipts binding model, belief, calibration split, action, outcomes and verifier versions.

## Frozen synthetic corpus

- two- and three-hypothesis terrain/contact models with exact posterior;
- fully informative and uninformative observations;
- contradictory evidence leading to widen/branch/abstain;
- delayed and stale observations;
- missing, saturated or clipped sensor;
- correlated channels that naive conditional-independence fusion would double-count;
- OOD terrain outside calibration support;
- calibration drift across individuals;
- aleatoric slip despite concentrated epistemic posterior;
- robustly admissible probe versus informative but inadmissible probe;
- evidence that narrows model uncertainty but not physical disturbance bounds;
- action-dependent observation likelihood;
- adaptive sampling and stopping;
- input permutation and deterministic replay.

## Qualification scorecard

Report independently:
- exact posterior agreement on finite fixtures;
- robust viable-action agreement against #4061;
- Brier score and log score;
- predictive-set/interval coverage and interval width;
- selective risk versus abstention/coverage;
- OOD detection recall and false alarms;
- sensor-health detection latency;
- predicted versus realized information gain;
- probe energy/time and viability margin consumed;
- calibration by held-out individual, terrain and sensor-health strata;
- worst-case computation latency and deadline misses;
- false robust-safe outcomes (target zero on frozen corpus).

Do not collapse robustness, calibration, task success, information gain and energy into one weighted reward.

## Proposed targets (not achieved)

- Exact posterior agreement on every frozen finite known-answer fixture.
- Zero path for active sensing to bypass robust action admissibility.
- Zero false robust-safe outcomes on the frozen adversarial corpus.
- Target 90% predictive-set coverage only after the calibration design, sample size and tolerance are frozen; report empirical confidence intervals.
- Stale, missing, uninformative, contradictory or OOD evidence must not increase confidence without a justified update.
- Narrower epistemic posterior must not imply narrower physical-disturbance bounds without evidence.

## Matched baselines

At matched model, observation budget, calibration split, compute budget and task, compare:
- robust set-based viability only (#4083 + #4061);
- belief-space CLF/CBF and conformal-prediction method of arXiv:2603.10572 (within its stated assumptions);
- impact-aware robust convex MPC (RA-L 2026);
- fixed/no-probe sensing;
- heuristic active contact probing;
- probabilistic belief update without robust action screening;
- dual-channel architecture.

Primary question: does information gathering improve useful decision coverage or recovery while preserving the robust channel's zero-false-safe property? Report both channels separately.

## Claim ceiling

Exact discrete fixtures qualify the Bayesian arithmetic only. Calibration on a frozen distribution does not imply safety under arbitrary shift. Conformal coverage depends on its assumptions and calibration protocol. Robust viability remains conditional on a sound set-valued model. Hardware evidence additionally requires sensor calibration/timing, physical contact/actuator bounds, OOD trials and measured deadlines.

## References

- Jia et al. (2026), contact sensing survey: https://doi.org/10.1007/s10462-026-11644-0
- Vahs, Verhagen & Tumova (2026), preprint arXiv:2603.10572: https://arxiv.org/abs/2603.10572
- Impact-Aware Robust Convex MPC (IEEE RA-L, 2026): https://doi.org/10.1109/LRA.2026.3692086
