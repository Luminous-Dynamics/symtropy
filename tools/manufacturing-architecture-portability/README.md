# Manufacturing architecture portability — M0 profile contract

This folder starts [MFG-ARCH-PORTABILITY-001 / #6579](https://github.com/Luminous-Dynamics/symtropy/issues/6579).

## Scope

M0 defines a versioned, source-reviewable manifest for declaring a *composition* of industrial information layers. It is deliberately not a protocol implementation or a production runtime.

The architecture is layered, not a contest between mutually exclusive protocols:

- event transport / Unified Namespace (for example MQTT);
- machine information models (for example OPC UA Machinery);
- contextual manufacturing APIs (for example CESMII i3X);
- equipment observation documents (MTConnect);
- semantic/context models and relationships;
- digital-thread links between product, process, execution, inspection and twin evidence.

A profile may combine several layers. Each layer pins its own specification revision, implementation identity, read/write scope, semantics preserved, unsupported semantics and evidence/admission state.

## Files

- `profile.schema.json`: JSON Schema Draft 2020-12 for the M0 architecture-profile contract.
- `profiles/uns-mqtt.synthetic.json`: one synthetic-only event-transport profile fixture. It does **not** implement an MQTT broker or claim MQTT conformance.

## Hard boundaries

- The benchmark's initial client contract is read-only: write, command and product-release capabilities are structurally prohibited by the schema.
- Source documents, source-owned records and their digests remain authoritative. A normalized field is a derived view, not a replacement for its source.
- Unknown, stale, contradictory and unsupported semantics must be reported, not silently coerced.
- Standard publication, implementation existence, schema admission, protocol conformance and independent qualification are different states.
- A protocol-valid event does not prove source truth, calibration, process capability, part conformity, safety or authorization.
- The manifest is an integration-test declaration, not a cybersecurity assessment, safety case or standards certificate.

## Composition rules

1. Treat each layer as an independently versioned component.
2. Pin exact specification/format and implementation revisions; do not use the word "latest" as a version pin.
3. Record what the implementation actually supports and what it does not.
4. Preserve the native identity, units, frame, clock, sequence, calibration and source locator at every adapter boundary.
5. Preserve source-owned evidence references and explicitly identify the system that owns each authority.
6. Replaying identical admitted inputs, profile revision and evaluator version must reproduce the same normalized semantic result wherever deterministic replay is claimed.
7. Test protocol conformance, semantic preservation and task-level decision value as separate experiments.

## Next increment

The next step is to add reference-only profiles for i3X 1.0 and OPC UA for Machinery 1.04.2, then connect each profile to a shared frozen workload and independent evaluator. The i3X upstream conformance suite must be reused rather than reimplemented. MTConnect/QIF source admission remains owned by #3025/#3026/#3033; composition VVUQ remains owned by #3121; context and decision-value ablations remain owned by #5622.

No live broker, endpoint, CNC machine, robot, PLC, write method or physical asset is contacted by this M0 fixture.

## Reference profiles added in M0.1

- `profiles/i3x-1.0.reference.json`: source-reviewed i3X 1.0 API profile. The upstream suite is a separate external referee (60 conformance tests). When a concrete read-only endpoint exists, run the upstream CLI with `--no-writes`; do not expose or declare update capabilities. The upstream suite requires Node.js >=18.17 for its own test runner; this does not add a Node runtime to Foundry.
- `profiles/opcua-machinery-1.04.2.reference.json`: separately pins OPC UA for Machinery Part 1 v1.04.2, published 2026-11-01.
- `profiles/opcua-robotics-1.02.reference.json`: independently pins OPC UA for Robotics Part 1 v1.02. Do not conflate this with Machinery or infer robot safety conformance.

Reference profiles are source-reviewed specifications only. Their runtime semantics have not been implemented or tested. The `required_fields_to_preserve` list is a future adapter/evaluator requirement, not a claim that fields are currently preserved. `preserved_fields` remains empty and every profile stays `not_run`.

## Offline Rust profile validator

A standalone Rust validator lives in `validator/`. It pins the Draft 2020-12 implementation to `jsonschema = 0.62.0` and disables the crate's default remote/file reference-resolution features. The Foundry profile schema is self-contained, so profile validation must not retrieve schemas or other data from the network.

Run from the validator directory:

```sh
cargo test
cargo run -- ../profile.schema.json \
  ../profiles/uns-mqtt.synthetic.json \
  ../profiles/i3x-1.0.reference.json \
  ../profiles/opcua-machinery-1.04.2.reference.json \
  ../profiles/opcua-robotics-1.02.reference.json
cargo clippy --all-targets -- -D warnings
```

The validator performs both formal JSON Schema validation (including format assertions) and independent semantic checks for duplicate standard/layer identities, unresolved standard references, layer-kind/reference-role mismatch, invalid access paths, implementation/evidence-state promotion, reference-only source misuse, synthetic-to-conformance promotion and semantic-field contradictions.

The dedicated workflow bootstraps this isolated tool's `Cargo.lock` once and uploads the generated lockfile as an artifact. Commit that exact lockfile before changing this workflow to `--locked`. Until that artifact is reviewed and the lock is committed, dependency resolution is not yet frozen. The validator has no endpoint access and cannot establish runtime conformance.

## Portable observation projection and semantic referee

`portable-observation.schema.json` defines the exact candidate projection shape for a source-preserving observation. It requires the adapter-profile identity, original source-document digest, source schema identifier/version, machine and data-item identities, source and reference timestamps, clock mapping and uncertainty, native kind/unit/value, source locator and opaque-extension manifests.

The validator's `semantic_evaluator` compares candidate projections against evaluator-owned truth one required field at a time. It does not normalize or silently coerce values. The current test-only truth is one finite synthetic MTConnect sample record; it is deliberately compiled only in the test module. A passing comparison establishes exact agreement for that fixture only.

The negative controls cover wrong machine identity, missing sequence, altered unit, changed clock mapping, dropped opaque extension, and additional candidate fields. Additional fields are surfaced in the evaluator report rather than silently discarded. The candidate projection schema separately rejects undeclared root fields; unknown source-standard extensions belong in the explicit opaque-extension manifest.

The observation timestamp numbers in this fixture are synthetic and are not admitted as real plant-clock or physical measurement values. Time-unit and clock-domain claims still require source-specific clock and unit qualification. The exact comparator intentionally does not invent tolerances.

## Explicit cycle-to-inspection association referee

`inspection-join.schema.json` and the test-only `inspection_evaluator` extend the projection check to an explicit manufacturing-cycle ↔ QIF-result relationship. The strict record binds:
- exact cycle, QIF result, part, lot, operation and characteristic identities;
- cycle source, MTConnect source and QIF source document digests repeated inside the association evidence;
- association receipt identity, authority label, digest and validity interval;
- QIF result status, source measurement value/unit, derived value/unit and transform revision;
- source/reference timestamps, both clock-map digests, reference-clock identity and measurement window.

The evaluator independently compares the association's bound source digests, cycle and QIF result IDs against the candidate's cycle and QIF records. It checks measurement and decision times against both cycle and association effectivity.

For the finite fixture, combined timestamp uncertainty is recomputed as the **sum of the absolute cycle and QIF clock uncertainty bounds**. This is deliberately conservative when dependence/correlation has not been qualified; it does not assume statistical independence and does not use root-sum-square. The supplied combined value must exactly equal this reconstruction and remain below the declared maximum.

Important claim ceiling: all values and hashes are synthetic. The evaluator verifies structure and internal consistency; it does not cryptographically authenticate the association authority, prove that its clock mappings are correct, qualify the CMM, recompute metrology uncertainty, interpret the QIF spec, assess conformity, or authorize product release. The production receipt/authority owners must independently admit those claims.
