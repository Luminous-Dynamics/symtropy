# Scan-to-BIM Measurement Observability Oracle

A small, standalone Rust executable and synthetic known-answer corpus for issue #4589.

## Scope

The oracle keeps four gates separate:

1. Geometric visibility.
2. Observation capture/return status.
3. Metrology evidence (sensor profile, calibration, applicability, independent reference lineage and uncertainty declaration).
4. Task sufficiency (expanded uncertainty compared with a declared task tolerance).

A result is `pass` only when every gate passes. `fail` and `unresolved` remain distinct. An unresolved upstream gate prevents downstream task qualification.

## Run

```sh
cargo fmt --manifest-path tools/scan-bim-measurement-observability/Cargo.toml -- --check
cargo test --manifest-path tools/scan-bim-measurement-observability/Cargo.toml
cargo clippy --manifest-path tools/scan-bim-measurement-observability/Cargo.toml --all-targets -- -D warnings
cargo run --manifest-path tools/scan-bim-measurement-observability/Cargo.toml -- \
  --corpus tools/scan-bim-measurement-observability/fixtures/measurement-observability.json
```

## Claim ceiling

The fixture contains synthetic values and digest-shaped identifiers. This oracle tests the deterministic contract and its decision boundaries; it does not verify the truth of the referenced records, establish independence of real surveys, calibrate a sensor, measure physical accuracy, or qualify any engineering task in the physical world. The coverage factor/probability are declared inputs, not an empirically validated coverage guarantee.

For physical work, bind these records to the exact producer evidence owned by #1607 and the visibility receipt owned by #4292. Keep all empirical calibration and test sets grouped by independently measured target and capture lineage.
