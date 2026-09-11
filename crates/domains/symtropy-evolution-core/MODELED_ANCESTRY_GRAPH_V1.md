# Modeled ancestry graph V1

Status: reference/source contract only. This document is not executable qualification.

## Purpose

PHYLO-04D1 stores persistent ancestry-copy nodes and modeled-locus parent->child edges produced by PHYLO-04C in an exact append-only graph.

Its core theorem is:

`record exact ancestry first != simplify it at the same time`.

D1 never removes or contracts genealogy. PHYLO-04D2 owns simplification.

## Identity

`AncestryCopyId` is persistent genealogical identity.

It is not:

- a vector index;
- a renderer/entity ID;
- a compact table row number;
- a simplification-local remap ID.

D1 stores nodes keyed directly by `AncestryCopyId` and requires map key == embedded copy identity.

## Generation coordinate

D1 introduces `AncestryGeneration(u64)` as an explicit monotone genealogical coordinate.

It is intentionally distinct from rendering time and from any particular population-process generation contract.

For every ancestry edge:

`source.generation < child.generation`.

This strict rule makes cycles impossible in a valid V1 graph.

Independent reproduction events may create distinct child copies at the same generation.

## Nodes

`AncestryGraphNode` binds:

- persistent `copy_id`;
- chromosome identity;
- ancestry generation;
- optional reproduction birth event.

Roots/imported copies have `birth_event = None` and must have no incoming ancestry edges.

Descendant copies appended from PHYLO-04C have `birth_event = Some(ReproductionEventId)` and must have exactly one incoming source edge for every modeled locus on their chromosome.

## Edges

`AncestryGraphEdge` binds:

- chromosome ID;
- locus ID;
- source ancestry-copy ID;
- child ancestry-copy ID.

For every edge:

- both endpoints exist;
- both endpoints belong to the edge chromosome;
- the locus belongs to that chromosome in the exact current chromosome map;
- the source generation is strictly earlier;
- one child cannot have multiple incoming sources for the same modeled locus.

Edges are stored in strict canonical semantic order. Duplicate edges therefore fail canonical validation.

ParentA/ParentB role is not retained in the long-lived graph edge. Role is process provenance in PHYLO-04C. The role-specific descendant `AncestryCopyId` already distinguishes the resulting child copies, while the append receipt can replay the exact 04C process if role evidence is required.

## Root initialization

`ModeledAncestryGraph::new_roots(...)` accepts explicit root declarations and canonicalizes node input order through the node map.

It rejects:

- empty root sets;
- duplicate copy IDs;
- root declarations with reproduction birth events;
- unknown chromosomes;
- malformed current schema/map authority.

D1 does not infer root identity from C2 homolog row order.

## Append one reproduction

`append_descendant_ancestry_to_graph(...)` consumes:

- exact source graph;
- exact schema/map;
- both parents' exact phased genetic + PHYLO-04A ancestry authority;
- both parent-specific recombination profiles;
- both closed gamete derivation evidence values;
- both exact PHYLO-04B gamete-ancestry derivations;
- exact PHYLO-04C descendant ancestry derivation;
- explicit child `AncestryGeneration`.

The function fully revalidates PHYLO-04C from current upstream authority before graph mutation.

It then atomically:

1. creates every new child ancestry-copy node;
2. binds the exact reproduction event as each new node's birth event;
3. requires every modeled-locus source copy to exist in the source graph;
4. requires every source generation to be strictly earlier than the child generation;
5. appends modeled-locus source->child edges;
6. canonicalizes edge ordering;
7. validates the complete result graph.

Existing child IDs are rejected.

## Append provenance

`AncestryGraphAppendProvenance` binds:

- source graph digest;
- exact schema digest;
- exact chromosome-map digest;
- exact PHYLO-04C descendant provenance digest;
- child generation;
- canonical appended child IDs;
- canonical appended edge set;
- result graph digest;
- append algorithm version.

Restore-time `validate_current` deterministically replays the complete append from the exact source graph and upstream current authority.

## Canonical graph identity

Graph digest order is semantic:

- nodes are ordered by persistent `AncestryCopyId` through `BTreeMap`;
- edges are sorted by `(chromosome, locus, source copy, child copy)`.

Thus two independent same-generation reproduction events that reference the same source graph and are appended in opposite orders can converge to the same final graph identity, provided both are otherwise independent and valid.

Their individual append receipts remain different because their source graph digests differ.

## Scientific boundary

The graph is authoritative only for modeled loci.

It does not establish:

- physical genomic interval ancestry;
- exact crossover breakpoint locations;
- organism identity;
- mutation ancestry;
- species ancestry;
- ecological causality.

## Successor

PHYLO-04D2 defines focal/protected sample authority and deterministic graph simplification. D2 must preserve exact focal modeled-locus ancestry while pruning unreachable lineages and contracting removable unary transmissions.

The hard qualification gate is simplification-interval invariance: equivalent forward histories simplified every 1, 7, 31, 101 generations and only at the end must converge to equivalent final focal ancestry.