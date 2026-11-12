# GMD thermosphere drag-law oracle

A dependency-free Rust reference kernel for the instantaneous aerodynamic drag law used in the GMD thermosphere/orbit-resilience programme.

## Scope

This crate evaluates the analytic drag acceleration, force, atmosphere-relative dissipation and conservative acceleration magnitude envelope for declared local density, ballistic coefficient and spacecraft-relative velocity.

It does **not** forecast thermospheric density, model attitude or atmospheric co-rotation, propagate an orbit, estimate covariance/conjunction probability, model spacecraft radiation/charging, or claim flight-dynamics qualification. It is an analytic mechanics oracle only (E1 synthetic software evidence).

## Run

    cargo fmt --manifest-path tools/gmd-thermosphere-drag-oracle/Cargo.toml -- --check
    cargo test --manifest-path tools/gmd-thermosphere-drag-oracle/Cargo.toml --locked --offline
    cargo clippy --manifest-path tools/gmd-thermosphere-drag-oracle/Cargo.toml --locked --offline --all-targets -- -D warnings

## Known-answer fixture

For rho = 1e-12 kg/m^3, Cd = 2.2, A = 10 m^2, m = 100 kg, and speed = 7,500 m/s, the ballistic coefficient is 100/22 kg/m^2, drag acceleration magnitude is 6.1875e-6 m/s^2, and drag force magnitude is 6.1875e-4 N. This is a synthetic algebraic fixture, not an observed LEO density or a prediction for a particular spacecraft.

## Claim ceiling

A passing unit suite proves the declared equation, units and input validation on the frozen fixtures. It does not validate a density model, atmosphere state, spacecraft parameters, orbit propagation or operational satellite risk.
