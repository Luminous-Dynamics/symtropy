# Cargo Composition Lineage — Industry and Standards Comparator Register

## Standards baseline: reuse GS1 EPCIS/CBV, do not invent another vocabulary

The applicable public semantic baseline is GS1 EPCIS 2.0.1 / CBV 2.0.0:
- EPCIS AggregationEvent describes physical parent/child aggregation and disaggregation. It can represent individual child EPCs and class-level child-quantity lists; its action model includes ADD, OBSERVE and DELETE. DELETE semantics can disaggregate all children when child lists are omitted, so the source adapter must preserve that distinction.
- EPCIS AssociationEvent represents a more general or longer-lived association and must not be assumed to mean temporary physical containment.
- EPCIS TransformationEvent describes input objects being partially or fully consumed to produce output objects. It is not a reversible containment edge and must not be coerced into AggregationEvent.
- EPCIS eventTime and repository recordTime are distinct. Keep raw source timestamps and the exact source profile/version.
- CBV supplies shared business-step/disposition vocabulary, but event vocabulary conformance does not prove the event's issuer authority or physical truth.

References:
- EPCIS 2.0.1: https://ref.gs1.org/standards/epcis/2.0.1/
- AggregationEvent: https://ref.gs1.org/epcis/AggregationEvent
- AssociationEvent: https://ref.gs1.org/epcis/AssociationEvent
- TransformationEvent: https://ref.gs1.org/epcis/TransformationEvent
- CBV 2.0.0: https://ref.gs1.org/standards/cbv/2.0.0/

The current P0 Rust reducer is intentionally a strict subset: one explicit module child per ADD or child-specific DELETE. It does not claim EPCIS conformance, and does not implement OBSERVE, childQuantityList, AssociationEvent, TransformationEvent, or DELETE-all. Mycelix #437 owns the versioned loss-aware mapping and must report unsupported/lossy translation instead of silently coercing those semantics.

## Existing terminal and visibility capabilities

- Kaleris describes N4 as a terminal operating system integrating vessel, yard, rail, gate and control-room operations, with equipment and yard optimization capabilities. It is a relevant source-native TOS baseline, not a capability to dismiss or claim we replace. https://kaleris.com/terminal-operating-system/kaleris-n4-tos/
- Dues publishes Southern African port visibility pages and describes frequent refreshes from terminal-published files. Its DGT view exposes vessel status, berth planning, import storage and export stack information. This is a useful regional public aggregation/parser comparator; it remains a secondary view derived from terminal files, not independently adjudicated packing ground truth. https://dues.co.za/ports and https://dues.co.za/ports/durban-gateway-terminal
- project44 describes ocean visibility, carrier/forwarder connectivity, container tracking and ETA prediction. These are established shipment-visibility capabilities. Vendor-published network/coverage figures are not independently audited benchmark labels. https://www.project44.com/platform/visibility/ocean/
- DCSA Track & Trace provides an industry container-shipping event/API baseline. Exact public profile/version availability matters; do not claim certification/conformance to a profile the test cannot lawfully obtain and execute. https://dcsa.org/standards/track-and-trace/

Do not infer that any named vendor lacks aggregation, packing, inventory or custody-adjacent capabilities. Those must be tested from permissioned source-native exports and the vendor's actual licensed profile.

## Where the proposed contribution sits

The contribution is not “we have a parent-child graph” or “we support EPCIS-like ADD/DELETE”. Those concepts already exist.

The research question is whether independently reconciled source events can reconstruct a physically plausible, revision-aware composition graph across multiple systems while:
- retaining source-native identity and event versions;
- preserving exact parent/child membership and disaggregation;
- rejecting cycles, double parents, identity substitution and unsupported semantic coercion;
- emitting no count from an unqualified graph;
- keeping physical module identities separate from TEU, tonnes, moves, bookings and shipments;
- preventing aggregation from being mistaken for custody, title, customs release or delivery.

The graph composes with existing module identity/compatibility (#1602), canonical box lineage (#6631), custody continuity (#6662), conserved backlog/stock (#6493), and end-to-end freight service (#3683). Mycelix #437 owns the cross-standard adapter and translation-loss receipt.

## Matched P1/P2 experiment

Use the same permissioned event snapshots, module catalogue, source identity mappings, as-of cutoff, and independently adjudicated packing/strip manifest for every system.

Baselines:
1. Source-native WMS/TOS parent-child/handling-unit history, where the operator provides a lawful export.
2. Dues/other public terminal-view projection, where the source snapshots and refresh time can be archived lawfully.
3. Direct EPCIS 2.0.1 AggregationEvent projection, pinned to exact public release and CBV version.
4. Mycelix #437 versioned EPCIS adapter with mapping-loss output.
5. Plain relational parent-child joins.
6. Independent finite graph oracle.
7. Symtropy composition reducer over canonical upstream module IDs.

Metrics:
- parent-child edge precision/recall and exact graph reconstruction rate;
- false parent, missed edge, duplicate parent and cycle rates;
- disaggregation/reparenting accuracy;
- revision, void, replay and historical-cutoff correctness;
- unsupported/lossy translation and unresolved-ID fraction;
- fraction of source records with independent packing-manifest adjudication;
- root/leaf projection correctness, with measure and denominator declared;
- latency, memory, throughput and worst-case graph-size behavior.

The ground truth must be independently adjudicated from physical packing/strip records or instrumented transfers, not generated by the tested WMS/TOS or EPCIS projection itself.

## Claim ceiling

No superiority result exists yet. A synthetic P0 pass establishes the reducer's finite graph invariants only; it does not establish actual packing, module compatibility, load securing, contents, legal custody, title, customs status, cargo release, or delivery.
