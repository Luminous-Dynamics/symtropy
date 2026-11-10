# Symtropy qualified-throughput evaluator — V0

A standalone Rust policy/reporting tool that normalizes simulation throughput by case-level correctness, full horizon completion and workload coverage.

## Run

From this directory:

```sh
cargo test
cargo run -- fixtures/independent-environments.json
```

The CLI reads a finite JSON benchmark campaign and emits a deterministic report. It does not run any physics engine.

## Primary metric

`qualified_simulated_ticks_per_second` counts only the requested simulation horizon of cases that pass every declared gate, divided by the full campaign wall-clock time.

A case qualifies only when:
- it completed;
- its model semantics are explicitly matched;
- its complete requested horizon ran;
- a numerical error upper bound is present and within the case-specific tolerance;
- hard-constraint/invariant evidence is present and within the limit;
- deterministic replay matches when required;
- an upstream evidence receipt digest is present.

Non-completed outcomes, mismatches, partial horizons, missing evidence and out-of-bound results remain visible and contribute zero qualified ticks.

## Reported dimensions

Raw completed ticks/s, qualified ticks/s, case and horizon coverage, every failure/timeout/abstention/not-run count, maximum error ratio among qualified cases, latency p50/p95/p99, peak memory, and energy per qualified tick when a compatible source-bound energy measurement exists.

The Pareto frontier is computed only across the declared single comparison group and uses qualified throughput, coverage, p95/p99 latency and memory. Energy is included only if every campaign in the group provides a measurement of the same scope; mixed or absent energy never becomes zero.

## Comparison discipline

Keep single-world rigid-body, independent-environment batch and MPM campaigns in separate workload families. Use a common suite digest, case IDs, error envelopes, replay requirements and hardware profile digest. If CPU and GPU systems use different hardware profiles, create separate comparison groups; do not silently mix them.

A backend that does not expose a required observable may still appear in the raw report, but the affected case is not qualified. Use the upstream semantic-equivalence and evidence authorities to decide whether a mapping is valid; do not substitute unavailable fields with zero.

## Claim ceiling

This evaluator validates finite campaign receipt structure and applies caller-supplied gates. It does not authenticate source receipts, validate the numerical truth of reported bounds, assess the physical correctness of an engine, calculate energy from raw telemetry, or prove SOTA performance.

The bundled fixture is synthetic-only test data with invented measurements and placeholder digests. It is not a real benchmark result.
