# Dredge telemetry validator v0.1

Dependency-free, synthetic-first reference adapter for [DREDGE-TELEMETRY-001 #3309](https://github.com/Luminous-Dynamics/symtropy/issues/3309).

## Run

```sh
cargo test --locked --manifest-path benchmarks/dredge-telemetry-validator/Cargo.toml
cargo clippy --locked --manifest-path benchmarks/dredge-telemetry-validator/Cargo.toml --all-targets -- -D warnings
```

## Scope

- Models Hopper, Mechanical, Pipeline Standard/Small Business and Scow Monitoring/Ullage profile identities.
- Profile sensor requirements are declarative. This crate intentionally does not claim to encode the exhaustive DQM sensor lists or certify DQM protocol conformance.
- Preserves raw sensor observations, timestamps, units, calibration, position datum and quality.
- Uses an independent spatial-fix field (latitude/longitude microdegrees, declared datum and uncertainty); position is not double-counted as a scalar sensor.
- All time ticks and sensor ranges in the fixtures are synthetic, profile-specific values; they are not DQM contract values.
- Distinguishes required sensors from optional fitted sensors. Optional density/velocity values are retained when supplied; missing optional values never become zero.
- Treats scow bin-volume interpretation as unreliable for mounded or unevenly settled loads while retaining the raw ullage/volume observations.
- Detects duplicate/replayed event identity, changed payload digest, sequence gaps, clock issues, stale calibration and profile/configuration mismatch.
- Never creates a dry-mass transfer. A separate qualified mass conversion event must bind direct calibrated mass or characterized wet volume/density/solids-fraction evidence to a lot and sample.

## Claim ceiling

This is a synthetic observation-envelope validator. It is not a DQM implementation or certification, vessel telemetry service, hydrographic datum transformation, production estimator, sediment suitability determination, environmental compliance evaluator, permit or mass ledger. DQM is a USACE programme and contract-specific comparator; use the current governing contract/specification and competent authority requirements for real projects.
