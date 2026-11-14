# LOCO Actuator Power Map V0

Dependency-free Rust reference for evidence-qualified actuator electrical/mechanical power maps, tracked by [LOCO-ACTUATOR-MAP-001 / #4179](https://github.com/Luminous-Dynamics/symtropy/issues/4179). It extends the boundary-resolved energy ledger in [#4044 / PR #4178](https://github.com/Luminous-Dynamics/symtropy/pull/4178); it does not create a second battery/CoT ledger.

## Paradigm shift: actuator behavior is a calibrated operating envelope, not a constant efficiency

A constant motor-efficiency number cannot represent torque/speed quadrant changes, bus-voltage dependence, thermal derating or uncertainty. V0 evaluates one actuator/drive/gearbox/firmware-specific rectilinear map over four signed/physical axes:

1. signed joint torque (micro-newton-metres, µN·m);
2. signed angular velocity (microradians/second, µrad/s);
3. temperature (milli-degrees Celsius);
4. DC-bus voltage (millivolts).

The map is scoped by actuator, drive, gearbox, firmware, operating-domain and map digests. No universal PMSM constants or efficiency floors are introduced.

## Per-node evidence contract

Every grid node separately preserves:
- unique calibration sample ID;
- input artifact digest;
- calibration digest;
- lower/nominal/upper signed electrical input power (pW);
- lower/nominal/upper signed mechanical joint power (pW).

The nominal mechanical power at each node must equal signed torque × angular velocity under the declared units. Duplicate sample IDs, empty provenance, invalid power intervals and mismatched mechanical nominal values are rejected.

A prediction returns all 16 contributing corner nodes, their source/calibration identities and Q64 interpolation weights. This is provenance exposure, not cryptographic authentication. A production evidence adapter must authenticate the referenced artifacts and bind the caller-supplied map digest to the actual tensor and metadata.

## Bounded interpolation and uncertainty

V0 uses Q16 fractions per axis. The 16 non-negative corner weights sum exactly to 2^64. Electrical and mechanical bounds are interpolated separately. Lower values are rounded outward/down; upper values outward/up; the nominal electrical value uses nearest-ties-to-even.

When a coordinate fraction is not exactly representable in Q16, V0 evaluates the corners of the resulting fraction-quantization box and widens the reported lower/upper bounds. Mechanical interpolation must enclose exact query-coordinate tau*omega, otherwise the lookup fails closed.

Queries outside any calibrated axis return OutsideCalibrationEnvelope. There is no extrapolation, clamping, default temperature, default bus voltage or fallback motor model.

Grid dimensions are bounded (2–16 points per axis and at most 4,096 total nodes) to constrain memory and arithmetic.

## Electrical and mechanical accounting remain separate

At the query point:

    P_mechanical[pW] = torque[µN·m] * angular_velocity[µrad/s]
    P_residual = P_electrical_input - P_mechanical

Electrical input is signed: positive means power enters the actuator electrical boundary; negative means measured/modelled energy is returned across that boundary. V0 derives non-negative electrical draw and electrical return from this electrical interval only.

Mechanical power is signed separately. Positive mechanical power is output delivered to the joint; negative mechanical power is work absorbed by the actuator. Negative mechanical work never creates a claim of electrical regeneration.

The signed electrical-minus-mechanical residual is reported as:
- Consistent: its entire interval is non-negative;
- Uncertain: its interval crosses zero;
- Inconsistent: its entire interval is negative.

This residual is not a decomposition into copper, iron, inverter, gearbox or thermal losses. Those require independently instrumented boundaries. The map does not assert instantaneous passivity from a noisy interval and does not double-count downstream energy from the battery ledger.

## Research comparison and SOTA boundary

Bjelonic, Tischhauser & Hutter (IJRR, first online 31 July 2026) report a physics-grounded PMSM energy model calibrated bottom-up across actuators, airborne trajectories and ground locomotion. They report a 32% full-CoT reduction on ANYmal and transfer to ten additional robots. That is an important comparator; this V0 is not a reproduction and makes no efficiency claim.

- Paper: https://doi.org/10.1177/02783649261459628
- Dataset: https://doi.org/10.3929/ethz-c-000783505
- Public code: https://github.com/leggedrobotics/pace-sim2real

RAIBO2's 2026 marathon/CoT result (reported CoT 0.25) is a full-system hardware co-design reference, not an actuator-map-only or directly matched FlyGym result: https://doi.org/10.1038/s41586-026-11102-5.

The appropriate comparison sequence is:
1. direct measured actuator-terminal V/I at matched sample times;
2. direct joint torque × angular velocity;
3. vendor/nominal constant-efficiency model;
4. calibrated map on held-out points from the same actuator;
5. held-out actuator unit of the same type;
6. held-out platform/actuator family, only when its hardware identity and calibration domain are genuinely unseen.

Report power error by torque/speed quadrant, temperature and bus voltage; interval coverage/width; OOD rejection; energy-integral error; measurement boundary and timing quality; and calibration-data volume. Keep runtime, accuracy, calibration, transfer and energy improvement separate.

## Tests

Twenty deterministic Rust tests cover:
- exact four-axis known-answer interpolation;
- Q1–Q4 signed torque/speed operation;
- negative mechanical work without fabricated electrical regeneration;
- electrical return derived only from signed electrical power;
- strict out-of-envelope rejection on every axis;
- inclusive maximum-axis boundary;
- per-node sample, input-artifact and calibration provenance for all 16 corners;
- duplicate sample rejection;
- mechanical nominal = signed tau*omega validation;
- empty identities and invalid power intervals;
- monotonic/undersized/oversized axis rejection;
- exact grid tensor-size matching and grid resource limit;
- consistent, uncertain and inconsistent loss intervals;
- nearest-ties-to-even and outward rounding;
- negative-power interpolation and signed boundary quadrants.

## Commands

    cargo fmt --manifest-path research/loco-actuator-map-v0/Cargo.toml -- --check
    cargo test --manifest-path research/loco-actuator-map-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/loco-actuator-map-v0/Cargo.toml --locked --all-targets -- -D warnings

Dedicated exact-head workflow: .github/workflows/loco-actuator-map.yml.

## Integration

- LOCO-ENERGY-001 / #4044, exact energy boundary and CoT ledger: PR #4178.
- LOCO-BENCH-001 / #3357, matched controller/body/physics and compute baselines.
- LOCO-CONTACT-INFO-001 / #4004, action-conditioned contact information.
- LOCO-CONTACT-CALIBRATION-001 / #4148, contact probability calibration.

## Claim ceiling and next work

Synthetic grids validate interpolation, interval propagation, no-extrapolation, provenance-retention and sign arithmetic only. They do not identify a real motor, drive or gearbox. Hardware maps require calibrated torque, angular velocity, voltage/current, temperature and DC-bus measurements; synchronized clocks; held-out operating points; and independent actuator units reserved before fitting.

The next step is a dataset-backed adapter for the public PACE/ANYmal actuator-energy data, followed by leakage-resistant held-out operating-point and held-out-actuator transfer evaluation. This V0 makes no claim of outperforming Bjelonic et al., RAIBO2 or any commercial actuator model.
