# habitat-observation-robustness

Standalone Rust verifier for the frozen synthetic v0.2 habitat observation-mode fixture.

Run from the repository root:

```sh
cargo fmt --manifest-path tools/habitat-observation-robustness/Cargo.toml -- --check
cargo test --locked --manifest-path tools/habitat-observation-robustness/Cargo.toml
cargo run --locked --manifest-path tools/habitat-observation-robustness/Cargo.toml -- docs/research/fixtures/HABITAT_OBSERVATION_ROBUSTNESS_V0_2.json
```

Report claim ceiling: `synthetic_observation_robustness_not_site_safety`. No physical or survey-quality claim is made.
