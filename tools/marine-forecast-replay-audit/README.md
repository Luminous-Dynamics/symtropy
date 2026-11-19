# Marine Forecast Replay Audit

Standalone Rust admission gate for historical marine replay, tracked by Symtropy #6209.

This tool does not download or decode GRIB/NetCDF, generate forecasts, interpolate observations, simulate vessels, score routes, or authorize operations. It audits whether source references and temporal/rights metadata may be assigned to a deployable policy, retrospective evaluator, or observation-validation path.

## Commands

    cargo test --manifest-path tools/marine-forecast-replay-audit/Cargo.toml
    cargo run --manifest-path tools/marine-forecast-replay-audit/Cargo.toml -- tools/marine-forecast-replay-audit/fixtures/synthetic-replay-bundle-0001.json
    cargo clippy --manifest-path tools/marine-forecast-replay-audit/Cargo.toml --all-targets -- -D warnings

## Information-set rules

- A deployable forecast object requires model/cycle identity, forecast lead, valid time, grid/CRS/axis metadata, units, artifact digest and explicit first-availability evidence. Synthetic fixtures use the separate synthetic_fixture marker and never establish a real source availability claim.
- The chronology is source issue time <= first demonstrable availability <= decision cutoff. retrieved_at is retained separately; a later archive download cannot be used to infer what was available historically. For fixed-lead forecast objects, issue time + lead hours must equal valid time, and retrieval cannot precede declared availability.
- A forecast valid time must match the decision target instant.
- ERA5/Copernicus-style realized reanalysis belongs only in evaluator_context_source_ids.
- NDBC-like station observations belong only in validation_observation_source_ids in this initial profile. They are sparse point checks, not route-wide forecast inputs.
- Commercial route exports are not deployable weather forecasts. Route geometry does not reproduce hidden forecast cycles, polars or routing settings.
- Unknown rights/availability make the record incomplete; restricted or unauthorized research use blocks replay. Synthetic-only rights and evidence cannot be promoted into a non-synthetic replay.
- Non-redistributable source data can still be assessed under an authorized research permission; this tool does not redistribute data.
- A syntactically valid SHA-256 digest identifies bytes but does not authenticate a publisher or prove measurement quality.

The fixture is synthetic and must remain SYNTHETIC_ONLY. No real NOAA/ECMWF/Copernicus/NDBC or vendor objects are included.
