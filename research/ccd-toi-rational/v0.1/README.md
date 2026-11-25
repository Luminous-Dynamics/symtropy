# CCD TOI Rational Evaluator v0.1

A dependency-free exact-rational evaluator for reported continuous-collision-detection time-of-impact claims.

## Contract

- Oracle input is an independently supplied exact rational root for each positive query, or no root for a negative query.
- The algorithm output is one of: unavailable, exact rational point estimate, or a reported rational interval.
- A point estimate is not an interval certificate. They are scored separately.
- An interval is valid only when both endpoints are in normalized time [0,1] and lower <= upper.
- Rational comparisons use a continued-fraction algorithm, avoiding cross-product overflow.
- Interval width arithmetic is checked; arithmetic overflow fails closed.
- Duplicate query identities and TOI claims on oracle-negative queries fail closed.
- Zero-positive populations return unavailable TOI recall.
- The evaluator reports point exact matches, interval-root enclosures, under-enclosures, unavailable TOI claims, and exact rational p50/p95/p99 interval widths.

## Scope

The v0.1 implementation uses u128 rational numerators and denominators. It does not parse algebraic WXF roots, derive TOI roots, compute collision geometry, or independently prove that a producer's enclosure is conservative beyond checking it against the supplied exact oracle root. These are explicit boundaries, not implied capabilities.

The crate is a scorecard component for constructed exact-root fixtures and later independently qualified oracle adapters. It must not be pooled with Boolean recall, broadphase pair recall, or performance denominators.

## Run

```sh
cargo fmt --manifest-path research/ccd-toi-rational/v0.1/Cargo.toml -- --check
cargo test --manifest-path research/ccd-toi-rational/v0.1/Cargo.toml --locked
cargo clippy --manifest-path research/ccd-toi-rational/v0.1/Cargo.toml --locked --all-targets -- -D warnings
```

## Promotion targets (aspirational)

- 100% exact-root enclosure coverage on every supported exact-oracle stratum.
- Zero under-enclosures and zero invalid/out-of-domain interval claims.
- Complete TOI reporting on methods that claim TOI; Boolean-only methods remain TOI-unavailable.
- Preserve exact rational or independently isolated algebraic roots before adding binary64 approximations.
- Report normalized interval-width distributions by scene, query class and arithmetic profile.
