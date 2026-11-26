# Orbital inspection chord-tube verifier v0.1

A small independent geometry checker for a single relative-motion interval in normalized keep-out coordinates. It is an initial mathematical slice of [Symtropy #7657](https://github.com/Luminous-Dynamics/symtropy/issues/7657), not an orbital propagator or flight-safety shield.

## Contract

Let q(t) in R^3 be the relative position in normalized keep-out coordinates, so the protected region is the unit sphere ||q|| <= 1. This includes a sphere directly and an ellipsoid after a fixed, declared whitening transform q = Lr with K = L^T L.

For one smooth segment [t0,t1], assume:

- the endpoint estimates are within norm error epsilon of the true q(t0) and q(t1);
- the complete segment is twice continuously differentiable in this coordinate frame;
- a valid upper bound M satisfies ||q''(t)|| <= M for every time in the segment;
- any impulsive burn, frame discontinuity, target-attitude discontinuity or model switch splits the trajectory into separate smooth segments.

The linear chord interpolation between the true endpoints has a vector error no greater than M h^2 / 8, where h = t1-t0. This follows from the second-derivative interpolation remainder. The supplied endpoint error is added, so the true path lies within a tube of radius:

    e = epsilon + M h^2 / 8.

The kernel computes the exact squared distance from the origin to the segment joining the supplied endpoints, then uses integer arithmetic and an outward ceiling for M h^2 / 8. It emits CONDITIONAL_CLEAR only if the chord's minimum distance is strictly greater than 1+e. Equality, possible contact, and insufficient geometric clearance return NOT_PROVEN. Invalid values and integer overflow are explicit non-success outcomes.

## What is and is not verified

The kernel verifies the geometric consequence of the supplied endpoint and second-derivative bounds. It does not derive or authenticate those bounds. In particular, it does not:
- propagate an eccentric orbit or solve a nonlinear force model;
- establish that M bounds the actual relative acceleration for the whole interval;
- qualify state/covariance or keep-out whitening transforms;
- validate source bytes, frame/time basis, target attitude or actuator execution;
- prove terminal passive safety, an abort-reachable set or an operational action ticket.

The receipt therefore hard-codes claim_ready=false, source_bytes_authenticated=false, authority_granted=false, and physical_validation=NOT_ASSESSED. CONDITIONAL_CLEAR is a conditional mathematical result, not a #7573 safety claim. A production integration must source the complete interval bound from an independently qualified dynamics/enclosure provider and bind the provider proof, model, transform, action and time basis. An under-bounded M can invalidate the conclusion; the kernel cannot detect that by examining the number alone.

## Why use a chord tube?

The direct 2026 inspection-shield comparator uses an adaptive time grid. Its inter-step rule bounds the permitted interval using distance-to-keep-out divided by the maximum of the two endpoint speeds; the paper notes this approximation is exact for monotonic speed and relies on the slow-dynamics / maximum-step regime otherwise. This kernel explores a different route: verify a continuous geometric tube around each chord, provided a whole-interval curvature bound is independently available.

It is intentionally only a challenger primitive. A looser bound is safe but may lead to NOT_PROVEN; the future benchmark must measure this conservatism against the sampled shield, not hide it.

## Units

- q coordinates and endpoint-error norms are integer micro-units of normalized keep-out radius: 1 unit is 10^-6.
- M is integer micro-units per second squared.
- h is supplied in nanoseconds.
- The curvature term is outward-rounded as ceil(M * h_ns^2 / (8 * 10^18)) micro-units.
- The keep-out boundary is at 1,000,000 coordinate units.

The corpus is comma-separated with the header in the fixture. The expected field is used only by the independent oracle, not by the Rust evaluator.

## Reproduce

    cargo fmt --manifest-path tools/orbital-inspection-chord-tube-bench/Cargo.toml -- --check
    cargo test --locked --manifest-path tools/orbital-inspection-chord-tube-bench/Cargo.toml
    cargo clippy --locked --manifest-path tools/orbital-inspection-chord-tube-bench/Cargo.toml --all-targets -- -D warnings
    CORPUS=tools/orbital-inspection-chord-tube-bench/fixtures/chord-tube-corpus-v0_1.csv
    DIGEST=$(sha256sum "$CORPUS" | cut -d' ' -f1)
    cargo run --quiet --locked --manifest-path tools/orbital-inspection-chord-tube-bench/Cargo.toml --bin orbital-inspection-chord-tube-bench -- "$CORPUS" "$GIT_COMMIT" "$DIGEST"
    python3 tools/orbital-inspection-chord-tube-bench/tests/reference_oracle.py "$CORPUS" "$GIT_COMMIT" "$DIGEST"

The Python challenger uses fractions.Fraction, arbitrary-precision integers and an independently written segment-distance calculation. It also checks the tight quadratic interpolation example q(t)=ell(t)+c*t*(1-t), for which the maximum chord deviation is exactly sup||q''||/8.

## Promotion boundary

This v0.1 corpus establishes only agreement on a finite synthetic mathematical contract. It does not establish orbit-propagation truth, continuous-time spacecraft safety, physical measurement quality, operational readiness or SOTA performance. See #7657 for the staged eccentric-LEO reproduction, independent trajectory verifier, passive-safety challenger and benchmark tournament.
