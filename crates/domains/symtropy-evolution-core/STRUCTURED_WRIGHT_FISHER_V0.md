# Structured Wright–Fisher V0

Status: implemented/static; not executable-qualified.

## Purpose

This contract defines the first forward-time structured-population reference process in `symtropy-evolution-core`.

Its central identity is:

`structured migration = neutral reproduction/drift + explicit parental-source choice`.

It is deliberately a marginal allele-frequency model. It does not create migrant organisms, genotypes, haplotypes, pedigrees, kinship, ecological fitness, or ancestry that the aggregate population state does not contain.

## Required source authority

One structured generation consumes:

- the exact current `HereditarySchema`;
- one validated `PopulationStructureProfile`;
- the complete generation-G population-state map;
- the complete generation-G trajectory-point map;
- one `MetapopulationSnapshot` revalidated against those exact maps;
- one existing neutral `PopulationProcessProfile` using `NeutralIndependentLocusWrightFisher`;
- one `PopulationTransitionId`.

Experiment identity and generation are not separately supplied. They come from the validated source snapshot.

## Simultaneous-source rule

Every destination generation-G+1 population is derived from the immutable generation-G source cut represented by the snapshot.

No destination generated earlier in iteration order may become a parental source for another destination in the same generation.

This rule is semantic, not merely an implementation preference. Thread scheduling, map insertion order, ECS order, batching, or parallel execution must not change the result.

## Zero-migration theorem

After full structure/snapshot validation, an all-zero migration profile delegates every population directly to the existing `neutral_wright_fisher_step(...)` with the same source state, source point, transition ID, and process profile.

Therefore V0 requires exact/pathwise equality at `M=0`, not only equality in distribution or expectation.

The structured implementation must not duplicate the neutral draw grammar for this case.

## Nonzero migration semantics

For each destination population `j`, locus `l`, and destination allele-copy ordinal `c`:

1. draw one deterministic parental-source variate in `[0, 1_000_000)`;
2. map it through the canonical destination parental-source distribution from `PopulationStructureProfile`;
3. draw a second deterministic ordinal within the selected source population's generation-G allele-copy pool at locus `l`;
4. copy that allele identity into destination `j`;
5. repeat until the destination's own fixed census/ploidy copy count is filled.

The destination retains its own census. A source population with a different census changes the size of the within-source sampling pool; it does not replace the destination census.

## Stochastic coordinates

V0 uses two domain-separated SHA-256 semantic streams:

- `structured-parental-source-choice:v1`;
- `structured-source-allele-choice:v1`.

Both bind destination population, experiment, transition, generation, exact neutral process-profile authority, locus, destination-copy ordinal, and rejection-sampling attempt.

The within-source stream additionally binds the selected source population identity.

The migration matrix/digest is intentionally **not** included in the underlying source-choice variate. Migration rates instead move deterministic probability thresholds over a stable opportunity field. This permits controlled common-random-number parameter sweeps while exact structured-transition provenance still changes whenever structure authority changes.

## Probability mapping

Parental-source probabilities are integer ppm and sum to exactly 1,000,000 for every destination after adding the implicit self/stay probability.

Sources are traversed in canonical `PopulationId` order. Zero-probability sources are absent from the sampled row. Source-choice mapping therefore has one deterministic interpretation for one exact structure profile.

Both source-choice and within-source ordinal sampling use rejection before modulo reduction to avoid modulo bias.

## Result authority

`StructuredPopulationTransitionResult` contains:

- the complete destination population-state map;
- the complete destination trajectory-point map;
- one `StructuredPopulationTransitionProvenance` receipt for the simultaneous generation.

The receipt binds:

- exact hereditary-schema digest;
- exact population-structure digest;
- exact neutral process-profile digest;
- exact source metapopulation-snapshot digest;
- experiment identity;
- transition identity;
- generation G and G+1;
- exact destination state digest per population;
- exact destination trajectory-point digest per population.

Restored provenance is data until `validate_current(...)` revalidates source authority, destination authority, and deterministic re-derivation.

## Required V0 fixtures

Static fixtures cover or are intended to cover:

- exact zero-migration equivalence to independent neutral stepping;
- 100% cyclic two-population migration swapping opposing fixed allele pools, which detects accidental in-place within-generation updates;
- all destination points advancing exactly one generation in one experiment;
- structure-authority changes staling old provenance;
- symmetric and asymmetric one-generation migration-mixture expectations;
- paired migration-rate sweeps reusing the same source-choice opportunity field;
- destination census preservation;
- checkpoint/Serde restore followed by pathwise-identical continuation.

## Deliberate non-claims

V0 does not establish:

- literal organism migration;
- migrant mortality or transit cost;
- age/sex/stage-biased dispersal;
- genotype or haplotype covariance;
- linkage disequilibrium;
- chromosome crossover;
- pedigrees or exact ancestry;
- variable census or density regulation;
- population split/merge;
- pulse admixture;
- extinction/recolonization;
- ecological or sexual selection;
- speciation;
- canonical world time;
- accelerated deep-time closure validity.

Those require separate authorities and independently qualified successors.

## Optimization rule

This module is a correctness/reference lane. Faster multinomial, diffusion, GPU, cohort, or deep-time approximations may replace it only after demonstrating closure/fidelity against this explicit process over declared regimes. Optimization must not silently redefine stochastic coordinates or demographic meaning.
