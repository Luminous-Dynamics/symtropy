# Matched scaling baseline: furnishing oracle v0.2 (brute-force)

**Purpose:** measure the pre-BVH v0.2 oracle on exactly the same synthetic workload used by v0.3. This branch changes only benchmark wiring and workflow; it does not modify v0.2 geometry logic.

## Why

A timing number for the optimized oracle alone cannot demonstrate an improvement. This benchmark uses the same functional_envelope_scaling.rs source file, Rust 1.96.0, hosted runner image, workload generator, iteration counts and timing scope as v0.3. It runs on the v0.2 brute-force implementation so we can compare identical tasks against the implementation with the BVH.

## Method

    cargo fmt --manifest-path tools/furnishing-layout-audit/Cargo.toml -- --check
    cargo test --locked --manifest-path tools/furnishing-layout-audit/Cargo.toml
    cargo clippy --locked --manifest-path tools/furnishing-layout-audit/Cargo.toml --all-targets -- -D warnings
    cargo bench --locked --manifest-path tools/furnishing-layout-audit/Cargo.toml --bench functional_envelope_scaling

The benchmark times the full oracle on a preconstructed in-memory JSON value, including schema/content validation and geometry checks, excluding corpus construction. Inputs are synthetic sparse grids with one functional-envelope check per item. The output reports p50/p95 for 25, 100, 1,000 and 10,000 items/tasks.

## Interpretation and gate

- Compare each tier against v0.3 on a matching runner/environment; retain both raw outputs and machine metadata.
- Report the speedup ratio alongside absolute p50/p95, test outcomes, finding counts and correctness.
- Timing is not allowed to compensate for any lost or newly false-safe geometry finding.
- A single shared-runner result is a diagnostic, not an SLA. Repeated reference-machine runs and peak-memory data remain necessary.
- If the 10,000-record v0.2 baseline exceeds the workflow's 30-minute cap, document the censoring as a timeout rather than substituting a smaller number or claiming a measured result.
- The baseline branch is a measurement harness only; it should not be merged into main as a product feature. The results should be copied into the v0.3 evidence record after the matched run passes.
