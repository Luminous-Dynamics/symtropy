# Task-conditioned synthetic-joint co-design: comparator register v0.1

**Date:** 18 November 2026  
**Owner:** [JOINT-TASK-CODESIGN-001 #6064](https://github.com/Luminous-Dynamics/symtropy/issues/6064)  
**Parent:** [SYNTH-JOINT-001 #6037](https://github.com/Luminous-Dynamics/symtropy/issues/6037)  
**Status:** M0 source/metric register; no optimizer or physical replication claimed.

## Why this register exists

Current actuator figures often use the same unit (N·m/kg) but refer to different quantities:

- continuous rated output torque divided by assembled actuator mass;
- momentary peak output torque divided by motor or module mass;
- output torque from an individual joint module rather than the whole installed subsystem;
- predicted performance rather than a measured prototype;
- device performance under a jump, walking, wrist or exoskeleton test with different task and thermal boundaries.

These must not be treated as one leaderboard. This register establishes separate J1/J2/J3 comparisons and an industry-context row. All numeric values below are source-reported, not Luminous results.

## B1 — J1 geared leg actuator / task co-design

**Source:** Huang, Dong & Zhao, *Task-Oriented Co-Design and Optimization of Geared Actuators for Robotic Applications*, 19 September 2026, [arXiv:2609.22795](https://arxiv.org/abs/2609.22795).

| Metric | Commercial DM8009 comparator | Fabricated 18S/20P prototype |
|---|---:|---:|
| Measured/estimated assembled mass | 0.896 kg (paper table) | 0.729 kg (prototype measurement) |
| Rated output torque | 20.0 N·m | 26.0 N·m nominal test setting (26 N·m) |
| Rated output torque density | 22.3 N·m/kg | 35.7 N·m/kg using measured prototype mass |
| Peak torque | 40.0 N·m | 51.5 N·m (paper table) |
| Maximum speed | 320 rpm | 250 rpm |

**Physical task comparison:** two-DOF vertical-guide jumping-leg bench; 5.5 kg common bench load; 48 V DC bus. The study reports slider rise at matched normalized command levels:

| Normalized command | DM8009 torque / rise | Optimized torque / rise |
|---|---:|---:|
| 0.75× rated | 15.0 N·m / no jump | 19.5 N·m / 0.490 m |
| 1.00× rated | 20.0 N·m / 0.490 m | 26.0 N·m / 0.580 m |
| 1.50× rated | 30.0 N·m / 0.615 m | 39.0 N·m / 0.745 m |
| 2.00× rated | 40.0 N·m / 0.750 m | 52.0 N·m / 0.840 m |

At twice-rated command the reported measured rise increases 12.0%; at rated command it increases 18.4%. This is an experimentally validated reduced-order jump bench, **not** whole-humanoid locomotion or an independently verified commercial deployment.

The study uses simulated Unitree G1 task trajectories (fast walking, jumping and stair ascent) to construct task-specific torque-speed requirement envelopes, then optimizes a two-DOF vertical-guide model. It explicitly omits whole-body balance and lateral dynamics. At twice-rated command, the optimized design-stage model predicts 1.008 m while the prototype measures 0.840 m: a 16.6% overprediction relative to the model prediction. Importantly, this is not a uniform push-off model error: the model predicts a shorter push-off phase (0.194 s versus 0.240 s measured) and less push-off rise (0.287 m versus 0.343 m), then overpredicts total post-release rise. Replacing model motor parameters and actuator mass explains 11.3 percentage points of the total discrepancy; 5.3 percentage points remain for transmission, guide and control effects. The paper's 2.0× nominal command is 52 N·m, slightly above the 51.5 N·m tabulated peak rating; the reported measured hip/knee torque peaks are 42.96/49.28 N·m. Keep command, tabulated rating and measured joint torque as separate quantities.

**Fair comparison contract for our J1 reproduction:**
- Preserve the exact test mass/load, 48 V bus, rated command normalization, actuator boundary, gear/output shaft and slider-rise definition.
- Report measured-mass density and model-estimated-mass density separately.
- Reproduce torque-speed and thermal/current assumptions; do not treat peak torque as sustainable output.
- Report all four torque points, including no-jump/failed points. Do not report only the best condition.
- Keep simulation, paper-reported physical measurement and our reproduction as three distinct evidence classes.
- Reproduce model prediction error as part of the benchmark, not just the task score.

## B2 — J2 compact pitch-yaw wrist with embedded compliance

**Source:** Tsai et al., *A compact wrist mechanism with integrated compliant transmission for high-torque-density and variable-stiffness actuation*, Mechanism and Machine Theory 229 (November 2026), [DOI 10.1016/j.mechmachtheory.2026.106577](https://doi.org/10.1016/j.mechmachtheory.2026.106577).

| Metric | Paper-reported value |
|---|---:|
| Total mass | 0.59 kg |
| External dimensions | 59 × 83 × 102 mm |
| Reported volume | 499.5 cm³ |
| Continuous pitch torque | 8.10 N·m |
| Continuous yaw torque | 8.30 N·m |
| Reported torque density | 14 N·m/kg |
| Control / logging rate | 1 kHz real-time loop; data logged at 1 ms intervals |

**Architecture:** serial pitch-yaw joint with harmonic drives and integrated compliant transmission elements. A dual-encoder scheme and transmission-error compensation estimate torque from compliant deformation. The paper evaluates torque estimation, active backdrivability, variable stiffness and position control.

**Fair comparison contract for our J2 reproduction:**
- Keep pitch and yaw output torque separate; do not collapse the two axes into one scalar.
- Verify whether the 14 N·m/kg denominator uses the complete wrist mass and how continuous torque is defined.
- Match external dimensions / volume, range of motion, load orientation, duty and the control/measurement rate.
- Compare dual-encoder torque inference to an independent output torque transducer; report bias, uncertainty coverage, hysteresis and transmission-error calibration.
- Report achievable stiffness range, modulation rate, hold power, backdrive torque and tracking performance, not merely the density.
- J2 is not directly rankable against the J1 jumping actuator.

## B3 — industry context: Unitree H1 manufacturer specification

**Source:** [Unitree H1 product page](https://www.unitree.com/h1/), manufacturer-listed H1 specification.

The current product page lists **189 N·m/kg peak torque density** and **360 N·m maximum joint torque**.

This is a manufacturer-reported peak specification. The page's displayed figure is not, by itself, a matched continuous rated torque density measurement against B1 or B2. Until the motor/joint boundary, mass denominator, peak duration, thermal starting state, bus voltage and test procedure are available, this value must be labelled vendor peak/context only and excluded from numeric superiority claims.

## B4 — J3 tendon-coupled finger/gripper

No single 2026 paper in this register provides a protocol-matched, complete-system torque-density baseline for a tendon-coupled finger group with force estimation, slip recovery, serviceability and sensor/communication accounting.

The first J3 cohort should therefore establish:
- 2–4 coupled joints, tendon routing geometry and moment arms;
- actuator, spool, tendon, pulleys, sensors, end-effector and remote power/drive mass;
- matched objects and independent grasp/contact ground truth;
- tendon slack, creep, friction, preload, breakage and remount cases;
- slip detection latency, contact localization, grasp retention, object damage, recovery and accepted task outcomes;
- external transducer truth versus tendon-derived estimate, with calibration uncertainty;
- full source energy, motor/spool work, tendon elastic storage and auxiliary sensing/compute energy.

J3's first competitive metrics should be useful manipulation, sensing economy and graceful recovery, not an arbitrary N·m/kg number.

## Cross-family scoreboard — keep dimensions separate

| Metric | J1 leg | J2 wrist | J3 finger |
|---|---|---|---|
| Rated torque density | Required | Required per axis and defined package | Secondary |
| Peak torque density | Separate transient row | Separate transient row | Secondary |
| Continuous duty / thermal | Required | Required | Required |
| Torque-speed envelope | Required | Required per axis | Required per tendon/joint group |
| Backdrive / reflected inertia | Required | Required | Required |
| Stiffness / damping matrix | If relevant | Core metric | Core metric |
| Task acceptance / failure | Jump height, contact and failed trials | Tracking, wrench rejection, insertion | Grasp retention, slip, damage and recovery |
| Energy | Full electrical source to output | Full electrical source to output | Full group and auxiliary sensing energy |
| Lifecycle | Gear/fatigue/repair | Compliant transmission drift/repair | Tendon creep/fatigue/remount/repair |

## Design targets, conditional on reproduction

- **J1:** use 35.7 N·m/kg measured rated output density as the initial reproduction threshold only if our assembly boundary, rated current density and thermal duty match. A 20% improvement above that is a stretch goal; task rise, energy and lifetime remain separate objectives.
- **J2:** use 14 N·m/kg as the paper-reported comparison point. A 16.8 N·m/kg stretch goal is meaningful only at matched continuous pitch/yaw output, complete wrist mass and envelope, while preserving or improving workspace, torque-estimation uncertainty and stiffness control.
- **J3:** pursue better task outcome and force-state observability with lower full-system sensing/communications cost; set numeric thresholds after baseline repeatability and independent measurement variance are known.

## Source and evidence rules

- Paper-reported values are not reproductions.
- Vendor-reported peak figures are not rated continuous measurements.
- Simulation task trajectories are not physical task outcomes.
- Unknown denominator or missing test conditions means NOT_COMPARABLE, not a guessed normalization.
- All failed trials and exclusions remain in denominators.
- The complete installed boundary includes remote actuation, cables/tendons, drivers, pumps/valves, reservoirs, sensors, thermal management and harnessing.
- A 20% target remains a hypothesis until a matched independent experiment demonstrates it.


## B5 — J2 alternative: parallel origami-compliant wrist

**Source:** Hu et al., *Bioinspired wrist joint with variable stiffness using a parallel mechanism and origami coupling*, Mechanism and Machine Theory 228, article 106545 (15 October 2026), [DOI 10.1016/j.mechmachtheory.2026.106545](https://doi.org/10.1016/j.mechmachtheory.2026.106545).

**Architecture reference:** a 3-PRS/SS parallel mechanism is coupled to a Yoshimura origami structure and an antagonistic tendon-pneumatic actuation arrangement. The paper reports experimental evaluation of motion flexibility and variable-stiffness behavior. Numeric stiffness, force, mass, range and lifecycle values are not admitted here because the accessible source material does not establish a complete matched metric set.

**Why it matters:** this is a genuinely different J2 topology from the serial pitch-yaw Tsai wrist. The comparison should test parallel versus serial kinematics, stiffness-matrix conditioning, parasitic motion, workspace, mode-switching effort and the installed mass/energy of pneumatic hardware. It is a qualitative architecture comparator, not a numeric torque-density rival.

## B6 — tendon as both transmission and proprioceptive channel

**Source:** *Worm-inspired proprioceptive continuum robot with retractile motion and sensorized tendons*, Mechatronics 118 (September 2026), article 103524, [DOI 10.1016/j.mechatronics.2026.103524](https://doi.org/10.1016/j.mechatronics.2026.103524).

**Reported reference facts:** two spring-based sections, each driven by three tendons; a nichrome–nylon braid (four 0.10 mm nylon filaments and one 0.08 mm nichrome wire, approximately 0.55 mm assembled diameter) provides tendon-length sensing. The paper reports tendon-length calibration RMSE of 1.19 mm over a 40–80 mm range and nominal task-space tracking RMSE of 2.5 / 3.0 / 1.8 mm on X/Y/Z.

**Important limitation:** the evaluated robot also uses tendon tension transducers, motor encoders and base/tip IMUs. Its result does not show that tendon resistance can replace all external sensors, nor does it qualify a high-load musculoskeletal joint. It is an unusually relevant J3/J2 sensing architecture and a direct source for a sensor ablation experiment.

**Luminous experiment:** compare (i) tendon resistance/length alone, (ii) tendon signal plus encoders, (iii) tendon signal plus independent tension measurement, and (iv) the complete reference sensor suite. Hold physical article, load, calibration budget and held-out trajectories fixed. Score state/tension estimation error, uncertainty coverage, fault identifiability, energy, wiring mass, calibration drift and degraded-mode behavior.

## B7 — parametric TPU musculoskeletal structures

**Source:** Yoshimura et al., *Accelerating Musculoskeletal Robotics Through Parametric Design and 3D-Printed Flexible Structures*, Advanced Intelligent Systems (2026), [DOI 10.1002/aisy.70537](https://doi.org/10.1002/aisy.70537).

**Architecture reference:** one parameterized FDM/TPU workflow generates muscle-, tendon-, ligament- and cushioning-like structures by varying pattern geometry. The reported life-scale leg demonstrates coordinated multijoint motion under external suspension and partial body-weight support; torso roll, pitch and yaw are constrained to ±20°.

**Comparison boundary:** this supports design-for-manufacture, patterned compliance and modular structure exploration. External suspension and partial body-weight support are essential qualifications; it is not evidence of unsupported full-body locomotion, a qualified load-bearing knee, or long-term durability.

**Luminous experiment:** compare solid TPU, patterned TPU, cable/tendon, and rigid-link baselines under identical coupon and joint fixture boundary conditions. Measure force–displacement loops, anisotropy, hysteresis, rate/temperature dependence, creep, stress relaxation, fatigue, repairability, dimensional repeatability and print-to-print variation. Only promote a material model into a joint model after coupon-level uncertainty and fatigue envelopes exist.

## B8 — self-healing dielectric-elastomer artificial muscle material

**Source:** Mao et al., *A self healable dielectric elastomer artificial muscle*, Nature Communications 17, 5451 (15 May 2026), [DOI 10.1038/s41467-026-72611-5](https://doi.org/10.1038/s41467-026-72611-5).

**Reported material/actuator results:** a bimodal-network zwitterionic dielectric elastomer reports 125% stable strain at 25 MV/m over 150,000 cycles and self-healing following mechanical or electrical damage. The paper also demonstrates artificial-arm and soft-gripper configurations. The reported healing process uses 60 °C and reaches approximately 90% mechanical-property recovery after 60 hours in the described film test.

**Boundary and safety:** these are material and laboratory actuator results, not a qualified load-bearing joint. The field strength, electrode design, dielectric breakdown, leakage current, thermal behavior, encapsulation and repair process are all first-class hazards and engineering constraints. No high-voltage drive, fabrication, or human-contact work is authorized by this register.

**Luminous research hypothesis:** lifecycle value may improve more through damage detection + self-healing + post-heal requalification than through peak strain. A meaningful test must compare pristine, damaged, healed and repeatedly healed specimens; record output force/stroke, leakage, breakdown margin, temperature, hysteresis and cycle-to-failure. The post-heal article must be requalified rather than assumed restored.

## New cross-architecture opportunities

1. **Self-sensing load paths:** a tendon, flexure or compliant gear should be evaluated as both a force path and a sensor. Ablate external sensors one at a time and retain an independent truth instrument.
2. **Mechanical preprocessing:** geometry and compliance can shape contact signals before software inference. Credit only the incremental task improvement in a matched ablation, after reflected inertia, hysteresis and energy costs are charged.
3. **Service-equivalent lifetime:** score accepted task cycles between interventions, repair time, component replacement mass/cost, post-repair calibration and requalification—not raw cycles to failure alone.
4. **Damage-tolerant actuation:** compare replaceable cartridges, passive fail-safe constraints and self-healing material paths under the same fault and post-fault task. Healing is not equivalent to safe restoration.
5. **Manufacturing-aware co-design:** optimize topology together with print orientation, wall/lattice parameters, tolerances, inspection access and part-to-part variance. A theoretically superior geometry that cannot be repeatedly manufactured is not a superior design.



## B9 — industry context: Unitree GO-M8010-6 joint motor

**Source:** [Unitree GO-M8010-6 manufacturer motor page](https://unitree-robot.com/go1/motor.html), captured 18 November 2026.

**Manufacturer-listed values:** approximately 0.530 kg, 23.7 N·m maximum torque, 1:6.33 reduction ratio, maximum rotational speed 30 rad/s at 24 V DC, 40 A maximum current, 0.63895 N·m/A torque constant, 6 kHz communication/control frequency and 15-bit motor encoder. Listed feedback includes torque, angle, angular velocity and temperature; command interface includes torque, angle, angular velocity, stiffness and damping.

The arithmetic ratio 23.7/0.530 = **44.72 N·m/kg** is a Luminous calculation from approximate listed mass and manufacturer maximum torque. It is not a manufacturer-rated continuous torque density. The exact speed/output boundary, maximum-torque duration, complete installed-mass boundary and thermal duty must be confirmed. Retain it as a vendor maximum/context row, not as a rated-output winner over J1.

## B10 — low-voltage self-healing dielectric-elastomer material

**Source:** Lai et al., *A Skin-Inspired High-κ Self-Healing Polymer for Low-Voltage Dielectric Elastomer Actuators*, Advanced Materials 38(56), e23659 (October 2026), [DOI 10.1002/adma.202523659](https://doi.org/10.1002/adma.202523659); PubMed record: https://pubmed.ncbi.nlm.nih.gov/42569803/.

**Abstract-reported values:** a skin-inspired high-permittivity self-healing polymer PABTU has dielectric constant 15 at 1 kHz. The ULTRA bilayer actuator increases reported breakdown strength from approximately 33 to 43 V/µm, shows visible deformation at 30 V, and exceeds 130% areal strain at 120 V. The article reports thin films around 3 µm.

**Boundary:** this is a material/DEA actuator reference, not a load-bearing synthetic joint. The abstract does not provide a directly comparable cycle-life or healed-property-recovery value. Extract those from the complete article before lifecycle comparison. Do not conflate applied voltage, electric field, thickness, area strain, blocked force and useful joint work.

**Research opportunity:** compare high-voltage, high-strain DEA against low-voltage high-κ DEA under matched actuator output work, cycle rate, load, film thickness, encapsulation, leakage, temperature and damaged/healed states. A lower applied voltage matters only if the complete system also improves efficiency, lifetime, packaging and fault containment.



## B11 — source-method detail for the J1 actuator co-design reference

Direct review of the complete arXiv HTML for Huang, Dong & Zhao (arXiv:2609.22795v1) establishes the following source details, now carried into the JSON registry:

- Seven slot–pole families: 18S/16P, 18S/20P, 18S/22P, 24S/22P, 24S/26P, 24S/28P and 27S/30P.
- Eight cross-sectional geometry variables; motor length and winding choices are applied subsequently through analytical scaling.
- Seven separate motor surrogates, each using 1,000 valid Latin-hypercube designs, standardized quadratic ridge regression and 200 independent validation cases. The reported R² interval 0.9681–0.9993 is for representative 18S/20P outputs only.
- Fundamental winding factor constraint k_w,1 > 0.9; package bounds D_act ≤98 mm and L_act ≤50 mm.
- Constrained NSGA-II with population 48, 30 generations and 1,488 evaluations per enumerated case; inner bounded L-BFGS-B optimizes twelve fifth-order Bézier torque-profile coefficients.
- Simulated Unitree G1 task input peaks: fast walking 103 N·m / 16.6 rad/s; jumping 139 N·m / 20.1 rad/s; stair ascent 109 N·m / 14.3 rad/s. These are MuJoCo task-profile requirements, not physical G1 measurements.
- The reduced vertical-guide model constrains the foot, retains hip/knee push-off and contact, and excludes whole-body balance and lateral dynamics.

This corrects the earlier six-family transcription. It also clarifies the comparison ceiling: a source-faithful variable and optimizer description is not an independent reproduction of the motor electromagnetic surrogates, reducer tooth-strength sizing, Pareto search or physical jump experiment.


## B12 — whole-robot energy reference: PACE sim-to-real and actuator-energy pipeline

**Source:** Bjelonic, Tischhauser & Hutter, *Towards bridging the gap: Systematic sim-to-real transfer for diverse legged robots*, The International Journal of Robotics Research, first published 31 July 2026, [DOI 10.1177/02783649261459628](https://doi.org/10.1177/02783649261459628); arXiv:2509.06342v2.

**Reported system-level findings:** bottom-up identification spanning actuator drives, fixed-base/in-air trajectories and on-ground locomotion; three primary platforms (ANYmal, Tytan and Minimal) plus ten additional robots; no dynamic-parameter randomization for the reported zero-shot transfer; a reported 32% reduction in ANYmal's full cost of transport, with the paper's reported ANYmal full COT value 1.27.

**Why this matters:** this is a strong transfer and energy-model comparator for the joint portfolio. It links actuator electrical/mechanical loss representation to full-robot energy, not just torque density. The publicly released <Link url="https://github.com/leggedrobotics/pace-sim2real" title="PACE code"/> and [PACE dataset](https://doi.org/10.3929/ethz-c-000783505) provide a reproducibility target; the dataset is CC BY 4.0 and contains actuator, single-joint, single-leg and full-robot data. This is a whole-system/context row, not an actuator torque-density comparison.

## B13 — whole-robot endurance reference: RAIBO2 marathon

**Source:** Lee et al., *A quadruped robot designed to complete a marathon on a single battery charge*, Nature (23 September 2026), [DOI 10.1038/s41586-026-11102-5](https://doi.org/10.1038/s41586-026-11102-5).

**Reported system-level result:** the RAIBO2 quadruped completed a marathon in 4 h 19 min 52 s on one battery charge and reported total cost of transport 0.25, compared in the paper with a human benchmark of 0.37. The paper attributes the result to integrated hardware, low-resistance motor-driving circuitry and locomotion-policy improvements.

**Comparison boundary:** whole-robot battery-level/system outcome; not a joint-level actuator result and not a directly comparable COT result to ANYmal or the J1 vertical-guide bench without matching COT conventions, robot mass, payload, terrain, speed and electrical-energy boundaries. Treat as an aspirational system benchmark.

## B14 — high-reduction friction/transfer comparator

**Source:** Kang et al., *Enhancing Sim-to-Real Transfer for a High-Gear-Ratio Quadruped Robot via Extended Actuator Dynamics Identification*, Machines 14(9), 1031 (9 September 2026), [DOI 10.3390/machines14091031](https://doi.org/10.3390/machines14091031).

**Reported result:** on a 55 kg quadruped using high-reduction (>40:1) actuators, adding Stribeck friction to a PACE-style Coulomb–Viscous identification model lowered identification error by 16% relative to the Coulomb–Viscous model on the identification trajectory. The benefit generalized to an unseen trajectory. Reported simulation-to-real mean-velocity degradation changed from 40.2% to 27.2% at 0.3 m/s commands and from 26.8% to 21.6% at 1.0 m/s commands.

**Why it matters:** this gives #4206 a recent high-ratio transfer benchmark. It supports explicit low-speed stiction/Stribeck modeling, but does not prove that every high-ratio actuator needs those same parameters or that the reported improvements transfer to another platform. Match excitation, trajectory, mass, gear ratio, actuator identity and velocity-degradation calculation before claiming reproduction.

