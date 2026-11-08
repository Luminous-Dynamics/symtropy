# LOCO Actuator Power Map V0

Dependency-free Rust reference for a bounded, calibrated four-axis actuator electrical-power map, tracked by [LOCO-ACTUATOR-MAP-001 / #4179](https://github.com/Luminous-Dynamics/symtropy/issues/4179).

## Paradigm shift: actuator energy is an operating-envelope model, not a constant efficiency

A single motor-efficiency constant cannot represent torque/speed quadrant changes, bus-voltage dependence or thermal derating. V0 evaluates a calibrated rectilinear map scoped to one exact actuator, drive, firmware and calibration identity.

The lookup axes are:

1. signed joint torque (micro-newton-metres);
2. signed angular velocity (microradians/s);
3. temperature (millicelsius);
4. DC-bus voltage (millivolts).

Each grid node contains lower, nominal and upper signed electrical-input power in pW. Negative power is retained as electrical energy returned according to the map's declared drive sign convention. Four torque/speed quadrants are preserved; no absolute-value symmetry is assumed.

## Bounded interpolation and uncertainty

V0 uses fixed-point Q16 fractions per axis. The 16 non-negative corner weights sum exactly to 2^64. Lower values are rounded outward/down, upper values outward/up and nominal values to nearest with ties-to-even.

Because Q16 coordinate fractions can quantize an operating point, V0 evaluates the corners of the fraction-quantization box and expands the reported lower/upper power bounds to cover that interpolation uncertainty. It does not silently treat the fixed-point interpolation as exact continuous arithmetic.

Queries outside any calibrated axis return an explicit OutsideCalibrationEnvelope error. There is no extrapolation, efficiency floor or fallback motor model.

## Mechanical power and loss consistency

The operating point gives exact mechanical joint power:

    P_mechanical[pW] = torque[µN·m] * angular_velocity[µrad/s]

The reference computes:

    P_loss_interval = P_electrical_input_interval - P_mechanical

It labels the interval:
- Consistent: its entire loss interval is non-negative;
- Uncertain: the interval crosses zero;
- Inconsistent: its entire interval is negative.

This is a consistency check under the declared sign conventions, not a decomposition into copper, iron, inverter, gearbox or thermal losses. Those require independently instrumented boundaries. Negative mechanical work is not assumed to imply electrical regeneration.

## Identity and provenance

Each map binds:
- actuator ID;
- drive ID;
- firmware digest;
- calibration digest;
- source artifact digest;
- caller-supplied map digest;
- four strictly increasing calibrated axes and the full grid.

Every prediction returns actuator/drive/firmware/calibration/map identity, interpolation cell, Q16 fractions, electrical input interval, mechanical power and loss-consistency result.

The crate validates identity fields are non-empty; it does not cryptographically authenticate source files or prove that the map digest commits to the supplied tensor. That belongs in the external artifact receipt/verifier. V0 assumes one calibration/source artifact for the full grid; per-cell provenance can be added when a dataset needs heterogeneous calibration lineage.

## Tests

Twelve deterministic fixtures cover:
- exact four-axis linear interpolation;
- four-quadrant signed power;
- strict no-extrapolation on every axis;
- inclusive upper boundary and final-cell semantics;
- negative-power outward rounding;
- loss consistency/uncertainty/inconsistency;
- invalid identities and power intervals;
- invalid axis order/length;
- exact tensor-size matching;
- signed midpoint interpolation;
- provenance returned in predictions;
- ties-to-even and outward floor/ceil behavior.

## Commands

    cargo fmt --manifest-path research/loco-actuator-map-v0/Cargo.toml -- --check
    cargo test --manifest-path research/loco-actuator-map-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/loco-actuator-map-v0/Cargo.toml --locked --all-targets -- -D warnings

Dedicated exact-head workflow: .github/workflows/loco-actuator-map.yml.

## Industry comparator and transfer boundary

Bjelonic, Tischhauser & Hutter (IJRR, first online 31 July 2026) report a physics-grounded PMSM energy model calibrated bottom-up across actuators, airborne trajectories and ground locomotion. They report a 32% full-CoT reduction on ANYmal and transfer to ten additional robots. The publication is an industry reference; this V0 is not a reproduction and makes no performance claim.

- Paper: https://doi.org/10.1177/02783649261459628
- Dataset: https://doi.org/10.3929/ethz-c-000783505
- Public code: https://github.com/leggedrobotics/pace-sim2real

RAIBO2's 2026 marathon/CoT result (0.25) is a full-system hardware co-design reference, not an actuator-map-only or directly matched FlyGym result: https://doi.org/10.1038/s41586-026-11102-5.

## Claim ceiling and next work

Synthetic grids validate interpolation, interval propagation, no-extrapolation and sign arithmetic only. They do not identify a real motor, drive or gearbox. Hardware maps require calibrated torque, angular velocity, voltage/current, temperature and bus voltage; held-out operating points and actuator units must be reserved before fitting. A platform-general transfer claim requires a genuinely held-out unit/platform.

The next step is an independent dataset-backed calibration adapter and held-out transfer campaign, then integration with [LOCO-ENERGY-001 / #4044](https://github.com/Luminous-Dynamics/symtropy/issues/4044) for boundary-resolved energy/CoT receipts. No improvement over the published actuator-energy baseline is claimed by this reference.
