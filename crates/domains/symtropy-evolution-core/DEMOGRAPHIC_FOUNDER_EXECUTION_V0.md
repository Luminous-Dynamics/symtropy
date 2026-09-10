# Demographic Founder / Recolonization Execution V0

Status: implemented/static; not executable-qualified.

## Purpose

This contract defines aggregate-fidelity execution for `FounderEvent` and `Recolonization`.

Its central boundary is:

`sampled founder gene pool != literal emigrant organisms removed from the source`.

At the current representation level, Symtropy owns marginal allele-copy counts. It can initialize a new aggregate population from a sampled source gene pool, but it cannot identify exact emigrants, founder genotypes, haplotypes, kinship, sex/age composition, transit mortality, or source depletion.

## Required authority

Execution requires the exact current hereditary schema, source population structure, explicit successor population structure, source population states and trajectory points, source metapopulation snapshot, ordinal-zero demographic intervention cursor, founder/recolonization event declaration, and demographic structure-transition receipt.

The successor structure must already satisfy the event-specific membership theorem from `DEMOGRAPHIC_STRUCTURE_TRANSITIONS_V0.md`.

## Timing

The event is an instantaneous intervention before reproduction from source generation G.

The new population receives a generation-G trajectory point in the same stochastic experiment. Biological generation time does not advance.

## Non-depleting source theorem

The source population state and source trajectory point are copied into the result unchanged.

V0 therefore models a sampled propagule/gene-pool initialization. It does not assert that founder organisms were removed from the source population. A future organism-level dispersal authority may model depletion, transit survival, demographic stage, and exact ancestry.

## Founder sampling

For founder census F with `0 < F <= N`, where N is the source census, V0 samples exactly `F * ploidy` conceptual source allele copies without replacement at every independent locus.

Bottleneck and founder execution share one internal without-replacement selection mechanic. They retain separate domain-separated stochastic coordinate grammars.

Founder survivor priorities are full SHA-256 values keyed by:

- experiment identity;
- demographic event identity;
- generation;
- source population identity;
- destination population identity;
- locus identity;
- allele identity;
- ordinal within the source allele count.

Founder census is deliberately absent from the opportunity field, allowing nested common-random-number severity sweeps. Exact event and provenance authority still change when F changes.

## Census limit

V0 rejects `F > N`.

A founder population larger than the sampled source census requires different biological semantics, such as reproduction/growth after colonization or a declared with-replacement propagule model. The reference executor does not silently switch distributions.

## Result authority

`DemographicFounderExecutionResult` contains the complete post-event population map, complete trajectory-point map, successor-structure metapopulation snapshot, successor demographic history cursor, and one `DemographicFounderExecutionProvenance` receipt.

The receipt binds:

- exact founder execution model;
- exact event declaration digest;
- exact demographic structure-transition digest;
- exact source and successor structure digests;
- source and result snapshot digests;
- source history-cursor digest;
- experiment and generation;
- source and destination population identities;
- exact source state/trajectory digests;
- exact founded state/trajectory digests.

Restored receipts are evidence-shaped data until `validate_current(...)` revalidates all source/successor authority and deterministically re-derives the result.

## Required invariants

V0 is intended to qualify:

- source population state and trajectory remain exactly unchanged;
- exactly one event-declared new population appears;
- destination census and per-locus copy totals are exact;
- fixed source alleles remain fixed in the founder population;
- founder/recolonization targets are absent before and present after execution;
- result membership exactly matches successor structure;
- `F > N` fails closed;
- founder-census sweeps are nested under one common opportunity field;
- unrelated loci do not reroll focal-locus founder opportunities;
- source or destination identity changes the stochastic field;
- changed source snapshot/event/structure-transition/successor structure invalidates receipt authority;
- Serde-restored receipts require deterministic revalidation;
- output remains in the same experiment and biological generation;
- V0 accepts only a root demographic cursor.

## Scientific limits

V0 does not establish exact founder organisms, source depletion, whole-genome founder genotypes, chromosome linkage, haplotypes, pedigrees, kinship, age/sex/stage bias, transit mortality, ecological cause, population growth, admixture from multiple source populations, selection, speciation, or world-time authority.

## Optimization rule

This is a reference lane. Faster hypergeometric/multinomial/GPU/deep-time closures must earn equivalence over a declared domain. Optimizations must not silently change the non-depleting source semantics or stochastic coordinate grammar.
