# PHRI-CONTINUOUS-COUPLING-001 — tactile field reference

Status: deterministic synthetic reference for one flat 32×32 tactile tile. No live sensor, human-subject, clinical or deployed-controller result is claimed.

The reference accepts a complete calibrated normal-pressure field and computes:
- integrated normal force;
- thresholded active contact area;
- center of pressure in a declared 3D surface frame;
- source/tile/frame/profile provenance.

## V0 contract

- 32×32 fixed taxel grid and fixed validity/saturation masks.
- Pressures are already calibrated into pascals; calibration is bound by digest.
- Taxel area, grid pitch, origin and orthonormal surface axes are supplied by a calibration/geometry profile.
- All 1024 taxels must be valid and none may be marked saturated. V0 rejects incomplete/saturated frames; it does not fill missing values or rescale clipped readings.
- Calibration digest, geometry revision digest, measurement-profile digest, tile ID and surface-frame ID must match.
- The clock ID, boot epoch, sequence and timestamp are retained as source provenance. The code does not calculate freshness or AoI; #4713/#4715 and #4117 remain the owners.
- The contact threshold is a calibration/configuration input, not a biological tissue-safety or pain threshold.

## Important boundaries

The output is a geometric/force summary, not a semantic intent class. Pressure or contact does not equal consent. It is not a command, does not select assistance, and does not authorize actuation.

This V0 tile model is planar and reports normal pressure only. It does not estimate shear, slip, curvature, pressure-time tissue exposure, local stress, biological safety or multi-tile global registration. Those require separately calibrated models and physical tests.

The contact field should be used as a continuous interaction observation alongside the wearer's explicit stop/refusal channel, local safety authority, passivity limits and actuator/thermal constraints. A change in contact distribution must never override cancellation or withdrawal.

## Run

    cargo fmt --manifest-path research/phri-continuous-coupling-001/Cargo.toml -- --check
    cargo test --locked --manifest-path research/phri-continuous-coupling-001/Cargo.toml
    cargo clippy --locked --manifest-path research/phri-continuous-coupling-001/Cargo.toml --all-targets -- -D warnings

The 10 authored tests use synthetic known answers and rejection fixtures. They do not qualify an actual tactile sensor or human-contact system.

