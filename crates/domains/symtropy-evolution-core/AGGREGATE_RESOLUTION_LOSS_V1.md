# Aggregate resolution loss V1

Status: source/reference contract only. This document is not executable qualification evidence.

## Purpose

POPGEN-05B makes information loss explicit when an exact POPGEN-05A linked-census projection enters the existing allele-only neutral Wright–Fisher aggregate lane.

Core theorem:

`exact linked projection at G -> exact allele-only aggregate transition to G+1`

while:

`exact mutation-origin fate at G+1 = not represented`.

## Existing process semantics are unchanged

V1 delegates the biological transition to the existing `neutral_wright_fisher_step` under `NeutralIndependentLocusWrightFisher`.

POPGEN-05B does not modify the drift RNG grammar, process profile, destination allele counts, or transition receipt.

It wraps that exact transition with a versioned resolution-loss certificate.

## Retained destination information

`AlleleOnlyNeutralIndependentLocusV1` retains:

- population identity;
- census size;
- exact per-locus allele-copy counts;
- experiment/generation trajectory position;
- exact aggregate transition provenance.

## Information no longer represented

After the allele-only aggregate step, V1 does not represent:

- exact individuals or genotype assignment;
- haplotype phase/linkage;
- persistent ancestry-copy identity;
- active mutation-origin partition/counts;
- mutation predecessor/history chains.

This is a statement about the representation's authority, not a claim that those biological facts ceased to exist.

## Source-history binding

The loss certificate binds:

- exact source `LinkedCensusProjectionDigest`;
- exact source MUT-05D observation digest;
- exact source aggregate population digest;
- exact aggregate transition provenance digest;
- exact destination population digest;
- versioned loss profile.

Therefore two source linked censuses may have identical aggregate allele counts and therefore produce the same allele-only destination under shared stochastic coordinates, while still producing different loss-certificate authority because their source mutation histories differed.

## Safety rule

The V1 result type deliberately contains **no destination `MutationFateObservation`**.

Copying, proportionally scaling, or otherwise guessing source mutation-origin counts after the allele-only Wright–Fisher step would fabricate provenance and is forbidden by this contract.

A future origin-aware aggregate process must be a distinct model with its own stochastic semantics and qualification.

## Restore-time authority

Serialized continuation/certificate data is evidence-shaped data only.

`validate_current(...)` re-executes from the exact current source projection, subject multiset, source trajectory point, transition identity, and process profile, then requires complete structural equality.

## Explicit non-claims

POPGEN-05B establishes no:

- mutation-origin fate after the aggregate step;
- exact individual/genotype reconstruction;
- phenotype/developmental effect;
- fitness or selection coefficient;
- adaptation, sweep, reproductive isolation, or speciation.

## Required V1 qualification

- wrapped destination equals direct `neutral_wright_fisher_step` exactly;
- retained/lost information profile is explicit and stable;
- destination has no mutation-fate field/API;
- source projection history is bound into certificate identity;
- wrong source projection, trajectory point, transition ID, or process profile fails replay;
- unsupported process models fail closed;
- Serde restoration requires exact replay;
- no selection/fitness semantics exist.

## Successor boundary

A separate POPGEN-05C may introduce an origin-aware neutral aggregate closure in which modeled baseline/origin classes are sampled as provenance subclasses nested under alleles.

That closure must sum exactly back to the ordinary allele counts and must be independently qualified against explicit linked ensembles. It must not alter the current Wright–Fisher V1 semantics.
