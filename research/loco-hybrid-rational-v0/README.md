# LOCO Hybrid Rational V0

Dependency-free exact-rational reference fixtures for interval arithmetic and conservative hybrid event-order linearization.

Research parent: [LOCO-HYBRID-MATH-001 / #4113](https://github.com/Luminous-Dynamics/symtropy/issues/4113) and [LOCO-SETVAL-001 / #4083](https://github.com/Luminous-Dynamics/symtropy/issues/4083).

## Purpose

The finite robust-viability oracle can only reason about the successor graph it receives. This crate tests a small part of the numerical bridge that must eventually generate that graph:

- exact rational arithmetic;
- closed rational intervals;
- interval addition, subtraction, multiplication and affine images;
- interval intersection and containment;
- conservative enumeration of event-order linearizations from bounded event-time windows.

The crate is a reference fixture, not a continuous reachability solver.

## Arithmetic contract

A rational is stored in canonical form as a signed i128 numerator and strictly positive i128 denominator. Construction reduces by the greatest common divisor and normalizes sign. Arithmetic is checked; overflow and division by zero return explicit errors. There is no floating-point conversion, rounding, saturation or wrapping.

Some operations may return ArithmeticOverflow when an intermediate or comparison cross-product cannot be represented, even if a more sophisticated arbitrary-precision implementation could reduce it. This is intentionally fail-closed. The reference is bounded, not an arbitrary-precision number package.

The interval endpoints are exact rational inputs. The crate does not establish that a user-supplied interval encloses a physical state, parameter, contact force, or event time.

## Interval semantics

All intervals are closed. Multiplication checks all four endpoint products. Affine images compute the interval extension of:

    y = slope * x + offset

where all three quantities may be intervals.

This is a conservative interval extension. It may overestimate the true image when variables are dependent or correlated. The result is exact for the interval operations performed on the supplied rational intervals, not necessarily the tightest possible reachable set.

The implementation does not use outward-rounded IEEE-754 arithmetic. Exact fractions avoid rounding for these fixtures; they do not solve continuous ODE reachability or establish a plant model.

## Hybrid event-order semantics

Each event has a caller-supplied closed time window. If the upper bound of event A is strictly below the lower bound of event B, A must precede B.

For overlapping or merely touching windows, both total-order linearizations are retained. This is deliberately conservative and may include spurious orders. For example, windows [0, 1] and [1, 2] can represent A-before-B or simultaneity at t=1; retaining B-before-A is an additional conservative linearization, not a claim that reverse ordering is physically feasible.

A total-order linearization is not a simultaneous-event reset model. When two impacts can be simultaneous and their physics are coupled, the caller must include an explicit joint guard/reset branch. Sequential reset maps must not be assumed to commute.

Event enumeration is deterministic and bounded. If the configured maximum number of orders is exceeded, the function returns an error rather than a partial order list.

## Included qualification fixtures

Twenty deterministic Rust tests cover:

- rational sign normalization, reduction and extreme signed values;
- exact addition, subtraction, multiplication and division;
- negative divisors and checked overflow;
- interval multiplication across positive/negative domains;
- uncertain affine images and negative slopes;
- closed intersections, shared boundaries and disjoint intervals;
- monotonicity of affine interval images under state-interval widening;
- strictly separated, overlapping and touching event windows;
- event-order input permutation;
- finite-grid checks that no feasible strict order is omitted;
- explicit conservatism for touching windows;
- duplicate and negative-time event rejection;
- event-order resource exhaustion and the empty-event case.

Independent Python Fraction cross-checks were also run during authoring against 200,000 randomized rational divisions and 100,000 randomized rational sums/products. Those checks are supplementary development evidence, not a substitute for exact-head Rust verification.

## Commands

    cargo fmt --manifest-path research/loco-hybrid-rational-v0/Cargo.toml -- --check
    cargo test --manifest-path research/loco-hybrid-rational-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/loco-hybrid-rational-v0/Cargo.toml --locked --all-targets -- -D warnings

A dedicated exact-head workflow is defined at .github/workflows/loco-hybrid-rational.yml. No CI pass is claimed until formatting, all tests and strict Clippy succeed at the exact PR head.

## Claim ceiling

A passing fixture qualifies this implementation's rational arithmetic, interval operations and event-order overapproximation on the tested inputs only. It is not:

- an outward-rounded floating-point interval library;
- a validated ODE integrator or continuous reachable-set engine;
- a guard-crossing-time estimator;
- a physical contact or friction model;
- a simultaneous-impact reset solver;
- a sound continuous-to-finite abstraction;
- a robot safety certificate.

The next step is to add independently tested guard intersections and reset-image fixtures, then export conservative transitions into the finite oracle only after the #4083 abstraction obligations are satisfied.

## Research baselines

- Madabushi et al. (2026), Differentiable Invariant Sets for Hybrid Limit Cycles with Application to Legged Robots: https://arxiv.org/abs/2604.05108
- Maoudj et al. (2026), SPINDER, an open-source 18-DoF hexapod robot with hierarchical CPG control and analytic IK: https://doi.org/10.1038/s41598-026-56856-0
- Xue et al. (2026), Impact-Aware Robust Convex Model Predictive Control for Quadruped Locomotion on Uncertain Terrain: https://doi.org/10.1109/LRA.2026.3692086