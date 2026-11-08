# CIVDYN Joint Population Fidelity v0.2

A dependency-free Rust reference for testing whether simulated populations reproduce joint statistical structure, including uncertainty at the independent-unit level.

## v0.1 descriptive reference

The base evaluator reports marginal total-variation distances separately from joint total-variation distance and mutual-information differences. It detects cases where marginals match exactly while the association is destroyed. It also provides declared-stratum comparisons and preserves observation identity and independent-unit identity.

## v0.2 independent-unit cluster bootstrap

The `cluster_bootstrap` module adds percentile bootstrap intervals for:

- Left marginal total-variation distance.
- Right marginal total-variation distance.
- Joint-distribution total-variation distance.
- Observed mutual information (nats).
- Simulated mutual information (nats).
- Absolute mutual-information gap (nats).

### Resampling contract

1. Validate the original observation identities and categories.
2. Group rows by the declared `independent_unit_id`.
3. Require at least two independent units in each population.
4. For each replicate, sample the observed clusters with replacement, drawing the same number of clusters as originally observed.
5. Independently resample simulated clusters with replacement, drawing the same number as originally observed.
6. Retain every row belonging to each selected cluster. Repeated selections receive new bootstrap unit and observation IDs, so a sampled cluster is represented as a new draw rather than being rejected as a duplicate.
7. Recompute every descriptive metric and report the 2.5th and 97.5th percentile as a 95% percentile interval.

The bootstrap uses a deterministic SplitMix64 seed stream with rejection sampling for unbiased cluster-index selection. The default is 2,000 replicates; the API fails closed below 200 replicates. The seed and replicate count are inputs and should be frozen in the experiment manifest.

### Interpretation and limits

These intervals quantify sampling variability under the declared independent-cluster resampling design. They do not correct selection bias, establish population representativeness, handle survey weights, or create causal identification. The current implementation treats clusters as exchangeable and resamples the two populations independently. Paired agent/seed designs, stratified cluster sampling, informative cluster sizes, few-cluster corrections, and longitudinal missingness need separate protocols.

Unequal cluster sizes are retained in full. Therefore the target is the row-weighted pattern distribution under the observed cluster-size process; the implementation does not silently normalize every unit to equal row weight. For repeated-measures populations, this estimand must be considered before interpreting the interval.

The percentile interval is a transparent baseline, not a claim that coverage is guaranteed under every dependence structure. Empirical coverage needs to be tested on known-answer generators with varying cluster counts, imbalance, heavy tails, missingness and subgroup structure.

## Synthetic negative control

Observed pairs:
- 50% (low, low)
- 50% (high, high)

Simulated independent pairs:
- 25% each of (low, low), (low, high), (high, low), (high, high)

Expected point estimates:
- Left marginal TV = 0.
- Right marginal TV = 0.
- Joint TV = 0.5.
- Observed mutual information = ln(2) nats.
- Simulated mutual information = 0.

This is a software-control fixture, not a human-data result or a reproduction of SSDataBench.

## Run

```sh
cargo fmt --manifest-path research/civdyn-joint-population-fidelity/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path research/civdyn-joint-population-fidelity/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path research/civdyn-joint-population-fidelity/Cargo.toml --all-targets
```

## Next increments

1. Stratified cluster bootstrap and design-based weights.
2. BCa or studentized intervals with cluster-count diagnostics and coverage studies.
3. Conditional and multivariate fidelity beyond two categorical variables.
4. Sequence/covariate dependence and life-course trajectory distributions.
5. Long-tail support, subgroup coverage, missingness and attrition diagnostics.
6. Source-pinned external validation on rights-cleared datasets.
