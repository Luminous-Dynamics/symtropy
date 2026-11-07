# CIVDYN Joint Population Fidelity v0.1

A dependency-free Rust reference for detecting a failure mode that marginal-only population metrics miss.

## Motivation

Two generated populations can have exactly the same marginal distribution for income and health while having a completely different relationship between them. A system that checks only each variable independently can incorrectly pass the simulation.

SSDataBench motivates extending population validation beyond univariate distributions to bivariate associations, multivariate outcomes, event sequences and sequence/covariate relationships. This crate implements only the first narrow slice: bivariate categorical dependence and declared-stratum comparisons.

## Metric contract

For a declared pair of categorical variables, the evaluator reports separately:

- Total variation distance for the left marginal.
- Total variation distance for the right marginal.
- Total variation distance for the full joint distribution.
- Mutual information for observed and simulated pairs, measured in nats.
- Absolute mutual-information gap.
- Row counts and distinct independent-unit counts for support visibility.

The evaluator never collapses these values into one score. Stratum comparisons fail closed when a stratum is absent from either population. Missing values are not treated as an extra zero category.

The `independent_unit_id` is retained so the next uncertainty layer can resample at the correct level. v0.1 reports descriptive discrepancies only: it does not produce p-values, confidence intervals, or population-inference claims. Repeated rows from one person must not be counted as independent evidence.

## Synthetic negative control

The tests compare:
- Observed: 50% low-income/low-health and 50% high-income/high-health.
- Simulated: 50% low-income/high-health and 50% high-income/low-health.

Both marginal distributions match exactly (TV = 0), but the joint supports are disjoint (joint TV = 1), and the observed mutual information is ln(2) nats while the simulated mutual information is 0.

This is a software-control fixture, not a human-data result or SSDataBench reproduction.

## Run

```sh
cargo fmt --manifest-path research/civdyn-joint-population-fidelity/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path research/civdyn-joint-population-fidelity/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path research/civdyn-joint-population-fidelity/Cargo.toml --all-targets
```

## Next increments

1. Cluster-aware resampling using the declared independent unit.
2. Conditional and multivariate outcome fidelity (not just two-variable categorical dependence).
3. Sequence/covariate association and life-course trajectory pattern fidelity.
4. Long-tail support, subgroup coverage and missingness diagnostics.
5. External validation on rights-cleared datasets under source-pinned protocols.
