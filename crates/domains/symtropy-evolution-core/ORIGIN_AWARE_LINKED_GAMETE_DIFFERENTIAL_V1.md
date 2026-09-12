# Origin-Aware Linked-Gamete Differential V1

## Purpose

Qualification-only Oracle C for POPGEN-05E / #718.

This tranche asks whether the narrow neutral origin-partition law used by POPGEN-05C agrees with an explicit linked-genome transmission cell when the biological assumptions are deliberately aligned.

It does not modify production code.

## Exact explicit source

The qualification constructs one real diploid parent through the existing linked stack:

1. two homozygous modeled-locus root parents;
2. zero-crossover linked gametes;
3. modeled gamete ancestry;
4. diploid linked offspring;
5. descendant ancestry-copy materialization;
6. append into the modeled ancestry graph;
7. certain modeled-locus linked mutation on both descendant chromosome copies;
8. MUT-05C mutation-lineage derivation.

The resulting parent has:

- one modeled locus;
- one current allele fixed on both homolog copies;
- two persistent descendant `AncestryCopyId`s;
- two distinct active `MutationOriginDigest`s, one per persistent copy.

The test validates this mutation-lineage state before sampling.

## Explicit transmission lane

For each replicate the test derives **two independent real zero-crossover linked gametes** from that exact parent using distinct reproduction-event IDs.

For each gamete it then:

1. derives `ModeledGameteAncestry`;
2. reads the transmitted persistent `source_copy_id` at the focal locus;
3. resolves that copy through `MutationLineageState::entry(...)`;
4. reads the active mutation-origin digest.

The qualification therefore never infers ancestry from current allele value, canonical homolog row, or gamete content alone.

The two gametes are paired into a two-copy explicit transmission sample, producing an origin-count outcome `0`, `1`, or `2` for one frozen origin.

No new mutation occurs during this sampling epoch.

## Aggregate lane

The same exact mutated parent is projected through:

`MutationFateSubject -> LinkedCensusProjection -> OriginAwarePopulationState`.

The aggregate source must contain:

- current allele count `2`;
- modeled-baseline count `0`;
- exactly two active origins;
- count `1` for each origin.

Each replicate then runs one POPGEN-05C neutral aggregate transition from that fixed source.

Because the allele is already fixed, the allele destination remains exactly two copies; only the within-allele origin partition changes.

## Frozen ensemble and gates

The V1 corpus uses `4096` replicate pairs.

Both the explicit linked-gamete histogram and the aggregate origin-aware histogram must separately remain within absolute probability tolerance `0.04` of the analytical two-copy 50/50 null:

- `P(N=0) = 1/4`;
- `P(N=1) = 1/2`;
- `P(N=2) = 1/4`.

The total-variation distance between the explicit and aggregate histograms must be `<= 0.06`.

These replicate counts and tolerances are frozen before any exact-head execution result is observed and must not be widened afterward.

## Why this is not a whole-population sexual-model validation

This oracle intentionally validates only a **neutral transmission cell**:

- one diploid parent;
- two homolog copies;
- one fixed allele;
- two active mutation origins;
- two independently addressed gamete draws.

It does not model family-size variance, mate choice, sex ratio, overlapping generations, pedigree structure, population-wide gamete pools, migration, mutation during the epoch, linkage across multiple loci, or ecological selection.

Therefore agreement supports only this narrow transmission closure.

A mismatch is allowed to reveal an applicability problem. It must be classified rather than tuned away.

## Failure classes

A failure should be classified as at least one of:

- POPGEN-05C provenance-sampling defect;
- linked-gamete assortment defect;
- gamete-ancestry/persistent-copy resolution defect;
- mutation-lineage origin-resolution defect;
- deterministic semantic-RNG/domain-separation problem;
- fixture/qualification defect;
- finite-ensemble fluctuation;
- genuine model-assumption mismatch.

## Relationship to other POPGEN-05E oracles

- E1 checks the analytical multinomial/binomial law directly.
- E2 compares against an independent SplitMix64 reference sampler.
- E3 compares against the existing explicit linked-genome transmission machinery.

These are intentionally different evidence paths.

## Non-claims

A PASS does not establish:

- general Wright-Fisher equivalence of the explicit sexual simulator;
- arbitrary population structure;
- linkage/genotype equivalence;
- real mutation rates;
- ecological realism;
- fitness, selection, adaptation, or speciation.

## Evidence rule

Authored source is not execution evidence. E3 becomes evidence only when its exact product head executes under a captured subject/toolchain lane.
