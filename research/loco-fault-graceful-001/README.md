# LOCO-FAULT-GRACEFUL-001 — Fault diagnosis, graceful degradation and robust recovery

**Status:** protocol and synthetic fixtures. No physical safety or fault-tolerance result.

Issue: [#4438](https://github.com/Luminous-Dynamics/symtropy/issues/4438).

## Objective

Measure whether a legged robot can move from an unexpected fault to an evidence-qualified, robustly recoverable operating mode.

This workstream composes existing Symtropy authorities rather than creating replacements:
- #4058/#4061 robust finite viability and recovery;
- #4083 sound hybrid abstraction;
- #4119 action-time evidence freshness;
- #4206 actuator command-to-motion response;
- #4304/#4308 coupled uncertainty/viability frontiers;
- #4310/#4313 viability-conditioned value-of-information;
- #4324/#4326 typed multi-outcome safety;
- #4327/#4328/#4329 empirical held-out capability envelopes.

It connects those primitives to the fault-tolerance benchmark layer (#3357/#3475). It does not own raw sensor classification, actuator identification, physical viability computation or the general scorecard.

## 2026 SOTA reference points and their evidence limits

- **FT-CPG** (IEEE RA-L 10(7), July 2025, DOI 10.1109/LRA.2025.3572772): learned CPG coordination under multi-joint locking/power-loss faults with reported zero-shot transfer to a Unitree Go1. This is a hardware locomotion/fault-adaptation baseline.
- **Adaptive Gait Timing** (Gravina et al., arXiv:2608.07328, 7 Aug 2026): proprioceptive-history fault inference and a learnable gait-frequency action. High-fidelity simulation covers uneven terrain; real-world experiments on the 68 kg Kyon cover sudden power-loss fault on flat ground. Do not claim uneven-terrain hardware validation from this paper.
- **FT-WBC** (Zhong et al., arXiv:2606.24466, accepted CoRL 2026): lower-body fault estimation plus whole-body posture adaptation for a Go2 + Airbot Play arm, with real-world demonstrations under weakening/locked lower-limb faults. The official repository currently says code is coming soon; treat as a paper/project-page comparator, not yet as an available reproducible code baseline.
- **ARER-MetaRL** (Wu et al., PLOS ONE, 16 Oct 2026, DOI 10.1371/journal.pone.0361099): short-context task inference, modular policy routing and adaptation for joint locking, with 0–3 locked joints in meta-training and up to six in severity evaluation. The reported experiments are Isaac Sim, not hardware validation.
- **Concurrent servo and velocity-sensor failures in heavy-legged robots** (Control Engineering Practice 169, 106761, 2026, DOI 10.1016/j.conengprac.2026.106761): a sensor/actuator co-failure baseline. Include only where details/data are reproducible.
- **FT-Net** (IEEE RA-L, 2023, DOI 10.1109/LRA.2023.3329766): learned fault recovery under partial/complete motor failures, with three-legged transitions and simulation/hardware experiments.

Sources:
- https://doi.org/10.1109/LRA.2025.3572772
- https://arxiv.org/abs/2608.07328
- https://arxiv.org/abs/2606.24466
- https://doi.org/10.1371/journal.pone.0361099
- https://doi.org/10.1016/j.conengprac.2026.106761
- https://doi.org/10.1109/LRA.2023.3329766

These results are not directly rankable: different robots, fault definitions, terrain, sensors, task objectives and evidence tiers. Match within strata before comparison.

## Paradigm shift: fault-contingent future control

The operational loop is:

1. **Detect:** distinguish a genuine actuator/sensor degradation from contact transients, impacts, terrain changes and normal gait variability.
2. **Isolate without premature commitment:** preserve a set of plausible fault hypotheses and their severity bounds.
3. **Admit physical actions:** consume the robust action/continuation result under every still-physically-possible fault hypothesis. A diagnosis posterior never authorizes an action.
4. **Probe only when useful and viable:** use #4313 to compare diagnostic information value after accounting for probe branches, latency, energy, actuator reserve and post-probe continuation.
5. **Degrade gracefully:** continue at a reduced declared capability only when the corresponding action set is admitted; otherwise brace, safe-hold, retreat or abort.
6. **Recover/reconfigure:** measure time, energy, workspace, actuator reserve and retained task capability—not just whether the controller switched modes.
7. **Reconcile:** stale/missing actuator readback or execution receipts produce Unknown and block unverified follow-on mutations.

The critical distinction is **diagnostic confidence ≠ physical admissibility**. A high-confidence wrong diagnosis must not create an unsafe action.

## Fault taxonomy

Keep distinct:
- joint locked / restricted range;
- partial torque weakening / derating (continuous severity);
- complete power loss / zero torque;
- free-swing / transmission disconnect;
- intermittent actuator output / dropout bursts;
- command delay, actuator tracking delay, saturation, slew limit and stiction/friction change;
- encoder bias, velocity sensor bias, frozen measurement, dropout, quantization and delay;
- motor current, temperature or bus-voltage sensor bias/missingness;
- communications delay, stale command and command/readback disagreement;
- simultaneous actuator + sensor faults;
- unknown / out-of-taxonomy fault.

Each scenario binds exact robot joint identity, body side/segment, fault mode, severity value/unit, onset time, onset gait phase, duration, waveform/schedule, random seed, actuator model, calibration and fault-injection authority.

Inject onset during stance, lift-off, swing and touchdown, across forward/sideways/backward commands, turns, stairs, slopes, impacts, payload and loco-manipulation conditions.

## Controlled injection and evidence tiers

Simulation fault injection, software actuator emulation, hardware-in-the-loop, tethered hardware and untethered hardware are separate evidence tiers.

Physical experiments must use bounded and supervised fault injection (e.g. controlled torque/current limiting and hardware-supported fault simulation), guarded operating areas, low-speed progression, physical restraint where appropriate, and an independent emergency stop. Destructive component damage is not a required test method.

Do not move from one locked-joint hardware demonstration to a general claim over all fault modes. Each result is scoped to its robot, actuator, fault, terrain and sensor domain.

## Diagnosis / action admission evidence contract

At each decision point preserve:
- raw sensor values, source identity, timestamps and clock mappings;
- commanded reference vs delivered reference/readback, measured torque/current/temperature/voltage only where physically measured;
- calibration, actuator and sensor health;
- candidate fault hypotheses and their calibrated joint/correlation provenance;
- posterior/credal set and explicit Unknown/OOD state;
- upstream viability/recovery receipt for each plausible fault hypothesis;
- selected action and its admission result across the whole plausible physical fault set;
- state/model/abstraction/actuator/domain/horizon digest and freshness;
- control deadline, observation age and fallback identity.

For action (a) at decision time (t), define the current plausible hypothesis set (H_t). The action is eligible only when the upstream physical authority reports it admitted for **every** (h\in H_t), under the complete physical successor set and valid receipts. If the evidence is Unknown for any plausible hypothesis, the action is not admitted. A new probe may reduce (H_t) only after its own physical path and post-probe continuation pass #4313/#4308.

A zero posterior probability does not delete a physically possible fault branch from the robust gate.

## Outcome state machine

Every episode reports one terminal or continuation disposition:

- **Nominal:** no fault and healthy-envelope receipt remains valid.
- **Suspected:** fault evidence exists but isolation is unresolved.
- **Degraded-mobile:** only reduced capability is admitted; exact allowed commands/horizon are recorded.
- **Safe-hold / brace:** locomotion is not admitted, but a supported safe state remains admitted.
- **Recovery:** robust path to the declared recovery target remains admitted.
- **Retreat:** task continuation is rejected; a separate retreat/return route remains admitted.
- **Aborted-safe:** task is abandoned while reaching the declared safe target.
- **Unrecoverable / failure:** no admitted action or recovery path exists, with a counterexample witness.
- **Unknown:** evidence, model, timing, identity or compute is insufficient. Unknown is never counted as successful recovery.

A controller mode label is not evidence that the physical transition happened; bind it to action/application/observation receipts.

## Required baseline matrix

Use identical robot/morphology, sensor budget, fault onset phase, terrain/route, command profile, episode horizon and compute/latency budgets:
- healthy nominal controller;
- nominal controller + threshold detector + fixed fallback;
- FT-Net;
- FT-CPG;
- adaptive gait timing;
- ARER-MetaRL (simulation locked-joint comparator);
- FT-WBC (only where reproducible code/configuration is available);
- model-based fault-tolerant WBC/MPC;
- CBF/QP or barrier/shield filter over the same nominal controller;
- context/adaptation-only without robust viability admission;
- viability-only without active diagnosis;
- Symtropy diagnosis + viable information + robust frontier + multi-outcome audit.

Ablate no temporal history, no fault isolation, no active probe, no recovery objective, no set-valued fault hypotheses and no action-time evidence gate. Unavailable or unmatched methods remain out of formal superiority rankings.

## Scorecard

Keep independent dimensions:

**Diagnosis:** detection latency, isolation accuracy, false alarms, missed detections, severity calibration, OOD/Unknown abstention, joint/mode localization and posterior calibration.

**Assurance:** robust action-set size, rejected/unknown gate findings, false-safe actions, decision-to-admission latency, receipt freshness, remaining viable/recovery horizon and deadline misses.

**Task capability:** velocity/pose tracking, route success, progress, time to completion, retained mobility and time in each operating mode.

**Recovery:** probability of reaching a safe state, time-to-safe-state, time-to-nominal-recovery, path length, energy, task horizon remaining and censored/unrecovered episodes.

**Safety:** #4326 event vector—falls, slip, foot scuff, shank/body collision, impact, support/contact loss, actuator saturation/thermal derating, emergency stop, object drop and Unknown/missingness.

**Resources:** electrical/mechanical energy boundaries, torque/slew/thermal headroom, compute, memory, policy/diagnostic latency and communication bandwidth.

Never trade a hard physical constraint against task reward, speed or energy.

## Frozen synthetic known-answer corpus

Machine-readable fixtures: [fixtures/known_answers_v0_1.json](fixtures/known_answers_v0_1.json).

1. **Healthy transient:** a normal touchdown residual must not trigger a confident actuator-fault isolation.
2. **Ambiguous fault:** two plausible fault hypotheses have disjoint safe action sets; only their robust common action may be selected.
3. **High-confidence wrong diagnosis:** the selected action is rejected under the still-possible true fault hypothesis.
4. **Zero-mass physical branch:** a severe fault remains in the physical gate even with zero declared posterior mass.
5. **Unknown/OOD:** unknown fault is abstained; only admitted safe-hold/retreat fallback is allowed.
6. **Joint sensor + actuator fault:** the isolation path cannot depend on the failed velocity channel.
7. **Stale actuator map:** prior validity receipt is rejected after thermal/voltage/configuration change.
8. **Intermittent fault recovery:** debounce/hysteresis avoids mode chatter and false healthy re-entry.
9. **Probe consumes horizon:** post-probe recovery uses the reduced horizon.
10. **No viable motion but safe hold exists:** task is aborted safely instead of forced to continue.
11. **No safe hold/recovery:** report unrecoverable with a concrete witness.
12. **Manipulation CoM shift:** leg fault and arm reachability/object retention are separate outcomes.
13. **Proxy shift:** foot scuff/Unknown increases while shank collision decreases; no all-safety claim.
14. **Deadline miss:** stale decision is not applied; no successful recovery is inferred from an unexecuted command.

## Proposed qualification targets (not achieved)

- 100% exact agreement with finite diagnostic/admission/state-machine fixtures.
- Zero selected actions outside the upstream admitted set for every plausible fault hypothesis.
- Zero stale/mismatched actuator/model/calibration/clock receipts accepted.
- Zero physical fault branches or Unknown mass silently deleted.
- Detection/isolation metrics reported with confidence intervals by fault class, severity, onset phase and robot.
- At matched hard constraints, non-inferior task capability and recovery vs the strongest matched baseline.
- Stretch: ≥20% lower median time-to-safe-state or ≥20% fewer unrecoverable episodes at matched fault exposure, with no worsening of any required adverse category or hard resource limit.
- Progressive hardware qualification on at least two distinct robot/actuator stacks before broad cross-platform claims.

With zero independent failures, a one-sided 95% upper bound below 1%, 0.1%, 0.01% requires approximately 299, 2,995 and 29,956 independent trials respectively. Repeated control ticks within a rollout are not independent fault trials.

## Claim ceiling

The protocol/fixture/Rust reference validates only finite evidence handling and scoring semantics. Simulator evidence qualifies only its declared simulator/model. Hardware fault-tolerance claims require calibrated controlled fault injection, independent event labels, actuator/sensor response measurements, matched domains, recovery evidence, uncertainty intervals and a separately qualified action-time physical assurance path.
