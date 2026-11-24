# Planar Patch Limit-Surface Oracle v0.1

A dependency-free Rust reference for an aggregate planar friction-wrench ellipsoid. It is an independent model family alongside the point-level friction-cone oracle in #7127.

## Declared model

The admissible aggregate friction wrench in the contact frame is:

    w = [f_u, f_v, m_n]

with semi-axes:

    a = mu * N * [e_u, e_v, e_n]

and admissible set:

    (f_u / a_u)^2 + (f_v / a_v)^2 + (m_n / a_n)^2 <= 1

where mu is a dimensionless non-negative friction scale, N is the declared compressive normal load, e_u and e_v are positive tangential force shape scales, and e_n is a positive torsional shape scale with units of length.

## Implemented queries

### Analytic support

For dual direction d:

    h(d) = sqrt((a_u d_u)^2 + (a_v d_v)^2 + (a_n d_n)^2)

The oracle returns the support value and a maximizing wrench on the ellipsoid boundary.

### Membership

The membership query returns an explicit interior / boundary / outside disposition. Zero normal load or zero friction gives zero friction capacity. A nonzero wrench under zero capacity receives a typed zero-capacity status and no invented infinite normalized radius.

### Maximum dissipation

For nonzero slip twist v = [v_u, v_v, omega_n]:

    w_i = -(a_i^2 * v_i) / sqrt(sum_j (a_j * v_j)^2)

The result lies on the ellipsoid boundary and has non-positive power v dot w. At zero slip, maximum dissipation does not select a unique friction wrench; the API returns a set-valued StickingSet rather than an arbitrary boundary point. Slip and positive-power numerical tolerances are explicit inputs.

## Research relationship

The model is motivated by the aggregate planar patch / equivalent-contact-point limit-surface formulation in Huang et al., *A Unified Complementarity-based Approach for Rigid-Body Manipulation and Motion Prediction* (2026): https://arxiv.org/abs/2602.04522.

This crate does not reproduce the full paper's complementarity formulation, ECP normal-contact law, planner, rigid-body dynamics or time stepping. It implements only the independently testable aggregate friction ellipsoid support, membership and local maximum-dissipation subproblem.

Do not assume equivalence with Symtropy's discrete point Coulomb cones. Model-to-model differences require an explicit mapping and should otherwise be reported as DIFFERENTIAL_NOT_EQUIVALENT.

## Run

    cargo fmt --manifest-path research/planar-patch-limit-surface-oracle/v0.1/Cargo.toml -- --check
    cargo test --manifest-path research/planar-patch-limit-surface-oracle/v0.1/Cargo.toml --locked
    cargo run --manifest-path research/planar-patch-limit-surface-oracle/v0.1/Cargo.toml --example patch_limit_tournament --locked
    cargo clippy --manifest-path research/planar-patch-limit-surface-oracle/v0.1/Cargo.toml --locked --all-targets -- -D warnings

The standalone crate is explicitly built and tested by the existing three-platform repository CI workflow.

## Proposed benchmarks (not achieved)

- zero false support or membership results against analytic ellipsoid fixtures;
- zero positive-power friction responses outside the declared tolerance;
- 100% of zero-slip cases preserve a set-valued sticking status;
- compare support and membership against the discrete point model only where a declared geometric/constitutive mapping exists;
- report pure tangential sliding, pure torsional spin and mixed translation/rotation separately;
- add slip onset, spin onset, object displacement, residual and energy/work in a future dynamics adapter.

## Claim ceiling

This is an analytic planar constitutive oracle, not a physics engine or a physical validation result. It does not cover arbitrary 3D patches, anisotropic pressure evolution, changing contact geometry, normal complementarity, multi-body coupling or full solver convergence.

Related work: #7216, #7127, #7155, #7157, #7195, #7199, #7203, #7212 and #460.
