# LOCO-FAULT-ADAPT-001 — Control-authority-aware fault-tolerant locomotion

**Status:** protocol and synthetic fixtures. No robot fault-tolerance result or safety claim.

Issue: [#4402](https://github.com/Luminous-Dynamics/symtropy/issues/4402).

## Research objective

After an unannounced loss or degradation of control authority, can a legged robot detect what changed, infer what remains controllable, reorganize coordination, modify its task plan and preserve an independently recoverable future?

This is not simply fault-conditioned gait generation. It composes:
1. Fault observability and evidence quality.
2. A fault-conditioned remaining control-authority envelope.
3. Adaptive coordination: gait period, phases, contacts, support and load redistribution.
4. Task-level graceful degradation: continue, slow, retreat, stop or request help.
5. Independently qualified robust viability and recovery after each transition.

The controller may adapt its beliefs and behavior. It may not adapt away physical constraints, erase a possible fault branch or reuse a pre-fault certificate after its assumptions change.

## Current SOTA baselines and their boundaries

**ARER-MetaRL / locked joints (simulation).** Wu et al., PLOS One, 16 Oct 2026, evaluate a 12-actuated-joint quadruped in Isaac Sim. Meta-training covers 0–3 stuck joints, with severity tests up to six simultaneous locked joints. It combines context inference, modular policy routing and gradient regularization; reported MELV/MERP at one/six locked joints are 0.86/0.84 and 0.55/0.54, compared with 0.21/0.18 and 0.05/0.03 for the single-task baseline. This is a simulation result. The paper explicitly identifies actuator delay/bandwidth, torque saturation, backlash, sensor noise, communication latency and contact uncertainty as physical-transfer challenges.  
Source: https://doi.org/10.1371/journal.pone.0361099

**Adaptive gait timing / actuator power loss (hardware + simulation).** Gravina et al., *Learning Fault-Tolerant Locomotion with Adaptive Gait Timing* (arXiv:2608.07328, 7 Aug 2026), use latent alignment from proprioceptive history and a learnable gait-frequency action. The paper validates on a 68 kg Kyon: real-world experiments on flat ground under power-loss faults, and uneven-terrain tests in high-fidelity simulation. The paper's reward uses shank-collision and foot-slip proxies, so our multi-outcome event audit remains necessary.  
Source: https://arxiv.org/abs/2608.07328

These are complementary baselines, not directly interchangeable. ARER-MetaRL explores many locked-joint configurations in simulation; adaptive-gait-timing adds physical power-loss evidence but narrower hardware terrain coverage. Formal comparisons must use matched robots, faults, task, exposure and compute.

## Ownership boundaries

| Evidence / mechanism | Existing owner | Composition use |
|---|---|---|
| Motor/gearbox power and loss map | #4044 / #4179 | Actual control authority and energy boundary |
| Actuator transient response, stiction and tracking | #4206 | Command-to-motion fault dynamics |
| Contact sensing and correlated fusion | #4004 / #4017 / #4230 | Fault-vs-slip-vs-terrain discrimination |
| Robust viability and recovery | #4058 / #4061 | Independent admissible actions and safe-stop/recovery |
| Sound set-valued transition abstraction | #4083 | Physically possible fault/contact successors |
| Certificate validity on state/actuator/sensor change | #4119 | Invalidate stale pre-fault assurance |
| Coupled multi-axis viability boundary | #4304 / #4308 | Fault × terrain × delay × reserve uncertainty |
| Active contact information and viable VOI | #4310 / #4313 | Admitted diagnostic probes and post-probe continuation |
| Adverse event vector | #4324 / #4326 | Full failure taxonomy and proxy-shift suppression |
| Held-out empirical capability envelope | #4327 / #4328 / #4329 | Transfer and empirical capability, not robust proof |

This workstream owns the fault taxonomy, scenario matrix, matched comparison, latency/recovery scorecard and finite coverage auditor. It does not duplicate any of the above or implement another viability solver.

## Fault taxonomy V0

**Actuator / transmission**
- reduced continuous torque or directional torque authority;
- complete torque/power loss;
- locked joint at fixed or uncertain angle;
- free-swinging / unpowered joint;
- intermittent torque, dropout or delay;
- torque/current/rate saturation;
- backlash, stiction, Stribeck friction, hysteresis or tracking drift;
- thermal or supply-voltage derating.

**Sensing / communication**
- encoder offset, drift, quantization, dropout or stuck signal;
- IMU bias, saturation or delayed samples;
- force/torque/contact sensor bias or dropout;
- clock offset/uncertainty and out-of-order arrival;
- command transport delay, stale command or duplicate command;
- shared power/clock/common-cause failure.

**Combinations**
- joint and terrain/contact fault simultaneously;
- multiple joints on one limb and across limbs;
- correlated multi-actuator supply fault;
- actuator + encoder/IMU fault;
- intermittent failure/recovery and repeated mode changes.

These are typed dynamical families, not one scalar severity. A torque multiplier is not automatically equivalent to a locked or free-swinging joint. Each injected fault must bind to a validated actuator/sensor model and explicit physical interpretation.

## Experimental matrix

Every scenario records:
- morphology and exact robot configuration digest;
- actuator/sensor model, firmware, controller and physics digest;
- fault family, affected joint/channel set, magnitude/severity interval and direction;
- onset timestamp and locomotion phase (stance, swing, push-off, flight, transition);
- fault persistence, intermittency and recovery profile;
- terrain family, friction/contact uncertainty, payload and command trajectory;
- sensing/communication latency and clock-mapping evidence;
- train/validation/held-out identity and replay digest;
- admissible operating limits, safety/viability receipt and fallback.

Start with a controlled covering design, not the full Cartesian product. Use pairwise coverage for exploration, but reserve explicit adversarial combinations for fault × gait-phase × contact-mode × terrain × sensor-delay. Never claim exhaustive coverage from a sparse pairwise design. The frozen manifest identifies all untested combinations.

## Matched policy baselines

1. Healthy nominal controller with no adaptation (negative control).
2. Known-fault oracle + fault-specific gait/controller (information upper bound, not a deployable fair baseline).
3. Fault-unaware domain-randomized RL.
4. Random joint masking / torque-scaling curriculum.
5. ARER-MetaRL-style context inference and modular routing (locked joints).
6. Adaptive gait-frequency / latent fault inference (power loss).
7. Conventional fault detector + controller switching.
8. Model-based control reconfiguration using measured torque/velocity constraints.
9. CBF/QP or robust MPC with reduced control authority.
10. Reference-free sampling-based MPC (no predefined gait/contact sequence).
11. Symtropy: fault evidence + remaining control-authority set + adaptive coordination + robust viability/recovery gate.
12. Symtropy ablations: remove history inference, active probe, timing adaptation, set-valued fault outcomes, recovery objective, viability gate, actuator reality map and multi-outcome event audit.

A baseline that has privileged true fault identity at deployment must be labelled an oracle upper bound. Match training samples, interaction steps, compute budget, test exposure, morphology and operating constraints. Unmatched runs do not enter formal superiority rankings.

## Scorecard — no scalar fault-tolerance reward

**Diagnosis:** onset-to-detection latency; fault localization; severity error; calibration; false alarm; Unknown/abstention; ability to distinguish fault from slip/terrain/impact.

**Adaptation:** detection-to-stable-gait latency; time to regain minimum task capability; gait frequency and phase change; contact sequence; load redistribution; residual control authority by joint/direction; adaptation oscillation and reversals.

**Task and physical outcomes:** task completion; route progress/speed; recoverable stop/retreat success; recovery time; falls; slip; foot scuff; shank/body collision; impact; emergency recovery; contact loss; thermal/torque saturation; Unknown/missingness. Consume #4326's vector, do not reimplement its classifier.

**Assurance:** certificate invalidation time after fault onset; set-valued successor completeness; robust viability/recovery admission; false-safe count; counterexample witness; admitted continuation set after observation/adaptation delay.

**Resources:** energy by #4044 boundaries; actuator losses by #4179; transient delivered-motion/torque mismatch by #4206; temperature and voltage headroom; observation age; controller/inference latency; missed deadlines; memory/compute.

## Safety and adaptation gate sequence

1. Detect possible fault and create a new evidence epoch; invalidate all stale actuator/state assumptions and dependent certificates.
2. Maintain a belief/set over fault hypotheses, including Unknown; do not collapse ambiguous evidence to one fault label.
3. Construct or obtain the qualified set-valued physical successor and remaining control-authority evidence.
4. Evaluate candidate adaptations, including diagnostic probes, only after the robust physical gate and fallback/retreat admission.
5. Check the entire mode transition, actuator and sensing delay, and remaining horizon; do not check only the final gait.
6. Apply the command only when its evidence receipt is current and action identity matches.
7. If diagnosis or viability is Unknown, fall back to the independently admitted conservative action or safe stop. Do not continue on an unqualified adaptation.

## Frozen adversarial cases

Machine-readable synthetic cases are in [fixtures/known_answers_v0_1.json](fixtures/known_answers_v0_1.json).

- sudden complete torque loss during stance, swing, push-off and contact transition;
- same nominal fault at different joint locations and locking angles;
- reduced torque with directional saturation and asymmetric actuator map;
- locked joint vs free-swinging joint with superficially similar proprioception;
- intermittent dropout near touchdown;
- fault evidence confounded by low friction, slip or foot scuff;
- encoder drift vs real mechanical deflection;
- IMU/clock delay coincident with actuator failure;
- common voltage drop across multiple motors;
- stale pre-fault certificate presented after actuator model change;
- gait frequency adaptation improves tracking but removes retreat/stop viability;
- one fault label with two possible physical successor modes; preserve both;
- two simultaneous faults with a hidden common-cause branch;
- adaptation computation misses its command deadline;
- diagnostic probe consumes remaining actuator, thermal or risk reserve;
- zero-probability but physically possible branch eliminates recovery.

## Proposed qualification targets (not achieved)

- 100% exact fault scenario, injection, morphology and matched-trial identity validation.
- Zero stale pre-fault certificate acceptance after any fault-relevant state/model/actuator/sensor/clock change.
- Zero commands outside independently admitted action/continuation sets.
- Zero loss of Unknown/alternative fault hypotheses in the evidence path.
- Adaptation latency reported as a distribution by family, gait phase and severity. Stretch target: within one 50 Hz controller period (20 ms) for directly observable abrupt torque-loss detection; do not impose this target on ambiguous faults without testing.
- Improve held-out task/recovery frontier at matched limits without worsening any preregistered adverse category.
- Maintain an independently admitted stop/retreat option wherever the physical system actually has one; otherwise report no-recovery scenarios explicitly.
- Demonstrate held-out joint identity/severity and simultaneous-fault generalization without true fault labels at deployment.
- Keep simulation, supported/tethered hardware and untethered hardware evidence separate.

## Claim ceiling

V0 qualifies scenario completeness, identity matching, trial comparability and fault response scorecard semantics over supplied records. It does not prove a robot can tolerate any fault. Physical claims require controlled fault injection, measured actuator reality, independent event labels, sufficient matched trials and a separately qualified robust viability/recovery gate.


## 11. Executable finite Rust V0 reference

The standalone, dependency-free Rust crate consumes:
- a frozen list of held-out fault scenarios and explicit training/validation scenario IDs;
- policy descriptors, marking whether privileged fault truth is available in training and/or deployment;
- shared morphology, actuator, sensor, terrain, command, task, physics, taxonomy, detector, interaction/compute budget and exposure identities;
- paired test-block IDs;
- one trial record for every policy × held-out scenario × paired block.

The reference:
- rejects policy comparisons with different morphology or actuator/sensor/task/physics/compute/exposure contexts;
- rejects train/validation/held-out scenario leakage;
- rejects deployment access to true fault identity unless the method is explicitly marked as an oracle upper bound;
- validates severity, affected components, onset phase, fault-injection and model identity;
- requires complete matched trial coverage and rejects duplicate/missing trials;
- requires each trial to carry independent event-vector and viability receipt digests;
- rejects stale-certificate or unadmitted applied actions;
- retains Unknown and missed fault diagnoses separately from false family classification;
- reports detection and adaptation deadline misses independently;
- preserves separate controlled-stop, recoverable-fall, unrecovered-fall, task-completion and Unknown outcomes.

The V0 tests include matched matrix coverage, missing/duplicate trials, train/validation/test leakage, context mismatch, privileged truth misuse, stale certificates, unadmitted commands, Unknown diagnosis, fault misclassification, late detection/adaptation, recovery-evidence inconsistency, scenario-model mismatch and invalid resource context.

Qualification commands:

    cargo fmt --manifest-path research/loco-fault-adapt-001/Cargo.toml -- --check
    cargo test --manifest-path research/loco-fault-adapt-001/Cargo.toml --locked
    cargo clippy --manifest-path research/loco-fault-adapt-001/Cargo.toml --locked --all-targets -- -D warnings

The exact-head workflow is authoritative for Rust 1.96. The crate audits supplied benchmark identities and records; it does not diagnose raw telemetry, classify physical events, prove actuator models or certify viability.
