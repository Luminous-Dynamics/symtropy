# Interventional Dynamics Identifiability V1

A dependency-free Rust design diagnostic for linear state-space models used in intervention-driven neural and locomotion research.

**Parent:** [#3547 — LOCO-DYN-ID-001](https://github.com/Luminous-Dynamics/symtropy/issues/3547)  
**Related:** #3540 / #3548 (scalar IV baseline), #3375 (expected information gain), #3534 (source evidence), #3538 (connectome uncertainty), #3450 / #3454 (locomotion identifiability and blind prediction).

## The three questions are not interchangeable

For a declared linear system:

`s[t+1] = A s[t] + B u[t]`
`y[t] = C s[t]`

V1 computes three separate algebraic diagnostics:

1. **Observability:** can the latent state be distinguished from the output sequence? The finite-horizon observability matrix is stacked as C, CA, CA², ...
2. **Reachability / controllability:** which state directions can be reached from the input? The finite-horizon controllability matrix is [B, AB, A²B, ...].
3. **Experiment-design / parameter-regression rank:** does the caller-supplied realized regression design span the declared coefficient space? This design matrix must be constructed from correctly aligned state, input and history features by the caller.

A state-space model can be observable but poorly excited by the interventions. A state can be reachable but unobserved. A full-rank regression does not prove randomized intervention assignment, exclusion, temporal independence or causal identification.

## Numerical method

- Matrix dimensions and finite entries are validated.
- Rank is estimated using column-pivoted, twice-reorthogonalized modified Gram-Schmidt.
- Columns are normalized before rank estimation so that changing measurement units does not alone change the declared numerical rank.
- The returned diagonal-ratio condition proxy is explicitly a QR diagnostic, not a spectral condition number.
- A caller-selected relative tolerance is required; no universal tolerance is silently assumed.
- Input dimension, horizon and matrix element ceilings are bounded.

For difficult or near-singular research claims, compare against an independent SVD implementation in the benchmark harness. This small dependency-free crate is an auditable screening oracle, not a replacement for high-precision numerical linear algebra.

## Output interpretation

The report includes observability rank, reachability rank and parameter-design rank independently, their declared dimensions, full-rank flags, pivot orders and condition proxies. Rank deficiency is a positive diagnostic result, not an exception to be hidden by ridge regularization.

It does **not** return a causal estimate or assert that an identified subspace corresponds to unique biological cell states. Latent state coordinates can be equivalent up to invertible transformations; observable-space predictions should be the primary external comparison.

## Synthetic tests

Fixtures include:
- observable but not fully reachable systems;
- reachable but unobservable systems;
- systems that are both observable and reachable;
- rank-deficient experiment designs;
- column permutation and input unit-rescaling invariance;
- explicit dimensions, tolerance and non-finite input rejection.

All fixtures are synthetic. Passing them qualifies only the linear algebra and diagnostic implementation.

## Run

```sh
cargo fmt --manifest-path research/interventional-dynamics-identifiability-v1/Cargo.toml -- --check
cargo test --manifest-path research/interventional-dynamics-identifiability-v1/Cargo.toml --locked
cargo clippy --manifest-path research/interventional-dynamics-identifiability-v1/Cargo.toml --locked --all-targets -- -D warnings
```

## Research baselines this prepares

- Thams et al. (2024), JMLR 25(302), *Identifying Causal Effects using Instrumental Time Series: Nuisance IV and Correcting for the Past*. I.i.d. IV methods are not automatically consistent on temporally dependent data; history adjustment and process-specific identification conditions matter.
- Nejatbakhsh & Wang (2025), ICML / PMLR 267, *Identifying Neural Dynamics Using Interventional State Space Models*. Interventional SSMs target prediction under novel causal manipulations and provide identifiability results, beyond observational SSM fit.

This crate does not implement either paper's estimator. It provides transparent design-rank and finite-horizon system diagnostics that can be used before such estimators are run.
