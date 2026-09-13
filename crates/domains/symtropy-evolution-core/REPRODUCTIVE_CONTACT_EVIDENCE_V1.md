# SEL-09A — Complete Reproductive-Contact and Realized Gene-Flow Evidence v0.1

## Status

This document freezes the V1 source contract for exact cross-lineage reproductive-contact and realized gene-flow evidence.

It does **not** establish reproductive isolation, species identity, or speciation.

The governing separation is:

`replicated adaptation != reproductive opportunity != reproductive failure != realized gene flow != reproductive isolation`

SEL-08B2 may establish replication-qualified adaptation. SEL-09A asks a separate question: for an exact preregistered set of cross-lineage reproductive opportunities, what contact/reproduction outcomes and realized hereditary gene-flow evidence were observed under frozen protocols and authorities?

SEL-09B is responsible for reproductive-isolation evidence.

## Adaptation is not a prerequisite

A population pair can exhibit reproductive contact or reproductive barriers without SEL-08B2 adaptation evidence, and adaptation can occur despite unrestricted gene flow.

Accordingly, V1 does not consume adaptation as authority for reproductive outcomes.

No adaptation digest can substitute for contact, parentage, fertility, or realized-gene-flow evidence.

## Preregistered opportunity census

`ReproductiveContactStudyDesign` is outcome-free.

Before outcomes are supplied, it freezes:

- study identity;
- lineage A authority;
- lineage B authority;
- generation interval;
- exact context policy;
- exact complete reproductive-opportunity census;
- opportunity-definition authority;
- contact protocol;
- pairing protocol;
- mating protocol;
- conception protocol;
- offspring-viability protocol;
- offspring-fertility protocol;
- parentage authority;
- realized-gene-flow materialization authority;
- demography-accounting authority;
- missing-data authority.

Outcome observations cannot choose this denominator after the fact.

## Evidence-bearing cross-lineage membership

Every preregistered opportunity binds two persistent individuals.

Each parent slot carries separately:

- persistent `EvolutionIndividualId`;
- the lineage authority the individual is claimed to belong to;
- lineage-membership evidence.

Parent A must bind the exact declared lineage-A authority and parent B must bind the exact declared lineage-B authority.

Changing only lineage-membership evidence changes the design identity.

A mislabeled parent/lineage binding is rejected before reproductive outcomes are interpreted.

This is still an external evidence trust edge: evolution-core binds the membership authority/evidence exactly but does not independently infer lineage membership from the hash.

## Exact context and opportunity completeness

Every opportunity binds:

- semantic opportunity ID;
- exact generation;
- exact parent A/B membership evidence;
- exact evolutionary-context digest;
- exact contact-zone evidence.

Opportunity declarations are canonicalized by semantic opportunity ID.

The following fail closed:

- duplicate opportunity ID;
- empty opportunity census;
- opportunity outside the preregistered interval;
- same individual in both parent slots;
- wrong parent/lineage authority;
- undeclared context drift under `ExactContext`;
- noncanonical persisted order.

`ValidatedReproductiveContactStudyDesign<'_>` is non-Serde and requires fresh replay of all current design authorities and opportunity declarations.

## Terminal reproductive outcomes

Every realized opportunity has exactly one terminal outcome:

- `NoContact`;
- `ContactNoPairing`;
- `PairedNoMating`;
- `MatingNoConception`;
- `ConceptionNoViableOffspring`;
- `ViableOffspringFertilityUnknown`;
- `ViableInfertileOffspring`;
- `ViableFertileOffspring`;
- `Unavailable` at an explicit observation stage.

The terminal enum prevents impossible combinations such as simultaneously encoding `NoContact` and a viable offspring.

## Frozen stage protocols are executable constraints

Each observed stage carries a `ReproductiveStageEvidence` pair:

- exact frozen protocol authority;
- exact observation-evidence authority.

Later terminal states carry the complete observed stage chain.

For example, `MatingNoConception` binds and validates contact, pairing, mating, and conception evidence against the corresponding preregistered protocols.

A caller may not switch only the conception method after observing the result while leaving the design unchanged.

For viable offspring, the full chain through offspring viability is required. Fertile/infertile outcomes additionally bind the frozen fertility protocol.

`Unavailable` binds the explicit stage, frozen missing-data authority, and reason; it does not fabricate earlier stage observations.

## Offspring parentage and reproduction provenance

When a viable offspring is represented, `ObservedOffspringEvidence` binds:

- reproduction event ID;
- exact persistent parent-A individual ID;
- exact persistent parent-B individual ID;
- existing `ReproductionProvenanceDigest`;
- frozen parentage authority;
- parentage evidence.

The persistent parent IDs must exactly equal the preregistered opportunity pair.

Thus a valid reproduction provenance from some other pair cannot be attached to the current opportunity.

The underlying `ReproductionProvenanceDigest` already represents the existing reproduction engine's schema, reproductive event, mode, operator profile, parental hereditary states, and child hereditary state. SEL-09A reuses that identity rather than inventing another heredity engine.

V1 still treats current parentage/reproduction evidence as a bound trust edge; SEL-09A itself does not reconstruct the entire reproductive derivation from raw schema/parent/operator/child values during local validation.

## Fertility is separate from offspring production

The following remain distinct:

`offspring produced != offspring viable != offspring fertile != realized later gene flow`

A viable infertile hybrid is not represented as a prezygotic failure.

A viable fertile hybrid is explicit evidence against a complete reproductive-barrier narrative, but SEL-09A itself still does not emit an isolation status.

## Realized gene flow is a separate evidence channel

Every opportunity carries one `RealizedGeneFlowObservation`:

- `NoneObserved` under the frozen gene-flow materialization authority;
- `Realized` with exact ancestry/gene-flow evidence;
- `Unavailable` under the frozen missing-data authority.

Realized hereditary contribution is intentionally separate from direct contact observation.

A direct record may say no contact was observed while independent ancestry evidence demonstrates realized gene flow. In that case the study summary prioritizes `CrossLineageGeneFlowObserved` rather than erasing the ancestry evidence.

This allows direct observation failure, migration, historical contribution, or incomplete observation to remain scientifically visible.

## Complete opportunity ledger

`ReproductiveContactStudy::capture(...)` requires exactly one current input for every preregistered opportunity.

Inputs may arrive in any order; persisted records are canonicalized to preregistered opportunity order.

Omitted successful hybrids, omitted failed attempts, duplicate inputs, undeclared opportunities, changed demography authority, changed stage protocols, changed parentage authority, changed persistent parent pair, changed gene-flow materialization authority, and changed missing-data authority all fail closed.

No favorable or unfavorable reproductive episode may silently disappear from the represented interval.

## Typed 09A summary

`ReproductiveContactStudyStatus` is one of:

- `CrossLineageGeneFlowObserved`;
- `ViableFertileHybridObserved`;
- `ViableInfertileHybridObserved`;
- `CrossLineageReproductionObserved`;
- `NoRealizedGeneFlowWithObservedOpportunity`;
- `NoContactObserved`;
- `InsufficientEvidence`.

This status is a compact description of the complete opportunity ledger, not a reproductive-isolation classifier.

Status precedence intentionally preserves strong positive evidence:

1. realized gene flow;
2. viable fertile hybrid;
3. viable infertile hybrid;
4. viable offspring with fertility unknown;
5. unavailable evidence;
6. all opportunities with no observed contact;
7. otherwise observed opportunity without realized gene flow.

The persisted status is recomputed locally from the complete ledger. Serialized status tampering invalidates canonicalization.

## Opportunity denominator theorem

`zero observed hybrids != reproductive isolation`

A study can distinguish:

- preregistered opportunity but no observed contact;
- contact without pairing;
- pairing without mating;
- mating without conception;
- conception without viable offspring;
- viable offspring with unknown fertility;
- viable infertile offspring;
- viable fertile offspring;
- realized hereditary gene flow;
- unavailable evidence.

This prevents geographic separation, mate scarcity, absent opportunity, observation failure, or missing ancestry data from being silently rewritten as reproductive incompatibility.

## Representation identity versus current authority

`ReproductiveContactStudyDesign` and `ReproductiveContactStudy` are serializable evidence representations.

Their canonical digests establish representation identity only.

Current authority remains non-Serde:

- `ValidatedReproductiveContactStudyDesign<'_>` requires fresh current preregistration replay;
- `ValidatedReproductiveContactStudy<'_>` requires a current design plus fresh complete opportunity evidence inputs.

A deserialized study cannot self-authorize.

## Adversarial corpus

The V1 corpus covers at minimum:

- canonical opportunity ordering;
- no-contact observation distinct from observed reproductive failure;
- viable infertile and viable fertile hybrids remain distinct;
- real reproduction-provenance digest from the existing offspring engine;
- realized ancestry/gene flow outranks contradictory direct no-contact observation;
- omitted preregistered opportunity rejection;
- changed contact-stage protocol rejection;
- changed gene-flow materialization authority rejection;
- lineage-membership-only design identity drift;
- parent A bound to the wrong lineage rejected;
- offspring evidence bound to the wrong persistent parent pair rejected;
- exact-context drift rejection after restoration;
- Serde restore plus fresh current replay;
- unavailable contact/gene-flow evidence remains typed insufficient evidence;
- serialized status tampering rejection;
- wire shape contains no isolation/species/speciation claim.

## Explicit non-claims

SEL-09A does **not** establish:

- reproductive isolation;
- intrinsic incompatibility;
- behavioral isolation;
- ecological isolation;
- species identity;
- a historical speciation event;
- irreversible lineage separation;
- universal absence of gene flow.

Even repeated `NoRealizedGeneFlowWithObservedOpportunity` studies remain contact/gene-flow evidence only.

## Successor

SEL-09B must consume multiple current 09A studies under a preregistered barrier design with explicit opportunity thresholds, independence rules, context compatibility, contradiction rules, and barrier-component semantics before reproductive-isolation authority exists.

A viable fertile hybrid or realized hereditary gene flow must remain visible as potential contradiction under the chosen isolation model.

## Qualification status

Source review, static reasoning, and mergeability are not executable qualification.

A CI-only helper must execute one exact frozen 09A product head with pinned Rust and named prerequisite corpora. Queued or unassigned jobs are neither PASS nor product-code FAIL.
