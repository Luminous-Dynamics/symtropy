# BUILT-INTEROP-007 — Paired source-to-target semantic delta

This Rust tool compares one typed BUILT-INTEROP-002 report made from the source IFC with a second report made from the target IFC, under the same IDS requirements bytes. It emits an `exchange_semantic_preservation` gate with a hash-addressed evidence payload.

## Run

```sh
cargo run --locked --manifest-path tools/built-interop-007/Cargo.toml -- \
  --source-ifc ./source.ifc \
  --target-ifc ./target.ifc \
  --ids ./requirements.ids \
  --source-report ./source-adapter-report.json \
  --target-report ./target-adapter-report.json \
  --output ./semantic-delta-gate.json
```

Use reports emitted by the pinned BUILT-INTEROP-002 adapter. The CLI verifies each report's exact IFC digest, shared IDS digest, and its inner `semantic_result_sha256` by reconstructing the adapter's typed semantic-digest struct. It does not infer results from report filenames or parse diagnostics text.

## Decision policy

- **PASS:** both reports explicitly declare IFC4 and IDS 1.0; coverage is complete for the supported subset; every compared GlobalId is unique and valid; at least one stable object was compared; and no source property/object loss was found.
- **FAIL:** under the supported, confirmed profile, a source object has no counterpart in a complete target set, a populated source `IFCWALL.Name` is missing at target, or the target `Name` value changed.
- **UNRESOLVED:** schema/IDS profile is unconfirmed, report coverage is incomplete, stable identities are missing/invalid/duplicated, a supported observation is not evaluated, or no meaningful comparison can be made.

Target-only object additions and values added where the source had no value are recorded as additions, but do not alone fail preservation. Equal absence in source and target is counted as preserved **state**, not IDS conformance. The separate `ids_model_conformance` gate is responsible for whether requirements are satisfied.

## Evidence / output contract

The output has the standard gate shape accepted by BUILT-INTEROP-006:
- `gate.gate_id = exchange_semantic_preservation`
- gate status is `PASS`, `FAIL` or `UNRESOLVED`
- `gate.evidence[0].sha256` hashes the exact serialized `evidence_payload`
- `gate.evidence[0].locator = report-json-pointer:/evidence_payload`
- payload records source/target/IDS hashes and sizes, exact source and target adapter-report hashes, sorted findings, comparison counts and reason.

The assembler also independently verifies the payload digest, source/target/IDS byte hashes and lengths, exact supported profile, this tool's pinned config hash, and status/count/reason consistency. The explicit `profile_confirmed` bit prevents an apparent delta under an unconfirmed schema/IDS profile from being promoted to FAIL or PASS.

The payload distinguishes `profile_confirmed` from `comparison_coverage_complete`: a known IFC4/IDS 1.0 profile can still have incomplete identity/observation coverage. A definite change/loss/dropped object becomes FAIL only under the confirmed profile; otherwise the report remains UNRESOLVED. The assembler validates these fields against typed status semantics before accepting the gate.

## Scope boundaries

This implementation compares **only** the supported literal `IFCWALL.Name` observation. It is not a general IFC graph matcher and does not compare property sets, quantities, units, tolerances, material layers, relationships, geometry, topology, or building-code semantics. It does not certify a building, design, manufacturing release, structural safety, or environmental benefit.

No official licensed IFC/IDS fixtures are copied. Unit tests use synthetic bytes and typed observations only.

## Benchmarks

Contract tests cover identical/preserved values, changed values, lost values, dropped and added objects, equal absence, invalid/duplicate identities, wrong artifact or IDS digest, tampered semantic digest, unconfirmed IFC schema, and invariance of delta findings under STEP row/observation ordering changes.

Initial targets:
- 100% detection of seeded changed/lost/dropped supported observations.
- zero PASS when schema/IDS declaration or identity coverage is unresolved.
- zero comparison by STEP entity number when stable GlobalId is unavailable.
- zero acceptance of source/target reports bound to other IFC bytes or a different IDS file.
- stable findings/counts under STEP row reordering, while retaining exact report byte digests as provenance.
- on rights-cleared real exchanges, measure critical-property loss, false acceptance/rejection, quantity/unit/tolerance/geometry/topology delta, correction hours, manufacturing blockers, material recovery, cost and embodied-carbon difference. Do not publish improvements without independent expected outcomes and real baseline measurements.
