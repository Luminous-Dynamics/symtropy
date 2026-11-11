# PHRI-PASSIVITY-001 — interaction-port passivity reference

Status: deterministic synthetic Rust reference. No robot controller, actuator authority, complete passivity proof, physical measurement or human-contact qualification is claimed.

This dependency-free Rust crate contains two intentionally narrow pieces:

1. A signed interaction-port energy-budget observer.
2. A diagonal virtual inertia/stiffness parameter-update energy calculation and bounded convex projection.

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

## Run

    cargo fmt --manifest-path research/phri-passivity-001/Cargo.toml -- --check
    cargo test --locked --manifest-path research/phri-passivity-001/Cargo.toml
    cargo clippy --locked --manifest-path research/phri-passivity-001/Cargo.toml --all-targets -- -D warnings

The current reference contains 26 authored deterministic tests covering:
- signed wrench/twist power and energy balance;
- irregular sample intervals and trapezoidal integration;
- interior within-interval energy minima;
- sample ordering, clock/boot, port, frame, measurement-profile and maximum-gap rejection;
- conservative uncertainty and violation tolerance;
- exact inertia/stiffness storage deltas;
- bounded parameter projection, positive storage budget, uncredited storage release and infeasible lower bounds.

These tests are authored, not yet compiled or executed.

## Relevant 2026 research comparators

- Poignonec et al., *NMPC-based variable admittance control with passivity constraints*, Robotics and Autonomous Systems (August 2026): https://doi.org/10.1016/j.robot.2026.105483
- Mühlbauer et al., *Passive Variable Impedance For Shared Control*, arXiv:2604.20557 (v2 July 2026): https://arxiv.org/abs/2604.20557
- Wan et al., *Human–Robot Interaction: External Force Estimation and Variable Admittance Control Incorporating Passivity*, Robotics 15(5), 84 (April 2026): https://doi.org/10.3390/robotics15050084
- Dinc et al., *Passivity-Guaranteed Variable Impedance and Compliance Control via Online Parameter Optimization*, arXiv:2611.10778 (submitted 7 November 2026): https://arxiv.org/abs/2611.10778

The new Dinc et al. preprint is a particularly important comparator: its reported method uses a convex QP to optimize inertia/stiffness rates, including configuration-dependent compliance, and compares against energy tanks and a nonpassivity-based method on a Kinova Gen3. The Luminous diagonal projector is a small independent reference baseline only; it is not a reproduction of that full method.

## Explicit limits

- This is not a passivity controller, energy-tank manager, complete variable-admittance implementation or actuation authority.
- The observer assumes a caller-qualified piecewise-linear port-power trace between samples. A maximum sample gap limits interpolation span; it does not prove the trace was band-limited or that no unobserved transient occurred.
- The energy-uncertainty allowance and metrology-profile digest are supplied by the caller; no instrument certificate or sensor truthfulness is inferred.
- The diagonal virtual storage model omits off-diagonal inertia/stiffness coupling, time-varying damping storage, configuration-dependent full matrices and the complete robot/controller storage function.
- A passivity budget deficit is not a complete stability or human-contact safety verdict.
- Force/torque/pressure limits, independent local safety authority, stop/recovery, actuator saturation and thermal protection remain separate constraints.
- Electrical input/return and battery energy remain owned by Symtropy #4044/#4179; mechanical port work is not inferred to be battery regeneration.
- Human-subject testing requires separate ethics and safety approval.

The reference supports Symtropy #4728 and composes with JointLab #4337 and the existing dynamics/actuator/energy owners.
