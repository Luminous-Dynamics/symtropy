# EHD PIV adaptive acquisition

A deterministic, event-triggered scheduler for choosing the next measurement action under a fixed acquisition budget. It is a scheduling policy reference, not a sensor, solver, safety controller or physically validated EHD measurement system.

## Run

    cargo fmt --manifest-path tools/ehd-piv-adaptive-acquisition/Cargo.toml -- --check
    cargo test --manifest-path tools/ehd-piv-adaptive-acquisition/Cargo.toml
    cargo clippy --manifest-path tools/ehd-piv-adaptive-acquisition/Cargo.toml --all-targets -- -D warnings

## Policy

- Low-cost sentinel metrics trigger capabilities, not an arbitrary composite score.
- Hard target requirements add independent velocity, direct force, full control-volume and electrical-power capabilities as needed.
- Available diagnostic actions are selected by a bounded set-cover search minimizing total cost, then latency, then stable action IDs.
- Any potentially perturbative or unknown-observer-impact diagnostic requires observer requalification after measurement.
- An already-unqualified observer state requires requalification before measurement; both boundaries are required when both conditions apply.
- A field-covariance target always requires high-resolution PIV, spatial-region coverage and reference qualification.
- Every action must require a replayable evidence receipt.
- If the catalog cannot cover all requirements within budget, latency and action-count limits, the output is explicit abstention.
- The scheduler never replaces missing direct-force evidence with PIV and never declares a physical claim qualified.

## Comparison baselines

Use identical sentinel sequences and measurement budgets to compare:
1. fixed periodic high-fidelity acquisition;
2. sentinel-only;
3. fixed-threshold escalation;
4. this deterministic risk/capability-aware set-cover policy;
5. budget-matched random scheduling;
6. a future robust EVSI / expected-information-gain policy.

Report detection recall by event class, false-trigger rate, detection latency, diagnostic cost, observer requalification count, evidence completeness, abstention reason and accepted-window coverage.

## Limits

The v0.1 policy consumes precomputed diagnostic metrics. It does not estimate them from images, infer causal dependence, estimate Bayesian information gain, choose operating conditions, control hardware or verify diagnostic receipts. Metric thresholds and action capabilities must be frozen by the experiment owner before evaluation.
