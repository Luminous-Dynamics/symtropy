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

## Conditional innovation whitening

`whiten_conditional_innovation_sensitivities` accepts the existing and candidate sensitivity blocks plus `Sigma00`, `Sigma11`, and `Sigma10`. It computes the conditional sensitivity and Schur-complement covariance, then returns the whitened innovation. A repeated observation with high cross-correlation therefore contributes only its new independent information. The resulting rows can be passed to the candidate ranker. V0 is bounded to 512 total rows, requires every covariance block to be qualified, and rejects a non-SPD Schur complement rather than adding jitter.

## Observation covariance whitening

`whiten_sensitivity_with_covariance` performs Cholesky whitening for dense SPD covariance matrices with at most 512 observation rows. It computes `L^-1 J` so the whitened Gram matrix is `J^T Sigma^-1 J`. It rejects asymmetric, ragged, non-finite and singular covariance inputs; no symmetrization or jitter is silently introduced. This is a reference implementation only; large images require structured covariance operators with cross-pixel and cross-frame terms.

## Raw covariance-aware action ranking

`rank_covariance_aware_measurement_actions` is the end-to-end wrapper. It accepts raw baseline/candidate Jacobian blocks and covariance blocks, computes baseline whitening and the candidate Schur-complement innovations itself, and then ranks actions. Use this rather than manually passing assumed-independent pixel rows. Its manufactured control ranks an independent view above a highly correlated repeated view.

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

Condition number is the ratio of largest to smallest nuisance-profiled physical singular values; it is not computed using the unprofiled physical maximum.

The current manufactured suite contains 23 tests, including analytic correlated-noise oracles and fail-closed covariance validation.

The current manufactured suite has 27 tests, including the conditional innovation and Schur-complement cases.

The crate now contains 28 manufactured tests, including a raw-covariance action-ranking comparison between correlated repeated and independent measurements.
