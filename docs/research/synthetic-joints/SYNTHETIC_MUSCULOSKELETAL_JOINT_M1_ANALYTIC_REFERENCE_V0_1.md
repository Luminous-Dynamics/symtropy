# Synthetic musculoskeletal joints: M1 analytic reference set v0.1

**Date:** 18 November 2026  
**Workstream:** [SYNTH-JOINT-001 #6037](https://github.com/Luminous-Dynamics/symtropy/issues/6037)  
**Evidence level:** M1 analytic known-answer fixtures; not a simulation result, hardware measurement, or qualification. Corpus version 0.1.2 contains twelve cases.

## Purpose

This first increment makes the joint programme executable without waiting for CI runners or physical hardware. It freezes small independent analytic cases that later engine adapters and candidate architectures must reproduce.

The intended paradigm is **joint-as-integrated-organ**, not “motor + hinge”: a complete module co-designs the load path, kinematics, actuator, transmission/tendon, compliant energy storage, sensing, local response and service interface. Biology supplies useful functions—antagonistic actuation, elastic energy storage, distributed proprioception, structural constraints and local reflexes—not a requirement to copy biological shape.

This fixture set is deliberately small. Every number is synthetic and dimensioned in SI units. These fixtures establish equation-level semantics only; they do not imply that the current Symtropy engine implements the corresponding physical model.

## Current source audit that constrains M1

The following files were fetched at exact repository commit `96496dac05833b9bd1d53d4889cbf646c3a594ae` (18 November 2026):

| Source | Blob SHA | M1-relevant observation |
|---|---|---|
| [`joints/hinge.rs`](https://github.com/Luminous-Dynamics/symtropy/blob/96496dac05833b9bd1d53d4889cbf646c3a594ae/crates/core/symtropy-physics/src/joints/hinge.rs) | `6e77cb284b092d955143faa7f2aed8898387d01c` | `HingeJoint` stores optional angle limits, but its shown solve paths do not enforce them. The motor path is velocity-only; it computes torque from target/current angular velocity. The applied impulse is split 50/50 between dynamic endpoints rather than being derived from their effective inverse inertia. |
| [`joints/mod.rs`](https://github.com/Luminous-Dynamics/symtropy/blob/96496dac05833b9bd1d53d4889cbf646c3a594ae/crates/core/symtropy-physics/src/joints/mod.rs) | `93be02d7d87b20294a4619db7cd015b0fa6d967e` | Shared `MotorDrive` exposes position and velocity targets; its generic `calculate_impulse` method takes current position and velocity. This API-level promise must not be confused with hinge runtime support. |
| [`articulation.rs`](https://github.com/Luminous-Dynamics/symtropy/blob/96496dac05833b9bd1d53d4889cbf646c3a594ae/crates/core/symtropy-physics/src/articulation.rs) | `7925bc6ec7fe619bc161ef476675d7d42b982e27` | `ChainBuilder` maps `LinkSpec.angle_limits` into the hinge object and provides a simple serial chain; the builder does not itself establish that the solver enforces those limits. |

These observations are consistent with the dedicated solver owner [#4516](https://github.com/Luminous-Dynamics/symtropy). They are a reason to keep solver semantics and architecture scoring separate. They do **not** establish that every other joint type is defective or that physical hardware is unsafe.

The core currently exports ball, fixed, hinge and prismatic joint types. That is a useful rigid-body base, but it is not yet a full synthetic-musculoskeletal joint model: it does not by itself represent antagonist tendon states, variable stiffness, compliant transmission, local tendon proprioception, soft tissue hysteresis, or serviceable multi-layer joint cartridges.

## Independent analytic fixtures

The machine-readable corpus is [`synthetic_joint_analytic_fixtures_v0_1.json`](./synthetic_joint_analytic_fixtures_v0_1.json). Independent Rust equation oracles are in [`synthetic_joint_analytic_oracle.rs`](../../../crates/core/symtropy-physics/tests/synthetic_joint_analytic_oracle.rs).

1. **Torsional spring:** `tau = -k theta`, `U = 0.5 k theta^2`, and actuator power to quasistatically increase spring displacement is `k theta omega`. This separates elastic torque, stored energy and work.
2. **Antagonistic tendons:** with equal constant moment arms, net torque is `r(T_f - T_e)`; for two equal linear tendon stiffnesses, local joint stiffness is `2 k_t r^2`. Pretension can change co-contraction/stiffness without changing ideal net torque.
3. **Sensorized tendon:** `delta_l = T/k_t`; strain/elongation is a physical signal with hysteresis, temperature and creep omitted in this analytic fixture. It is not automatically an independent torque measurement.
4. **Gear-reflected rotor inertia:** `J_ref = N^2 J_m`; ideal output torque and speed scale inversely with the gear ratio while power is conserved. The full installed joint still has friction, gear compliance, motor copper/iron loss and drive losses.
5. **Signed joint work:** `P = tau omega`, `W = integral(P dt)`. Negative mechanical work is not proof of electrical regeneration; regeneration must be measured at the electrical boundary.
6. **Coupled matrix stiffness:** for `tau = K q`, `U = 0.5 q^T K q`; symmetric positive-definite K is a fixture precondition. Off-diagonal terms are real cross-axis coupling, not a display artifact.
7. **Unilateral tendon slack:** extension below slack length produces zero tension and zero elastic energy; after engagement, force and stored energy follow the linear tendon law.
8. **Nonlinear strain-stiffening spring:** a cubic conservative torque law separates restoring torque, stored energy and incremental stiffness.
9. **Transmission backlash:** a symmetric deadband creates zero transmitted torque before engagement; both positive and negative engagement directions are checked.
10. **Coulomb/viscous friction:** moving friction must dissipate mechanical energy, while zero-speed stiction is set-valued and cannot be represented by `sign(0)=0`.
11. **Corrected compliance-deflection uncertainty:** propagates effective stiffness and already corrected deflection uncertainty under an explicit independent-input assumption.\n12. **Dual-encoder differential:** computes motor-angle/gear-ratio minus output-encoder angle, applies calibrated zero offset, estimates torque and propagates motor/output encoder, zero-offset and stiffness uncertainty.

### Executable oracle scope

The Rust integration test now contains twelve independently written known-answer calculations. It is intentionally not coupled to the Symtropy hinge solver, an HLB policy, or a simulator adapter. The checked-in JSON was separately parsed and its twelve expected outputs were recalculated from the stated equations; all twelve agree to a 1e-12 absolute/relative tolerance. The two-axis fixture is `tau = [0.2, -1.7] N m`, `U = 0.18 J`, `det(K) = 35`, and positive definite. The six added controls cover tendon slack, cubic strain-stiffening, backlash in both directions, dissipative friction plus set-valued static friction, corrected-deflection uncertainty, and explicit dual-encoder differential/torque uncertainty.

This source-level validation is not a Rust test execution. The CI workflow for the current head is queued; no CI pass is claimed.

Every fixture includes assumptions and a claim ceiling. Future tests should reject invalid units, non-finite values, negative stiffness/inertia, non-positive gear ratios, mismatched dimensions and unsupported model modes rather than quietly repairing the inputs.

## November 2026 industry and research baseline update

The current evidence suggests three different leaderboards. Do not combine their numbers.

| Reference | Reported boundary | Correct use in our programme |
|---|---|---|
| Huang, Dong & Zhao, *Task-Oriented Co-Design and Optimization of Geared Actuators for Robotic Applications* (19 Sep 2026), [arXiv:2609.22795](https://arxiv.org/abs/2609.22795) | Fabricated 18S/20P actuator: 0.729 kg, 26 N·m nominal rated output in the bench campaign, 35.7 N·m/kg measured-mass rated density. DM8009 comparison: 0.896 kg, 20 N·m rated output, 22.3 N·m/kg. On a common 5.5 kg load / 48 V two-DOF vertical-guide jump bench, the prototype produced 0.840 m vs 0.750 m slider rise at twice-rated torque (12.0% higher). | Strong J1 / leg actuator comparator. It is a reduced-order jumping-leg bench, not whole-humanoid validation. The paper reports 16.6% prediction-to-experiment error in the twice-rated rise model, so use measured results for the physical comparator and retain the model residual. |
| Tsai et al., *A compact wrist mechanism with integrated compliant transmission for high-torque-density and variable-stiffness actuation* (Nov 2026), [DOI 10.1016/j.mechmachtheory.2026.106577](https://doi.org/10.1016/j.mechmachtheory.2026.106577) | 0.59 kg pitch-yaw wrist, 59 × 83 × 102 mm; 8.10 / 8.30 N·m continuous pitch/yaw torque; 14 N·m/kg reported torque density; embedded compliant transmission and dual-encoder torque estimation with transmission-error compensation; variable stiffness/backdrivability tests and 1 kHz real-time control loop. | Strong J2 benchmark. Reproduce its exact torque-density denominator, continuous-duty condition and package boundary before comparing a candidate. |
| [Unitree H1 manufacturer specification](https://www.unitree.com/h1/) | 189 N·m/kg **peak** torque density and 360 N·m maximum joint torque as manufacturer-listed values. | Industry context only. Vendor peak density is not directly comparable with the rated output density of B1 or the 2-DoF wrist result. Obtain actuator mass, duration, thermal, torque-speed and measurement-boundary definitions before ranking. |

A November 2026 review of variable-stiffness methods recommends task-oriented integration of mechanisms/materials/structures and unified evaluation that includes auxiliary actuation systems, stiffness sensing, low-power stiffness retention and lifecycle reliability ([Actuators 15(11), 576](https://doi.org/10.3390/act15110576)). A September 2026 paper on dimensioning antagonistic artificial muscles specifically treats force–length behavior and co-contraction as the determinant of achievable joint stiffness range ([DOI 10.1016/j.mechmachtheory.2026.106453](https://doi.org/10.1016/j.mechmachtheory.2026.106453)).

### Revised competitive targets

- **J1 leg joint:** treat 35.7 N·m/kg measured rated density as a reproduction threshold only where rated-current density, package boundary, continuous-duty condition and mass denominator match. A >20% gain is a stretch objective, not a claim.
- **J2 wrist:** use 14 N·m/kg as the current paper-reported comparison point. A 16.8 N·m/kg stretch threshold is meaningful only if the same continuous pitch/yaw torque and mass/volume boundaries are reproduced, while preserving its workspace, embedded torque estimate and variable-stiffness performance.
- **J3 finger/gripper:** optimize accepted manipulation, contact-force estimation, slip response, service and sensor/communication cost. A single torque-density target is not a meaningful comparison.
- For all three, report rated versus peak, continuous versus transient, single-joint versus whole installed subsystem, and measured versus vendor/model-reported results distinctly.

The new task-conditioned optimizer work is tracked separately in [JOINT-TASK-CODESIGN-001 #6064](https://github.com/Luminous-Dynamics/symtropy/issues/6064); this document remains the synthetic-joint architecture and analytic-reference owner.

## Architecture comparison hypothesis

The first comparison should use one named article at a time:

- **J1:** a single-axis knee/elbow-like cartridge. Establishes effort/velocity/position semantics, reflected inertia, hard limits, friction, thermal derating and energy boundaries.
- **J2:** a two-axis wrist/ankle-like cartridge. Establishes full stiffness matrices, cross-axis coupling, workspace, singularities and stiffness modulation.
- **J3:** a tendon-coupled finger/gripper joint group. Establishes differential tendon routing, pretension, local sensing, contact and fault response.

Candidate families remain separate: geared rotary, quasi-direct/direct drive, series elastic, antagonistic variable stiffness, tendon-driven, rigid-flexible parallel, fluidic/electrohydraulic, artificial muscle and hybrid. A candidate does not win because it looks biomimetic or has an impressive peak number. It must improve a named application frontier under matched task, payload, installed mass/volume, source/thermal boundary, sensing, calibration and duty cycle.

## Research anchors reviewed

These works establish a current design space, not a universal winner:

- Hu et al., 15 October 2026, parallel + Yoshimura-origami wrist with antagonistic tendon/pneumatic stiffness control. DOI: [10.1016/j.mechmachtheory.2026.106545](https://doi.org/10.1016/j.mechmachtheory.2026.106545).
- Tsai et al., November 2026, compact wrist with compliant transmission, embedded torque sensing and variable stiffness; abstract reports 0.59 kg and 14 N·m/kg. DOI: [10.1016/j.mechmachtheory.2026.106577](https://doi.org/10.1016/j.mechmachtheory.2026.106577). This is the nearest current wrist comparator, but the exact torque-density boundary must be matched before claiming parity or improvement.
- Yoshimura et al., 2026, parametric 3D-printed TPU muscle/tendon/ligament/cushion analogues. DOI: [10.1002/aisy.70537](https://doi.org/10.1002/aisy.70537).
- 2026 sensorized-tendon continuum robot, where tendon transmits motion and measures its own length/resistance. DOI: [10.1016/j.mechatronics.2026.103524](https://doi.org/10.1016/j.mechatronics.2026.103524).
- Mao et al., *A self healable dielectric elastomer artificial muscle*, Nature Communications 17, 5451 (15 May 2026), report 125% stable strain at 25 MV/m for 150,000 cycles and self-healing after mechanical/electrical damage. DOI: [10.1038/s41467-026-72611-5](https://doi.org/10.1038/s41467-026-72611-5). This is a materials/actuator result, not yet a complete load-bearing robotic-joint qualification; electric field, electrode, insulation, packaging, thermal, load, and repair protocol must be evaluated at installed-joint level.
- A DEA automated lifetime-screening platform (11 August 2026) showed that systematic scanning across voltage, frequency, electrode concentration and conductive filler could improve operational lifetime of tested Elastosil actuators by up to 100% under boundary conditions. DOI: [10.1002/advs.77008](https://doi.org/10.1002/advs.77008). This motivates a materials-and-joint design-of-experiments pipeline; it is not a universal lifetime multiplier.
- 2026 artificial-muscle and dielectric/fluid-actuator reviews identify broad material classes and persistent system-level problems in lifetime, voltage, integration, fabrication and the force/strain/speed trade-off. DOI: [10.1002/smll.75538](https://doi.org/10.1002/smll.75538), [10.1038/s44182-026-00074-3](https://doi.org/10.1038/s44182-026-00074-3).
- 2026 hydraulic-joint review emphasizes installed mass/volume, fluid path, valves, leakage, heat rejection and duty-qualified output. [Technologies 14(11), 693](https://www.mdpi.com/2227-7080/14/11/693).

## What the first benchmark should measure

Do not collapse the following into one score:

| Dimension | Minimum report |
|---|---|
| Kinematics | DOF topology, reachable workspace, Jacobian conditioning, cross-axis coupling, range and repeatability |
| Output | Continuous/peak torque-speed curves, duty, torque density using the complete installed boundary |
| Interaction | Reflected inertia, breakaway/backdrive torque, friction, mechanical impedance, stiffness/damping matrix |
| Dynamic response | Bandwidth, step/chirp response, overshoot, settling, saturation and command-to-motion latency |
| Energy | Source/bus input, actuator input, mechanical work, recovered electrical energy and auxiliary loads as distinct ledgers |
| Thermal | Time constants, hot spots, warm-up, derating and sustainable duty |
| Sensing | Angle/velocity/tendon tension/wrench/strain error, uncertainty, bandwidth, hysteresis, drift and remount sensitivity |
| Reliability | Fatigue, creep, stress relaxation, tendon wear, backlash/friction drift, seal/leak and calibration drift |
| Service | Cartridge swap time, parts replaced, repairability, re-calibration and requalification burden |
| Safety/evidence | Actual applied motion/effort, hard-limit behavior, local fallback, passivity residual where applicable, provenance and OOD rejection |

The paper-reported 14 N·m/kg wrist is a useful external reference, but it is not a universal target for a knee, finger or exoskeleton joint. The benchmark should seek Pareto improvements on a named task and declared denominator—e.g. more backdrivable torque at the same thermal duty and installed mass, or lower lifecycle energy at matched task success—without trading away required limits, reliability or evidence quality.

## Next work (does not depend on CI runners)

1. Bind an analytic oracle for each fixture in a separate implementation from any controller/solver being evaluated.
2. Add exact engine adapters only after #4516 resolves/labels the relevant hinge capabilities.
3. Add static and dynamic actuator maps from #4179/#4206 and thermal state from #4610.
4. Add stiffness/passivity and interaction metrics by consuming #4728/#4754, not implementing duplicates.
5. Add tendon sensor lifecycle and physical reference measurements via #4753/#5624/#5680.
6. Only then compare task decisions with HLB on/off and mechanical-computation attributions under #5991 and Symthaea #12735.

## Claim ceiling

The PR adds a research note, a synthetic JSON corpus and Rust analytic-oracle source. No Symtropy joint-model change is included. The Rust test source has **not been executed**; CI for the current head is queued and is not a pass. No simulator, HIL or physical test is claimed. No actuator or joint has been built, measured, or physically qualified. The next promotion gate is exact-head executable tests, then engine adapters only after #4516 semantics are repaired or explicitly marked unsupported, followed by independently instrumented hardware measurements.
