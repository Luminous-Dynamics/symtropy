# SAFRB usable-service vector qualifier v0.1

**Status:** deterministic contract qualifier prototype; no field or standards qualification  
**Language:** Rust 2024, no external dependencies

## Purpose

A service can have enough energy over an hour while still failing its instantaneous power, voltage, frequency, reactive-power, waveform or process-output requirements. This crate composes typed, upstream-qualified predicate intervals into a non-compensatory service verdict.

It deliberately does not implement any physical solver or verifier:
- #7294/#7337 own bounded energy continuity and sustained restoration.
- #6450 owns the generic electricity trajectory and weighted-kWh scoring.
- #6544 owns native three-phase/sequence/grounding/fault/protection input readiness.
- #6451 owns HIL/commissioning qualification.
- #6317 owns beneficiary floors and robust branch-risk calculations.
- #6405/#6427 own independent byte/signature/role/parent verification.

## Logic

For each profile predicate, the evidence interval is compared with a profile-bound admissible envelope:

- Entire interval inside envelope: GUARANTEED.
- Entire interval outside a hard bound: FAILED.
- Interval overlaps a hard bound: INDETERMINATE.
- Missing, stale, conflicting, unverified or not-assessed evidence: INDETERMINATE.
- NOT_APPLICABLE is valid only when the versioned profile contains an explicit reason.

Composite service logic is non-compensatory:
- any REQUIRED failure => FAILED;
- otherwise any REQUIRED indeterminate => INDETERMINATE;
- only all REQUIRED guarantees => GUARANTEED.

OPTIONAL results are reported separately and cannot compensate for or mask a required predicate. No implicit unit conversion occurs; evidence and profile units must match exactly.

## Typed units

The initial supported canonical units are:
- Energy: Wh
- Active power: W
- Voltage: milliper-unit (mPu)
- Frequency: millihertz (mHz)
- Reactive power: var
- Waveform quality: parts per million (ppm)
- Process output: a profile-defined unit code that must match exactly

The integer representation avoids floating-point rounding in the core interval comparison. It does not establish the quality or correctness of upstream measurements.

## Evidence boundary

The qualifier requires source, digest, provider, qualification and upstream verifier receipt references for evidence declared accepted. It checks their structural presence and identity binding only. It does not resolve or verify bytes, cryptographic signatures, calibration certificates or issuer roles; those remain responsibilities of the independent verifier. A caller cannot treat a self-declared value as physically verified merely because this crate accepted its structural record.

## Run

\`\`\`sh
cargo fmt --manifest-path tools/safrb-usable-service-vector/Cargo.toml -- --check
cargo test --manifest-path tools/safrb-usable-service-vector/Cargo.toml --locked
cargo clippy --manifest-path tools/safrb-usable-service-vector/Cargo.toml --all-targets --locked -- -D warnings
\`\`\`

## Tests

The unit suite covers energy/power non-compensation, voltage/frequency hard failure, interval overlap, missing/stale/conflicting/unverified evidence, source receipt requirements, unit/boundary mismatch, authorized not-applicable profiles, optional predicates, uncertainty widening, duplicate evidence and absolute time-origin translation.

These are deterministic software-contract tests. They do not constitute grid, process, protection, HIL, commissioning or operational validation.
