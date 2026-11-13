# SUF Fibre Calibration Reference Kernel

A dependency-free Rust reference implementation for the **single, already-disambiguated fibre span** case.

## What it does

- Separates interrogator optical range, physical fibre chainage, route station and projected map coordinates.
- Applies two separately declared monotone piecewise-linear calibrations.
- Binds each observation to exact fibre, fibre revision, route revision, interrogator configuration, physical start/end endpoint identities, direction and source digest.
- Binds route-station zero and increasing direction to the actual physical endpoint pair, including reverse-end interrogation.
- Rejects stale/mismatched identities, out-of-range values, invalid anchors and unsupported coordinate frames.
- Interpolates along a versioned route polyline only when the caller explicitly declares a projected-metre CRS and easting/northing axis order.
- Produces a conservative horizontal-equivalent uncertainty bound by summing declared absolute error bounds. Callers must first propagate native axis errors through the exact transform sensitivities. This is not a statistical confidence interval.
- Returns route positioning only. It does not attribute the result to a utility asset.
- Includes a discrete expected-information / expected-value-of-sample-information evaluator that keeps information gain separate from decision-loss reduction and suppresses net-value rankings when cost and loss units are incomparable.
- Includes a receipt-gated advisory action selector that ranks positive comparable net-value actions under a fixed resource budget and rejects candidates missing owner/safety receipt references.
- Includes a source-envelope semantic validator for raw-vs-derived lineage, modality boundaries, typed sample axes and pitch/resolution separation.
- Includes an event-conditioned coverage evaluator with complete route-length partitioning, separate independent event/background truth gates, unique physical-event and false-alarm episode IDs, 95% Wilson POD intervals and false alarms per observable fibre-km-day.
- Includes a deterministic integration benchmark with 10,000 seeded samples over a bent projected route, independent oracle equations, endpoint checks and monotonicity checks.

Each eligible event ID must represent one independent physical event (not a crop, frame, overlapping window or repeated observation); detected event IDs must be a subset of eligible IDs. False-alarm episode IDs similarly deduplicate overlapping alarm windows. The evaluator rejects duplicates and withholds numerators/denominators when their independent truth source is absent. The coverage module also provides an idealized zero-miss binomial sample-size function; 299 independent trials are needed for a one-sided 95% upper miss-probability bound of 1% when no misses are observed.

## Decision-value evaluator

The decision module accepts a declared prior, decision-loss matrix and per-hypothesis measurement-outcome likelihoods. It computes expected information gain in bits and expected value of sample information in the declared loss unit. It only computes net value when the measurement cost uses exactly the same declared unit. A high-information measurement that cannot change the optimal decision is not recommended when it costs more than zero. This is an offline mathematical evaluator, not a learned probability model or dispatch/authorization system.

The advisory selector validates digest syntax only; callers must independently authenticate receipt issuers, scope, expiry, purpose and safety-plan coverage. A selected action is a recommendation, not authorization, dispatch or execution.

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

Passing this crate's tests establishes behavior of the declared arithmetic, semantic-envelope and synthetic geometry contracts only. The 10,000-sample test is synthetic and is not physical DAS performance. It is not a field result, calibration certification, spatial conformance claim or engineering approval.
