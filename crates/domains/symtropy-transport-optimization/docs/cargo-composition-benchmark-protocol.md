# Cargo Composition Lineage Benchmark Protocol v0.1

## Research question

Can a deterministic, source-qualified composition graph reconstruct the physical parent/child handling-unit structure across terminal, warehouse, feeder and inland handoff systems without inventing identities, duplicating cargo stock, or converting aggregation into custody, title or transformation?

The v0.1 evaluator consumes module IDs from the existing cargo-module contract and revisioned source event assertions. It does not implement an EPCIS adapter or authenticate event issuers.

## Scope and semantic distinctions

Three relations remain distinct:

1. **Physical aggregation/disaggregation** — an identity-preserving association between a parent handling unit and child handling units. The child retains its own identity.
2. **Transformation/repacking/consumption** — input objects are partly or fully consumed and output objects are created. This requires a material-process or transformation event with explicit yield, waste, conversion and output identities.
3. **Custody transfer** — a separately authorized handoff that changes the custodian. A parent/child association does not change custody.

P0 implements only instance-level ADD and child-specific DELETE. It does not implement:
- EPCIS OBSERVE;
- class-level childQuantityList;
- EPCIS DELETE with omitted child list meaning disaggregate all children;
- AssociationEvent;
- TransformationEvent;
- content-manifest digest mutation;
- quantity conversion into TEU, tonnes or moves.

Unsupported source semantics must remain Unsupported/NeedsProfileWork; they must not be guessed or coerced.

## Frozen P0 contract

- Input module catalogue: unique, non-empty module IDs with an already-declared CargoModuleFamily.
- Event key: (source_id, source_event_id).
- Event fields: revision, publication time, event time, parent ID, child ID, ADD/DELETE, planned/actual, source profile/version, location, opaque evidence reference, void flag.
- Explicit as-of publication cutoff.
- Future-published events are excluded.
- Latest visible revision supersedes earlier revisions.
- Same-key, same-revision conflicting payloads reject the snapshot.
- Exact duplicate replay is idempotent.
- Latest void revision suppresses the event.
- Planned events never mutate graph membership.
- Actual events apply in event-time order; event-time ticks are synthetic and dimensionless in P0.
- If same-time events touch the same module, the batch is ambiguous and not ordered by source ID.
- Independent same-time events over disjoint module sets can be applied deterministically.
- The active graph is acyclic, self-containment forbidden, and each child has at most one active parent.
- ADD requires a known parent/child and an unassociated child.
- DELETE removes only the exact active parent-child edge.
- A graph with any composition failure cannot emit module-count projections.

## Composition and accounting invariants

The graph is a relation, not an additive stock ledger.

For a nested structure:

`CONTAINER → PALLET → CARTON`

The catalogue contains three physical module identities. It does not contain six units because each edge has two endpoints; nor does it count a carton and its containing pallet as separate delivered shipments.

Supported projections:
- AllModuleIdentities — count every distinct module identity in the declared catalogue.
- RootHandlingUnits — identities with no active parent.
- LeafHandlingUnits — identities with no active children.

These are different measures. None equals ISO container count, TEU, discharged boxes, moves, shipment count, tonnes or delivered cargo without a separately qualified measurement mapping and declared denominator.

A failed or unresolved graph returns no count projection.

## Independent finite oracle / falsification suite

The authored tests cover:
- nested container→pallet→carton aggregation;
- exact disaggregation and reparenting to another container;
- one active parent per child;
- unmatched DELETE;
- self-containment;
- multi-edge cycle creation;
- same-time shared-module ambiguity;
- disjoint same-time event permutation invariance;
- planned and future-published records;
- latest void and same-revision collision;
- exact duplicate replay;
- missing/unknown module;
- missing evidence reference;
- module count projection refusal when graph unqualified;
- separate all/root/leaf count projections.

Hard P0 gates:
- zero cycles in qualified graphs;
- zero children with multiple active parents in qualified graphs;
- zero unmatched deletes hidden as success;
- zero invalid count projections;
- zero duplicate physical stock increments from parent/child edges;
- deterministic report under reverse input order.

## P1 semantic adapter tests

Reuse Mycelix #437 for versioned external standard translation. Pin exact GS1 EPCIS 2.0.1 and CBV 2.0.0 profile IDs where used, and preserve raw fields and loss reports.

Minimum adversarial cases:
- OBSERVE incorrectly promoted to ADD;
- class-level childQuantityList expanded into fabricated unique identities;
- DELETE with no child list misread as one-child deletion;
- TransformationEvent coerced to aggregation;
- AssociationEvent coerced to physical container contents;
- eventTime substituted with recordTime;
- source revision/void lost during translation;
- unknown child identity resolved by similar string;
- aggregation event treated as custody transfer or proof of delivery;
- unsupported source field silently discarded.

Only claim round-trip losslessness or semantic equivalence where exact profile fixtures and independent round-trip oracles demonstrate it.

## P2 independent packing/strip validation

Use permissioned, independently adjudicated packing manifests, warehouse handling-unit scans, interchange receipts and container stripping/unpacking records. Maintain a held-out set by facility, operator, module family, event source and time period.

Measure:
- edge-level precision / recall;
- exact parent-child graph reconstruction rate;
- false parent assignments and missed memberships;
- cycle / multiple-parent contradiction rate;
- abstention and unresolved identity fraction;
- reparenting and disaggregation correctness;
- source correction/void handling;
- coverage by module family and facility;
- latency and memory per 1k / 100k / 1m events.

Ground truth must come from independent packing / strip adjudication, not from the same TOS export used as the tested system's input.

## Matched competitors

Replay identical adjudicated event tapes and as-of cutoffs against:
1. source-native WMS/TOS parent-child inventory history;
2. a direct GS1 EPCIS 2.0.1 AggregationEvent projection;
3. the Mycelix #437 versioned/loss-aware EPCIS adapter;
4. a plain relational parent-child join;
5. a simple independent finite graph oracle;
6. Symtropy cargo composition graph.

Compare schema/profile coverage separately from physical membership correctness. Do not claim EPCIS conformance from a similar internal enum. Do not claim superiority over WMS/TOS vendors without permissioned data and independent ground truth.

## Claim ceiling

P0 is a synthetic relation reducer only. It does not prove packing, container contents, fit, lashing, load securing, dangerous-goods compliance, seal integrity, custody, legal title, customs release, cargo delivery, or inventory-book correctness. It does not alter module content digests, custody state, location, seal state, inventory stock or any operational system.
