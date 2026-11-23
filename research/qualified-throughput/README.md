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

Raw simulated ticks/s (including partial work from timed-out runs), qualified ticks/s, case and horizon coverage, every failure/timeout/abstention/not-run count, maximum error ratio among qualified cases, latency p50/p95/p99, peak memory, and energy per qualified tick when a compatible source-bound energy measurement exists.

The evaluator accepts repeated campaigns for one engine ID, but requires each replication to use the same version, executable and solver/configuration digests. It reports per-engine empirical p05/median/p95 qualified throughput and coverage/latency/memory medians; these are descriptive quantiles, not confidence intervals. A project publication target of at least 20 replications is flagged explicitly. The Pareto frontier compares engine aggregates and includes qualified throughput, case coverage, error ratio, p95/p99 latency and memory. Energy is included only if every campaign provides the same energy scope, telemetry source, API version and sampling semantics, with a measurement window exactly matching campaign wall time. Mixed or absent energy never becomes zero.


## Replication and measurement discipline

- A `campaign_id` identifies each repeated measurement; `engine_id` identifies a pinned implementation/profile across replications.
- Keep build and configuration digests identical across replications for one engine ID. If the solver/contact/configuration changes, it is a different engine/profile identity.
- Freeze the full requested case list and per-case simulated horizon. A missing record invalidates the campaign; timeout/failed/abstained/not-run must be explicit.
- The 20-replication flag is a programme publication target, not a claim of statistical sufficiency for every workload. Report the raw campaign samples and use an appropriate predeclared statistical analysis before making inferential claims.
- Empirical p05/p50/p95 are descriptive order statistics; they are not a 95% confidence interval.
- Energy measurement must span the full exact campaign wall-time window. Use a consistent scope, sensor source, API version and sampling semantics across every campaign before including energy in a common frontier.

## Comparison discipline

Keep single-world rigid-body, independent-environment batch and MPM campaigns in separate workload families. Use a common suite digest, case IDs, error envelopes, replay requirements and hardware profile digest. If CPU and GPU systems use different hardware profiles, create separate comparison groups; do not silently mix them.

A backend that does not expose a required observable may still appear in the raw report, but the affected case is not qualified. Use the upstream semantic-equivalence and evidence authorities to decide whether a mapping is valid; do not substitute unavailable fields with zero.

## Claim ceiling

This evaluator validates finite campaign receipt structure and applies caller-supplied gates. It does not authenticate source receipts, validate the numerical truth of reported bounds, assess the physical correctness of an engine, calculate energy from raw telemetry, or prove SOTA performance.

The bundled fixture is synthetic-only test data with invented measurements and placeholder digests. It is not a real benchmark result.
