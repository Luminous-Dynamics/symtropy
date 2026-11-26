# RRVR Paired Cluster-Aware Inference v0.1

This isolated Rust crate computes preregistered paired descriptive effects for binary RRVR endpoints without collapsing safety and utility into a weighted scalar.

## Statistical contract

- Pair inputs are frozen and shared; realized control/treatment outcomes may differ.
- Outcomes are event, no_event, unknown or disputed.
- Unknown/disputed count conservatively as adverse for the declared endpoint estimand, and their counts remain separately visible.
- Primary risk difference is equal-cluster-weighted; episode-weighted rates are also reported descriptively.
- Bootstrap resamples whole clusters, not individual episodes.
- Percentile cluster-bootstrap intervals are emitted as **exploratory descriptive intervals only** in v0.1. Their coverage is not simulation-qualified, and v0.1 never labels them inferential, regardless of draw count.
- McNemar is withheld in v0.1. Unique cluster IDs do not establish independent sampling units; an external, independently reviewed campaign-design attestation is required before conditional exact-pair inference can be enabled.
- The zero-adverse-cluster upper bound is shown only with an explicit assumption label. It is conditional on IID independent cluster-level Bernoulli outcomes; this crate does not verify those assumptions.
- `control_event_state_n` and `treatment_event_state_n` count the literal `Event` enum state. For `maximize_success` endpoints, this is a success count.
- No composite score is emitted.

The bootstrap PRNG is a versioned deterministic SplitMix64 profile for reproducibility; it is not a cryptographic generator.

## Run

- Format: `cargo fmt --manifest-path research/mobility-rrvr-paired-inference-v0.1/Cargo.toml -- --check`
- Evaluate: `cargo run --locked --manifest-path research/mobility-rrvr-paired-inference-v0.1/Cargo.toml --bin rrvr_paired_inference -- MANIFEST.json`
- Tests: `cargo test --locked --manifest-path research/mobility-rrvr-paired-inference-v0.1/Cargo.toml`
- Clippy: `cargo clippy --locked --manifest-path research/mobility-rrvr-paired-inference-v0.1/Cargo.toml --all-targets -- -D warnings`

## Claim ceiling

The first fixture is synthetic. This crate emits paired descriptive estimates conditional on the supplied manifest and declared cluster mapping. It does not prove randomization, causal identification, actual cluster independence, sampling representativeness, valid fault delivery, physical safety or benchmark reproduction. Confirmatory inference requires an independently justified sampling/cluster design and simulation-validated coverage for the chosen estimator. A large bootstrap draw count cannot compensate for a small number of independent clusters.
