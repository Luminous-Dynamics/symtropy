# Demographic population-split execution V0

Status: implemented/static reference contract; not executable-qualified.

## Purpose

`PopulationSplit` represents one instantaneous aggregate population fragmentation at generation `G` before reproduction. It does not advance biological generation time and it does not establish speciation.

## Core theorems

`population split != speciation`

`pure fragmentation != hidden growth or mortality`

For source census `N` and canonical daughter censuses `N_i`, V0 requires:

`sum(N_i) == N`.

A daughter total below `N` is a split plus loss and must be expressed as ordered demographic events, for example bottleneck then split. A daughter total above `N` is a split plus growth and requires a future reproduction/growth authority before the split.

## Aggregate genetic conservation

At each independent locus, V0 enumerates every conceptual source allele copy exactly once, assigns a deterministic full-width SHA-256 priority, sorts the source copies by priority, and partitions the ordered copies into canonical daughter-capacity ranges of `N_i * ploidy`.

Consequently, for every locus:

- each daughter receives exactly its declared allele-copy capacity;
- no source allele copy is duplicated;
- no source allele copy disappears;
- the union of daughter allele-copy multisets exactly reconstructs the source allele-copy multiset.

The source population is removed from active state, the declared daughters are inserted, and all unrelated populations and trajectory points remain exactly unchanged.

## Stochastic coordinates

The V0 partition priority is domain separated and binds:

- experiment identity;
- demographic event identity;
- source generation;
- source population identity;
- locus identity;
- allele identity;
- within-allele source-copy ordinal.

Daughter declarations are canonicalized by population identity before authority minting. Reordering the same daughter declarations therefore does not create a different event or result.

Daughter census allocation is part of the exact event declaration/provenance authority. The source-copy opportunity field itself is independent of vector iteration order.

## Timing and trajectory semantics

The source snapshot and all daughter trajectory points are at the same experiment and generation `G`.

The split is a same-generation demographic intervention. Ordinary neutral/structured reproduction later advances `G -> G+1`.

## Provenance

`DemographicSplitExecutionProvenance` binds:

- exact split execution model;
- exact event declaration digest;
- exact demographic structure-transition digest;
- exact source and successor structure digests;
- exact source and result snapshot digests;
- exact source demographic-history cursor digest;
- experiment and generation;
- source population identity;
- source final population-state and trajectory-point digests;
- canonical daughter population identities with their resulting state and trajectory-point digests.

Restored provenance is evidence-shaped data only. It regains current authority only after source authority is revalidated and the complete split result is deterministically re-derived.

V0 accepts an ordinal-zero demographic history cursor only. Same-generation chaining requires a later proof-bundle/persistence authority that can revalidate the predecessor execution chain.

## Scientific fidelity boundary

Current `PopulationGeneticState` stores marginal allele-copy counts at independent loci. Therefore V0 does **not** claim that the per-locus partitions correspond to one coherent set of daughter organisms.

In particular it does not establish:

- individual survivor/migrant identity;
- multilocus genotypes;
- haplotypes or phase;
- chromosome/linkage conservation;
- pedigrees or kin groups;
- sex/age/stage composition;
- reproductive isolation;
- species identity;
- ecological or geographic cause of the split.

Chromosome/individual-resolution successors must own those richer claims.

## Required qualification fixtures

The reference implementation should demonstrate:

- daughter census sum unequal to the source fails closed;
- the source is absent after execution;
- only the exact declared daughters are introduced;
- every daughter has exactly `N_i * ploidy` copies at every locus;
- daughter allele-count unions reconstruct the source exactly at every locus;
- fixed alleles remain fixed in every daughter;
- unrelated populations and trajectory points remain exactly unchanged;
- daughter declaration order cannot change event/result identity;
- changed event/source snapshot/successor structure/structure transition stales provenance;
- Serde-restored receipts require deterministic revalidation;
- output remains at the same experiment and generation;
- a non-root input history cursor is rejected in V0.

## Non-goals

No speciation, reproductive-isolation threshold, ecology, selection, morphology, hidden population loss/growth, exact organism partition, chromosomes/linkage, ancestry, geographic cause, or world-time authority is introduced here.

## Optimization rule

A future faster implementation may replace the reference sorting algorithm only behind a new execution model/version or with evidence that it preserves the exact declared V0 stochastic mapping and canonical outputs. Performance equivalence alone is not semantic equivalence.
