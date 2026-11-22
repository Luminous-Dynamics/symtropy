# Symtropy Rare-Event Kernel

An isolated, dependency-light Rust package for mathematical primitives used by the orbital conjunction rare-event assurance programme.

This crate is a research kernel, not an operational collision-probability product. It deliberately does not implement orbit propagation, random sampling, candidate event-mode discovery, proposal fitting, confidence sequences for importance-sampling weights, or flight operations. It includes only a Bernoulli-specific time-uniform confidence sequence.

## Current primitives

- Strict positive-definite covariance validation using a Cholesky factor.
- Multivariate Gaussian log density without explicit matrix inversion.
- Gaussian half-space probability and log-probability, reduced to a one-dimensional normal CDF.
- Analytic tail-probability sweep from 1e-3 to 1e-9 across dimensions 1, 2, 3, 6 and 12.
- Stable log-domain standard-normal interval mass, including extreme positive and negative tails.
- Log mass for axis-aligned boxes under independent standard-normal coordinates.
- A Gaussian cube-tail union-bound primitive for probability bookkeeping.
- Outward-rounded interval enclosures for affine images of finite axis-aligned cells.
- A conservative norm lower bound and static collision-ball cell-exclusion predicate.
- Deterministic longest-axis cell subdivision with stable child split paths.
- Conservative standard-normal cell-mass upper bound using a cell-local density ceiling that tightens with distance from the nominal Gaussian mean.
- A Gaussian Chernoff / union-bound estimate for mass outside the finite whitened cube, plus a residual-mass ledger for unresolved cells.
- A budgeted affine-cell traversal that separately classifies cells wholly inside caller-declared chart boxes, cells proven event-free, and unresolved cells outside chart coverage.
- A disconnected-event-union verifier with component-specific chart assignments. A cell is accounted only when each event component is either covered by its own chart or proven event-free; one chart cannot suppress another mode, even when their input-space regions overlap.
- An interval-polynomial time-window classifier that proves no-contact / guaranteed-contact only by enclosing the entire relevant time interval, and otherwise returns an explicit ambiguous disposition.
- A fail-closed event-mass composition primitive that combines an unnormalised admitted-region event-mass interval with an independent upper bound on the disjoint residual complement. It requires matching event/model/target/partition identities and a caller-declared disjoint/exhaustive partition, preserving both component evidence records and keeping the composition explicitly conditional on that declaration. A typed adapter converts the existing affine-union residual verifier report into residual evidence while preserving its budget-exhaustion state.
- Dense-grid adversarial tests checking sampled affine images remain inside the interval enclosure and sampled norms remain above the reported lower bound.
- Streaming log-domain weighted event sums, contribution ESS, plug-in relative standard error, and a clearly labelled normal-approximation interval diagnostic.
- A beta(1,1)-mixture Bernoulli e-process and inverted time-uniform confidence sequence for independent bounded event indicators, with an exhaustive short-horizon optional-stopping coverage test.
- Full normalized Gaussian-mixture log density using log-sum-exp.
- Log importance weights and one-draw weighted event indicators.
- Strict Loewner covariance-floor validation, including the strictness test needed for a half-nominal covariance lower bound.

## Run the isolated tests

From the repository root:

```sh
cargo generate-lockfile --manifest-path benchmarks/orbital-conjunction/rare-event-kernel/Cargo.toml
cargo fmt --manifest-path benchmarks/orbital-conjunction/rare-event-kernel/Cargo.toml -- --check
cargo test --locked --manifest-path benchmarks/orbital-conjunction/rare-event-kernel/Cargo.toml --all-targets
cargo clippy --locked --manifest-path benchmarks/orbital-conjunction/rare-event-kernel/Cargo.toml --all-targets -- -D warnings
```

The package has its own Cargo workspace and does not require the full Symtropy/Bevy workspace.

The current authored test corpus contains 114 authored `#[test]` cases: 30 in `lib.rs`, 25 in `coverage.rs`, 13 in `residual.rs`, 10 in `sequential.rs`, 10 in `stats.rs`, 10 in `trajectory.rs`, and 16 in `event_mass_composition.rs`. The 98-test parent passed its exact-head gate; this 111-test revision requires its own exact-head workflow.

## Important limitations

- The covariance floor helper checks only the matrix inequality. It does not prove all assumptions of the constrained-mixture finite-variance theorem.
- A finite proposal mixture does not prove event-mode completeness.
- The Gaussian half-space function is an analytic fixture, not a disk/ellipsoid collision integral or trajectory first-contact solver.
- The log-tail, box-mass and cube-tail routines are floating-point numerical kernels, not formally verified outward-rounded probability bounds; they cannot be used as an independent residual-mass certificate without additional enclosure work.
- The affine-cell module widens basic IEEE-754 operations by one adjacent representable value and fails closed on overflow. Its grid tests are useful negative controls, not a formal proof of the arithmetic implementation. It covers finite affine maps only; it is not a validated nonlinear ODE enclosure.
- The residual traversal bounds possible event mass only outside component-specific caller-declared chart boxes under a fixed standard-normal nominal model and static affine collision-ball event union, with chart-to-component assignments. A chart-covered region is not thereby proven to have accurate proposal density, correct likelihood weights, or complete event support.
- The polynomial trajectory classifier accepts coefficient intervals supplied by the caller. It does not construct or validate a Taylor remainder, establish the coefficient enclosures, solve nonlinear dynamics, or formally machine-verify its interval arithmetic. It is a fail-closed local component; ambiguous cases require high-fidelity evaluation.
- The Gaussian cell-mass bound is intentionally loose and the cube-tail bound uses a Chernoff polynomial lower bound. The traversal is preliminary numerical conservative bookkeeping, not a formally machine-verified certificate or a proof of the physical model.
- The cell splitter uses a deterministic longest-axis rule. Shared closed faces are covered by both children; they have zero measure only for a nondegenerate continuous Gaussian and still remain relevant to geometric tangency tests.
- The weighted indicator primitive does not construct confidence intervals or establish finite-sample coverage.
- The proposal density must be the complete mixture density, including all defensive components.
