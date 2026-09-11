# Descendant ancestry materialization V1

Status: reference/source contract only. This document does not constitute executable qualification.

## Purpose

PHYLO-04C converts two fully revalidated parental linked-gamete ancestry derivations plus one exact V2 linked-offspring derivation into persistent ancestry identities for the child and modeled-locus parent->child inheritance edges.

The central theorem is:

`parent ancestry copy != child ancestry copy`.

A chromosome copy transmitted through reproduction descends from parental ancestry copies; the child receives a new persistent `AncestryCopyId` rather than reusing a parent copy identity.

## Inputs

V1 consumes the exact current:

- `HereditarySchema`;
- `ChromosomeMap`;
- ParentA phased source state, PHYLO-04A ancestry sidecar, recombination profile, closed linked-gamete evidence, and PHYLO-04B gamete-ancestry derivation;
- ParentB equivalents;
- one `DiploidLinkedOffspringDerivationV2`;
- externally expected `ReproductionEventId`.

Every upstream authority is revalidated before descendant ancestry is materialized.

## Child-copy identity

For each chromosome and parent contribution, V1 derives a fresh semantic `AncestryCopyId` from the frozen domain:

`symtropy:evolution:descendant-ancestry-copy:v1\0`

The derivation binds:

- reproduction event ID;
- parent role;
- chromosome ID;
- exact `GameteAncestryDerivationProvenanceDigest` for that contribution.

The resulting ID is encoded as `derived-v1:<sha256 hex>` and is deterministic/idempotent for the exact same contribution.

It is not derived from:

- child C2 row position;
- child genetic content alone;
- iteration order;
- mutable RNG state.

V1 rejects a derived child ID that collides with another child ID or with a parental source ancestry ID present in the modeled-locus edge set.

## Child PHYLO-04A sidecar

The two new child-copy IDs are materialized through the existing `PhasedAncestryState` authority.

For each chromosome:

- if ParentA and ParentB gamete haplotypes differ in complete modeled content, two singleton ancestry classes are created in the child's canonical genetic-content order;
- if the two gamete haplotypes are allele-identical, one content class contains both distinct child ancestry-copy IDs.

Thus parent role remains process provenance. It never becomes a claim that child canonical row 0 is ParentA or row 1 is ParentB.

## Modeled-locus inheritance edges

For every modeled locus and parent contribution, V1 emits:

`ModeledAncestryInheritanceEdge { chromosome_id, locus_id, parent_role, source_copy_id, child_copy_id }`.

`source_copy_id` comes from the fully revalidated PHYLO-04B gamete ancestry for that parent.

`child_copy_id` is the newly materialized descendant copy for that parent/chromosome.

A recombinant parental history such as `A -> B -> A` therefore becomes three modeled-locus edges from changing parental ancestry copies into one new child chromosome-copy target.

The edge set establishes ancestry only at modeled loci. It does not establish:

- physical sequence intervals;
- exact crossover count;
- exact breakpoint coordinate;
- ancestry between markers.

## Materialization canonical form

For each chromosome in canonical map order, `descendant_copies` contains exactly:

1. ParentA child copy;
2. ParentB child copy.

`edges` then contains, in canonical chromosome/locus order:

1. all ParentA modeled-locus edges for that chromosome;
2. all ParentB modeled-locus edges for that chromosome.

The materialization validates exact schema/map/event/child-state/child-ancestry authority, exact copy and edge counts, canonical role/order structure, globally unique child-copy IDs, and exact agreement between descendant IDs and the child PHYLO-04A sidecar.

## Provenance and restore semantics

`DescendantAncestryDerivationProvenance` binds:

- exact V2 offspring provenance digest;
- exact ParentA gamete-ancestry provenance digest;
- exact ParentB gamete-ancestry provenance digest;
- exact descendant materialization digest;
- derivation version.

Deserialization restores evidence-shaped data only.

`validate_current` revalidates:

1. V2 linked-offspring derivation;
2. ParentA PHYLO-04B derivation;
3. ParentB PHYLO-04B derivation;
4. child PHYLO-04A sidecar;
5. materialization structure/current authority;
6. all bound digests;
7. full deterministic re-execution of PHYLO-04C.

A restored object never regains current authority solely because its hashes or JSON parse successfully.

## Error propagation

PHYLO-04C preserves upstream authority boundaries explicitly:

- `EvolutionError` failures remain evolution failures;
- `AncestryAuthorityError` failures remain PHYLO-04A authority failures;
- `GameteAncestryError` failures remain PHYLO-04B authority failures.

They are not collapsed into a generic descendant mismatch.

## Non-goals

V1 does not model:

- organism identity;
- mutation ancestry;
- species identity;
- population pedigree;
- unmodeled sequence ancestry;
- physical crossover breakpoints;
- ancestry graph simplification;
- tskit compatibility.

## Successor

PHYLO-04D should persist these descendant-copy nodes and modeled-locus edges into a bounded ancestry graph with explicit focal/extant sample sets, then define a deterministic simplification receipt. Qualification must include simplification-interval invariance before that graph can be used as a deep-time memory closure.