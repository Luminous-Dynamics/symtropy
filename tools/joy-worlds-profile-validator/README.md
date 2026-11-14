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

It rejects unpaired candidate sets, missing constraints/outcomes/bounds, unit mismatches, invalid uncertainty bounds and any non-synthetic input. Known outcomes must explicitly provide a finite value, lower bound and upper bound; an exact point estimate must state equal bounds rather than silently implying certainty. A failed hard constraint in any scenario excludes the candidate from the Pareto set; unknown constraints or outcomes make it `not_evaluable`. The robust envelope uses lower bounds for maximization and upper bounds for minimization, and never creates a single weighted score. Each objective declares its own non-negative dominance tolerance in that objective's units.

Benchmark targets and proposed acceptance criteria: [benchmark targets v0.1](../../docs/research/joy-worlds/benchmark-targets-v0.1.md).

The receipt also includes evidence-gap triage per objective: it names unknown observation counts, candidate IDs and evidence references alongside the known-interval frontier-leverage signal. This can surface where evidence is missing without converting missing data to a numeric value. The receipt includes **endpoint sensitivity**: conservative versus optimistic-envelope Pareto fronts, stable/uncertain membership, and one-objective-at-a-time frontier leverage. This helps prioritize follow-up measurement but is not formal expected value of information because it uses no probability distributions, stakeholder utility function or measurement costs. Optimistic values across objectives may come from different scenarios and need not be jointly attainable.

## Reproducible synthetic performance benchmark

Generate a 10,000 candidate-scenario-pair workload with eight dimensions and collect seven timing samples. The benchmark emits the exact input SHA-256, input byte size, all timing samples, median and nearest-rank p95:

```sh
cargo run --release --manifest-path tools/joy-worlds-profile-validator/Cargo.toml --bin portfolio-benchmark -- \
  --candidates 500 --scenarios 20 --objectives 8 --repetitions 7
```

Use a fixed, documented host and record its CPU, OS and Rust version. The target is p95 below 1 second, not a claimed result. Fixture generation is excluded from the timed section; JSON parsing and comparison are included.

A small CI smoke invocation is included to test the benchmark binary without making the 10,000-pair performance threshold a flaky shared-runner gate. Unit tests exhaust 729 two-objective values for three candidates against a separately written Pareto oracle and exhaust all 81 combinations of four three-state hard constraints to test fail-closed precedence.

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
