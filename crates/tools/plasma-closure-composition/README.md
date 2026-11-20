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

This makes internal-face contributions equal and opposite by construction. An 18-test authored suite includes a four-cell periodic ring with face-varying weights; it compares assembled cell updates against an independently written incoming-minus-outgoing reference stencil and checks component-wise global cancellation. The primitive handles internal faces only; external boundary fluxes require a separately specified boundary contract.


## Sampled-interface entropy audit

The `entropy` module adds a narrow numerical audit for a common entropy pair at one supplied interface state pair. For the declared orientation (right minus left), it evaluates Tadmor's inequality:

    (v_R - v_L) · F_hat <= psi_R - psi_L

or the residual:

    r = Delta_v · F_hat - Delta_psi <= 0

Each candidate is bound to the same entropy-pair digest and exact left/right state fingerprints. The audit evaluates every candidate residual, requires each to be within the declared tolerance, requires the deterministic weight sum to equal exactly 1.0, then checks the blended residual and the weighted-residual identity. It refuses mixed entropy pairs or state identities.

**This is a sampled-interface check, not a proof over all admissible states.** SHA-256-shaped fingerprints and candidate bindings are declarations, not authenticated evidence. The routine does not establish that the entropy pair is valid for a given plasma moment hierarchy, that a learned candidate satisfies the inequality globally, or that the full space-time discretization is entropy stable. A real plasma application must first derive the applicable entropy variables, potential, sign convention, source/work terms and discrete operator from its governing equations.

Background:
- Tadmor's entropy stability framework for conservative numerical fluxes.
- Klein et al. (2026), *Generalized Tadmor Conditions and Structure-Preserving Numerical Fluxes for the Compressible Flow of Real Gases*, [arXiv:2603.15112](https://arxiv.org/abs/2603.15112), which extends flux conditions to secondary structures and introduces discrete null-consistency for non-convex structures. That result motivates future structure-specific contracts; it is not assumed to transfer directly to plasma closures.


## External-boundary accounting

The `boundary` module accepts one already-computed external boundary flux oriented with the declared outward normal. For each component it retains both the outward integrated flux and the adjacent-cell contribution:

    outward export = A * F_out
    cell update     = -A * F_out

Their sum is zero for that boundary interaction, but the outward export is **not** cancelled against an internal neighbour. This prevents an open or prescribed physical boundary from being accidentally treated as periodic. The module binds the boundary condition, outward-normal identity, cell/boundary state fingerprints, boundary profile/model digests and solver compatibility key.

This is accounting only. It does not solve sheath, conducting-wall, absorbing, reflecting, open or inflow boundary physics, and a fingerprint is not proof that the boundary state is physically correct. A solver adapter must source-pin its actual boundary condition and orientation convention.

## Claim boundary

- Caller-supplied Qualified status is not authenticated here.
- A well-formed digest does not prove that the referenced bytes are available or correct.
- The authored test suite contains 36 Rust tests: the independent periodic-grid stencil oracle, a 36-case sampled entropy-convexity sweep, and seven external-boundary accounting tests, including a two-cell mixed internal/external global-balance case. These are not reported as executed until Rust tooling or hosted CI actually runs them.
- Matching declared compatibility keys do not prove the solver adapter is semantically correct.
- This primitive does not choose experts, compute weights, implement hysteresis, normalize units, validate pressure-tensor PSD, prove hyperbolicity or entropy consistency, or execute a plasma solver.
- Synthetic unit tests do not reproduce ECsim, the Landau-damping papers, the magnetosheath paper or an industrial solver.

Symthaea #12918 owns specialist-profile eligibility and selector research. Symtropy #6240/#6274 remain the pressure-tensor and discrete-operator referees; #1685 remains the solver-run receipt owner.

See [Symtropy issue #6288](https://github.com/Luminous-Dynamics/symtropy/issues/6288) for the research protocol and benchmark tournament.
