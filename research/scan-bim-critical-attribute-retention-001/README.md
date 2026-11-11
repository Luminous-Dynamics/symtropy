# SCAN-BIM-CRITICAL-ATTRIBUTE-RETENTION-001 — Requirement-weighted property continuity v0.1

**Status:** P0 typed receipt auditor and synthetic fixture implemented; exact-head Rust/CI qualification pending.  
**Issue:** [#4763](https://github.com/Luminous-Dynamics/symtropy/issues/4763)  
**Parent:** [Representation continuity PR #4764](https://github.com/Luminous-Dynamics/symtropy/pull/4764)  
**Branch:** `research/scan-bim-critical-attribute-retention-v0-1`

## 1. Paradigm: properties are obligations, not just fields

IFC parsing, schema validity, IDS conformance and semantic diff answer different questions. A file can parse and still lose a project-required material or performance property. A structural diff can identify that a value changed; it does not know whether the property is mandatory for this project, whether the target value is acceptable, or whether the change blocks the intended exchange.

This module consumes typed producer evidence and applies the exact project's requirement profile. It does **not** infer criticality from names such as `FireRating`, `ThermalTransmittance` or `AcousticRating`. The project owner declares whether each obligation is critical, important or advisory and whether it is required.

## 2. Ownership — consume existing authorities

| Concern | Existing owner | This module |
|---|---|---|
| IFC delivery gates, exchange profile and edit/round-trip qualification | #4271 / #4273 | Consumes delivery scope |
| Representation lineage, claim origin, uncertainty and topology | #4471 / #4764 | Consumes exact revision/entity context |
| IFC schema-directed semantic snapshot/diff | #2117 / #2126 | Consumes its report digest and per-property finding IDs |
| Source/target semantic preservation | #2073 / #2074 | Consumes per-property status/findings for its supported IFC4/IDS profile |
| Unit-aware quantity equivalence | #2143 (proposal/owner) | Consumes a scoped report when its supported implementation is available; performs no conversion |
| Physical component identity | #3331 | Consumes correspondence event and exact source/target entity IDs |

The adapter checks receipt consistency and project disposition. It does not authenticate referenced report bytes or prove their producers are correct.

## 3. P0 receipt and dispositions

The versioned closed JSON Schema is `schema/property-retention-receipt.schema.json`; the synthetic known answer is `fixtures/known_answers_v0_1.json`. The Rust module is `tools/scan-bim-replay-contract/src/property_retention.rs`, exported by the existing crate, with CLI `scan_bim_property_retention_audit`.

Every receipt binds source/target IFC digests and revisions, replay/scene, exchange profile, requirement-profile digest and completeness, semantic-diff report digest, identity-correlation report digest, and (when normalized equivalence is claimed) a unit-comparison report digest.

Every declared requirement binds exact subject identity, IFC entity/property-set/property name/data type, project-declared importance and requiredness. Exactly one comparison row is required per requirement. Comparison evidence binds source/target states and value digests, correspondence event, outcome, findings, and reason.

Disposition semantics:

- `preserved_exact`: source and target are present and exact value digests match.
- `preserved_normalized`: source/target raw values differ, and a separately scoped unit/semantic comparison receipt supplies a canonical value digest and finding ID. This module does not compute equivalence.
- `changed`: two different present values with a semantic-diff finding and reason.
- `lost`: source present, target absent, with a semantic finding.
- `added`: source absent, target present; not counted as source-property retention.
- `unsupported`: target profile reports the property unsupported; remains visible and cannot qualify.
- `unresolved`: unit, identity, profile or evidence comparison is incomplete/ambiguous.
- `absent_both`: explicitly distinct from transformation loss; a required critical property absent on both sides is a requirement gap and blocks the profile.

A numeric mismatch without a unit comparison receipt cannot become normalized preservation. A missing or unsupported unit comparator is visible as unresolved/unsupported, not assumed equivalent.

## 4. Outcome policy

- Any required critical property that is changed, lost or absent on both sides => `blocked`.
- Any critical unsupported/unresolved property or incomplete/unknown requirement inventory => `unresolved`.
- Required important property loss/change or advisory degradation => `degraded`.
- `preserved` only when the requirement inventory is complete, all critical obligations are supported and no required important degradation remains.

The report keeps separate denominators and counts. It does not collapse schema compliance, property retention or geometry into a weighted score.

## 5. P0 synthetic fixture and tests

The frozen known-answer fixture contains three required properties: FireRating (critical), ThermalTransmittance (critical, normalized by a synthetic external receipt), and AcousticRating (important). All three are preserved in the positive case.

The 22 module unit tests cover exact and normalized preservation, critical loss/change, critical unsupported and absent-both states, important/advisory degradation, missing unit evidence, wrong entity identity, missing/duplicate comparison rows, duplicate requirements, incomplete profiles, unknown-to-default promotion, criticality independent of property naming, target-only additions, critical/required consistency, bounded finding reasons and input-order invariance.

The exact-input SHA-256 is intentionally byte-sensitive; findings are canonicalized by requirement ID.

## 6. Benchmark design and proposed targets

No weighted composite can offset a critical property loss with good geometry or schema conformance.

| Track | Measure |
|---|---|
| Required-property retention | Preserved required obligations / source-present required obligations, with exact numerator/denominator |
| Critical loss | Missing/changed critical values / source-present critical values |
| Critical unresolved | Unsupported/unresolved critical obligations / declared critical obligations |
| Coverage | Independently compared obligations / total declared obligations |
| Requirement completeness | Complete/partial/unknown inventory state and count of uncovered obligations |
| Operational exchange | Per-toolchain failure rate, manual corrections, review minutes and round-trip latency |

Proposed P0 gates (not results): 100% frozen-oracle agreement; zero critical loss or unresolved promoted to preserved; 100% wrong artifact/revision/entity/profile binding rejection; 100% deterministic findings under input permutations; exactly one comparison outcome per declared requirement.

P1 should use independently reviewed, rights-cleared building models and at least two exporter/importer toolchains. Freeze each project's requirement/IDS profile before transformation. Report by building, tool version and property family; do not treat hundreds of properties in one IFC file as hundreds of independent buildings.

## 7. Industry and research comparison

The 2026 Mutis & Dulanto Revit–ArchiCAD study reports material-property loss across house, hotel and hospital models, with custom and regulatory properties (including fire/thermal fields) among the vulnerable areas. It is a direct comparator for property continuity, not evidence that every Revit/ArchiCAD exchange loses the same properties: https://doi.org/10.1061/JSDCCC.SCENG-2348

The 2026 transformation-fidelity paper proposes PRR, SFI, SCR, ADS and EL across IFC, gbXML and COBie. It highlights why schema compliance is not semantic fidelity: https://doi.org/10.1016/j.rineng.2026.112704. Do not claim to reproduce its exact PRR/SFI/SCR/ADS equations unless their published definitions, denominators, and unsupported/missing treatment are implemented and matched.

buildingSMART IDS 1.0 can express property, material, classification and value requirements; it does not itself validate geometry or external/dynamic physical behaviour: https://www.buildingsmart.org/standards/bsi-standards/information-delivery-specification-ids/

The differentiation is not “we have a property diff.” It is the fail-closed joining of project-declared criticality, typed property comparison evidence, identity correspondence, independent unit evidence and explicit exchange outcome—without building another IFC or unit-conversion engine.

## 8. Claim ceiling

P0 checks synthetic receipt semantics only. It does not parse or authenticate IFC bytes, establish requirement completeness, perform unit conversion, validate fire/thermal/acoustic performance, establish code compliance, structural safety or asset-mutation authority. External results apply only to named tool versions, schemas, profiles and test populations.
