# SUF Fibre Calibration Reference Kernel

A dependency-free Rust reference implementation for the **single, already-disambiguated fibre span** case.

## What it does

- Separates interrogator optical range, physical fibre chainage, route station and projected map coordinates.
- Applies two separately declared monotone piecewise-linear calibrations.
- Binds each observation to exact fibre, fibre revision, route revision, interrogator configuration, direction and source digest.
- Rejects stale/mismatched identities, out-of-range values, invalid anchors and unsupported coordinate frames.
- Interpolates along a versioned route polyline only when the caller explicitly declares a projected-metre CRS and easting/northing axis order.
- Produces a conservative horizontal uncertainty bound by summing declared absolute error bounds. This is not a statistical confidence interval.
- Returns route positioning only. It does not attribute the result to a utility asset.

## Explicit non-goals

This crate does **not**:
- parse vendor DAS formats or waveform payloads;
- estimate calibration anchors from DAS or traffic signals;
- implement a geodetic/CRS transformation engine;
- resolve branched, multiplexed or ambiguous fibre topology;
- establish survey control or physical ground truth;
- authenticate a source publisher or prove the source digest is trustworthy;
- implement SensorThings, MUDDI, PROV-O or authorization;
- detect faults, classify events, estimate protection state or command SCADA;
- establish physical DAS accuracy or replace qualified survey work.

A caller must supply independently obtained calibration anchors, route geometry, coordinate-frame metadata, identity bindings and error bounds. A branched or ambiguous topology must be split into independently identified spans and resolved by a competent upstream process; the kernel intentionally abstains.

## Coordinate pipeline

`optical_range_m → physical_fibre_chainage_m → route_station_m → (easting_m, northing_m)`

Asset attribution is deliberately absent from the result type. It requires a separate owner-authorized evidence relationship.

## Build and test

From the repository root:

```sh
cargo test --locked --manifest-path tools/suf-fibre-calibration/Cargo.toml
cargo clippy --locked --manifest-path tools/suf-fibre-calibration/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path tools/suf-fibre-calibration/Cargo.toml -- --check
```

## Claim ceiling

Passing this crate's unit tests establishes behavior of the declared arithmetic and rejection contract only. It is not a field result, calibration certification, spatial conformance claim or engineering approval.
