# plasma-closure-composition

A dependency-free Rust reference for composing already-qualified specialist closure fluxes on one shared internal face.

## Run

    cargo fmt --manifest-path crates/tools/plasma-closure-composition/Cargo.toml -- --check
    cargo test --locked --manifest-path crates/tools/plasma-closure-composition/Cargo.toml
    cargo clippy --locked --manifest-path crates/tools/plasma-closure-composition/Cargo.toml --all-targets -- -D warnings

## Contract

For candidate fluxes F_k and declared weights w_k, the primitive computes:

    F_mix = sum_k(w_k * F_k),    w_k >= 0,    sum_k(w_k) = 1

Weights are checked against an explicit absolute tolerance and are not renormalized. Candidates are sorted by stable expert ID before accumulation, and a compensated sum is used for both the weight total and each flux component.

Every candidate must be explicitly Qualified, carry syntactically valid profile/model SHA-256 identities, and match the request's exact compatibility key: governing model, conserved state, closure quantity, species scope, normalization, units, frame, geometry, mesh digest, staggering, boundary semantics, discrete operator, solver build, spatial dimension and component count.

The input flux is oriented from the request's left cell toward its right cell. The face is evaluated once. Both integrated cell contributions are derived from the same integrated flux:

    left contribution  = -face_measure * F_mix
    right contribution = +face_measure * F_mix

This makes internal-face contributions equal and opposite by construction. The primitive handles internal faces only; external boundary fluxes require a separately specified boundary contract.

## Claim boundary

- Caller-supplied Qualified status is not authenticated here.
- A well-formed digest does not prove that the referenced bytes are available or correct.
- Matching declared compatibility keys do not prove the solver adapter is semantically correct.
- This primitive does not choose experts, compute weights, implement hysteresis, normalize units, validate pressure-tensor PSD, prove hyperbolicity or entropy consistency, or execute a plasma solver.
- Synthetic unit tests do not reproduce ECsim, the Landau-damping papers, the magnetosheath paper or an industrial solver.

Symthaea #12918 owns specialist-profile eligibility and selector research. Symtropy #6240/#6274 remain the pressure-tensor and discrete-operator referees; #1685 remains the solver-run receipt owner.

See [Symtropy issue #6288](https://github.com/Luminous-Dynamics/symtropy/issues/6288) for the research protocol and benchmark tournament.
