# Chromosome recombination-process authority V1

Status: CHROM-03C3A implemented/static; not executable-qualified.

## Purpose

`ChromosomeRecombinationProfile` is the explicit process/applicability authority required before linked meiosis/crossover execution.

Core theorems:

`genetic map positions != complete recombination-process model`.

`phased hereditary state != meiosis authority`.

C1 says where modeled loci lie in genetic-map space. C2 says which alleles coexist on the same homolog. Neither authority says that a crossover occurred or defines the stochastic process that could produce one.

## Reference process

V1 exposes one deliberately narrow model:

`PoissonCrossoversNoInterferenceV1`.

Under this model, gametic crossover points are assumed to form a homogeneous Poisson process in the declared genetic-map coordinate. Genetic distance in Morgans is interpreted as expected gametic crossover count over an interval.

This is an explicit **no crossover interference** reference assumption. It is not asserted to be universal biology.

Pairwise recombination fraction is not stored as the primitive process parameter. Under a no-interference Poisson model, recombinant marker states arise from the parity of crossovers between markers; a pairwise map function therefore follows from the process rather than defining the full multilocus process.

## Why process domains are separate from locus positions

`ChromosomeMap` stores positions of modeled hereditary loci. Those outermost loci are not automatically chromosome/process endpoints.

Each `ChromosomeRecombinationDomain` therefore declares an explicit `GeneticMapIntervalMicromorgans` with positive length. The interval may start before the first modeled locus and/or end after the last modeled locus.

Every mapped locus on that chromosome must nevertheless lie inside the declared process interval.

This avoids silently collapsing unobserved chromosome span onto marker span and gives C3B an explicit domain over which to define crossover breakpoint semantics.

## Units

Process intervals use the same unit-safe `GeneticMapPositionMicromorgans` coordinates as C1.

The coordinates are genetic-map coordinates, not physical base-pair positions. V1 introduces no physical sequence coordinate system.

## Applicability

The V1 reference recombination process is explicitly diploid-only (`schema.ploidy == 2`).

C2 phased storage may represent broader euploid states, but that does not grant a meiosis model for those states. Polyploid meiosis requires a separate future authority.

## Exact authority binding

A profile binds:

- profile version and semantic profile ID;
- exact hereditary-schema ID and digest;
- exact chromosome-map ID and digest;
- recombination process model;
- canonical complete chromosome-domain set;
- exact start/end genetic-map coordinates for every chromosome process domain.

Changing the exact schema, map, process model, or any process interval changes or stales the authority.

## Canonical domain rules

- every mapped chromosome has exactly one process domain;
- no unknown, duplicate, or missing chromosome domains;
- map key must equal embedded domain chromosome ID;
- process interval length must be positive;
- every mapped locus must lie inside its chromosome's interval;
- domain declaration insertion order is non-semantic and is canonicalized by chromosome ID.

Transport restoration alone grants no authority. Malformed restored interval/key/set data must revalidate before digesting or execution.

`length_micromorgans()` itself validates before subtraction so hostile invalid restored intervals cannot gain an arithmetic underflow/panic path.

## Semantic digest

`ChromosomeRecombinationProfileDigest` is domain-separated and binds all exact profile authorities and canonical process intervals. It is computed only after current schema/map/profile validation succeeds.

Serde/JSON bytes are transport, not semantic identity.

## Scientific claims C3A permits

A validated profile may claim only:

- which explicit crossover process model is selected;
- that this model is applicable to the current diploid schema;
- the exact genetic-map process domain assigned to each mapped chromosome.

C3A does not establish that any crossover, gamete, recombinant haplotype, ancestry interval, or child exists.

## C3B execution successor

C3B should consume:

- exact `HereditarySchema`;
- exact `ChromosomeMap`;
- exact C2 `PhasedHereditaryState`;
- exact `ChromosomeRecombinationProfile`;
- exact reproduction/gamete semantic context.

It should emit one gamete plus revalidatable chromosome-local segment provenance.

Stochastic coordinates must be chromosome-local so adding an unrelated chromosome cannot shift existing chromosome outcomes. The exact source phased-state digest must bind any local homolog slot used in provenance; canonical row index is not persistent chromosome identity.

Breakpoint endpoint conventions and the exact integer realization of the Poisson process belong to C3B's versioned stochastic grammar, not C3A.

## Future models

Crossover interference, sex-specific recombination maps, hotspots represented in physical coordinates/rate maps, gene conversion, structural variation, chromosome-specific inheritance systems, and arbitrary polyploid meiosis require separate explicit models.

## Qualification boundary

C3A remains source-reviewed/static until its exact head has Rust `cargo check`, tests, rustfmt and strict Clippy evidence. Scientific rationale and source review are not executable qualification.
