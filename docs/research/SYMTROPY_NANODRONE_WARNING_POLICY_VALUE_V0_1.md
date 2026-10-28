# Symtropy × Symthaea: counterfactual value of warning-triggered control v0.1

**Status:** Research protocol proposal. No implementation, simulator run, real-flight result, or safety claim is certified by this document.

## Research question

When a causal warning fires, is a declared intervention better than continuing the nominal controller from the same simulator state—and does the frozen warning-triggered policy improve whole-episode outcomes on unseen scenario families?

Keep three claims separate:

1. **Prediction:** the warning scores an impending error on a fixed, pre-intervention target definition.
2. **Intervention mechanism:** at a particular checkpoint, a specified response changes the counterfactual outcome relative to continuation.
3. **Policy value:** deploying the frozen warning-plus-response policy changes mission outcomes, burden, and resource use over complete episodes.

Good detection does not imply intervention value. Good checkpoint recovery does not alone imply that the entire policy improves outcomes.

## Research basis and scope

- Nano-Drone (2026) supplies a real-flight multi-horizon dynamics benchmark with approximately 75k samples and 0.5 s open-loop forecasts: https://doi.org/10.1016/j.conengprac.2026.106871
- NanoBench supplies 172 Crazyflie recordings and separate identification, controller, and state-estimation protocols: https://arxiv.org/abs/2603.09908
- The July 2026 NanoBench contamination preprint reports 12.1% lower apparent one-second error under a contaminated protocol, but its predeclared 95% interval crossed zero: https://arxiv.org/abs/2607.18482
- RESCUE (October 2026) evaluates runtime monitors together with counterfactual interventions and finds that detection metrics alone need not imply reduced intervention cost: https://arxiv.org/abs/2610.13516
- Uncertainty-aware residual MPC is a relevant control baseline and replication reference, not a directly comparable result unless vehicle, dataset, controller, horizon, and budget match: https://doi.org/10.1016/j.cja.2026.104424

RESCUE studies manipulation in MuJoCo, not quadrotor flight. Port its evaluation principle, not its thresholds, effect sizes, or safety claims. This protocol likewise must not infer transfer from Nano-Drone to NanoBench or pool their scores.

## Unit of evaluation and estimands

The independent unit is the preregistered scenario/flight family, with initial-condition and exogenous-disturbance seeds nested beneath it. An intervention checkpoint is a repeated measure, not an independent flight.

For scenario family i, define the primary estimand as a paired full-episode policy difference:

    Δ_i = U_i(π_warning+response) - U_i(π_nominal)

where U is a predeclared vector of mission outcomes and costs, not a hidden aggregate score. Report each component separately. Summarize the distribution of Δ_i with paired uncertainty over scenario families and, when model training is variable, fresh training seeds. Any cost-weighted utility requires explicit units, consequence weights, and sensitivity analyses registered before confirmatory scoring.

For diagnostic checkpoint j, compare response a and continue a0 after restoring the same full pre-action state:

    δ_ij(a) = U_ij(a | s_ij) - U_ij(a0 | s_ij)

This is a simulator counterfactual conditional on the model and state-restoration fidelity—not an automatically identified real-world causal effect. Checkpoint-level effects diagnose where a policy does or does not help; they must not replace the full-episode policy estimand.

A post-intervention failure not occurring is not automatically a false alarm: the action might have prevented it. Preserve the untreated/shadow trajectory or branch for prediction-quality labels, then score realized post-action outcomes separately. Never define a warning as correct solely from the outcome that its own intervention changed.

## Counterfactual branch contract

At each checkpoint selected by a frozen evaluation manifest:

1. Save and restore the complete state needed to reproduce subsequent behavior. At minimum record plant state; actuator lag/saturation; battery/voltage state; sensor delay/dropout queues and filter state; controller integrators and memory; monitor/HDC/LTC hidden state; disturbance process state; simulator time; and all relevant random-number state.
2. Fork one branch under nominal continuation and separate branches for each explicitly named intervention (for example, robust fallback, bounded attitude/rate mode, controlled hover, or controlled landing only where the simulator supports it).
3. Couple exogenous randomness across paired branches by semantic keys such as (scenario seed, timestep, disturbance channel). A single mutable RNG stream can desynchronize after different action paths and turn a policy comparison into a different-disturbance comparison. State clearly which stochastic terms are exogenous and which are state/action dependent; use common random numbers only where the coupling is physically meaningful.
4. Store each branch's checkpoint identity, snapshot digest, policy/intervention version, exogenous seed identity, restore-verification result, and output digest. Fail closed when restore equivalence or branch completeness cannot be established.
5. Run through a fixed post-checkpoint horizon and to episode termination where appropriate. Retain timeouts, divergence, non-finite state, constraint violations, and deadline misses using prespecified failure outcomes/penalties; never delete a difficult branch from the denominator.

A matching digest alone does not prove that all hidden simulator state was captured. Include a determinism/replay test and report restoration residuals or equivalence checks for observable state, controller memory, and exogenous disturbance sequences.

## Baselines that separate signal value from action value

Evaluate these on identical preregistered scenario families and budgets:

- **Nominal / never intervene:** current controller throughout; primary baseline.
- **Warning-triggered response:** frozen candidate warning plus one declared intervention policy.
- **Rate-matched random trigger:** same intervention budget and temporal eligibility constraints, with random checkpoint assignment fixed by a separate seed schedule.
- **Simple operational trigger:** a predeclared recent-error, residual-spectrum, actuator-saturation, or telemetry rule.
- **Always-intervene / conservative reference:** only when it is a meaningful simulator comparator; label it as a reference policy, not automatically as a bound.
- **Oracle trigger:** optional diagnostic ceiling using simulator ground truth. Never call it deployable or use its result as a fair model baseline.

Use the same intervention for the warning and random-trigger arms to isolate where an intervention is allocated from what the intervention does. A second factorial comparison may vary the intervention policy while holding the trigger fixed. Distinguish the trigger-information effect from the response-design effect.

## Required reporting: separate scoreboards

### A. Warning prediction

On targets defined before intervention: AUROC, average precision/AUPRC, event recall, warning lead, missed severe events, false-alert episodes per flight-minute, and realized alert rate. Report paired flight-bootstrap deltas at matched and actual alert rates, preserving undefined metrics as null.

### B. Checkpoint mechanism

For each trigger × response pair: fraction of triggered checkpoints still recoverable under the declared response; successful recovery fraction; outcome difference versus continuation; remaining recoverability horizon; intervention latency; and failed/invalid branch counts. Report conditional-on-trigger results as diagnostics, with denominators explicit.

### C. Full closed-loop policy value

At preregistered horizons and by regime report:

- mission completion, timeout, divergence, and hard-boundary violations (count, duration, and severity);
- tracking error, with all failures represented in the primary failure-aware endpoint;
- battery/energy cost, actuator saturation and intervention count/duration;
- time-to-recovery, missed recoverable failures, unnecessary intervention burden, and any modeled mission reset/restart cost;
- p50/p95/p99/max end-to-end warning-to-action latency, memory and hard deadline misses on declared target hardware.

Do not merge meters, joules, violation counts, and milliseconds into a single headline number. If a domain-specific cost is used for policy selection, declare it and show the entire outcome vector plus sensitivity to each uncertain cost.

## Split, inference, and receipt rules

- Split by complete recording/flight and scenario family; keep all checkpoints and rollout starts from a family together. Do not randomly split overlapping windows.
- Freeze model artifacts, training seeds, thresholds, action policy, intervention availability, rollout horizon, failure penalties, scenario distribution, and all primary endpoints before confirmatory evaluation.
- Use development scenarios to build the method, validation scenarios to select thresholds/policy, and untouched confirmatory scenario families for final estimates.
- Use the same exogenous scenario seeds and initial states across policy arms where meaningful. Resample the same complete scenario-family clusters for paired policy differences; cross family-level resampling with fresh training seeds if the trained model is part of the claim. Branches sharing a checkpoint are nested outcomes, never extra independent sample units.
- Report uncertainty over scenario families and training seeds. With too few independent families/seeds, return NOT_ESTIMABLE instead of narrow intervals based on hundreds of correlated checkpoints.
- Every report binds input/manifest digests; exact simulator/plant/controller/monitor/intervention versions; model/config/checkpoint/scaler hashes; training and scenario seed schedules; thresholds; unit/horizon; branch restore checks; source commit(s); run/toolchain identity; and evidence class.
- Keep exploratory and confirmatory reports distinct. A test-informed threshold or selected endpoint invalidates the confirmatory label.

## Negative controls and required tests

1. **Exact replay:** same checkpoint, same policy and keyed exogenous disturbances reproduces the expected trajectory within preregistered numeric tolerance.
2. **Restore corruption:** omit or mutate actuator delay, controller integrator, sensor queue, or HDC/LTC memory; the restore check must detect it or make the limitation explicit.
3. **No-effect intervention:** in a known synthetic fixture, the policy difference is zero in expectation and the estimator's repeated-trial coverage/type-I behavior is measured.
4. **Known benefit / harm:** synthetic response cases with declared beneficial and harmful effects recover the correct sign within stated uncertainty; no fixture is labeled physical evidence.
5. **Random-trigger control:** compare warning trigger against a rate-matched random trigger under the same response and scenario/seed budget.
6. **Prevention label:** a severe error prevented in the intervention branch remains a pre-intervention warning opportunity; it is not silently relabeled false positive because realized post-action error was low.
7. **Selection leakage:** threshold/policy chosen after confirmatory scores are available must be rejected or marked exploratory.
8. **Incomplete grid:** missing/duplicated family × seed × policy × intervention × checkpoint branches fail closed; failed branches stay visible.
9. **Disturbance coupling:** test that exogenous disturbance tapes match across paired branches even after action-dependent execution paths diverge.
10. **Budget and timing:** late warnings, late responses, missed deadlines, unbounded action duration, and intervention-capacity exhaustion count against the policy.

## Proposed targets (research targets, not standards or results)

- At least 20% fewer safety-boundary violations than the strongest preregistered, matched-budget baseline, with no more than 5% relative tracking-error regression; also report absolute counts and paired intervals.
- Positive paired full-episode policy value versus both nominal/never-intervene and rate-matched random-trigger baselines, on untouched scenario families and across the declared disturbance regimes.
- No material rise in mission failure, actuation saturation, energy cost, or hard deadline misses hidden by the primary metric. Report all as separate gates.
- 100% of registered branches accounted for; 100% of invalid-state/restore, future-informed threshold and incomplete-grid controls fail closed.
- If the interval includes no effect or fails the preregistered worthwhile-benefit margin, report the result as inconclusive or failed—not as a win based on a favorable point estimate.

Targets do not qualify flight safety. A positive synthetic result is evidence only about the declared simulator and scenario family. Hardware-in-the-loop and physical flight need independent, staged qualification, matched safety protocols and separate evidence receipts.

## Implementation sequence

1. Freeze the state-snapshot and deterministic disturbance-tape contract.
2. Add a tiny standard-library or existing-dependency synthetic oracle with null, benefit, harm, prevention, late-warning, and restoration-corruption fixtures.
3. Add strict report/receipt validation and complete branch-grid checking.
4. Integrate with the failure-aware rollout evaluator from [PR #2796](https://github.com/Luminous-Dynamics/symtropy/pull/2796) only through a versioned input/report contract; do not make that open-loop evaluator imply policy benefit.
5. Run the synthetic controls locally and in a focused CI workflow; then select a simulator implementation and compare existing-controller and fallback responses.
6. Only after the offline gates are met, run paired closed-loop simulation under actuator lag, voltage/battery shift, sensor delay/dropout, and bounded disturbances. Hardware evidence remains a separate future gate.

## Claim ceiling

This protocol is not itself a result. A passing synthetic oracle validates evaluator behavior under known fixtures. A passing simulator experiment supports only a simulator-scoped comparison. Neither proves real-flight superiority, flight safety, or generalization across vehicles.
