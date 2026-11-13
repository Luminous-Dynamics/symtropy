# Dynamic source-history audit

This crate implements P0 of issue #4255 using the exactly solvable reduced
retarded single-pole model

    g_R(t) = theta(t) exp(-i omega t).

For piecewise-constant source histories S(t), it calculates

    activation(t) = integral_{-infinity}^{t} exp(-i omega (t-tau)) S(tau) d tau

and reports an independent instantaneous direct channel

    direct(t) = D S(t).

The total reduced response is activation + direct. The analytic segment
integrals avoid numerical quadrature and include a small-argument expansion to
avoid cancellation near omega=0.

## Included controls

- No response before source support.
- Exact rectangular-pulse activation during and after the source interval.
- Separation of history activation from the local/direct channel.
- Time-translation equivariance.
- Complex source-amplitude linearity.
- Different past histories with identical instantaneous source values.
- Stable zero-frequency limit.
- Sorted/non-overlapping segment validation and stable-pole guard.

## Scope and claim boundary

This is a reduced linear retarded-pole model only. It is not the Schwarzschild
EMRI activation formula, does not implement horizon redshift, does not produce
the published redshift tower, and is not a black-hole Green-function solver.

It provides mathematical controls for the source-history interface before the
published model is reproduced using #4109's canonical Green function and
validated against #4052's independent time-domain evolution.

## Reproduction

    cargo fmt --manifest-path tools/bh-dynamic-source-audit/Cargo.toml -- --check
    cargo clippy --manifest-path tools/bh-dynamic-source-audit/Cargo.toml --all-targets -- -D warnings
    cargo test --manifest-path tools/bh-dynamic-source-audit/Cargo.toml --all-targets
