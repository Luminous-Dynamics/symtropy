# BUILT-INTEROP-006 — Typed evidence report assembler

This Rust CLI composes independently produced gate objects into the full eight-gate report contract used by BUILT-INTEROP-004. It consumes structured JSON—not terminal logs, filenames or inferred outcome labels.

## Run

```sh
cargo run --locked --manifest-path tools/built-interop-006/Cargo.toml -- \
  --source-ifc ./source.ifc \
  --ids ./requirements.ids \
  --target-ifc ./target.ifc \
  --ifc-schema IFC4 \
  --supported-subset IFC4-IDS1.0-subset-v0.1 \
  --run-id exchange-2026-10-18T2300Z \
  --reported-at 2026-10-18T21:00:00Z \
  --source-commit-sha 0123456789abcdef0123456789abcdef01234567 \
  --evaluator-name built-interop-report-assembler \
  --gate-json ./ifc-schema-gate.json \
  --output ./assembled-report.json
```

Replace illustrative commit/run values with real provenance. A gate file may be the direct JSON gate object or a producer envelope containing a top-level `gate` object, such as BUILT-INTEROP-005's output. Other top-level report fields are ignored only at the envelope layer; the gate itself is deserialized into a closed typed structure that rejects unknown fields.

The CLI requires source IFC, IDS requirements, target IFC, declared evaluation profile, run ID, RFC-3339 timestamp, evaluator source commit SHA and evaluator name. It can accept repeated `--gate-json` arguments. It emits the same report on stdout and optionally to `--output`; exit status is 0 for QUALIFIED, 1 for BLOCKED, 2 for valid-but-UNQUALIFIED, and 64 for malformed arguments/evidence.

## Decision semantics

- `QUALIFIED`: all eight mandatory gates are PASS.
- `BLOCKED`: any required gate is FAIL.
- `UNQUALIFIED`: no gate failed, but one or more are NOT_RUN or UNRESOLVED.

If a producer did not supply a gate, the assembler creates that gate with status `NOT_RUN`, no tool and no evidence. Computing artifact byte digests does not itself set the artifact-integrity gate to PASS. It proves which bytes this process read, not whether an external expected digest or a chain of custody matches.

Gate IDs are closed and ordered: `artifact_integrity`, `step_syntax`, `ifc_schema_conformance`, `ids_xsd_validity`, `ids_semantic_validity`, `ids_model_conformance`, `exchange_semantic_preservation`, and `engineering_release_qualification`. Duplicate/unknown gates are rejected. PASS/FAIL require a valid tool/config hash and at least one hash-addressed evidence item. NOT_RUN cannot carry tool or evidence. Evidence IDs are globally unique in the assembled report.

## Typed producer integration

`--gate-json` accepts either a normalized `Gate` object / envelope or the exact `BUILT-INTEROP-002-ifc-ids-v0.1` producer report. That adapter report is deserialized with closed Rust types. Its `ifc_input_sha256` must match the exact **target IFC** bytes in this exchange report, and its `ids_input_sha256` must match the exact IDS requirements file; a digest mismatch rejects assembly.

The mapping is deliberately conservative:
- `SubsetConformant` becomes `PASS` only when the IFC4 token and explicit IDS 1.0 evidence are present, every applicable wall has exactly one `Preserved` observation, the count agrees, and there are no diagnostics.
- `Blocked` becomes `FAIL` only when typed outcomes identify a required `Name` value as `Changed` or `Missing` under the supported profile.
- Identity collisions, invalid identity, a schema/profile mismatch, unsupported IDS facets, `NotEvaluated`, and incomplete observation sets remain `UNRESOLVED`; diagnostic strings are not parsed to infer results.
- The assembler records the SHA-256 of the exact adapter report bytes as its evidence record and independently reconstructs the typed adapter semantic digest before accepting the report.

This binds the conformance claim to the deliverable IFC and IDS bytes rather than accidentally validating a different model. The source IFC remains separately hashed for exchange comparison.

## Paired-preservation gate verification

For `exchange_semantic_preservation`, the assembler requires an embedded, explicitly referenced `evidence_payload`; a bare gate cannot bypass validation. It checks source IFC SHA-256 and byte length, target IFC SHA-256 and byte length, IDS SHA-256, the exact supported profile, pinned tool version and policy/configuration hash, and the typed count/status/reason invariants.

The delta payload now includes an explicit `profile_confirmed` bit. The assembler accepts `FAIL` only when the supported profile is confirmed and at least one definite changed/lost/dropped count is nonzero. A report with an unconfirmed profile remains `UNRESOLVED`, even when it records an apparent difference. `PASS` requires confirmed profile, complete comparison coverage, at least one compared object, no unresolved cases, and no definite losses.

## Cross-tool integration

`test-pipeline.sh` runs the pinned BUILT-INTEROP-005 IFC schema gate against the checked-in IFC4 wall fixture, verifies the typed producer envelope and evidence hash, then passes that JSON directly to this assembler. It asserts the schema result is either PASS or UNRESOLVED (never hides FAIL), all seven unprovided gates remain NOT_RUN with no tool/evidence, and the assembled overall result is UNQUALIFIED. This integration is deliberately not a qualified exchange: no semantic-preservation or engineering evidence has been generated.

## Schema and testing

- `assembled-report-v1.schema.json` pins the BUILT-INTEROP-004-shaped full report.
- Rust structs have `deny_unknown_fields` and enforce cross-field rules not expressible using local JSON types alone.
- Tests include missing-gate fail-closed behavior, all-pass / fail / unresolved decision logic, duplicate gates, evidence/tool requirements, unknown fields, malformed hashes, duplicate evidence IDs, exact-byte artifact hashing, normalized ordering, timestamp/run ID/source SHA validation and report-shape contracts.

A digest's format is checked here; evidence content, external attestations, artifact custody and validator soundness are **not authenticated** by this assembler. Never claim structural safety, building-code compliance, physical qualification or impact improvements from this report alone.

## Benchmarks

Initial deterministic contract targets: 100% rejection of seeded malformed gate/report inputs; zero absent gates promoted to PASS; exact byte hashes/lengths for all three artifacts; order-invariant normalization of supplied gates; and 100% preservation of all mandatory gate identities. Real exchange outcomes should later add independent golden oracles, false acceptance/rejection, critical-property loss, quantity/unit/tolerance/geometry/topology deltas, rework hours, manufacturing blockers, cost, material recovery and embodied-carbon measurements on rights-cleared source/target pairs.

The synthetic unit tests do not claim that any real IFC model or building has passed.
