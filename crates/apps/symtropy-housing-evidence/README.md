# Symtropy Housing Evidence Verifier

**Status: experimental verification tooling. Not a building-code engine or engineering certification.**

This binary is a fail-closed verifier for the draft housing evidence manifest. It:

- validates manifests against the checked-in JSON Schema Draft 2020-12 schema embedded at compile time;
- rejects external schema references, so schema validation cannot fetch remote content;
- checks artifact/claim identifiers and SHA-256 cross-references, including the executable/container and configuration digests of tools used by evaluated claims;
- requires an independently pinned schema/case/jurisdiction/model/rule-pack/toolchain contract and mandatory-claim definitions before accepting an aggregate `PASS`;
- compares each mandatory claim's type, statement, element scope, requirement reference, validity domain, and acceptance criteria against that contract;
- requires evaluated `PASS` and `FAIL` claims to carry non-empty input/output artifact digests, tool identity, and measured-result content;
- requires mandatory code-compliance claims to cite a legislation/standard/regulation/technical-guidance source with an edition or effective date, clause/location, public/licensed rights status, and a source digest resolving to locally verifiable artifact bytes;
- tests both rejection of incomplete code-source metadata and acceptance of a complete synthetic source snapshot (not real law);
- requires local artifact bytes under an explicit root, verifies their SHA-256 digests for every referenced artifact in a candidate `PASS`, and rejects traversal or linked files that resolve outside the root;
- emits a machine-readable report with separate producer-declared `declared_status`, conservative `effective_status`, and `evidence_integrity_status` fields, plus the actual verifier-binary and schema SHA-256 values.

It does **not** parse or interpret legislation/SANS clauses, independently recompute structural/building-physics results, determine whether a measured result satisfies an engineering criterion, establish physical truth, verify a person's professional registration, or issue approvals. A successful package check sets `evidence_integrity_status: EVIDENCE_INTEGRITY_VERIFIED`; it does not replace the declared or effective engineering status. `effective_status` stays `PASS` only if the manifest declared PASS and all integrity checks succeed. The manifest's declared engineering status remains an assertion from the evidence producer.

## Run

From the Symtropy repository root:

```text
cargo run -p symtropy-housing-evidence -- docs/built-environment/fixtures/example-not-evidence.json
```

The checked-in fixture is deliberately synthetic and remains `NOT_EVALUATED`. It contains a placeholder hash and does not represent a building or engineering calculation.

## Candidate PASS

A separate requirements file has this shape:

```json
{
  "schema_version": "0.1.0",
  "schema_sha256": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
  "verifier_version": "0.1.0",
  "verifier_binary_sha256": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
  "case_id": "project-a/dwelling-001",
  "jurisdiction": {
    "country_code": "ZA",
    "region": "Gauteng",
    "municipality": "confirmed-municipality-id",
    "resolution_status": "confirmed"
  },
  "model": {
    "model_id": "dwelling-001",
    "revision_id": "revision-7",
    "format": "IFC",
    "schema_or_version": "IFC4X3_ADD2",
    "unit_system": "SI",
    "artifact_ids": ["dwelling-model"]
  },
  "model_artifacts": [
    {
      "artifact_id": "dwelling-model",
      "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    }
  ],
  "rule_pack": {
    "rule_pack_id": "ZA-low-rise-reviewed-v1",
    "status": "reviewed_for_use",
    "edition": "explicit-applicable-edition",
    "effective_date": null,
    "source_references": []
  },
  "toolchain": [
    {
      "tool_id": "approved-solver",
      "name": "Approved Solver",
      "version": "4.2.0",
      "executable_or_container_sha256": "0000000000000000000000000000000000000000000000000000000000000000",
      "configuration_artifact_sha256": null,
      "invocation": "solver --input model.ifc",
      "runtime": "pinned runtime image"
    }
  ],
  "mandatory_claims": [
    {
      "claim_id": "structural-baseline-case",
      "claim_type": "structural_analysis",
      "statement": "Required structural design checks for the named model elements.",
      "subject_element_ids": ["column-01", "beam-01"],
      "requirement": null,
      "tool_id": "approved-solver",
      "input_artifact_sha256": ["aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"],
      "output_artifact_sha256": ["bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"],
      "validity_domain": "defined model revision and stated load case",
      "acceptance_criteria": "All specified capacity and serviceability limits are met."
    }
  ]
}
```

The requirements contract must come from a controlled source independent of the manifest, and its expected SHA-256 digest must be supplied out of band. It pins the exact embedded schema digest, verifier package version and executable digest as well as an explicit mapping from each model artifact ID to its SHA-256 digest. This prevents digest assignments from being swapped between IDs while preserving an otherwise unchanged digest set. It pins the schema version, case ID, jurisdiction, exact model, rule-pack and toolchain objects, plus the complete mandatory claim contract—not just stable claim IDs. The model object pins its revision and artifact IDs; the independent contract binds each model artifact ID to its exact digest, and model, source-snapshot, tool and result bytes are verified locally. Each claim contract pins its type, statement, element scope, requirement reference, approved tool ID, exact input and output artifact digests, validity domain, and acceptance criteria. The all-zero/all-`a`/`b`/`c`/`d` executable, schema and input/output digests above are illustrative placeholders only; never use these literal values for a real case. Merely storing this file next to a manifest does not make it an independent trust anchor.

### Establishing the out-of-band pins

1. Build the exact verifier artifact intended for the run, for example with `cargo build --release -p symtropy-housing-evidence`.
2. Execute that exact binary against the deliberately `NOT_EVALUATED` fixture and retain its report. Use the report's `schema_sha256`, `verifier_version`, and `verifier_binary_sha256` values to populate the independent requirements contract. The fixture run establishes only the identity of the schema and executable; it does not validate a solver.
3. Have the responsible reviewer or release process construct and approve the full expected-claims contract from the controlled design requirements, approved rule pack, intended model, toolchain and mandatory claims. Include each claim's precise input and output artifact digests.
4. Compute the expected-claims file's SHA-256 over its exact bytes and convey/pin that digest through a trusted channel outside the manifest and requirements file. Keep the exact binary unchanged between the identity report and the candidate verification run. Rebuilding, changing target/platform, or modifying the embedded schema changes a pin and must require a new approved contract.
5. Run the candidate command with `--expected-claims`, the independently supplied `--expected-claims-sha256`, and `--artifact-root`. Any mismatch, missing artifact, or changed criteria must prevent `evidence_integrity_status: EVIDENCE_INTEGRITY_VERIFIED`.

Computing a hash is not the approval act: the expected contract and the channel that pins its digest are the trust anchor. Never copy the digest from the same untrusted package and treat that alone as independent approval.

Artifacts referenced by `model.artifact_ids`, `model.unit_mapping_artifact_sha256`, and claim input/output digest arrays must be present in `evidence_artifacts`. Their `uri` values must be safe, unescaped relative file paths under the artifact root. Example: `model/main.ifc`, with the file located at `<artifact-root>/model/main.ifc`.

```text
cargo run -p symtropy-housing-evidence -- \
  project-manifest.json \
  --expected-claims mandatory-claims.json \
  --expected-claims-sha256 <digest-pinned-out-of-band> \
  --artifact-root ./project-artifacts \
  --require-pass
```

The verifier hashes the exact bytes of the running executable, embedded schema, manifest, and provided requirements file and includes those digests and the verifier version in its JSON report. The expected-claims digest must be supplied from a trusted source outside the manifest; hashing a requirements file and then trusting a digest copied from that same untrusted file does not create an independent trust anchor.

## Exit codes

- `0`: verification completed without a blocking error. Inspect `declared_status`, `effective_status`, and `evidence_integrity_status`; this exit code is not engineering certification.
- `2`: malformed input/schema, inconsistent artifact references, unavailable/mismatched referenced bytes, or a declared `PASS` whose evidence package could not be integrity-verified.
- `3`: `--require-pass` was requested but the manifest did not declare `PASS` with `effective_status: PASS` and `evidence_integrity_status: EVIDENCE_INTEGRITY_VERIFIED`. This is an evidence-package gate, not certification.

Use `--require-pass` in gates where an engineering verification result is required. Preserve the JSON report as an evidence artifact.

## Solver adapter evaluation

See [solver-adapter-evaluation.md](../../../docs/built-environment/solver-adapter-evaluation.md) for the current research memo. It treats CalculiX as a candidate for a bounded static comparison—not an adopted or certified solver—and flags GPL-2.0 redistribution review and beam/truss connection cases as explicit gates. OpenSees is reserved as a later candidate until its product-license path is resolved.

## Analytical structural reference fixture

The repository also includes [`cantilever-beam-linear-static-v1.json`](../../../docs/built-environment/benchmarks/cantilever-beam-linear-static-v1.json) and an integration test in `tests/reference_cantilever_beam.rs`. It checks the elementary Euler-Bernoulli formulas for rectangular-section second moment of area, cantilever tip deflection, and support bending stress, and rejects non-positive input values.

This is an analytical oracle and unit/verification plumbing test only. It does **not** run or validate a finite-element solver, and the beam case is not a proxy for a real building element's full load combinations, buckling, connections, support behaviour, fire performance, or code compliance. The next structural milestone is to compare a suitable external solver against this oracle, then against reference FEM cases and qualified engineering review.

## Trust boundaries

An `evidence_integrity_status: EVIDENCE_INTEGRITY_VERIFIED` result requires a valid embedded schema, a schema-valid manifest, exact matching against the independently pinned schema/case/jurisdiction/model/rule-pack/toolchain contract, exact mandatory-claim coverage and criteria—including input and output digests—resolvable references, and locally verified artifact bytes. This verdict does **not** independently calculate structural capacity, heat transfer, fire behaviour, costs, or code compliance. The manifest's `overall_status: PASS` is still a declared claim by the producer. Independent solver verification, correct interpretation of applicable requirements, qualified review, and statutory approval remain separate gates.

The schema and verifier are versioned together. If the schema changes, its matching test fixture and verifier tests must be reviewed in the same change. The schema's JSON Schema validity does not prove that the mandatory requirement set, criteria, model revision, or rule-pack revision are the ones intended by the project; the externally pinned expected-claims contract constrains those values. The trusted party must still review and pin the contract itself.
