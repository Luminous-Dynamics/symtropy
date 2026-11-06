# CivDyn Uncertainty and Resilience v0.1

A parameter uncertainty and sensitivity laboratory that reuses the existing cross-scale resilience engine and its conserved stock-flow ledger. It does not create a second physical simulator.

## Why two designs?

The Latin-hypercube ensemble summarizes outcomes induced by a declared seven-dimensional design measure. It asks what outcome ranges appear under the specified parameter ranges.

Morris elementary-effects screening estimates which individual factors change outputs most strongly and which factors have context-dependent or nonlinear effects (high sigma). It is a screening method, not a variance decomposition.

These answer different questions and must not be conflated.

## Seven parameter factors

| Factor | Declared range |
|---|---:|
| Basin water supply multiplier | 0.7–1.3 |
| Basin energy supply multiplier | 0.7–1.3 |
| Household water/energy demand multiplier | 0.8–1.2 |
| Initial distributed reserve multiplier | 0.5–1.5 |
| Water shock retention | 0.0–0.6 |
| Energy shock retention | 0.0–0.6 |
| Shock duration | 2–5 ticks |

These are initial stress-test ranges, not empirical priors or fitted distributions. The LHS design assumes independent uniform marginal distributions; no evidence is claimed that real inputs are independent or uniform. Correlated inputs require source-grounded joint distributions in a later version.

## Design A — Latin hypercube

- 30 deterministic samples; fixed seed 0x4353535f55513031.
- Every factor uses each of the 30 equal-probability strata exactly once.
- Every parameter vector is run against 3 shocks × 4 intervention policies.
- Output: p05/p50/p95, mean, worst-decile tail mean, service-floor breach fraction and sustained-recovery success fraction.
- Water and energy outcomes remain separate.

The output quantiles and fractions describe the declared design measure only. They are not statistical confidence intervals, posterior predictive intervals, real-world event probabilities or independent stochastic simulation replications.

## Design B — Morris screening

- 30 trajectories over seven factors.
- Four-level grid, delta = 2/3, eight points per trajectory.
- Exactly one factor changes per step.
- Output reports mu-star (mean absolute elementary effect) and sigma (effect dispersion) for total unmet water, total unmet energy, minimum water coverage and minimum energy coverage.

A large mu-star indicates a factor with strong output influence over the sampled space. A large sigma indicates nonlinear effects or interactions may matter. Neither statistic is a causal effect or a Sobol index.

## Physical and scientific guardrails

- The cross-scale engine and resource ledger remain the sole authority for inventory, transfer and conservation.
- m³ and kWh never combine.
- No universal resilience score.
- Recovery is inherited from the reference engine: all four households must exceed the declared service floor for the required consecutive post-shock ticks.
- Zero or incomplete recovery is not silently replaced with an invented duration.
- The reference model is deterministic conditional on parameters. This v0.1 samples parameter uncertainty, not process stochasticity or structural uncertainty.
- No real-world geography, utility observations or household data are imported.

## Reproduction

```sh
cargo fmt --manifest-path research/civdyn-uncertainty-resilience/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path research/civdyn-uncertainty-resilience/Cargo.toml --all-targets -- -D warnings
cargo check --locked --manifest-path research/civdyn-uncertainty-resilience/Cargo.toml --all-targets
cargo test --locked --manifest-path research/civdyn-uncertainty-resilience/Cargo.toml --all-targets
cargo run --locked --manifest-path research/civdyn-uncertainty-resilience/Cargo.toml --bin uncertainty-resilience-sweep
```

## Next gates

1. Validate ranges and joint input distributions against source-backed regional observations.
2. Add structural uncertainty (topology, transfer rules, institutional latency, maintenance/degradation models).
3. Add separate within-parameter process seeds if stochastic processes are introduced; use 30+ independent replications for inferential claims and account for nested design.
4. Test on held-out historical shocks and independent regional observations.
5. Add robust decision metrics and adaptive trigger maps: satisficing robustness, minimax regret, service-floor regret and switching conditions.
