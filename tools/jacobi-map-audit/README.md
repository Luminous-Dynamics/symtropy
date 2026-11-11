# Jacobi map audit

A small Rust library for independently auditing 2x2 screen-projected Jacobi
maps emitted by Kerr variational and finite-difference ray-bundle solvers.

## Why this layer exists

A ray tracer can produce a visually plausible image while its local lens map is
ill-conditioned, has the wrong parity, or uses inconsistent source/observer
screen normalization. This crate audits the geometric map itself, separately
from image rendering.

It provides:

- Scale-normalized 2x2 singular values and condition estimates without forming
  the potentially overflowing matrix product A-transpose times A.
- A fused multiply-add determinant calculation and log absolute determinant /
  log magnification to preserve dynamic range near strong focusing.
- A declared entrywise error bound propagated to a conservative operator-norm
  uncertainty bound. If the smallest singular value is not separated from
  that bound, the result is Unresolved and no magnification is emitted.
- A strict distinction between Resolved, IllConditioned, Singular and
  Unresolved. The library never clamps a singular determinant into a finite
  magnification.
- Independent observer/source screen rotations using D-prime = R-source
  transpose times D times R-observer. Proper rotations must preserve the
  singular values and determinant sign.
- Caustic-bracket qualification only for one exact branch identity, one
  source/observer area-normalization identity, and a strictly increasing
  branch parameter. A sign change is accepted only when both determinant signs
  are resolved against the declared map uncertainties.
- Variational-versus-finite-difference map discrepancy under a matching
  normalization and branch identity.

## Evidence contract

Input samples carry a canonical branch identifier and a normalization ID.
The latter must identify the observer angular basis and source screen-area
normalization, including endpoint and unit conventions. The componentwise
absolute uncertainty bound is an input to the audit, not an uncertainty
estimate invented by this crate.

CausticBracket.interpolated_parameter_estimate is a diagnostic linear-in-
determinant interpolation. The full endpoint interval remains authoritative.
It is not an exact-geodesic event error bound, does not prove the branch is
continuous between samples, and does not replace independent integration and
screen-grid refinement.

## Synthetic fixture

The synthetic-screen-map-atlas-v0.1.json fixture contains flat-space and
synthetic matrix cases only. It is explicitly not a Kerr ray result, a
physical caustic, or a benchmark score. The JSON Schema is in
schema/jacobi-map-evidence-0.1.0.schema.json.

## Commands

cargo fmt --manifest-path tools/jacobi-map-audit/Cargo.toml -- --check
cargo clippy --manifest-path tools/jacobi-map-audit/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path tools/jacobi-map-audit/Cargo.toml --all-targets

No Kerr geodesic is integrated by this crate. No new physical caustic,
magnification, observation fit or SOTA superiority is claimed.
