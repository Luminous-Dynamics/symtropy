# RRVR Executable Scorecard v0.1

This isolated Rust crate turns raw RRVR episode records into deterministic per-arm scorecards. It is stacked on the matched benchmark protocol v0.2 and does not replace the native task/recovery benchmark metrics.

## Run

- Format: cargo fmt --manifest-path research/mobility-rrvr-benchmark-v0.2/evaluator/Cargo.toml -- --check
- Evaluate: cargo run --locked --manifest-path research/mobility-rrvr-benchmark-v0.2/evaluator/Cargo.toml --bin rrvr_benchmark_evaluate -- research/mobility-rrvr-benchmark-v0.2/evaluator/fixtures/rrvr_results_v0_1.json
- Test: cargo test --locked --manifest-path research/mobility-rrvr-benchmark-v0.2/evaluator/Cargo.toml
- Clippy: cargo clippy --locked --manifest-path research/mobility-rrvr-benchmark-v0.2/evaluator/Cargo.toml --all-targets -- -D warnings

## Input and matching contract

The input format is versioned and strict. Unknown fields reject. Every episode declares a pair ID and arm. A pair must contain at least two unique arms and exactly one native and one full_rrvr arm. All arms in a pair must match on task, policy digest, scene digest, initial-state digest, action-prefix digest, seed, simulator profile, fault schedule, observer profile and expected disposition. Independent oracle dispositions must also agree across arms in a pair. Mismatches reject the entire bundle; the evaluator never repairs or silently drops pairs.

The native_benchmark_preserved assertion must be true for every record. It prevents the overlay from replacing or suppressing the host benchmark's native outcome fields.

## Scorecards

Each arm emits native task success, native episode safety, native policy-recovery success, fault request/acceptance/execution/independent-observation counts, false NoEffect, false Applied, false Closed, duplicate one-shot effects, stale effects after accepted fencing, unsafe admissions, false holds, Unknown/Disputed, evidence yield, latency percentiles and cluster cardinalities.

Every metric contains eligible, adjudicated, event and unresolved counts. A zero denominator serializes as null, not zero. Missing physical measurement remains unresolved rather than being counted as a safe result. Injector-stage contradictions are visible counters, not silently discarded.

Latency quantiles use the nearest-rank method over nonnegative supplied samples and always report sample count. The evaluator reports distinct task, policy, scene, seed, world, robot asset, hardware, firmware, fault-schedule and observer-profile clusters. It does not fabricate an independence assumption or confidence interval.

## Claim ceiling

The fixture is synthetic and qualifies evaluator behavior only. It is not an executed robot campaign, fault-delivery proof, independent physical observation, SOTA comparison, HIL result, physical safety claim, PL/SIL result, certification or field-reliability estimate.
