# Demographic pulse-admixture execution V0

Status: implemented/static reference contract; not executable-qualified.

## Purpose

`PulseAdmixture` is one instantaneous generation-`G` aggregate genetic contribution from an existing source population into an existing destination population.

It is not continuous migration, population growth, or literal organism transport.

## Core theorems

`pulse admixture != continuous migration`

`aggregate genetic contribution != exact migrant organisms`

`admixture != hidden census growth`

The destination census is preserved exactly. The source population remains undepleted in V0. All unrelated populations remain exactly unchanged.

## Integer quantization

Current aggregate state stores integer allele-copy counts, so a ppm ancestry fraction must map to an integer number of marginal copies.

For destination census `N`, ploidy `P`, total copies `C = N * P`, and declared source fraction `f_ppm`, V0 defines:

`K = floor((C * f_ppm + PROBABILITY_SCALE_PPM / 2) / PROBABILITY_SCALE_PPM)`.

The implementation performs this arithmetic in widened checked integer space. Exact half-copy ties round upward.

The execution receipt records both the declared fraction and the realized `K`.

A small nonzero fraction may therefore quantize to `K = 0`. In that case biological state/trajectory/snapshot remain unchanged while the demographic history cursor still advances, preserving the distinction between a no-effect intervention and no intervention.

## Census-preserving replacement

At every independent locus V0:

1. chooses exactly `K` destination marginal copies to remove without replacement;
2. chooses exactly `K` source marginal copies to contribute without replacement;
3. subtracts the selected destination counts;
4. adds the selected source counts;
5. reconstructs the destination state at its original census.

The source state and source trajectory point are not modified.

V0 requires `K <= source_census * ploidy`. If the source does not contain enough marginal copies for the without-replacement reference model, execution fails closed. A future with-replacement or reproductive-contribution model must use a new model/version rather than silently changing semantics.

## Stochastic domains

Destination removal and source contribution use independent full-width SHA-256 priority domains.

### Destination-removal coordinates

- experiment identity;
- event identity;
- generation;
- destination population identity;
- locus identity;
- allele identity;
- within-allele destination-copy ordinal.

### Source-contribution coordinates

- experiment identity;
- event identity;
- generation;
- source population identity;
- destination population identity;
- locus identity;
- allele identity;
- within-allele source-copy ordinal.

The declared fraction and realized `K` are deliberately absent from both opportunity fields. A parameter sweep that holds experiment, event ID, generation, source and destination identities fixed therefore moves cutoffs over one common removal/contribution ordering. Changing the event ID intentionally creates a different stochastic event.

Exact event/provenance authority changes whenever the declared fraction changes even when the opportunity field is shared.

## Timing and population structure

Execution is an instantaneous intervention before reproduction from generation `G` and does not advance biological generation time.

`PulseAdmixture` is membership preserving. V0 therefore uses the same exact `PopulationStructureProfile` before and after execution. It does not create or modify a persistent migration edge.

## Provenance

`DemographicAdmixtureExecutionProvenance` binds:

- exact execution model;
- exact event declaration digest;
- exact demographic structure-transition digest;
- exact population-structure digest;
- source/result snapshot digests;
- source demographic-history cursor digest;
- experiment and generation;
- source and destination population identities;
- declared source fraction ppm;
- realized replacement-copy count `K`;
- source pre-event state and trajectory digests;
- destination pre-event state and trajectory digests;
- destination post-event state and trajectory digests.

Restored provenance is evidence-shaped data only. `validate_current(...)` revalidates all source authority and deterministically re-derives the complete result before accepting it as current.

V0 accepts an ordinal-zero history cursor only. Same-generation chaining waits for a later proof-bundle authority.

## Scientific fidelity boundary

Current state stores marginal independent-locus allele counts. V0 therefore does not establish:

- exact migrant/contributor organisms;
- source depletion;
- coherent multilocus ancestry blocks;
- haplotypes or chromosome tracts;
- genotype covariance;
- pedigrees/kinship;
- sex/age/stage bias;
- transit mortality;
- reproductive success;
- hybrid-species identity;
- ecological cause.

Although one common `K` is used at all loci, independently selected marginal copies do not imply the same coherent organisms contributed across loci.

## Required qualification fixtures

The reference implementation should demonstrate:

- destination census remains exact;
- source state/trajectory remain exactly unchanged;
- unrelated population state/trajectory remain exactly unchanged;
- quantized `K = 0` preserves biology while advancing history;
- realized `K` follows the documented rounding rule;
- 100% admixture reproduces source marginal counts when source and destination copy capacities match;
- source-capacity violation fails closed;
- fraction sweeps with fixed stochastic coordinates reuse one opportunity ordering;
- changed source/destination identity changes the appropriate stochastic field;
- changed event/source snapshot/structure transition stales provenance;
- Serde-restored receipts require deterministic revalidation;
- output remains in the same experiment/generation;
- non-root history cursors are rejected in V0.

## Optimization rule

A faster implementation may replace full sorting only if it preserves this exact model/version's selected-copy semantics for the same semantic priorities. Otherwise it must use a new execution model/version and qualify distributional or semantic equivalence explicitly.
