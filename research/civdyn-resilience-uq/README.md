# CivDyn Resilience UQ v0.1

A deterministic parameter-design and global sensitivity reference built on the existing cross-scale resilience simulator and shared conserved resource ledger.

## What the experiment means

The benchmark explores consequences across a declared *parameter design*. Its 30 Latin-hypercube rows are not 30 stochastic simulation replications, not empirically calibrated priors, and not predictive confidence intervals. There is no process randomness in this v0.1 experiment. The fixed seed determines design coordinates only.

The uniform marginal ranges are software-design assumptions for sensitivity screening, not empirical claims:

| Factor | Range | Mapping |
|---|---:|---|
| Basin water supply multiplier | 0.75–1.25 | Linear |
| Basin energy supply multiplier | 0.75–1.25 | Linear |
| Household demand multiplier | 0.85–1.35 | Linear |
| Distributed district/household reserve multiplier | 0.50–1.50 | Linear |
| Water-shock supply retention | 0–0.50 | Linear |
| Energy-shock supply retention | 0–0.50 | Linear |
| Shock duration | 2–4 ticks | Rounded discrete mapping |

Water and energy remain separate, in m³ and kWh. A parameterized scenario is run under all 3 shock types and all 4 declared transfer policies.

## Design A — Latin hypercube

The default design uses 30 samples. Each factor occupies every one of 30 equal-width unit-hypercube strata exactly once. The executable reports each design coordinate, its mapped physical parameter, and every shock/policy outcome.

For each outcome, shock and policy, it reports mean, linearly interpolated p05/p50/p95, bottom/top 10% tail means, and the fraction of design runs breaching the metric's declared service/zero condition.

These are descriptive summaries under this LHS design measure. They are not calibrated probabilities of real events or posterior credible intervals.

## Design B — Morris screening

The default design uses 30 trajectories, 7 factors, 4 grid levels and delta = 2/3. Each trajectory has 8 points (base plus one move per factor). Each elementary effect changes exactly one factor.

For every shock/policy/outcome/factor combination, the benchmark reports:
- μ: mean signed elementary effect;
- μ*: mean absolute elementary effect;
- σ: sample standard deviation of elementary effects.

μ* is a screening statistic; σ indicates effect variation/interactions/nonlinearity but does not uniquely identify their cause. This is not Sobol variance decomposition, causal attribution, or a posterior sensitivity estimate. The discrete shock-duration factor has a quantized mapping, so its Morris effects should be interpreted as finite changes over the declared 2–4 tick range.

## Outcomes

Water unmet demand, energy unmet demand, water/energy deficit area, minimum water/energy coverage, water/energy service-floor breach fraction and recovery success are retained separately. There is no universal resilience scalar.

## Run

```sh
cargo fmt --manifest-path research/civdyn-resilience-uq/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path research/civdyn-resilience-uq/Cargo.toml --all-targets -- -D warnings
cargo check --locked --manifest-path research/civdyn-resilience-uq/Cargo.toml --all-targets
cargo test --locked --manifest-path research/civdyn-resilience-uq/Cargo.toml --all-targets
cargo run --locked --manifest-path research/civdyn-resilience-uq/Cargo.toml --bin resilience-uq-sweep
```

## Claim ceiling and next gates

The reference checks deterministic design construction, parameter-to-scenario mapping, native-unit outcome propagation and sensitivity arithmetic on synthetic scenarios. It does not establish regional parameter distributions, real-world likelihoods, empirical forecast accuracy, policy optimality or a causal effect.

V1 must source and justify parameter ranges and correlations, add structural alternatives (topologies, institutional delay, endogenous maintenance), and add true independent stochastic replications nested within selected parameter points. Decision recommendations require robust decision analysis, regret and adaptation-trigger tests over held-out conditions.
