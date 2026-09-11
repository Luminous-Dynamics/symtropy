# Ancestry simplification schedule invariance V1

Status: PHYLO-04D2B2B qualification contract.

## Core theorem

For one deterministic modeled-locus ancestry history, one final focal/protected
retention authority, and one simplification profile, changing only when
representation compaction runs must not change final authoritative ancestry.

V1 qualifies schedules:

- every generation;
- every 7 generations;
- every 31 generations;
- every 101 generations;
- end only.

Every lane performs the same ancestry births in the same order. Only calls to
`resimplify_simplified_ancestry(...)` move.

After the final generation every lane is resimplified under the same final
retention authority. V1 requires exact canonical simplified-graph equality and
digest equality, not merely distributional similarity.

## Deterministic reference history

The V1 qualification history contains one chromosome with two modeled loci and
four original ancestry roots:

- root A supplies the surviving lineage at locus A;
- root B supplies the surviving lineage at locus B;
- root dead seeds a temporary lineage that later becomes unreachable;
- root protected is retained only as an explicit historical anchor.

The history proceeds through:

1. independent A, B, and temporary unary lineages;
2. one modeled-locus mosaic descendant whose locus A source is the A lineage
   and whose locus B source is the B lineage;
3. continued unary descent of that mosaic lineage;
4. disappearance of the temporary lineage from the focal set;
5. a true branch producing two surviving final focal lineages;
6. long unary descent down both final branches.

This simultaneously exercises:

- unary contraction;
- locus-specific ancestry;
- unreachable-lineage garbage collection;
- protected-anchor retention;
- branch retention;
- forward append after prior simplification.

## Checkpoint and chunk boundaries

All lanes cross the same Serde checkpoint/restore boundary mid-history and
validate the restored compressed graph before continuing.

The deterministic transition sequence is also processed through nonuniform
logical chunks whose boundaries intentionally do not align with most
simplification schedules. Chunk boundaries must not affect graph state.

## Metrics

Each lane records:

- final graph digest;
- final node count;
- final edge count;
- peak node count before optional per-generation simplification;
- peak edge count before optional per-generation simplification;
- append-step count;
- simplification count.

The end-only lane is the unbounded-history reference during the run. Periodic
lanes must retain materially fewer irrelevant nodes/edges in this history while
converging to the exact same final graph.

V1 treats memory bounding as a qualification property, not an optimization
claim inferred from algorithm shape.

## Adversarial controls

Qualification also requires:

- perturbing the mosaic descendant's locus-B source changes final authority;
- changing final focal/protected authority changes final authority;
- omitting the protected root changes final authority;
- checkpoint restore does not;
- simplification schedule alone does not;
- persistent `AncestryCopyId` values are never renumbered;
- no compressed edge is reinterpreted as direct reproduction.

## Scientific boundary

This theorem applies to the modeled-locus ancestry representation only. It does
not establish physical base-pair ancestry, exact crossover positions, mutation
ancestry, organism pedigree, species history, ecological causality, or tskit
wire-format equivalence.

## Qualification status

The source test defines the hard gate, but no PASS exists until the exact branch
head actually executes under Rust check/test/rustfmt/strict-Clippy qualification.
