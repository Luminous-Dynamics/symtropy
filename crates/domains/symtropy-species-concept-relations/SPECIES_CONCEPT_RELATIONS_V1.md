# SEL-10E1A3 Species-Concept Semantic Relations V1

## Purpose

SEL-10E1A3 adds an evidence-bearing relation layer between exact species-concept identities.

The core theorem is:

`distinct concept identity != semantic independence`.

A second concept family may generalize, refine, operationalize, overlap with, or otherwise depend on another concept. Downstream cross-model robustness must preserve that structure rather than counting family IDs as independent votes.

## Layering

The relation crate is a sibling layer above the open descriptor waist:

`symtropy-evolution-core -> symtropy-species-concept -> symtropy-species-concept-relations`

SEL-10E1A3 does not change the canonical identity of SEL-10E1 or SEL-10E1A2 objects.

## Endpoint identity

Relation endpoints are exact `OpenSpeciesConceptIdentity` values:

- family ID;
- family version;
- semantic content digest.

They intentionally exclude source-model qualification and validity-domain authority.

Therefore requalification or domain drift on one concrete model authority does not manufacture a new conceptual endpoint.

SEL-10E1A3 does not itself prove that either endpoint is currently active. A downstream consumer such as SEL-10E2 must exact-match relation endpoints to the current validated model-family descriptors it is actually comparing.

## Outcome-free relation design

`SpeciesConceptRelationDesign` is declared before a relation assessment is admitted.

It freezes:

- relation design ID;
- exact unordered concept pair;
- relation scope;
- relation protocol authority;
- missing-evidence policy;
- the built-in V1 relation rule.

The two endpoints are canonicalized lexicographically. Caller endpoint order is non-semantic.

Persisted restored bytes must already be canonical. A restored endpoint reversal is invalid rather than silently rewritten.

A concept cannot be related to itself in V1.

## Relation kinds

V1 supports:

- `EquivalentSemanticTarget`;
- `Generalizes`;
- `Refines`;
- `OperationalCriterionWithin`;
- `PartiallyOverlaps`;
- `OrthogonalEvidenceFramework`;
- `CompetingOntology`.

`Generalizes`, `Refines`, and `OperationalCriterionWithin` are directional.

The other V1 relations are symmetric.

Direction is expressed relative to the design's canonical left/right endpoint order, so swapping caller input order does not change relation identity.

## Dependency classes

V1 exposes only three semantic-dependency classes:

### `NestedOrCriterionDependent`

Used for:

- equivalent semantic target;
- generalizes;
- refines;
- operational criterion within.

These relations cannot satisfy a downstream semantic-independence gate.

### `Overlapping`

Used for partial overlap.

This remains dependency-visible and cannot silently upgrade to independence.

### `PotentiallyNonNested`

Used for:

- orthogonal evidence framework;
- competing ontology.

**This is not an independence claim.**

It means only that the qualified semantic relation is not represented as nested/criterion-dependent or overlapping in V1. Downstream robustness must separately establish evidence-source, qualification, implementation, and other relevant fault-domain independence.

## Assessment states

`SpeciesConceptRelationAssessment` is one of:

- `Qualified`;
- `Disputed`;
- `UnknownOrUnqualified`;
- `Unavailable`.

Only `Qualified` exposes a dependency class.

Disputed, unknown, and unavailable evidence expose no semantic-dependency result.

Under `FailClosed`, unknown or unavailable evidence cannot be persisted as relation evidence.

Under `ReportUnknownOrUnavailable`, those states remain explicit rather than being coerced into a relation kind.

## Qualified relation evidence

Qualified relation evidence binds:

- exact assertion kind and direction;
- semantic-mapping digest;
- scientific/reference authority;
- qualification authority;
- exact preregistered relation design.

The semantic-mapping digest is domain-separated and includes the exact design and assertion.

A mapping digest remains an evidence identity, not self-authenticating scientific truth.

## Disagreement

`Disputed` preserves:

- candidate relation assertion;
- semantic-mapping digest;
- scientific/reference authority;
- qualification authority;
- separate dispute authority.

A disputed relation never exposes a dependency class and therefore cannot be counted as semantic independence.

## Local representation validation

Local validation proves structural/canonical integrity:

- supported versions;
- nonzero concept family versions;
- distinct canonical endpoints;
- nonzero scope/protocol/evidence authority revisions;
- exact built-in relation rule;
- valid symmetric/directional relation shape;
- exact embedded design digest;
- fail-closed missing-evidence policy.

Local validation does **not** independently re-derive external scientific/reference or qualification authority.

## Current authority

Persisted designs and evidence are Serde representations.

`ValidatedSpeciesConceptRelationDesign<'_>` requires exact fresh replay of:

- endpoints;
- scope;
- protocol;
- missing policy.

`ValidatedSpeciesConceptRelationEvidence<'_>` requires exact fresh replay of the complete current assessment.

Changing only relation qualification, dispute evidence, or semantic assertion stales current authority.

## Canonical identity rules

- caller endpoint order is non-semantic;
- canonical endpoint order is part of persisted representation;
- directional reversal is semantic and changes evidence identity;
- relation qualification drift changes evidence identity;
- scientific-reference drift changes evidence identity;
- dispute-authority drift changes evidence identity;
- scope/protocol drift changes design identity.

## Required adversarial properties

The V1 corpus establishes:

1. swapped endpoint input canonicalizes to the same design;
2. reversed directional relation remains distinct;
3. same-identity self-relations fail;
4. restored noncanonical endpoint order fails;
5. restored zero endpoint family version fails;
6. restored zero scope/protocol/qualification revisions fail;
7. symmetric relations cannot be directed;
8. directional relations cannot be marked symmetric;
9. nested and criterion relations remain dependency-visible;
10. overlap remains dependency-visible;
11. orthogonal/competing relations are only potentially non-nested;
12. disputed/unknown/unavailable relations never expose a dependency class;
13. fail-closed missing evidence fails;
14. qualification drift changes evidence identity and stales replay;
15. embedded design-digest tampering fails locally;
16. relation-rule tampering fails locally;
17. locally canonical semantic relabeling cannot regain current authority without exact replay.

## Explicit non-claims

SEL-10E1A3 does not:

- decide which species concept is philosophically correct;
- establish that two concepts are statistically or scientifically independent;
- establish evidence-source fault-domain diversity;
- establish qualification fault-domain diversity;
- decide a concrete species boundary;
- infer a historical speciation event;
- establish nomenclature;
- establish universal taxonomy truth;
- perform SEL-10E2 robustness aggregation.

Its sole purpose is to make semantic dependence explicit enough that later robustness logic cannot double-count nested concepts.
