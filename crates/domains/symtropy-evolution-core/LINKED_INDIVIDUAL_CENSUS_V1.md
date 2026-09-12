# Linked Individual and Explicit Census Contract v1

## Purpose

This contract introduces the minimum persistent organism identity required before
selection/consequence semantics may be attached to explicit linked organisms.

It does **not** introduce fitness, phenotype, ecology, mating, survival, or
reproduction-success semantics.

## Core distinctions

```text
individual identity
    != genome content
    != phased-haplotype row
    != ancestry-copy identity
    != mutation-lineage identity
    != aggregate population state
```

Two organisms may carry identical genomes and still be distinct individuals.
One individual may own many persistent chromosome-copy ancestry IDs. An
aggregate allele-count state cannot reconstruct the explicit member set.

## EvolutionIndividualId

`EvolutionIndividualId` uses the same validation-preserving semantic-ID wire
contract as the rest of evolution-core. Empty/whitespace identities fail during
Serde restoration before higher-level authority can be re-earned.

The ID is persistent reference identity. It is not a genome hash and is not
inferred from ancestry-copy ordering.

## LinkedIndividualManifest

A V1 manifest binds one `EvolutionIndividualId` to the exact current:

- hereditary-schema digest;
- chromosome-map digest;
- phased hereditary-state digest;
- phased ancestry-state digest;
- mutation-lineage-state digest.

The manifest duplicates none of those state bodies. A restored manifest is data
until `validate_current(...)` revalidates every exact current authority and all
bound digests.

Changing the current genome, ancestry sidecar, or mutation-lineage state stales
an old manifest even if the individual ID string is unchanged.

## LinkedIndividualSubject

`LinkedIndividualSubject` is a borrowed runtime tuple of:

- validated manifest;
- exact phased hereditary state;
- exact phased ancestry state;
- exact mutation-lineage state.

It may convert losslessly to one MUT-05D `MutationFateSubject` with multiplicity
exactly one. Explicit individual identity is not encoded as MUT-05D
multiplicity.

## ExplicitLinkedPopulationCensus

The census binds:

- exact `PopulationId`;
- exact schema/map authority;
- one canonical unique member set of
  `EvolutionIndividualId -> LinkedIndividualManifestDigest`.

Input ordering is non-semantic. Member ordering is canonical by individual ID.
Census size is the number of explicit unique members.

`validate_current(...)` requires an externally expected `PopulationId`; a
restored census cannot relabel itself into another population merely because
its member manifests remain valid.

## Exclusive ancestry-copy ownership

Inside one explicit census, one persistent `AncestryCopyId` may belong to only
one individual.

This prevents two different individual IDs from laundering the same persistent
chromosome copy into duplicate organism membership.

The rule is about current explicit census ownership, not about ancestry graph
parent/descendant edges across generations.

## Relationship to aggregate population genetics

A validated census can expose its members as multiplicity-one MUT-05D subjects
and therefore project through POPGEN-05A.

Two different explicit censuses may project to the same
`PopulationGeneticState` when their allele counts match. This is intentional:

```text
same aggregate allele counts
    != same explicit individuals
```

No inverse reconstruction API exists.

## Deliberate non-goals

V1 does not establish:

- ecological/residency membership truth beyond the declared census;
- organism birth/parentage proof from the individual ID itself;
- age or life stage;
- phenotype;
- viability or survival;
- mate choice or mating success;
- fecundity;
- offspring survival;
- scalar fitness;
- beneficial/deleterious mutation labels;
- selection coefficients;
- adaptation or speciation.

Birth/parentage remains represented by the existing reproduction/ancestry
receipts. A later lifecycle/consequence layer may bind those facts to a
validated `EvolutionIndividualId` when needed.

## Selection gate

A future selection model should consume exact explicit census identity and
record decomposed observed consequences. Frequency change alone must never be
back-projected into fitness evidence.

Recommended successor decomposition:

```text
validated individual + explicit census
        ↓
contextual phenotype/environment exposure
        ↓
consequence observations
  - survival
  - mating opportunity/success
  - fecundity
  - offspring survival
        ↓
actual reproductive contribution
        ↓
population trajectory
```

No `fitness: f32` shortcut is authorized by this contract.

## Evidence boundary

Implementation/source review is not executable qualification. This tranche is
stacked downstream of the POPGEN-05E neutral qualification corpus; queued CI
runs remain unexecuted until a runner actually performs their steps.
