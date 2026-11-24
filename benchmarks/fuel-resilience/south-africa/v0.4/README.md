# SAFRB v0.4 — weighted disruption scenario tournament

**Status:** standalone scorecard kernel; synthetic fixtures only  
**Research cut:** 2026-11-23  
**Language:** Rust 2024, dependency-free  
**Parent:** SAFRB v0.2 network accounting and v0.3 deterministic min-cost-flow planner

## Why v0.4 exists

A candidate plan can look good on average and still fail catastrophically during rare compound disruptions. v0.4 provides a reproducible scorecard for weighted scenario outcomes and a strict matched comparison between baseline and candidate policies.

The scorecard reports expected shortages, critical-service rate, VaR95/CVaR95, worst-case shortage, weighted logistics cost and zone-level tail risk. CVaR uses a fractional boundary scenario and rounds the reported litre result upward, avoiding the common mistake of taking the largest 5% of scenario *counts* when the scenarios carry unequal probabilities.

## Metrics

- **Expected shortage:** probability-weighted mean, rounded upward to whole litres.
- **Critical service rate:** weighted critical litres delivered / weighted critical litres required.
- **VaRα:** smallest shortage value whose cumulative scenario probability reaches α.
- **CVaRα:** expected shortage in the worst (1−α) probability mass, including fractional use of the boundary scenario.
- **Worst case:** largest observed loss in the supplied scenario suite.
- **Zone-level risk:** the same expected/VaR/CVaR/worst metrics per critical demand zone.
- **Matched comparison:** rejects differences in scenario IDs, weights or critical demand frame before calculating CVaR reduction, service-rate delta and cost delta.

## Distributionally robust probability ambiguity

v0.4 also accepts per-scenario lower/upper probability bounds in basis points. The bound set must contain every scenario exactly once, each interval must be valid, and the intervals must admit a distribution summing to 10,000 basis points.

The implementation starts at the lower bounds and allocates remaining probability mass to scenarios in descending critical-shortfall order, respecting each upper bound. This yields a worst-case upper-tail distribution under the independent interval / simplex ambiguity set. It then reports worst-case expected critical shortage and worst-case CVaR. The selected probability vector is returned as a witness for audit.

These bounds are not learned probabilities. They must be calibrated from defensible event histories, independently reviewed expert elicitation or an explicitly documented uncertainty set. When no defensible probability bounds exist, report unweighted stress results and do not relabel them as real-world probabilities.

## Evidence and statistical boundaries

- Scenario weights are relative weights supplied by the benchmark designer. They are not objectively calibrated probabilities by default.
- Equal weights describe an empirical scenario suite, not real-world event likelihood.
- CVaR is only meaningful relative to the scenario distribution and loss definition. Always publish alternative weights, unweighted stress results and sensitivity.
- A good expected shortage or CVaR does not prove that critical services have socially acceptable priority rules. Publish zone-level and class-level outcomes.
- Cost units are abstract integers. Do not interpret as currency without sourced tariff units.
- This crate scores outcomes; it does not run a planner, verify fuel inventory or authorise allocations.

## Run

```sh
cargo fmt --manifest-path Cargo.toml -- --check
cargo test --manifest-path Cargo.toml --locked
cargo clippy --manifest-path Cargo.toml --all-targets --locked -- -D warnings
cargo run --manifest-path Cargo.toml --release --locked
```

## Tournament protocol

Compare greedy, min-cost flow, LP/MILP, two-stage stochastic, robust/distributionally robust and Symthaea-assisted policies on the same:
- topology and grade constraints;
- decision-time inventory and cargo evidence;
- demand locations/priorities;
- outage set and scenario IDs/weights;
- route costs, recovery window and compute budget.

Report both probability-weighted outcomes and unweighted worst-case stress. If scenario probabilities are not calibrated, present CVaR only as a conditional scenario-suite metric, not a real-world risk estimate.

## Claim ceiling

A passing test run establishes the software behavior of this scorecard for synthetic outcomes. It does not establish the probability of South African fuel shocks, a calibrated national CVaR, or the superiority of any allocation policy.
