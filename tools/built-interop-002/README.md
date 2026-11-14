# BUILT-INTEROP-002 — pinned IFC/IDS ingestion and a narrow requirement adapter

Tracking issue: https://github.com/Luminous-Dynamics/symtropy/issues/2010

## v0.1 supported profile

This is a **small real-format ingestion experiment**, not a general IDS checker or building-code tool. It parses:

- an IFC STEP file through `ifc-step = 0.5.0` and `ifc-model = 0.3.1`;
- an IDS document through `openbim-ids = 0.2.2`;
- only an explicitly declared IFC4 model and IDS 1.0 version;
- exactly one IDS specification, with exactly one `IFCWALL` applicability facet and exactly one required `Name` attribute carrying a literal `simpleValue`.

It then checks the parsed `Name` of every applicable `IFCWALL` against that IDS literal. For the pinned IFC4 profile, `IfcWall` inherits `IfcRoot.Name` at attribute slot 2; the slot is named as a constant and covered by synthetic test fixtures. This is not schema-generated attribute lookup and must not be generalized to other classes or schema releases without a new reviewed profile.

## Why the adapter is narrow

The published [OpenBIM.rs IDS status page](https://github.com/openbimrs/ids#status) says that IDS 1.0 reading/writing and IDS-document-vs-IFC-schema audit exist, but **checking an IFC model against IDS has not been implemented upstream**. This crate therefore evaluates only its declared subset. Extra properties, classifications, materials, `partOf`, restrictions/patterns, extra specifications, optional/prohibited cardinalities, version conflicts, and unknown attributes produce explicit blocked/unresolved results instead of being silently treated as passing.

`SubsetConformant` does not mean the IFC is normatively valid, a building is structurally safe, a permit condition is met, a product is qualified, or manufacturing can proceed.

## Exact crate pins and license boundary

The direct IFC/IDS crate versions are exact in `Cargo.toml`:

- `ifc-step = 0.5.0`
- `ifc-model = 0.3.1`
- `openbim-core = 0.1.0`
- `openbim-ids = 0.2.2`

The first two are AGPL-3.0-or-later. `openbim-ids` is MIT with default features; this crate intentionally enables **no** IDS audit feature. In particular, it does not enable the `audit` feature whose standard template data is documented as CC BY-ND 4.0. All direct dependencies are version-pinned and `tools/built-interop-002/Cargo.lock` is committed from the exact Rust 1.96.0 CI resolver output. The focused workflow checks the lockfile SHA-256 and uses `cargo test --locked` / `cargo clippy --locked`, so an accidental dependency re-resolution fails instead of silently changing the test environment.

## Run

```sh
cargo fmt --manifest-path tools/built-interop-002/Cargo.toml -- --check
cargo test --manifest-path tools/built-interop-002/Cargo.toml
cargo run --manifest-path tools/built-interop-002/Cargo.toml
```

The default run consumes the two synthetic files in `fixtures/`. Pass explicit IFC and IDS paths to compare other inputs within the same narrow profile. It emits raw artifact SHA-256 hashes, parser-declared schema/version evidence, per-wall observations, explicit dispositions, and a semantic result digest sorted by stable object identity. The raw hashes remain byte-sensitive when the semantic digest is stable under record-order changes. Each per-wall observation also identifies the exact IFC entity type, STEP record ID, attribute index (`2` in this IFC4-only profile), and attribute path (`IFCWALL.Name`). GlobalIds are checked for the 22-character IFC compressed-GUID lexical shape before use; malformed, missing or duplicate wall IDs mark affected observations `NotEvaluated`. Every member of a duplicate-ID set is marked ambiguous; the first occurrence does not remain falsely presented as an unambiguous preservation. This lexical check is not a substitute for authoritative IFC schema validation.

## Outcomes

- `SubsetConformant`: all declared profile conditions match and each applicable IFC wall has exactly the required literal `Name`.
- `Blocked`: schema/profile mismatch, no required applicable wall, a missing Name, or a value mismatch.
- `Unresolved`: IDS version is inferred rather than explicitly declared, the parsed spec is outside the implemented subset, or another prerequisite is unknown/unsupported.
- Malformed IFC STEP or malformed IDS XML is a parse error and a nonzero CLI exit, not a usable partial pass.

## Synthetic controls

The Rust test suite contains controls for the happy path, wrong literal, missing literal, mismatched IFC schema token, unsupported requirement facet, no applicable wall, malformed STEP, and order-independent semantic digest. These are software-contract fixtures only; they are not benchmark results from commercial CAD export, actual buildings, or a physical manufacturing process.

## Next integration steps

1. Generate and commit the reviewed Cargo.lock; verify all resolved licenses and checksums.
2. Keep schema validity as a separate gate. An integration candidate is Rust `ifc-schema` + `ifc-validate`: schema tables interpret the generic model, and the validator reports structural/type findings plus unsupported rules explicitly. Its docs state that some aggregate bounds and inverse-derived rules are not evaluated, so a clean report must be accompanied by unsupported-rule counts. Pin exact compatible crate versions and regenerate/review the lock before integrating; do not call parser success schema validity. References: https://docs.rs/ifc-schema/0.3.2/ifc_schema/ and https://docs.rs/ifc-validate/0.6.0/ifc_validate/.
3. Only after the exact subset is stable, grow to IFC schema-derived attribute lookup and explicit typed IFC properties; don’t hard-code more positional slots.
4. Extend IDS feature-by-feature with the full official IDS positive/negative corpus and independently pinned oracle results. Unimplemented facets must remain `NotEvaluated`.
5. Bind outputs to BUILT-INTEROP-001's structured contract by exact input digests and object/property identities, then measure semantic loss on exact-version exporter/importer pairs.
