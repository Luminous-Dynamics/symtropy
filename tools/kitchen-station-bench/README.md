# Kitchen Station Benchmark Harness

A small, standalone Rust CLI for validating sink/drying trial records and producing reproducible descriptive comparisons.

## Build and run

From the Symtropy repository root:

```sh
cargo test --manifest-path tools/kitchen-station-bench/Cargo.toml
cargo run --manifest-path tools/kitchen-station-bench/Cargo.toml -- --help
cargo run --manifest-path tools/kitchen-station-bench/Cargo.toml -- validate trial-records --contracts docs/research
cargo run --manifest-path tools/kitchen-station-bench/Cargo.toml -- report trial-records --contracts docs/research --format markdown
cargo run --manifest-path tools/kitchen-station-bench/Cargo.toml -- compare trial-records \
  --reference REF-0 --candidate P6-BATCH-WASH --scenario SINK-BATCH-FLOW-012 \
  --contracts docs/research --format markdown
```

The CLI is offline. It loads the current versioned scenario, metric registry, trial-record schema and candidate design-space files. Published commercial product/reference context is separately tracked in `docs/research/KITCHEN_PRERINSE_MARKET_REFERENCE_V1.json`; that catalogue is not ingested as physical measurement data. The supplementary physical test setup and study sequence are described in `docs/research/KITCHEN_PRERINSE_TEST_RIG_V1.md`; the rig has not been built or run. Relative artifact URIs are resolved against the trial-record directory and their SHA-256 digests are checked; HTTP/HTTPS/URN/data URIs are recorded but are not downloaded.

## Evidence and comparison rules

- Every primary scenario metric must be present with an explicit status. Missing, invalid, blocked and not-applicable values must state why; they are never converted to zero.
- Trial contract validation is not full Draft 2020-12 JSON Schema conformance. It is an explicit semantic validator for the rules described here; keep the contract and validator tests synchronized. Every report includes SHA-256 digests for the exact contract files and each loaded trial-record file, making the report content-addressable.
- Every scenario declares a fixed required safety-gate ID set. Each trial record must explicitly status every required gate; a missing gate or extra unregistered gate invalidates the contract, and unknown/failed gates block physical eligibility. Categorical endpoints with registry-declared `qualifying_values` must match those values (e.g. `cleaning_endpoint: pass`); a measured failure cannot be offset by better resource metrics or gate labels. Synthetic fixtures, simulation, literature and documentation records are never pooled as physical observations.
- Paired comparisons require equal trial IDs, identical scenario conditions and an identical protocol digest. Units must match exactly. Results are candidate-minus-reference descriptive differences, not rankings.
- Categorical endpoints appear as non-numeric and are not turned into numerical scores. The report does not decide cleaning efficacy, sanitation, certification, product superiority or statistical significance.
- One matched pair is descriptive only; even multiple pairs require a preregistered uncertainty/power plan before generalized claims.
- An empty campaign is reported as blocked, and the command exits non-zero; this is not a passing benchmark.

## Current limitation

This harness validates the integrity and comparability of records. It does not operate a sink, measure a faucet, interpret a food-safety standard, or establish performance. Physical trials need a calibrated fixture and fit-for-purpose review.