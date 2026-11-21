# Navigation transition validator

Standalone Rust validator for the synthetic Navigation Observability Transition benchmark.

The package uses two separate validation layers:

1. A standards-conforming JSON Schema Draft 2020-12 validator for each case.
2. A custom semantic cross-record validator for identifiers, time/frame references, event ordering, controller/evaluator separation and manifest agreement.

The package is deliberately isolated with its own Cargo workspace. It does not add a member or dependency to the Symtropy launcher workspace and does not implement a navigation estimator, runtime-assurance controller, time service, trust system or physical safety authority.

## Run

From the repository root:

```bash
cargo test --all-targets --manifest-path research/nav-transition-validator/Cargo.toml

cargo run --manifest-path research/nav-transition-validator/Cargo.toml -- \
  docs/research/navigation-transition-v0.1/nav_transition_case.schema.json \
  docs/research/navigation-transition-v0.1/fixtures/nav_transition_cases.v0.1.json \
  docs/research/navigation-transition-v0.1/manifest.v0.1.json
```

The CLI reports JSON_SCHEMA_DRAFT_2020_12_PASS and STRUCTURAL_SEMANTIC_PASS only when both layers succeed. The validator is pinned to jsonschema 0.61.2 with explicit Draft 2020-12 options and format validation enabled. The schema contains local references only; the package disables remote/file resolver defaults.

## Dataset admission gate

The same package includes a separate Draft 2020-12 schema and semantic admission validator for benchmark payloads. An admitted record requires an immutable release, file SHA-256 hashes, explicit dataset-payload rights evidence, permission for the requested use, a frozen train/test split with a digest, and explicit time/frame/calibration/ground-truth metadata. Candidate records may remain incomplete, but are counted as candidates and never promoted. The validator deliberately allows an explicitly declared unqualified time/frame state while surfacing it in the summary; unknown is not upgraded to qualified. The committed v0.1 registry currently contains four candidates and zero admitted datasets. For an admitted record, time-basis, spatial-reference and calibration states must be explicitly characterized; `unqualified` is allowed and reported as a limitation, but `unknown` blocks admission.

## Semantic checks

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

JSON Schema validates structural shape and declared constraints. The semantic layer validates selected relationships across records. Neither authenticates sensor identities, verifies calibrations, proves physical independence, verifies oracle correctness, solves frame transformations, correlates clocks, validates the mathematical covariance model, executes a navigation estimator, replays physics, or validates a physical model. A pass is not benchmark performance, field qualification or a navigation safety case.
