# Orbital source-bound curvature provider v0.1

This standalone Rust component derives an outward fixed-point bound on the second derivative of a normalized relative-position curve under a declared central-gravity model and a separately supplied bound on differential non-central acceleration.

It is an upstream input provider for the one-segment chord-tube geometry in PR #7659. It does not replace the orbit propagator or qualify the source bounds it consumes.

## Mathematical contract

For central gravity,

    g(r) = -mu * r / ||r||^3

and

    Dg(r) = mu * (3 r r^T / ||r||^5 - I / ||r||^3).

The radial eigenvalue magnitude is 2 mu / ||r||^3 and the transverse eigenvalue magnitudes are mu / ||r||^3, so the spectral norm is 2 mu / ||r||^3.

Given complete-segment bounds:
- ||r_chief|| >= R_chief,min;
- ||r_deputy - r_chief|| <= rho_rel,max;
- R_min = R_chief,min - rho_rel,max > 0;

every point on the line joining chief and deputy has radius at least R_min. The mean-value theorem gives:

    ||g(r_deputy) - g(r_chief)|| <= 2 mu_max * rho_rel,max / R_min^3.

Add the independently supplied differential acceleration upper bound A_diff,max for all non-central effects in the declared force profile:

    ||r_rel''|| <= 2 mu_max * rho_rel,max / R_min^3 + A_diff,max.

For a fixed, time-invariant keep-out normalization q = L r_rel:

    ||q''|| <= ||L||_2 * (2 mu_max * rho_rel,max / R_min^3 + A_diff,max).

This is the M input used by #7659. A burn, target event, thrust cutoff, frame change or force-profile discontinuity must split the smooth interval; no bound is reused across a discontinuity.

## Integer units

All values are integers:
- mu upper bound: mm^3 / s^2 (u128)
- chief geocentric radius lower bound: mm (u64)
- chief/deputy relative separation upper bound: mm (u64)
- differential relative acceleration upper bound: mm / s^2 (u64)
- transform operator norm: ceil(||L||_2 * 1,000,000 mm) (u64)
- output M upper bound: ceil(||q''|| * 1,000,000 s^2), reported with m_scale=1,000,000 and m_unit=s^-2 (u128)

With D = R_min^3, define:

    C = 2 * mu_upper * rho_rel_upper
    T = L_norm_scaled
    P = T * A_diff_upper

Then use the exact identity for integer P:

    ceil(T * (C / D + A_diff_upper)) = ceil(T * C / D) + P
    M_upper_scaled = ceil((T * C) / D) + P

The implementation separately checks T*C, T*A_diff_upper, and the final sum. This avoids materializing A_diff_upper*D and the combined numerator, which can overflow u128 even when the final outward bound is representable. The result remains exact and outward-rounded; no floating-point conversion or downward rounding is used. The receipt preserves C, D, T, both scaled terms and the final M witness. Non-positive R_min, genuine intermediate/final overflow, invalid inputs, missing/currentness/source-scope failures and unsupported time-varying transforms never produce BOUND_READY.

## Required evidence inputs

Five independent provider references must all be present, current and bound to the exact same scope_id:
- gravitational parameter upper bound;
- chief-radius lower bound;
- relative-separation upper bound;
- differential non-central acceleration upper bound;
- fixed-transform operator-norm upper bound.

The scope binds action, segment, target, chief, servicer, central body, keep-out profile/geometry, transform, frame, time basis, dynamics profile and exact smooth-segment TAI interval. The complete perturbation inventory must be declared. An inventory flag is an assertion from its provider, not proof that the real force model is complete.

Provider digests and identities are retained as references; this component does not authenticate issuer or source bytes.

## Frozen corpus and tests

The CSV has 36 deterministic known-answer cases covering:
- central-gravity-only and non-central differential acceleration;
- transform, chief-radius and separation monotonicity;
- exact and non-exact rational ceilings;
- zero separation;
- non-positive radial domain;
- incomplete perturbation inventory;
- time-varying transform;
- each provider failure/currentness/scope/digest gate;
- u128 overflow at the radial cube, central product, scaled central term, scaled perturbation term and final outward sum; a high-magnitude positive control demonstrates that avoidable aggregate intermediates no longer cause false overflow.

The Rust tests include monotonicity checks. The independent Python oracle uses arbitrary-precision integers and checked u128 emulation, and reproduces the full provenance-bearing receipt independently.

## Reproduce

    cargo fmt --manifest-path tools/orbital-gravity-curvature-bound-bench/Cargo.toml -- --check
    cargo test --locked --manifest-path tools/orbital-gravity-curvature-bound-bench/Cargo.toml
    cargo clippy --locked --manifest-path tools/orbital-gravity-curvature-bound-bench/Cargo.toml --all-targets -- -D warnings
    CORPUS=tools/orbital-gravity-curvature-bound-bench/fixtures/gravity-curvature-corpus-v0_1.csv
    DIGEST=$(sha256sum "$CORPUS" | cut -d' ' -f1)
    cargo run --quiet --locked --manifest-path tools/orbital-gravity-curvature-bound-bench/Cargo.toml --bin orbital-gravity-curvature-bound-bench -- "$CORPUS" "$GIT_COMMIT" "$DIGEST"
    python3 tools/orbital-gravity-curvature-bound-bench/tests/reference_oracle.py "$CORPUS" "$GIT_COMMIT" "$DIGEST"

## Claim boundary

A passing result establishes only that the fixed-point calculation conservatively follows from the declared numeric inputs. It does not establish that the chief radius, relative-separation, perturbation, transform or mu bounds describe a real spacecraft segment. It does not validate a force model, state estimator, source issuer, terminal passive-safe set or physical trajectory. The receipt hard-codes claim_ready=false, source_bytes_authenticated=false, authority_granted=false, and physical_validation=NOT_ASSESSED.

See #7692 for the upstream mathematical and physical qualification programme.
