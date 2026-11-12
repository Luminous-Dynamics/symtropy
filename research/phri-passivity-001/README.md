# PHRI-PASSIVITY-001 — interaction-port passivity reference

Status: deterministic synthetic Rust reference. No robot controller, actuator authority, complete passivity proof, physical measurement or human-contact qualification is claimed.

This dependency-free Rust crate contains two intentionally narrow pieces:

1. A signed interaction-port energy-budget observer.
2. A diagonal virtual inertia/stiffness parameter-update energy calculation and conservative storage-budget projection.
3. A distinct diagonal variable-impedance (VIC) passivity-rate QP reference based on Dinc et al. (arXiv:2611.10778).

## 1. Port convention and observer

The input wrench is the environment-on-robot wrench; the input twist is the co-located robot spatial velocity. Coordinates use SI units:

- wrench: [Fx, Fy, Fz, Tx, Ty, Tz] in N and N·m;
- twist: [vx, vy, vz, wx, wy, wz] in m/s and rad/s.

Power flowing into the robot/controller system is:

    P_in = wrench_environment_on_robot · twist_robot

Positive port work replenishes the caller-declared available energy budget. Negative port work depletes it. The observer uses trapezoidal integration over the actual integer-nanosecond sample interval.

The conservative endpoint budget is:

    E_lower(t) = E_initial_budget + W_port(t) - E_uncertainty

A deficit is reported when this lower bound becomes negative. In addition, the observer evaluates the minimum inside each piecewise-linear power interval. This catches an interval that drains energy and then replenishes it before the next sample, even when its net endpoint work is zero.

Each observer binds:
- monotonic clock ID and boot epoch;
- physical port ID and spatial frame ID;
- a nonzero digest of the exact metrology/calibration/frame/time-alignment profile;
- a maximum permitted sample gap;
- initial available energy budget, uncertainty allowance and violation tolerance.

Each sample binds its event ID, monotonic timestamp, clock/boot epoch, port, frame, measurement-profile digest, wrench and twist. Rejected inputs leave the observer unchanged.

**Important interpretation:** the initial energy budget is supplied by the caller. It must represent an independently justified available passivity/energy-tank budget. This observer does not infer physical robot kinetic or potential energy. The fixed uncertainty allowance must be derived from force/torque, velocity, calibration, synchronization, frame registration and numerical integration uncertainty; it is not calculated automatically.

## 2. Variable-parameter storage and bounded projection

For the diagonal virtual impedance model:

    H = 0.5 * sum(M_i * velocity_i^2 + K_i * error_i^2)

the exact change at fixed error/velocity caused by a parameter update is:

    delta_H = 0.5 * sum(delta_M_i * velocity_i^2 + delta_K_i * error_i^2)

Positive storage creation is treated as an energy-budget debit. A negative parameter-induced storage change is reported but not automatically credited as available energy.

The fixed-size parameter projector handles 12 diagonal parameters ([M0..M5, K0..K5]). It minimizes weighted squared deviation from a proposed update subject to:
- lower/upper delta bounds (which the caller must derive from rate and absolute parameter limits);
- strictly positive virtual inertia and nonnegative stiffness after the update;
- a conservative budget on the sum of positive parameter-induced storage increments.

The one-constraint convex projection uses a scalar KKT multiplier and 80 fixed bisection iterations. It allocates no memory. Negative updates cannot fund positive updates. This is a diagonal special case, not the full matrix-valued method in recent research; it does not solve the published controller's complete dynamics or QP.

## 3. Published diagonal VIC passivity-rate QP reference

The reference implements the paper's diagonal variable-impedance rate constraint for a desired task-space inertia scale alpha, fixed diagonal base inertia Lambda_d, stiffness K, error e and error velocity v:

    0.5 * sum(Lambda_d_i * v_i^2 * alpha_dot_i
              + e_i^2 * K_dot_i)
      <= sum(D_i * v_i^2)

with critically damped diagonal terms:

    D_i = 2 * sqrt(K_i * alpha_i * Lambda_d_i)

The lower-rate barriers preserve positive impedance parameters:

    alpha_dot_i >= -kappa_alpha_i * (alpha_i - alpha_min_i)
    K_dot_i     >= -kappa_K_i * K_i

The implementation minimizes weighted squared deviation from the reference parameter rates subject to this affine passivity half-space, the two lower-rate barriers and additional caller-supplied upper-rate bounds. The strictly convex diagonal objective and one affine passivity inequality reduce to a fixed-size weighted projection with a scalar KKT multiplier and 80 fixed bisection iterations.

This rate QP is intentionally **separate** from the conservative storage-budget projector in Section 2:
- The energy-budget projector limits positive parameter-induced storage creation against a caller-supplied finite budget and does not credit released storage.
- The paper-style VIC rate QP uses the current-state damping term in a sufficient continuous-time passivity inequality; it does not consume a finite auxiliary energy reservoir in the same way.
- A result from either primitive is only a parameter proposal. It does not itself realize the impedance dynamics or authorize actuator commands.

The paper assumes accurately known rigid-body dynamics, a full-row-rank task Jacobian, and known external force feedback at the selected Cartesian interaction port. Its guarantee is for the analyzed task-space input/output pair; it explicitly does not guarantee passivity of redundant-robot null-space dynamics. The full compliance extension uses configuration-dependent matrix inertia, a continuously aligned generalized eigenbasis and an additional basis-rate term. That extension is **not** implemented here.

The authored tests cover the affine passivity projection known answer, no-projection behavior, lower-rate barriers, nonuniform objective weights, upper-rate bounds and invalid state/bounds. They are synthetic arithmetic checks, not a reproduction of the Kinova Gen3 experiments.

## Run

    cargo fmt --manifest-path research/phri-passivity-001/Cargo.toml -- --check
    cargo test --locked --manifest-path research/phri-passivity-001/Cargo.toml
    cargo clippy --locked --manifest-path research/phri-passivity-001/Cargo.toml --all-targets -- -D warnings

The current reference contains 35 authored deterministic tests covering:
- signed wrench/twist power and energy balance;
- irregular sample intervals and trapezoidal integration;
- interior within-interval energy minima;
- sample ordering, clock/boot, port, frame, measurement-profile and maximum-gap rejection;
- conservative uncertainty and violation tolerance;
- exact inertia/stiffness storage deltas;
- bounded parameter projection, positive storage budget, uncredited per-axis storage release, infeasible lower bounds and coefficient-overflow rejection;
- published diagonal VIC rate-QP arithmetic, passivity half-space projection, parameter decay barriers, nonuniform objective weights and extra upper-rate bounds.

These tests are authored, not yet compiled or executed.

## Relevant 2026 research comparators

- Poignonec et al., *NMPC-based variable admittance control with passivity constraints*, Robotics and Autonomous Systems (August 2026): https://doi.org/10.1016/j.robot.2026.105483
- Mühlbauer et al., *Passive Variable Impedance For Shared Control*, arXiv:2604.20557 (v2 July 2026): https://arxiv.org/abs/2604.20557
- Wan et al., *Human–Robot Interaction: External Force Estimation and Variable Admittance Control Incorporating Passivity*, Robotics 15(5), 84 (April 2026): https://doi.org/10.3390/robotics15050084
- Dinc et al., *Passivity-Guaranteed Variable Impedance and Compliance Control via Online Parameter Optimization*, arXiv:2611.10778 (submitted 7 November 2026): https://arxiv.org/abs/2611.10778

The new Dinc et al. preprint is a particularly important comparator: its reported method uses a convex QP to optimize inertia/stiffness rates, including configuration-dependent compliance, and compares against energy tanks and a nonpassivity-based method on a Kinova Gen3. The implementation here adds a narrow diagonal rate-QP reference for the paper's sufficient continuous-time task-space condition. It does not implement the full generalized-compliance extension or reproduce the Kinova experiments.

## Explicit limits

- This is not a passivity controller, energy-tank manager, complete variable-admittance implementation or actuation authority.
- The observer assumes a caller-qualified piecewise-linear port-power trace between samples. A maximum sample gap limits interpolation span; it does not prove the trace was band-limited or that no unobserved transient occurred.
- The energy-uncertainty allowance and metrology-profile digest are supplied by the caller; no instrument certificate or sensor truthfulness is inferred.
- The diagonal virtual storage model and VIC-rate QP omit off-diagonal inertia/stiffness coupling, generalized compliance, the configuration-dependent eigenbasis-rate term, null-space passivity, discrete-time integration proof and the complete robot/controller storage function.
- The VIC rate-QP uses additional caller-supplied upper-rate caps for engineering tractability; these are implementation constraints, not part of the paper's stated QP theorem.
- A passivity budget deficit is not a complete stability or human-contact safety verdict.
- Force/torque/pressure limits, independent local safety authority, stop/recovery, actuator saturation and thermal protection remain separate constraints.
- Electrical input/return and battery energy remain owned by Symtropy #4044/#4179; mechanical port work is not inferred to be battery regeneration.
- Human-subject testing requires separate ethics and safety approval.

The reference supports Symtropy #4728 and composes with JointLab #4337 and the existing dynamics/actuator/energy owners.
