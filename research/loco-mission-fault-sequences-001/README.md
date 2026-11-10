# LOCO-MISSION-FAULT-SEQUENCES-001
## Mission-scale cumulative recovery reserve under sequential disturbances

**Status:** protocol and synthetic known-answer corpus, v0.1  
**Canonical issue:** https://github.com/Luminous-Dynamics/symtropy/issues/4549  
**Single-event campaign:** https://github.com/Luminous-Dynamics/symtropy/issues/4358  
**Recovery topology:** https://github.com/Luminous-Dynamics/symtropy/issues/4511

## 1. Research question

Can a legged robot complete a sequence of disturbances—not merely survive one isolated fault—while preserving enough physical and evidence-qualified reserve to recover from the next event?

The benchmark treats mission recoverability as a **trajectory of a changing physical system**. Battery, temperatures, actuator authority, mechanical condition, sensing/calibration confidence, time and recovery topology carry across every event. The state is never reset between episodes unless a measured and independently qualified recharge, cooldown, replacement, recalibration or repair operation justifies that exact component update.

## 2. Why this is not another single-fault campaign

The canonical #4358 campaign owns matched physical fault episodes: mechanism, onset, phase, severity, terrain, payload, detection, reconfiguration, recovery and independent split. It already includes interacting faults, gradual/intermittent profiles, thermal derating and recovery.

This programme owns a different axis: **ordered multi-event missions with cross-event state carryover**.

A robot may pass a fault and still leave the recovery episode with:
- less battery energy and insufficient reserve for the next safe stop;
- elevated motor, driver, battery or structural temperature;
- reduced torque/slew authority during cooldown;
- accumulated impact, backlash, friction, wear or overload-coupling changes;
- degraded sensor calibration, timing or health evidence;
- less task time, recovery horizon or retreat distance;
- fewer independent physical recovery routes or a smaller minimal cut set.

Fresh-reset trials can make a mission look much more resilient than it is.

## 3. Paradigm: carry a physical reserve state across event boundaries

Represent mission state as a typed vector, not a scalar:

| State family | Examples | Evidence boundary |
|---|---|---|
| Electrical | SOC, bus voltage, usable energy, source resistance | Calibrated pack/BMS and independent current/voltage metrology |
| Thermal | motor, drive, battery and structure temperatures; cooldown | Sensor calibration and thermal response model |
| Actuation | torque/slew envelope, saturation, response delay, derating | Command-to-motion and actuator loss/response evidence |
| Mechanical | impacts, backlash, friction, clutch slip/re-engagement, wear proxies | Calibrated sensor/inspection or declared Unknown; no invented wear law |
| Sensing | bias/health, clock synchronization, calibration validity | Source identity, uncertainty, freshness and common-cause group |
| Mobility | robust continuation, recovery topology, independent paths and cut sets | #4061/#4083 and #4511/#4540 receipts |
| Mission | task/route progress, payload, elapsed time, deadline, retreat target | Frozen mission contract and state observations |

For sequence event \\(k\\), apply observed fault-induced physical changes first, refresh upstream recovery topology, and only then admit and charge the compensating action. A rejected compensation must not erase the fault's actuator/thermal effects. Bookkeeping may be represented as:

\[
z_{k+1}=F(z_k,e_k,a_k,o_k,r_k)
\]

where \(e_k\) is the next disturbance, \(a_k\) is the chosen action, \(o_k\) is the observation and \(r_k\) is the recovery/repair/cooldown transition. This is an interface abstraction, not a claim that one universal state equation is known.

Every state component preserves its own units, calibration, uncertainty, provenance, age and update authority. Unqualified updates remain Unknown or conservative intervals. No implicit health reset is permitted. Mission energy costs debit state of charge, while the separately declared recovery/diagnostic costs debit the recovery reserve; current actuator authority is checked against each event's required authority before that event is admitted. The terminal safe-stop energy requirement is checked against remaining state of charge.

### 3.1 Recovery debt

Track the post-event change in reserve as a vector:
- electrical energy spent and remaining recoverable energy;
- thermal headroom consumed and predicted/measured cooldown;
- actuator authority and response margin lost;
- mechanical damage/wear evidence or unresolved inspection debt;
- sensing/calibration/timing uncertainty and requalification obligations;
- time and route budget consumed;
- changes in independent recovery paths and common-cause cut sets.

“Debt” is descriptive bookkeeping, not a fungible currency. Do not add joules, degrees, milliseconds and wear proxies into one scalar.

### 3.2 Cooldown, retreat and repair are first-class decisions

A controller may choose to wait, cool, slow down, retreat, recalibrate, replace a module or stop. Each operation has resource costs and state effects. A repair or replacement does not restore nominal health automatically: the affected component, firmware, calibration, sensor map and actuator response must be requalified before the corresponding capability returns.

## 4. Hypotheses

1. Event order changes mission outcome even when fault marginals and event counts are identical.
2. A successful recovery can reduce the ability to recover from the next fault.
3. Cooldown, retreat or safe-stop decisions can preserve more future recoverability than immediate task continuation.
4. Individually viable diagnostic probes can collectively consume the reserve needed for a later emergency.
5. Passive mechanical overload protection changes the post-event state, sometimes beneficially and sometimes by introducing disengagement/re-engagement or torque-capacity costs.
6. A sequence-aware reserve vector and #4511 topology measurements predict mission failure better than isolated event success, raw battery SOC or action count alone.

These are falsifiable hypotheses, not established results.

## 5. Sequence corpus and strata

Reuse the exact single-event fault models and evidence from #4358/#4359/#4493. This benchmark owns the event order, cross-event state carryover and sequence-level evaluation split; it does not redefine fault injection.

Initial sequence families:
- repeated moderate torque derating with incomplete thermal cooldown;
- locked-joint fault followed by separate actuator/encoder ambiguity;
- thermal derating followed by high-load terrain and payload transition;
- intermittent drive dropout followed by low-friction contact disturbance;
- individually recoverable events whose combined recovery energy exceeds the initial reserve;
- second fault during active recovery from the first;
- repeated impact/overload events changing clutch, backlash or mechanical state;
- diagnostic probe followed by a fault during the remaining horizon;
- same event multiset in reversed order;
- repair/reconfiguration followed by explicit requalification;
- unknown event after earlier reserve depletion;
- passive-mechanical, active-controller, combined and neither negative-control arms.

Freeze robot/morphology/configuration, firmware, initial state, terrain, payload, ambient conditions, event identities/order/onset/severity/duration/overlap, task, controller budgets, energy boundary, sensor evidence, evaluator-only truth and terminal recovery target.

The controller must not receive hidden future events, evaluator-only fault labels or scenario IDs encoding the answer.

## 6. Matched baseline matrix

1. Isolated episodes with fresh-reset state (diagnostic/optimistic baseline only).
2. Sequential episodes with no cross-event health/reserve memory.
3. Sequential stateful controller tracking battery and thermal state only.
4. Full source-qualified state carryover with conservative unknowns and hard resource constraints.
5. #4511/#4540 topology-aware continuation and cut-set reserve tracked after each event.
6. Passive mechanical protection only.
7. Active controller adaptation only.
8. Combined mechanical and active adaptation.
9. Oracle event/state knowledge as an upper-bound-only analysis arm.

Pair candidates on identical frozen sequences, initial health, event order, onset phase, terrain/payload and exposure. Keep robot morphologies/hardware revisions in separate strata. Match training data, action/observation limits, compute, controller deadlines and energy measurement boundaries.

## 7. Scorecard: trajectories plus terminal outcomes

Report every sequence as a time-indexed trajectory:
- mission completion, useful work, distance, elapsed time and deadline;
- per-event detection/isolation, admission, mode transition, safe-hold, recovery and reconfiguration times;
- full #4326 adverse-event vector and exposure per event and cumulatively;
- energy consumption and remaining recovery/safe-stop energy after every event;
- motor/drive/battery/structure temperature, headroom, cooldown and torque/slew derating;
- actuator command-to-motion residual, response drift and saturation where measured;
- mechanical backlash/friction/impact/clutch state where calibrated;
- sensor health, calibration, timing and uncertainty changes;
- #4511/#4540 unique route classes, independent paths, minimal cut sets and Unknown coverage;
- sequence length/order conditioned completion and recovery distributions;
- remaining horizon, retreat/safe-stop reachability and terminal reserve;
- missingness, stale evidence, out-of-domain model use and requalification debt.

Do not collapse outcomes into one mission score. A completed task with a violated hard constraint, hidden adverse event, unqualified reset or no terminal safe-stop route is not a qualified success.

## 8. Frozen known-answer and adversarial cases

The synthetic corpus includes:
- every event passes alone after a fresh reset, but the ordered sequence fails under real carried-over reserve;
- identical event multiset, reversed order, same additive resource totals but different state-dependent torque-demand admissibility;
- incomplete cooldown and a second fault while torque remains derated;
- a probe sequence that is individually viable but collectively exceeds energy/time budget;
- impact/clutch slip on event one, with an altered coupling state on event two;
- stale or biased SOC/temperature evidence preventing a false reserve claim;
- repair/replacement without requalification cannot restore capability;
- final task completion but no admitted terminal safe-stop/recovery route;
- hidden event/log gap prevents crediting a lower adverse-event count;
- permutation invariance for record ordering, but not event-order invariance.

See \`fixtures/known_answers_v0_1.json\` and its Draft 2020-12 schema. They exercise bookkeeping semantics only; they are not robot measurements.

## 9. Statistical and metrology contract

- The mission sequence / robot run is the independent experimental unit; events inside one run are not independent Bernoulli trials.
- Pair methods on identical event sequences and initial states; randomize run order and control battery/thermal starting points.
- Stratify by robot, morphology, firmware, payload, terrain, ambient temperature and event family.
- Hold out event orders and multi-event combinations; do not tune on the final sequence split.
- Report paired differences and mission-run-level bootstrap/cluster confidence intervals.
- Retain aborted, failed, Unknown, missing-evidence and incomplete sequences in denominators.
- Calibrate energy/current/voltage, temperature, force/torque, timing and adverse-event detection.
- Keep simulation, test rig and physical robot tiers distinct.
- Do not infer physical lifetime or arbitrary fault-sequence reliability from short simulated sequences.

## 10. Industry and SOTA comparisons

- **RAIBO2 (Nature 2026):** holistic hardware, motor-driver and policy energy design; completed 42.195 km in 4 h 19 min 52 s on one charge with reported total CoT 0.25. It is an endurance/energy reference, not a sequential-fault tolerance result. Match total energy boundary, mass, speed, terrain and payload before comparing. DOI: https://doi.org/10.1038/s41586-026-11102-5
- **LoComposition (arXiv:2606.15896v2, 12 Oct 2026):** separates task reward, operational constraints, energy minimization and terrain adaptation; reports 56% lower CoT and 96% fewer operational-limit violations against its own complex-reward baseline, with zero-shot Go2 transfer. Use it as an energy/constraint controller baseline; do not reuse its relative percentages as matched Symtropy scores. https://arxiv.org/abs/2606.15896
- **INSECTRON (npj Robotics, 5 Nov 2026):** magnetic overload couplings and modular leg/drive architecture; reports 91–95% recovery across 360 controlled vertical-perturbation trials. This demonstrates passive mechanical disturbance tolerance, not repeated multi-event mission resilience or quadruped actuator-fault recovery. https://doi.org/10.1038/s44182-026-00106-y
- **UED quadruped benchmark (arXiv:2610.40364, 29 Oct 2026):** 14 curriculum methods across five morphologies; in-distribution winners can differ from held-out terrain leaders. Use its held-out/morphology separation discipline for fault-sequence training/evaluation. https://arxiv.org/abs/2610.40364
- **Anticipatory Gait Switching (arXiv:2610.24895, 17 Oct 2026):** look-ahead MDP gait selection on Go2; compare sequence-aware gait/cooldown/retreat decisions against reactive and anticipatory baselines. https://arxiv.org/abs/2610.24895
- **ISO 18646-5:2026:** standardized locomotion performance methods, explicitly not safety verification/validation. Use for nominal/degraded performance profiles, not mission safety certification. https://www.iso.org/standard/86850.html

The differentiator is not a claim to beat every specialized controller on its native metric. It is to evaluate the system across ordered events, carry state honestly, preserve recoverability and demonstrate mission-level limits.

## 11. Proposed targets (not achieved)

Synthetic reference:
- 100% exact agreement on frozen sequence-state and event-order known answers.
- Zero silent reset of battery, thermal, actuator, mechanical, sensor, calibration, mission or topology state.
- Zero continuation when no robustly admitted terminal recovery/safe-stop path remains.
- Zero acceptance of stale/replayed/mismatched state, health, actuator or viability evidence.
- Zero adverse-category masking through aggregation.
- Event-order effects reproduced exactly on deterministic fixtures.
- Unknown model/state updates remain Unknown; never substitute nominal values.

Physical evaluation:
- Establish non-inferiority in mission completion and no worsening in any preregistered adverse category against the strongest matched stateful baseline.
- Demonstrate sequence-aware reserve adds held-out predictive value beyond isolated episode scores before claiming it.
- Report tested sequence lengths, event-order families and exposure; do not claim universal mission resilience.
- Preserve sufficient energy and independently admitted recovery topology for the exact declared terminal safe-stop target.

## 12. Ownership and claim ceiling

Consumes #4358/#4359/#4493, #4438/#4441, #4511/#4512/#4540, #4044/#4179/#4206, #4119, #4326, #4338/#4340 and #4360/#4362.

It does not create a second actuator-fault campaign, fault detector, physical model, viability solver, recovery-topology auditor or morphology optimizer. A synthetic validator proves only exact finite bookkeeping and declared carryover semantics. Physical mission resilience requires matched repeated-event robot trials with independent metrology.

Claims are limited to the tested robot/hardware revision, sequence/order, terrain, payload, ambient condition, exposure and evidence tier. No arbitrary-fault tolerance, continuous-time safety, certification or service-lifetime claim follows.
