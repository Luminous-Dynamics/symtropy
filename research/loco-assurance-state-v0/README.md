# LOCO Assurance State V0

Dependency-free Rust reference state machine for deterministic certificate-evidence validity checks immediately before action application.

Research parent: [LOCO-ASSURANCE-001 / #4117](https://github.com/Luminous-Dynamics/symtropy/issues/4117) and [LOCO-SETVAL-001 / #4083](https://github.com/Luminous-Dynamics/symtropy/issues/4083).

## Core separation

The package keeps two predicates distinct:

1. **Physical admissibility:** the independent robust-viability path says the proposed action is admissible.
2. **Assurance validity:** the model, calibration, sensor, actuator and timing evidence required by the certificate remains inside its declared envelope.

An action is permitted only when both gates pass. A rejected physical action cannot be overridden by a valid evidence report. Invalid or incomplete evidence does not prove the robot physically unsafe; it requires revalidation, abstention or a separately qualified fallback.

## Implemented

- Certificate identity, model digest and inclusive validity interval.
- Required sensor IDs bound to exact calibration digests.
- Sensor freshness, health, missing timestamps and OOD status.
- Actuator temperature, supply voltage, tracking error and explicit derating state.
- Compute start/completion, maximum compute latency, action deadline and command age.
- Fixed, explicit units: ns, milli-degrees Celsius, mV and microradians.
- Deterministic sorted/deduplicated reason-coded findings.
- Three assurance statuses: Valid, Unknown and Invalid.
- Independent robust viability gate combined through a fail-closed operational decision.
- Twenty deterministic fault and boundary tests.

## Status semantics

- Valid: all required evidence is present and inside the certificate envelope.
- Unknown: necessary evidence or time ordering is unavailable/uncertain; no permission is granted. When the clock is uncertain, time-derived expiry, freshness and deadline comparisons are suppressed rather than evaluated against untrusted timestamps.
- Invalid: a known assumption mismatch, expiry, unhealthy/OOD/stale sensor, actuator-envelope violation or deadline violation exists.

The gate is evaluated immediately before action application. A delayed command must be rechecked; a previous Valid report must not be reused after the evidence snapshot has changed.

## Verification

    cargo fmt --manifest-path research/loco-assurance-state-v0/Cargo.toml -- --check
    cargo test --manifest-path research/loco-assurance-state-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/loco-assurance-state-v0/Cargo.toml --locked --all-targets -- -D warnings

A dedicated exact-head workflow is defined at .github/workflows/loco-assurance-state.yml. No verification pass is claimed until all three steps succeed on the exact PR head.

## Claim ceiling

This package qualifies only deterministic checks on supplied evidence fields. It does not validate the truth of a sensor's health bit, calibration digest, clock, actuator thermal/voltage bound, physical viability result or model digest. Platform-specific limits need independent identification and validation.

Runtime assurance/Simplex monitoring and fallback are established concepts; this work does not claim them as new. The research contribution being tested is the explicit coupling of certificate lifecycle validity to changing sensor, actuator and timing state, and measuring false-valid, unnecessary-invalid and fallback outcomes.

## Hardware-motivated fault cases

SPINDER reports foot-contact switch false negatives on soft/compliant surfaces, servo deadband/backlash and thermal torque derating after sustained gait. These motivate candidate tests for stale/incorrect sensor evidence and actuator-envelope invalidation, but they do not validate the thresholds in this package.

- SPINDER, Scientific Reports 16, 26147 (2026): https://doi.org/10.1038/s41598-026-56856-0
- Contact sensing survey (2026): https://doi.org/10.1007/s10462-026-11644-0
- Impact-aware robust convex MPC (IEEE RA-L, 2026): https://doi.org/10.1109/LRA.2026.3692086