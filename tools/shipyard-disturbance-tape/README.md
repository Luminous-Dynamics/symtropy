# Shipyard disturbance tape validator

Status: research prototype; synthetic fixture only. This crate does not establish
yard physics, feasible schedules, optimality, industrial uplift, or production
readiness.

## Build and test

The crate intentionally has no third-party dependencies and supports offline
testing with Rust 1.96:

    cargo test --locked --offline --manifest-path tools/shipyard-disturbance-tape/Cargo.toml
    cargo clippy --locked --offline --manifest-path tools/shipyard-disturbance-tape/Cargo.toml --all-targets -- -D warnings

## CLI

Pass the exact SHA-256 of the input bytes, a single decision tick, and distinct
output paths:

    sha256sum tools/shipyard-evaluation-lab/fixtures/synthetic-disturbance-tape-v0.json
    cargo run --locked --offline --manifest-path tools/shipyard-disturbance-tape/Cargo.toml -- validate tools/shipyard-evaluation-lab/fixtures/synthetic-disturbance-tape-v0.json <expected-sha256> 5 /tmp/policy-snapshot.json /tmp/evaluator-stream.json

The policy snapshot contains only policy-visible events whose delivery tick is
less than or equal to the requested tick. It contains neither evaluator truth
nor evaluator-only events. It is a snapshot for one decision epoch, not a
stream that should be reused at later ticks. A caller must re-run the validator
for each decision tick.

The evaluator stream contains all events, truth payloads, and the source digest;
keep it outside the policy process. The parser rejects duplicate object keys,
unknown fields and event types, unsupported contract versions, malformed or
out-of-range integer ticks, duplicate event IDs, digest mismatch, invalid event
payloads, and inconsistent visibility/observation values. The SHA-256 is over
exact input bytes; no whitespace or object-key normalization is performed
before hashing.

## Scope boundary

The CLI checks tape structure and causal visibility boundaries only. It is not
a runtime sandbox, simulator, geometric checker, optimizer, or industrial
calibration layer. The output-path split is a protocol boundary, not OS-level
access control. Production use needs an orchestrator that controls which file
or object is mounted into the candidate process.
