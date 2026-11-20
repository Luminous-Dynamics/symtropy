# Navigation transition semantic validator

Standalone Rust validator for the synthetic Navigation Observability Transition benchmark.

This package is deliberately isolated with its own Cargo workspace. It does not add a member or dependency to the Symtropy launcher workspace and does not implement a navigation estimator, runtime-assurance controller, time service, trust system or physical safety authority.

## Run

From the repository root:

```bash
cargo test --manifest-path research/nav-transition-validator/Cargo.toml

cargo run --manifest-path research/nav-transition-validator/Cargo.toml -- \
  docs/research/navigation-transition-v0.1/fixtures/nav_transition_cases.v0.1.json \
  docs/research/navigation-transition-v0.1/manifest.v0.1.json
```

The CLI reports `STRUCTURAL_SEMANTIC_PASS` only when its cross-record checks succeed. This output does not imply that a JSON Schema implementation has been run.

## What it checks

- Corpus/manifest identity and exact case-ID set equality.
- Unique case, source, phase, observation, transition, fault and evaluator-truth IDs.
- Required body/frame identity agreement across phases and truth states.
- Consistent repeated declarations for frame IDs and time-basis IDs.
- Phase environments are included in each case's domain path.
- Each observation's source, frame and time-basis references resolve.
- An observation modality, where supplied, matches its registered source.
- Controller observations have nondecreasing offsets.
- Fault source references resolve and fault intervals do not run backward.
- Transition phase references resolve and the transition's expected state matches its destination phase.
- Evaluator truth vectors are finite, have the required dimensions, and reference declared body/frame identities.
- Event labels refer to declared faults and do not adjudicate cause/actor attribution.
- Expected dispositions use supported states and have nondecreasing offsets.
- Evaluator-only truth/labels are not embedded in controller-visible observation records.
- Failure-domain inventories are present and internally unique; the validator never infers independence from sensor count.

## Deliberate limits

This is a semantic cross-reference and consistency validator, not a full JSON Schema Draft 2020-12 implementation. Run a standards-compliant JSON Schema validator against `nav_transition_case.schema.json` as a separate gate.

It does not authenticate sensor identities, verify calibrations, independently prove that two declared failure domains are physically independent, verify oracle correctness, solve frame transformations, correlate clocks, validate covariance matrices, execute a navigation estimator, replay physics, or validate a physical model. It checks declared IDs and consistency only.

The test suite is authored but has not yet been executed in the current work session. A compiled result, CI pass, benchmark result or navigation qualification must not be inferred from the source files alone.
