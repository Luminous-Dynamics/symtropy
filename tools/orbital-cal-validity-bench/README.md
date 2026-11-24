# Orbital Calibration Validity Benchmark v0.1

**Benchmark:** ORBITAL-CAL-VALIDITY-001  
**Evidence class:** synthetic  
**Claim ceiling:** synthetic calibration-scope and acquisition-state validity only

This standalone Rust evaluator checks whether each required calibration scope is applicable to the actual acquisition interval and observed component configuration, with a complete change-event inventory and fresh required reference-artifact check.

The v0.1 evaluator does not calibrate instruments, estimate measurement uncertainty, authenticate certificate issuers, certify camera/robot accuracy, prove metrological traceability or authorize a physical inspection. It only determines whether the supplied evidence satisfies this frozen contract. Certificate, check and change-event SHA-256 fields bind the supplied evidence identifiers and are shape-checked; the evaluator does not hash or authenticate external certificate/event bytes.

## Semantics

- Acquisition time is an interval: [center - uncertainty, center + uncertainty].
- Calibration validity uses [valid_from, valid_until). The full acquisition interval must lie inside it.
- A boundary-straddling acquisition is INDETERMINATE.
- A definitely prior invalidating event invalidates only its explicitly affected scopes.
- An event whose time interval overlaps acquisition is INDETERMINATE.
- An incomplete event inventory cannot produce VALID_WITHIN_SCOPE.
- Certificate, reference-check and change-event receipts are bound by 64-hex SHA-256 identifiers. These are references, not cryptographic authenticity verification.
- Required configuration is checked per scope; a mismatch is IDENTITY_MISMATCH and a missing field is INDETERMINATE.
- Reference-check HOLD is DRIFT_SUSPECTED; missing/unknown evidence is INDETERMINATE; stale check evidence is EXPIRED.
- Calibration validity does not imply frustum, visibility, photometric usability, feature measurability, measurement uncertainty or conformity.

## Run locally

```sh
cargo fmt --manifest-path tools/orbital-cal-validity-bench/Cargo.toml -- --check
cargo test --locked --manifest-path tools/orbital-cal-validity-bench/Cargo.toml
cargo clippy --locked --manifest-path tools/orbital-cal-validity-bench/Cargo.toml --all-targets -- -D warnings
cargo run --quiet --locked --manifest-path tools/orbital-cal-validity-bench/Cargo.toml --bin orbital-cal-validity-bench -- docs/research/orbital-truss/fixtures/calibration-validity-v0_1.json <40-hex-source-commit>
python3 tools/orbital-cal-validity-bench/tests/reference_oracle.py docs/research/orbital-truss/fixtures/calibration-validity-v0_1.json <40-hex-source-commit>
```

See [#7339](https://github.com/Luminous-Dynamics/symtropy/issues/7339) and the [contract protocol](../../docs/research/orbital-truss/calibration-chain-validity-v0_1.md).
