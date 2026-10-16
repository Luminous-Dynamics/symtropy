# Symtropy Measurement Planner

A small deterministic Rust kernel for exact finite-state value-of-information (VOI) comparisons. It asks whether a feasible, permitted measurement action can improve a decision under an explicitly supplied finite set of possible worlds, probability model, decision utilities and measurement cost.

This is a decision-analysis calculator, not an agronomic model. It does not infer crop yields, create probabilities, choose nutrient targets, authenticate evidence, evaluate individual dietary needs or prove that a measurement is safe. Inputs and units remain the responsibility of their source-owning systems.

## Key paradigm

Do not optimize a plan from unsupported point estimates alone. When probability models are justified, compare the expected utility of acting now with the expected utility after observing a potential measurement. When the probability basis is unjustified, return not_estimable and no numeric EVSI. When probabilities are synthetic, arithmetic is available for tests but output remains synthetic_fixture_only, not a real recommendation.

Hard blockers cannot be traded away by a high expected-value score. A blocked action has no VOI score. Cost is supplied in the same scaled utility unit as the decisions; the model does not invent currency-to-utility exchange rates.

## Exact arithmetic

Probabilities and observation likelihoods are non-negative integer parts per million (PPM) and must sum exactly to 1,000,000. No floating-point math or rounding is used.

For each decision, baseline expected utility is the exact numerator sum(world_probability_ppm × utility_scaled), over a denominator of 1,000,000.

For each measurement outcome, posterior utility numerator is calculated without division: sum(prior_probability_ppm × outcome_likelihood_ppm × utility_scaled), over a denominator of 1,000,000,000,000.

The best feasible decision for each possible outcome maximizes that numerator. Gross EVSI is expected-after-measurement utility minus baseline utility converted to the common PPM-squared denominator. Net EVSI subtracts measurement cost on that denominator. Numerators are reported as decimal strings to preserve exact integers across JSON clients. Tie-breaking is deterministic by lexicographic decision ID.

Expected-value calculations are emitted only for calibrated_empirical or reviewed_expert_elicitation probability bases, and those actions must carry non-empty probability-model/version and calibration or reviewed-elicitation evidence references. Those labels and references are caller-declared: the crate does not fetch, authenticate or validate the calibration report or the sufficiency of the expert elicitation. Synthetic probability inputs remain synthetic-only even if their arithmetic is exact.

## Contract

- Scenario-wide structural errors (unsupported schema, invalid prior normalization, duplicate IDs, missing world utilities or no feasible decision) return blockers instead of a report.
- Each decision must define exactly one utility per declared world. A feasible decision has no hard blockers; an infeasible one must carry at least one.
- Every measurement action's likelihood matrix must define the same outcome IDs for every world and each world's likelihood row must sum to PPM.
- A measurement marked hard-infeasible or with inconsistent hard-gate metadata is blocked and receives no numeric score.
- An unjustified probability basis returns not_estimable with no numeric EVSI. The calculator does not quietly assume a uniform prior.
- Evidence-backed probability bases without both a stable model/version reference and a calibration or review-evidence reference return not_estimable. References are copied into the result, but are never authenticated by this crate.
- A synthetic-fixture probability basis produces arithmetic results only for mechanics tests and is labelled synthetic_fixture_only.
- Outputs are sorted by stable IDs. Input collection order does not affect result order or integer results.
- Arithmetic uses checked wide-integer operations. Nutrient/resource hard constraints must remain outside utility scoring; this kernel does not decide whether those constraints are legitimate.

## Run

    cargo fmt --manifest-path crates/domains/symtropy-measurement-planner/Cargo.toml -- --check
    cargo test --manifest-path crates/domains/symtropy-measurement-planner/Cargo.toml --locked
    cargo clippy --manifest-path crates/domains/symtropy-measurement-planner/Cargo.toml --locked --all-targets -- -D warnings

## Validation boundary

The suite includes a deterministic 100-case two-world comparison against a closed-form oracle, in addition to named hand-calculated and negative-control cases. A dedicated gate verifies that missing probability-model/evidence references prevent an evidence-backed score from being emitted. All generated cases remain synthetic. Passing them would establish arithmetic/contract behavior for those fixtures, not calibrated probabilities, correct local production forecasts, crop suitability, food safety or real-world benefit. See AGRI-VOI-001 (https://github.com/Luminous-Dynamics/symtropy/issues/1661) for proposed oracle, regret, calibration, performance and field-validation benchmarks.
