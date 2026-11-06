# CivDyn Operational Validity v0.1

A dependency-free reference kernel for comparing independent empirical windows with independent simulation replications.

## What it measures

Each metric is compared separately within an explicit stratum and unit:
- sample count, mean, sample standard deviation and median;
- p10/p50/p90/p99 quantiles;
- descriptive two-sided 99% t intervals for the two sample means;
- two-sample Kolmogorov–Smirnov distance;
- one-dimensional empirical Wasserstein-1 distance;
- simulated-minus-observed mean and quantile deltas.

Each metric declares separate minimum observed and simulated sample counts, their independent-unit definitions, a dependence assessment and a sample-size rationale. The kernel does not impose a universal count of 30. Sample IDs are unique within each side, and each SampleSet must match its declared unit and independent-unit definition. These declarations are protocol assertions; unique IDs cannot prove that windows, groups or runs are genuinely independent. Empty, duplicate, non-finite, undersized, unit-mismatched or independence-unit-mismatched inputs are rejected.

## Important statistical limits

- The kernel compares independent samples; it does not assume paired windows merely because sample counts match.
- KS and Wasserstein-1 are descriptive distances. This v0.1 does not emit p-values or equivalence decisions.
- The 99% mean intervals use conservative Student-t critical values, including small-sample degrees of freedom. They describe uncertainty in each sample mean; overlapping intervals are not proof of equivalence.
- Dependence between adjacent empirical windows, overlapping data, seasonality and measurement error must be handled by the domain adapter and sampling design. The kernel cannot make dependent windows independent.
- A good distributional match does not prove the model used the right mechanism. Causal/path validation and ablations remain separate.
- No pooled operational-validity score is emitted. Metrics and strata remain separate.

## Synthetic demonstration

The `operational-validity-sweep` binary emits five synthetic patterns inspired by recent operational validation work: activity, thread depth, network core concentration, root-post toxicity and comment toxicity. All values are artificial software fixtures. They are not observations and do not reproduce any published data.

## Run

```sh
cargo fmt --manifest-path research/civdyn-operational-validity/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path research/civdyn-operational-validity/Cargo.toml --all-targets -- -D warnings
cargo check --locked --manifest-path research/civdyn-operational-validity/Cargo.toml --all-targets
cargo test --locked --manifest-path research/civdyn-operational-validity/Cargo.toml --all-targets
cargo run --locked --manifest-path research/civdyn-operational-validity/Cargo.toml --bin operational-validity-sweep
```

## Evidence claim ceiling

This crate validates a comparison kernel on synthetic data. It does not establish operational validity for a social or civilizational model. A real benchmark needs licensed, source-pinned empirical windows, predeclared patterns, calibration/holdout separation, adequate independent samples and purpose-aligned mechanism diagnostics.


## Design-specific sample-size contract

The library requires each caller to set:
- minimum observed sample count;
- minimum simulated sample count;
- observed and simulated independent-unit definitions;
- a dependence assessment;
- a sample-size rationale.

The threshold is not hard-coded. The current synthetic forum-style fixture uses 30 empirical windows and 30 independent simulation runs to match the cited study design. Another benchmark may declare a different threshold where its design/power rationale supports it. The comparator checks declared units and unique IDs but cannot establish statistical independence from metadata alone.
