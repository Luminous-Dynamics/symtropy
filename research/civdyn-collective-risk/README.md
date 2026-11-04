# CivDyn Collective Risk v0.1

A dependency-free Rust reference task for the six-seat collective-risk dilemma described in Huynh et al. (2026), *Cooperation Without Adaptation: LLM Agents in a Collective-Risk Dilemma*.

## Source-aligned task

- 6 seats, 10 simultaneous rounds.
- Each seat starts with 40 units.
- Allowed contribution per round: 0, 2 or 4.
- Shared target: 120 units.
- Reaching the target preserves each seat's unspent endowment.
- Missing the target destroys unspent savings with probability (p).
- Scoring uses expected payoffs analytically; the catastrophe lottery is not sampled.

The paper's welfare-optimal expected payoff has a pivot at (p^*=T/(6e)=0.5). Below the pivot, zero contribution is welfare-optimal; above the pivot, reaching the target is welfare-optimal; both modes are optimal at the pivot. This is a welfare benchmark, not a unique prediction of individual play.

## Implemented

- Deterministic simultaneous-round simulation with public previous-round contribution history.
- Transparent strategies: always-zero, fixed fair-share, fixed maximum, risk-pivot, last-round defection and copy-previous-group-mean.
- Expected payoff vector, group/per-seat welfare, target status, overshoot, shortfall and payoff Gini.
- Exhaustive best response against five scripted partners; the focal seat's maximum search space is (3^{10}=59,049) legal action-sequence candidates before endowment filtering.
- A risk-grid CSV sweep from (p=0.0) to (p=1.0), with fixed-partner best-response rows at low, pivot and high risk.

## Run

```sh
cargo fmt --manifest-path research/civdyn-collective-risk/Cargo.toml --all -- --check
cargo check --locked --manifest-path research/civdyn-collective-risk/Cargo.toml --all-targets
cargo test --locked --manifest-path research/civdyn-collective-risk/Cargo.toml --all-targets
cargo run --locked --manifest-path research/civdyn-collective-risk/Cargo.toml --bin collective-risk-sweep
```

## Scientific claim ceiling

This crate reconstructs the public game contract and its analytical proposition. It does **not** reproduce the paper's LLM prompts, model outputs, human/LLM sample, mixed-population selection process, or exact model-specific results. The scripted policies are inspectable baselines, not claims about real agents.

The next extension is a finite-population empirical game-theoretic analysis (N=6, 30 and larger) with hypergeometric table composition and payoff-biased imitation. Mechanisms from CoopEval (repetition, reputation, mediation and outcome-contingent contracts) must be explicit interventions, not inferred from a policy name.
