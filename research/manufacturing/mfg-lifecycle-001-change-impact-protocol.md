# MFG-LIFECYCLE-001 — Manufacturing twin change-impact and requalification protocol

**Work item:** [MFG-LIFECYCLE-001 #3164](https://github.com/Luminous-Dynamics/symtropy/issues/3164).  
**Implementation:** Rust reference and synthetic fixture are developed on `feat/manufacturing-lifecycle-change-impact-20261031`.  
**Parent:** [MFG-TWIN-VVUQ-001 #3121](https://github.com/Luminous-Dynamics/symtropy/issues/3121).

## 1. Research question

A manufacturing digital twin's qualification is conditional on exact model code/configuration, source-data identity, sensor/instrument and calibration, material lot, machine/route/process configuration, mappings, environmental envelope and context of use.

A revision change can make downstream evidence or claims stale without changing the shape of the dependency graph. A one-time commissioning test cannot establish indefinite validity.

The target is a deterministic, independently checkable answer to:

> Given an exact revision change, which evidence objects and engineering claims are affected, in which scopes and validity intervals, and through what dependency paths?

The result is a *requalification set*, not a new qualified status.

## 2. Research context and differentiation

The timing is useful:
- ISO 23247-6:2026 standardises integrated, unified and federated manufacturing twin composition: https://www.iso.org/standard/87426.html
- ISO/AWI 23247-8 was approved and registered on 23 October 2026; it is under development and addresses VVUQ across requirements, development, commissioning, operation, maintenance and decommissioning: https://committee.iso.org/standard/94858.html
- NISTIR 8620 (21 July 2026) reports persistent manufacturing digital-twin barriers in interoperability, VVUQ, cybersecurity and workforce readiness: https://www.nist.gov/publications/digital-twins-workshops-summary-report
- Lugaresi, Dhore and Matta (Procedia CIRP, 2026) describe online validation/calibration for a manufacturing discrete-event-simulation twin using event sequences and performance indicators: https://doi.org/10.1016/j.procir.2026.03.284
- A 2026 *Journal of Systems and Software* article presents a graph-based digital-twin model consistency management framework: https://doi.org/10.1016/j.jss.2025.112750
- 2026 research on semantic drift identifies limitations in digital-twin frameworks' semantic drift handling: https://doi.org/10.1016/j.future.2025.108240

These are complementary comparison lanes, not interchangeable methods. Online drift detection estimates when a model diverges; semantic consistency detects mismatches; graph-based consistency checks relationships. The Symtropy slice focuses on the downstream **claim-impact and requalification semantics** after a declared, revision-bound change. It must compose with drift detection and VVUQ owners rather than replace them.

## 3. Core model

A versioned directed dependency graph contains:
- Items: model, configuration, schema, interface mapping, calibration, material, process, environment, simulation, measurement, uncertainty budget and derived evidence.
- Claims: physics, measurement, conformity, release, intervention and causal-outcome claims.
- Dependencies: directed edges bound to the exact upstream revision.
- Change records: old/new revision and digest, change class, semantic-change declaration, scope, effective interval and receipt reference.

The tool accepts synthetic change receipts for the first reference implementation. It does not authenticate the issuer or verify that a physical change took place.

## 4. Impact semantics

### 4.1 Revision binding

Every dependency names the exact upstream revision it used. A dependency referencing an unknown or different revision is unresolved; it is never treated as unaffected.

### 4.2 Scope

- `GLOBAL` changes affect every downstream scope.
- A local change affects its matching scope.
- A GLOBAL item cannot be changed under a local scope declaration.
- GLOBAL downstream claims/items are conservatively affected by local changes that feed them.
- An explicit dependency path is still required; there are no guessed relationships.

### 4.3 Time

Claims and items use inclusive UTC validity intervals in this v0.1 profile. If a claim validity interval ended before a change became effective, preserve the historical claim and its old receipt. If the validity intervals overlap, the claim enters the requalification set.

The reference tool currently uses closed interval comparison. A future profile may adopt explicit half-open intervals if every producer/consumer agrees; mixed conventions are prohibited.

### 4.4 Change class

- SUBSTANTIVE: arbitrary semantic model/component change.
- SCHEMA: schema or interface-mapping revision.
- CALIBRATION: calibration item only.
- MATERIAL: material or material-lot item.
- ENVIRONMENT: environment/boundary-condition item.
- PROCESS: process/process-parameter/route item.
- METADATA_ONLY: the sole no-op is a new metadata revision with identical content digest and explicit `semantic_change=FALSE`.

A semantic change cannot be labelled metadata-only. Identical hashes do not prove that the engineering meaning is unchanged. For a semantic change, new revision and digest are required.

### 4.5 Output

For each change, report:
- direct changed item;
- affected downstream evidence items;
- every impacted claim;
- current claim status;
- one deterministic shortest witness path;
- count of dependency paths (saturating at u64);
- deduplicated union across all change records.

A future implementation should add all path witnesses where tractable and produce a compact Merkle-bound impact receipt. v0.1 keeps the shortest witness plus path count to avoid exponential report growth.

## 5. Benchmark corpus

Frozen synthetic scenarios:
1. A global physics-model change used by two lines.
2. A local calibration change affecting measurement, uncertainty and conformity on one line only.
3. A future-effective change preserving a historical claim whose validity expired beforehand.
4. A long-lived claim whose validity overlaps the change.
5. An explicit metadata-only no-op.
6. Identical digest with a semantic-change declaration incorrectly labelled metadata-only.
7. Old revision/digest mismatch.
8. Unknown item and unknown dependency.
9. Dependency cycle.
10. Wrong upstream revision on a dependency edge.
11. Change-class / target-kind incompatibility.
12. Item/change validity non-overlap.
13. Two distinct paths to the same claim.
14. Multiple changes with deduplicated impact union.
15. A 10,000-item dependency chain.

Keep positive examples. A reject-all graph checker is not acceptable.

## 6. Metrics and targets

Report each separately:
- affected-claim recall against independently authored known-answer oracle;
- unaffected-claim precision;
- scoped false-invalidation rate;
- historical retroactive-invalidation rate;
- under-invalidation rate for known downstream claims;
- evidence/path precision and completeness;
- ambiguous revision / scope rejection rate;
- dependency-cycle detection rate;
- exact requalification-set agreement;
- duplicate-claim inflation after multi-change union;
- traversal runtime and peak memory at 100 / 1,000 / 10,000 / 100,000 items;
- runtime versus number of changes, graph density and shared-dependency fan-out.

Target gates:
- 100% recall for known affected claims in the frozen corpus.
- Zero unrelated-scope claims invalidated by a scoped change.
- Zero historical claims rewritten by future-effective changes.
- 100% fail-closed behavior for cycles, unknown dependency IDs, invalid revision binding and unsupported scope semantics.
- Deterministic byte-identical reports for identical admitted inputs.
- Independent implementation agrees exactly on the impacted claim set and witness-path validity.

No combined score may hide an under-invalidation or false historical invalidation.

## 7. Baselines

Compare against:
1. **No invalidation:** intentionally unsafe baseline; it leaves every claim qualified.
2. **Invalidate all:** intentionally safe but operationally unusable baseline; it requalifies all claims regardless of scope/time.
3. **Naive descendants-only:** traverses graph reachability but ignores exact revisions, scope and validity intervals.
4. **Scope-only graph traversal:** adds scope but ignores time.
5. **Time-only graph traversal:** adds time but ignores scope.
6. **Reference algorithm:** exact revision + scope + time + deterministic DAG traversal.
7. **Manual review:** prospective industrial baseline; compare reviewer effort and missed impacted claims, not only computer runtime.

The reference must beat the naive baselines on the frozen corpus by eliminating their known false positives and false negatives while retaining 100% recall. This is a correctness comparison, not a performance superiority claim against commercial products.

## 8. Industrial integration

The engine consumes immutable item/claim/dependency records and change receipts from existing owners:
- model VVUQ receipts;
- MSA/calibration receipts;
- MTConnect/QIF mapping and schema admission;
- product/material/route genealogy;
- conformity and intervention receipts.

It must not write to a controller, trigger recalibration, authorize product release, or automatically mark claims qualified. The requalification orchestrator and authorities remain separate.

## 9. Claim ceiling

The v0.1 reference establishes deterministic declared-change impact on a synthetic dependency graph. It does not detect physical drift, authenticate change records, prove the changed artifact bytes, decide whether a new model is valid, perform requalification or establish production impact. Every real claim needs exact admitted revision and evidence from its existing owner.
