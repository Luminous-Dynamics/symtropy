# Orbital truss independent datum-network benchmark v0.1

Tracks [issue #6836](https://github.com/Luminous-Dynamics/symtropy/issues/6836), as a separate follow-on to [as-built geometry v0.1 / PR #6811](https://github.com/Luminous-Dynamics/symtropy/pull/6811).

## Why this exists

The v0.1 geometry evaluator validates a supplied proper rigid 2D transform and checks declared frame/datum identifiers. It does not consume actual datum-control correspondences. This benchmark tests that missing seam without changing the frozen v0.1 contract.

## Algorithm

Fit a proper rigid 2D transform from matched external control points only. The fit minimizes planar correspondence error under rotation and translation; it does not estimate scale and cannot introduce a reflection. Points are sorted by stable ID before summation so record-order permutation does not change the result.

For centered source coordinates x_i and target coordinates y_i:

- D = sum(x_ix*y_ix + x_iy*y_iy)
- C = sum(x_ix*y_iy - x_iy*y_ix)
- theta = atan2(C, D)
- t = mean(y) - R(theta) mean(x)

A returned pose is not automatically observable. The control network must meet declared minimum span and maximum triangle-area requirements. At least three control points are required to expose planar degeneracy, and at least one independent check point is mandatory.

Check points are withheld from fitting and evaluated only after the transform is fixed. Each residual is compared with a declared hard radial allowance:

allowed = base residual limit + source point error bound + target point error bound + registration error bound

These terms are hard upper bounds in this synthetic contract, not GUM standard uncertainties, standard deviations, coverage intervals or an ISO 14253-1 decision profile. No RSS aggregation or independence assumption is made.

- QUALIFIED: control network observable and all controls and withheld checks are within hard bounds.
- REJECTED: fitted transform is contradicted by a control or withheld-check residual.
- INDETERMINATE: insufficient/ill-conditioned network, missing checks, duplicate identities, article-derived basis or invalid frame labels.

## Frozen corpus

Fourteen synthetic cases cover known rotation/translation recovery, record-order permutation, insufficient controls, collinearity, insufficient span, near-degeneracy, article-derived datum, withheld-check mismatch, control outlier, scale mismatch, reflection, duplicate point identity, missing holdout and invalid frame identity.

## Scope and claim ceiling

The input labels and SHA-256 digest do not authenticate who established the reference network, coordinate frame, raw instrument observations, calibration records or physical datum monuments. This is synthetic 2D registration mathematics only. It is not 3D pose covariance, metrology traceability, a GUM implementation, ISO 14253-1 compliance, GD&T, physical article acceptance, structural qualification, NDE/POD or flight authorization. Receipts always set authority_granted=false and physical_validation=NOT_ASSESSED.

## Reproducible build

```sh
cargo fmt --manifest-path tools/orbital-truss-independent-datum-bench/Cargo.toml -- --check
cargo test --locked --manifest-path tools/orbital-truss-independent-datum-bench/Cargo.toml
cargo clippy --locked --manifest-path tools/orbital-truss-independent-datum-bench/Cargo.toml --all-targets -- -D warnings
cargo run --locked --manifest-path tools/orbital-truss-independent-datum-bench/Cargo.toml -- docs/research/orbital-truss/fixtures/independent-datum-v0_1.json <40-hex-source-commit>
```

The standard-library Python reference is separately implemented and is not production code.

## Next research boundary

A true 3D successor needs SO(3) pose handling, six-degree-of-freedom pose uncertainty, correlation-aware propagation, datum-reference-frame semantics, calibrated instrument interfaces and a named decision rule. That should be a new versioned contract, not an incremental reinterpretation of the 2D hard bounds here.
