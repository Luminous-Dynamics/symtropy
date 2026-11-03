# Hill Static Equilibrium V1

A dependency-free Rust isometric Hill-type musculotendon equilibrium oracle.

## Model scope

V1 includes:
- tabulated active fiber force-length, passive fiber force-length and tendon force-length curves;
- piecewise-linear interpolation with no silent extrapolation;
- constant-thickness pennation geometry;
- normalized curve forces scaled by maximum isometric force;
- first-order activation/deactivation state update using the exact exponential solution for piecewise-constant excitation;
- a bounded bisection solver for static fiber-tendon force equilibrium;
- typed no-bracket, non-unique, invalid-geometry, iteration-limit and tolerance failures.

The static equilibrium equation is:

    F_tendon(l_t / l_t_slack)
      = F_iso * [a * f_active(l_f / l_opt) + f_passive(l_f / l_opt)] * cos(phi)

where:
- l_t = l_MT - l_f cos(phi);
- constant-thickness pennation uses sin(phi) = l_opt sin(phi_opt) / l_f;
- active, passive and tendon curves are source-bound sampled tables.

## Explicit limitations

This is a static/isometric Hill-type equilibrium slice, not a complete OpenSim Millard2012EquilibriumMuscle port. It does not yet implement force-velocity, dynamic fiber length/velocity integration, tendon ODEs, fiber damping, moment arms, joint torques or contact.

The curve tables and muscle parameter digests are syntax-checked references supplied by an external evidence authority; this package does not hash/authenticate the underlying assets. It does not claim biological muscle parameters or reproduce FlyMimic.

A static equilibrium may be non-unique in a slack/no-force regime. V1 returns NonUniqueEquilibrium rather than choosing an arbitrary fiber length when the bracket endpoints and midpoint are all within the declared force tolerance. The root solver returns one bracketed root and does not prove global uniqueness for non-monotonic curve sets.

## Synthetic tests

The 13 tests cover known-answer zero-pennation equilibrium, constant-thickness pennation and cosine force projection, piecewise-linear interpolation, no extrapolation, malformed curve tables and digests, exact activation/deactivation updates, slack non-uniqueness, passive-only equilibrium, no-bracket rejection, impossible geometry, invalid policies and bounded iteration diagnostics.

All fixtures are synthetic. They verify the local equation/solver contract only.

## References

- OpenSim Millard 2012 model documentation: https://opensimconfluence.atlassian.net/wiki/spaces/OpenSim/pages/53090555/Millard+2012+Muscle+Models
- OpenSim API equilibrium equation: https://opensim-org.github.io/opensim-moco-site/docs/1.0.0/html_user/classOpenSim_1_1Millard2012EquilibriumMuscle.html
- Millard et al. (2013): https://doi.org/10.1115/1.4023390
- Musculoskeletal-model verification and validation guidance: https://doi.org/10.1115/1.4029304

## Run

    cargo fmt --manifest-path research/hill-static-equilibrium-v1/Cargo.toml -- --check
    cargo test --manifest-path research/hill-static-equilibrium-v1/Cargo.toml --locked
    cargo clippy --manifest-path research/hill-static-equilibrium-v1/Cargo.toml --locked --all-targets -- -D warnings

The package is a standalone Cargo workspace under research/. It does not modify Symtropy's main workspace or production dependency graph.
