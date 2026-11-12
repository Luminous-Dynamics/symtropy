# EPANET-RS Differential Qualification Manifest v0.1

This directory pins an initial downstream qualification corpus for the Rust-native [Vitens/epanet-rs](https://github.com/Vitens/epanet-rs) solver against the official [OWA-EPANET 2.3.5](https://github.com/OpenWaterAnalytics/EPANET/releases/tag/v2.3.5) reference.

## What is pinned

- Upstream repository HEAD reviewed: `fee0d03b11ed71a6da7366f07290d6e44556add4`.
- Published crate: `epanet-rs 0.2.3` (2026-07-02); the consuming Cargo.lock must record the crates.io checksum when the crate is materialized.
- OWA reference engine: tag `v2.3.5`, commit `444725fbf16ab28b0470ef0ed281dc4849d49583`.
- Upstream solver test source Git blob, 15 named test functions, and 18 upstream INP fixture paths with exact Git blob SHAs.
- Upstream validation and benchmark workflow Git blob SHAs.

The manifest references upstream files; it does **not** copy their bytes. Materialize selected fixtures only when executing the differential tests, by exact upstream commit/path/blob SHA, and preserve original attribution/licence.

## Feature gate

Upstream README feature declarations at the reviewed revision:
- Implemented/declared: INP, PDA, controls, emitters and partial API compatibility.
- Not implemented/unchecked: pipe leakage/FAVAD, rules, energy and quality.

Therefore:
- **OWA remains the acceptance oracle**.
- All FAVAD/pipe-leakage cases route to OWA until epanet-rs implements that capability and passes exact differential tests.
- The upstream parallel-speed claim applies only to its stated supported extended-period class without tanks or controls. It is not a general 5× guarantee.
- Serial feature support does not imply parallel-mode support.

## Important distinction

The upstream repo's `validate.yml` clones the OWA default branch shallowly and compares with `rtol=0.01`, `atol=0.1`; this is not an immutable OWA 2.3.5 parity receipt.

The upstream benchmark workflow uses one `benchmarks/benchmark.inp` network with Hyperfine warmups and at least five runs. That is a useful smoke benchmark, not a representative WDN/PAT, FAVAD, leakage, tank or controller qualification.

## Current execution status

`fixture-manifest.json` correctly reports:
- imported fixture bytes: 0;
- reproduced fixture outputs: 0;
- passed differential cases: 0;
- completed local benchmark runs: 0.

This is deliberate. The manifest is the first reproducible input artifact, not a claim that the solver has been tested by Luminous. The next step is to acquire the pinned crate, materialize selected fixtures, run serial OWA/Rust parity, then test parallel mode only for eligible networks.

## Acceptance sequence

1. Pin exact crate archive checksum and toolchain; pin OWA binary/source build and digest.
2. Reproduce upstream expected-value tests and compare OWA 2.3.5 vs epanet-rs on identical units/options.
3. Extend to PDA, valve/PCV, emitter, tank and control scenarios, with separate serial and parallel status.
4. Explicitly verify unsupported-feature rejection / OWA fallback for FAVAD, rules, energy and quality.
5. Only after numerical parity, benchmark 100 / 1,000 / 10,000 independent scenarios and report throughput, p50/p95/p99 latency, memory, worker scaling and deterministic replay.

See Symtropy issue [EPANET-RS-DIFF-001 #4398](https://github.com/Luminous-Dynamics/symtropy/issues/4398).
