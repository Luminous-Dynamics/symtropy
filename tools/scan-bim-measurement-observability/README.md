# Scan-to-BIM Measurement Observability Oracle

A standalone Rust executable and frozen synthetic known-answer corpus for issue #4589.

## Four separate qualification gates

1. **Visibility:** independently derived geometric line-of-sight state.
2. **Observation:** captured, missing, invalid, mixed, contradictory or unknown return.
3. **Metrology:** sensor profile, calibration evidence, explicit operating envelope, independent reference lineage, stochastic uncertainty and systematic-error bound.
4. **Task sufficiency:** conservative total error bound against the declared task tolerance.

Overall PASS requires all four gates to pass. FAIL and UNRESOLVED are distinct; a failed or unresolved prerequisite makes downstream qualification NOT_RUN. Unknown values never silently become zero or pass.

## Sensor applicability

The evaluator does not accept a caller-supplied `in_domain` Boolean. It derives applicability from the profile's declared minimum/maximum range, maximum incidence angle, supported surface-response classes and environment classes, compared with observed capture conditions. Missing profile envelope or missing observed condition remains UNRESOLVED. Out-of-range, grazing, unsupported response or unsupported environment does not pass.

## Uncertainty rule in P0

The synthetic scalar measurand retains:
- stochastic standard uncertainty (u), declared coverage factor (k), and evidence/method identity;
- a separate systematic-error bound (b), with its own method identity and evidence digest.

The P0 task bound is conservative (k u + b). We do not RSS-combine a bounded systematic effect with a stochastic term. Missing systematic bound or its evidence is UNRESOLVED, not zero. This is a one-dimensional mechanics oracle, not a full 3D point/pose covariance, registration covariance or shared-view correlation model. Those remain with survey owner #1607 and are required for P1.

Qualified reference validation also binds disjoint calibration and holdout group IDs, an exact calibration-fit receipt and a frozen-parameter digest. The evaluator rejects overlapping calibration/holdout groups and any fit that was not frozen before holdout evaluation. This is a synthetic anti-leakage gate; it does not prove real-world independence of survey faces or sessions.

## Frozen corpus and tests

- 31 synthetic cases.
- 13 Rust unit tests.
- Negative controls cover occlusion, visible-but-not-captured surfaces, missing/mixed/contradictory returns, invalid/stale/missing calibration, duplicate/correlated reference lineage, out-of-domain range/incidence/surface/environment, malformed sensor profile, missing systematic bound, missing bound evidence, negative bound, task-specific insufficiency and invalid uncertainty.
- The fixture uses digest-shaped identifiers and synthetic numbers. Digests are not authenticated producer evidence.

## Reproducible workflow

The standalone workspace commits a pinned 23-package `Cargo.lock`. Its GitHub Actions workflow runs Cargo with `--locked`, then uploads the evaluator receipt, source/fixture/lock digests and exact commit identity. A queued/cancelled workflow is not a pass; only a completed successful run for the exact pull-request head qualifies the synthetic contract.

## Run

```sh
cargo fmt --manifest-path tools/scan-bim-measurement-observability/Cargo.toml -- --check
cargo test --manifest-path tools/scan-bim-measurement-observability/Cargo.toml
cargo clippy --manifest-path tools/scan-bim-measurement-observability/Cargo.toml --all-targets -- -D warnings
cargo run --manifest-path tools/scan-bim-measurement-observability/Cargo.toml -- \
  --corpus tools/scan-bim-measurement-observability/fixtures/measurement-observability.json
```

## Claim ceiling

The oracle qualifies deterministic contract behaviour on synthetic records only. It does not authenticate upstream evidence, establish real survey independence, calibrate a sensor, measure physical accuracy or qualify a building-engineering decision. Physical use must bind exact #1607 survey/calibration/covariance evidence and #4292 visibility receipts, use independent metrology, and be qualified for the exact instrument, target and task domain.
