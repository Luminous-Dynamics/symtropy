# CivDyn Robust Policy v0.1

A decision-analysis layer over the existing `civdyn-resilience-uq` and `civdyn-cross-scale-resilience` crates. It introduces no second physical simulator or stock-flow ledger.

## Paradigm

The benchmark asks which service-allocation policies remain defensible across a declared finite parameter design, where each fails, and how far each policy is from the best available policy in each matched design point.

It does **not** select a universal "best policy" by averaging water, energy, service and recovery into one score.

## Frozen V0 design

- 30 seeded Latin-hypercube parameter points from the existing seven-factor UQ design.
- Each point runs all 3 shocks × all 4 policies.
- Policies: no transfer, proportional solidarity, deficit-first and reserve-protected transfer.
- All policies are compared against the same parameter point and shock.
- LHS rows are a deterministic space-filling design measure, not calibrated event probabilities, iid simulations or posterior draws.

## Hard service constraints

Default thresholds are explicitly declared in `ServiceConstraints`:
- minimum household water coverage >= 0.95;
- minimum household energy coverage >= 0.95;
- water service-floor breach fraction <= 0.05;
- energy service-floor breach fraction <= 0.05.

A policy is labelled **finite-design robust-feasible** only when every declared LHS point passes every hard constraint. If any design point fails, the result is false. If no failure is observed but one or more outcomes are unavailable, the result is unknown. This finite-design flag is not a certificate over the continuous parameter box.

## Metric-specific regret

At each matched design point and shock, the four policies are compared independently for each of the nine UQ outcome metrics:
- unmet water (m³), unmet energy (kWh);
- water and energy deficit area;
- minimum water and energy coverage;
- water and energy service-floor breach fractions;
- recovery success.

For minimize metrics:
`regret = policy_value - best_policy_value`.

For maximize metrics:
`regret = best_policy_value - policy_value`.

The report gives mean, p95 and worst regret in each metric's native unit, plus the number/fraction of design points where each policy is tied for metric-best. It also counts distinct winners across the finite design. A tie can contribute to more than one policy's best_count; best fractions therefore need not sum to one.

## Missing outcomes

A missing policy result is `None`, not a zero. If any policy result is missing at a design point/shock, that design point is counted as an unknown comparison for every metric rather than computing regret against an incomplete opponent set. Constraint feasibility remains unknown unless a known failure is already present.

## Run

```sh
cargo fmt --manifest-path research/civdyn-robust-policy/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path research/civdyn-robust-policy/Cargo.toml --all-targets -- -D warnings
cargo check --locked --manifest-path research/civdyn-robust-policy/Cargo.toml --all-targets
cargo test --locked --manifest-path research/civdyn-robust-policy/Cargo.toml --all-targets
cargo run --locked --manifest-path research/civdyn-robust-policy/Cargo.toml --bin robust-policy-sweep
```

## Claim ceiling and V1

V0 assesses a finite deterministic parameter design with synthetic scenarios. It does not estimate real-world event probabilities, prove continuous robust feasibility, establish causal intervention effects or identify a globally optimal policy.

V1 should add source-grounded correlated parameter/structural uncertainty, held-out regional/historical shocks, independent stochastic replications, repeated-shock reserve-capital constraints and adaptive signpost policies under nonanticipativity (no use of future information).
