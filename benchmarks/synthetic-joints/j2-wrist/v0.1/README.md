# J2 two-DoF wrist — topology-agnostic analytic reference v0.1

**Owner:** [J2-WRIST-TOPOLOGY-001 #6136](https://github.com/Luminous-Dynamics/symtropy/issues/6136)  
**Parent:** [SYNTH-JOINT-001 #6037](https://github.com/Luminous-Dynamics/symtropy/issues/6037)  
**Evidence class:** synthetic analytic known-answer fixtures plus source-bound comparator metadata; not a model of a published prototype and not a physical qualification.

## Why a topology-agnostic benchmark

Serial, parallel and rigid-flexible wrists allocate mass, actuation, compliance and constraints differently. Comparing only peak torque or nominal torque density can hide weak task directions, singularities, cross-axis stiffness or uncharged auxiliary systems.

The benchmark therefore compares every candidate through a shared two-dimensional task-space port. Each topology supplies its own locally valid Jacobian and joint-space stiffness. The common mathematical contract does not assume that serial or parallel architectures share their native coordinates.

## Literature anchors

### B1 — integrated compliant serial pitch-yaw wrist (November 2026)

Tsai et al., “A compact wrist mechanism with integrated compliant transmission for high-torque-density and variable-stiffness actuation,” *Mechanism and Machine Theory* 229 (November 2026), DOI [10.1016/j.mechmachtheory.2026.106577](https://doi.org/10.1016/j.mechmachtheory.2026.106577).

The paper reports a 0.59 kg, two-DoF serial pitch-yaw wrist with external dimensions 59 × 83 × 102 mm and stated volume 499.5 cm³. It reports continuous output pitch/yaw torques of 8.10/8.30 N m and torque density 14 N m/kg. The architecture uses harmonic drives with an integrated compliant cycloidal gear and compliant cap. Dual encoders and transmission-error compensation infer output torque from embedded compliance. The paper reports experiments for torque control, zero-stiffness control, variable-stiffness control, active backdrivability and position control, using a 1 kHz real-time loop.

The paper authors report that the integrated compliance removes the need for external torque sensors in that wrist. This is not evidence that an independent torque reference is unnecessary for qualification or safety. An external calibrated reference is still required when measuring estimator error and uncertainty.

Do not assume that its quoted torque density includes all external drivers, power electronics, wiring, cooling or remote controller hardware unless the original mass and system boundary tables establish that.

### B2 — rigid-flexible parallel wrist with origami coupling (October 2026)

Hu et al., “Bioinspired wrist joint with variable stiffness using a parallel mechanism and origami coupling,” *Mechanism and Machine Theory* 228 (15 October 2026), DOI [10.1016/j.mechmachtheory.2026.106545](https://doi.org/10.1016/j.mechmachtheory.2026.106545).

The paper describes a 3-PRS/SS parallel mechanism nested with a Yoshimura origami structure, a rigid inner guide frame, flexible outer chamber and antagonistic tendon/pneumatic actuation for stiffness modulation. The abstract reports theoretical and experimental validation of motion flexibility and stiffness tunability. Quantitative matched torque-density, installed mass, air-source energy and independent torque-reference values are not entered in this register until their source tables and test boundaries are inspected.

### B3 — older experimental parallel wrist (2014)

Chu et al., “Design and experiment of a compact wrist mechanism with high torque density,” *Mechanism and Machine Theory* 78 (August 2014), DOI [10.1016/j.mechmachtheory.2014.03.002](https://doi.org/10.1016/j.mechmachtheory.2014.03.002). The source reports an experimentally validated 5R spherical parallel mechanism driven by two linear motors, with torque density 7.20 N m/kg. It is an older reference, not an apples-to-apples matched comparison with B1 or B2.

The comparator JSON stores reported facts separately from unknown fields. The value 14/7.2 is not presented as a valid 1.94× performance comparison; the mechanisms, denominators, duty and test protocols are unmatched.

## Shared two-DoF task-space contract

For local coordinates q and task coordinates x:

    x_dot = J(q) q_dot
    tau = J(q)^T F
    tau^T q_dot = F^T x_dot

Here J is the local 2×2 velocity Jacobian, F is the task-space wrench/tangent effort, and tau is generalized joint effort. The virtual-work identity is a required invariant for every mechanism-specific model.

For a nonsingular square local mapping, the linearized stiffness transformation is:

    K_x = J(q)^(-T) K_q J(q)^(-1)

The complete 2×2 task-space stiffness matrix must be reported. A scalar isotropic stiffness can obscure cross-axis coupling and weak directions. This transformation assumes the same local coordinates, small perturbations, a locally constant Jacobian and a nonsingular mapping; configuration-dependent geometric stiffness and preload terms require additional model terms in a higher-fidelity treatment.

The analytic kernel computes:
- singular values of J;
- condition number sigma_max / sigma_min;
- absolute determinant as the local area/manipulability measure;
- wrench-to-generalized-effort mapping;
- virtual-work equality;
- joint-to-task stiffness transform;
- stored quadratic energy for a positive-definite stiffness matrix.

For numerical safety, matrix inversion is rejected when the condition number exceeds 1e8. This is a declared software cutoff for this reference, not a mechanism-specific physical singularity threshold. Mechanism-specific workspace and singularity margins must be derived from the actual geometry and tolerances.

## Synthetic fixtures and limitations

The test matrices are synthetic and are not copied from the Tsai or Hu prototypes. Coverage includes:
1. Identity Jacobian with unit singular values.
2. A coupled, nonsingular Jacobian with known singular values and condition number.
3. Task wrench and joint velocity virtual-work equality.
4. Joint stiffness transformed to task coordinates.
5. Positive task-space stored energy.
6. Rank-deficient and ill-conditioned inverse rejection.
7. Non-symmetric / non-positive-definite stiffness rejection.
8. Non-finite input rejection.

The reference does **not** implement:
- Tsai's serial pitch-yaw geometry, harmonic drives, compliant cycloidal gear or encoder compensation;
- Hu's 3-PRS/SS kinematics, Yoshimura origami mechanics, tendon routing, pneumatic chambers or antagonistic stiffness controller;
- a calibrated physical stiffness matrix;
- a multi-DoF quaternion orientation error, workspace or collision model;
- actuator torque-speed maps, pneumatic supply energy, heat rejection, fatigue or impact dynamics.

Those remain source extraction and mechanism-specific M2 work. No prototype geometry or parameter has been inferred from an abstract.

## Matched J2 task suite

For a real comparison, bind each candidate to the same:
- task-space pose/orientation grid and required reachable set;
- external wrench direction, amplitude, duty and contact duration;
- payload and moving-link inertia;
- desired task-space stiffness/damping matrix and transition schedule;
- trajectory velocity/acceleration and accepted task outcome;
- electrical bus or pneumatic source pressure/flow/energy;
- thermal ambient and heat-rejection condition;
- sensing information and estimator sampling/control period;
- complete installed mass/volume, including motors, drivers, cables, valves, compressor/reservoir, cooling and mounting;
- instrumentation calibration, uncertainty and exact configuration identity.

Report two frontiers: matched-envelope performance under common constraints, and each topology's native optimum. A design should not be declared better just because it moves an auxiliary source, motor or controller outside its accounting boundary.

## Scorecard and promotion

Report workspace coverage, condition-number distribution, directional manipulability, cross-axis coupling, full stiffness/damping matrices, continuous and peak wrench envelopes, torque-estimation error and uncertainty, backdrive torque, impact margin, power/energy, thermal derating, mass/volume, fatigue, maintenance and requalification.

M1 targets exact fixture agreement and virtual-work equality within declared floating-point tolerance; singular/ill-conditioned mappings and invalid stiffness matrices fail closed. M2 must compare independent mechanism-specific kinematic implementations. M3 requires a separately approved physical protocol with external calibrated torque/wrench references, independent pose tracking, synchronized input energy and thermal measurements. A physical torque estimator cannot qualify itself using only the encoders and compliance it consumes.

## Run the independent oracle crate

This directory is an isolated, dependency-free Cargo package:

    cargo fmt --manifest-path benchmarks/synthetic-joints/j2-wrist/v0.1/Cargo.toml -- --check
    cargo test --manifest-path benchmarks/synthetic-joints/j2-wrist/v0.1/Cargo.toml --locked
    cargo clippy --manifest-path benchmarks/synthetic-joints/j2-wrist/v0.1/Cargo.toml --all-targets --locked -- -D warnings

Exact-head Rust execution is required before tests may be reported as passing.

## Claim ceiling and safety

The tests establish only the listed two-dimensional analytic identities and synthetic fixtures. They do not reproduce a cited wrist, establish a hardware capability, validate pneumatic pressure systems, or demonstrate human-contact safety. No fabrication, pressurization, energization, impact test or human-contact trial is authorized by this reference. Physical work requires separately approved requirements, hazard analysis, competent test procedures and independently calibrated instrumentation.
