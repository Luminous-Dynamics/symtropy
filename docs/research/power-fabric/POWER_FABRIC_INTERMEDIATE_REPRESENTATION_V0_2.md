# Power Fabric Intermediate Representation v0.2

**Status:** executable research contract and validated wire-ingestion boundary; not a normative electrical standard, electrical solver, installation approval, or certification  
**Date:** 2026-11-14  
**Owner issue:** https://github.com/Luminous-Dynamics/symtropy/issues/5206  
**Parent:** https://github.com/Luminous-Dynamics/symtropy/pull/5188

## 1. Why an intermediate representation

The module contract describes typed module interfaces. The Electrical Service Graph describes declared topology and evidence. The next layer binds them into a versioned artifact that can be imported, transformed, analyzed, projected to different audiences, and independently checked without making any one application or vendor model authoritative.

The Power Fabric IR is a composition and evidence layer, not a competing electrical ontology. Standard-specific models retain their own classes, namespaces, relationships, and scope. Adapters map only semantics that are demonstrably equivalent.

## 2. Core invariants

1. Stable Power Fabric object identity is distinct from manufacturer, serial, CIM mRID, and IEC 81346 reference designation.
2. External identifiers are namespaced and preserved.
3. Design intent, declared data, observed data, and verified data are separate claim domains. They are not an automatic maturity ladder.
4. Value, unknown, unavailable, not applicable, and contradictory are distinct states.
5. Observed and verified claims require an evidence reference. Unknown claims cannot carry an evidence ID.
6. Contradictory claims identify at least two distinct conflicting claim IDs.
7. Relationships are typed by namespace and local semantic name, with explicit direction.
8. Dangling relationship endpoints and extension owners are rejected.
9. Opaque extension payloads are retained as UTF-8 text and are not interpreted by the IR core.
10. Untrusted wire DTOs deserialize separately and must pass validated constructors before becoming trusted IR objects.
11. Maps iterate in stable key order. This establishes deterministic structural iteration, not RFC 8785 canonical JSON, a cryptographic digest, or reproducible compiler output by itself.
12. Every compilation result must bind exact input document/revision IDs, adapter/compiler versions, profiles, assumptions, and output digests.

## 3. Version and migration policy

The first implementation supports schema major 0 and minor versions up to 2. Unsupported major/minor versions fail closed. An adapter must not silently reinterpret an older schema with newer semantics.

A migration is a separate transformation that emits source and destination schema versions, input identity, mapping/adapter version, preserved and changed object identities, unresolved and explicitly lossy fields, diagnostics, and output revision identity. Historical source artifacts are immutable; migration creates a new revision linked to its parent.

## 4. Identity model

Each object has a stable Power Fabric ID, a namespace-to-external-identifier map, a namespaced object kind, and zero or more claims. The initial implementation intentionally avoids requiring a global UUID format. Sites and adapters can use stable opaque identifiers while preserving source identifiers. A later profile may define URI/UUID conventions, but only with explicit migration rules.

## 5. Claim model

### Domains

| Domain | Meaning | Example |
|---|---|---|
| DesignIntent | Requirement or design target | Intended backup service duration |
| Declared | Source says a property/capability exists | Nameplate voltage |
| Observed | Instrument, inspection, or field observation | Measured voltage at a documented boundary |
| Verified | A qualified review/test supports a specific claim | Independently reviewed configuration |

A newer observation does not delete the manufacturer's declared rating. A verified claim does not retroactively change design intent. Consumers select and reconcile claims according to an explicit policy.

### States

| State | Meaning | Must not be interpreted as |
|---|---|---|
| Value | A typed value is present | Automatically true outside its provenance/scope |
| Unknown | No admissible value is known | Zero, false, safe, or absent |
| Unavailable | A value cannot currently be obtained | Not applicable |
| NotApplicable | The property does not apply under declared scope | Missing data |
| Contradictory | Multiple distinct claim references conflict | A value selected by ordering |

Decimal lexical values are retained (for example, 12.50) and malformed or non-finite decimal forms are rejected. Units remain explicit strings until a separately qualified unit registry/conversion profile is introduced.

## 6. Provenance

Each claim records a non-empty source identity, optional evidence identity, and optional source timestamp. Observed and Verified domains require evidence references. Evidence references are identifiers, not proof of authenticity. A later resolver must check that each identifier resolves to the exact artifact, subject/configuration, method, scope, time, and reviewer applicable to the claim.

The current core retains the timestamp string but does not yet validate full RFC 3339 grammar, trusted clock, signature, revocation, or freshness policy. Those belong in a profile/evidence-validation layer.

## 7. Relationships

A relationship has stable identity, source and target object IDs, namespaced semantic type, explicit direction, claim domain, and provenance. The IR does not infer PowerPath semantics from a relationship label. An adapter maps a source relationship to a Power Fabric relationship type only when a reviewed mapping rule supports that interpretation.

The existing Service Graph remains the topology analysis kernel. The IR composes with it through explicit mappings rather than duplicating BFS or physical-system solver responsibilities.

## 8. Opaque extensions

An extension has its own ID, namespace, media type, owner (document/object/relationship), and opaque UTF-8 payload. The core validates the envelope and reference target but never parses or rewrites payload contents.

This protects vendor-specific or future-standard data against premature normalization. Preservation of a payload string is not proof of complete standards round-trip equivalence: adapters still need to preserve source encoding, namespace declarations, ordering-sensitive structures, signatures, and external references where required.

## 9. Validated wire ingestion

External JSON is first decoded into WireDocument, an untrusted DTO with unknown fields rejected. Call WireDocument::into_validated() to construct trusted IR through the same invariant-enforcing constructors used by native Rust callers.

Do not add Deserialize directly to trusted IR types: that creates a second path around validation. A future adapter can retain unknown vendor data only through explicit extension records; unrecognized top-level fields are rejected rather than silently dropped.

## 10. Standards profile boundary

The IR uses namespaced identifiers rather than copying IEC/CIM/SCL classes into a new universal ontology.

The standards register must pin exact publication status and version:
- ISO 81346-10:2022 remains the published edition; ISO/WD 81346-10 edition 2 is a working draft, not a published standard.
- IEC 61850-6 consolidated publication: IEC 61850-6:2009 + AMD1:2018 + AMD2:2024 (SCL), bounded to IED/substation configuration exchange.
- CIM: IEC 61970-301:2020 + AMD1:2022 for the base CIM, together with the relevant IEC 61968 distribution profile/edition.
- IEC 62351 is a multi-part series whose individual part/edition and applicability must be recorded. A 2026 series catalogue does not make all included parts edition 2026.
- For storage planning and safety, assess relevant IEC 62933 parts rather than inventing an EES schema.
- For low-voltage installation principles, examine IEC 60364 series scope and national adoption; IEC publication does not prove local conformity.
- For DER interconnection, assess applicable IEEE 1547 family profiles where jurisdiction and frequency/system context make them applicable.

South African applicability must be separately established against current SANS publications, applicable legislation, and registered-practitioner responsibilities. Public government material identifies SANS 10142-1:2024 Ed 3.2 in a 2024 standards notice. The Department of Employment and Labour's 2025 notice reiterates CoC and registered-contractor requirements. These are research pointers, not a complete current legal register or conformity assessment.

## 11. Benchmarks and acceptance

### Contract correctness
- Zero duplicate identity acceptance across objects, relationships, claims, and extensions.
- Zero dangling relationship or extension-owner acceptance.
- 100% rejection of observed/verified claims without evidence references.
- 100% rejection of unknown claims carrying evidence IDs.
- 100% preservation of claim-domain separation.
- 100% preservation of extension payload UTF-8 bytes at the model boundary.
- 100% deterministic key iteration for identical input structures.

### Adapter conformance
- Exact identity preservation or explicit source-to-target identity ledger for every mapped object.
- Zero silent loss of in-scope semantics, properties, relationships, namespaces, or extensions.
- All unsupported/ambiguous semantics reported as unresolved.
- Round-trip equivalence tested per standard/profile/version, not by comparing file text alone.
- Every transform pins source standard edition, adapter version, and target profile.

### Compiler quality
- Full and incremental results semantically equivalent for frozen inputs.
- Every changed input invalidates all affected derived claims, and no unrelated claim, measured against a dependency oracle.
- Deterministic diagnostics and stable codes.
- Performance measured at 100, 1,000, 10,000 and 100,000 graph objects with disclosed hardware, input generation, and compiler revision.

### Human factors
- Preregistered comparison against incumbent documentation.
- Measure correctness, time, severity-weighted errors, confidence calibration, and recognition of unknowns.
- Proposed 95% correctness on preregistered non-hazardous tasks is a research goal, not a safety threshold.
- Critical false-safe interpretation target: zero in frozen qualification scenarios, with limitations explicitly reported.

## 12. Claim ceiling

This increment does not compute electrical compatibility, load flow, protection, fault current, arc flash, conductor sizing, grounding, battery runtime, source capacity, physical isolation, live power state, or installation conformity. It does not authenticate evidence, establish trusted time, or issue a Certificate of Compliance. A validated IR document is a structurally valid information artifact, not proof that the represented installation exists or is safe.
