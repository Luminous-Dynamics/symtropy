# Dredge solids conversion v0.1

Dependency-free Rust reference evaluator for Symtropy issue #3340.

## Run

~~~sh
cargo test --locked --manifest-path benchmarks/dredge-solids-conversion/Cargo.toml
cargo clippy --locked --manifest-path benchmarks/dredge-solids-conversion/Cargo.toml --all-targets -- -D warnings
~~~

## Supported V0 measurement basis

dry mass = wet volume × wet slurry density × dry-solids mass fraction

Inputs are fixed-point:
- wet volume: microlitres;
- wet slurry density: milli-kilograms per cubic metre;
- dry-solids mass fraction: permille (1,000 = 100%);
- output: integer grams, with a conservative lower/upper interval.

The conversion scale is 1,000,000,000,000 product-units per gram. Lower mass rounds down; upper mass rounds up. All products/additions are checked.

The density is the density of the wet slurry. The solids fraction is explicitly a mass fraction, not a volume fraction. A scow ullage observation or wet-volume observation alone cannot produce dry mass.

## Evidence binding

The conversion manifest binds one plant, telemetry event, material lot, activity, time interval and spatial scope. Wet-volume and density observations must match that manifest. The solids fraction sample must refer to the same lot and spatial scope and be sampled within the declared interval, with an explicit representativeness attestation. Calibration, currentness, unit, quality and evidence states are checked.

The conversion result is a bounded dry-mass observation only. It never creates a sediment ledger transfer. The caller must separately obtain an authorized transfer/receipt and preserve this conversion ID as provenance.

## Replay

An in-memory deterministic cursor records the complete accepted input. Exact duplicate replay is idempotent. Reusing the same conversion ID with changed inputs is inconsistent. This cursor is a reference test mechanism, not a durable or cryptographically authenticated event store.

## Claim ceiling

This is synthetic fixed-point arithmetic and evidence validation. It is not a dredge production estimator, representative sampling protocol, laboratory method, measurement certification, environmental assessment, sediment-suitability determination, permit or ledger authority. It does not assume independent statistical errors; it propagates declared deterministic bounds only.
