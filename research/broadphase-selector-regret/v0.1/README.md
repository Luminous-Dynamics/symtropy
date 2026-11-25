# Broadphase Selector Regret Evaluator v0.1

Sequence-level, evidence-gated evaluator for adaptive broadphase portfolios.

## Why this exists

A per-frame minimum is not a physically attainable selector oracle: it can change backends every frame without paying rebuild, migration, warm-up or synchronization cost. This crate uses dynamic programming to compute the minimum-cost complete path across a held-out sequence, subject to pair-completeness qualification and explicit directed transition costs.

## Contract

- The evaluator accepts frozen backend/frame observations; it does not run broadphase algorithms or generate pair-oracle data.
- Every backend for a frame must share the same AABB snapshot, collision-filter digest, environment digest, oracle result digest and oracle denominators.
- A backend is eligible only if the receipt is valid, oracle coverage is complete, pair counts conserve, and it has zero false negatives, duplicate rows, unknown endpoints, unsupported cases and invalid prune certificates.
- A selected online backend must be qualified on that exact frame. Unqualified fast results cannot enter the score.
- Every held-out frame is retained in order. Frame indices are contiguous and start at zero.
- Selector decisions must occur before the simulation frame and disclose the latest completed frame used; future/current outcome leakage is rejected.
- Offline oracle cost includes full-step and directed transition costs. Online policy cost additionally includes selector overhead.
- Fixed-backend comparison requires that the backend be qualified for every frame in the entire sequence.
- Zero-positive oracle sequences are unavailable rather than receiving a fictitious perfect recall score.

## Run

```sh
cargo fmt --manifest-path research/broadphase-selector-regret/v0.1/Cargo.toml -- --check
cargo test --manifest-path research/broadphase-selector-regret/v0.1/Cargo.toml --locked
cargo clippy --manifest-path research/broadphase-selector-regret/v0.1/Cargo.toml --locked --all-targets -- -D warnings
```

## Metrics

Per sequence: online total cost, sequence-level offline oracle cost/path, best fixed qualified backend, regret, regression versus best fixed, switch count, selector/switch overhead, selected broadphase cost, p50/p95/p99 selected broadphase and full-step latency, and peak selected-path memory.

Per held-out family and campaign: median/p95 regret, p95 regression versus best fixed, median/p95 selector-plus-switch overhead, and an aggregate ratio of total selector/switch cost to total selected broadphase time. The aggregate ratio is the primary 2% overhead gate; per-sequence quantiles expose tail behavior.

Basis points use 100 basis points per percentage point (500 = 5%, 1,000 = 10%).

## Promotion targets

Targets are not achieved results: zero unqualified selections; median held-out regret <=5%; p95 regret <=10%; selector plus switch overhead <=2% of selected broadphase time on average; and no held-out family with >5% p95 regression versus its best fixed qualified backend. Compare whole scene sequences, not random frames from the same scene.

## Claim boundary

This measures replayed costs over a declared finite receipt corpus. It does not prove the oracle implementation, authenticate clocks or hardware telemetry, train a selector, establish global optimality, or show that a production physics engine is faster. The online policy must remain shadow-only until independently reviewed held-out evidence passes.
