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
