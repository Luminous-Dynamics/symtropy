# South African Fuel Resilience Benchmark (SAFRB) — v0.1

**Status:** research kernel; not operationally qualified  
**Research cut:** 2026-11-23  
**Implementation:** dependency-free Rust 2024  
**Archive:** `safrb-v0.1.tar.gz` (contains source, schema, source register, scenario manifest and benchmark protocol)

## Purpose

A reproducible first test of whether fuel-security claims preserve physical accounting and evidence quality under disruption. v0.1 is a national-aggregate diesel balance kernel, not a provincial digital twin, current stock estimate, supply forecast or operational recommendation.

The kernel enforces an important rule: only measured, quality-released and physically accessible volumes qualify as immediately usable inventory. Reported stock, contracts, pipeline line-fill, cargo nominations and vessel milestones remain distinct. Import receipts enter only at discharge.

It simulates deterministic 7/14/30/60/90-day import shocks with a 30-day recovery tail, and explicitly records unserved demand, storage overflow and daily mass-balance residuals. Its demonstration demand anchor is a rounded 2024 historical working baseline; all other rates and stocks are synthetic fixtures.

## Contents

- `Cargo.toml`, `Cargo.lock`: isolated, dependency-free crate.
- `src/`: library, executable and tests.
- `schema/inventory-observation.schema.json`: inventory evidence schema.
- `case.toml`: frozen benchmark case definition and fixture provenance.
- `research/SOURCES.md`: source register and known caveats.
- `research/evidence-conflicts.csv`: unresolved 2024 diesel-export figure conflict.
- `research/BASELINE_TOURNAMENT.md`: matched baselines and scoring protocol.
- `research/PARADIGM_SHIFTS_AND_SCORECARD.md`: research thesis, competitive position, metrics, pilot wedge and safety boundaries.

## Run locally

Extract the archive and enter `safrb/`:

```sh
cargo fmt --check
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo run --release
```

## Qualification status

The authoring execution environment did not include `cargo`, `rustc` or `rustfmt`. Therefore formatting, Rust tests and Clippy are **not claimed as passed** in this submission. The JSON schema and TOML manifests were syntax-parsed in the authoring environment. The Rust checks must be run on the exact PR head before the kernel can be called build-verified.

## Hard boundaries

- Stress durations are scenarios, not forecasts.
- Aggregate mass-balance correctness is not pipeline-hydraulic validation.
- Public method datasets (for example, Tennessee Eastman and AI4I) are not petroleum ground truth.
- No live or confidential operator data is included.
- Initial industrial integration is read-only. No PLC/DCS/SIS writes or automatic fuel allocation.
- No claim of outperforming AVEVA, SLB, Honeywell or another incumbent is made. The first competitive wedge is cross-organisation evidence reconciliation and resilience decision assurance.

See `research/PARADIGM_SHIFTS_AND_SCORECARD.md` for the full evaluation plan.
