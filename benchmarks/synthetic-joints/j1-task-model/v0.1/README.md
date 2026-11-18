# J1 source-faithful vertical-guide task model — reference v0.1

**Owner:** [J1-TASK-MODEL-REPRO-001 #6114](https://github.com/Luminous-Dynamics/symtropy/issues/6114)  
**Parent:** [JOINT-TASK-CODESIGN-001 #6064](https://github.com/Luminous-Dynamics/symtropy/issues/6064)  
**Source:** Huang, Dong & Zhao, *Task-Oriented Co-Design and Optimization of Geared Actuators for Robotic Applications*, arXiv:2609.22795v1 (19 September 2026), https://arxiv.org/html/2609.22795v1  
**Evidence class:** analytic synthetic reference fixtures; not source reproduction, simulator validation, hardware result or safety case.

## 1. Why this benchmark exists

A task-conditioned actuator search is only as credible as the task evaluator beneath it. The source paper couples motor and gearbox candidates to a two-DOF vertical-guide jumping leg, actuator-specific torque-speed limits, contact constraints, reflected inertia, a predictive catch/braking rule and candidate-specific hip/knee torque profiles. A motor design-space transcription alone cannot reproduce its Pareto ranking.

This first reference isolates source equations that can be tested without reconstructing missing mass/inertia parameters. It is intentionally small and independent of Symtropy's articulated solver. It establishes kinematic, virtual-work, Bézier, stopping-distance and contact-cone semantics. The full constrained dynamics and release trajectory remain unimplemented because their complete geometry, mass matrix and parameter provenance must first be reconstructed.

## 2. Source task boundary

The paper uses MuJoCo Unitree G1 fast-walking, jumping and stair-ascent traces to motivate task-specific torque-speed envelopes. These are **simulated task inputs**, not physical measurements from a G1:

| Simulated task | Peak torque | Peak speed |
|---|---:|---:|
| Fast walking | 103 N·m | 16.6 rad/s |
| Jumping | 139 N·m | 20.1 rad/s |
| Stair ascent | 109 N·m | 14.3 rad/s |

The physical task validation is a two-DOF vertical-guide jumping-leg bench with 5.5 kg common bench load and 48 V DC bus. The source model retains coupled hip/knee push-off, ground contact and the resulting joint trajectories but excludes whole-body balance and lateral dynamics. It is not a whole-humanoid jumping benchmark.

## 3. Coordinate and kinematic contract

Let `q = [q1, q2]`, where `q1` is absolute shank angle and `q2` is the included knee angle. The foot is the origin. The source equal-segment case assumes `l_s = l_t`.

```text
x_h(q) = l_s cos(q1) - l_t cos(q1 - q2)
y_h(q) = l_s sin(q1) - l_t sin(q1 - q2)
x_h(q) = 0                         // vertical-guide constraint
A(q) = partial x_h / partial q
J_y(q) = partial y_h / partial q
```

With `u = q1 - q2`, the analytic rows are:

```text
A_1 = -l_s sin(q1) + l_t sin(u)
A_2 = -l_t sin(u)
J_y1 = l_s cos(q1) - l_t cos(u)
J_y2 = l_t cos(u)
```

For an admissible velocity, `A(q) q_dot = 0`; slider velocity is `y_dot = J_y(q) q_dot`. The guide reaction is a constraint force and does zero virtual work on an exactly admissible velocity.

## 4. Generalized torque and power contract

The paper defines hip and knee output torques positive in the leg-extension direction and maps them into generalized coordinates as:

```text
B tau = [-tau_hip, tau_hip + tau_knee]^T
P = q_dot^T B tau
```

This mapping must be preserved exactly. Do not silently reinterpret `q1` and `q2` as two independent absolute joint angles. A power oracle checks the generalized-force mapping independently of a solver.

## 5. Candidate-specific control and contact

The source uses one fifth-order Bézier curve for each joint torque ratio:

```text
s = (q2 - q2_initial) / (q2_final - q2_initial)
alpha_j(s) = sum(i=0..5) c[j,i] * C(5,i) * (1-s)^(5-i) * s^i
tau_j(s) = alpha_j(s) * tau_available_j
0 <= c[j,i] <= 1
```

There are twelve policy variables: six hip coefficients and six knee coefficients. For this reference, only the polynomial/partition semantics are tested; no L-BFGS-B policy optimization is implemented.

Sticking contact constraints are:

```text
F_n >= 0
|F_t| <= mu * F_n
```

These are feasibility conditions, not soft objectives. Negative normal force or an exceeded friction cone is an invalid contact state.

## 6. Predictive braking

The source uses an inertia-aware stopping distance to coordinate braking before a travel limit:

```text
d_stop = omega_plus * t_delay
       + J_effective * omega_plus^2 / (2 * tau_brake)
       + position_margin
```

where the effective inertia includes reflected rotor inertia. This reference checks the arithmetic only. It does not reproduce the source's full actuator/robot effective inertia or brake scheduling.

## 7. Analytic known-answer fixtures

The JSON and Rust test corpus use synthetic values, not values claimed to come from the paper. They cover:

1. Equal-length vertical-guide pose, height and Jacobians.
2. Tangent velocity satisfying the guide constraint and its slider velocity.
3. Hip/knee generalized torque mapping and virtual-work power.
4. Fifth-order Bézier partition of unity and linear coefficient interpolation.
5. Predictive stopping-distance arithmetic.
6. Valid contact-cone point and invalid friction/normal-force controls.

All trigonometric results are computed in radians. The Rust tests compare against stored double-precision values with a small absolute/relative tolerance.

## 8. What is not yet reconstructed

- The complete configuration-dependent mass matrix `M(q)`, velocity-dependent vector `h(q,q_dot)` and gravity vector `G(q)`.
- Exact geometry, segment mass/inertia and source initial/final conditions for an end-to-end trajectory replay.
- Contact integration and release/catch event handling at the source's numerical settings.
- The full motor surrogate and gearbox equations; see [DOE manifest #6079 / PR #6082](https://github.com/Luminous-Dynamics/symtropy/pull/6082).
- The source's NSGA-II and inner L-BFGS-B searches.

These omissions must not be hidden behind a generic two-link simulator. They remain M2 reproduction tasks.

## 9. Required regression when promoted

Any eventual model reproduction must retain the source's four-command jump sweep, the DM8009 no-jump at 0.75×, all measured slider rises, and the prediction residual decomposition. At twice-rated command the source reports 1.008 m design-stage predicted rise versus 0.840 m measured; the model predicts 0.194 s / 0.287 m push-off duration/rise versus 0.240 s / 0.343 m measured. Replacing motor parameters and actuator mass explains 11.3 percentage points of the 16.6% total rise overprediction; 5.3 points remain for transmission, guide and control effects. Those are mandatory regression observations, not a fitting target to erase.

## 10. Run the independent oracle crate

This directory is a dependency-free Cargo package, deliberately isolated from the repository's main workspace:

```bash
cargo fmt --manifest-path benchmarks/synthetic-joints/j1-task-model/v0.1/Cargo.toml -- --check
cargo test --manifest-path benchmarks/synthetic-joints/j1-task-model/v0.1/Cargo.toml --locked
cargo clippy --manifest-path benchmarks/synthetic-joints/j1-task-model/v0.1/Cargo.toml --all-targets --locked -- -D warnings
```

The unit tests live in `src/j1_task_oracle.rs`, included only under `cfg(test)`. The lockfile has no third-party dependencies. Exact-head Rust execution is still required before these tests can be reported as passing.

## 11. Claim ceiling

Passing these oracle tests establishes only the listed analytic equations and synthetic fixtures. It does not reproduce the paper's dynamics, validate Symtropy's hinge engine, qualify a physical actuator, demonstrate a whole humanoid, or authorize hardware energization or human contact.