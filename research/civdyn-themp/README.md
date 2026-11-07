# CivDyn THEMP Passport v0.1

A dependency-free Rust auditor for explicit relationships between model theory, empirical evidence, operationalization, model purpose and competing explanations.

## Research basis

Wijermans, Vriens & Andrighetto (2026), *Harmonising Theory and Empirics: The THEMP Approach for Developing, Testing and Exploring Social Simulation Models*, JASSS 29(4), 6. DOI: 10.18564/jasss.6211.

THEMP highlights three model-development stages:
- design: characterize constructs and mechanisms separately from parameter calibration;
- testing: process credibility, internal validation, verification and external validation;
- exploration: sensitivity, counterfactuals and alternative explanations.

The passport makes that lifecycle explicit but does not attempt to automate scientific judgement.

## Contract

Every model declares its purpose and scope. Constructs either map to one or more operational indicators or are explicitly marked theory-only with a reason. Indicators declare units, aggregation rules, missing-data policy and zero semantics independently.

Explanatory mechanisms declare their theoretical rationale, lag, boundary conditions and falsifier. Each mechanism also links to a competing explanation or documents an open research gap. Alternatives must identify at least one discriminating observable pattern.

Predictions bind to a predeclared pattern, outcome indicator and held-out partition. External validation requires empirical observations from that partition. Calibration and external validation may not use the same source/partition pair.

Evidence-use records distinguish characterization, calibration, process credibility, internal validation, verification, external validation, sensitivity exploration and counterfactual exploration. Source kinds separately distinguish theoretical syntheses, empirical datasets/studies, simulation outputs and scenario specifications. Claim classes remain four distinct categories: historical observation, contemporary observation, model inference and counterfactual projection. A theory review is not mislabeled as an observation. Simulation output cannot serve as external empirical validation. Reuse of the same source family across separate calibration and holdout partitions is disclosed as a warning; using the same source partition for both remains an error.

No universal theory/empirics score is produced. The report contains a set of conformance findings and checked-object counts.

## Synthetic controls

The executable evaluates five synthetic passports:
- reference: a complete theory/empirics passport;
- calibration/holdout leakage;
- an unexplained, unoperationalized construct;
- an explanatory mechanism without a falsifier;
- a model inference incorrectly presented as external empirical validation.

All data are artificial. They are not measurements from any society, region or historical case.

## Relationship to existing infrastructure

- CivBench MAD owns evaluation-distribution and execution-stack manifests.
- Humanity Atlas remains the exact run/input/output receipt boundary.
- CIVDYN causal-path owns structural intervention-to-outcome paths and paired-effect qualification.
- Mycelix owns provenance, authority, consent and contestability.
- This crate audits the theory, operationalization and evidence-use logic that precedes and interprets those artifacts.

## Run

```sh
cargo fmt --manifest-path research/civdyn-themp/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path research/civdyn-themp/Cargo.toml --all-targets -- -D warnings
cargo check --locked --manifest-path research/civdyn-themp/Cargo.toml --all-targets
cargo test --locked --manifest-path research/civdyn-themp/Cargo.toml --all-targets
cargo run --locked --manifest-path research/civdyn-themp/Cargo.toml --bin themp-passport-sweep
```

## Claim ceiling

A conformant passport means the model has explicitly documented theory/evidence relations and passes these structural checks. It does not establish that the theory is true, that causal effects are identified, that the data are representative, or that empirical validation is adequate. Synthetic CI only tests the auditor.
