# Ancestry reachability pruning V1

Status: reference/source contract only. This document does not constitute executable qualification.

## Purpose

PHYLO-04D2A removes ancestry graph content that is unnecessary for a declared focal/protected copy set while preserving the exact D1 whole-copy graph contract.

Its core theorem is:

`reachability pruning != ancestry contraction`.

D2A never creates bypass edges. Every retained edge is an exact edge from the source D1 graph.

## Retention authority

`AncestryRetentionSet` contains:

- at least one focal `AncestryCopyId`;
- zero or more protected `AncestryCopyId`s;
- a version.

Validated construction sorts both sets and rejects duplicate identities and focal/protected overlap.

Restore-time validation requires strict canonical order, disjoint sets, and existence of every focal/protected ID in the exact source graph.

Changing the retention set changes exact pruning authority.

## Whole-copy closure rule

A D1 node is a complete modeled chromosome copy. Every retained non-root node must preserve exactly one incoming source edge for every modeled locus on its chromosome.

Therefore D2A uses complete-parentage closure rather than merely retaining the direct per-locus paths from focal samples.

If a node is retained for any reason, D2A also retains all of that node's incoming modeled-locus edges and all source nodes referenced by them. Newly retained non-root source nodes recursively require their own complete parentage.

This means D2A may conservatively retain ancestry at loci that are not themselves on a focal locus path. That is intentional: V1 keeps the unmodified D1 graph representation exact.

PHYLO-04D2B is the layer that later introduces modeled-locus bypass edges and stronger compression.

## Algorithm

1. validate exact source D1 graph and retention authority;
2. seed a work stack with focal and protected IDs;
3. retain each popped node;
4. if it is an original root, stop for that node;
5. otherwise retain every incoming D1 edge for the node;
6. push every source-copy endpoint;
7. repeat until closure reaches no new node;
8. clone exactly the retained source nodes and edges into the result;
9. validate the result through the unchanged D1 graph validator.

No node identity is renumbered. No edge is synthesized. No generation or birth-event field is modified.

## Receipt

`AncestryReachabilityPruningProvenance` binds:

- pruning version;
- source graph digest;
- exact schema digest;
- exact chromosome-map digest;
- retention-set digest;
- canonical retained-node IDs;
- result graph digest.

Restore-time validation deterministically reruns pruning from the exact source graph and retention authority and compares both result graph and receipt.

## Idempotence

Applying D2A to its own result with the same focal/protected set must return the identical graph and graph digest.

## Scientific boundary

D2A is conservative graph garbage collection only. It does not:

- contract unary ancestry transmissions;
- create synthetic source->descendant edges;
- change persistent `AncestryCopyId` values;
- infer physical genomic intervals;
- infer exact crossover breakpoints;
- retain organism/species/ecological identity.

## Successor

PHYLO-04D2B introduces a separate compressed-graph representation/authority where removable unary transmissions may be bypassed independently at each modeled locus.

D2B must prove simplification schedule invariance: the same deterministic forward history simplified every 1, 7, 31, 101 generations and only at the end must converge to equivalent final focal ancestry.