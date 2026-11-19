# plasma-symmetry-transform-audit

Standalone, offline Rust reference for **proper SO(3) rotations** of explicitly synthetic scalar, polar/axial vector, and rank-2 tensor fields.

This is a geometric transformation test harness. It is not a plasma solver, does not generate PIC data, does not discover closures, and does not establish that an arbitrary plasma regime has SO(3) symmetry.

## Run

    cargo test --locked --manifest-path crates/tools/plasma-symmetry-transform-audit/Cargo.toml
    cargo run --locked --manifest-path crates/tools/plasma-symmetry-transform-audit/Cargo.toml -- benchmarks/plasma/symmetry-transform-v0.json
    cargo clippy --locked --manifest-path crates/tools/plasma-symmetry-transform-audit/Cargo.toml --all-targets -- -D warnings

The crate has an isolated Cargo workspace boundary and does not join the Symtropy root workspace. It makes no network calls and launches no external solver.

## v0.1 profile

The only admitted profile is `synthetic-so3-scalar-vector-tensor-v0.1`:

- Scalars are invariant.
- Polar and axial vectors transform as `R v` for proper rotations.
- Rank-2 tensors transform as `R T R^T`.
- The matrix must be finite, orthogonal and have determinant +1.
- Reflections and shears are refused.
- Field identities, units and finite values are required.
- Receipts bind the request digest, transform matrix, output fields, audit metrics and a fixed synthetic-only claim ceiling.

The audit reports scalar invariance, vector norm invariance, tensor trace/Frobenius/determinant invariance, matrix orthogonality and determinant residuals separately.

## Frozen corpus

`benchmarks/plasma/symmetry-transform-v0.json` contains seven analytical/synthetic cases: identity, two generic proper rotations, reflection refusal, shear refusal and duplicate identity refusal. Unit tests additionally verify an exact quarter-turn vector and pressure-tensor result, deterministic replay and tamper rejection.

## Scientific boundary

This first slice is intentionally limited to the SO(3) geometric action. It does **not** implement Galilean or Lorentz boosts, electromagnetic field transformations, gauge transformations, kinetic distribution transforms, or plasma-regime applicability decisions.

A successful software test means only that the declared synthetic transformation is implemented consistently. It does not establish physical symmetry of magnetic reconnection, a tokamak, MHD, PIC, or any other plasma regime. Source-pinned transformation profiles and physical applicability reviews are separate qualification work.
