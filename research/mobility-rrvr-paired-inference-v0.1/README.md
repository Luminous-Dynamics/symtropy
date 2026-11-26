# RRVR Paired Cluster-Aware Inference v0.1

This isolated Rust crate computes preregistered paired effects for binary RRVR benchmark endpoints without collapsing safety and utility into a weighted scalar.

## Statistical contract

- Pair inputs are frozen and shared; realized control/treatment outcomes may differ.
- Outcomes are event, no_event, unknown or disputed.
- Unknown/disputed count conservatively as adverse for the declared endpoint estimand, and their counts remain separately visible.
- Primary risk difference is equal-cluster-weighted; episode-weighted rates are also reported descriptively.
- Bootstrap resamples whole clusters, not individual episodes.
- Exact McNemar is reported only when each pair is its own independent cluster.
- A zero-adverse-treatment-cluster result reports the exact one-sided upper bound (1 - \alpha^{1/n_{clusters}}). This is a bound on the cluster-level adverse probability, not a guarantee.
- No composite score is emitted.

The bootstrap PRNG is a versioned deterministic SplitMix64 profile for reproducibility; it is not a cryptographic generator.

## Run

- Format: `cargo fmt --manifest-path research/mobility-rrvr-paired-inference-v0.1/Cargo.toml -- --check`
- Evaluate: `cargo run --locked --manifest-path research/mobility-rrvr-paired-inference-v0.1/Cargo.toml --bin rrvr_paired_inference -- MANIFEST.json`
- Tests: `cargo test --locked --manifest-path research/mobility-rrvr-paired-inference-v0.1/Cargo.toml`
- Clippy: `cargo clippy --locked --manifest-path research/mobility-rrvr-paired-inference-v0.1/Cargo.toml --all-targets -- -D warnings`

## Claim ceiling

The first fixture is synthetic. This crate computes inference conditional on the preregistered campaign design and supplied paired records. It does not prove randomization, causal identification, actual independence of clusters, sampling representativeness, valid fault delivery, physical safety or benchmark reproduction. Those properties must be evidenced by the campaign. Cluster bootstrap intervals are not meaningful when there are too few independent clusters; the report says so explicitly.
