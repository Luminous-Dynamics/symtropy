# FlyGym Evidence Adapter V0

Standalone, dependency-free Rust evidence gate for [LOCO-FLY-001 / #3875](https://github.com/Luminous-Dynamics/symtropy/issues/3875).

## Pinned upstream target

V0 accepts FlyGym / NeuroMechFly version 2.1.1 only. The v2 API is a breaking rewrite from v1; do not mix v1 controllers, old data schemas or unpinned main-branch output with v2.1.1 trials.

Official references:
- Versioned Simulation API: https://neuromechfly.org/2.1.1/api_reference/flygym/simulation/
- FlyGym sensorimotor model and docs: https://github.com/NeLy-EPFL/flygym/blob/main/docs/index.md
- Version release: https://github.com/NeLy-EPFL/flygym/releases/tag/v2.1.1
- Package release: https://pypi.org/project/flygym/2.1.1/

The documented Simulation API exposes state/control and telemetry methods including step, reset, forward, set_actuator_inputs, get_actuator_forces, get_joint_angles, get_joint_velocities, get_body_positions, get_body_rotations, get_site_positions and get_ground_contact_info. The latter reports six-leg contact-found flags, forces, torques, positions, normals and tangents. The adapter records the source's actual leg order explicitly instead of assuming it matches the internal canonical order.

## Rust-owned evidence boundary

The upstream FlyGym sidecar is responsible only for:
- constructing the exact version-pinned model/world;
- applying the documented simulator control calls;
- sampling the upstream state/contact/actuator APIs;
- exporting typed values, source ordering, units, artifact hashes and action-delivery receipts.

This Rust crate validates the experiment evidence before downstream scoring. It does not execute FlyGym, compute asset hashes, verify signatures, or authenticate that the sidecar reported the truth. All SHA-256 digests must be computed externally and supplied as 32-byte values; this crate rejects empty digests but does not claim cryptographic provenance verification.

## Manifest

Each trial binds:
- FlyGym semantic version (V0 requires exactly 2.1.1) and 40-character source commit;
- MuJoCo version, backend and explicit Warp version for GPU runs;
- physics timestep, control period, timestamp tolerance, solver name/tolerance;
- explicit length, force and torque units;
- model asset, world asset, controller and configuration digests;
- initial-state digest, random seed, telemetry-schema digest.

The control period must be an integral multiple of the physics timestep. Observations must be finite, strictly chronological and aligned to the declared physics clock within the declared tolerance.

## Telemetry contract

The schema preserves the actual upstream six-leg order as a six-element permutation. Each joint channel has a stable ID, leg, axis, position unit and separate velocity unit. Each actuator channel has a stable ID, referenced joint channel, control mode, input unit and output-force unit.

Each telemetry frame captures:
- timestamp;
- joint positions and velocities in schema order;
- actuator inputs and actuator forces in schema order;
- per-leg ground-contact found flag, force, torque, contact position, normal and tangent;
- body position and x/y/z/w orientation quaternion.

Every frame must match the exact schema dimensions. All values must be finite and body quaternion must be nonzero. The adapter preserves the raw contact values even when contact_found is false; consumers must use the flag rather than inventing a new measurement.

## Intervention evidence

Each intervention record includes:
- assigned action ID, assignment time, target legs/channels, requested value and duration;
- value and duration tolerance declared before outcome;
- delivered action ID, actual delivery time, actual value and duration;
- explicit verified flag and nonempty verification-receipt digest.

A trial with an assignment but no delivery receipt fails validation. Unverified delivery, mismatched action IDs, unregistered channels, duplicate targets, excessive latency or delivered value/duration outside declared tolerance are rejected. A failed delivery is not silently counted as an ineffective biological intervention.

## Deterministic tests

15 tests cover:
- acceptance of a complete CPU v2.1.1 baseline trial;
- GPU Warp version requirement;
- verified intervention assignment/delivery;
- rejection of version drift, invalid source SHA and missing artifact digest;
- rejection when a joint channel omits its separate velocity unit;
- physics/control clock consistency;
- exact six-leg permutation and joint/actuator schema integrity;
- unknown actuator-to-joint references;
- telemetry vector dimensions;
- timestamp chronology and physics-grid alignment;
- finite sensor values and valid body quaternion;
- missing/unverified action receipts;
- intervention latency/value/duration tolerances;
- duplicate action IDs, unknown channels and delivery-ID mismatch.

## Industry comparison rules

The adapter is intended to make a fair comparison possible; it does not claim that Symtropy is currently faster or more biologically accurate.

- Pin the exact FlyGym v2.1.1 source and assets. Do not compare with v1 or undocumented main-branch state.
- Separate CPU MuJoCo and GPU Warp runs, and pin the relevant backend/runtime version for each.
- Match morphology, world, initial state, physics timestep, control cadence, sensors, action schedule, random seed and contact/adhesion parameters across controllers.
- Compare native FlyGym CPG, rule-based and hybrid tutorial controllers against translated Symtropy coupled-phase, Hopf and hybrid controllers.
- Add the 2026 proprioceptive DRL-CPG/Hopf hexapod as a paper-level comparator first; executable reproduction requires source, weights and asset pins.
- Publish mechanism-prediction metrics separately from embodied gait metrics and simulator throughput.

Performance metrics must include steps/s, wall time, CPU/GPU, memory, warm-up, render/no-render, control inference p50/p95/p99 and energy. A throughput claim is not a biological-fidelity claim.

## Not implemented

- FlyGym execution or a Python sidecar.
- Binary/JSON parsing or automatic SHA-256 calculation.
- Signed hardware/sensor receipts or cryptographic trust anchors.
- Multi-trial aggregate statistics, paired-seed confidence intervals or power analysis.
- Native CPG/DRL-CPG controller implementations.
- Joint/site schema migration between versions.
- Live-animal experimentation or biological CPG inference.

## Run

    cargo fmt --manifest-path research/loco-flygym-evidence-v0/Cargo.toml -- --check
    cargo test --manifest-path research/loco-flygym-evidence-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/loco-flygym-evidence-v0/Cargo.toml --locked --all-targets -- -D warnings
