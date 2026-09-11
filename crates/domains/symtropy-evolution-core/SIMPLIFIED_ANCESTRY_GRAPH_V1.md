# Simplified modeled-locus ancestry graph V1

Status: reference/source contract for PHYLO-04D2B1.

## Purpose

V1 compresses a validated `ModeledAncestryGraph` relative to an exact
`AncestryRetentionSet` without changing persistent genealogical copy identity.
It is an end-only/reference transform. Continuing forward simulation from this
compressed representation belongs to PHYLO-04D2B2.

## Core semantic distinction

`AncestryGraphEdge` and `SimplifiedAncestryEdge` are intentionally different
semantic types.

A D1 `AncestryGraphEdge` means exact direct modeled-locus parentage created by a
reproduction event.

A `SimplifiedAncestryEdge` means only that its source is the nearest retained
ancestor of its child at that exact modeled locus after valid simplification.
Zero or more omitted chromosome copies may lie between the two endpoints.

A simplified edge therefore does **not** claim:

- direct reproduction;
- an exact crossover coordinate;
- a physical base-pair interval;
- ancestry between adjacent modeled loci;
- organism identity or species identity.

## Input authority

`simplify_modeled_ancestry(...)` requires:

- a currently valid D1 `ModeledAncestryGraph`;
- a currently valid D2A `AncestryRetentionSet`;
- the exact hereditary schema;
- the exact chromosome map;
- `FocalProtectedRootsAndBranchesV1`.

The transform first runs D2A complete-parentage pruning. D2B1 then performs its
stronger contraction independently at each modeled locus.

## Per-locus active ancestry

For each `(chromosome_id, locus_id)`:

1. seed the active set with focal/protected copies on that chromosome;
2. trace the unique D1 incoming edge backward at that locus;
3. ignore D2A closure edges that are not on those selected per-locus paths;
4. compute forward child degree in the active focal subgraph;
5. retain selected copies, reachable original roots, and nodes whose active
   forward degree is not exactly one;
6. bypass every other unary transmission copy;
7. connect each retained non-root copy to its nearest retained ancestor at the
   same modeled locus.

Retained-node requirements are then unioned across all modeled loci and
chromosomes.

A node may therefore remain globally because it matters at locus A while being
bypassed completely at locus B. This is expected and is why the compressed
representation is distinct from D1's whole-copy direct-parentage graph.

## Persistent identity

`AncestryCopyId` is never renumbered by simplification.

Container ordering, future compact table rows, and serialization layout are not
biological identity.

Node generation, chromosome identity, and birth-event metadata are copied from
the exact D1 source graph for every retained node.

## Validation

A current V1 compressed graph must prove at minimum:

- exact schema and chromosome-map binding;
- supported graph/profile version;
- canonical focal/protected retention authority;
- every retained focal/protected copy exists;
- node key equals embedded persistent copy identity;
- every node belongs to a known chromosome;
- simplified edges are in strict canonical order;
- every edge endpoint exists and belongs to the edge chromosome;
- every edge locus belongs to that chromosome;
- source generation is strictly earlier than child generation;
- at most one incoming simplified edge exists per `(child, locus)`;
- every focal/protected non-root copy has incoming ancestry at every modeled
  locus on its chromosome;
- every non-root node that participates as an ancestor at a modeled locus has
  its own incoming simplified ancestry at that locus;
- original roots have no incoming simplified edges.

The participating-node rule prevents a restored graph from retaining a branch
node below its descendants while silently severing that branch from its own
ancestor.

## Provenance

`AncestrySimplificationProvenance` binds:

- V1 derivation version;
- exact source D1 graph digest;
- exact schema/map digests;
- exact D2A pruning provenance digest;
- simplification profile;
- canonical retained persistent-copy IDs;
- exact simplified graph digest.

Restored provenance regains authority only by re-running the full D2A + D2B1
transform from the exact current source graph.

## Required reference theorems

V1 qualification includes:

- a pure unary chain contracts to `root -> focal`;
- a protected unary copy is retained and divides that path;
- a shared active branch copy for multiple focal descendants is retained;
- recombination can produce different retained roots/paths at different loci;
- a copy retained globally for one locus may be absent from another locus's
  compressed path;
- unreachable lineages removed by D2A remain absent;
- persistent ancestry IDs are unchanged;
- repeated end-only simplification from the same source/retention/profile is
  deterministic;
- restored results require exact replay;
- malformed branch ancestry fails structural validation.

## Non-goals

V1 does not support appending new births directly to a simplified graph and does
not establish simplification-schedule invariance. Those are PHYLO-04D2B2.

V1 also does not model mutation tables, physical sequence intervals, exact
breakpoint locations, organism pedigree, species history, or tskit wire-format
compatibility.
