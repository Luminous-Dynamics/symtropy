# Chromosome map authority V1

Status: CHROM-03C1 implemented/static; not executable-qualified.

## Purpose

`ChromosomeMap` is an optional exact sidecar authority over `HereditarySchema`.

The base hereditary schema catalogs loci and allowed alleles. It does **not** say that loci belong to chromosomes, have a biological order, or are linked. Those claims become available only when an exact chromosome map revalidates against the exact hereditary-schema digest.

Core theorem:

`locus catalog != chromosome/linkage authority`.

## Compatibility

C1 does not modify `HereditarySchema`, `HereditaryState`, population aggregates, reproduction V0, or `RecombinationMode::IndependentLoci`.

`HereditaryState` therefore remains explicitly unphased. Existing evolution histories that do not opt into a chromosome map keep their existing meaning.

A chromosome map alone also does **not** grant haplotype phase or crossover authority. Those require successor tranches.

## V1 types

- `ChromosomeId`
- `ChromosomeMapId`
- `GeneticMapPositionMicromorgans`
- `ChromosomeLocus`
- `ChromosomeDefinition`
- `ChromosomeMap`
- `ChromosomeMapDigest`

The two semantic IDs use the crate's validation-preserving Serde boundary.

## Coordinate convention

`ChromosomeLocus::position` has the dedicated type `GeneticMapPositionMicromorgans`, which wraps one `u64` integer genetic-map coordinate.

One Morgan is a genetic map-length unit; one micromorgan is one millionth of a Morgan. Integer coordinates are used so exact semantic identity does not depend on floating-point formatting or platform behavior. The dedicated position type also prevents callers from accidentally passing a physical sequence coordinate or another distance unit directly into the chromosome-map API.

These coordinates are **not** physical base-pair positions.

V1 does not define a direct map-distance-to-recombination-probability formula. A later recombination operator must declare that model explicitly.

## Canonical representation

The chromosome map stores chromosomes in a `BTreeMap` keyed by `ChromosomeId`.

Therefore chromosome declaration/insertion order is not biological meaning and cannot change canonical identity.

Within one chromosome, however, `ChromosomeDefinition::loci` is an ordered vector. That vector order is biological authority and is never sorted by the constructor.

Map positions must be strictly increasing in that declared order.

## Exact schema binding

A map stores both the hereditary schema ID and exact `HereditarySchemaDigest`.

Revalidation requires both to match the current schema. Reusing a schema ID after changing allele definitions, ploidy, locus membership, or other digest-bound schema content stales the chromosome map.

## Complete-partition theorem

A valid chromosome map must be a complete one-to-one partition of the hereditary schema's loci:

- at least one chromosome exists;
- every chromosome contains at least one locus;
- chromosome map key equals embedded chromosome ID;
- chromosome IDs are unique at validated construction;
- every mapped locus exists in the hereditary schema;
- no locus appears twice within or across chromosomes;
- every hereditary-schema locus appears exactly once;
- map positions are strictly increasing within each chromosome.

Unknown, duplicated, omitted, key-mismatched, empty, or non-increasing restored data cannot regain authority.

## Semantic digest

`ChromosomeMapDigest` is domain-separated and binds:

- chromosome-map version;
- chromosome-map ID;
- hereditary-schema ID;
- exact hereditary-schema digest;
- canonical chromosome set;
- ordered locus IDs within each chromosome;
- exact integer micromorgan coordinates.

The digest is computed only after current validation succeeds.

Serde/JSON bytes are transport, not semantic identity.

## Scientific claims C1 permits

With a currently validated `ChromosomeMap`, Symtropy may claim only that:

- these hereditary loci are assigned to these chromosomes;
- loci occur in this declared chromosome order;
- these integer genetic-map coordinates belong to the declared map authority.

C1 does not permit claims about:

- which allele copies share one homolog/haplotype;
- which exact chromosome copy came from which parent;
- crossover number or locations;
- recombination fractions;
- physical sequence/base-pair coordinates;
- chromosome segregation;
- gene conversion or structural variation.

## Successor design

CHROM-03C2 should add an optional phased hereditary-state sidecar bound simultaneously to exact `HereditarySchema` and `ChromosomeMap` authorities.

CHROM-03C3 should add a versioned diploid reference meiosis/crossover operator with semantic keyed draws and interval inheritance provenance.

CHROM-03C4 should qualify the model statistically/reference-differentially and expose interval inheritance for PHYLO-04 ancestry.

## Non-goals

No physical sequence simulation, centromere mechanics, transposons, gene conversion, structural variants, arbitrary polyploid meiosis, selection, ecology, reproductive isolation, speciation, or population-demographic change is introduced in C1.

## Qualification boundary

C1 remains source-reviewed/static until the exact head has Rust `cargo check`, tests, rustfmt and strict Clippy evidence. A source-level theorem or independently reasoned test corpus is not executable qualification.
