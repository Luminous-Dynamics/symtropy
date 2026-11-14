# Power Fabric standards adapter and round-trip harness v0.1

**Status:** executable adapter-conformance contract; no concrete IEC/CIM/SCL parser or conformity result claimed  
**Date:** 2026-11-14  
**Owner issue:** https://github.com/Luminous-Dynamics/symtropy/issues/5215  
**Parent chain:** PR #5222 → #5218 → #5217 → #5211

## 1. Why a mapping ledger comes before parsers

Standards interoperability is a semantic preservation problem, not merely a file-format exercise. An adapter can emit valid XML or JSON while silently dropping identity, relationships, namespaces, units, extensions, qualifiers, or applicability context.

This increment defines the reporting contract first. Concrete parsers and exporters must later demonstrate that they populate it faithfully for an exact source profile and edition.

## 2. Exact standards pin

A mapping profile pins:
- standard identifier and part;
- edition and amendments/corrigenda;
- publication status (Published, WorkingDraft, TechnicalReport, NationalAdoption, Withdrawn);
- jurisdiction where applicable;
- adapter version;
- target Power Fabric IR schema version;
- written applicability scope;
- source input and target output digests for each run.

A standards lifecycle change does not mutate a historical profile. A new edition creates a new profile version and a separately qualified transform.

## 3. Mapping outcomes

| Outcome | Required structure | Meaning |
|---|---|---|
| ExactPreservation | Target and no loss entries | Source semantic item preserved exactly under the declared mapping |
| SemanticEquivalent | Target and no loss entries | Different representation, same declared meaning |
| LossExplicit | Target and one or more loss entries | Target emitted with every declared loss listed |
| Unsupported | No target and explanation | No mapping is provided |
| Ambiguous | No target and explanation | More than one plausible semantic mapping exists |

Unsupported and ambiguous records make the run incomplete. Explicit losses make it CompleteWithDeclaredLoss, not lossless success. An incomplete source inventory takes precedence over every other status. A complete inventory whose declared counts do not match its mapping or extension records is rejected.

The kernel requires one record per source semantic reference and checks caller-supplied source semantic-item and extension counts against the record counts when the inventory is declared complete. If one source item expands into several target items, a future profile version must introduce a grouping record rather than hiding multiplicity in a string.

## 4. Extension handling

Every source extension must be recorded as:
- PreservedOpaque: target extension ID and source/target payload digests match;
- Mapped: explicit target extension ID and target digest;
- UnsupportedExplicit: no target and an explanation.

No missing record is equivalent to preservation. The run includes expected source semantic-item and extension counts; they are compared with the recorded ledgers when the inventory is declared complete. Concrete adapters must enumerate their source extension inventory and bind the source input digest. Opaque extension payloads remain uninterpreted by this conformance kernel.

Equality of caller-supplied payload digests is a contract check only. A concrete adapter or independent verifier must calculate and verify those digests over the appropriate source bytes.

## 5. Semantic round-trip comparator

The comparator compares adapter-normalized semantic atoms, not serialized file bytes. Each atom binds namespace, source object reference, semantic path, lexical value and optional unit. Adapters must normalize equivalent XML/RDF/JSON serializations to the same atom form without erasing meaningful distinctions.

The comparator reports missing and unexpected atoms separately. Duplicate atoms are rejected rather than silently deduplicated. Comparison is limited to the in-scope atom set supplied by the adapter; scope selection and canonicalization therefore need independent review and frozen fixtures.

A semantic match demonstrates equality of the supplied normalized in-scope inventory only. It is not a standards-conformity claim.

## 6. First profile targets

### Reference designation
Start with IEC 81346-1 and ISO 81346-10:2022 as the published baseline. The ISO catalogue currently lists ISO/WD 81346-10 edition 2 as under development and intended to replace the 2022 publication. It must be a separate WorkingDraft profile, not silently substituted for the published 2022 edition.

The mapping must preserve original designations and structure. It must not mint official-looking designations without the owner's actual 81346 rules and policy.

### CIM distribution
Start with a narrow frozen subset of IEC 61970-301:2020 + AMD1:2022 and the relevant IEC 61968 distribution profile/edition. Do not treat all CIM classes as household wiring semantics. Preserve mRID, source namespace, profile version, terminals/connectivity semantics, units and extensions.

### IEC 61850 SCL
Limit the initial scope to IEC 61850-6:2009 + AMD1:2018 + AMD2:2024 consolidated publication and its IED/substation configuration exchange domain. Do not generalize SCL into an entire building-installation ontology.

### Cybersecurity and storage
Map exact applicable parts individually. IEC 62351 is a multi-part series; IEC 62351-7:2025 is a distinct network/system-management data-object model edition. Storage profiles should assess relevant IEC 62933 parts, notably IEC 62933-3-1:2025 for planning/performance and IEC 62933-5-1:2024 for safety considerations.

For low-voltage installation principles, include IEC 60364-1:2025 in the dated standards register. Its 2025 edition addresses safety services and standby supplies, energy efficiency, prosumer installations, and periodic verification of protective measures. It is a normative installation standard, not a data interchange format; its obligations belong in qualified jurisdiction/rule profiles, not guessed CIM mappings.

### South Africa
A national applicability profile is separate from IEC mapping success. The current SANS register, SANS 10142-1 edition/amendments, Electrical Installation Regulations, utility requirements and registered-practitioner responsibilities require authoritative confirmation. Public sources identify SANS 10142-1:2024 Ed 3.2. The Department of Employment and Labour notice dated 2025-10-22 says owners remain responsible and CoCs require inspection/testing by competent registered electrical contractors. This programme does not automate or issue CoCs.

## 7. Frozen adapter benchmark

For each standard/profile/adapter version:
1. Freeze source bytes, digest, scope and source edition.
2. Freeze expected source semantic atoms and source identity set.
3. Import to Power Fabric IR and populate the mapping ledger.
4. Export to the chosen target format.
5. Re-import the exported artifact using a separately versioned path.
6. Compare normalized in-scope semantic atoms.
7. Compare source identities, relationships, units, namespaces, extensions and qualifiers.
8. Report missing/unexpected atoms, explicit losses, ambiguous mappings, unsupported source content and reconciliation effort.

Targets:
- 100% source inventory accounting or explicit IncompleteSourceInventory.
- Zero silent drop of in-scope identity, property, relation, unit, namespace or extension.
- Zero lossless status when any Unsupported, Ambiguous or LossExplicit record exists.
- 100% agreement with frozen semantic atom fixtures.
- Independent reproduction of mapping status and semantic round-trip report.

No performance or superiority claim until concrete adapters are compared against established tools and source files under matched tasks.

## 8. Industry boundary

CIM, IEC 61850 SCL and OpenFMB already provide established utility/IED semantics and interoperability architecture. NIST's smart-grid testing programme emphasizes standards-based interoperability test programmes and certification frameworks. Power Fabric should not relabel these as inventions. Its proposed differentiator is a versioned, evidence-bound semantic transformation ledger linked to human-facing projections, asset lifecycle, and change invalidation.

ETAP already markets broad electrical digital-twin engineering and operational analysis. SPAN documents circuit-level control, backup priorities, local on-premise access, and warns that software OFF is not an appropriate service isolation position. Power Fabric must compete on measured semantic preservation, evidence completeness, change-impact accuracy, and comprehension—not claim to invent monitoring or digital twins.

## 9. Claim ceiling

This contract is a mapping/benchmark layer. It is not a standards conformity assessment, certified adapter, utility acceptance, electrical design review, installation inspection, protection study, CoC, or permission to energize. A clean round trip covers only the explicitly supplied in-scope semantic inventory.

## 10. Public research references checked 2026-11-14

- ISO 81346-10:2022 published edition: https://www.iso.org/standard/75471.html
- ISO/WD 81346-10 edition 2 lifecycle: https://www.iso.org/standard/93711.html
- IEC 61850-6 consolidated SCL edition: https://webstore.iec.ch/en/publication/103863
- IEC 61970-301 CIM base: https://webstore.iec.ch/en/publication/62698
- IEC 61968-11 distribution extensions: https://webstore.iec.ch/en/publication/6199
- IEC 62351-7:2025: https://webstore.iec.ch/en/publication/76108
- IEC 62933-3-1:2025: https://webstore.iec.ch/en/publication/75427
- IEC 62933-5-1:2024: https://webstore.iec.ch/en/publication/72239
- IEC 60364-1:2025: https://webstore.iec.ch/en/publication/63699
- OpenFMB reference architecture: https://openfmb.openenergysolutions.com/docs/openfmb/intro/
- NIST Smart Grid Testing and Certification: https://www.nist.gov/programs-projects/smart-grid-testing-and-certification
- ETAP digital twin: https://etap.com/products/etap-digital-twin
- SPAN on-premise: https://support.span.io/hc/en-us/articles/38584072525207-SPAN-Home-On-Premise
- SPAN app safety guidance: https://support.span.io/hc/en-us/articles/4411214235159-Navigating-the-SPAN-Home-App
- South African SANS 10142-1:2024 Ed 3.2 notice: https://www.gov.za/sites/default/files/gcis_document/202408/51050gen2686.pdf
- South African CoC/registered-contractor notice, 2025-10-22: https://www.gov.za/news/media-statements/employment-and-labour-electrical-installation-laws-and-certificate-compliance

These sources establish public scope and publication metadata, not a complete normative applicability determination. Edition, amendments, national adoption, licensing, and installation scope must be checked before implementing normative rules.
