# EPCIS Transformation Group → Material Balance Receipt Link v0.1

Issue: https://github.com/Luminous-Dynamics/symtropy/issues/6741

Related independent owners:
- Symtropy MATERIAL-ACCOUNTED-ROUTES-001A #5879: https://github.com/Luminous-Dynamics/symtropy/issues/5879
- Symtropy OCEAN-MULTISTREAM-BALANCE-001 #6440: https://github.com/Luminous-Dynamics/symtropy/issues/6440
- EPCIS TransformationEvent semantic work: https://github.com/Luminous-Dynamics/symtropy/pull/6778
- EPCIS composition transactions and parent-scoped DELETE_ALL: https://github.com/Luminous-Dynamics/symtropy/pull/6857

The composition transaction reducer treats each exact EPCIS source event revision atomically. Its DELETE_ALL conflict footprint includes the parent and every currently active direct child, so a same-tick independent transfer of an affected child is ambiguous rather than lexically ordered around the disaggregation. Nested child contents remain unchanged.

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
- each source namespace, event ID, source-local revision and immutable source-payload digest;
- a canonical group-manifest SHA-256 reference, which the owner case pins separately.

For a shared transformation group, the exact transformation_id must match the owner case's transformation_id. For an event-local TransformationEvent with no transformationID, exactly one effective source event is allowed. A no-ID event cannot be used to silently assemble a multi-event group.

The adapter rejects empty source groups, invalid/padded source identities, revision zero, malformed source digests, duplicate source event revisions and conflicting revisions for the same (source_id, source_event_id) identity. It also requires the supplied group-manifest digest to be well-formed and to match the exact digest pinned in the owner case binding. The group manifest's canonical serialization and digest production are upstream responsibilities; this adapter checks syntax/equality only and does not hash or authenticate bytes.

These checks validate the shape and internal identity of the supplied effective group. They do not prove that the capture stream is complete, that the process is closed, or that every physical input/output event was reported. The upstream observation-completeness programme remains a separate necessary qualification.

That upstream work is already tracked and has an executable synthetic benchmark: [ORBITAL-RELIABILITY-OBSERVATION-COMPLETENESS-001 #6659](https://github.com/Luminous-Dynamics/symtropy/issues/6659), [event-capture observability PR #6676](https://github.com/Luminous-Dynamics/symtropy/pull/6676), [observation-process identifiability PR #6731](https://github.com/Luminous-Dynamics/symtropy/pull/6731), and [externally anchored POD trend PR #6751](https://github.com/Luminous-Dynamics/symtropy/pull/6751). PR #6676's current exact head and both recorded workflows are green; this adapter does not recreate its independent event reference/counter/loss qualification. Its receipt should be connected by a future full owner-case composition, not inferred from an EPCIS group's own event count.

## Exact owner binding

A MaterialBalanceCaseBinding names the owner case, manifest, transformation-group reference, transformationID, process, input-lot set, output scope and expected owner receipt source digest. The normalized TransformationGroupEvidence must carry the same exact process_ref, input_lot_set_ref and output_ref, and all three must equal the owner case binding. This prevents an owner mass-balance receipt for one input/output scope from being attached to a different transformation group by merely reusing the case ID.

The one supplied owner receipt must satisfy:
1. gate == MASS_BALANCE;
2. exact case_id and manifest_ref;
3. exact subject_ref == process_ref;
4. exact scope_ref == input_lot_set_ref;
5. exact process, input-lot-set and output-scope references match between the normalized transformation group and owner case;
6. the canonical transformation-group manifest digest exactly matches the digest pinned by the owner case;
7. the owner receipt source digest is well-formed and exactly matches the digest pinned by the case binding;
8. exactly one MassBalance receipt for this case.

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

## Competitive position and standards boundary

GS1 EPCIS/CBV 2.0.1 publishes normative schemas, JSON-LD/ontology artefacts and event semantics. Its AggregationEvent already distinguishes ADD, incomplete OBSERVE and DELETE-all; its TransformationEvent already defines unordered, many-to-many input/output contribution and shared transformationID groups. The implementation must remain loss-aware and must not claim that parsing or passing a private fixture suite is GS1 certification.

DCSA Track & Trace 2.2 already defines interoperable shipment, transport, equipment and related visibility events and provides an official implementation guide, reference implementation, conformance framework and hosted sandbox. As of 23 November 2026, DCSA's public certification record lists Track & Trace in the 2.x family; its published implementation guide is specifically for 2.2. The public DCSA-TNT repository also contains a Postman collection with a 3.0.0 version string, but that repository artefact alone does not establish a stable 3.x release or certification availability. Pin the exact official release and role from DCSA's current reference before implementation; do not silently substitute a beta or repository default. The strategy is to adopt DCSA's event language and use its official conformance process, not create a competing event taxonomy or claim that our adapter supersedes DCSA. DCSA states that a conformance certification is specific to the tested standard version, role, module and scope; it does not cover an entire product, prove that it is live in production, or transfer automatically to another major version. The public DCSA Track & Trace consumer certification toolkit repository is explicitly marked pre-alpha and says it must not be used to establish conformance; do not treat it as a substitute for the hosted suite.

Operational comparators have different jobs:
- **Kaleris N4** is a terminal operating system integrating vessel, yard, rail, gate and control-room workflows. Its 2026 Yard Intelligence Suite extends yard planning/optimization. We should integrate with the TOS and measure evidence quality across the boundary, not replace dispatch or terminal execution.
- **project44 Ocean Visibility** markets broad carrier/forwarder connections, multimodal stitching, exception alerts and predictive ETAs. Its published coverage and performance figures are vendor claims, not independently adjudicated ground truth. It is a strong baseline for visibility and ETA quality.
- **Dues** is a particularly useful Southern African comparator. It aggregates terminal-published vessel, berth, stack and update information and retains observed plan changes. Its update cadence is published as every 30 minutes. It can support tests of publication-to-display latency, change detection and forecast quality, but its derived display is not independent physical ground truth and does not establish the cargo's physical composition or custody.

The field is not missing dashboards, event standards, visibility networks or operational TOS capability. The opportunity is an interoperable *assurance layer* that can identify which source/revision/evidence supports a cargo identity, composition, process lineage, custody claim or delivery projection—and explicitly abstain where it cannot.

Symtropy's contribution is a layered assurance boundary:
1. preserve GS1/DCSA source semantics and exact source identities;
2. reconstruct source revisions and atomic composition without losing parent/child or nested identity;
3. qualify chronology and observation-channel limitations separately;
4. preserve TransformationEvent many-to-many semantics and bind the group to a separately owned process/material balance receipt;
5. keep inventory, custody, terminal execution, full route disposition and physical acceptance with their respective owners.

### Matched benchmark protocol

Do not compare a synthetic Symtropy reducer against a live commercial platform as if the datasets and responsibilities were equivalent. Use the same shipment/container cohort, source snapshot, as-of cutoff, event availability and identity catalogue for each arm. Keep the strongest operational product in its own domain and compare each capability only where responsibilities overlap.

| Capability | Matched comparators | Primary metrics | Safety/validity gate |
|---|---|---|---|
| Standards interoperability | GS1 EPCIS 2.0.1 artefacts; DCSA T&T 2.2 conformance implementation | Normative test pass rate, semantic preservation, unsupported-field rate | No silent coercion; unsupported features remain explicit |
| Composition reconstruction | Source-native WMS/TOS replay; EPCIS reference fixtures; independent graph oracle | Edge precision/recall, revision accuracy, replay agreement, permutation invariance where semantics allow | Zero false-complete states on negative controls |
| Custody chronology | Carrier/terminal event sequence; independently adjudicated handoff manifests | Strictly ordered chain precision, ambiguous/unqualified fraction, false-order rate | No event is ordered on overlapping/touching/incomparable time intervals |
| Observation completeness | Independent event injection/counters; capture HIL #6659 | Miss probability bounds, detection probability, false-bounded rate, loss-counter sensitivity | No completeness claim from an internally consistent log alone |
| Transformation/material accounting | MATERIAL-ACCOUNTED-ROUTES-001A owner oracle; independent process/material measurements | Exact scope joins, owner-gate agreement, material-balance residual and uncertainty coverage from owner | No yield/allocation/physical balance inferred from EPCIS lists |
| Visibility/ETA | project44 outputs, Dues published board/history, carrier/terminal source publications | Publication-to-display latency, event availability, ETA MAE/median absolute error, p90 error, interval coverage and calibration by prediction horizon | Compare only events available at prediction time; vendor display is not physical truth |
| Terminal execution | Kaleris N4 / terminal-native operational records | Yard dwell, rehandles, moves/hour, gate turn time, berth/yard plan deviation | Read-only shadow before any operational-policy trial |
| Network outcomes | Existing terminal policy, FCFS, local optimizer, robust/multi-objective planner | Backlog area, p90 end-to-end delay, demurrage/detention, essential-cargo reliability, cost and emissions | Preregistered matched or randomized rollout; report spillovers and worst-served cohorts |

### Proposed qualification targets (not achieved results)

**P0 software contracts**
- 100% expected-state agreement on the frozen positive/negative corpus.
- Zero false-complete or false-balanced outputs on every frozen negative control.
- 100% deterministic replay for identical source bytes, schemas, evaluator and policy versions.
- 100% exact identity/scope binding; zero fuzzy-ID or nearest-timestamp fallback.
- 100% preservation of source fields that the adapter does not own; unsupported semantics are quarantined or explicitly represented.

**P2 independent adjudication**
- At least 3,000 independently adjudicated negative cases before making a tight false-complete-rate statement. With zero false accepts in 3,000 independent representative Bernoulli trials, the one-sided 95% exact upper bound is approximately 0.10%; clustered/common-cause cases require cluster-aware inference and a larger effective sample.
- Report precision, recall, false-complete rate, abstention rate and coverage-risk curves separately. Do not hide unsafe false accepts inside a single F1 or accuracy score.
- Report all strata: event family, terminal/source, container/module type, revision pattern, event density, clock domain, missingness mechanism and observation-channel fault class.

**P3 read-only shadow**
- Measure p50/p95 source-publication-to-ingestion, ingestion-to-qualified-projection and correction-to-reconciled-state latency.
- Measure fraction of source records retained, rejected, quarantined, duplicated, superseded and still unresolved at each as-of cutoff.
- Require zero unexplained changes in deterministic replay and zero operational writes.

**P4/P5 operational evaluation**
- Compare existing terminal-local planning, FCFS, current TOS recommendations, exact small-case optimization and robust recovery policies on identical demand/capacity/weather/closure inputs.
- Retain the prior stretch target of at least 10% reduction in held-out network backlog area and p90 end-to-end delay only if essential-cargo reliability, safety, emissions and worst-served cohorts do not degrade. This is a research target, not a forecast or current achievement.
- Require preregistered outcome definitions, independent adjudication, spillover analysis and a holdout period/site before a superiority claim.

The benchmark should compete on false-accept prevention, exact-scope binding, transparent abstention, replay determinism and evidence coverage—not by claiming to replace established EPCIS repositories, TOS products, visibility networks, process simulators or metrology laboratories.

## References

- GS1 EPCIS/CBV 2.0.1 normative artefacts: https://ref.gs1.org/standards/epcis/2.0.1/artefacts
- GS1 TransformationEvent model: https://ref.gs1.org/epcis/TransformationEvent
- DCSA official conformance scope and public 2.x certification record: https://dcsa.org/standard-conformance
- DCSA Track & Trace 2.2 implementation guide: https://reference.dcsa.org/content/standards/guidelines/implementation-guides/implementing-track-and-trace
- DCSA public release catalogue: https://reference.dcsa.org/content/standards/releases
- DCSA-TNT reference repository (inspect exact tags/collection version; not a certification result): https://github.com/dcsaorg/DCSA-TNT
- DCSA Track & Trace certification toolkit (pre-alpha; not a conformance authority): https://github.com/dcsaorg/DCSA-Certification-ToolKit-API-Consumer
- GS1 Global Traceability Standard and CTE/KDE framework: https://support.gs1.org/support/solutions/articles/43000734535-what-is-the-gs1-traceability-standard-
- NIST metrological traceability policy: https://www.nist.gov/metrology/metrological-traceability
- MATERIAL-ACCOUNTED-ROUTES-001A: https://github.com/Luminous-Dynamics/symtropy/issues/5879
- OCEAN-MULTISTREAM-BALANCE-001: https://github.com/Luminous-Dynamics/symtropy/issues/6440

## Claim ceiling

A MassBalanceGatePass is one correctly bound owner gate receipt. It does not prove physical consumption/production, all-event capture, process completeness, material quality, product conformity, safety, environmental compliance, route acceptance, custody or release. No live inventory, WMS, TOS, payment, customs, dispatch, production or physical process mutation is performed.
