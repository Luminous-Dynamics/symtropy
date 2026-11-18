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

## Synthetic waveform replay (S0.2)

The `waveform` module adds deterministic sample-level transformations for explicit synthetic input arrays:

- Dropout, integrity failure and stale calibration remove numeric values and retain unavailable/invalid states.
- Delay references an earlier source sample; initial samples without a past source remain value-less and delayed.
- Saturation clamps to declared finite bounds.
- Bias adds a declared finite offset.
- Gain drift linearly interpolates a positive gain factor across a half-open interval.
- Fault intervals on the same waveform must not overlap; disjoint/intermittent intervals are supported.
- Fault schedules are canonicalized by sample interval and ID before hashing, so reordering independent schedule entries does not change the fault-schedule digest.
- The output receipt binds the input-series digest, canonical fault-schedule digest, sample timebase, per-sample source index and non-empty fault identity.

This is a deterministic software perturbation of synthetic vectors, **not** an instrument model. The declared saturation/bias/gain parameters are challenge parameters, not fitted physical calibration values. No noise model, ADC behavior, detector transfer function, plasma kinetics, or inverse reconstruction is implied. All output samples and receipts remain synthetic.

Example API shape:

```rust
use plasma_diagnostic_fault_replay::waveform::{
    run_waveform_replay, WaveformFault, WaveformFaultMode, WaveformRequest,
    WAVEFORM_SCHEMA_VERSION,
};

let request = WaveformRequest {
    schema_version: WAVEFORM_SCHEMA_VERSION.to_owned(),
    scenario_id: "synthetic-shot-001".to_owned(),
    channel_id: "density".to_owned(),
    units: "synthetic-unit".to_owned(),
    time_origin_ns: 0,
    sample_interval_ns: 1_000,
    input_values: vec![0.1, 0.2, 0.3, 0.4],
    faults: vec![WaveformFault {
        fault_id: "synthetic-bias".to_owned(),
        start_sample: 1,
        end_sample: 3,
        mode: WaveformFaultMode::Bias { offset: 0.05 },
    }],
};
let receipt = run_waveform_replay(&request)?;
```

### Frozen waveform corpus

`benchmarks/plasma/waveform-fault-replay-v0.json` contains eight deterministic cases: nominal input, dropout, integrity failure, delay, saturation, additive bias, gain drift and stale calibration. The waveform unit-test audit checks each sample's expected state, optional value, source index and active fault identity, then verifies every receipt remains synthetic.

The corpus uses hand-authored vectors and declared perturbation parameters. It is a regression fixture for the transformation implementation, not a statistically representative instrument-failure dataset and not a substitute for the TokaMark robustness benchmark.

## Symthaea observation-frame export

The `observation_export` module converts a verified waveform receipt into a sequence of wire-compatible Symthaea `PlasmaObservationFrame` v0.1 records. It is deliberately **synthetic-only**:

- Numeric values from `synthetic_nominal`, `degraded`, and value-bearing `delayed` samples map to the `Synthetic` disposition.
- Dropout and delayed samples without an earlier source map to `Unavailable`.
- Integrity failure and stale-calibration samples map to `Invalid`.
- The adapter never emits `Measured`, `Delayed`, or `Degraded` measurement dispositions because the synthetic replay has no physical source/calibration evidence.
- The outer envelope preserves the original sample state, active fault ID, source sample index, sample timestamp, and waveform receipt digest.
- The export itself remains `SYNTHETIC_WAVEFORM_FAULT_REPLAY`, `physical_measurements=false`, with its fixed synthetic-only claim ceiling.

The JSON frame shape matches Symthaea's observation schema for interchange. It is not a source-authentication mechanism and does not prove the synthetic values are physically plausible. Downstream consumers must still apply the normal evidence gate; synthetic frames must abstain at a direct-measurement gate.

Tests cover state mapping, leading delay without a source sample, serialization round-trip, receipt tampering, and attempted measured-state/physical-measurement promotion.
