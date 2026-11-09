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


## Structural model-set decision robustness (v0.2 research slice)

The model_set_robustness module adds a second uncertainty axis alongside the V0 parameter design: disagreement between structurally distinct candidate models.

It does not duplicate the simulator, parameter UQ design or resource ledger. It consumes per-model policy outcomes and explicit model-qualification receipts. A candidate with no_primary_discrepancy_detected is eligible for a bounded comparison. Candidates marked primary-discrepancy-detected, not-assessed or out-of-domain are retained in the qualification report but excluded from the qualified-model comparison. If no candidate qualifies, the result is not assessed.

A model with no detected discrepancy is not thereby proven true. A policy stable across the qualified set is stable only within that declared set; the report also exposes whether model-set coverage is partial.

### Decision contract

- Every candidate model, case, policy and metric has a declared identity/digest.
- Outcomes must match the model, case and policy digests in the manifest.
- Objective metrics retain their own direction and native unit.
- Hard constraints are evaluated separately and cannot be traded against another objective.
- Missing outcomes or missing metric values remain unknown; there is no partial oracle.
- A policy is robust-feasible only if it passes every hard constraint in every qualified model and matched case.
- Objective-wise winners and ties are calculated separately for each model/case.
- Rank reversals, winner-set intersection and model-family counts are reported explicitly.
- Worst-case regret and minimax policy are reported per objective, only for policies feasible across the entire qualified set and with complete comparisons.
- No model probabilities are inferred and no universal cross-objective score is emitted.

The initial reference fixture has two qualified structural models and one model excluded for a detected primary discrepancy. Both qualified models share the same three policies and two matched shock cases. The preferred policy reverses between models, only the reserve-protected policy remains feasible in all qualified model/case combinations, and its worst-case unmet-water regret is 3 m3. The excluded candidate is reported, so this result cannot be mistaken for a complete model-set guarantee.

Run the model-set-decision-sweep binary with the locked manifest at research/civdyn-robust-policy/Cargo.toml. The existing civdyn-robust-policy workflow runs this known-answer sweep and checks the qualification coverage, ranking reversal, robust feasibility and metric-specific minimax regret outputs.

### Claim ceiling

This is a finite, synthetic decision-set reference. It does not demonstrate that the candidate model set is complete, that model families are independent, that an adequacy-qualified model is true, or that a policy is optimal in reality. It does not assign equal probabilities to models. The minimax-regret result is conditional on the declared models, matched cases, constraints and objective definitions. A partial model set must remain labelled partial.


## Structural model-set decision robustness (v0.2 research slice)

The model_set_robustness module adds a second uncertainty axis alongside the V0 parameter design: disagreement between structurally distinct candidate models.

It does not duplicate the simulator, parameter UQ design or resource ledger. It consumes per-model policy outcomes and explicit model-qualification receipts. A candidate with no_primary_discrepancy_detected is eligible for a bounded comparison. Candidates marked primary-discrepancy-detected, not-assessed or out-of-domain are retained in the qualification report but excluded from the qualified-model comparison. If no candidate qualifies, the result is not assessed.

A model with no detected discrepancy is not thereby proven true. A policy stable across the qualified set is stable only within that declared set; the report also exposes whether model-set coverage is partial.

### Decision contract

- Every candidate model, case, policy and metric has a declared identity/digest.
- Outcomes must match the model, case and policy digests in the manifest.
- Objective metrics retain their own direction and native unit.
- Hard constraints are evaluated separately and cannot be traded against another objective.
- Missing outcomes or missing metric values remain unknown; there is no partial oracle.
- A policy is robust-feasible only if it passes every hard constraint in every qualified model and matched case.
- Objective-wise winners and ties are calculated separately for each model/case.
- Best-policy intersection, distinct winner counts and model-family counts are reported explicitly.
- Worst-case regret and minimax policy are reported per objective, only for policies feasible across the entire qualified set and with complete comparisons.
- No model probabilities are inferred and no universal cross-objective score is emitted.

The initial reference fixture has two qualified structural models and one model excluded for a detected primary discrepancy. Both qualified models share the same three policies and two matched shock cases. The preferred policy reverses between models, no single best policy persists across all model/case combinations, only the reserve-protected policy remains feasible in all qualified model/case combinations, and its worst-case unmet-water regret is 3 m3. The excluded candidate is reported, so this result cannot be mistaken for a complete model-set guarantee.

Run the model-set-decision-sweep binary with the locked manifest at research/civdyn-robust-policy/Cargo.toml. The existing civdyn-robust-policy workflow runs this known-answer sweep and checks the qualification coverage, ranking reversal, robust feasibility and metric-specific minimax regret outputs.

### Claim ceiling

This is a finite, synthetic decision-set reference. It does not demonstrate that the candidate model set is complete, that model families are independent, that an adequacy-qualified model is true, or that a policy is optimal in reality. It does not assign equal probabilities to models. The minimax-regret result is conditional on the declared models, matched cases, constraints and objective definitions. A partial model set must remain labelled partial.
