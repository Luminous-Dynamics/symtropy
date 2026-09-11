# Simplified ancestry forward V1

Status: PHYLO-04D2B2A reference/source contract.

## Purpose

V1 allows deterministic forward ancestry recording to continue after a D2B1
`SimplifiedAncestryGraph` has already compacted older genealogy.

The contract deliberately separates two operations:

1. append one exact descendant reproduction without deleting history;
2. resimplify under the next focal/protected retention authority.

This separation is required for the later simplification-schedule theorem.

## Append semantics

`append_descendant_ancestry_to_simplified_graph(...)` consumes:

- a current valid simplified graph;
- exact hereditary schema and chromosome map;
- fully current PHYLO-04C descendant ancestry evidence and all upstream current
  authorities needed to replay it;
- an explicit later `AncestryGeneration`;
- an explicit next `AncestryRetentionSet`.

Every 04C parental source copy referenced at a modeled locus must already exist
in the simplified source graph. Every descendant copy ID must be new.

New relations are stored as `SimplifiedAncestryEdge` values. Even when one such
edge happens to represent a direct one-generation inheritance, the persistent
compressed graph gives it only the weaker V1 meaning: nearest retained ancestor
at that modeled locus. Exact direct reproduction remains owned by the bound 04C
receipt.

Append changes the graph's retention authority to `next_retention`, but it does
not delete now-unneeded old nodes or edges. Compaction belongs to resimplify.

## Resimplification semantics

`resimplify_simplified_ancestry(...)` consumes an already-compressed graph and a
new canonical retention set.

For each modeled locus it:

1. traces active ancestry backward from the next focal/protected copies;
2. computes active forward child degree;
3. retains selected copies, original roots, and true branch nodes;
4. bypasses all other unary transmission copies;
5. unions retained node requirements across loci;
6. emits one canonical simplified graph under the same simplification profile.

Because source edges may already skip historical copies, this operation is a
contraction over ancestry-path statements rather than a replay of direct
reproduction edges.

## Next retention authority

The next retention set is atomic with append/resimplification. It may:

- promote newly born descendant copies to focal status;
- remove old parents from focal status;
- preserve explicit historical ancestry anchors through `protected_copy_ids`.

All selected IDs must exist in the relevant result/source graph. Retention
ordering is canonical; duplicates, focal/protected overlap, and empty focal sets
fail closed.

## Provenance

Append provenance binds:

- append version;
- exact source simplified graph digest;
- exact schema/map digests;
- exact PHYLO-04C descendant provenance digest;
- child generation;
- exact next retention authority;
- canonical appended child IDs;
- canonical appended simplified edges;
- exact result graph digest.

Resimplification provenance binds:

- resimplification version;
- exact source simplified graph digest;
- exact schema/map digests;
- exact next retention authority;
- canonical retained persistent-copy IDs;
- exact result graph digest.

Serde restoration never restores authority by itself. Both receipts regain
current authority only through deterministic replay.

## Invariants

V1 requires:

- source ancestry IDs used by new descendants already exist;
- child IDs do not already exist;
- source generation is strictly earlier than child generation;
- source graph is never mutated;
- append and resimplification preserve persistent `AncestryCopyId` values;
- resimplification is idempotent for an already-canonical graph under the same
  retention/profile;
- protected anchors survive resimplification;
- obsolete unreachable ancestry may disappear only during resimplification.

## Non-goals

V1 does not yet prove 1/7/31/101/end-only simplification schedule invariance.
That is PHYLO-04D2B2B.

It also does not claim physical sequence intervals, mutation ancestry, organism
pedigree, species history, distributed graph execution, or tskit wire-format
compatibility.
