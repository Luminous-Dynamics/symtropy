# LOCO-MODAL-FAILOVER-001 — Recoverability-gated multimodal perception degradation and fallback

**Status:** protocol and synthetic fixtures. No physical failover or safety result.

Issue: [#4332](https://github.com/Luminous-Dynamics/symtropy/issues/4332).

## Objective

When vision, contact sensing, proprioception or actuation telemetry degrades, determine what evidence remains valid, select only a behaviour independently admitted for every possible health state, and preserve a physically recoverable future through the transition.

This workstream owns the cross-modal fault-mode/failover state machine and its composition with upstream mode-transition viability receipts. It does not create another sensor estimator, OOD classifier, contact fusion engine, state estimator, viability solver, controller, sensor driver or energy ledger.

## Current research comparator: PROOF (CoRL 2026)

Wang et al., *PROOF: Perceptive Robustness via Out-of-Distribution Fallback for Legged Locomotion* (CoRL 2026) use a two-stage teacher/student design. The teacher learns perception-driven and proprioceptive fallback behaviours by randomly masking exteroceptive latent features on non-parkour terrain. The student predicts exteroceptive latent features and uncertainty using a CNN-GRU and negative log-likelihood training; a learned gate suppresses unreliable depth features.

The project page describes a Unitree Go2 with RealSense D435 and simulation evaluation under severe visual corruption. Across rough flat, uphill, downhill, upstairs and downstairs, it reports success percentages of 100, 99, 100, 95 and 100 for PROOF, versus 96, 79, 84, 73 and 91 for the MSE latent-distillation ablation. The page also shows real-world qualitative trials: 70 cm gap, 50 cm platform, blocked vision, dim light, leaf occlusion, grass/mud and stair transitions.

Most importantly, it explicitly discloses two limitations:
- its learned gate sometimes fails to switch correctly in sparse grass;
- extended extreme parkour operation overheats the Go2 motors and changes actuator characteristics, increasing the sim-to-real gap.

This is a useful baseline—not a reason to assume a learned OOD gate is sufficient. The next step is to make fallback admissibility depend on the actual remaining multimodal evidence, actuator authority, transition transient, timing and robust recoverability.

Source: https://syw-robotics.github.io/proof-loco/

## Paradigm shift

| Conventional fallback | Symtropy direction |
|---|---|
| Vision confidence -> use proprioception | Typed health state for each evidence group and common-cause dependency |
| One learned OOD score | Source/time/calibration-qualified health evidence plus calibrated/selective detection |
| Assume proprioception is reliable | Model D1/D2/D3 and exteroceptive failure combinations |
| Switch to a nominal fallback policy | Use only a mode and transition independently admitted for all compatible physical states |
| Software gate toggles instantly | Mode transition is a physical action with latency, momentum and support consequences |
| Vision recovers -> switch back | Hysteretic re-entry after source revalidation and fresh viability admission |
| Single-fault tests | Pairwise and selected higher-order correlated sensor/actuator faults |
| Success rate only | Failover timing, recovery, viability, multi-outcome events and post-failover capability |

## Modality families and typed health

Stable evidence-group IDs:
- D1: foot-interface pressure, shear and contact geometry;
- D2: near-foot wrench, CoP, vibration and impact transients;
- D3: joint/body proprioception, motor current, velocity and actuator telemetry;
- X1: depth, stereo/RGB, LiDAR and terrain/map observations;
- X2: inertial, localization or navigation evidence where available.

A health report has source identity, calibration digest, clock mapping, acquisition/arrival timestamps, freshness window, common-cause group, diagnostic model digest and health state:
- Valid: fresh, calibrated, in-domain and integrity-qualified;
- Degraded: usable only under a declared bounded model;
- OOD: outside a calibrated operating domain;
- Unavailable: missing, stale, saturated, disconnected or failed;
- Unknown: evidence is insufficient to classify health.

A confidence score cannot promote OOD, Unknown or Unavailable to Valid. Dependencies must be explicit: two features from one camera do not become two independent modalities; motor position and velocity from one encoder share a failure mode.

Actuator state is separate from sensor health: torque authority, rate/voltage limits, thermal derating, tracking error and command latency. Thermal derating must not be mislabeled as a vision OOD event.

## Mode and transition contract

A mode identifies:
- required and optional evidence groups;
- accepted health states and common-cause assumptions;
- controller/behaviour identity;
- actuator envelope;
- model, domain and abstraction digests;
- compute/observation deadlines;
- upstream mode-transition robust viability/recovery receipt;
- admitted transition action set and explicit emergency/controlled-stop fallback.

Example modes: full perception; vision-degraded contact/proprioceptive; haptic/proprioceptive with vision unavailable; restricted local recovery; qualified controlled stop/external assistance.

Do not assume proprioceptive fallback is safe when D3 is also degraded. The target mode must be qualified for the entire compatible sensor/actuator fault signature.

A transition is a physical action, not a software flag. Its latency, body momentum, current contact/support state, energy and actuator loads are part of the transition prefix. The abstract state cannot be held constant if the robot moves during failover.

## V0 finite composition rule

The evaluator consumes:
- current and target mode;
- source-qualified evidence-health reports;
- the complete declared set of health signatures still physically possible given Unknown/dependency evidence;
- a fresh upstream mode-transition admission receipt for each possible signature;
- state/model/abstraction/domain/actuator/clock identities;
- transition duration and maximum deadline;
- mode rank and upgrade dwell time.

For every possible health signature, retrieve an exact matching receipt. A receipt is valid only for its exact source set, health states, actuator envelope, initial state, model, abstraction, domain, transition and validity interval.

Gate order:
1. Validate source identity, calibration, clock mapping, freshness, common-cause assumptions and sensor/actuator health.
2. Enumerate possible health signatures without deleting one because a learned classifier assigned it low probability.
3. Require fresh upstream physical admission for every possible signature and transition prefix.
4. Intersect upstream admitted action sets across all possible signatures.
5. Select only an action in that intersection.
6. An upgrade to a richer mode requires dwell time, source revalidation and fresh viability admission. A safety-critical downshift is not delayed by upgrade hysteresis.
7. If any signature is Rejected or Unknown, or the common action intersection is empty, do not enter the proposed mode. Return explicit Unknown/rejected disposition and use an emergency-stop action only if separately admitted across every possible signature.

A learned OOD gate can propose a mode. It cannot authenticate its own health evidence or certify the transition.

## Hysteresis and re-entry

- Separate downgrade and upgrade conditions.
- Minimum dwell for upgrades only.
- A maximum downshift-decision deadline that hysteresis cannot exceed.
- Explicit transition-in-progress state.
- A source must pass fresh health and calibration checks before re-entry.
- Re-entry also requires a new viability/transition receipt.
- Fault hypotheses and common-cause groups remain unresolved until evidence discriminates them.
- A probe to distinguish fault hypotheses must pass #4313; information gain does not override viability.

## Matched baselines

- PROOF-style learned uncertainty gate + proprioceptive fallback.
- PROOF-style MSE latent distillation without uncertainty NLL gate.
- Raw OOD threshold + fixed fallback controller.
- Vision-only policy with no fallback (negative control).
- D1/D2/D3 haptic-proprioceptive controller without exteroception.
- Multimodal state estimator with no independent transition viability gate.
- CBF/MPC or qualified controlled-stop baseline.
- Symtropy evidence-qualified mode machine with #4119, #4061/#4083 and #4308 admission.

Match robot, terrain, task, action alphabet, compute, sensor layout, update rate, exposure, actuator state and energy boundaries. Mark unmatched baselines rather than forcing a ranking.

## Fault-injection matrix

Vision: complete/partial occlusion, sparse grass, leaf, reflective surface, glare, low light, depth holes, frozen frame, delay and misregistration. LiDAR: dropout, blind sector, multipath and time misalignment. D1: pressure/shear dropout, saturation, calibration drift and shared wiring/power fault. D2: wrench bias, vibration burst, clipping, delayed/replayed sample. D3: encoder bias/dropout, IMU saturation, motor-current disagreement, clock skew and model residual. Actuator: thermal derating, voltage sag, torque/rate saturation, command delay and increased tracking error.

Correlated cases include camera+depth; IMU+visual odometry; encoder+derived velocity; D1+D2 shared power; thermal derating+high load; and visual OOD+slip. Also test false OOD while perception is useful, missed OOD while a gate remains confident, channel recovery then repeated failure during transition, stale health/viability receipts, transition-induced support loss, no viable fallback, PROOF-like sparse-grass gate failure and motor-overheating stress.

## Scorecard

Keep independent:
- health detector confusion matrix, selective risk/coverage and calibration by channel/fault family;
- detection delay and false OOD/mode transitions per operating hour;
- failover decision latency, transition duration, missed deadlines and chattering;
- possible-signature coverage and signatures with common admitted actions;
- viability/recovery classifications and counterexample witnesses;
- recovery success, time, energy and remaining viable action set;
- task completion, capability envelope and cross-fitted CVaR;
- full #4326 event vector (falls, slip, foot scuff, shank/body collision, impact, emergency recovery, Unknown/missingness);
- actuator thermal/voltage/torque/rate margin and energy;
- re-entry correctness, stale receipt rejections and interventions.

Do not combine OOD classification, physical viability, recovery and task success into one score.

## Proposed qualification targets (not achieved)

- Zero mode transitions without fresh independent viability evidence for every declared possible health signature.
- Zero false-safe transitions in the frozen finite sensor/actuator fault corpus.
- 100% rejection of stale, replayed or identity-mismatched health and viability receipts.
- Bounded failover decision latency within the preregistered control deadline.
- Zero re-entry before source revalidation and fresh physical admission.
- No adverse-outcome regression hidden by task completion or detector accuracy.
- At matched hard constraints, non-inferior held-out capability and tail performance versus PROOF-style fallback, with measurable reduction in unsafe/unrecoverable transition outcomes.

## Claim ceiling

V0 qualifies finite evidence-mode state transitions and admission semantics over supplied upstream health and viability results. It does not validate OOD detector accuracy, physical transition abstractions, continuous-time safety, hardware sensor reliability or robot-level superiority. Hardware qualification must separately validate detectors, actuator models, transitions and the full adverse-event taxonomy.
