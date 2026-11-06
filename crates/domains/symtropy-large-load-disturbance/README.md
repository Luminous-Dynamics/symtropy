# Symtropy Large-Load Disturbance Conformance

Trace-based conformance checks for operator-defined PCC voltage/frequency ride-through, grid-connection continuity and post-disturbance active-power recovery.

## Paradigm

A demand reduction caused by an uncommanded breaker trip or ride-through failure is not intentional flexibility. This crate keeps disturbance performance separate from dispatch flexibility and evaluates observable PCC RMS traces against a pinned requirement set.

## v0.1 functionality
- Pinned operator requirement ID, version, jurisdiction, source digest and PCC.
- Piecewise contiguous voltage/frequency envelope segments with operator-specified RoCoF bounds and connection behavior.
- Explicit stay-connected and momentary-cessation zones with an allowed disconnected-duration bound.
- Waveform digest, measurement-chain digest, clock model, timestamp uncertainty, RMS window and measurement bandwidth binding.
- Initial event-start coverage, maximum inter-sample gap and Nyquist consistency between the sample cadence and claimed measurement bandwidth.
- Chronology, sample-gap, observation-horizon and measurement-quality checks with fail-closed abstention.
- Active-power recovery after declared stable-voltage/frequency dwell, with recovery deadline, minimum pre-event-power fraction, maximum absolute ramp and overshoot.
- Optional operator-defined reactive-power range and maximum PCC current envelope; missing required Q/current samples cause abstention rather than being silently ignored.
- Explicit intentional-demand-response credit is false for every result.

## Limits
This evaluates RMS-like PCC observations; it does not simulate converter internals, positive-sequence dynamics, AC network stability, EMT/PDT, harmonics, unbalance, protection or fault current. Caller-supplied receipt IDs/digests are provenance labels, not cryptographic authentication. Requirements must be provided by the relevant operator; no universal grid-code curve is hardcoded.

## Run
```sh
cargo test -p symtropy-large-load-disturbance
cargo clippy -p symtropy-large-load-disturbance --all-targets -- -D warnings
```
