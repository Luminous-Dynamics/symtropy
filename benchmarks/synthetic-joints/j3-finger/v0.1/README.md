# J3 tendon-driven finger — pose-robust force-feasibility oracle v0.1

**Owner:** [J3-POSE-ROBUST-HAND-001 #6138](https://github.com/Luminous-Dynamics/symtropy/issues/6138)  
**Parent:** [SYNTH-JOINT-001 #6037](https://github.com/Luminous-Dynamics/symtropy/issues/6037)  
**Evidence class:** source transcription of PACT plus synthetic analytic tests; not reproduction of its prototype or complete force-polytope benchmark.

## Research anchor: PACT (3 November 2026)

Kumar et al., “PACT: Pose-Robust Actuation Co-Design for Tendon-Driven Robotic Hands,” arXiv:2611.05302: https://arxiv.org/abs/2611.05302

PACT couples tendon routing, signed moment arms, capstan dimensions and actuator allocation to pose-dependent force requirements and drivetrain bandwidth. Its four-DoF finger has three flexion/extension coordinates and an MCP abduction/adduction coordinate. Flexion/extension uses an asymmetric four-tendon N+1 routing, while a separate bidirectional actuator drives abduction/adduction.

The source enforces the complete equilibrium:

    J(q)^T f = R T + e_AA * tau_AA
    0 <= T <= T_max
    |tau_AA| <= tau_AA,max

Here J(q) is the 3×4 fingertip Jacobian, f is the 3D fingertip force, R is the 4×4 signed routing/moment-arm matrix, T are pull-only tendon tensions and e_AA = [1,0,0,0]^T.

The source emphasizes that pseudoinverse projections can invent feasible forces by discarding joint-torque components that the tendon system cannot generate. PACT instead enforces every joint-equilibrium equation and rejects singular/lower-dimensional force sets.

### Source geometry and task transcription

The machine-readable JSON records:
- eight source pose rows (small/medium/large pinch, medium/small tripod, extended reach, deep flexion, abducted pinch);
- source Table IV routing signs/moment arms, converted from millimetres to metres;
- capstan diameters 12.7, 11.0, 13.0 and 36.0 mm;
- actuator allocation and source reported gear ratios, rotor inertias, continuous/peak output torques and output speeds;
- opposing truncated Coulomb friction-cone requirements with mu=0.5, palmar limit 15 N and dorsal limit 4 N;
- a 5 Hz minimum isolated flexion/extension bandwidth constraint.

PACT's shared design improves the source-reported minimum task-force region coverage from 0.24 (baseline) to 0.88, with maximum pose coverage 1.00 and a predicted minimum flexion/extension bandwidth of 5 Hz. This is a **volume-coverage** measure over a required force region, not a directional force maximum.

### Crucial physical-validation boundary

The source prototype's six-axis fingertip load-cell force validation was conducted at one pose, with actuator current limits set to 50% of peak, 5-second steady-state samples and five repetitions per actuator combination. The paper explicitly says whole-hand grasp stability was not established.

The source's frictionless force model overpredicted measured force magnitude by approximately 24.5% and had a 14.82° directional error at the tested pose. Estimated cumulative losses increased toward distal joints (MCP/PIP/DIP effective efficiencies approximately 0.86/0.76/0.68). Re-scoring the shared design with these joint-dependent losses lowered worst-pose coverage from 0.88 to 0.75. Those residuals are mandatory regression observations, not a tuning target to erase.

## What this M1 oracle implements

The Rust reference solves the exact **direction-constrained** four-DoF static equilibrium for a full-rank 4×4 routing matrix:

    f = alpha * d
    J(q)^T d = b
    T = alpha * (R^-1 b) - tau_AA * (R^-1 e_AA)

The bounded variables are alpha >= 0, 0 <= T_i <= T_i,max and -tau_AA,max <= tau_AA <= tau_AA,max. After eliminating the four tendon tensions, only two variables remain: the desired force scale alpha and the bidirectional A/A torque tau_AA. The implementation enumerates pairwise intersections of the resulting two-dimensional half-space boundaries, checks every inequality and verifies the reconstructed four-joint equilibrium at the selected optimum.

This is an exact active-set/vertex method for the stated reduced, full-rank directional problem. It is not a pseudoinverse approximation. The reference rejects rank-deficient routing matrices, a zero task-torque direction and invalid force/tension inputs.

The package also checks tendon kinematic rates and power conjugacy:

    l_dot = -R^T q_dot
    tau^T q_dot = -T^T l_dot + tau_AA * q_dot_AA

The identity fixture tests the sign convention and power balance.

## Full 3D force-polytope reference extension

The same M1 package now includes a generic 3D force-polytope and coverage reference in `src/force_polytope.rs`:

1. Construct the bounded set in augmented coordinates `(f_x, f_y, f_z, tau_AA)` from the complete joint equilibrium, per-tendon lower/upper tension bounds, and bidirectional A/A torque bound.
2. Eliminate the scalar auxiliary torque using Fourier–Motzkin projection to obtain a three-dimensional half-space (H-polytope) representation in fingertip force space.
3. Enumerate feasible intersections of triples of boundary planes to obtain candidate vertices, reject invalid/unbounded regions, and triangulate each convex face to compute volume.
4. Compute required-force-region coverage as `volume(force_set intersect required_region) / volume(required_region)`.

The synthetic fixtures check exact cube volumes, identical and partial intersections, an empty intersection, invalid/unbounded regions, rank-deficient Jacobians, and nonzero minimum tendon tension (pretension). These are generic analytic geometry tests; they are not PACT's source task-force regions.

The projection retains per-tendon minimum and maximum tensions. This is useful for future antagonistic/pretension models: a signed joint-torque interval is not substituted for the physical tendon tension bounds.

## Deliberate non-claims

- The generic H-polytope/volume machinery does **not** yet reproduce PACT's reported eight-pose force-region coverage; source-specific pose Jacobians and the exact required-force-region halfspaces still need extraction and independent review.
- A directional force maximum is not PACT's full required-force-region volume coverage.
- The synthetic identity routing/Jacobian fixtures are not the PACT prototype.
- The source routing matrix is transcribed and checked for rank/invertibility, but the source's pose-dependent Jacobians and motor/control system are not reconstructed here.
- This M1 oracle does not model tendon friction, backlash, slack dynamics, elastic compliance, capstan losses, actuator torque-speed maps, thermal duty, controller dynamics, contact-patch changes or whole-hand grasp force closure.
- The PACT prototype residual (24.5% force magnitude deviation / 14.82° direction error) is recorded as a source observation, not represented as a result from this package.

## Next benchmark tiers

- **M0 source:** verify all table values, signs, units, geometry, pose coordinates and test protocol against source version and supplemental material.
- **M1 analytic:** direction-constrained force LP, generic 3D H-polytope projection/volume coverage, virtual-work identity and synthetic positive/negative controls (this package).
- **M2 simulation:** reconstruct fingertip Jacobian at all eight poses, calculate exact feasible force polytopes and source-equivalent region coverage; implement PACT's 5 Hz isolated-joint drivetrain bandwidth equations. Add direction-dependent tendon efficiency and compare frictionless and loss-aware predictions without retuning away the reported errors.
- **M3 instrumented finger/gripper:** measure external 3D fingertip wrench, individual tendon force, tendon length/strain, joint pose, current/voltage, speed and temperature. Test all poses, directions, force limits and sustained loading, not one representative pose.
- **M4 lifecycle and system:** wear/creep, remount, friction drift, cable damage, sensor ablation, disturbance recovery, held-out objects, multi-finger grasp and service requalification.

## Run the independent oracle crate

This directory is a dependency-free Cargo package:

    cargo fmt --manifest-path benchmarks/synthetic-joints/j3-finger/v0.1/Cargo.toml -- --check
    cargo test --manifest-path benchmarks/synthetic-joints/j3-finger/v0.1/Cargo.toml --locked
    cargo clippy --manifest-path benchmarks/synthetic-joints/j3-finger/v0.1/Cargo.toml --all-targets --locked -- -D warnings

Exact-head Rust execution is required before any test may be reported as passing.

## Claim ceiling and safety

Passing the tests establishes only the listed static direction-constrained equilibrium and virtual-work identities with synthetic fixtures, plus inversion of the source-transcribed routing matrix. It does not reproduce PACT, demonstrate its reported 0.88 pose coverage, qualify a tendon-driven finger or establish human-contact safety. No fabrication, energization, human-contact test or use of inferred tendon tension as a safety signal is authorized by this reference. Physical work requires separately approved requirements, hazard analysis, competent procedures and independently calibrated instrumentation.
