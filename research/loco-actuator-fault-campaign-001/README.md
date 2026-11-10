# LOCO-ACTUATOR-FAULT-CAMPAIGN-001 — Matched fault diagnosis, reconfiguration and recovery benchmark

**Status:** benchmark protocol and synthetic fault-injection manifests. No physical fault-tolerance result is claimed.

Issue: [#4358](https://github.com/Luminous-Dynamics/symtropy/issues/4358).

## Scope and ownership

This is the matched *experimental campaign* for actuator-fault diagnosis and graceful locomotion degradation. It owns:
- source-pinned fault-injection scenario manifests and evaluator-only truth;
- matched controller and adaptation comparisons;
- diagnosis detection/isolation/calibration metrics;
- fault-severity response curves and recovery scorecards;
- independent training/validation/held-out fault and terrain splits.

It does not implement another estimator, fault-state enumerator, controller, viability oracle, safety-vector aggregator, capability envelope estimator, or operating-envelope conjunction.

| Concern | Owner | Campaign consumes |
|---|---|---|
| Command-to-motion, stiction, Stribeck, backlash and actuator response | #4206 | Source-qualified response map and residuals |
| Actuator losses and energy boundaries | #4179 / #4044 | Electrical/mechanical/thermal evidence |
| Runtime certificate and actuator health validity | #4119 | Fresh state/model/actuator/timing receipt |
| Robust viability, recovery and sound abstraction | #4058 / #4061 / #4083 | Independent physical action and recovery admission |
| Multimodal mode transition/fallback | #4332 / #4334 | Health-mode transition semantics |
| Complete common-cause fault possibility space | #4342 / #4345 | Source-qualified fault signature envelope |
| Viable diagnostic action value | #4310 / #4313 | Post-probe continuation and decision value |
| Complete adverse-outcome taxonomy | #4324 / #4326 | Typed events, exposure, severity and measurement coverage |
| Held-out terrain capability and tail metrics | #4327 / #4329 | Capability and cross-fitted lower-tail outcome |
| Qualified operating envelope | #4338 / #4340 | Exact-cell conjunction and limiting witnesses |
| General degraded-loop metrics | #3475 | Complementary broad embodied metrics |
| Matched platform/controller comparisons | #3357 | Platform identity and controller benchmark baseline |

## Research baseline and why the comparison must be matched

- **Adaptive gait timing:** Gravina et al., *Learning Fault-Tolerant Locomotion with Adaptive Gait Timing* (arXiv:2608.07328, 7 Aug 2026) use an asymmetric actor–critic, latent alignment and a learnable gait-frequency command. High-fidelity simulation covers uneven terrain; physical experiments use a 68 kg Kyon quadruped on flat ground. This is a strong gait-reorganization comparator, but simulation and physical validation scopes remain separate. https://arxiv.org/abs/2608.07328
- **Diagnosis-conditioned MoE:** Turrisi et al., *Mixture-of-Experts RL for Fault-Tolerant Legged Locomotion* (arXiv:2606.25965, 24 Jun 2026) use a fault-detection module to activate specialized experts, reporting improved fault-scenario performance over monolithic PPO and conditional-computation benefits. Reproduce the diagnosis module and measure its wrong/Unknown outputs; do not feed oracle truth to the deployable policy. https://arxiv.org/abs/2606.25965 — code: https://github.com/iit-DLSLab/fault-locomotion-isaaclab
- **FT-WBC:** Zhong et al., *FT-WBC: Learning Fault-Tolerant Whole-Body Control for Legged Loco-Manipulation* (arXiv:2606.24466, 23 Jun 2026; CoRL 2026) combine a proprioceptive fault estimator and posture adaptation, with simulation and real Go2-with-arm demonstrations under weakening and locked joint faults. Evaluate in a separate legged-manipulator stratum. https://arxiv.org/abs/2606.24466 — project: https://ft-wbc.github.io/
- **Functional morphology compensation:** Guo et al., *Fault-tolerant control of a mole cricket-inspired quadruped robot with functional arms under single-leg loss* (Advanced Bionics, Mar 2026) use functional arms for missing-leg compensation and report 85.5–90.9% normal mobility in their tested hardware configurations. This is a morphology/control co-design comparator, not a directly matched Go2 policy baseline. https://doi.org/10.1016/j.abs.2026.02.001
- **Concurrent servo/sensor failures:** *Evaluation of servo fault status and fault-tolerant control for heavy-legged robots under concurrent velocity sensor failures* (Control Engineering Practice 169, Apr 2026, 106761) motivates coupled actuator/measurement failure campaigns and capability-aware shutdown; platform and robot class are separate. https://doi.org/10.1016/j.conengprac.2026.106761
- **Active diagnosis and diagnosis-gated recovery:** Tian's SSRN preprint (posted 24 Aug 2026) uses a squat/hip-sway probe before choosing heterogeneous recovery modules. Treat as an exploratory baseline until independent peer review and hardware qualification; no safety guarantee is inferred from it. https://papers.ssrn.com/sol3/papers.cfm?abstract_id=7345456

### Additional 2026 comparators and mechanical co-design

- **ARER-MetaRL (Wu et al., PLOS One, 16 Oct 2026):** context encoder, task-conditioned modular routing and regularized fast adaptation for fixed-angle locked joints. The 12-DoF Isaac Sim study uses 1 kHz physics / 50 Hz control, meta-trains on 0–3 locked joints and tests to six. At one/six locked joints, MELV/MERP are 0.86/0.84 and 0.55/0.54, versus 0.21/0.18 and 0.05/0.03 for single-task RL. This is a locked-joint simulation baseline, not evidence for intermittent faults, derating, free-swing or hardware. https://doi.org/10.1371/journal.pone.0361099
- **LoComposition (Kordos et al., arXiv:2606.15896 v2, revised 12 Oct 2026):** separates task rewards, hard operational constraints, energy minimization and terrain-adaptive exteroception; removes explicit gait priors. It reports 56% lower CoT and 96% fewer operational-limit violations than its complex-reward baseline, with zero-shot transfer to a physical Go2 using LiDAR elevation mapping. Use it as an energy/operational-constraint decomposition baseline under matched fault severity and post-fault task demand; it is not a fault detector. https://arxiv.org/abs/2606.15896
- **Anticipatory Gait Switching (Aladum et al., arXiv:2610.24895, 17 Oct 2026):** near/mid/far height-map bands and a look-ahead MDP that encodes platform-specific gait-switch constraints. Indoor/outdoor Go2 experiments report fewer late switches, less time in ineffective gaits and improved roll stability against reactive baselines. Include it as a terrain-aware gait-supervisor baseline during fault onset and reconfiguration; it does not diagnose actuator faults or certify viability. https://arxiv.org/abs/2610.24895
- **Reference-Free Sampling MPC (Schramm et al., ICRA 2026, DOI 10.1109/ICRA57385.2026.11696779):** spline-parameterized MPPI discovers contact creation/breaking without handcrafted gait/contact sequences and runs on CPU. Go2 hardware validates emergent gaits and basic jumping; complex acrobatics are simulation only. Use as a gait-generation/replanning baseline after fault onset, with identical compute and physical action-admission constraints. https://doi.org/10.1109/ICRA57385.2026.11696779
- **INSECTRON passive mechanical resilience (Romano & Manduca, npj Robotics, 5 Nov 2026):** a centimeter-scale hexapod uses magnetic self-aligning clutches that slip under overload, tolerate load imbalance and support tool-free modular leg replacement. The article reports stable locomotion over multiple slopes/substrates and recovery from vertical perturbations in over 90% of trials. It is not directly comparable to a Go2/ANYmal: keep it in a microhexapod/mechanical-co-design stratum. It is a strong comparator for this campaign's F19–F21 overload-clutch scenarios and motivates four distinct arms: passive mechanism only, active controller only, passive + active, and neither. Report nuisance slip, re-engagement latency, phase error, wear and mission capability—not only whether the robot avoids a stall. https://www.nature.com/articles/s44182-026-00106-y
- **Predictive Safety Curricula (Ovinnikov et al., arXiv:2609.37070, 29 Sep 2026):** data-allocation baseline rather than a fault-tolerant controller. Its ANYmal-D hardware study reports 63% lower shank-collision incidence against learning-progress sampling, but foot scuffs were excluded from the safety cost and increased. Use only as a training/diagnostic-case prioritization arm; score every #4326 adverse category. https://arxiv.org/abs/2609.37070

The evidence roles must remain distinct: ARER is currently a locked-joint simulation baseline; gait-switching, LoComposition and sampling-MPC include Go2 hardware demonstrations within their own stated scopes; INSECTRON is a separate microhexapod mechanical system; PSC is a curriculum/data-allocation method. No headline result transfers across morphology, fault family, task, energy boundary or evidence tier without a matched trial.

- **ISO 18646-5:2026 (published 24 Jul 2026):** use the complete-machine locomotion test conditions and test items as a common nominal-versus-degraded performance spine: rated speed, stopping distance, maximum climbing height, stair capacity, maximum slope, dragging force, crossing capacity, minimum pass-through height and minimum travel width. Bind the exact test conditions and configuration to every pre/post-fault pair. The standard explicitly does **not** verify or validate safety requirements, so ISO performance results must remain separate from robust viability, recovery and the #4326 adverse-outcome vector. Official ISO record: https://committee.iso.org/cms/live/live/en/sites/isoorg/contents/data/standard/08/68/86850.html


## Paradigm shift: fault-qualified graceful degradation

Do not collapse the chain into “classifier says joint 3 failed, therefore switch to expert 3”. The deployable controller should preserve a *set of still-plausible fault hypotheses* while diagnosis is incomplete. Candidate actions and mode transitions are admitted only when the physical transition and required recovery remain valid under every currently plausible fault signature. Active diagnostic motions are evaluated only when their prefix, observation latency and post-probe continuations preserve a recoverable future.

This creates three separate decisions:
1. **Diagnosis:** what fault modes remain plausible, with calibrated set coverage and explicit Unknown?
2. **Reconfiguration:** which transition and adapted action are physically admitted for that entire plausible set?
3. **Mission choice:** continue, reduce speed/load, reach a recovery waypoint, or stop—based on remaining viability, capability, reserve and evidence.

A confident but wrong top-1 diagnosis is not allowed to shrink the robust possibility set. A fault outside the declared model becomes Unknown, not the nearest known class.

## Frozen fault scenario format

Machine-readable synthetic manifests are in [fixtures/fault_scenarios_v0_1.json](fixtures/fault_scenarios_v0_1.json). The current pack contains 21 synthetic scenarios, including passive overload-clutch slip, nuisance slip under nominal load and failed re-engagement.

Each scenario includes:
- source/model and injection-method identity;
- platform/morphology, actuator and sensor component IDs;
- fault family, physical mechanism, severity and units;
- onset time/phase, abrupt/ramp/intermittent profile and duration;
- terrain/task/payload/command and initial contact state;
- hidden evaluator-only fault truth;
- runtime-visible evidence-channel restrictions;
- expected gate semantics and fault-family split assignment.

Runtime inputs must not expose scenario IDs, truth labels, future onset or privileged simulator fields.

### Initial fault families

- Loss of effectiveness and torque/power loss: constant and ramped fractions of nominal authority.
- Locked/stuck joint: specified lock angle, bounded range, rigid or compliant lock.
- Free-swinging/zero torque: passive joint dynamics retained.
- Intermittent dropout: burst duration, duty cycle and onset schedule.
- Rate/slew/current saturation and asymmetric actuator limits.
- Command-response lag and dynamic tracking loss (#4206 model).
- Position/velocity bias, stuck encoder, quantization, delayed timestamp and dropout.
- Thermal derating and supply/driver/common-bus faults.
- Mechanical transmission and overload-coupler faults: backlash, stiction, seizure, torque-threshold slip, nuisance slip under nominal load, delayed/failed re-engagement and repeated-cycle wear. Parameterize torque-slip threshold, hysteresis, re-engagement latency and phase error; do not collapse these into generic torque loss.
- Multi-joint and common-cause combinations, with no marginal-probability multiplication.

### Cross-factors

Cross fault severity with:
- terrain/obstacle family and friction;
- payload and CoM offset;
- gait phase at onset (stance, swing, touchdown, turn, transition);
- sensor-health signature and common-cause fault group;
- task/horizon (stand, turn, walk, steps, obstacle, recovery, manipulation where applicable).

Nested fault axes are only used when increasing severity truly nests the physical uncertainty set. Categorical or non-nested fault mechanisms remain distinct families.

## Matched method matrix

Where reproducible, compare:
1. Fault-unaware nominal baseline (negative control only).
2. Passive robust/fault-augmented baseline with no online diagnosis.
3. Residual-observer or threshold FDD plus fixed compensator.
4. Adaptive gait timing (arXiv:2608.07328).
5. Diagnosis-conditioned MoE (arXiv:2606.25965).
6. FT-WBC (arXiv:2606.24466) in matched Go2+arm stratum.
7. ARER-MetaRL-style context adaptation for locked-joint simulation stratum.
8. Active probe and diagnosis-gated recovery (SSRN method as exploratory baseline).
9. Symtropy set-valued plausible-fault diagnosis + #4345 envelope + #4334 failover + #4061 viability/recovery admission.
10. Ablations: no active diagnosis, no common-cause envelope, no robust action admission, no actuator response/thermal model, no recovery objective; oracle-fault-label policy as an upper bound only.

Formal comparisons require the same platform/morphology, terrain/task, fault onset and severity, initial state/gait phase, sensor budget, command, horizon, interaction/training budget, compute and physical resource limits. Mark non-comparable method/platform pairs as unmatched strata rather than pooled winners.

## Scorecard — keep dimensions separate

**Diagnosis:** fault-family and component confusion matrix; fault-set coverage; false alarms per robot-hour; missed/incorrect isolation; detection delay; severity error and interval coverage; Brier/ECE; Unknown/abstention; false mode transitions; compute and memory.

**Reconfiguration:** time to newly admitted action/mode; mode chatter; failed transition attempts; admitted action-set size; stable-step fraction; task completion; command tracking; fault severity at which controlled stop is required.

**Physical recovery:** robust viability/recovery witnesses, recovery success probability, worst-case horizon and time, recovery energy, remaining viable options and reserve.

**Adverse outcomes:** all #4326 categories—fall, slip, foot scuff, shank/body contact, impact, contact loss, emergency recovery, Unknown and missingness—with counts, exposure and severity.

**Resources:** battery/source, DC bus, actuator electrical and mechanical energy; peak torque/slew, thermal/voltage margin; compute/deadline misses.

**Mission:** route completion, time, throughput, payload/manipulation success and held-out capability. No safety, viability or resource hard gate is traded against task reward.

## Statistical contract

- Freeze train, validation, fault-family held-out and final test manifests before training.
- Pair every method by platform, scenario, seed, initial state, fault onset, fault profile and task budget.
- Training seeds/robot-fault episodes are statistical clusters. Control ticks inside one run are not independent Bernoulli trials.
- Report every fault family and severity separately plus preregistered pooled strata.
- Evaluate abrupt, ramped, persistent and intermittent onset.
- Use independent fault-injection truth/replay; the evaluated controller cannot author or verify its own ground truth.
- Keep simulation, rig, physical hardware and field trials in separate evidence tiers.
- Use #4326 event definitions and exposure denominators; missing/unknown observations are not event-free.
- A diagnosis-set coverage claim must include a nominal coverage target and independent fault-truth labels; a high top-1 score alone is insufficient.

## Proposed targets (not achieved)

- 100% exact scenario and injection-manifest validation on the frozen corpus.
- Zero access to evaluator-only fault truth or future onset by deployable methods.
- Zero action/mode transitions without fresh independent admission for every currently plausible fault signature.
- Zero stale/replayed/mismatched actuator, diagnosis, fault-space or viability evidence acceptance.
- 100% detection of frozen unknown-fault, wrong-isolation, missing-coverage and pre-validation re-entry adversarial cases.
- Report diagnosis-set coverage/calibration with intervals; freeze numerical accuracy thresholds only after baseline reproduction and power analysis.
- At matched hardware/resource budgets, non-inferior held-out capability and no worsening in any preregistered adverse category versus the strongest matched fault-tolerant baseline.
- Stretch: enlarge the held-out fault × terrain × payload set for which the #4340 qualification conjunction remains Valid, without weakening physical, recovery, adverse-event or reserve gates.

## Claim ceiling

This protocol and its synthetic scenario files do not establish arbitrary-fault tolerance, fault-model completeness, continuous-time safety or physical reliability. Any comparison must report its fault families, severity ranges, platform, terrain/task, exposure and evidence tier. The final operating claim is limited to directly tested condition cells whose upstream fault-space, actuator, diagnosis, viability, safety, capability, reserve and assurance evidence is valid.
