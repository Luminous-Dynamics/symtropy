# EPCIS AggregationEvent → Cargo Composition Projection Protocol v0.1

Issue: https://github.com/Luminous-Dynamics/symtropy/issues/6742

This module connects the already-parsed AggregationEvent semantics in epcis_aggregation.rs to the existing instance-level cargo composition reducer. It is a deterministic, read-only projection report. It does not parse EPCIS JSON/XML, authenticate the source issuer, prove physical packing or mutate WMS/TOS/inventory state.

## Semantic projection table

| EPCIS normalized effect | Projection | Current reducer behaviour |
|---|---|---|
| ADD with childEPCs | One source-linked instance membership event per child EPC | Supported by cargo composition |
| ADD with childQuantityList | Separate attributed class-quantity ledger assertions | Never expanded to physical module IDs |
| OBSERVE | Observation ledger only; parent may be absent and list may be partial | No graph mutation |
| DELETE with childEPCs | One source-linked child-specific membership deletion per named child | Supported by cargo composition |
| DELETE with childQuantityList | Separate class-quantity deletion assertions | No instance membership change |
| DELETE_ALL | First-class parent-scoped DisaggregateAll transaction | Removes only that parent's direct aggregation edges; nested child contents remain |
| Invalid / missing / unqualified evidence | Quarantine with source key and reason | No graph event or count result |

The source event's child lists remain unordered; the projection never treats list position as causal or temporal order. QuantityElement values retain exact source lexemes, optional quantity/UOM states and repeated entries. Unknown quantity is not zero. Class quantities do not contribute to AllModuleIdentities, RootHandlingUnits or LeafHandlingUnits.

## Source revisions and idempotency

- Resolve latest visible non-void source-event revision at the caller's explicit publication cutoff.
- Exact duplicate same-key/same-revision envelopes are idempotent.
- Conflicting same-key/same-revision content rejects the projection.
- A newer source revision replaces the original event in full.
- Each instance child uses a stable, length-prefixed derived composition event ID tied to the original source event ID and child EPC. The provenance sidecar retains the exact original source ID, source event ID and revision.
- If a newer source revision removes an earlier child assertion, a derived voided tombstone supersedes the older child projection. This prevents a stale child edge from surviving a source correction.
- Source envelopes are retained in the report with eventTime, eventTimeZoneOffset, recordTime, source profile/version, source payload digest, adapter build digest, time evidence reference and revision.

The derived projection key is not represented as an original EPCIS source event. Consumers must keep its provenance sidecar and never publish it back to source systems as if it were the original event.

The cargo reducer now accepts one atomic CompositionEventBatch per exact originating source event/revision. Multiple child ADDs or DELETEs in one source event are one transaction, not independent simultaneous events. Independent source events that touch the same module at the same event-time tick still produce an AmbiguousEventTime failure. The conflict footprint for DELETE_ALL includes the selected parent and every currently active direct child it would remove, so a same-tick transfer/reparent of one of those children cannot be lexically ordered around the parent-scoped deletion. A failed multi-child transaction rolls back every child mutation.

## Chronology and tick-domain safety

A caller supplies an already-normalized integer event-time tick and an explicit chronology qualification:

- Qualified: source-event/revision-bound time-evidence ID, immutable evidence SHA-256, time-basis ID, tick-quantum ID and a closed earliest/latest interval.
- SyntheticP0: an explicit fixture profile. Ticks are dimensionless and have no physical-time meaning.
- Unqualified: the event is quarantined.

For a Qualified event, the module checks:
- the evidence ID equals the canonical exact (source_id, source_event_id, revision) binding;
- the evidence digest is valid lowercase SHA-256 syntax;
- the event tick lies inside the stated interval;
- all actual events in the projection batch use one common time basis and tick quantum.

Before integer event-time ticks reach the composition reducer, the projection independently proves that every actual event touching the same module has a strictly disjoint interval. It sorts by (earliest, latest, source-event-key) and scans the prefix maximum latest endpoint, requiring max(previous latest) < current earliest. Overlap and touching reject the batch. This is O(n log n) per projection batch plus a linear scan; it does not infer order from raw eventTime lexemes or recordTime.

The projection trusts the upstream custody/time adapter to produce an authentic, source-bound qualified evidence digest and interval. It validates the reference binding and consistency but is not itself a clock-calibration verifier. It does not independently authenticate calibration artifacts or physical clock readings.

## Completeness and count policy

AggregationCompositionProjectionReport.evaluate_instance_composition calls the grouped composition transaction reducer if there are no quarantined selected source events. DELETE_ALL is a supported atomic mutation and no longer blocks counts by itself. The reducer still refuses counts when any transaction is unqualified, invalid or ambiguous.

Class-quantity and observation ledgers remain separate and do not invalidate an otherwise complete instance graph merely by existing. They are not converted into instance stock. Any graph-level composition failure remains the responsibility of the underlying reducer and its fail-closed count projections.

DELETE_ALL is now consumed by a first-class DisaggregateAll mutation in CompositionEventBatch. It removes all active Membership edges whose direct parent_module_id exactly equals the named EPCIS parent. It does not remove the parent's own membership under another parent, does not cascade into any child's own contents, and does not touch AssociationEvent or any other relation family. Unknown parents fail closed. The CompositionReport records both the number of disaggregation-all events and the number of direct edges removed.

A corrected or voided source revision emits RetractIfPresent compensation for child assertions that the previous source revision no longer contains. This operation is distinct from physical child-specific DELETE: it is idempotent when the superseded edge is absent in a from-scratch replay, but removes that exact edge when replaying against prior state. Ordinary EPCIS DELETE remains strict and an unmatched physical deletion still fails. All compensation and current-revision additions are applied in one atomic batch, with retractions before additions and full rollback on any other failure.

## Adversarial test coverage

The Rust tests cover:
- ADD with instance children plus separate class quantities;
- OBSERVE without parent and no membership mutation;
- child-specific DELETE;
- DELETE_ALL parent-scoped direct-child removal with nested contents preserved;
- class-only ADD and unknown quantity preservation;
- repeated quantity entries without aggregation;
- exact duplicate replay and conflicting same-revision rejection;
- source revision child-removal tombstones;
- latest void revision suppressing prior membership;
- future-published revision exclusion;
- unqualified chronology quarantine;
- exact event-time evidence ID binding;
- invalid interval enclosure;
- overlapping/touching event intervals on a shared module;
- mixed time bases/tick quanta and mixed P0/P1 domains;
- event arrival permutation;
- raw eventTime, eventTimeZoneOffset, recordTime and source evidence digest preservation;
- distinct AllModuleIdentities, RootHandlingUnits and LeafHandlingUnits projections.

These are synthetic semantic tests. They do not prove physical contents, custody, seal integrity, delivery, or operational truth.

## Claim ceiling and next integration

This projection is read-only. It does not update stock, cargo modules, WMS, TOS, customs, bookings, delivery, custody or payment records. It does not prove packing happened or that a source assertion is physically true.

The next reducer increment should add an explicit parent-scoped DELETE_ALL operation as a first-class event type, preserving event sourcing, revision/void semantics, active parent invariants and historical replay. Only after that path has independent tests should DELETE_ALL cease to block count projections.
