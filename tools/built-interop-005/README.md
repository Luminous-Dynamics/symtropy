# BUILT-INTEROP-005 — Coverage-aware IFC schema validation gate

A small Rust CLI that converts the upstream OpenBIM.rs schema validator report into one `ifc_schema_conformance` gate compatible with BUILT-INTEROP-004. This is executable gate code, not an assertion that every normative IFC rule is implemented.

## Why this is a separate gate

The pipeline distinguishes file identity, STEP syntax, IFC schema validation, IDS XML/XSD validity, IDS semantics, IFC-vs-IDS conformance, exchange preservation, and downstream engineering qualification. A file may parse while still violating schema rules.

The tool uses:
- `ifc-model = 0.3.1`
- `ifc-step = 0.5.0`
- `ifc-schema = 0.3.2`
- `ifc-validate = 0.6.0`

The versions are exact in Cargo.toml. CI commits and hashes the generated Cargo.lock; the final workflow is expected to validate with `--locked` against the committed lock.

## Run

```sh
cargo run --locked --manifest-path tools/built-interop-005/Cargo.toml -- path/to/model.ifc
cargo test --locked --manifest-path tools/built-interop-005/Cargo.toml
```

No argument defaults to a small, synthetic IFC4 wall fixture. The CLI prints JSON in all validation outcomes and exits with `0=PASS`, `1=FAIL`, or `2=UNRESOLVED`. File I/O/usage failures return a separate nonzero code and write diagnostics to stderr.

## Gate policy

- **FAIL:** at least one reported schema error exists.
- **UNRESOLVED:** parsing fails, the declared schema is absent/unknown/unbundled, the model is incomplete, a rule evaluation errors, any checks are reported unsupported, or the report is truncated.
- **PASS:** the named validator reports zero errors, zero evaluation errors, zero unsupported findings, and a complete result.

Warnings remain visible but do not alone block the gate. The report preserves the validator's `errors`, `evaluation_errors`, `warnings`, `unsupported`, and `truncated` counts and its stable findings (severity, rule ID, path and message). It includes SHA-256 and byte length for the exact input bytes. The evidence digest hashes the deterministic serialized `evidence_payload`; the gate references it with JSON Pointer locator `report-json-pointer:/evidence_payload`.

The profile/configuration hash pins this policy identifier: `DECLARED-IFC-SCHEMA-SUPPORTED-RULES-v0.1` plus the fail-closed mapping. The gate reports dependency versions and does not guess a schema: `validate_declared` refuses unknown or unbundled schemas.

## Explicit limitations

Upstream documents that `ifc-validate` does not evaluate arbitrary EXPRESS WHERE expressions, does not derive all INVERSE relationships, and leaves some expression-based aggregate constraints unsupported. Its `Report::is_conformant()` intentionally counts errors and evaluation errors but does **not** count unsupported findings. This CLI therefore checks the summary counts separately and blocks a PASS when unsupported > 0.

Even a PASS is bounded to the named, implemented validator profile and the declared schema tables. It does not claim complete normative IFC semantics, IDS compliance, geometry equivalence, building-code compliance, design adequacy, structural safety, manufacturing release or physical qualification. These require independent gates and competent discipline-specific evidence.

## Current test controls

The Rust tests exercise (1) input hash/length and declared-schema provenance, (2) malformed STEP -> UNRESOLVED, (3) unknown schema -> UNRESOLVED rather than IFC4 fallback, (4) malformed GlobalId -> schema FAIL, (5) all fail-closed status mapping cases including unsupported/evaluation-error/truncated, (6) stable output/evidence digest, and (7) structured finding fields.

## Benchmarks

Acceptance targets:
- 100% of seeded malformed/unsupported/unknown-schema controls fail closed.
- 0 schema-version mismatches evaluated against a guessed table.
- 0 unsupported/evaluation-error/truncated cases promoted to PASS.
- 100% of findings preserve rule IDs and paths as structured fields.
- Byte-identical report payload for repeat runs on identical input bytes and profile.
- Report parse time, validator time, peak memory, error/finding counts, unsupported counts and truncation reported separately at scale; no claim of speed/accuracy improvement without a measured baseline.

## Research references

- [`ifc-validate` 0.6.0 API and limitations](https://docs.rs/ifc-validate/latest/ifc_validate/)
- [`ifc-schema` 0.3.2 API and bundled schema profiles](https://docs.rs/ifc-schema/latest/ifc_schema/)
- [buildingSMART IDS implementer guide](https://github.com/buildingSMART/IDS/blob/development/Documentation/ImplementersDocumentation/developer-guide.md) — IDS XML/XSD and semantic auditing remain independent of IFC schema validation.
