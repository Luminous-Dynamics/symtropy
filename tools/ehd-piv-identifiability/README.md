# EHD-PIV local identifiability kernel

Dependency-free Rust diagnostics for distinguishing physical flow parameters from measurement nuisance parameters.

## Critical input contract

The row-major sensitivity/Jacobian matrix is in **whitened residual units**: measurement residual rows must already have been whitened by a qualified observation-noise covariance, including relevant correlations. Each parameter column is multiplied by its declared characteristic parameter scale. The resulting matrix has dimensionless parameter perturbations and unit-noise residual sensitivity.

If the residual noise covariance is unknown, correlated or not independently qualified, the caller must not pretend that raw pixels/vectors are independent unit-variance observations. This V0 kernel does not perform the whitening or authenticate covariance provenance.

## What it computes

- Deterministic one-sided Jacobi SVD, singular values and right singular directions.
- Joint numerical rank and condition.
- Nuisance-column span from the nuisance SVD.
- Projection of physical sensitivity columns onto the orthogonal complement of the nuisance span.
- Nuisance-profiled physical singular values, rank and condition.
- High-correlation physical/nuisance parameter pairs and weak joint parameter combinations.
- Cost-aware candidate acquisition ranking by rank gain and weakest nuisance-profiled physical sensitivity gain.

The central diagnostic is not merely whether the joint Jacobian is full rank. It is whether physical sensitivity remains after nuisance directions are allowed to explain the same observations. A deficiency confined to nuisance parameters can coexist with identifiable physical parameters.

## Candidate measurement ranking

The crate accepts candidate actions whose new rows are conditional-whitened innovations, including covariance with existing observations. It ranks actions by physical rank gain, smallest nuisance-profiled physical singular-value gain per cost, then deterministic tie-break. A candidate that only adds nuisance information does not earn a physical-information recommendation. V0 is a local E-optimal-style heuristic, not full EVSI or D-optimal Bayesian design, and does not actuate hardware.

## Interpretation

A physical status of `Identifiable` means local sensitivity is full-rank after nuisance profiling and its condition is under the configured limit. `IllConditioned`, `RankDeficient` and `NoInformativeSensitivity` must not be converted to a positive physical claim.

A full-rank local Jacobian is not a proof of global/structural identifiability, unimodal posterior, unbiased estimator or model correctness. Priors and regularization can stabilize an optimizer without adding information to the observations; prior-dominated inference must be labelled separately.

## Run

    cargo fmt --manifest-path tools/ehd-piv-identifiability/Cargo.toml -- --check
    cargo test --manifest-path tools/ehd-piv-identifiability/Cargo.toml
    cargo clippy --manifest-path tools/ehd-piv-identifiability/Cargo.toml --all-targets -- -D warnings

## Manufactured tests

The current crate contains 14 deterministic tests, including exact and near flow/nuisance confounding, nuisance-only rank deficiency, absent physical sensitivity, parameter scaling, row permutation, underdetermined systems, malformed data and decomposition non-convergence.

The crate currently contains 18 deterministic manufactured tests, including candidate actions that resolve or preserve flow/gain confounding, cost-aware ranking, and nuisance-only actions that receive no physical-information credit.
