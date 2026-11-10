# Symthaea Driver Apprenticeship Contract v0.1

**Status:** research specification; not an implementation or driving result  
**Symthaea owner:** [#11414 — embodied driver apprenticeship](https://github.com/Luminous-Dynamics/symthaea/issues/11414)  
**Symtropy environment owner:** [#4536 — camera-first virtual driving environment](https://github.com/Luminous-Dynamics/symtropy/issues/4536)  
**SAM integration:** #11258, #11291, #11349, #10043, #10320, #11375, #11393, #11394, #11396  
**Execution integration:** #3532, #3387, #3543, #3562, #3571, #4451, #4514

## 1. Purpose

Define a human-style apprenticeship programme in which Symthaea learns to drive by receiving observations, issuing controls, experiencing environment consequences, learning from demonstrations and corrective interventions, and progressively transferring skills to unseen situations.

"Human-style" describes the training process, not a claim that Symthaea has human cognition, human consciousness, or human-equivalent driving competence.

A VR headset is optional. The essential property is a closed-loop, partially observed virtual world with realistic sensor timing and control consequences. The model consumes virtual sensor streams; the human instructor may use a wheel/pedals, gamepad, keyboard, or optional HMD.

## 2. Current starting point and honest gap

The existing public `symthaea-vehicle` implementation is a useful sensorimotor research substrate:
- `VehicleHdcEncoder` maps 20 engineered simulator-state/road/mesh channels to a 16,384-dimensional ContinuousHV;
- `VehicleController` applies an HdcLtcUnifiedNetwork and a steering/throttle/brake projection;
- the documented controller configuration uses three layers of eight neurons;
- the training loop uses a bicycle-model simulator, a 200 Hz physics / 50 Hz training design, a PD cruise teacher and a staged LaneKeep curriculum;
- FEP modulates selected controller parameters.

This is **not yet** camera-first driving. The current encoder receives numeric state channels, not raw camera frames. The bicycle model is a simplified single-track dynamics model. The crate's own top-level module documentation describes the camera coprocessor, host, CAN actuators and communications diagram as a target deployment shape, not implemented hardware.

Therefore the first research step is not to declare HDC-LTC superior. It is to add a qualified perception path and compare the hybrid model against matched recurrent and transformer baselines.

## 3. Research-informed architecture

### 3.1 Hybrid architecture, not an HDC-versus-transformer contest

Test a compositional architecture with explicit responsibilities:

1. **Perception encoder:** camera/video features, optional stereo, and explicitly declared proprioceptive/instrument channels. A compact convolutional or pretrained vision encoder is a reasonable starting point. HDC is not assumed to replace visual feature learning.
2. **Structured HDC scene representation:** bind actor identity hypotheses, object attributes, relations, road geometry, source roles and temporal identity. Preserve multiple hypotheses and unknowns.
3. **Continuous-time temporal memory:** HDC-LTC/CfC state maintains event timing, sensor asynchrony, temporal context and memory across variable observation intervals.
4. **Action-conditioned latent world model:** predict future ego/actor/road state and observation outcomes conditioned on candidate actions. Train on actual transitions, not video appearance alone.
5. **Policy learner:** begin with demonstrations, progress to corrective intervention/DAgger and constrained simulator-only RL after the environment and evaluation gates exist.
6. **SAM assurance interface:** provide evidence, uncertainty, action envelopes, decision-sensitive query requests, option fragility and recovery timing. The learner cannot self-authorize its own safety status; #10012 remains the independent runtime-assurance owner.
7. **Replay and consolidation:** freeze checkpoints, recurrent state, scenario manifests and all action/observation receipts for reproducible evaluation.

HDC/LTC's hypothesized advantages are compositional binding, irregular-time memory, temporal adaptation and efficiency. The hypothesis must be falsified against compact GRU/LSTM and transformer baselines under matched inputs, data, compute and deployment limits.

### 3.2 Why action-conditioned learning matters

ICLR 2026's World-In-World evaluates world models in closed-loop embodied environments and reports that visual quality alone does not guarantee task success; controllability and action-observation post-training matter. DriveVLA-W0 (ICLR 2026) is a current driving-specific comparator combining future visual prediction with VLA planning. DriveWorld-VLA (ICML 2026) is another strong latent world-model/VLA comparator with reported NAVSIMv2 results.

SAM should ask a stricter question than whether a generated future looks plausible: **does the model predict the consequences of candidate actions accurately enough to improve a decision, while preserving options and respecting deadlines?**

### 3.3 Apprenticeship as active experiment design

Do not merely reduce generic prediction uncertainty. Use SAM's decision-sensitive measurement and option-fragility mechanisms to choose:
- which scenarios to practice;
- which fault or actor behavior to vary;
- when a second observation could change the action;
- when to request human correction;
- which previously learned skill needs consolidation.

A human demonstration is most useful when it can change a feasible action, action ranking, recovery outcome or unresolved policy choice before the relevant deadline.

## 4. Curriculum and promotion gates

A curriculum is a versioned set of skills and promotion evidence, not a fixed percentage of training episodes. A learner can revisit earlier stages. Stage promotion requires held-out evidence and task progress, not merely reward.

### Stage 0 — sensorimotor calibration
Learn steering sign/rate, throttle, braking, actuator delay, inertial response and vehicle dimensions in a low-speed empty proving ground. Test action-axis signs, units, rate limits and coordinate frames.

### Stage 1 — procedural vehicle control
Lane centering, speed control, curves, smooth braking, stopping, turns and route following. Human demonstrations may initialize behavior; teacher influence must decay under a declared schedule.

### Stage 2 — road rules and route semantics
Traffic signals, road markings, lane changes, stop/yield rules, roundabouts and route following. Traffic-side/jurisdiction profiles are explicit data. Never infer left-hand/right-hand traffic from steering-wheel position.

### Stage 3 — interactive traffic
Following, gap selection, merging, intersections, unprotected turns, pedestrians, cyclists, buses and emergency vehicles. Use reciprocal actors when the scenario requires other agents to respond to the ego vehicle's actions.

### Stage 4 — uncertainty and adverse conditions
Occlusion, glare, rain, fog, night, stale frames, dropped sensors, localization drift, friction change, actuator delay, compute deadline misses and common-cause faults.

### Stage 5 — active apprenticeship and long-tail practice
Use #10043/#10320/#11375 to find decision-changing scenarios and request human feedback only when the information could change a decision or recovery status. Keep all queries, corrections and outcomes receipted.

### Stage 6 — consolidation and transfer
Freeze prior skills, replay regression episodes, and test held-out maps, geometry, vehicle dynamics, weather, camera calibration, actor styles, traffic conventions and fault families. Opening a sealed holdout and adapting to it creates a new evaluation generation.

## 5. Human demonstration protocol

The instructor may use a calibrated wheel/pedal set, keyboard/gamepad, and optionally an HMD with head tracking. Capture only with informed consent and rights-cleared provenance:
- exact policy-view image/frame identities;
- sensor configuration and timing;
- input control axes, calibration, dead zones and units;
- command and applied-actuator timestamps;
- route/scenario profile;
- explicit corrections, takeovers and instructor comments;
- learner predictions, actions and outcomes.

Do not treat simulator truth, teacher actions, evaluator annotations or future outcomes as ordinary learner observations. The #11258 privilege-separation contract and frozen evaluation must prevent teacher blending, future-label leakage, online updates or exploration noise during assessment.

Human driving simulators have been shown to improve some immediate simulated skills, but a systematic review of novice-driver interventions found the evidence for real-road transfer and long-term safety benefits unclear and low-confidence. Consequently this programme must keep simulator skill, human-likeness and real-world transfer as separate claims.

## 6. Frozen comparison arms

| Arm | Model | Purpose |
|---|---|---|
| A | Existing HDC-LTC + engineered 20-channel state + PD teacher | Current vehicle-crate reference |
| B | Same HDC-LTC with learned camera encoder + structured scene | Isolate visual representation |
| C | Compact GRU/LSTM + same perception | Recurrent baseline |
| D | Compact transformer temporal policy + same perception | Attention baseline |
| E | HDC-LTC + action-conditioned latent world model | Test predictive learning |
| F | E + SAM decision-sensitive sensing/recoverability | Test assurance-guided learning |
| G | Human instructor demonstrations/corrections | Human reference distribution, not an autonomous policy |

Ablate scene binding, LTC dynamics, world-model loss, demonstrations, corrective interventions, active query selection, domain randomization and SAM gating.

Match observations, action spaces, training samples, environment transitions, compute budget, model/parameter budget where feasible, precision, hardware and p50/p95/p99 inference budget. When an exact match is impossible, disclose the mismatch and do not claim a causal advantage.

## 7. Benchmark ladder

1. Analytic vehicle-control fixtures and existing bicycle-model tests.
2. Minimal visual closed-loop virtual environment with complete receipts.
3. CARLA closed-loop evaluation after exact source composition, environment and rights are pinned.
4. Bench2Drive for native route-driving metrics.
5. Bench2Drive-Robust after source-protocol and rights assurance.
6. nuPlan-R / reciprocal actor scenarios, retaining its declared selected-reactive-agent scope.
7. NAVSIM v2 for its native EPDMS planning measure; it is not a substitute for full online visual interaction.
8. DrivingGen for world-model visual/temporal/trajectory evaluation; it is not a driving competence benchmark.
9. SAM recoverability, option-fragility, residual-decision-time and reciprocal option-externality experiments.

Never merge visual quality, prediction accuracy, route success, comfort, safety proxies, option preservation, recoverability, robustness and physical evidence into one score.

## 8. Metrics

- data and human-intervention samples required per skill gate;
- route completion, task success and route progress;
- simulator-defined contact/collision and infraction outcomes;
- explicit traffic-rule profile compliance;
- speed/lane/path tracking and comfort/jerk;
- prediction-set coverage and sharpness;
- actor track identity/continuity and calibrated uncertainty;
- action-conditioned prediction error and counterfactual sensitivity;
- option widths, policy-tree recoverability depth and option-fragility profile;
- decision-sensitive query rate, value and latency;
- fallback admissibility and residual decision time;
- held-out map/scenario/fault-family performance;
- p50/p95/p99 inference, planning, sensing and replay latency;
- memory, compute, energy and parameter count;
- denominator completeness, failures, censored/incomplete episodes and fresh-process replay.

## 9. Prospective objectives

These are hypotheses/targets, not results:
- exact agreement on finite analytic controls;
- zero hidden-state/teacher/evaluator leakage in frozen evaluation;
- complete episode and branch accounting;
- lower held-out intervention rate or higher route success at matched data/compute without worst-family constraint, fallback, calibration or deadline regression;
- a measurable HDC/LTC advantage on at least one preregistered metric (irregular-time robustness, memory, data efficiency, latency/energy or graceful missingness) versus compact recurrent and transformer baselines;
- transfer to sealed scenario families without holdout adaptation.

A negative result showing that a conventional recurrent/transformer model is better is scientifically useful and must be reported.

## 10. Claim ceiling

A passing virtual curriculum establishes competence only for the named simulator, sensor/control interface, vehicle model, scenario profile, training protocol and sealed evaluation split. It does not establish human-equivalent driving, human driver-training efficacy, sim-to-real transfer, real-world collision probability, roadworthiness, public-road authorization, certification or SAE Level 4/5 capability.

Physical transfer requires a separately approved progression through software-in-the-loop, hardware-in-the-loop and low-consequence closed-course testing with independent instrumentation and safety authority.

## References

- World-In-World, ICLR 2026: https://proceedings.iclr.cc/paper_files/paper/2026/hash/5b4263be85820683d78675cc18d2efc7-Abstract-Conference.html
- DriveVLA-W0, ICLR 2026: https://proceedings.iclr.cc/paper_files/paper/2026/hash/0d70423f59c5fdd24f0dd3fa52e34623-Abstract-Conference.html
- DriveWorld-VLA, ICML 2026: https://proceedings.mlr.press/v306/jia26f.html
- DrivingGen, ICLR 2026: https://proceedings.iclr.cc/paper_files/paper/2026/hash/a940e52cc37f63cd067c52265d1701cd-Abstract-Conference.html
- Human novice-driver simulator-training systematic review (Journal of Safety Research, 2024): https://doi.org/10.1016/j.jsr.2024.08.002
- Human-guided continual learning for self-driving (Nature Communications, 2026): https://www.nature.com/articles/s41467-026-77370-x
