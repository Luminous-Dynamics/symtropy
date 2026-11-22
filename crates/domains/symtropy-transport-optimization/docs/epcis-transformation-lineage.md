# EPCIS TransformationEvent Lineage Semantics — v0.1

Issue: https://github.com/Luminous-Dynamics/symtropy/issues/6741

This reference layer translates the meaning of an already-parsed GS1 EPCIS 2.0.1 TransformationEvent. It is deliberately separate from AggregationEvent, custody handoffs and the cargo-composition parent/child graph.

It is not an EPCIS JSON/XML parser, complete field-preserving event store, wire-format schema validator or certification claim.

## Normative event cardinality

The GS1 TransformationEvent model describes physical or digital objects identified at instance level (inputEPCList, outputEPCList) or class level (inputQuantityList, outputQuantityList) that are consumed as inputs and produced as outputs. Inputs may contribute to any outputs; the event does not assert a one-to-one mapping.

The normative cardinality rule is:

| transformationID | Inputs | Outputs | Valid event shape |
|---|---|---|---|
| Omitted | At least one | At least one | Event-local transformation |
| Present | At least one | None | Input-only member of a longer process |
| Present | None | At least one | Output-only member of a longer process |
| Present | At least one | At least one | Input/output member of a longer process |
| Present | None | None | Invalid: no input or output |

When transformationID is omitted, only that event's inputs may contribute to that event's outputs. When present, every input in any event sharing the identical value may have contributed to every output in that same group. The group may span multiple event records.

The identifier is matched exactly. The reference layer does not repair URI strings or claim URI syntax validation; the pinned wire-format/profile layer must do that.

## The representation is a many-to-many relation, not a containment graph

The normalized event retains four separate domains:

- instance-level input EPCs;
- class-level input QuantityElements;
- instance-level output EPCs;
- class-level output QuantityElements.

An event without a transformationID gets an event-local relation scope. Events with the same exact transformationID are assembled into a shared transformation group.

The group exposes attributed input and output assertions, with their original source event/revision references. It explicitly reports:

- UnallocatedManyToMany: the source permits many-to-many contribution, but does not identify which input contributed to which output;
- NotEstablished: the group is not proven complete, closed or fully reported.

The implementation deliberately does not create an allocation matrix, infer a process sequence from list positions, deduplicate source lists or decide that a group is complete because one output event arrived.

## QuantityElement preservation

The transformation adapter reuses the shared AggregationEvent QuantityElement validator and type. It preserves:

1. no quantity and no UOM as an unknown/unspecified quantity;
2. a positive integer count without UOM;
3. a positive physical measure with UOM.

Original quantity lexemes are retained, including exponent form and large integer counts. Class quantities are never expanded into fabricated instance IDs, never added to container/pallet/carton identity counts, and never added across incompatible classes or units.

Repeated QuantityElement entries are preserved exactly as source assertions. The module does not aggregate them.

UOM lexical checks do not authenticate membership in a particular UN/CEFACT Recommendation 20 release, category or current/deprecated status. Full code-list validation requires a pinned vocabulary artifact and profile.

## Source provenance retained by v0.1

The typed event preserves:

- source ID, source event ID and source-local revision;
- source profile ID/version;
- exact transformationID, if present;
- all four input/output lists;
- raw eventTime, eventTimeZoneOffset and optional repository recordTime;
- selected superclass context fields: bizStep, disposition, readPoint.id, bizLocation.id;
- opaque errorDeclaration content/reference.

The source-local revision is provenance metadata, not an EPCIS TransformationEvent field; this layer does not assign a minimum revision number. eventTime and repository recordTime remain separate and are not parsed, normalized or swapped.

Other superclass fields, source/destination lists, sensor data, ILMD, extension vocabularies and complete wire payload retention remain the responsibility of the future profile-aware EPCIS parser/event envelope. No claim of full field-level losslessness is made.

## Replay and grouping

build_transformation_groups:

1. keys event revisions by (source_id, source_event_id, revision);
2. treats exact duplicate source-event revision rows as idempotent replays;
3. rejects conflicting content under the same exact source/event/revision identity;
4. groups only identical non-empty transformationID values;
5. keeps events with no transformationID event-local;
6. sorts groups and member events deterministically through ordered keys;
7. attributes every input/output assertion to its source event;
8. emits no inferred per-object allocation or group-closure state.

Arrival-order permutation of a frozen input set must produce the same normalized grouping report.

## Explicitly separate event families and claims

| Concept | EPCIS / Symtropy representation | What it does not establish |
|---|---|---|
| Physical containment | AggregationEvent | Consumption or production |
| Process transformation | TransformationEvent | Per-input allocation, yield or mass balance |
| Durable/other association | AssociationEvent | Container contents unless that relation is explicitly defined |
| Business transaction context | TransactionEvent / bizTransactionList | Physical nesting or custody |
| Freight custody transfer | Separate custody event/correlation model | Transformation merely because cargo was repacked |
| Material balance | Separate measurement-backed evaluator | Fraud, loss or waste from missing records |

A repacking operation may involve an AggregationEvent, a TransformationEvent, both, or neither depending on what actually happened and what was observed. The adapter must not infer event family from a product or package label.

## Material balance is a second proof

A future balance evaluator may operate only after independent qualification of:

- same material identity or explicit, versioned recipe/conversion relationship;
- compatible physical dimension and pinned UOM vocabulary;
- known quantity state, unit, scale and sign;
- process boundary, time window and output scope;
- coproducts, by-products, waste, scrap, moisture, evaporation and other losses;
- traceable input/output measurements and uncertainty;
- instrument calibration evidence and in-service applicability;
- independent output/receiver evidence.

Do not treat unknown quantity as zero, fixed count as mass, quantity values from different classes as additive, or a missing output event as proof of material loss. A residual outside tolerance should be Unresolved or Inconsistent under a declared model, not automatically a fraud finding.

## Adversarial oracle suite

The Rust tests cover:
- no-ID event with both inputs and outputs;
- no-ID event missing either side;
- shared-ID input-only and output-only events;
- shared-ID empty event rejection;
- multiple inputs and outputs across events sharing one ID;
- distinct IDs staying in different groups;
- no-ID events remaining event-local;
- same EPC in input and output retained without an automatic rework/fraud inference;
- repeated instance and class entries preserved;
- unknown class quantity retained as unknown;
- invalid fractional count without UOM rejected;
- timestamp and recordTime separation;
- selected business context and error declaration retained;
- malformed provenance and transformation ID rejection;
- conflicting same-revision event content rejected;
- exact duplicate replay idempotency;
- grouping determinism under event arrival permutation;
- no inferred mass balance or group completion;
- source attribution on input/output quantities.

These are synthetic semantic tests. They do not independently prove that any input was physically consumed or any output was produced.

## Benchmarks and gates

P0 semantic contract:
- 100% exact agreement with hand-authored golden transformation vectors;
- zero fabricated instance identities or per-input allocations;
- zero input/output mixing with the AggregationEvent containment graph;
- zero group-completion or mass-balance inference;
- deterministic grouping under event-arrival permutation;
- exact replay idempotency and conflicting-revision rejection.

P1 interoperability:
- Run independently adjudicated GS1 EPCIS 2.0.1 JSON-LD and XML fixtures through a future parser and compare every supported field with the normalized meaning layer.
- Pin schema, context, CBV and UOM profile versions.
- Report unsupported-field and semantic-loss rate by source profile.
- Target zero silent loss for supported fields.

P2 independent transformation evidence:
- Compare against independently adjudicated batch, packing/repacking, production and receiving records.
- Measure instance lineage precision/recall, class-quantity preservation, group assembly accuracy, allocation-unknown fraction, and false-complete group rate separately.
- Future target: zero false-complete groups across at least 3,000 independently adjudicated incomplete/invalid negative cases; this is a target, not an achieved result.

P3 measurement-backed process balance:
- Use only preregistered material identities, UOM conversion versions, instrument traceability, process boundaries, coproduct/waste capture and uncertainty budgets.
- Report residual distributions and measurement-coverage rates by material/process class.
- Never compare a generic yield across incompatible products.

## Qualification and claim ceiling

The exact-head crate workflow is the qualification gate for this Rust semantic reference implementation. A passing result means formatting, locked crate tests and strict Clippy pass; it does not mean GS1 wire-format conformance.

No live TOS/WMS writes, inventory/stock mutations, custody changes, dispatch decisions, payments, customs messages or operational optimization are performed. A signature, blockchain or trusted event source cannot substitute for physical measurement or independent adjudication.

## References

- GS1 EPCIS 2.0.1 normative specification: https://ref.gs1.org/standards/epcis/2.0.1/
- GS1 TransformationEvent linked-data model: https://ref.gs1.org/epcis/TransformationEvent
- GS1 JSON-LD TransformationEvent example: https://github.com/gs1/EPCIS/blob/master/JSON/Example_9.6.4-TransformationEvent.jsonld
- GS1 EPCIS XSD TransformationEvent structure: https://github.com/gs1/EPCIS/blob/master/XSD/EPCglobal-epcis-2_0.xsd
