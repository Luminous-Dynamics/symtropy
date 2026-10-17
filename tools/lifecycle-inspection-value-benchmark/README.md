# Exact finite value-of-inspection benchmark

Standalone Rust reference benchmark for [Symtropy #1881](https://github.com/Luminous-Dynamics/symtropy/issues/1881). It measures whether a hypothetical inspection is worth its declared cost because its possible outcomes change a maintenance decision. The benchmark evaluates consequences; it is **not** a diagnostic model, inspection scheduler, or authorization path.

## Run

```sh
cargo fmt --manifest-path tools/lifecycle-inspection-value-benchmark/Cargo.toml -- --check
cargo test --manifest-path tools/lifecycle-inspection-value-benchmark/Cargo.toml
SOURCE_REVISION="$(git rev-parse HEAD)" cargo run --manifest-path tools/lifecycle-inspection-value-benchmark/Cargo.toml
```

## Frozen decision problem

Two hidden states have explicit synthetic integer prior weights. Three available actions have declared losses in each state. Each inspection profile supplies integer likelihood weights for an alarm outcome, with the complementary weights assigned to a clear outcome. Likelihoods for both hidden states sum to the same declared denominator.

The objective is the expected total loss numerator, including inspection cost. It is kept on a common integer denominator so the benchmark does not use floats or quietly invent priors. Every binary-test policy is compared against an exhaustive enumeration of all action assignments to alarm/clear outcomes. The no-inspection baseline and perfect-information lower bound are computed separately.

The fixture intentionally demonstrates that a cheaper imperfect check can provide more net decision value than a more accurate but expensive test, while a test that does not change the selected action can have negative net value.

## Commands and outputs

The report binds the exact raw manifest SHA-256, source revision, and tool version. Every candidate reports the action for each observable outcome, decision-loss numerator before inspection cost, inspection-cost numerator, total-loss numerator, gross expected value of sample information (EVSI), net EVSI, and independent exhaustive-policy agreement.

The perfect-information result is a lower bound on decision loss. Gross EVSI cannot exceed the maximum gross value of perfect information. Net EVSI may be negative.

## Research anchors

- Dadfarnia, Sharp & Herrmann (2025), systematic review of evaluating condition-monitoring benefits in industrial maintenance: https://doi.org/10.1016/j.jmsy.2025.06.015. The review highlights the need to make the monitoring method, maintenance policy, engineering/financial outputs and uncertainty assumptions explicit.
- NIST publication page for that review: https://www.nist.gov/publications/comprehensive-evaluations-condition-monitoring-based-technologies-industrial
- Crespo del Castillo & Parlikad (2024), *Evaluating Investment in Condition Monitoring for Fleet Maintenance*, explicitly discusses remaining-useful-life uncertainty and investment value: https://doi.org/10.1016/j.ifacol.2024.08.150
- Evaluation of the Digital Product Passport for Remanufacturing (2025), controlled two-laboratory study examining how lifecycle information affects remanufacturing decisions and cycle time: https://doi.org/10.1016/j.ifacol.2025.09.147

These works motivate evaluation discipline; they do not validate the synthetic losses, priors, or inspection likelihoods in this fixture.

## Claim ceiling

Synthetic finite-decision contract only. It does not establish real fault prevalence, sensor/inspection sensitivity, product-specific losses, remaining useful life, actual maintenance economics, or real-world EVSI. Symthaea may consume this kind of bounded decision-value result when selecting *proposals* after its existing action-selection prerequisites qualify. No inspection, repair, replacement or other physical action is authorized.
