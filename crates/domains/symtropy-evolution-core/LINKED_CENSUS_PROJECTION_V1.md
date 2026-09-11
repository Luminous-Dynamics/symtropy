# Linked census projection V1

Status: source/reference contract only. This document is not executable qualification evidence.

## Purpose

POPGEN-05A bridges exact linked genomes + mutation lineage into the existing aggregate `PopulationGeneticState` used by deep-time population processes.

Core theorem:

`declared explicit linked census -> exact aggregate allele counts`

while:

`aggregate allele counts != recoverable individual genomes or mutation history`.

## Declared census boundary

The caller supplies a `PopulationId` and a canonical multiset of validated MUT-05D subjects/multiplicities.

V1 treats that supplied multiset as the **declared census for this projection**. It does not discover ecological or real-world population membership independently.

## Exact outputs

`LinkedCensusProjection` contains:

- the existing `PopulationGeneticState`;
- the exact MUT-05D `MutationFateObservation` over the same subjects;
- census size;
- exact digests of both outputs;
- population identity and projection version.

## Allele consistency theorem

For every modeled locus and allele, the aggregate population's allele-copy count must equal the MUT-05D allele count exactly.

The aggregate population census equals the sum of subject multiplicities. Consequently every locus contains exactly `census * ploidy` allele copies under the existing population validator.

## Information-loss boundary

Projection into `PopulationGeneticState` intentionally discards:

- haplotype phase;
- chromosome-copy ancestry;
- assignment of alleles to exact individuals;
- mutation-origin identity;
- mutation predecessor history.

The paired MUT-05D observation retains mutation-origin count information, but neither output reconstructs the exact historical microstate from aggregate counts.

## Deep-time role

This is a resolution-transition seam:

`explicit linked organisms/history -> aggregate population genetics -> accelerated population process`.

Later promotion from aggregates to explicit organisms must create a qualified representative/new explicit state. It must never claim that aggregate counts preserved the exact historical individual arrangement.

## Restore-time authority

Serialized projection data is not current authority.

`validate_current(...)` reprojects the exact externally supplied population ID + current subjects and requires full structural equality.

Changing population identity therefore changes projection authority while leaving the underlying biological allele counts unchanged.

## Explicit non-claims

POPGEN-05A establishes no:

- external/ecological population-membership truth;
- exact individual reconstruction from aggregate counts;
- phenotype or developmental effect;
- fitness or selection coefficient;
- adaptation, sweep, reproductive isolation, or speciation.

## Required V1 qualification

- root linked census projects exact allele-copy counts;
- census equals sum multiplicities;
- duplicate multiplicity scales aggregate + mutation-fate counts identically;
- input order is irrelevant;
- recurrent same-allele/different-origin state collapses to one aggregate allele count while retaining separate MUT-05D origins;
- aggregate/fate allele counts agree for every locus;
- zero or overflowing census fails closed;
- source genome/ancestry/lineage drift fails before projection;
- Serde restore requires exact reprojection;
- changing `PopulationId` changes projection digest/authority but not allele counts;
- no individual reconstruction API exists.

## Successor boundary

Once this bridge is qualified, deep-time evolution can retain compact aggregate genetics for ordinary epochs and preserve mutation-origin fate summaries only where scientifically or causally useful.

Selection remains a separate layer that consumes explicit phenotype/ecology/reproductive consequences rather than rewriting projection history.
