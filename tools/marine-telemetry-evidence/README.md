# Marine Telemetry Evidence Gateway

Standalone Rust 1.96-compatible read-only evidence qualifier for Symtropy #6315.

## Command

```sh
cargo fmt --manifest-path tools/marine-telemetry-evidence/Cargo.toml -- --check
cargo clippy --manifest-path tools/marine-telemetry-evidence/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path tools/marine-telemetry-evidence/Cargo.toml
cargo run --manifest-path tools/marine-telemetry-evidence/Cargo.toml -- audit-telemetry tools/marine-telemetry-evidence/fixtures/synthetic-qualified-telemetry-0001.json
cargo run --manifest-path tools/marine-telemetry-evidence/Cargo.toml -- audit-telemetry tools/marine-telemetry-evidence/fixtures/synthetic-common-cause-not-independent-0001.json
```

## Evidence contract

A signal only qualifies when its observation:
- matches the source's registered identity, gateway, transport, firmware and decoder version/digest;
- carries a digest that matches captured raw frame bytes and a canonical digest over the observation;
- is fresh at the decision cutoff and within source clock-skew/uncertainty bounds;
- has a calibration record valid at both observation time and decision cutoff;
- matches the declared physical unit/range and stays within the uncertainty contract;
- has good source quality and a monotonically increasing sequence within the registered boot epoch.

A source reboot needs a higher boot-epoch ordinal and explicit transition receipt. Gateway ingest order and per-source sequence are replay-checked.

## Independence and disagreement

Source count is not the same as independent source count. The evaluator computes the maximum set of pairwise disjoint failure-domain memberships across gateway, network, power and calibration domains. Two sensors sharing any declared failure domain cannot be counted as independent corroboration. If the independent quorum is not met, the signal is `DEGRADED` and unusable.

If individually qualified sources disagree beyond the signal-specific tolerance, the signal is `CONFLICT` and unusable. The tool does not average conflicting values or choose a majority vote. It reports a conservative envelope across accepted measurement intervals.

## Standards boundary

NMEA 2000 and IEC 61162-450 are transport/interface standards, not evidence of calibration or truth. This project defines only an adapter-neutral envelope and synthetic frame bytes; it does not reproduce licensed PGN definitions, implement a standards-conformant decoder, or claim NMEA/IEC certification. A qualified maritime integrator must validate actual gateway adapters against the licensed standards and hardware.

## Safety and trust boundary

- The output is read-only and never authorizes actuation.
- SHA-256 establishes identity of captured bytes/canonical records, not authenticity of a device, issuer or calibration laboratory.
- Registry identity status and calibration evidence must be provisioned and independently authenticated by the trust/assurance system.
- A `SYNTHETIC_ONLY` pass is a software known-answer test, not a real sensor or vessel result.
- A stale, conflicting, untrusted, uncalibrated or unknown critical signal is not a valid mission-state update. A separately reviewed degraded-mode policy would need its own explicit age/fallback contract.
