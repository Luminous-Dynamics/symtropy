# CivDyn Human Fidelity v0.1

A dependency-free reference kernel for measuring individual-response, population-distribution and cross-context trajectory fidelity separately.

## Why separate the layers?

A simulator can predict an individual's modal response but homogenize the population. It can match an aggregate distribution while assigning choices to the wrong individuals. It can match isolated contexts while breaking the order-dependent transitions that connect them.

This crate therefore reports three views, not one aggregate grade:

- **R1 / micro:** per-trial multiclass Brier score, log loss and top-choice accuracy against matched, opaque participant/context/step records.
- **R2 / population:** observed versus generated choice frequencies, total variation, Jensen–Shannon divergence, entropy gap and coverage of observed tail options.
- **Trajectory:** total-variation distance between adjacent-step transition distributions for matched individuals.

All metrics can be inspected globally and by declared context/cohort slice. Cohort labels are source-provided; the benchmark does not infer sensitive membership from text.

## Model comparison controls

The synthetic executable compares:
- `direct_prompt_mode_collapse`: one population-wide response tendency with a single generated choice;
- `structured_context_reference`: an intentionally matched reference that preserves individual response context and generated choices.

The second is a software oracle, not an LLM or human baseline. Its purpose is to show that improving R1 and R2 together is measurable and that mode collapse is detectable even when output probabilities are well-formed.

## Population coverage

A deterministic prefix curve reports R2 metrics at N={10,20,30,50,60}. This is a reproducible diagnostic, not a confidence interval or a saturation claim. Inferential use requires source-appropriate clustered or seeded repeated subsampling, held-out participant/site/time splits and predeclared uncertainty intervals.

## Run

```sh
cargo fmt --manifest-path research/civdyn-human-fidelity/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path research/civdyn-human-fidelity/Cargo.toml --all-targets -- -D warnings
cargo check --locked --manifest-path research/civdyn-human-fidelity/Cargo.toml --all-targets
cargo test --locked --manifest-path research/civdyn-human-fidelity/Cargo.toml --all-targets
cargo run --locked --manifest-path research/civdyn-human-fidelity/Cargo.toml --bin human-fidelity-sweep
```

## Claim ceiling and privacy

The synthetic fixtures contain only generated opaque IDs and artificial choices. They are not human observations and do not reproduce SimBench, ReliMap, OmniBehavior or any other upstream benchmark.

Real datasets require source rights, ethics/privacy review, anonymization, explicit participant-level split provenance and protection against re-identification. Never place direct identifiers or inferred sensitive attributes in these structures. Do not optimize demographic parity as a universal objective; report declared, adequately-supported slices and the decision rule being evaluated.


## Participant-cluster bootstrap extension

The follow-on `human-fidelity-bootstrap` executable runs a deterministic cluster bootstrap at N={10,20,30,50,60} with 200 replicates per sample size and policy.

The resampling unit is the unique opaque participant ID. Every observed and simulated context/step record for a sampled participant is carried together. Repeated draws receive distinct internal bootstrap IDs, so multiple observations from one person are never treated as independent people.

For each N it reports mean and percentile 95% intervals (p2.5/p97.5) for:
- R1: Brier, log loss and top-choice accuracy;
- R2: total variation, Jensen–Shannon divergence and entropy gap;
- trajectory: adjacent-context transition TVD;
- tail behaviour: observed tail-option coverage.

Undefined transition/tail metrics remain undefined and expose valid/undefined replicate counts. They are not imputed as zero.

The intervals quantify participant-composition uncertainty conditional on the supplied dataset and its observed sampling design. They are not predictive intervals, model-discrepancy intervals or external population/region/time generalization intervals. V0 does not stratify resampling by cohort; source-design-specific stratification and higher-level site/household clusters are V1 gates.

Run:

```sh
cargo run --locked --manifest-path research/civdyn-human-fidelity/Cargo.toml --bin human-fidelity-bootstrap
```
