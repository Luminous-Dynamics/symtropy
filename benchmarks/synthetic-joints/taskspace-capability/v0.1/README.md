# Pose-Dependent Task-Space Capability Field v0.1

**Status:** M1 rotational analytic reference and literature protocol. This package does not model a specific serial/parallel wrist or ankle prototype, and does not make a physical capability claim.

**Research owner:** [TASKSPACE-COMPLIANCE-ENVELOPE-001 #6189](https://github.com/Luminous-Dynamics/symtropy/issues/6189)  
**Parent J2 comparison:** [J2-WRIST-TOPOLOGY-001 #6136](https://github.com/Luminous-Dynamics/symtropy/issues/6136)  
**Common benchmark portfolio:** [SYNTH-JOINT-001 #6037](https://github.com/Luminous-Dynamics/symtropy/issues/6037)

## 1. Paradigm shift

A single nominal Jacobian or scalar stiffness value does not qualify a wrist/ankle architecture. The capability varies over configuration, task direction, payload, internal stiffness state, assembly tolerances and external loading.

The required object is a field over the task workspace:

```text
C(q, s) = {
  J(q,s), singular_values, condition_number,
  task_space_stiffness, feasible_wrench_set,
  power/energy, uncertainty
}
```

The scorecard must retain local minima and distributions. A high average workspace score must not hide a singularity or an unusable force direction at a critical task pose.

## 2. Current research anchors

### Variable-stiffness rigid-flexible parallel wrist

Hu et al., *Bioinspired wrist joint with variable stiffness using a parallel mechanism and origami coupling*, Mechanism and Machine Theory 228 (15 October 2026), article 106545, DOI https://doi.org/10.1016/j.mechmachtheory.2026.106545.

The source describes a 3-PRS/SS parallel mechanism nested with a Yoshimura origami structure, rigid inner guidance, flexible outer chamber and antagonistic tendon/pneumatic stiffness actuation. The abstract reports theoretical and experimental validation of tunable stiffness and motion flexibility. Matched installed mass, output torque, pneumatic source energy and independent torque-reference details remain unknown until the source tables and test boundary are inspected.

### Spherical parallel wrist

Yavuz and Himmetoglu, *A Novel Parallel Exoskeleton for Wrist Rehabilitation: Conceptual Design, Kinematics, and Singularity Analysis*, Machines 14(9), 1039 (12 September 2026), DOI https://doi.org/10.3390/machines14091039.

The source reports a 3-DoF spherical parallel architecture with base-fixed actuators and a virtual centre. Its rotated-frame model reports 94.14% high-dexterity workspace coverage at condition number below 5 and 99.78% usable coverage below 10; the initial-frame results are 78.80% and 92.18%. These are source-specific kinematic results, not universal thresholds or a matched industrial-wrist leaderboard.

### Self-reconfigurable parallel ankle

Alrais et al., *Design and Control of an Enhanced Self-Reconfigurable Parallel Ankle Rehabilitation Exoskeleton*, Design for Augmented Humanity 1(3) (4 August 2026), DOI https://doi.org/10.1177/29776481261463651.

The source reports a three-limb reconfigurable parallel platform for static mobilization and dynamic gait-oriented modes, position-level inverse kinematics, Jacobian evaluation over workspace and experimental trajectory/setpoint tracking with limited cross-axis coupling. Its motivation includes migrating instantaneous ankle axes and the parasitic loads caused by axis misalignment.

### Targeted compliant parallel mechanisms

Liu et al., *Anthropomorphic model-driven design-evaluation method for targeted compliant parallel rehabilitation mechanisms*, International Journal of Mechanical Sciences 326 (15 September 2026), article 111917, DOI https://doi.org/10.1016/j.ijmecsci.2026.111917.

The source proposes separate primary-motion transmissibility and concomitant-motion adaptability indices. This is a useful distinction: a mechanism should transmit required axes accurately while accommodating or constraining secondary axes according to the task; minimizing all coupling indiscriminately is not always correct.

### Redundant versus non-redundant parallel ankle

Zhang et al., *Comparative analysis and optimization of an ankle rehabilitation redundantly actuated robot and its non-redundant counterpart using a modified trajectory and new dynamic indices*, Journal of Mechanical Engineering Science (24 April 2026), DOI https://doi.org/10.1177/09544062261439475.

The source reports average-limb-inertia-coupling and variation indices, inverse dynamics and NSGA-II optimization of coupling/workspace. Its data are available on request, so this is a methodological reference rather than a fully reproducible public dataset.

## 3. Mathematical scope of the M1 crate

This package adds a dimension-specific **3×3 rotational** analytic reference:

- `SO(3)` exponential and principal logarithm maps;
- rotation-matrix orthonormality and right-handed determinant check;
- singular values from the eigenvalues of `JᵀJ` using a symmetric Jacobi eigensolver;
- condition number and absolute determinant/manipulability volume;
- a declared inverse cutoff at condition number (10^8);
- local joint-to-task stiffness transformation for a square, nonsingular map:
  `K_task = J^(-T) K_joint J^(-1)`.

This is deliberately not yet a 6D spatial twist/wrench model. Mixed translational/rotational coordinates require an explicit characteristic length and a documented metric before singular values or condition numbers can be compared.

The inverse cutoff is a numerical guard for this reference, not a physical singularity or safety threshold. A mechanism-specific workspace threshold must be justified from the task and geometry.

## 4. Synthetic known-answer and negative controls

Twelve Rust test definitions cover:
1. Isotropic identity Jacobian.
2. Anisotropic diagonal Jacobian with known singular values, condition and determinant.
3. Invariance of singular values and determinant under a left rotation.
4. Singular and ill-conditioned inverse rejection.
5. General `SO(3)` exp/log round trip.
6. Small-angle stable limit.
7. Near-(pi) finite principal logarithm.
8. Non-rotation rejection.
9. Identity stiffness transformation.
10. Rotated stiffness transform symmetry/positive definiteness.
11. Non-SPD stiffness rejection.
12. Non-finite input rejection.

These are synthetic numerical tests, not source-prototype reproductions.

## 5. What remains for M2

The next research stage must build mechanism-specific kinematics:
- serial pitch-yaw wrist with its actual gear/compliance architecture;
- Hu 3-PRS/SS origami wrist including tendon and pneumatic stiffness state;
- Yavuz spherical wrist workspace using both coordinate formulations;
- Alrais reconfigurable ankle with position-level inverse kinematics and alignment/reconfiguration states.

For each mechanism, record exact configuration, coordinate conventions, geometry, actuator map, constraints and as-built boundary. Evaluate a preregistered configuration grid and trajectories. Retain full distributions of condition number, minimum singular value, task stiffness eigenvalues, primary-axis transmission, concomitant-motion response and parasitic wrench.

The current crate cannot infer a prototype Jacobian from an abstract or CAD image. Source-specific geometry, parameter uncertainty and independent review are required.

## 6. Matched scorecard

Report separate Pareto axes:
- workspace coverage below source-aligned condition thresholds, plus full condition-number CDF and worst-case singularity margin;
- minimum singular value and directional manipulability;
- primary-motion transmissibility and concomitant-motion accommodation/rejection;
- task-space stiffness eigenvalues, principal directions and cross-axis coupling;
- feasible task-wrench coverage and directional margins;
- alignment/centre-offset sensitivity and parasitic wrench;
- backdrivability, reflected/moving mass, total installed mass and volume;
- peak/continuous torque-speed, impact and acceleration envelope;
- stiffness tunability, transition time, hysteresis and holding power;
- electrical and pneumatic auxiliary energy, including compressor/valve supply where relevant;
- thermal derating, durability, service and requalification.

Do not collapse these metrics into a universal dexterity score. A parallel mechanism can have high stiffness and still be poor if its task workspace or alignment robustness is inadequate.

## 7. Run the analytic reference

The package is dependency-free and isolated from the root Cargo workspace:

```bash
cargo fmt --manifest-path benchmarks/synthetic-joints/taskspace-capability/v0.1/Cargo.toml -- --check
cargo test --manifest-path benchmarks/synthetic-joints/taskspace-capability/v0.1/Cargo.toml --locked
cargo clippy --manifest-path benchmarks/synthetic-joints/taskspace-capability/v0.1/Cargo.toml --all-targets --locked -- -D warnings
```

The dedicated workflow is `.github/workflows/taskspace-capability-field-reference.yml`. The current environment does not include Cargo/rustc/rustfmt; exact-head CI is required before reporting tests as passing.

## 8. Claim ceiling and safety

This M1 package establishes only the listed synthetic 3D rotational identities and fixtures. It does not reproduce Hu, Yavuz, Alrais or Tsai, establish a hardware workspace, qualify a pneumatic mechanism, or demonstrate human-contact safety.

No fabrication, pressurization, energization, impact testing or human-contact trial is authorized by this reference. Physical work requires separately approved requirements, hazard analysis, competent procedures and independently calibrated instrumentation.
