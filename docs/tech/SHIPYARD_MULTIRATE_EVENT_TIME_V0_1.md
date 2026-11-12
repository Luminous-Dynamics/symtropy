# Multirate event-time integrity v0.1

Tracking: [SHIPYARD-MULTIRATE-EVENT-TIME-QUAL-001 #4947](https://github.com/Luminous-Dynamics/symtropy/issues/4947)

## Paradigm

A shared clock coordinate does not mean a shared sample cadence. The system should join samples to physical events using calibrated clock transforms and uncertainty envelopes, while preserving every channel's original sample rate and source timestamps.

This v0.1 library:
- uses integer nanoseconds and checked affine clock transforms;
- binds stream, WID, joint, pass, channel, source-byte digest, transform digest and calibration digest;
- enforces calibration validity intervals, monotonic source timestamps and sequence numbers;
- applies half-open event windows [start, end);
- excludes samples whose uncertainty envelope overlaps a pass boundary;
- separately counts clipped, dropped, invalid, out-of-window, boundary-ambiguous and late-after-cutoff observations;
- enforces per-channel cadence, coverage, maximum gap and event-specific alignment-uncertainty budgets;
- returns an alignment receipt with exact selected sequence numbers;
- does not resample, interpolate, invent missing samples or claim physical measurement accuracy.

## BAM relevance

The BAM companion paper describes UTC timestamps on a TwinCAT 3 Runtime / EtherCAT master clock and reports <10 µs drift for the relevant Beckhoff analog-to-digital terminals. It also identifies different sampling cadences: process current/voltage 0.1 ms; robot TCP coordinates 8 ms; power-source OPC-UA variables 100 ms; control-system power/gas/thermocouples 1 ms; scan profiles 50 Hz; video trigger 40 Hz.

The <10 µs drift statement is not generalized to every upstream transport. The 100 ms OPC-UA channel is not upsampled to 0.1 ms; the 0.1 ms process channel is not downsampled by rewriting its raw samples. Each alignment receipt carries its own stream cadence and uncertainty.

## V0 implementation and tests

- Dependency-free Rust 2024 crate in an isolated nested Cargo workspace.
- Draft 2020-12 input contract.
- 32 unit/mutation tests for identity, positive/negative offset and drift, uncertainty growth, expired calibrations, invalid clock rates, mismatched domains, WID/pass mismatch, duplicate/out-of-order samples, missing digests, half-open boundaries, boundary ambiguity, clipped/dropped samples, late arrival, gaps, insufficient cadence, uncertainty budget, modality-specific expected sample counts, missing transform/calibration digests, invalid event windows, future event cutoffs and jitter-aware expected counts.
- Focused Rust 1.96 workflow runs formatting, tests and Clippy with warnings denied.

The test fixture uses synthetic integer timestamps and symbolic SHA-256 identifiers; no real BAM source bytes are present. It is an event-time contract demonstration, not evidence that physical devices were synchronized to the stated precision.

## Claim ceiling

A PASS validates only the synthetic stream/event-time contract and the deterministic integer mapping. It does not validate actual hardware clock accuracy, sensor calibration, weld quality, NDT, causality or permission to use raw BAM files.
