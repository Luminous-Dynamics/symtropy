# Bidirectional Hand Task-Space Comparison v0.1

**Status:** M0 source register and M1 comparison protocol. No matched experiment, independent reproduction or physical qualification is claimed.

**Research owner:** [BIDIRECTIONAL-HAND-ENVELOPE-001 #6159](https://github.com/Luminous-Dynamics/symtropy/issues/6159)  
**Related J3 baseline:** [PACT finger qualification #6138](https://github.com/Luminous-Dynamics/symtropy/issues/6138)  
**Robust co-design:** [#6145](https://github.com/Luminous-Dynamics/symtropy/issues/6145)  
**Thermal mission envelope:** [#6155](https://github.com/Luminous-Dynamics/symtropy/issues/6155)  
**Shared evidence contract:** [#6140](https://github.com/Luminous-Dynamics/symtropy/issues/6140)

## 1. Research question

When does antagonistic bidirectional tendon actuation produce a material improvement in dexterous task capability over asymmetric/unilateral N+1 tendon actuation, and when is its additional actuator, tendon, preload, energy, sensing and lifecycle burden unjustified?

The comparison must not start from architecture labels. Both systems use tensile tendons: the relevant distinction is the set of reachable joint efforts and fingertip wrenches across the task workspace, including neutral crossing, palmar/dorsal access, and direction reversal.

## 2. Source anchors and evidence boundaries

The machine-readable record `source_comparison.json` transcribes two non-matched 2026 research sources.

### B1 — PACT N+1

Kumar et al., *PACT: Pose-Robust Actuation Co-Design for Tendon-Driven Robotic Hands*, 3 November 2026: https://arxiv.org/abs/2611.05302

PACT uses four pull-only tendons for flexion/extension and a separate bidirectional MCP abduction/adduction actuator. The paper reports a nominal eight-pose force-region coverage minimum of 0.88 in its model, with a 5 Hz minimum isolated flexion/extension bandwidth requirement. Its measured prototype behavior differs from predictions by up to 24.5%, largely attributed to transmission losses. The physical force-set check is one pose with selected feasible-set vertices; it does not constitute an eight-pose physical validation.

### B2 — BATM antagonistic hand

Lee et al., *Design and Validation of an Antagonistic Tendon-Driven Dexterous Robotic Hand with Bidirectional Operation*, 28 September 2026: https://arxiv.org/abs/2609.36241

The preprint describes antagonistic tendons wound in opposite directions around rotary joints. Each cable only pulls; the agonist/antagonist roles switch around the neutral position. It reports a 200 mm, 220 g, 17-actuated-DoF hand with 32 finger tendons and a distinct actuator module. Source-reported figures include 13.2 Hz average joint transmission bandwidth, 29 N maximum fingertip force, up to 0.15 mm repeatability after tendon pre-stretching, Kapandji 8, and completion of all 33 GRASP Taxonomy grasp types from palmar and dorsal sides. It also reports 5,400 cycles over three hours without observed failure.

These are source-reported results, not measurements independently replicated by Symtropy. The paper itself identifies quantitative comparison of bidirectional task-time and travel-distance benefits against a unidirectional baseline as future work. It also identifies lower-ratio QDD motors and tendon tension/slack sensing as potential improvements. This is a particularly useful open problem for our matched benchmark.

### Comparisons that must remain separate

Do not state that 29 N is superior to PACT's 0.88 coverage, or that 13.2 Hz directly beats 5 Hz:
- 29 N is a reported maximum at a particular fingertip force test; 0.88 is a modelled required-force-region coverage metric.
- 13.2 Hz is a reported average measured joint bandwidth; 5 Hz is a source-matched isolated F/E design constraint/prediction.
- PACT's physical force-set validation and BATM's maximum fingertip force use different task, direction, pose and measurement boundaries.
- PACT's finger mass and BATM's distal-hand mass plus separate actuator module are not the same system boundary.

The source register preserves these mismatches and unknowns.

## 3. Generalized tendon equilibrium

For a design x and joint pose q, retain tendon tensions as the physical control variables:

```text
tau_tendon(q) = R(q, x) T
T_min(q, state) <= T <= T_max(q, speed, temperature, service)
J(q)^T f = tau_tendon(q) + tau_other(q)
```

- `R(q,x)` is the signed tendon routing/moment-arm matrix.
- `T_min` is a declared physical lower tension/preload bound. It is not automatically zero; slack is a distinct state.
- `T_max` is constrained by motor torque-speed and thermal limits, tendon strength, capstan limits and service state.
- `J(q)` is the fingertip/task Jacobian.
- `tau_other` explicitly models separate bidirectional motors, passive springs or other non-tendon effort.

For an antagonistic pair, do not replace two nonnegative cable tensions with an unconstrained signed torque. Retain both tension bounds, routing geometry, elasticity, pretension and slack/hysteresis. A signed generalized torque interval may be a derived projection, but it is not a sufficient physical evidence record.

Power consistency must hold for the declared tendon-length convention:
`Tᵀ l_dot = tau_tendonᵀ q_dot` within the model's sign/frame convention and numerical tolerance. This is an analytic virtual-work identity, not proof of physical transmission efficiency.

## 4. Matched task protocol

Define one common task specification and execute both architectures under the same:
- object geometry and material;
- contact location, fingertip pad geometry and friction characterization;
- task goal and success/failure criteria;
- payload and external wrench profile;
- initial object/hand pose and allowed wrist pose;
- observation and controller information budget;
- energy boundary and installed-system accounting;
- thermal initial state, ambient/cooling boundary and mission history;
- repeat count and held-out task/pose split.

A separate native-capability report may show each architecture's best achievable envelope. It must sit beside, not replace, the matched-envelope comparison.

### Task families

1. **Palmar grasp:** pinch, tripod, power grasp, precision placement, release/regrasp.
2. **Dorsal grasp/access:** dorsal pinch/grasp, object engagement from the opposite side, manipulation without wrist reorientation where task geometry permits.
3. **Neutral-crossing:** flexion/extension through zero, bidirectional motion reversal, approach/retraction, AbAd through neutral.
4. **Reorientation:** object roll/pitch/yaw, in-hand repositioning and recovery from slip.
5. **Compliant interaction:** alignment/insertion under controlled misalignment and external wrench.
6. **Disturbance recovery:** externally applied impulse/disturbance with recovery time, object retention and force peaks.
7. **Held-out transfer:** unseen object dimensions, contact locations, poses, payloads and direction combinations.

For every task, publish the task definition and all failure modes. Grasp taxonomy count is a useful task-completion indicator but is not a replacement for measured force coverage, stability, repeatability or success probability.

## 5. Metrics — report separately, no aggregate winner

### Workspace and task performance
- Palmar and dorsal reachable task-space sets.
- Neutral-crossing range and singularity/conditioning across the required workspace.
- Completed task fraction, time-to-completion, path/travel distance and number of wrist/arm reorientations.
- Retry, intervention, drop/slip, timeout and failure rates.
- Held-out success with uncertainty intervals and full denominators.

### Force and impedance
- Full feasible fingertip wrench region by pose and direction.
- Minimum pose/direction coverage against the same required task regions.
- Directional force margins reported separately from region coverage.
- Antagonistic stiffness/preload range, hysteresis and cross-axis coupling.
- Backdrive torque/impedance and force-release behavior.

### Transmission and control
- Motor-to-joint position and velocity transmission error.
- Bandwidth by joint, direction, amplitude and load; flexion and extension separately.
- Tendon tension and elongation, slack onset, pretension drift, hysteresis and creep.
- Commanded vs independently measured output and control saturation.

### Energy and thermal
- Battery/DC-bus electrical energy per successful task, with controller and remote actuator module included.
- Mechanical output work and negative mechanical work separately; do not equate negative mechanical work with recovered electrical energy.
- Peak/continuous torque-speed envelope, thermal derating and hot-start/partial-cooldown performance.
- Antagonistic preload/holding energy and its impact on energy, thermal load and tendon wear.

### Mass, integration and lifecycle
- Distal moving mass, complete installed mass and volume, actuator-module mass, wiring, driver and cooling boundary.
- Tendon, guide, pulley, encoder and force-sensor count.
- Assembly and calibration time, pretension procedure, service accessibility, repair time and requalification.
- Wear, fray, termination failure, capstan/pulley degradation and performance after remount/pretension reset.

## 6. Experimental design and evidence tiers

### M0 — source traceability
- Freeze source files, exact source revisions and extracted tables.
- Record units, coordinate frames, tendon length sign, force sensor location, test conditions and system boundaries.
- Keep undisclosed geometry, mass, torque, preload, energy and lifecycle fields unknown.

### M1 — analytic correctness
- Generalize the J3 force-feasibility oracle to arbitrary signed routing matrices and per-tendon lower/upper tension bounds.
- Include unilateral, antagonistic and hybrid architecture known-answer fixtures.
- Require independent force-witness reconstruction and exact actuator/tendon-bound checks.
- Negative controls: slack cable, impossible negative tension, insufficient pretension, wrong routing sign, singular routing matrix, infeasible force direction, unit/frame mismatch and non-finite values.
- Verify virtual-work equality and dimensionally correct power.

### M2 — numerical reproduction
- Independently reconstruct source kinematics, force sets and task metrics.
- Compare candidate solvers with convergence studies and matched initial conditions.
- Keep modelled force-region coverage separate from physical source measurements.
- Freeze design, validation and held-out splits; no tuning on held-out outcomes.

### M3 — instrumented component
Use a guarded non-human-contact setup with:
- calibrated six-axis fingertip wrench reference;
- per-tendon tension and elongation;
- joint-angle encoders and independent kinematic reference;
- synchronized motor current/voltage, output speed and temperature;
- configuration/article identity, calibration and raw traces.

Measure both directions, all relevant poses, neutral-crossing and repeated cycles. Include prestretched and non-prestretched conditions as separate strata. Model-derived force or motor-current inference is not an independent tension measurement.

### M4 — integrated manipulation
Run the matched common tasks on multiple articles and held-out objects, poses and payloads. Preserve all retries, drops, derating, intervention and censored runs. Report both task success and measured physical margins.

### M5 — lifecycle
Repeat after tendon wear, creep, remounting, rerouting, pretension reset and repair. Report calibration invalidation and requalification. A newly calibrated hand does not represent an aged or serviced hand.

## 7. Prospective benchmark targets

These are targets, not achieved results:

- Reproduce B1's source-defined 0.88 nominal model coverage and 5 Hz isolated flexion/extension condition under its own original definitions.
- Reproduce B2's source-reported 33/33 GRASP task set and neutral-crossing range under its own protocol before claiming source reproduction.
- Produce a common, matched palmar/dorsal task suite with per-task completion time, travel distance, reorientation count, success, force margin and energy.
- Establish whether antagonistic actuation reduces wrist/arm reorientation and total task travel/time; this is the most direct missing comparison identified in B2.
- Quantify whether antagonistic stiffness and neutral crossing increase full task-space force capability enough to justify preload power, actuator count, added mass and calibration.
- Target zero false-feasible analytic witnesses and independent reconstruction of accepted feasible-force witnesses.
- Compare energy per successful task and lower-tail task completion under held-out payloads and poses, not only average success.
- Set statistical thresholds and lifecycle acceptance only after instrument repeatability, sample size, uncertainty and application risk requirements are preregistered.

## 8. Claim ceiling

This is a source register and experiment protocol only. It is not a matched study, independent reproduction, hardware qualification, prosthetic or rehabilitation validation, or safety case. No fabrication, energization, human-contact testing or clinical use is authorized by this document.
