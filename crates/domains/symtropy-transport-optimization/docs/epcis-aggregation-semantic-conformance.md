# EPCIS AggregationEvent Semantic Preservation — v0.1

Issue: [MARITIME-EPCIS-AGGREGATION-SEMANTICS-001](https://github.com/Luminous-Dynamics/symtropy/issues/6718)

This reference layer translates the meaning of an already-parsed GS1 EPCIS 2.0.1 AggregationEvent. It closes a known boundary in cargo-composition v0.1: that reducer supports instance-level ADD and child-specific DELETE, but does not yet implement OBSERVE, class-level childQuantityList, or DELETE with omitted children.

This is **not** a JSON/XML parser, full EPCIS adapter, certification, or conformance claim.

## The semantic firewall

An adapter must not reduce every source event to an unconditional parent-child edge mutation. The three EPCIS actions have different meanings:

| EPCIS action | Parent | Child list | Normalized meaning | Membership mutation |
|---|---|---|---|---|
| ADD | Required | At least one of childEPCs / childQuantityList | Add the listed instance and/or class-quantity relationships | Yes |
| OBSERVE | Optional | At least one list must be non-empty | Record an observation that may be incomplete | **No** |
| DELETE | Required | One or more listed entries | Remove only the listed children | Yes |
| DELETE | Required | Both lists empty/omitted | Explicitly disaggregate every child from this parent | Yes — delete all |

An OBSERVE is neither an ADD nor a snapshot asserting that all unlisted children are absent. It can be partial, and the parent itself may be unobserved or unknown. The safe representation is therefore a distinct observation value which cannot be mistaken for a membership command.

An empty DELETE is not an empty event. EPCIS defines it as the disaggregation of all children from the identified parent. Conversely, empty ADD and empty OBSERVE are rejected by the reference contract.

## Identity and quantity are not interchangeable

EPCIS childEPCs names instance-level child identities. These may be mapped to existing canonical physical module identities only by the upstream, source-qualified lineage layer.

EPCIS childQuantityList names class-level quantities. For example, a quantity of 12.50 EA of a given class is not twelve known physical child IDs, and it must not be expanded into fabricated pallet/carton identities. The reference module retains the decimal lexeme exactly and keeps class quantities in a distinct typed field.

The class-level quantity path must remain outside AllModuleIdentities, RootHandlingUnits, and LeafHandlingUnits until a separate, explicit measurement mapping establishes the unit, denominator, identity granularity and admissible aggregation rules. It must never be added to instance counts by default.

## Temporal and source provenance

The normalized result retains these fields independently:

- source ID and source event ID;
- source profile ID and version;
- raw EPCIS eventTime lexeme;
- EPCIS eventTimeZoneOffset lexeme;
- optional raw repository recordTime lexeme.

The offset is checked for the EPCIS +/-HH:MM lexical shape and the legal range through 14:00; it is not used to silently rewrite eventTime. Timestamp parsing, source timezone meaning, clock identity, normalization, calibration and uncertainty qualification remain with the established timestamp owners (#1930 / #1955).

recordTime is repository bookkeeping. It is not the time at which the physical event occurred. It must not replace eventTime, become an event-time bound, or be mixed into a physical chronology calculation.

Source-event revision selection, publication cutoffs, error declarations, voiding, issuer authentication and the exact event key remain responsibilities of the upstream source admission and replay layers. This module does not rewrite source records.

## Executable contract

The Rust reference function normalize_aggregation_event:

1. Rejects blank or padded source identities, source profiles, instance child IDs and parent IDs rather than trimming them into a new identity.
2. Rejects duplicate instance children and duplicate (object class, UOM) quantity keys rather than silently deduplicating.
3. Preserves positive decimal quantity strings exactly; it does not use floating-point values.
4. Rejects empty ADD/OBSERVE child lists and requires a parent for ADD/DELETE.
5. Distinguishes OBSERVE, subset DELETE and DELETE_ALL as separate enum variants.
6. Preserves eventTime, eventTimeZoneOffset and recordTime separately.
7. Performs no composition graph mutation and invents no physical module identities.

The unit-of-measure field is required to be a non-empty, unpadded identifier. Semantic membership in a particular UN/CEFACT or GS1 code-list release is not asserted here; that requires a separately pinned code-list profile. This prevents a syntactically plausible code from being misrepresented as independently verified.

## Falsification suite

The tests cover:

- ADD with instance children;
- ADD with class-level quantities;
- ADD carrying both lists;
- OBSERVE with a parent;
- OBSERVE without a parent;
- incomplete OBSERVE remaining non-mutating;
- subset DELETE;
- DELETE_ALL when both lists are empty;
- empty ADD / OBSERVE rejection;
- missing-parent rejection for ADD / DELETE;
- duplicate instance child rejection;
- exact-decimal preservation and non-positive / malformed quantity rejection;
- duplicate class/UOM key rejection;
- separate eventTime and recordTime retention;
- invalid timezone-offset rejection;
- no repair of padded identifiers.

The oracle is hand-authored against the action semantics and deliberately does not claim that the event itself is physically true.

## Explicitly separate event families

- AggregationEvent models a strong physical containment relationship.
- AssociationEvent models a different, potentially longer-lived association and must not be silently mapped to physical containment.
- TransformationEvent describes transformation inputs and outputs. It requires its own material-balance contract for input, output, yield, loss, scrap, unit conversion and provenance; it is not an aggregation shortcut.
- DCSA planned / estimated / actual classification is a separate dimension from EPCIS action. Neither one implies the other.

## Industry and standard references

- [GS1 EPCIS 2.0.1 normative specification](https://ref.gs1.org/standards/epcis/2.0.1/)
- [GS1 AggregationEvent model](https://ref.gs1.org/epcis/AggregationEvent)
- [GS1 ObjectEvent model, including eventTime / eventTimeZoneOffset / recordTime / errorDeclaration](https://ref.gs1.org/epcis/ObjectEvent)
- [GS1 TransformationEvent model](https://ref.gs1.org/epcis/TransformationEvent)
- [GS1 long-term/fixed associations guideline](https://github.com/gs1/EPCIS/blob/master/Implementation%20Guideline/Section5dot10.md)

## Claim ceiling and next step

This module qualifies only a semantic mapping contract for a narrow, already-parsed AggregationEvent structure. It does not implement the EPCIS wire format, dereference evidence, authenticate issuers, validate source rights, prove a physical packing operation, verify a receiver, establish custody, or certify a standards implementation.

The next integration should feed only instance-level ADD and subset DELETE into the existing graph reducer. OBSERVE should create an observation record; DELETE_ALL should invoke a parent-scoped graph operation with explicit semantics; class quantities should enter a separate, source-qualified quantity ledger. Association and transformation events need independent contracts before they are admitted.
