# EPCIS Transformation Group → Material Balance Receipt Link v0.1

Issue: https://github.com/Luminous-Dynamics/symtropy/issues/6741

Related independent owners:
- Symtropy MATERIAL-ACCOUNTED-ROUTES-001A #5879: https://github.com/Luminous-Dynamics/symtropy/issues/5879
- Symtropy OCEAN-MULTISTREAM-BALANCE-001 #6440: https://github.com/Luminous-Dynamics/symtropy/issues/6440
- EPCIS TransformationEvent semantic work: https://github.com/Luminous-Dynamics/symtropy/pull/6778
- EPCIS composition transactions and parent-scoped DELETE_ALL: https://github.com/Luminous-Dynamics/symtropy/pull/6857

## Ownership boundary

This adapter does not calculate material balance, yield, process conversion, residue, mass loss, water/energy conservation, uncertainty propagation or recovered-material grade. Those calculations and their physical/process boundaries belong to their canonical owner and its independent oracles.

The adapter consumes the already-normalized, effective EPCIS TransformationEvent source references and one owner-native MASS_BALANCE receipt. Its responsibility is exact identity/scope/reference binding and conservative status composition only.

The current MATERIAL-ACCOUNTED-ROUTES-001A Stage 0 contract in PR #5912 defines a Receipt with:
- case_id;
- manifest_ref;
- gate;
- receipt_ref;
- subject_ref;
- scope_ref;
- source_digest;
- status.

For the MassBalance gate, the owner evaluator expects subject_ref == process_ref, scope_ref == input_lot_set_ref, one receipt, and the status vocabulary below. The owner evaluator also checks the SHA-256 string shape, but its current digest_check is explicitly FORMAT_ONLY_NOT_AUTHENTICATED when complete: this adapter does not upgrade that to cryptographic authentication.

The handoff type carries an external expected receipt source digest from the selected case evidence bundle and compares it exactly with the owner receipt. It does not retrieve those bytes or prove the digest's signature/issuer.

## Transformation-group binding

The adapter's TransformationGroupEvidence is a DTO for the caller to build from the normalized EPCIS TransformationEvent grouping result. It contains:
- exact group_ref;
- optional exact transformation_id;
- effective source-event revisions selected at one declared publication/as-of cutoff;
- each source namespace, event ID, source-local revision and immutable source-payload digest.

For a shared transformation group, the exact transformation_id must match the owner case's transformation_id. For an event-local TransformationEvent with no transformationID, exactly one effective source event is allowed. A no-ID event cannot be used to silently assemble a multi-event group.

The adapter rejects empty source groups, invalid/padded source identities, revision zero, malformed source digests, duplicate source event revisions and conflicting revisions for the same (source_id, source_event_id) identity.

These checks validate the shape and internal identity of the supplied effective group. They do not prove that the capture stream is complete, that the process is closed, or that every physical input/output event was reported. The upstream observation-completeness programme remains a separate necessary qualification.

That upstream work is already tracked and has an executable synthetic benchmark: [ORBITAL-RELIABILITY-OBSERVATION-COMPLETENESS-001 #6659](https://github.com/Luminous-Dynamics/symtropy/issues/6659), [event-capture observability PR #6676](https://github.com/Luminous-Dynamics/symtropy/pull/6676), [observation-process identifiability PR #6731](https://github.com/Luminous-Dynamics/symtropy/pull/6731), and [externally anchored POD trend PR #6751](https://github.com/Luminous-Dynamics/symtropy/pull/6751). PR #6676's current exact head and both recorded workflows are green; this adapter does not recreate its independent event reference/counter/loss qualification. Its receipt should be connected by a future full owner-case composition, not inferred from an EPCIS group's own event count.

## Exact owner binding

A MaterialBalanceCaseBinding names the owner case, manifest, transformation-group reference, transformationID, process, input-lot set, output scope and expected owner receipt source digest.

The one supplied owner receipt must satisfy:
1. gate == MASS_BALANCE;
2. exact case_id and manifest_ref;
3. exact subject_ref == process_ref;
4. exact scope_ref == input_lot_set_ref;
5. a well-formed source digest exactly matching the digest pinned by the case binding;
6. exactly one MassBalance receipt for this case.

Missing receipt produces Indeterminate. Multiple receipts, a wrong gate, a mismatched case/manifest/process/lot scope or an incorrect/malformed digest produce Blocked. No nearest timestamp, fuzzy identity, label match or quantity inference is performed.

The adapter does not check the other seven route gates (ROUTE, UTILITY, MATERIAL_GRADE, CIRCULAR_ALLOCATION, INSPECTION, PROVENANCE, CLAIM_CEILING). This result is not AcceptedSynthetic for the route as a whole.

## Owner status mapping

The mapping mirrors the current MATERIAL-ACCOUNTED-ROUTES-001A gate_result(Gate::MassBalance, status) contract:

| Owner status | Adapter result | Meaning |
|---|---|---|
| EXACT_BALANCED | MassBalanceGatePass | Owner's exact balance gate passes for the declared process/scope |
| POSSIBLE_WITH_UNCERTAINTY | Indeterminate | Not promoted to exact balance |
| UNKNOWN / MISSING | Indeterminate | Missing/unknown stays unknown |
| EXACT_UNBALANCED | Blocked | Owner reports exact imbalance |
| IMPOSSIBLE_WITHIN_BOUNDS | Blocked | Owner reports no feasible balance in declared bounds |
| INVALID | Blocked | Owner reports invalid evidence/model state |
| Any unsupported status | Blocked | Unknown vocabulary does not default to pass |

Even MassBalanceGatePass means only that one owner gate's exact receipt is bound. The owner must still compose the full case disposition, including material grade, circular allocation, inspection, provenance and claim ceiling. A gate pass is not a product release, route acceptance or physical claim.

## Explicit non-equivalences

- EPCIS TransformationEvent is a many-to-many process-lineage relation; it is not a mass-balance equation.
- A shared transformationID establishes possible contribution scope; it does not provide an input-to-output allocation matrix or prove that a process group is closed.
- QuantityElement entries remain class-level assertions. The adapter never expands class quantities into instance IDs, adds incompatible UOMs, defaults unknown to zero, or infers mass from counts.
- AggregationEvent containment and DELETE_ALL remain a different relation from process consumption/production.
- A valid owner receipt does not establish observation completeness, calibration traceability, physical presence, process boundary completeness or data-source independence.
- A well-formed and matching digest reference does not authenticate the artifact bytes or issuer.
- A passing software receipt does not establish a measured physical mass balance.

## Adversarial test corpus

The Rust module includes frozen tests for:
- exact balanced owner gate receipt;
- POSSIBLE_WITH_UNCERTAINTY abstention;
- exact unbalanced / impossible / invalid blocking;
- missing owner receipt;
- multiple owner receipts;
- wrong gate/case/manifest/process/lot scope;
- transformation group and transformationID mismatch;
- missing or malformed receipt digest;
- expected digest mismatch;
- malformed source-payload digest;
- zero source revision;
- event-local single-event success vs multi-event local group rejection;
- duplicate and conflicting source event revisions;
- unknown/missing status abstention;
- unsupported owner status blocking;
- claim ceiling retained on every decision.

## Benchmarks and gates

P0 synthetic binding conformance:
- 100% exact agreement with the frozen owner status/binding matrix.
- Zero MassBalanceGatePass decisions for missing, duplicate, mismatched, malformed, unbalanced, impossible or unsupported receipts.
- 100% exact case/manifest/process/input-lot/group reference matching.
- Zero fuzzy identity, nearest-time, inferred quantity or default-zero paths.
- Deterministic output for identical group/case/receipt inputs.
- 100% of returned results retain the MASS_BALANCE_GATE_ONLY_NO_PHYSICAL_OR_RELEASE_AUTHORITY claim ceiling.

P1 owner integration:
- Consume the exact pinned MATERIAL-ACCOUNTED-ROUTES-001A evaluator and schema revision once its API contract is stable.
- Compare the adapter's gate-only disposition with the owner's independent per-case MASS_BALANCE evaluation across all frozen positive/negative cases.
- Preserve the owner's complete per-case disposition and all other gate reasons; this module must not reconstruct the overall route result.
- Record exact owner repo/branch/commit/schema/fixture identities in every integration run.

P2 physical validation:
- Use owner-qualified measured inputs, lot identity, process boundary, calibrated measurements, uncertainty/covariance, coproducts/residue/waste, storage changes and complete inlet/outlet observations.
- Use the OCEAN-MULTISTREAM-BALANCE-001 contract for multi-stream physical control volumes where applicable.
- Report empirical residuals and uncertainty coverage only from the owner evaluator and admitted field evidence. This adapter produces no physical residual or accuracy metric.

## Competitive position

GS1 EPCIS/CBV 2.0.1 publishes normative JSON Schema, SHACL, XSD, JSON-LD context, ontology and REST OpenAPI artefacts. Conformance to those artefacts can establish data-shape and interface consistency; it cannot by itself establish the physical truth, completeness or mass conservation of the represented process.

Symtropy's contribution is a layered assurance boundary:
1. GS1 event-family semantics and identity preservation;
2. source-revision replay and atomic composition;
3. clock/capture evidence applicability and chronology qualification;
4. a bound handoff to independent process/material conservation receipts;
5. full route-gate composition remaining with the independent owner.

The benchmark should compete on false-accept prevention, exact-scope binding, transparent abstention, replay determinism and evidence coverage—not by claiming to replace established EPCIS repositories, manufacturing execution systems, process simulators or metrology laboratories.

## References

- GS1 EPCIS/CBV 2.0.1 normative artefacts: https://ref.gs1.org/standards/epcis/2.0.1/artefacts
- GS1 TransformationEvent model: https://ref.gs1.org/epcis/TransformationEvent
- GS1 Global Traceability Standard and CTE/KDE framework: https://support.gs1.org/support/solutions/articles/43000734535-what-is-the-gs1-traceability-standard-
- NIST metrological traceability policy: https://www.nist.gov/metrology/metrological-traceability
- MATERIAL-ACCOUNTED-ROUTES-001A: https://github.com/Luminous-Dynamics/symtropy/issues/5879
- OCEAN-MULTISTREAM-BALANCE-001: https://github.com/Luminous-Dynamics/symtropy/issues/6440

## Claim ceiling

A MassBalanceGatePass is one correctly bound owner gate receipt. It does not prove physical consumption/production, all-event capture, process completeness, material quality, product conformity, safety, environmental compliance, route acceptance, custody or release. No live inventory, WMS, TOS, payment, customs, dispatch, production or physical process mutation is performed.
