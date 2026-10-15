# Joy Worlds Profile Validator

A standalone Rust tool for structural validation of Joy Worlds manifests and deterministic comparison of explicitly synthetic destination scenarios.

## Run

From the repository root:

```sh
cargo test --manifest-path tools/joy-worlds-profile-validator/Cargo.toml
cargo run --manifest-path tools/joy-worlds-profile-validator/Cargo.toml -- \
  --portfolio docs/research/joy-worlds/experience-portfolio-v0.1.json \
  --regional docs/research/joy-worlds/regional-adaptation-profiles-v0.1.json \
  --decision-contract docs/research/joy-worlds/destination-decision-contract-v0.1.json
```

## Synthetic portfolio comparator

Run the deterministic comparator (the input fixture must say `status: synthetic_fixture`):

```sh
cargo run --manifest-path tools/joy-worlds-profile-validator/Cargo.toml --bin portfolio-compare -- \
  --input docs/research/joy-worlds/fixtures/synthetic-destination-scenarios-v0.1.json \
  --output /tmp/destination-decision-receipt.json
```

It rejects unpaired candidate sets, missing constraints/outcomes, unit mismatches, invalid uncertainty bounds and any non-synthetic input. A failed hard constraint in any scenario excludes the candidate from the Pareto set; unknown constraints or outcomes make it `not_evaluable`. The robust envelope uses lower bounds for maximization and upper bounds for minimization, and never creates a single weighted score.

Benchmark targets and proposed acceptance criteria: [benchmark targets v0.1](../../../docs/research/joy-worlds/benchmark-targets-v0.1.md).

## Current checks

- Required fields and non-empty evidence/unknown/constraint lists.
- Duplicate profile/candidate/rule IDs.
- Cross-reference integrity between comparison protocols and declared profiles.
- Water-related portfolio modules must declare a safety or qualified review dependency.
- Named coastal profiles must identify coastal drivers and constraints.
- Named ski profiles must explicitly address snow reliability, avalanche/mountain hazards and uncertainty around snowmaking.
- Named cruise profiles must cover ship and port scope and not assume shore-power capability implies port supply.
- The destination decision contract must retain hard-constraint precedence, explicit infeasible/not-evaluable states, no-build/reversible alternatives, separated outcome dimensions, negative controls and human authority/privacy boundaries.
- The synthetic comparator requires paired candidate sets, declared units/directions, complete hard-constraint checks and uncertainty bounds.
- Source URLs must be HTTPS.
- Malformed JSON and adverse/missing fields fail closed.

## Claim ceiling

A passing result means only that these manifests satisfy the implemented structural and cross-reference checks. It does not establish demand, market fit, sustainability, ecological carrying capacity, accessibility compliance, engineering correctness, safety, legal compliance or real-world forecast skill. The tool intentionally does not populate missing measurements or infer site parameters from profile labels.
