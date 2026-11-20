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

A source observation becomes a validated input only when it:
- matches the source's registered device identity/evidence, gateway, transport, firmware, decoder version/digest and calibration-chain identity;
- carries a digest that matches captured raw frame bytes and a canonical digest over the observation;
- is fresh at the decision cutoff, is not future-dated at that cutoff, and is within source clock-skew/uncertainty bounds;
- has a calibration record valid at both observation time and decision cutoff;
- matches the declared physical unit/range and stays within the uncertainty contract;
- has good source quality and a monotonically increasing sequence within the registered boot epoch.

A source reboot needs a higher boot-epoch ordinal and explicit transition receipt. Gateway ingest order, per-source sequence and source/receive timestamp monotonicity are replay-checked. The event timestamp must not be after the decision cutoff even when its clock skew falls inside the allowed tolerance. Each trusted source registry also stores the active boot epoch and last accepted sequence watermark; the first sample in a batch must advance beyond that persisted checkpoint, preventing cross-batch replay after process restart. The result emits the next sequence/time checkpoint, which the trust registry must persist before another batch is admitted.

## Independence and disagreement

Source count is not the same as independent source count. The evaluator computes the maximum set of pairwise disjoint failure-domain memberships across gateway, network, power and calibration domains. Two sensors sharing any declared failure domain cannot be counted as independent corroboration. If the independent quorum is not met, the signal is `DEGRADED` and unusable. The implementation computes the maximum pairwise-disjoint source set exactly for up to 16 contributing sources; above that bound it fails closed to no independent quorum.

If independently valid source observations disagree beyond the signal-specific tolerance, the signal is `CONFLICT` and unusable. No fused point estimate is emitted. The output envelope is the hull of source measurement intervals, not a statistical confidence interval. The tool does not average conflicting values or choose a majority vote. It reports a conservative envelope across accepted measurement intervals.

## Standards boundary

NMEA 2000 and IEC 61162-450 are transport/interface standards, not evidence of calibration or truth. This project defines only an adapter-neutral envelope and synthetic frame bytes; it does not reproduce licensed PGN definitions, implement a standards-conformant decoder, or claim NMEA/IEC certification. A qualified maritime integrator must validate actual gateway adapters against the licensed standards and hardware.

## Safety and trust boundary

- The output is read-only and never authorizes actuation.
- SHA-256 establishes identity of captured bytes/canonical records, not authenticity of a device, issuer or calibration laboratory.
- Registry identity status and calibration evidence must be provisioned and independently authenticated by the trust/assurance system.
- A `SYNTHETIC_ONLY` pass is a software known-answer test, not a real sensor or vessel result. The current synthetic suite has 20 authored Rust semantic/negative-control tests.
- A stale, conflicting, untrusted, uncalibrated or unknown critical signal is not a valid mission-state update. A separately reviewed degraded-mode policy would need its own explicit age/fallback contract.


## Status semantics

- `QUALIFIED`: every signal contract is met, including freshness, calibration, quality, uncertainty, source quorum and disagreement.
- `DEGRADED/ABSTAIN`: a required quorum is missing or an observation is stale, untrusted, uncalibrated, low quality, or time-uncertain. The affected signal is unusable.
- `CONFLICT/ABSTAIN`: two or more otherwise accepted sources disagree beyond the signal contract. No mean or majority vote is calculated.
- `INVALID`: canonical digest, raw-frame digest, units/range, event ordering, sequence/epoch receipt, timestamp ordering or schema-independent structural validation fails.
- `SYNTHETIC_ONLY`: all required signal contracts pass using synthetic inputs; this is not an operational qualification.

The tool is read-only in every status. Only a separately integrated and qualified state estimator may decide how a `QUALIFIED` receipt is consumed. No fallback/last-known-value policy is implicit.


## Industry interfaces and differentiation

- NMEA's public standards page lists Version 3.000 of NMEA 2000 with amendments and describes its interoperability/PGN scope, including navigation, battery management, engine, alert, AIS/DSC, electric propulsion and environmental domains: https://www.nmea.org/nmea-2000.html
- IEC 61162-450:2024 specifies test/interface requirements for high-speed shipboard navigation and radiocommunication Ethernet networks: https://webstore.iec.ch/en/publication/72731
- PredictWind currently describes eight-model routing and crew-facing briefings, while its comfort-routing help page documents a rerun without requested comfort limits when those limits cannot be met: https://www.predictwind.com/features/ai-weather-router ; https://help.predictwind.com/en/articles/2884622-weather-routing-change-weather-routing-settings

These are the current integration/competitor boundaries. This project does not implement either licensed standard or replace a route planner. Its intended differentiator is the independent evidence decision before a measurement is allowed to update mission state: time, sequence, identity, decoder, calibration, physical unit, uncertainty and failure-domain independence are checked together.
