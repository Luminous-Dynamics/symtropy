# LOCO Hybrid Rational V0

Dependency-free Rust reference fixtures for exact rational interval arithmetic and hybrid event-order enumeration.

Issue: https://github.com/Luminous-Dynamics/symtropy/issues/4083

## Scope

V0 deliberately implements only two bounded capabilities:

1. Exact rational closed-interval arithmetic for one-dimensional affine images, including uncertain slope and offset intervals.
2. Enumeration of every total event ordering consistent with caller-supplied closed event-time windows.

The event windows are inputs. This package does not compute continuous trajectories, locate guard crossings, integrate ODEs, apply contact reset maps, or establish a complete hybrid reachable set.

## Why exact rational intervals first

A finite-state viability oracle can be mathematically exact while its input transition graph is unsound. Conversely, naive floating-point min/max is not enough to claim a validated enclosure.

This reference therefore starts with exact rational endpoints. Addition, subtraction, multiplication, division and comparison use checked i128 arithmetic. Products reduce common factors before multiplication where possible. Any unrepresentable result returns an explicit error; it is never rounded, saturated or wrapped.

For uncertain affine coefficients, the interval product encloses all values over the supplied Cartesian product of the slope, state and offset intervals. This can be conservative, especially when variables are correlated. Correlations must not be discarded if doing so would invalidate the declared model relation.

## Event ordering

For two event windows A and B:
- if upper(A) < lower(B), A must precede B;
- if their closed windows overlap or touch, both orders remain possible.

The enumerator returns all total orders consistent with these necessary precedence constraints. Input order does not affect canonical output. A caller-supplied order limit returns an error rather than a partial order list.

The enumerator does not prove that an event actually occurs or that its time window is sound. It only preserves all orders implied by the supplied windows.

## Included tests

Fifteen deterministic tests cover:
- rational sign and common-factor reduction;
- exact rational add/subtract/multiply/divide;
- division-by-zero and arithmetic-overflow rejection;
- interval multiplication across positive and negative endpoints;
- uncertain affine images;
- negative-slope affine mapping;
- closed guard-style interval intersection, including shared boundaries;
- disjoint intervals;
- strictly separated, overlapping and touching event windows;
- permutation-invariant event-order enumeration;
- event-order resource-limit failure without partial output;
- duplicate/negative-time event rejection;
- empty event set behavior.

## Run locally

    cargo fmt --manifest-path research/loco-hybrid-rational-v0/Cargo.toml -- --check
    cargo test --manifest-path research/loco-hybrid-rational-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/loco-hybrid-rational-v0/Cargo.toml --locked --all-targets -- -D warnings

A dedicated Rust 1.96.0 workflow is defined in `.github/workflows/loco-hybrid-rational.yml`. Do not claim verification until all three commands pass on the exact PR head.

## Claim ceiling

Passing tests qualify only this exact rational arithmetic and event-window enumeration implementation. They do not qualify:
- continuous-time ODE integration;
- guard-crossing time computation;
- interval Taylor models or higher-dimensional flowpipes;
- completeness of a physical contact/actuator model;
- sound abstraction into #4061;
- a physical robot safety guarantee.

The next extension should be a separately verified exact-rational guard/reset fixture and conservative export to the finite viability oracle. Only after that should the project add validated interval ODE integration and phase-conditioned hybrid tubes.
