# Symtropy Rare-Event Kernel

An isolated, dependency-light Rust package for mathematical primitives used by the orbital conjunction rare-event assurance programme.

This crate is a research kernel, not an operational collision-probability product. It deliberately does not implement orbit propagation, random sampling, event-mode discovery, proposal fitting, confidence intervals, or flight operations.

## Current primitives

- Strict positive-definite covariance validation using a Cholesky factor.
- Multivariate Gaussian log density without explicit matrix inversion.
- Gaussian half-space probability and log-probability, reduced to a one-dimensional normal CDF.
- Stable log-domain standard-normal interval mass, including extreme positive and negative tails.
- Log mass for axis-aligned boxes under independent standard-normal coordinates.
- A Gaussian cube-tail union-bound primitive for probability bookkeeping.
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

## Important limitations

- The covariance floor helper checks only the matrix inequality. It does not prove all assumptions of the constrained-mixture finite-variance theorem.
- A finite proposal mixture does not prove event-mode completeness.
- The Gaussian half-space function is an analytic fixture, not a disk/ellipsoid collision integral or trajectory first-contact solver.
- The log-tail, box-mass and cube-tail routines are floating-point numerical kernels, not formally verified outward-rounded probability bounds; they cannot be used as an independent residual-mass certificate without additional enclosure work.
- The weighted indicator primitive does not construct confidence intervals or establish finite-sample coverage.
- The proposal density must be the complete mixture density, including all defensive components.
