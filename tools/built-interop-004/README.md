# BUILT-INTEROP-004 — Fail-closed validation ladder

This slice defines a **report contract**, not a new IFC/IDS validator. It makes the strongest claim a workflow is allowed to publish explicit and auditable.

## Paradigm shift: exchange as a chain of proofs

A file opening successfully is not proof that its model is schema-conformant; schema conformance is not IDS validity; IDS validity is not model conformance; and model conformance is not proof that export/import preserved required semantics or that a physical work is safe to release.

Every report accounts for all eight mandatory gates, in this order:

| Gate ID | Claim boundary |
| --- | --- |
| `artifact_integrity` | Input artifact identity and byte-integrity evidence exists |
| `step_syntax` | IFC STEP syntax / complete parse |
| `ifc_schema_conformance` | Declared IFC schema and model-rule conformance, with unsupported rules disclosed |
| `ids_xsd_validity` | IDS XML and the pinned IDS XSD |
| `ids_semantic_validity` | IDS semantic/audit rules beyond XML/XSD |
| `ids_model_conformance` | The IFC model meets the declared IDS requirements |
| `exchange_semantic_preservation` | Required meaning survives a source→target export/import, not just that files parse |
| `engineering_release_qualification` | Separate discipline-specific engineering / release evidence for the claimed use |

The report contract is version `1.0.0`; the IDS standard itself is version `1.0`. The gate states are `PASS`, `FAIL`, `UNRESOLVED` and `NOT_RUN`. Every gate is mandatory in this profile. Unknown, unsupported or unrun is never success. Overall decision is derived, not trusted from the producer:

- **QUALIFIED** — every gate is `PASS`.
- **BLOCKED** — at least one gate is `FAIL`.
- **UNQUALIFIED** — there is no failure, but one or more gates are `UNRESOLVED` or `NOT_RUN`.

A `PASS` or `FAIL` must name the tool and version, hash the configuration, and reference at least one hash-addressed evidence record. `NOT_RUN` cannot carry tool/evidence that implies an execution occurred. Each claim records its scope and limitations.

## Files

- `report-v1.schema.json`: versioned Draft 2020-12 JSON Schema with closed objects, enums, source/target artifact roles and conditional execution-evidence requirements.
- `validate-report.sh`: independent jq-based structural and cross-field checker. No Python or JavaScript runtime is used.
- `test-contract.sh`: deterministic synthetic positive reports and adversarial mutations; invoked by the focused GitHub Actions workflow.

## Run

```sh
bash tools/built-interop-004/test-contract.sh
bash tools/built-interop-004/validate-report.sh /path/to/report.json
```

The checker validates the **report contract** only. It does not open artifact locators, recompute reported input hashes, authenticate external attestations, prove an evidence payload is truthful, or certify a validator's implementation. Hash strings are required to have SHA-256 shape; matching file bytes must be verified by a separate integrity tool.

The synthetic fixtures deliberately make no claim about any actual building, IDS fixture, IFC model, CAD exporter, code compliance or engineering release. No official upstream IFC/IDS fixture bytes are copied here.

## Benchmarks / acceptance criteria

Current executable controls target 100% rejection of the seeded malformed-report cases: missing gate, duplicate gate identity, `QUALIFIED` with `NOT_RUN`, `PASS` without evidence, malformed artifact hash, overall-decision mismatch, `NOT_RUN` with execution evidence, unknown top-level field, and malformed report timestamp. CI also accepts one consistent `QUALIFIED`, one `UNQUALIFIED` and one `BLOCKED` synthetic report.

Future evidence-based targets (not claimed achieved by this contract slice):

- Zero false qualification in mutation-tested report contracts.
- 100% mandatory-gate coverage and unique gate identities.
- 100% of executed PASS/FAIL gates have evidence digests plus tool/configuration provenance.
- Zero unsupported checks silently promoted to PASS.
- Deterministic reports under object / row ordering changes.
- On rights-cleared source→target exchanges: measure critical-property false acceptance/rejection, semantic-loss rate, geometry/topology deltas, correction hours, manufacturing blockers, material recovery, cost and embodied-carbon delta. Do not publish impact improvements without measured baseline data.

## Research anchors

- [buildingSMART IDS v1.0 final standard](https://www.buildingsmart.org/information-delivery-specification-ids-v1-0-is-approved-as-a-final-standard/)
- [IDS implementer developer guide](https://github.com/buildingSMART/IDS/blob/development/Documentation/ImplementersDocumentation/developer-guide.md) — explains why XSD validity is not the complete semantic audit.
- [Rust `ifc-validate` documentation](https://docs.rs/ifc-validate/latest/ifc_validate/) — intentionally separates parsing from schema/model validation and reports unsupported rules instead of silently skipping them.

## Next work

1. Bridge the bounded adapter (#2013) and synthetic loss comparator (#2007) to this common report contract without string scraping.
2. Add a separately versioned IFC schema-validation gate and explicit unsupported-rule counts.
3. Execute the rights-aware corpus inventory (#2036) only after independent outcome oracles exist; filename labels alone are not ground truth.
4. Repeat actual CAD/IFC export-import tests across versions and toolchains, then attach downstream engineering attestations where a release claim is necessary.
