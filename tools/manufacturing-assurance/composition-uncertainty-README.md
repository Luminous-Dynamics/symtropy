# Correlation-preserving uncertainty composition (v0.1)

A dependency-free Rust reference for a narrow but important failure mode in composed digital twins: two subsystem outputs can share one or more latent uncertainty sources. Combining marginal variances as if the outputs were independent then gives the wrong downstream uncertainty.

This reference propagates a 2×2 covariance matrix through a linear map using:

```
U_y = A U_x A^T
```

It is a transparent analytical example, not a full implementation of the Guide to the Expression of Uncertainty in Measurement (GUM), not a Monte Carlo engine, and not a qualified production UQ service.

## Build and run

```sh
rustc --edition=2021 -D warnings --test tools/manufacturing-assurance/composition_uncertainty.rs -o /tmp/mfg-uq-tests
/tmp/mfg-uq-tests
rustc --edition=2021 -D warnings tools/manufacturing-assurance/composition_uncertainty.rs -o /tmp/mfg-uq
/tmp/mfg-uq
```

Or run the full assurance suite:

```sh
bash tools/manufacturing-assurance/run-tests.sh
```

## Known-truth example

Let both subsystem measurements share a common calibration bias with variance 4 K². Let subsystem A and B have independent local variances 1 K² and 2.25 K². Then:

- Var(A) = 5 K²
- Var(B) = 6.25 K²
- Cov(A,B) = 4 K²

For outputs sum = A+B and difference = A−B, the correct output covariance is:

- Var(A+B) = 19.25 K²
- Var(A−B) = 3.25 K²
- Cov(A+B,A−B) = −1.25 K²

The false independence model sets Cov(A,B)=0 and reports 11.25 K² for both outputs. It understates the sum variance by 41.56% and overstates the difference variance by a similar absolute amount. Correlation is not always harmful in one direction: its sign and the output function matter.

The fixture in `fixtures/shared-latent-covariance.tsv` is parsed by the Rust tests.

## Guardrails

The reference rejects non-finite covariance values, negative variances, asymmetric matrices outside tolerance, non-positive-semidefinite matrices, and non-finite transformation coefficients. It verifies identity transforms and equivalence of sequential and composed linear transformations.

## Claim ceiling

The implementation covers only a 2×2 real covariance matrix and deterministic linear transformations. It does not cover nonlinear propagation, non-Gaussian distributions, interval or epistemic uncertainty, Monte Carlo convergence, sparse high-dimensional matrices, temporal correlation, or decision risk. It must not be described as GUM-compliant or as an uncertainty certification.

The next phase should attach every covariance contribution to its physical source, scope, validity interval, unit, correlation group and immutable evidence digest, then propagate shared latent causes across model boundaries without silently assuming independence.
