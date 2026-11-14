# Dredge telemetry envelope v0.1

Dependency-free Rust reference validator for [Symtropy #3309](https://github.com/Luminous-Dynamics/symtropy/issues/3309).

## Run

```sh
cargo test --locked --manifest-path benchmarks/dredge-telemetry-envelope/Cargo.toml
cargo clippy --locked --manifest-path benchmarks/dredge-telemetry-envelope/Cargo.toml --all-targets -- -D warnings
```

## What it validates

- Exact plant identity, declared plant-profile kind, profile revision and event payload-digest binding.
- Profile-defined required and optional sensor sets, units, value ranges and data freshness.
- Sensor calibration/currentness, vessel position datum/uncertainty and event clock order.
- Duplicate event identity, changed-payload replays, sequence collisions, out-of-order events and telemetry gaps.
- Idempotent exact replay; exact duplicate records do not advance the cursor twice.
- Pipeline Small Business / Scow Monitoring optional-sensor behavior is represented by test profiles, not claimed as a full DQM profile implementation.

## Critical boundary

The normalized profile is an external contract input. The crate does not encode the complete USACE DQM specification, is not DQM-certified, and does not implement the DQM Web Service or any transport protocol.

The output is an observation-ingestion disposition only. It never produces dry-solid mass, wet-to-dry conversion, dredging quantity, sediment suitability, permit status or an environmental approval. A valid position/density/velocity/ullage observation is still only an observation. Dry-mass conversion requires separate qualified measurement evidence and an explicit mass-balance transfer.

A payload digest supplied by the caller is bound for replay detection; this crate does not calculate or cryptographically verify the digest. Authentication, signature verification, transport security and custody authority remain external.

All tests use synthetic schemas and values; no field performance or DQM conformance is claimed.
