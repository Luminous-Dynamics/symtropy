# Orbital truss as-built geometry benchmark v0.1

This crate implements the first executable slice beneath Symtropy #4263 / #6810: **bounded 2D as-built geometric conformance**.

It is deliberately not a new truss solver, point-cloud engine, CAD kernel, GD&T engine, NDE/POD model, stiffness estimator or structural acceptance authority.

## Core rule

A common-mode position error must not disappear because the inspected article was best-fit aligned back onto its nominal geometry. V0.1 therefore accepts only a declared rigid transform derived from independent external datums and explicitly rejects the article-derived-fit path.

For every required feature:

`r = ||R x_{survey} + t - x_{nominal}||_2`

`u = u_{feature} + u_{registration}`

The uncertainty terms are added conservatively. No independence, RSS cancellation, Gaussian distribution or covariance model is assumed.

`lower = max(0, r-u)`, `upper = r+u`.

- `ACCEPT` only when `upper <= tolerance`.
- `REJECT` when `lower > tolerance`.
- Otherwise `INDETERMINATE_HUMAN_REVIEW`.

A definite feature violation produces REJECT even if a different required feature is unobserved. Incomplete or invalid evidence can never produce ACCEPT.

## Frozen synthetic corpus

Thirteen cases cover nominal acceptance, common-mode datum offset, rigid-frame metamorphism, reflection, scale, missing critical coverage, wrong serial, stale revision, expired calibration, guard-band overlap, article-derived registration, duplicate feature measurements, and a certain oversize feature.

The evaluator reports per-feature residuals, combined uncertainty radius, lower/upper error bounds, tolerance and reason codes. The receipt binds the input bytes and source commit and always carries:

- `authority_granted=false`
- `structural_acceptance_authorized=false`
- `flight_authorized=false`
- `physical_validation=NOT_ASSESSED`

## What this establishes

Only deterministic software semantics for the frozen synthetic cases. The provided SHA-256 and calibration-window checks do not authenticate the survey instrument, calibration laboratory, external datum network, physical article or producer. It does not prove metrology traceability or physical truth.

## Benchmark targets (proposed, not achieved)

- zero false accepts on held-out critical negatives;
- 100% explicit abstention for missing observations, stale revision, invalid calibration, invalid registration and uncertainty overlap;
- invariance under equivalent rigid-frame changes;
- independent oracle agreement on frozen and sealed holdout corpora;
- report false-accept / false-reject / indeterminate rates by failure family, not only one aggregate score.

## Comparative boundary

NIST STEP File Analyzer and Viewer, CAx-IF AP242 recommendations and commercial CAD/CMM systems remain the specialist reference points for product definition, PMI, inspection programming and conformance. This crate begins downstream of those systems: it evaluates a small, transparent, reproducible contract over already-normalized feature observations. It does not replace NIST SFA, PC-DMIS, CAD/CAM, QIF validators or a metrology system.

## Build

```sh
cargo fmt --manifest-path tools/orbital-truss-asbuilt-geometry-bench/Cargo.toml -- --check
cargo test --locked --manifest-path tools/orbital-truss-asbuilt-geometry-bench/Cargo.toml
cargo clippy --locked --manifest-path tools/orbital-truss-asbuilt-geometry-bench/Cargo.toml --all-targets -- -D warnings
cargo run --locked --manifest-path tools/orbital-truss-asbuilt-geometry-bench/Cargo.toml -- \
  docs/research/orbital-truss/fixtures/asbuilt-geometry-v0_1.json <40-hex-source-commit>
```

The Python reference implementation is an independent standard-library oracle; it is not part of the production evaluator.
