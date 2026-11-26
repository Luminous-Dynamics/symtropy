# AERO Causal-Cut 10k Smoke Result — 2026-11-26

**Evidence class:** synthetic typed in-memory benchmark only  
**Workflow:** [GitHub Actions run 45775103370](https://github.com/Luminous-Dynamics/symtropy/actions/runs/45775103370)  
**Tested commit:** `655b6019af3377ee1a62c05f9d79fed884ff51b0`  
**Branch:** `research/aero-observer-coverage-qualification-v0.1`  
**Toolchain:** Rust 1.96.0  
**Runner image:** Ubuntu 26.04.1 LTS, `ubuntu26/20261122.9.1`

## Workload

| Parameter | Value |
|---|---:|
| Events | 10,000 |
| Source partitions | 32 |
| Explicit causal dependencies | 10,092 |
| Measurement iterations | 31 |
| Warm-up iterations | 3 |
| Final disposition | `CAUSALLY_CLOSED` |

## Observed latency

| Statistic | Latency |
|---|---:|
| p50 | 17.823 µs |
| p95 | 19.322 µs |
| p99 | 19.871 µs |

The measurements were emitted by `cargo run --locked --package symtropy-aero-material-qual --example vector_cut_scale -- 10000 31`.

## Verification context

The same workflow run passed:
- Rust formatting;
- the material-process, v0.2 evidence-binding, temporal replay, observer coverage, effectivity interval, vector evidence-cut, evidence-gap packet and SHM manifest schema/semantic checks;
- 74 Rust tests, with zero failures;
- Clippy with warnings denied;
- the existing material-process and v0.2 CLI smoke tests.

## Interpretation and limitations

This result measures the typed in-memory causal-cut validator with a deterministic synthetic workload. It does **not** measure:
- durable storage, database indexes or stream ingestion;
- cryptographic signature or timestamp-token verification;
- owner-native receipt/checkpoint verification;
- source discovery or observer completeness qualification;
- hardware/sensor capture, NDE or physical truth;
- end-to-end engineering decision time or cost.

The GitHub runner's detailed CPU model was not recorded in this smoke result. The measurement is not a matched-baseline comparison and does not establish a SOTA performance advantage. It is a first reproducible scale-smoke datapoint.

The proposed >=2x p95 improvement or >=30% index/compute reduction remains a prospective tournament target at matched semantics, correctness and resource envelope.

The synthetic oracle and workload are not independently reviewed. Do not use this result as evidence of physical qualification or airworthiness.
