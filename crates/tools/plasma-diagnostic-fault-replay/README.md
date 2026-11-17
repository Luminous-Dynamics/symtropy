# plasma-diagnostic-fault-replay

Standalone, offline Rust S0 evaluator for deterministic synthetic diagnostic-fault schedules and replay receipts. It is a scenario/replay tool, not a plasma solver and not a diagnostic reconstruction model.

## Commands

From the repository root:

    cargo test --locked --manifest-path crates/tools/plasma-diagnostic-fault-replay/Cargo.toml
    cargo run --locked --manifest-path crates/tools/plasma-diagnostic-fault-replay/Cargo.toml -- benchmarks/plasma/diagnostic-fault-replay-v0.json
    cargo clippy --locked --manifest-path crates/tools/plasma-diagnostic-fault-replay/Cargo.toml --all-targets -- -D warnings

The crate has its own Cargo workspace boundary and does not join the Symtropy root workspace. The evaluator performs no network calls and launches no external solver.

## S0 contract

- Deterministic scenario, schedule, timepoint and receipt SHA-256 identities.
- Channel-level and diagnostic-system-level fault targets.
- Explicit synthetic_nominal, unavailable, invalid, delayed and degraded channel states.
- Half-open fault windows: [start_time_ns, end_time_ns).
- Refusal on unknown targets, duplicate identities, invalid windows, out-of-range faults, non-monotonic timepoints and ambiguous overlapping faults.
- Deterministic canonicalization of channel and fault arrays before hashing.
- Receipt verification recomputes the receipt digest and its state-count summary.
- Every receipt remains SYNTHETIC_FAULT_REPLAY, physical_measurements=false, with a fixed claim ceiling.

## Important boundary

The fixture does not contain plasma measurements, reconstruct diagnostic values, estimate sensor failure rates, model instrument physics, or demonstrate disruption prediction. Fault injection patterns are software challenge inputs only. A valid replay receipt proves only the recorded scenario/fault identity and replay-contract consistency. SHA-256 here is an integrity identifier, not a digital signature or proof of author identity. It does not establish diagnostic accuracy or physical truth.

## Fault-mode interpretation

The replay engine currently maps a declared fault mode to an evidence disposition (unavailable, invalid, delayed or degraded) at selected timepoints. It does **not** synthesize corrupted waveform values, sensor noise distributions, acquisition electronics, or diagnostic reconstruction. In particular, saturation, bias and gain drift are represented as a degraded-channel state, not by numerical clipping or signal transformation. The corpus includes temporal schedules shaped around public MAST/TokaMark robustness scenarios, but remains a software fault-disposition benchmark only.
