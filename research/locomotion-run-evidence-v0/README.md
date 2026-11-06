# Locomotion Run Evidence V0

Standalone Rust schema and fail-closed comparison gates for issue #3889, LOCO-PLATFORM-001.

## Purpose

A simulator comparison is not valid merely because two CSV files have similar columns. This crate defines a versioned, engine-neutral provenance manifest that records what produced each trajectory and checks whether two runs satisfy a declared paired-comparison contract.

The manifest does not compute SHA-256 hashes itself. The caller supplies SHA-256 digests of immutable assets, configurations, receipts and raw trajectories. The canonical_bytes() method creates an unambiguous deterministic length-prefixed encoding for downstream hashing/signing; it is not itself a digest, signature or authenticity proof.

## Evidence record

RunManifest records:

- Evidence tier: synthetic, simulated or hardware.
- Engine name, version, backend, source commit and engine-native configuration digest.
- Controller name/digest.
- Morphology/body-asset name/digest.
- Engine-neutral physical-contract digest covering declared mass/inertia/gravity/contact/material semantics.
- Sensor and actuator configuration digests.
- Terrain identity/digest, terrain seed and trial seed.
- Initial-state digest.
- Action/intervention assignment digest, intervention-presence bit and optional per-run delivery-receipt digest.
- Raw trajectory digest and metric-version identifier.
- Timestep, control frequency, sensor frequency, declared sensor/actuator latency, integrator, solver name/tolerance/iteration count, duration and recorded sample count.
- Optional hardware profile: platform, CPU, accelerator, OS build, driver and calibration-record digest.

An intervention requires a delivery receipt. Hardware-tier evidence requires a hardware profile and calibration-record digest. Placeholder or malformed lowercase SHA-256 hex strings, malformed Git object IDs, nonpositive timebases and underspecified sample counts fail closed.

## Paired-comparison contract

validate_paired_comparison(left, right) permits engine name/version/backend, engine-native config, run ID, per-run receipt, raw trajectory digest and hardware profile to differ. These are the fields that describe engine-specific execution and the resulting record.

It requires these fields to match:

- Evidence tier.
- Controller identity and digest.
- Morphology/body asset identity and digest.
- Engine-neutral physical contract.
- Sensor and actuator configurations.
- Terrain identity/digest and seeds.
- Initial-state digest.
- Action assignment and intervention presence.
- Metric version.
- Timestep, control frequency, sensor frequency, sensor/actuator latency, duration and sample count.

This is deliberately strict. If the engines cannot implement an identical sensor/actuator/physics contract, the comparison must be declared non-paired or the unsupported difference must be surfaced explicitly. Do not silently substitute a different contact model, sensor, actuator, delay, control frequency or asset and still label the result paired.

Delivery receipts are independently required and bound for each run rather than byte-equal: their internal formats may be engine-specific. A higher-level adapter must verify that both receipts correspond to the same assigned action and declare any differences in actual applied timing, magnitude, latency or fidelity. The manifest gate alone does not authenticate those receipts.

## Current scope and limitations

V0 is a schema/validation substrate, not a simulator adapter or a cryptographic signing service. It does not:
- read or hash files;
- validate the semantic contents of a physical contract;
- inspect the actual trajectory;
- authenticate action-delivery receipts;
- calculate biomechanical/controller metrics;
- guarantee that two engines have numerically equivalent solvers;
- certify sim-to-real transfer or biological fidelity.

Those responsibilities belong to independent adapters, immutable artifact stores, metric implementations and evidence verifiers. An accepted manifest means only that the declared metadata is complete and pair-compatible under schema v1.

## Tests

14 deterministic tests cover canonical-encoding stability, cross-engine pair acceptance with engine-specific solver disclosures, physical/sensor/timebase/seed/latency mismatch rejection, action delivery-receipt requirements, hardware profile and calibration requirements, malformed digests/commit IDs, invalid solver settings, invalid timebase/sample count, missing required strings and unknown schema versions.

## Run

    cargo fmt --manifest-path research/locomotion-run-evidence-v0/Cargo.toml -- --check
    cargo test --manifest-path research/locomotion-run-evidence-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/locomotion-run-evidence-v0/Cargo.toml --locked --all-targets -- -D warnings
