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


## Scenario B: stakeholder preference robustness

The preference_robust module and preference-robust-sweep binary extend the metric-specific regret analysis without replacing it or creating a second simulator.

### Contract

A versioned preference contract declares:
- an ordered criterion vocabulary;
- direction for each criterion;
- an externally declared positive reference scale and evidence digest for each criterion;
- stakeholder/profile IDs;
- a normalized nonnegative weight vector for each preference vertex;
- the existing hard service constraints.

The reference scales are fixed before evaluation. They are not fitted from challenge outcomes. The five-criterion reference profile uses unmet water (m³), unmet energy (kWh), minimum water coverage, minimum energy coverage and recovery success.

Each profile utility is a transparent linear sum of signed, dimensionless criterion outcomes. The same scenario's feasible policies are compared under that profile only. A policy that fails a non-compensatory water or energy service floor cannot win because it performs well on another objective. Missing outcomes are UNKNOWN and make the preference comparison incomplete; they are never set to zero.

### Preference polytope and probabilities

The declared profile vectors are treated as the complete vertices of a finite preference polytope only when the contract author explicitly attests that completeness. For linear utility in the weights, worst-case regret over the convex hull is attained at an extreme point, so evaluating every declared vertex is sufficient for that declared polytope.

The software does not infer a population distribution over stakeholder values. Profile counts, vertex counts and best-profile counts are descriptive only—not probabilities, votes or consensus. If the supplied vectors are merely a sample of opinions, the result is finite-panel sensitivity only, not a polytope-robust guarantee.

### Outputs

The report preserves:
- per-profile, per-scenario, per-policy utility, regret, feasibility and Pareto-nondominance;
- per-profile/policy mean, p95 and worst normalized regret;
- per-policy finite-design feasibility across all scenarios;
- worst-case regret across declared preference vertices only for policies that are finite-design robust-feasible and whose full comparison set is observed;
- preference winner disagreement and cases where every profile selects the same single policy;
- explicit no-feasible-policy and incomplete-comparison outcomes.

There is no composite score across stakeholders, no universal winner and no probability attached to LHS or preference vertices. A minimax-regret policy is reported only relative to the declared finite preference polytope and scenario design.

### Known-answer and adversarial controls

Unit tests include:
1. water-priority and energy-priority vertices that select different policies in the same matched scenario;
2. a high-utility policy barred from winning when it violates an essential service floor;
3. identical policy outcomes producing zero regret and explicit ties;
4. no-feasible-policy abstention;
5. missing outcomes remaining unknown;
6. common unit-rescaling with a matching reference scale preserving utility;
7. invalid/duplicate preference contracts and non-normalized weights;
8. missing and inconsistent matched scenario records failing closed.

Run preference-robust-sweep with the locked manifest at research/civdyn-robust-policy/Cargo.toml. Its CSV retains native outcomes, criterion ordering, directions, scales and weights next to each utility/regret row.

### Claim ceiling

This is a synthetic finite-design and finite-profile benchmark. It does not establish that the declared stakeholder profiles are representative, legitimate or exhaustive. It does not infer moral preferences, real-world population weights or distributional probabilities. Preference contracts require external authorship, consent where appropriate, review and versioning. Robustness is only over the exact scenarios, hard constraints, criteria, reference scales and preference vertices in the frozen contract.
