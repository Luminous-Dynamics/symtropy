# PHRI-PASSIVITY-001 — interaction-port passivity observer

Status: deterministic synthetic reference. No controller, actuator authority, passivity proof for a robot, physical measurement or human-contact qualification is claimed.

This dependency-free Rust crate implements a narrow port-energy observer for one synchronized interaction port.

## Port convention

The input wrench is the environment-on-robot wrench; the input twist is the co-located robot spatial velocity. Coordinates use SI units:

- wrench: [Fx, Fy, Fz, Tx, Ty, Tz] in N and N·m;
- twist: [vx, vy, vz, wx, wy, wz] in m/s and rad/s.

Power flowing into the robot/controller system is:

    P_in = wrench_environment_on_robot · twist_robot

Positive port work replenishes the declared initial storage. Negative port work drains it. The observer integrates power between consecutive samples with trapezoidal integration using actual integer-nanosecond elapsed time.

The conservative storage lower bound is:

    E_lower(t) = E_initial + W_port(t) - E_uncertainty

A deficit is reported when E_lower becomes negative. A violation sample is counted only when the deficit exceeds the caller's declared tolerance.

## Identity and ordering

Every sample carries:
- sample ID;
- monotonic timestamp in integer nanoseconds;
- clock ID and boot epoch;
- spatial frame ID;
- measured environment-on-robot wrench;
- measured robot twist.

Clock, boot epoch, frame and event ordering must remain consistent. Rejected inputs do not mutate observer state. Unknown/cross-clock data never becomes zero work.

## Run

    cargo fmt --manifest-path research/phri-passivity-001/Cargo.toml -- --check
    cargo test --locked --manifest-path research/phri-passivity-001/Cargo.toml
    cargo clippy --locked --manifest-path research/phri-passivity-001/Cargo.toml --all-targets -- -D warnings

## Explicit limits

- This is a measurement reference, not a passivity controller, energy tank manager, variable-admittance implementation or actuation authority.
- The energy-uncertainty allowance is supplied by the caller; it is not inferred from instrument certificates.
- The continuous-time passivity condition must be derived for the exact controller storage function and discrete update. This observer does not account for time-varying virtual inertia, damping or stiffness storage on its own.
- Force/torque/pressure limits, physical safety supervisor, stop/recovery, actuator saturation and thermal protection remain separate constraints.
- Electrical input/return and battery energy remain owned by Symtropy #4044/#4179; mechanical output work is not inferred from port passivity.
- The measurement is meaningful only if wrench and twist are calibrated, synchronized, co-located, and expressed in the same frame.

The reference supports PHRI-PASSIVITY-001 / Symtropy #4728 and composes with JointLab #4337 and the existing dynamics/actuator/energy owners.
