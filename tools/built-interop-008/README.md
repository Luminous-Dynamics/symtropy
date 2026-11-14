# BUILT-INTEROP-008 — Schema-directed IFC structural semantic diff

This Rust CLI compares source and target IFC4 models after a format conversion or design exchange. It compares *structured model meaning within a declared bounded profile*, not raw file bytes.

## Run

```sh
cargo run --locked --manifest-path tools/built-interop-008/Cargo.toml -- \
  --source-ifc original.ifc --target-ifc exported.ifc \
  --output semantic-diff.json
cargo test --locked --manifest-path tools/built-interop-008/Cargo.toml
```

Exit codes: `0 = EQUAL`, `1 = DIFFERENT`, `2 = UNRESOLVED`, `64 = usage or I/O failure`. It prints one structured JSON report and can also write the report to a path.

## Canonicalization

- Uses the pinned IFC4 schema table to name positional attributes, not raw slot numbers where the schema knows the attribute.
- Parses with the pinned STEP codec and refuses to compare incomplete models, missing/unrecognized schema tokens or non-IFC4 schemas.
- Keys IfcRoot objects by a valid, unique IFC compressed GlobalId. Missing, malformed or duplicate identity is a coverage issue; it never becomes an assumed object match.
- Uses explicit local STEP-ID keys for non-root objects in v0.1. Changes in non-root numbering can therefore produce conservative false-positive changes. The report names this limitation; it does not claim graph-isomorphism.
- Preserves null versus derived, logical-unknown, enum/text/binary/typed values, integers versus reals, and ordered LIST/ARRAY values.
- Normalizes SET/BAG aggregate order only when the schema table marks the attribute with that aggregate kind. It does not invent unit conversions, tolerances or geometry equivalence.
- Canonically represents references through a stable target GlobalId when available; references to objects with only local STEP IDs retain that sensitivity.
- Emits raw source/target SHA-256 and byte lengths separately from semantic snapshot/result digests. Header-only changes can change raw hashes while remaining semantically equal in this profile.

## Result meanings

- **EQUAL:** canonical snapshots match and the profile had no recorded schema coverage, identity or reference uncertainty.
- **DIFFERENT:** canonical structural records differ and the profile has enough coverage for a comparison.
- **UNRESOLVED:** the model cannot be safely compared under this profile (parse/schema precondition, unknown entity, arity mismatch, ambiguous GlobalId or unresolved reference). Partial differences remain visible where snapshots were available.

A source/target pair with valid parse and same schema is not automatically equal: object identity, named attributes and typed references are compared. A DIFFERENT result is not necessarily an unacceptable design change; a separate requirement set or independent oracle must determine whether a specific difference matters.

## Included tests

The synthetic IFC4 fixture and Rust tests cover identical structural input, header-only byte changes, reordered DATA records and STEP-ID renumbering with stable GlobalIds, named attribute loss/change, GlobalId changes without guessed pairing, invalid/duplicate IDs, unsupported schemas, malformed STEP, attribute arity mismatch, GUID-based reference canonicalization, schema-defined SET/BAG versus LIST ordering, and deterministic result digests. A table-driven 64-case mutation matrix alters 32 wall names and 32 unique GlobalIds and asserts that every mutation is detected. Coverage issues are sorted before report hashing so input record order cannot reorder diagnostics.

## Benchmarks

Initial acceptance targets:
- 100% detection of seeded critical property/identity mutations in this bounded fixture suite.
- 0 unresolved/duplicate/missing IDs silently mapped to a stable root identity.
- 100% preservation of source/target byte digests and named changed attributes.
- 0 non-IFC4 or incomplete inputs reported EQUAL.
- Deterministic result digest for identical parsed semantics; report raw-digest changes separately.
- Measure false acceptance/rejection, stable GlobalId coverage, unresolved coverage, parse/snapshot/diff time, records/sec, peak memory and report size on rights-cleared golden exports.

Future coverage should add domain-specific property sets, quantities, units, placements, relationships, material layers and geometry with explicit tolerances. Until then, none of those downstream improvements is claimed.

## Limitations

This is not a complete normative EXPRESS checker, general IDS model checker, geometry-equivalence kernel, building-code inspector or structural / life-safety qualification tool. Results do not establish a building is safe to construct, nor demonstrate cost, rework or carbon improvements. Use independent licensed exports and a separately maintained golden oracle before reporting real-world rates.
