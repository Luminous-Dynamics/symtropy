# Industrial Telemetry Adapter

A dependency-free Rust parser and offline profiler for the three-column event format in the NIST / Manufacturing Technology Centre (MTC) box-assembly Technical Data Package (TDP).

## Pinned source

- Source repository: [NIST Smart Manufacturing Systems Test Bed](https://github.com/usnistgov/smstestbed)
- Source commit: `968279f14ebe96c03c8877eeb1901cf6b4c8fbab` (2020-05-20)
- Exact source file: [`tdp/mtc/Split/Box-OP1-Hurco02-01of20.txt`](https://github.com/usnistgov/smstestbed/blob/968279f14ebe96c03c8877eeb1901cf6b4c8fbab/tdp/mtc/Split/Box-OP1-Hurco02-01of20.txt)
- Expected Git blob SHA-1: `aa5e49ad87a787573259c4318b1516196e8b349a`
- Format documentation: [NIST/MTC TDP README](https://github.com/usnistgov/smstestbed/blob/968279f14ebe96c03c8877eeb1901cf6b4c8fbab/tdp/mtc/README.md)

The upstream TDP README describes `Split` as parsed/cleaned MTConnect data divided by operation and part instance. The same TDP provides related design, NC-code, and quality/inspection artefacts, which could support future evidence links. This crate does **not** claim that the trace establishes machine energy use, causal downtime, accepted product yield, total cost, or continuous availability.

The full 600 KB trace is **not vendored** here. Obtain it from the pinned URL and check its byte identity before profiling:

```sh
curl -L 'https://raw.githubusercontent.com/usnistgov/smstestbed/968279f14ebe96c03c8877eeb1901cf6b4c8fbab/tdp/mtc/Split/Box-OP1-Hurco02-01of20.txt' -o /tmp/Box-OP1-Hurco02-01of20.txt
git hash-object /tmp/Box-OP1-Hurco02-01of20.txt
```

Expected blob output is `aa5e49ad87a787573259c4318b1516196e8b349a`. The CLI prints this expected identity but **does not itself verify the input file's digest**; checking with `git hash-object` is a separate input qualification step. After that digest check, `verify-nist-reference` compares byte length, full summary counts, timestamp endpoints, precise largest-gap endpoints and all 11 field counts against the pinned reference profile. Profile agreement is a consistency check, not a cryptographic identity check. Keep attribution and the source repository's notices with any redistributions. NIST's repository says public information may generally be copied/distributed except material marked copyrighted and requests appropriate attribution; this is not a blanket conclusion about unrelated third-party artefacts.

## Run offline

From the repository root, after downloading and verifying the source file:

```sh
cargo test --locked --offline --manifest-path crates/tools/industrial-telemetry-adapter/Cargo.toml
cargo run --locked --offline --manifest-path crates/tools/industrial-telemetry-adapter/Cargo.toml -- summarize /tmp/Box-OP1-Hurco02-01of20.txt
cargo run --locked --offline --manifest-path crates/tools/industrial-telemetry-adapter/Cargo.toml -- verify-nist-reference /tmp/Box-OP1-Hurco02-01of20.txt
```

The crate has no external dependencies and defines its own Cargo workspace to avoid compiling the full Symtropy workspace. Parsing and tests require no network access. The runtime CLI reads a local file only. A dedicated [CI workflow](../../../.github/workflows/industrial-telemetry-adapter.yml) checks formatting, runs the standalone tests and Clippy offline, and smoke-tests the CLI against the pinned six-row source excerpt in `fixtures/nist_mtc_operation_excerpt.txt`.

## Source profile recorded during exploration

The pinned trace was inspected independently before this adapter was authored. Its observed profile was:

| Measure | Observed source profile |
|---|---:|
| Non-empty event records | 11,719 |
| Distinct field names | 11 |
| First event | `2017-02-02T16:37:08.9062500Z` |
| Last event | `2017-02-02T18:35:48.9687500Z` |
| Timestamp order inversions | 0 |
| Adjacent same-timestamp event pairs | 458 |
| Unique timestamps | 11,261 |
| Largest gap between observed event times | 5,369,937,500,000 ns (89.498958333 minutes) |

Observed field counts from that exploration:

| Field | Records |
|---|---:|
| `Feed_Rate` | 5,657 |
| `Block_Number` | 3,875 |
| `Program_Runtime_Seconds` | 1,781 |
| `Spindle_Speed` | 189 |
| `Spindle_Override` | 82 |
| `Rapid_Override` | 76 |
| `Feed_Override` | 43 |
| `Current_Tool` | 8 |
| `Spindle_Time` | 5 |
| `Program_Status` | 2 |
| `Part_Count` | 1 |

These numbers are from an independent high-precision scan of the pinned source, **not yet output from a run of this crate**. The exact maximum gap is from `2017-02-02T16:37:08.9062500Z` to `2017-02-02T18:06:38.8437500Z`. The adapter's `verify-nist-reference` mode is designed to reproduce this profile once it is executed locally. One-unique-timestamp inputs have no defined inter-event gap; the parser returns `None` instead of fabricating a zero-length gap at the Unix epoch.

## Safety and epistemic boundaries

- Each nonblank line must contain exactly three pipe-separated columns: UTC timestamp, field name, value. Malformed nonblank rows reject the trace; they are not silently dropped.
- UTC timestamps require `YYYY-MM-DDTHH:MM:SS[.fraction]Z`, with 1–9 fractional digits if present. Date and clock values are validated; non-UTC offsets and leap seconds are rejected.
- Timestamps are converted to integer nanoseconds, avoiding floating-point ordering loss.
- Input event order is preserved; inversions are counted rather than sorted away.
- Same-time events are valid and preserved; same-time adjacent pairs are reported separately from row counts.
- The maximum time gap is computed across sorted unique observed timestamps. It describes **sampling coverage only**; it is `None` when fewer than two unique timestamps exist.
- Missing events are not interpreted as idle, available, safe, productive, unavailable, or failed equipment.
- Values remain strings. Unit and state-code semantics require an explicitly versioned mapping before using them in a planner.
- The adapter is an ingestion/profiling layer, not an optimizer and not an operational technology interface.

## Qualification gates

The fourteen authored unit tests cover fractional precision, source-shaped excerpt parsing, same-time event preservation, long gaps, non-monotonic timestamps, malformed rows, strict UTC/date checks, leap-year handling, UTC epoch conversion, single-timestamp gaps, blank values, and temporal coverage queries. A dedicated workflow explicitly builds this nested standalone crate. Treat tests as qualified only when that workflow passes on the exact current head.

## Temporal coverage API

`find_unobserved_intervals` accepts a requested time window and a caller-declared maximum event-silence threshold. It reports intersecting intervals before the first event, between events, after the last event, or across a trace with no events. For between-event gaps it preserves the full source-gap duration even when only part of that gap overlaps the requested window. Events are sorted and deduplicated for this calculation, but the original trace ordering remains preserved in the parsed data and summary.

The API is deliberately **not** a scheduler admission approval: no reported large gap means only that this threshold detector found no long gap in the supplied window. It does not mean the machine was available, safe, idle, or productive. Downstream scheduling must combine it with separately qualified state/capacity evidence, and must fail closed when required evidence is absent. The tests include a proposed operation window that crosses the 89.498958333-minute gap and expects an explicit finding.
