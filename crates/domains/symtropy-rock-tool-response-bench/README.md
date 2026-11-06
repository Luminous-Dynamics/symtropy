# Symtropy Rock Tool Response Bench

Standalone semantic validator for mechanism-resolved rock/tool response bundles.

## Scope

This crate validates cross-field and cross-reference semantics in the v0.1 JSON bundle:

- unique record and physical specimen identity;
- ATCP / laser-PDC / ODC discriminator consistency and method-specific fields;
- finite positive kinematic/signal quantities;
- laser ON, source-energized sham and mechanical OFF baseline energy semantics;
- ODC speed ratio recomputation from independently measured angular/traction encoder observations, matching calibrations, raw artifacts and eccentricity;
- unique channels/artifacts, channel-to-raw-artifact linkage and clock-domain agreement;
- calibration, uncertainty, synchronization and anti-alias metadata;
- signal bandwidth not exceeding sample_rate / 2;
- mechanism-required independent diagnostic channels with expected SI units;
- observation source lineage and artifact parent DAG integrity;
- deterministic, sorted validation issues.

It does not perform JSON Schema 2020-12 validation, authenticate artifact bytes, determine whether calibration is physically valid, interpret acoustic/thermal signals, infer fracture mode, calculate MSE, predict excavation performance, rank methods, or control equipment.

SHA-256 strings are format-checked only. The artifact-ingestion boundary must hash the actual bytes and bind the digest to the stored object.

## Run

```sh
cargo test --manifest-path crates/domains/symtropy-rock-tool-response-bench/Cargo.toml
```

The frozen synthetic fixture is loaded from the research portfolio documentation. It contains no physical measurements or qualified equipment settings.

## API

- `validate_bundle_json(&str)`: parse JSON and return a semantic report.
- `validate_bundle(&serde_json::Value)`: validate an already-parsed JSON value.
- `ValidationReport::is_valid()`: return true only when no issues exist.

Full JSON Schema validation and the Rust semantic validator are separate B0 conformance gates.