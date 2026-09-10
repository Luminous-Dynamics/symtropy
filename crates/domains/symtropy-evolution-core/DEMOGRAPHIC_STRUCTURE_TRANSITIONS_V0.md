# Demographic Structure Transitions V0

Status: implemented/static authority contract; no genetic event execution is claimed.

## Purpose

DEMOG-04B2 separates population-membership consequences of a demographic event from the continuous migration matrix that applies after that event.

The central rule is:

`event-specific population membership != arbitrary future migration authority`.

A demographic event may require population identities to appear or disappear. It must not silently invent unrelated continuous migration changes.

## Authority

`DemographicStructureTransition` binds:

- exact `DemographicEventDeclarationDigest`;
- exact source `PopulationStructureProfileDigest`;
- exact successor `PopulationStructureProfileDigest`;
- event experiment identity;
- event generation.

It intentionally does not duplicate source/successor population sets inside its serialized receipt. Those sets are already canonical content of the bound structure profiles, and current validation recomputes the event-specific expected membership directly from those structures.

## Membership rules

For source set S:

- `CensusResize` => successor structure must be exactly identical to source structure;
- `PulseAdmixture` => successor structure must be exactly identical to source structure;
- `FounderEvent` => successor set is S plus the founded population;
- `Recolonization` => successor set is S plus the recolonized population;
- `Extinction` => successor set is S minus the extinct population;
- `PopulationSplit` => successor set is S minus the source plus exactly the daughter populations.

A V0 demographic structure transition may not produce an empty aggregate metapopulation.

## Migration boundary

Membership-preserving events require exact source/successor structure equality. This prevents a census resize or pulse-admixture declaration from smuggling in an unrelated migration-rate change.

Membership-changing events accept an explicit caller-supplied successor `PopulationStructureProfile` only when its population set exactly matches the event semantics. The transition binds the full successor structure digest but does not claim that any changed migration edges were caused by the event.

A later explicit structure-change authority should represent non-membership changes to continuous migration rates/edges when those changes have their own cause.

## Revalidation

A restored transition is evidence-shaped data until `validate_current(...)` succeeds against:

- the exact current demographic event declaration;
- current hereditary schema;
- exact source structure;
- exact source population states and trajectory points;
- exact source metapopulation snapshot;
- exact successor structure.

Changed event content, source structure, successor structure, experiment, or generation invalidates current authority.

## Runtime successor

The first demographic executor should consume both:

1. one validated `DemographicEventDeclaration`; and
2. one validated `DemographicStructureTransition`.

It should then produce post-event aggregate population state and a new generation-G metapopulation snapshot under the successor structure. The ordinary neutral/structured Wright–Fisher process can subsequently advance that post-event cut to G+1.

## Non-claims

This layer does not sample founders, downsample bottlenecks, mix alleles, mutate population state, generate post-event trajectory points, infer ancestry, prove ecological/geological cause, establish speciation, or alter canonical world time.
