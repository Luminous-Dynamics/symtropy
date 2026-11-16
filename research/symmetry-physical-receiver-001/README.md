# Independent symmetry-to-physical receiver v0.1

First executable receiver slice for [PHYS-SYMMETRY-RECEIVER-001 #5663](https://github.com/Luminous-Dynamics/symtropy/issues/5663), based on the versioned handoff schema in [#4784](https://github.com/Luminous-Dynamics/symtropy/pull/4784).

## Authority separation

The receiver recomputes the physical statements it accepts. It does not copy an upstream mathematical PASS into its own physical status. Its receipt is generated from the exact source receipt, generator set, fixture, numerical profile and task identity bundle.

V0 covers a fixed 3D dense generalized-coordinate re-description q = S y. It independently recomputes mass congruence, force and momentum covector pullback, constraint Jacobian transformation, action/generator conjugation, the appropriate finite-element isometry or infinitesimal-generator metric-skew law, canonical pairing, tangent metric norm, and dual cotangent norm.

All content encodings are versioned and domain-separated. SHA-256 is provided by RustCrypto sha2; the receiver hashes the exact canonical source receipt, generator set, physical fixture, individual physical objects and task-semantics bundle. The profile bounds encoded bytes, numerical tolerances and coordinate condition.

## Explicit inconclusive boundary

This receiver does not have an independently replayed physical one-step map. It therefore always sets discrete-step equivariance, physical observables, and numerical error-envelope certification to INCONCLUSIVE, and physical-equivalence authority to NONE. Scientific-search pruning is hard-coded false. A source-side mathematical receipt is not a step-map proof.

## Reproduction

    cargo fmt --manifest-path research/symmetry-physical-receiver-001/Cargo.toml --all -- --check
    cargo test --locked --manifest-path research/symmetry-physical-receiver-001/Cargo.toml
    cargo clippy --locked --manifest-path research/symmetry-physical-receiver-001/Cargo.toml --all-targets -- -D warnings

The Rust API accepts a typed source receipt, physical fixture and task digest bundle. A JSON-schema adapter that reads the complete handoff JSON into these types is a follow-on; v0.1 must not be described as a general envelope parser or cross-language canonical protocol.

## Claim ceiling

This is a bounded dense 3D receiver reference. It does not verify arbitrary Riemannian manifolds, full nonlinear dynamics, discrete-step equivariance, contact/friction, global Noether maps, or scientific task-equivalence pruning. All residual defects are f64 numerical checks, not interval-certified bounds.
