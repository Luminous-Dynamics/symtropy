# Inference Power–Service Flexibility — Synthetic Known-Truth Fixtures

**Fixture class:** synthetic known truth  
**Scope:** matched inference requests, per-request QoS, boundary power, energy efficiency and sustained response  
**Not:** production GPU control, real Argonne/MLPerf data, facility telemetry, dispatch proof or market accreditation

## Purpose

These fixtures distinguish energy efficiency from controllable grid flexibility and require useful work and service quality to be preserved while power changes are measured at a declared boundary.

## Known-truth cases

- Same four 100-token obligations, five 100 ms intervals, baseline boundary power 120 W and candidate [120, 100, 100, 100, 100] W: 20 W maximum reduction, 400 ms sustained interval from 100 ms, 0.0022222 Wh gross reduction, no rebound. Baseline energy is 0.0166667 Wh and candidate energy 0.0138889 Wh.
- Same useful requests and QoS yield 400 useful tokens. Useful tokens/J improve from 6.6667 to 7.6923 at the declared boundary; this is an efficiency result separate from the 20 W response envelope.
- A brief 200 ms reduction followed by rebound is reported but does not meet the 300 ms minimum sustained-response duration.
- A 200 ms reduction followed by 200 ms of rebound yields equal gross and rebound energy and zero net energy reduction. No net efficiency claim is permitted.
- p99 latency regression and useful-token loss block service equivalence even if boundary power falls.
- The declared boundary meter is authoritative; GPU/CPU/DRAM submeter values are retained separately and are not summed into it.

## Method

Request obligations are frozen across paired runs. Each request must be present in both arms and includes expected useful output tokens. Results provide success, actual output tokens, TTFT and completion latency. Power traces share an exact contiguous time axis. Per-request quantiles use nearest-rank p95/p99. Hard service gates and relative p99 regression limits are evaluated before flexibility qualification.

Power is measured from the explicit boundary meter in watts and integrated over exact interval duration to Wh. The response envelope requires a minimum signed boundary reduction for both a minimum consecutive sample count and minimum elapsed sustained duration. Negative reductions are preserved as rebound.

## Validation status

Fourteen Rust unit tests are authored in `inference_power_flexibility.rs`; fixture JSON contains eleven synthetic known-truth/negative-control cases. They are not represented as executed or passing. Rust build, rustfmt, Clippy and tests must be run on the exact PR head using a Rust toolchain.

## Useful-work and rebound hardening

Useful tokens/sec and tokens/J use completed useful output tokens, capped at the per-request obligation. Output from a failed request remains diagnostic output tokens but contributes zero completed useful work. The minimum-throughput and throughput-retention gates apply to useful tokens/sec, so excess output cannot inflate the service contract.

The response contract independently bounds maximum rebound power and rebound energy. A response can meet the minimum reduction, sample-count and sustain-duration requirements yet remain unqualified when its later rebound exceeds either owner-defined limit. The new fixture includes a 300 ms qualified reduction followed by 20 W of rebound over 100 ms; it is measured but rejected by the rebound envelope.

The stabilization timestamp is the time at which the minimum response sample-count and elapsed-duration requirements are first both met, not the start of the qualifying window.

The response contract also includes explicit maximum rebound power and energy. A sustained response followed by excessive rebound is measured but fails power-response qualification. Failed requests may retain partial output tokens in the raw diagnostic count; those tokens are excluded from completed useful work and useful tokens/J.
