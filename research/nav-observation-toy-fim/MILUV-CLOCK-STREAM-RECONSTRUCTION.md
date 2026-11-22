# MILUV clock-stream reconstruction contract

**Status:** standalone deterministic reconstruction kernel added; physical host-time bounds, oscillator-rate bounds and radio boot-epoch lineage are not yet qualified from MILUV payloads.

**Parent:** DS-TWR timestamp adapters in [PR #6815](https://github.com/Luminous-Dynamics/symtropy/pull/6815).

## 1. Why this is a separate problem

The per-transaction adapter can unwrap the three timestamps recorded by each radio inside one DS-TWR exchange, provided that the maximum intra-exchange event gap is known to be below one uint32 wrap.

That is not enough to reconstruct a radio's counter across an experiment. Long periods between transactions may span one or multiple wraps. A negative raw-counter difference alone cannot identify the exact number of wraps, and a radio reset can look like a wrap.

The new `miluv_clock_stream` module therefore reconstructs a sequence of observations for one radio clock and one stable event semantic. It accepts a wrap count only when exactly one integer count is consistent with independently bounded host acquisition time and oscillator rate.

## 2. Explicit stream identity

Every observation includes:

- `clock_id`: physical radio/tag identity;
- `event_kind`: one stable semantic such as initiator `tx1`, initiator `rx2`, responder `rx1`, etc.;
- `host_time_domain_id`: identity of the monotonic host acquisition clock;
- `boot_epoch`: externally established radio boot/reset epoch;
- `source_sequence`: strictly increasing transaction/event sequence;
- `host_time_ns` and a hard absolute error bound;
- original uint32 `raw_ticks`.

One stream cannot mix radio identities, event kinds, host-clock domains or boot epochs. Equal host-time stamps are permitted only if the supplied uncertainty still yields exactly one possible raw-clock advance.

A repeated `boot_epoch` value is not proof that the radio did not reset. The caller must obtain a reliable boot counter/uptime/status, establish continuity by independent evidence, or split/quarantine the stream when reset status is unknown.

## 3. Exact integer interval calculation

The reference DW1000 tick rate is:

- (499.2,	ext{MHz} 	imes 128 = 63{,}897{,}600{,}000) ticks/s;
- (63.8976) ticks/ns;
- (638{,}976/10{,}000) ticks/ns exactly under the documented nominal parameter.

For adjacent samples, let host timestamp centres be (h_{i-1},h_i), and their hard absolute error bounds be (e_{i-1},e_i). The possible elapsed host time is:

[
H_L=max(0,h_i-h_{i-1}-e_i-e_{i-1})
]

[
H_U=h_i-h_{i-1}+e_i+e_{i-1}
]

For a declared oscillator rate tolerance (ho) ppm, the possible integer tick advance is bounded by:

[
N_L=leftlceil H_L rac{638976}{10000}
left(1-rac{ho}{10^6}ight)ightceil
]

[
N_U=leftlfloor H_U rac{638976}{10000}
left(1+rac{ho}{10^6}ight)ightfloor
]

The implementation uses checked (u128) integer arithmetic and the exact rational tick rate; it does not make the wrap decision using floating-point rounding.

Let:

[
d_0=(r_i-r_{i-1})mod 2^{32}
]

The candidate advances are (d_0+k2^{32}), for integer (kge0), that lie in ([N_L,N_U]).

- Exactly one candidate: accept and record its inferred wrap count.
- No candidate: return `NoConsistentClockAdvance`.
- More than one candidate: return `AmbiguousClockAdvance`.
- Host gap beyond the declared continuity horizon: return `HostTimeGapTooLarge`.

The stream never chooses the nearest wrap, assumes one wrap per observed decrease, or bridges a declared excessive gap.

## 4. The assumptions are inputs, not hidden defaults

Two independent bounds are required:

### Host acquisition-time error

`host_time_abs_error_ns` must include more than timestamp quantization. It must conservatively bound the difference between the recorded host acquisition time and the radio-clock event time, including variable publisher scheduling, transport and message-processing latency.

The MILUV reference code uses ROS `time` for its experiment-level clock unwrapping. That does not by itself establish a hard upper bound on this event-to-host latency. The exact source semantics and empirical worst-case timing error must be verified before admitting these bounds.

### Oscillator-rate error

`clock_rate_tolerance_ppm` must cover the radio oscillator's rate deviation over the relevant operating conditions. It should be derived from the device specification and, where available, independently checked against clock-skew observations and calibration. The current module validates a configurable maximum of 100,000 ppm, but does not supply a default or claim that any particular value is qualified for MILUV.

### Radio boot/reset epoch

A counter reset, reboot or power cycle is not safely distinguishable from a wrap using the raw counter alone. The caller must source the boot epoch from reliable radio status or segment the stream at any possible reset. The current module rejects an explicitly changed epoch; it cannot detect a reset that the input lineage failed to identify.

## 5. Tests added

The module has deterministic synthetic tests for:

1. Exact nominal tick-rate reconstruction.
2. A single uint32 wrap.
3. Multiple wraps across a long gap when exactly one count is possible.
4. Rejection of an ambiguous multiple-wrap interval.
5. Rejection of host/radio timing inconsistency.
6. Equal host stamps with uncertainty sufficient for a unique result.
7. Mixed radio identity.
8. Mixed event semantics.
9. Mixed host time domains.
10. Changed boot epoch.
11. Non-increasing source sequence and decreasing host time.
12. Excessive host-time gap and invalid configuration.

These are equation/contract tests, not MILUV data results.

## 6. Remaining integration work

- Verify exact meanings and timing of MILUV ROS/header timestamps for every exported observation type.
- Establish a conservative event-to-host capture-latency bound per event semantic and experimental setup.
- Establish oscillator-rate tolerance bounds from device specifications and clock-skew data.
- Verify boot/reset lineage or explicitly segment uncertain streams.
- Select one stable event semantic per stream; do not combine `tx1`, `rx2`, `rx1`, etc. into a single sequence.
- Join the stream-level absolute wrap history with each transaction's local event offsets without changing the per-exchange DS-TWR durations.
- Compare against the authors' `PostProcess._unwrap_all_clocks` output on known, rights-cleared source-complete samples, retaining disagreements and ambiguous cases.
- Keep the truth-conditioned row-filter blocker [#6813](https://github.com/Luminous-Dynamics/symtropy/issues/6813) in force. A filtered processed CSV is not a source-complete controller stream.

## 7. Claim boundary

This is a deterministic integer interval algorithm and synthetic test suite. It is not a validated MILUV ROS timestamp model, oscillator calibration, reset detector, full dataset adapter, physical range result, or navigation integrity qualification. Until the two timing bounds and boot-epoch lineage are independently justified, the correct output for a real stream is **unqualified / do not unwrap**, not a guessed timestamp history.
