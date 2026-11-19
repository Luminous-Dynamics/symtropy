# plasma-stream-telemetry

Offline Rust replay auditor for high-rate plasma diagnostic stream timing, clock calibration, sequence integrity, queue pressure, deadlines and model identity.

## Run

    cargo fmt --manifest-path crates/tools/plasma-stream-telemetry/Cargo.toml -- --check
    cargo test --locked --manifest-path crates/tools/plasma-stream-telemetry/Cargo.toml
    cargo clippy --locked --manifest-path crates/tools/plasma-stream-telemetry/Cargo.toml --all-targets -- -D warnings
    cargo run --locked --manifest-path crates/tools/plasma-stream-telemetry/Cargo.toml -- benchmarks/plasma/stream-telemetry-v0.json

## Time semantics

Every timestamp identifies its source clock domain and calibration ID. A calibration maps one device clock into the trace's declared reference clock using an anchor pair, drift in ppb, a validity interval and explicit uncertainty. The auditor never subtracts timestamps from different clocks directly.

Stage latency is emitted only when both stamps map to the same reference and combined uncertainty remains within the declared budget. Invalid, expired, missing or incompatible clock calibrations produce CLOCK_UNQUALIFIED and no fabricated latency.

All times are integer nanoseconds. The calibration affine map is:

    t_ref = t_ref_anchor + (t_clock - t_clock_anchor)
            + (t_clock - t_clock_anchor) * drift_ppb / 1e9

Each stream trace is replayed in observed receive order. Events are never sorted to hide duplicates, gaps or reordering. Sequence gaps are explicit degradation; duplicates and out-of-order sequence numbers remain distinct dispositions.

## Metrics

The report separates:
- transport latency = receive - acquisition;
- queue wait = processing start - receive;
- processing latency = processing end - processing start;
- acquisition-to-decision latency;
- source-event age at decision;
- acquisition-relative deadline status;
- source sample period, clock uncertainty, queue depth, sequence gaps and optional downstream acknowledgement state.

A late historical sample can remain valid evidence while being stale for the live decision. Model identity is independently qualified. An unknown downstream acknowledgement remains unknown and never becomes success.

## Boundaries

This is telemetry/replay qualification only. It is not a disruption classifier, controller, FPGA runtime, PCS interface, hardware capture tool or actuator authority. Reuse Symtropy #6190 for native timestamp alignment and #1685 for solver run receipts. The report contains no action-command or actuator decision semantics.
