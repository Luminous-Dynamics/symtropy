# SCAN-BIM-IFC-DELIVERY-001 — OpenBIM delivery qualification v0.1

**Status:** protocol/schema starter; implementation and independent tool execution not yet claimed  
**Parent:** [SCAN-BIM-ADMISSION-001 #4086](https://github.com/Luminous-Dynamics/symtropy/issues/4086)  
**Tracking:** [#4271](https://github.com/Luminous-Dynamics/symtropy/issues/4271)

## 1. Research question

Does a Scan-to-BIM candidate produce an editable and interoperable building model whose required geometry, topology, identities, placements, units and information survive independent validation and a controlled import/edit/export round trip?

A file that exists, has a matching SHA-256, parses, or renders plausibly is not thereby an interoperable building model. Those are distinct claims with different evidence.

This workstream composes existing owners. It is not a new IFC parser, IDS checker, geometry kernel, semantic diff engine, survey engine, identity registry or lifecycle event ledger.

## 2. Claim ladder

The delivery report must keep the following claims separate:

1. **Artifact identity:** exact IFC and graph bytes match the Scan-to-BIM replay receipt.
2. **Syntax:** the selected STEP/IFC representation can be parsed completely by the named parser profile.
3. **Schema:** the named IFC schema validator's declared rule coverage and results are available.
4. **Information requirements:** IDS bytes are valid; the supported IDS semantics are known; IFC-versus-IDS checking is independently evidenced.
5. **Identity and graph crosswalk:** IFC entities map to Scan-to-BIM graph elements without collisions, omissions or ambiguous physical identity.
6. **Semantic preservation:** source-to-target and round-trip transformations preserve declared information, identifiers, units and relations.
7. **Geometry/topology preservation:** geometric representations, placements, openings and graph relationships survive within a predeclared exchange-profile tolerance.
8. **Editability:** a controlled edit is possible in a named independent implementation and survives re-export/re-import with the intended change and no unintended losses.

A required gate that is FAIL blocks qualification. A required gate that is UNRESOLVED or NOT_RUN leaves the delivery UNQUALIFIED. Unsupported validator features must not be converted to PASS.

The aggregate decision is mechanically derived from the gate set. The producer does not author an unconstrained success Boolean.

## 3. Existing owners to compose

| Concern | Existing owner | Adapter responsibility |
|---|---|---|
| Candidate/runtime/output identity | Scan-BIM #4188 / #4116 | Bind exact replay, scene and artifact digests |
| Geometry, matching, sampling | #4205, #4225, #4229, #4246 | Bind outputs; do not recompute |
| Survey frame/covariance | #1607 / #4228 | Require matching frame and uncertainty receipts |
| Building reconstruction/admission | #4086 | Consume its element graph and proposed model delta |
| IFC/IDS gate semantics | BUILT-INTEROP #2056 | Preserve gate identity, scope and status |
| IFC/IDS ingestion | #2010 / #2013 | Consume only the implementation's declared supported subset |
| IFC schema validation | #2059 / #2060 | Consume coverage-aware validator result |
| IDS XSD validation | #2079 | Consume offline, pinned XSD result |
| IDS corpus inventory | #2020 / #2036 | Respect rights and do not copy restricted fixtures |
| Source/target semantic comparison | #2007 / #2073 / #2117 / #2143 | Bind the exact source/target pair and result |
| Typed gate composition | #2061 | Reuse the existing evidence-report contract |
| Physical component identity | #3331 | Do not equate IFC GlobalId with proof of physical identity |

Before integrating a producer, inspect its current exact-head state, supported profile, license and test evidence. An open issue, draft PR or historical green run does not prove that a tool is currently available or that its full feature set is implemented.

## 4. Standards and exchange profiles

The official buildingSMART IFC release is IFC 4.3.2.0 (IFC4.3 ADD2), published as ISO 16739-1:2024. It is an appropriate modern target profile where the producing application and downstream consumers support it. Historical datasets and existing project exchanges may use earlier IFC schemas; the receipt must name the actual schema and exchange view rather than silently upgrading or mislabelling the file.

IDS 1.0 can express machine-checkable information requirements for entities, classifications, properties and values. IDS does not validate geometry. Geometry, topology, coordinate placement and physical accuracy therefore remain separate gates.

BCF may carry review issues and viewpoints, but a BCF topic is not a substitute for the IFC artifact, a semantic-preservation result or an independently validated model delta.

Primary references:
- buildingSMART IFC: https://www.buildingsmart.org/standards/bsi-standards/industry-foundation-classes/
- buildingSMART IDS: https://www.buildingsmart.org/standards/bsi-standards/information-delivery-specification-ids/
- buildingSMART BCF: https://info.buildingsmart.org/standards/bsi-standards/bim-collaboration-format/
- BUILT-INTEROP validation ladder: https://github.com/Luminous-Dynamics/symtropy/issues/2056

## 5. Receipt contents

The JSON Schema in schema/delivery-receipt.schema.json defines the initial wire shape. It binds:
- replay run and receipt digest;
- candidate source commit;
- building group, scene and source model revision;
- exact IFC and element-graph paths, lengths and SHA-256 digests;
- schema, exchange view, units, frame, axis order, handedness and placement convention;
- optional IDS artifact identity and requirement-set revision;
- IFC GlobalId ↔ element-graph crosswalk rows;
- one typed result for each required independent gate;
- producer identity, configuration digest, evidence artifacts and diagnostics;
- a derived decision and claim ceiling.

JSON Schema validates structure. A Rust verifier must additionally enforce cross-field invariants which JSON Schema cannot fully express, including:
- exactly one result for each required gate ID;
- no duplicate gate identities;
- all gate evidence references bind to the same replay, scene, model revision and IFC digest;
- every required graph element and IFC element is covered by the crosswalk or explicitly unresolved;
- one-to-one mapping constraints and collision reporting;
- PASS/FAIL require a named producer and non-empty evidence;
- NOT_RUN has no fabricated execution evidence;
- QUALIFIED is impossible unless all required gates pass;
- unsupported or partial validator coverage cannot be silently promoted.

## 6. P0 synthetic cases

The initial fixture uses synthetic metadata and placeholder IFC bytes. It does not contain a valid IFC model and all real IFC/IDS/geometry/round-trip gates remain NOT_RUN.

Required oracle cases:
1. Correct artifact binding with only integrity PASS remains UNQUALIFIED.
2. One missing required gate remains UNQUALIFIED.
3. One UNRESOLVED required gate remains UNQUALIFIED.
4. One FAIL required gate is BLOCKED.
5. A producer report bound to a different IFC digest is rejected.
6. A duplicate gate identity is rejected, even when the two rows have different statuses.
7. A duplicate GlobalId or graph ID is surfaced as a crosswalk collision.
8. A missing element on either side is not counted as preserved.
9. A stale scene/model/replay revision invalidates the affected gate.
10. Unit/frame/axis/handedness changes require an explicit, digest-bound transform.
11. Geometry beyond the declared profile tolerance fails the geometry gate even when IFC and IDS pass.
12. A missing hard IDS property blocks its declared downstream use.
13. An unsupported IDS facet is UNRESOLVED, not PASS.
14. STEP entity ordering changes alone may preserve semantic identity after canonicalization.
15. An import/export cycle that drops an opening, relationship, placement or property fails the relevant preservation gate.
16. A parser-only PASS never implies editability.
17. A visual preview never implies semantic or geometric equivalence.
18. The same inputs and evidence produce a deterministic normalized receipt.

## 7. Benchmark protocol

Keep all results separate. Do not build a weighted score that allows a strong parser score to compensate for lost geometry or a missing hard requirement.

| Track | Required measurements |
|---|---|
| Syntax/schema | Parse completeness; rule coverage; errors; unsupported and evaluation-error counts |
| Information | IDS requirement coverage; preserved/missing/changed/unsupported/not-evaluated counts |
| Identity | Crosswalk precision/recall; collision/orphan/missing element counts |
| Geometry | Bidirectional surface deviations; p50/p95/p99; dimensions; opening-boundary displacement |
| Topology | Adjacency, containment, connectivity, opening and placement relation preservation |
| Round trip | Retained entities/properties/relations/placements/geometry after import, edit, export and re-import |
| Provenance | Exact replay, scene, source revision, artifact, tool, configuration and validator digests |
| Operations | Import/export failure rate, manual repair minutes, review time and rework |

### Proposed P0 targets (not results)

- 100% agreement with the independent synthetic known-answer oracle.
- Zero QUALIFIED results if any mandatory gate is FAIL, UNRESOLVED or NOT_RUN.
- 100% detection of seeded hard-property loss, digest mismatch, stale revision, unit/frame drift and identity collision.
- 100% stable crosswalk preservation in the synthetic no-op round trip.
- Zero unsupported checks silently counted as PASS.
- Byte-stable normalized receipts for identical inputs and tool evidence.

### P1 targets (after independent implementations are available)

- At least two independently implemented IFC consumer/producer paths, with exact versions and configurations pinned.
- Same fixture and requirements across tools; no different-input comparisons.
- Report source → import → controlled edit → export → re-import losses by element and property.
- Publish geometry and semantic loss separately; report tool limitations and unsupported representations.
- Match or exceed manual expert workflow on editability/rework only under a matched task, model complexity, time budget and review protocol.

Numeric geometry tolerances are profile- and requirement-specific. No global tolerance is invented. No vendor superiority claim is permitted from a feature checklist or synthetic corpus.

## 8. First Rust contract auditor

The initial implementation is in `tools/scan-bim-replay-contract/src/delivery.rs`, exported through the existing replay-contract crate so it reuses the pinned serde/serde_json toolchain and lockfile.

The auditor:
- strictly deserializes the v0.1 receipt with unknown fields denied;
- requires the exact nine gate identities, exactly once and in canonical order;
- binds every gate to the same replay receipt digest and scene;
- verifies that IFC, element-graph and IDS subject digests match the receipt's declared artifact identities;
- requires producer/configuration/evidence/subject bindings for PASS and FAIL;
- requires NOT_RUN rows to have no producer, subjects or execution evidence;
- checks supported IFC ↔ graph crosswalk mappings for one-to-one uniqueness and evidence;
- derives QUALIFIED / BLOCKED / UNQUALIFIED itself and rejects a contradictory producer decision;
- prevents qualification while any identity mapping is ambiguous/unresolved or IDS requirements are absent;
- rejects path traversal and malformed lowercase SHA-256 / commit identities.

The unit corpus includes the five committed JSON known-answer cases plus Rust-side checks for all-pass qualification, unresolved identity, missing IDS inputs, path traversal and unknown fields.

**Important boundary:** this is a receipt-semantics auditor, not a full JSON Schema 2020-12 implementation. It does not hash referenced files, parse STEP/IFC, execute IDS, validate geometric/topological equivalence, or perform an import/edit/export round trip. Those remain separate producers/gates. The workflow is triggered by the Rust module changes; its exact-head result must be recorded separately and is not inferred from a queued run.

## 9. Claim ceiling

P0 establishes receipt-shape and decision semantics on synthetic data only. P1 can establish interoperability conformance for named tool versions, schemas, exchange profiles and fixtures. Neither establishes physical survey accuracy, building-code compliance, structural adequacy, professional sign-off, safe construction, or authority to mutate an authoritative asset record.

The acceptance boundary remains with #4086 and the relevant professional/asset owner.