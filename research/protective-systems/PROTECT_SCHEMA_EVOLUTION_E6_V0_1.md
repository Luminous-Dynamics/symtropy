# Protective Systems E6 — Schema lifecycle, compatibility and promotion control

**Snapshot:** 17 November 2026  
**Owner:** PROTECT-SCHEMA-EVOLUTION-001 #5728  
**Branch:** `research/protect-schema-evolution-e6-v0-1`  
**Parent:** E4 typed semantic projections #5643 / PR #5708  
**Adjacent work:** E5 capability assurance #5710 / PR #5712  
**Status:** dated six-profile registry, three upstream schema-root snapshots, Rust admission gate and 27 tests written; exact-head hosted execution pending

## 1. Paradigm shift: schema evolution is an evidence-affecting change event

A provider schema does not merely change how JSON is parsed. It can change identifier semantics, controlled vocabularies, relationship meanings, requiredness, time/CRS fields, rights declarations and the interpretation of historical evidence.

E6 distinguishes three things that are often conflated:

1. **Standard lifecycle:** whether the upstream document is final, draft, deprecated, retired or unknown.
2. **Adapter validation scope:** which exact schema/profile and which subset of requirements this repository has actually validated.
3. **Promotion/admission state:** whether a profile can be used to make new evidence claims, only be evaluated in shadow mode, remain selected-profile-only, or be used only to replay historical records.

A final standard does not automatically mean our adapter is fully conformant. A passing parser does not mean a draft standard is production-ready. A version number increase is not a compatibility proof.

## 2. Standards lifecycle snapshot (17 November 2026)

| Standard profile | Upstream lifecycle | E6 validation scope | Current E6 admission |
|---|---|---|---|
| DataCite Metadata Schema 4.7 | FINAL | Selected JSON:API/4.7 field rules; transitive XSD dependencies not bundled | FINAL_SELECTED_PROFILE_ONLY |
| DataCite Metadata Schema 4.8 draft | DRAFT preview | Not validated | DRAFT_SHADOW_ONLY |
| OGC API - Records Part 1 Core 1.0.0 | FINAL | Selected GeoJSON record checks; external schema references not bundled | FINAL_SELECTED_PROFILE_ONLY |
| OGC API - Records Part 2 Facets | DRAFT | Not validated | DRAFT_SHADOW_ONLY |
| OGC API - Records Part 3 CRUD/Harvest | DRAFT | Not validated | DRAFT_SHADOW_ONLY |
| OpenLineage RunEvent 2.0.2 | FINAL | Pinned official RunEvent JSON Schema; exact-head receipt pending | FINAL_CONFORMANCE_PENDING |

Sources checked:
- DataCite release history: https://schema.datacite.org/versions.html
- DataCite version inventory showing 4.8_draft and 4.7 default: https://app.readthedocs.org/projects/datacite-metadata-schema/versions/
- DataCite 4.7 XSD and release: https://schema.datacite.org/meta/kernel-4.7/
- OGC API - Records standard status: https://ogcapi.ogc.org/records/
- OGC official schema repository: https://schemas.opengis.net/ogcapi/records/part1/1.0/openapi/schemas/
- OpenLineage 2.0.2 schema: https://openlineage.io/spec/2-0-2/OpenLineage.json
- OpenLineage run-event semantics: https://github.com/OpenLineage/OpenLineage/blob/main/spec/OpenLineage.md

DataCite 4.8 is listed as a hidden draft build; 4.7 remains the default release. OGC Part 1 Core 1.0 is the standard, while Parts 2 and 3 are listed as drafts. E6 therefore keeps those drafts in shadow mode and does not treat separate OGC parts as compatible versions of Part 1.

## 3. Pinned root schema snapshots

The following exact bytes are committed in the E6 branch and checked by the Rust verifier.

| Profile | Local snapshot | Bytes | Upstream Git blob SHA-1 | SHA-256 |
|---|---|---:|---|---|
| DataCite kernel-4 / 4.7 | `verifier/schemas/datacite-4.7/metadata.xsd` | 42,747 | `b7cad8149301676bc39f17b6a77d776ab6556f57` | `be878f9afddeb677ec4cd274233de0c328246748305d9f58ee78352714412522` |
| OGC Records Part 1 GeoJSON | `verifier/schemas/ogc-records-1.0/recordGeoJSON.yaml` | 957 | `e9e4fcb700e2c5e19d26dbf0eb7f9f2be938768f` | `cc9c94abd30adf17dfba3fc7ff344afcd36376dd43b3519f535233994edf13b3` |
| OpenLineage RunEvent 2.0.2 | E3 vendored schema | 9,155 | `58167f71407add7734810cda9943b39b83c89aa2` | `69f68bee00b9beac88a87059c0102410e7bb05f3f43c46d02a0409831eceb0d2` |

**Dependency-closure warning:** the DataCite XSD imports the W3C XML namespace schema and includes additional local XSD files. The OGC GeoJSON record schema references other OGC record schemas and an OGC API - Features geometry schema. Those transitive assets are listed in the registry but are not all bundled in E6. E6 therefore labels both profiles as `PARTIAL_EXTERNAL_REFERENCES` and keeps them at selected-profile-only. The OpenLineage 2.0.2 JSON Schema is locally vendored and its current scope is the RunEvent JSON Schema, not the full OpenLineage service/API.

## 4. Admission policy

The E6 Rust gate evaluates profiles using lifecycle, schema digest, dependency closure, validation scope and receipt:

- `DRAFT` → `DRAFT_SHADOW_ONLY`, regardless of parser success, version ordering or a synthetic PASS receipt.
- `FINAL` + incomplete dependency bundle or selected-only validation → `FINAL_SELECTED_PROFILE_ONLY`.
- `FINAL` + complete pinned bundle + full declared schema scope + pending exact-head CI → `FINAL_CONFORMANCE_PENDING`.
- `FINAL` + complete pinned bundle + full declared schema scope + `PASS_EXACT_HEAD` receipt → `FULLY_ADMITTED`.
- `DEPRECATED`/`RETIRED` → `HISTORICAL_REPLAY_ONLY`; no new record admission.
- Missing/invalid digest, unknown lifecycle or unknown validator status → `ABSTAIN_UNVERIFIED`.

The report is deliberately conservative: as of this snapshot there are zero fully admitted profiles. The OpenLineage schema is pinned and has a declared full RunEvent-schema scope, but exact-head hosted validation remains pending.

## 5. Compatibility is a semantic decision, not a version comparison

Compatibility classes are explicit:
- `EXACT_SAME_PROFILE`
- `PATCH_COMPATIBLE`
- `ADDITIVE_COMPATIBLE`
- `LOSSY`
- `SEMANTIC_BREAK`
- `BREAKING`
- `UNKNOWN`
- `NOT_COMPARABLE_SEPARATE_PART`

The snapshot records DataCite 4.7 → 4.8 draft as `UNKNOWN` and blocked. OGC Parts 2 and 3 are separate draft parts, not version upgrades to Part 1 Core. OpenLineage 2.0.2 → 2.0.2 is exact same profile but still waits for the E3 exact-head receipt.

No compatibility class is inferred from version numbers. Changes to identifiers, rights, relation semantics, lineage, units, timestamps, CRS, cardinality or requiredness must be evaluated field by field. A new field may be retained raw in E4, but it may not enter a typed decision field without a versioned mapping decision.

## 6. Rust gate and tests

Binary: `protect-schema-evolution-e6`

From repository root:

```sh
cargo fmt --manifest-path research/protective-systems/verifier/Cargo.toml --all -- --check
cargo clippy --manifest-path research/protective-systems/verifier/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path research/protective-systems/verifier/Cargo.toml --bin protect-schema-evolution-e6
cargo run --manifest-path research/protective-systems/verifier/Cargo.toml --bin protect-schema-evolution-e6
```

The gate validates the Draft 2020-12 lifecycle registry schema, unique profile identities, compatibility references, the no-auto-upgrade policy, pinned root bytes/blob identities/SHA-256 values and the local DataCite/OGC/OpenLineage schema identifiers.

**27 unit tests are written**, including:
- final vs draft vs deprecated vs retired vs unknown state;
- missing/invalid schema digest abstention;
- incomplete dependency bundle restricting a final profile to selected-profile-only;
- exact-head receipt required for full admission;
- a draft remaining shadow-only even with a synthetic PASS receipt;
- version increment not implying compatibility;
- unknown compatibility requiring review/block/abstention;
- unknown profile references and duplicate profile identities rejected;
- root schema snapshot tampering/digest mismatch negative controls;
- no automatic upgrade and deterministic historical replay status.

Formatting, Clippy, compilation, test execution and executable output must be checked on the exact E6 head. A test count is not a passing result.

## 7. Benchmarks

| Metric | Prospective target |
|---|---:|
| Lifecycle state correctly distinguished | 100% frozen registry |
| Draft/unknown profile auto-promotions | 0 |
| Full admission without exact pinned schema SHA-256 | 0 |
| Full admission without complete dependency bundle | 0 |
| Full admission without exact-head validator receipt | 0 |
| Unreviewed semantic-breaking change admitted | 0 |
| Removed/unknown field silently defaulted or dropped | 0 |
| Historical profile replay determinism | 100% frozen corpus |
| Changed-field mapping manifest coverage | 100% |
| Independent validator disagreement concealed | 0 |
| Change-detection lag, review effort, migration CPU/memory | p50/p95 measured |

Compare E6 to a conventional schema registry, simple dependency/version bumps and manual adapter updates on the same frozen change corpus. The advantage to prove is not faster parsing; it is safer semantic promotion, better historical replay and less silent evidence reinterpretation.

## 8. Claim ceiling

E6 is a dated schema-lifecycle snapshot and a promotion-policy oracle. It does not continuously poll upstream services, validate every transitive XSD/YAML dependency, certify full DataCite/OGC server conformance, authenticate providers or prove scientific/physical correctness. Draft profiles remain shadow-only until their lifecycle, schema bundle, independent validator and mapping evidence justify promotion.
