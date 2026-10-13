# Symtropy Housing Evidence Verifier

**Status: experimental verification tooling. Not a building-code engine or engineering certification.**

This binary is a fail-closed verifier for the draft housing evidence manifest. It:

- validates manifests against the checked-in JSON Schema Draft 2020-12 schema embedded at compile time;
- rejects external schema references, so schema validation cannot fetch remote content;
- checks artifact/claim identifiers and SHA-256 cross-references, including the executable/container and configuration digests of tools used by evaluated claims;
- requires a separately provided mandatory-claim set before accepting an aggregate `PASS`;
- requires local artifact bytes under an explicit root and verifies their SHA-256 digests for every referenced artifact in a candidate `PASS`;
- emits a machine-readable report that distinguishes the declared status from the verifier's effective status.

It does **not** parse or interpret legislation/SANS clauses, check structural calculations, establish physical truth, verify a person's professional registration, or issue approvals. Hash verification shows only that retrieved bytes match a declared digest.

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
  "mandatory_claim_ids": [
    "structural-baseline-case",
    "fire-performance-case"
  ]
}
```

The requirements file must come from a controlled source independent of the manifest being evaluated, and its expected SHA-256 digest must be supplied out of band. Merely storing it next to a manifest does not make it an independent trust anchor.

Artifacts referenced by `model.artifact_ids`, `model.unit_mapping_artifact_sha256`, and claim input/output digest arrays must be present in `evidence_artifacts`. Their `uri` values must be safe, unescaped relative file paths under the artifact root. Example: `model/main.ifc`, with the file located at `<artifact-root>/model/main.ifc`.

```text
cargo run -p symtropy-housing-evidence -- \
  project-manifest.json \
  --expected-claims mandatory-claims.json \
  --expected-claims-sha256 <digest-pinned-out-of-band> \
  --artifact-root ./project-artifacts \
  --require-pass
```

The verifier hashes the exact bytes of the manifest, embedded schema, and provided requirements file and includes those digests and the verifier version in its JSON report. The expected-claims digest must be supplied from a trusted source outside the manifest; hashing a requirements file and then trusting a digest copied from that same untrusted file does not create an independent trust anchor.

## Exit codes

- `0`: the manifest/schema check completed without a blocking error. This alone does **not** mean the building passed; inspect `effective_status`.
- `2`: malformed input/schema, inconsistent artifact references, unavailable/mismatched referenced bytes, or a declared `PASS` that the verifier cannot support.
- `3`: `--require-pass` was requested but the effective status is a valid non-`PASS` state such as `NOT_EVALUATED` or `REQUIRES_REVIEW`.

Use `--require-pass` in gates where an engineering verification result is required. Preserve the JSON report as an evidence artifact.

## Analytical structural reference fixture

The repository also includes [`cantilever-beam-linear-static-v1.json`](../../../docs/built-environment/benchmarks/cantilever-beam-linear-static-v1.json) and an integration test in `tests/reference_cantilever_beam.rs`. It checks the elementary Euler-Bernoulli formulas for rectangular-section second moment of area, cantilever tip deflection, and support bending stress, and rejects non-positive input values.

This is an analytical oracle and unit/verification plumbing test only. It does **not** run or validate a finite-element solver, and the beam case is not a proxy for a real building element's full load combinations, buckling, connections, support behaviour, fire performance, or code compliance. The next structural milestone is to compare a suitable external solver against this oracle, then against reference FEM cases and qualified engineering review.

## Trust boundaries

A full aggregate `PASS` requires all of the following: a valid embedded schema, a schema-valid manifest, confirmed jurisdiction, a rule pack explicitly marked `reviewed_for_use`, an exact match between manifest mandatory claims and the independent expected-claim set, references that resolve, and locally verified artifact bytes. This is necessary for integrity but still not sufficient for regulatory approval.

The schema and verifier are versioned together. If the schema changes, its matching test fixture and verifier tests must be reviewed in the same change. The schema's JSON Schema validity does not prove that the manifest's mandatory requirement set is complete; the external expected-claims file is an explicit additional input for that reason.
