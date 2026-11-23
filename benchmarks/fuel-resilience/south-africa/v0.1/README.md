# South African Fuel Resilience Benchmark (SAFRB) — v0.1

**Status:** research kernel; not operationally qualified  
**Research cut:** 2026-11-23  
**Implementation:** dependency-free Rust 2024

## Purpose

A reproducible first test of whether fuel-security claims preserve physical accounting and evidence quality under disruption. v0.1 is a national-aggregate diesel balance kernel, not a provincial digital twin, current stock estimate, supply forecast or operational recommendation.

The model distinguishes reported, measured, verified-usable, committed, pipeline line-fill and cargo milestone states. Only measured, quality-released and physically accessible volumes qualify as immediately usable inventory. Import receipts enter the physical balance only at discharge.

## Source bundle

`safrb-v0.1-source.tar.gz` contains the standalone Rust crate, evidence schema, frozen case manifest and executable/test source:

- `Cargo.toml` and `Cargo.lock`
- `src/lib.rs` and `src/main.rs`
- `schema/inventory-observation.schema.json`
- `case.toml`
- package `README.md`

SHA-256 is recorded in `SHA256SUMS`.

## Demonstration boundaries

The demonstration uses a rounded 2024 historical South African diesel-demand anchor. All stock, domestic production and import-rate fixtures are synthetic. It exercises 7/14/30/60/90-day disruption cases and a 30-day recovery tail. These are stress tests, not forecasts.

This is an aggregate accounting/reference kernel, not a live South African fuel model, provincial network, pipeline hydraulic solver, refinery process simulator or operational decision system.

## Evidence rules

- Reported stock does not become verified-usable stock without qualifying measurement, product-quality evidence and physical accessibility.
- Cargo nomination, loading, sailing, arrival, discharge and quality release are different milestones.
- Daily mass balance, unserved demand and storage overflow are separately reported.
- Conflicting public source values are retained as unresolved evidence conflicts, not silently averaged or selected.
- Missing or stale inputs reduce the strength of the conclusion; they are not imputed as confirmed supply.

## Qualification status

The authoring environment did not include `cargo`, `rustc` or `rustfmt`. Rust formatting, compilation, tests and Clippy have **not** been claimed as passed. Run the checks below on the exact PR head with the repository's pinned Rust 1.96.0 toolchain.

```sh
cargo fmt --check
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo run --release
```

The JSON schema and TOML manifests were syntax-checked in the authoring environment. This does not substitute for Rust build qualification.

## Safety and claim ceiling

The initial model is read-only. It has no path to PLC, DCS, SIS, terminal-control or allocation execution. No live or confidential operator data is included. Better-than-industry claims require matched datasets, identical budgets, strong baselines and independent reproduction.

The intended initial research wedge is cross-organisation inventory and movement reconciliation, with resilience planning as a read-only analytical aid.
