# Evidence-gated shipyard digital-twin commissioning v0.1

Tracking: [SHIPYARD-DT-COMMISSIONING-001 #4671](https://github.com/Luminous-Dynamics/symtropy/issues/4671)

## Standards boundary

ISO/AWI 23247-7:2026 is an approved work item under development. Its published ISO project page describes commissioning activities covering initialization/configuration, digital-thread connections, frequency/latency/fidelity requirements, operational readiness and cooperation among twins. It is not yet a published standard. This repository benchmark is aligned conceptually; it makes **no ISO conformance claim**.

## What v0.1 implements

A dependency-light Rust evaluator commissions a synthetic welding-cell twin over five heterogeneous streams:
- current/voltage (1 tick interval; synthetic analogue of the published 0.1 ms stream),
- robot TCP (80 ticks),
- power-source OPC-UA (1,000 ticks),
- scan profiles (200 ticks),
- video triggers (250 ticks).

A tick is 0.1 ms in the frozen fixture, but objective values remain synthetic test units. These are not raw BAM time-series records and are not claims of actual deployed telemetry performance.

For every stream, the evaluator verifies:
- exact stream, owner-twin, schema/version and clock identity;
- synthetic-only evidence admission;
- source timestamp order and interval jitter;
- ingestion timestamp order relative to source timestamp;
- p50/p95/p99/max ingestion latency;
- latest observation age;
- clock offset and drift budgets;
- calibration identity, digest and expiry;
- source and transformation receipt digests;
- fidelity residual and measurement coverage.

At the twin level it verifies unique required streams, exact observations for every required stream, no unexpected observations, configuration digest and permitted capabilities. The output is a deterministic report with per-stream metrics and violations. All streams must declare the twin-wide master clock. A hard-coded capability ceiling prevents the input manifest from granting itself command authority; even a manifest that lists an elevated action as permitted is rejected. Maximum authority is read-only telemetry; no robot command, WPS modification, acceptance or release capability is granted.

## Files

- `crates/tools/shipyard-digital-twin-commissioning`: Rust 2024 evaluator and CLI in its own nested Cargo workspace.
- `docs/tech/SHIPYARD_DT_COMMISSIONING_V0_1.schema.json`: Draft 2020-12 input schema.
- `benchmarks/shipyard/digital-twin/commissioning-valid-v0.1.json`: five-stream synthetic positive fixture.
- `.github/workflows/shipyard-digital-twin-commissioning.yml`: pinned Rust 1.96 formatting, tests, Clippy and fixture execution.

The crate is isolated from the root Cargo workspace to avoid merge coupling with the other shipyard research PRs.

## Benchmark targets

- 100% agreement with the frozen finite-corpus oracle.
- Zero false commissioning across seeded stale, misbound, out-of-order, high-latency, high-jitter, expired-calibration, low-fidelity, low-coverage and authority-escalation cases.
- Exact p50/p95/p99/max latency reporting from the sample timestamp fixtures.
- Preserve modality-specific sampling intervals; never infer a common sample rate from a shared clock.
- Deterministic report for identical input/configuration bytes.

## Claim ceiling

PASS establishes only the declared synthetic commissioning contract. It does not establish physical twin fidelity, process qualification, model VVUQ, robot safety, NDT acceptance, weld quality, class compliance, seaworthiness or ISO 23247-7 conformity.
