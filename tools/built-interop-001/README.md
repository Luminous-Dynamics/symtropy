# BUILT-INTEROP-001 — semantic-loss benchmark for design-to-fabrication exchange

Tracking issue: https://github.com/Luminous-Dynamics/symtropy/issues/2005

## v0.1 scope

This crate compares a **synthetic source-requirement manifest** against imported/target observations. It is not an IFC parser or exporter, and its first fixtures do not claim a real CAD/BIM/IDS round trip has been executed. The purpose is to freeze the loss semantics and independent oracles before binding them to exact exchange-tool artifacts.

The key shift is from "the file parses or renders" to "every source requirement is either preserved, transformed by an explicit checked rule, or reported as loss." A plausible viewer preview is not an interchange correctness result.

**Conformant** means only that the synthetic observations satisfy this comparator's narrow requirements. It does not authorize manufacturing or installation, establish structural adequacy or code compliance, verify a product, or prove physical safety.

## Run

```sh
cargo fmt --manifest-path tools/built-interop-001/Cargo.toml -- --check
cargo test --manifest-path tools/built-interop-001/Cargo.toml
cargo run --manifest-path tools/built-interop-001/Cargo.toml -- \
  tools/built-interop-001/fixtures/panel-exchange-v0.1.json \
  --output /tmp/built-interop-report.json
```

The CLI emits raw-input, canonical-input and semantic-result SHA-256 digests. Canonicalization sorts scenarios, source requirements, target observations and transform rules by stable fields. Row-order permutations should not change the canonical/result digests; the raw-input hash still records byte-level differences.

## Machine-readable requirement results

Each scenario report now includes `requirement_results`. Every source requirement receives its own structured record with:
- source requirement ID, source object ID, property key and criticality;
- expected value/unit and observed target object/value/unit (nullable when absent);
- declared loss disposition and transform ID;
- requirement outcome (`Preserved`, `Transformed`, `Blocked`, or `Unresolved`);
- stable diagnostic codes such as `NO_TARGET_OBSERVATION`, `OBJECT_IDENTITY_MISMATCH`, `PRESERVED_VALUE_OR_UNIT_MISMATCH`, `MISSING_OR_DUPLICATE_TRANSFORM_RULE`, and `DECLARED_TRANSFORM_RESULT_MISMATCH`.

Consumers should use these fields rather than parsing human-readable issue strings. Diagnostic codes are a closed, serialized Rust enum in `SCREAMING_SNAKE_CASE` and are enumerated by the output schema; unknown spellings are contract changes, not informal text. Global malformed/orphan-record diagnostics remain in the scenario-level `issues` array. The Rust test suite uses the pinned `jsonschema` 0.58.6 crate to validate the synthetic manifest and serialized report against [semantic-loss-v0.1.schema.json](semantic-loss-v0.1.schema.json) and [report-v0.1.schema.json](report-v0.1.schema.json), and includes a negative control proving that an unlisted diagnostic code is rejected.

## Loss taxonomy

Every requirement is compared against target observations using one of these outcomes:

- **Preserved**: exact canonical value and unit match.
- **TransformedWithDeclaredRule**: an explicitly named transform must match the source/target units and reproduce the target numeric value within its preregistered absolute tolerance.
- **Unsupported**: target representation cannot support the required information.
- **Missing**: required property is absent in the target.
- **Changed**: target meaning/value differs.
- **NotEvaluated**: no valid comparison is supplied.

Hard requirements that are missing, changed, unsupported, not evaluated, unreferenced, duplicated or bound to the wrong object block the affected scenario. Advisory-only unknown/lost properties yield **Unresolved**; they are not silently promoted to preserved. A hard property cannot be made acceptable merely by labeling its observation differently.

Exact value comparison is intentionally strict: non-numeric values are compared as declared canonical strings. v0.1 does not silently normalize textual values. Unit conversion only occurs through a named transform with source/target units, scale, offset and an explicit tolerance. That rule is still a declared fixture rule—not proof that the unit conversion is physically appropriate for a real artifact. The approved v0.1 transform tuple (mm→m, scale 0.001, offset 0, absolute tolerance 1e-9) is pinned separately in Rust tests; zero/nonpositive scales are rejected so a conversion cannot erase the source quantity. This one tuple is a synthetic benchmark oracle, not a general engineering unit registry.

## Initial scenarios

| Scenario | Expected result | Adversarial control |
|---|---|---|
| 01-exact-property-preservation | Conformant | IDs, thickness and BOM reference are preserved exactly |
| 02-hard-property-explicitly-missing | Blocked | mandatory fire-class field has no value |
| 03-unit-changed_without_transform | Blocked | a unit/value change is mislabeled as preserved |
| 04-source-object-identity-swapped | Blocked | the observed object ID does not match the source object |
| 05-declared-unit-transform-correct | Conformant | mm to m conversion follows the fixture's pinned rule |
| 06-declared-transform-rule-missing | Blocked | transform ID cannot be resolved |
| 07-unknown-promoted-to-numeric-default | Blocked | unknown qualification state is changed into a positive value |
| 08-hard-property-unsupported | Blocked | mandatory qualification reference is unsupported |
| 09-advisory-property-not-evaluated | Unresolved | optional finish property is not assessed |
| 10-duplicate-imported-observation | Blocked | same source requirement maps to duplicate targets |
| 11-no-observation-record | Blocked | required source property has no target record |
| 12-unreferenced-target-observation | Blocked | target-only field cannot satisfy a source requirement |

Fixture dispositions are declared in JSON and independently pinned in Rust test code so altering both the data and its expected status does not make the suite agree with itself.

## Standards and integration boundaries

- The baseline is the official [IFC 4.3.2.0 documentation](https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/index.html), published as [ISO 16739-1:2024](https://technical.buildingsmart.org/standards/ifc/ifc-schema-specifications/).
- Use [buildingSMART IDS 1.0](https://www.buildingsmart.org/standards/bsi-standards/information-delivery-specification-ids/) where applicable to machine-checkable information requirements. IDS checks exchange information requirements; it is not a structural solver or construction approval.
- This work must reuse the housing-evidence and exact-geometry boundaries in Symtropy [PR #1596](https://github.com/Luminous-Dynamics/symtropy/pull/1596) and [SPATIAL-DESIGN-001 #1766](https://github.com/Luminous-Dynamics/symtropy/issues/1766).
- Mycelix owns canonical BOM, work-order, process, lifecycle, custody and material-flow records; the comparator checks reference preservation rather than becoming a second manufacturing/lifecycle database.
- Symthaea [MFG-VALUE-001 #7608](https://github.com/Luminous-Dynamics/symthaea/issues/7608) owns actual matched-baseline production-value claims.
- Sol-Atlas context references remain source-owned observations. This harness cannot infer site water, power, access or capacity from a map or missing property.

## Next stage

1. Pin one actual synthetic IFC file, one IDS file, tool/export/import versions, configuration and exact artifact digests.
2. Write an independent extractor for the selected fields or consume the exact qualified housing verifier's outputs; do not hand-label target outcomes as if they were independently parsed facts.
3. Add explicit loss reports by object/property ID and the impact scope for each unsupported/missing/changed/not-evaluated field.
4. Add round-trip tests across two exact tool/profile pairs only after the first path passes its source/target mutation corpus.
5. Extend property profiles to coordinate frames, placements and geometry with per-profile tolerances; do not invent a universal geometric tolerance.
6. Keep all real engineering, life-safety, statutory and physical qualification decisions with competent owners.

## Known limitations

- v0.1 does not parse IFC, validate IDS XML, access proprietary CAD, compare actual geometry, or fetch material passports.
- External authorities and reference IDs require owner-side trusted resolution; string identity alone does not authenticate a source.
- A hash establishes byte integrity relative to a digest, not source truth or engineering applicability.
- The numeric transform implementation is a bounded affine scalar transform; it is not a general unit ontology and must not be used for vector frames, temperatures, nonlinear properties or geometry without a separate reviewed rule profile.
- The current tests and target data are synthetic, not evidence that an actual IFC/IDS toolchain preserves a panel assembly.
- Exact-head rustfmt, tests, Clippy and repository CI must pass before software qualification. Queued, skipped, cancelled, stale or absent results are not passes.
