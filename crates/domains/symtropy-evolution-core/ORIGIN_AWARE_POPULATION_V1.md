# Origin-Aware Neutral Aggregate Closure V1

Status: source contract for POPGEN-05C / #713. This document does not constitute executable qualification.

## Purpose

V1 preserves a compact **current active mutation-origin partition nested under each allele** while advancing allele counts with the already-existing `NeutralIndependentLocusWrightFisher` reference model.

It does not modify the ordinary population process and it does not reconstruct an exact micro-history.

## Core theorem

For every modeled locus and allele after each transition:

`sum(destination provenance subclasses) == ordinary Wright-Fisher destination allele count`

exactly.

The ordinary allele transition is computed first by `neutral_wright_fisher_step`. Provenance subclass sampling happens only afterward and cannot alter that allele result.

## State

`OriginAwarePopulationState` contains:

- one exact `PopulationGeneticState`;
- for every nonzero allele bucket in that population:
  - modeled-baseline count;
  - zero or more active `MutationOriginDigest` counts.

Every provenance partition must sum exactly to the corresponding aggregate allele count.

One active origin digest may occur in only one locus/allele partition.

V1 intentionally does not retain:

- individual genotype assignments;
- haplotype phase or linkage;
- persistent ancestry-copy identity;
- mutation predecessor/history chains;
- extinct/dead micro-history.

## Exact initialization from linked history

`initialize_origin_aware_population_from_projection(...)` consumes an exact POPGEN-05A `LinkedCensusProjection` and its current MUT-05D subjects.

For each allele:

`baseline = allele_count - sum(active origin counts for that allele)`.

This derives a complete provenance partition without changing MUT-05D's wire shape.

## Neutral transition

`neutral_origin_aware_population_step(...)`:

1. validates the source state and source trajectory point;
2. requires `NeutralIndependentLocusWrightFisher`;
3. runs the existing `neutral_wright_fisher_step` unchanged;
4. for each nonzero destination allele bucket, samples current provenance subclasses conditional on that allele;
5. returns the origin-aware destination plus the exact ordinary transition and replay provenance.

No provenance draw can create, remove, or change a destination allele copy.

## Conditional provenance model

Within one source allele bucket, V1 treats modeled-baseline and active-origin classes as neutral exchangeable provenance subclasses.

For each destination copy already assigned to that allele by ordinary Wright-Fisher, V1 performs one unbiased bounded semantic draw over the source allele-copy count and maps it into the baseline/origin partition.

The semantic RNG binds:

- source origin-aware state digest;
- source trajectory-point digest;
- process-profile digest;
- transition ID;
- locus ID;
- allele ID;
- destination copy ordinal within the allele;
- rejection attempt;
- V1 RNG domain.

Bounded draws use rejection sampling over the 64-bit hash space; modulo bias is not accepted.

## Interpretation

V1 can represent neutral current-origin fate such as:

- persistence;
- stochastic loss/extinction of an origin while its allele survives through another origin or baseline class;
- competition between recurrent origins carrying the same allele;
- current-origin fixation within an allele bucket.

It cannot establish fitness, selective advantage, phenotype effect, sweep, adaptation, or exact individual genealogy.

## Recursive continuation

The destination `OriginAwarePopulationState` is itself valid input to the next aggregate generation. Therefore an origin-aware epoch may span many neutral generations without returning to explicit linked individuals.

## Relationship to POPGEN-05B

POPGEN-05B remains valid and unchanged: callers choosing the allele-only lane explicitly discard active-origin partition information.

POPGEN-05C is an optional higher-fidelity aggregate closure. It preserves more current provenance state at additional compute/storage cost.

Neither model is silently substituted for the other.

## Qualification requirements

Promotion requires exact-head execution proving at minimum:

- exact initialization from POPGEN-05A/MUT-05D;
- partition sum invariants;
- direct equality with ordinary Wright-Fisher allele destinations;
- all-baseline closure;
- recurrent origins remaining distinct provenance classes;
- deterministic origin loss while allele survives through another class;
- recursive multi-generation continuation;
- Serde restore + exact replay;
- drift in source state, trajectory point, transition ID, or process profile fails closed;
- no fitness or selection semantics are introduced.
