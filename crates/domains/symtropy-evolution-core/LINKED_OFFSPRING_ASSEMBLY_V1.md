# Diploid linked offspring assembly V1

Status: CHROM-03C3B1B implemented/static; not executable-qualified.

## Purpose

This tranche closes the first zero-crossover linked sexual reproduction loop.

It combines one exact ParentA linked gamete and one exact ParentB linked gamete, each revalidated from its own exact phased source under the same reproduction event and recombination profile, into the existing C2 `PhasedHereditaryState`.

Core theorems:

`two validated haploid gametes -> one diploid phased child state`.

`child genetic state != parentage history`.

`ParentA/ParentB role belongs to provenance, not canonical child homolog row order`.

## Inputs

`assemble_diploid_linked_offspring(...)` consumes current exact:

- diploid `HereditarySchema`;
- `ChromosomeMap`;
- `ChromosomeRecombinationProfile`;
- ParentA source `PhasedHereditaryState`;
- ParentA `LinkedGameteDerivation`;
- ParentB source `PhasedHereditaryState`;
- ParentB `LinkedGameteDerivation`;
- externally expected `ReproductionEventId`.

The ParentA gamete receipt must independently regain authority through C3B1 validation with `ParentRole::ParentA`. The ParentB receipt must independently regain authority with `ParentRole::ParentB`.

A raw `LinkedGamete` without its source-bound derivation receipt is insufficient parentage authority for this operation.

## Same-event theorem

Both contributions must belong to the same externally expected reproduction event.

Assembly does not infer an event from the two receipts, and restored receipts cannot relabel themselves to a different event merely because their gamete content happens to be compatible.

## Child assembly

For every mapped chromosome:

1. read ParentA's complete gamete haplotype;
2. read ParentB's complete gamete haplotype;
3. construct a two-haplotype `PhasedChromosomeState`;
4. allow C2 to canonicalize those whole haplotype rows as an unlabeled multiset;
5. assemble the complete C2 `PhasedHereditaryState`.

No stochastic draw occurs during this operation. Independent assortment already occurred when each gamete was derived.

No mutation occurs during this operation.

## Parent-role ordering is provenance only

A child chromosome whose ParentA contribution is lexicographically greater than its ParentB contribution may be stored with the ParentB-derived haplotype first after C2 canonicalization.

Therefore:

`child.haplotypes[0]` does **not** mean ParentA.

`child.haplotypes[1]` does **not** mean ParentB.

Parental contribution origin is retained only in `DiploidLinkedOffspringProvenance` through explicit ParentA/ParentB evidence.

This prevents canonical storage order from becoming false ancestry identity.

## Contribution evidence

`LinkedGameteContributionEvidence` binds for each role:

- exact `ParentRole`;
- exact source `PhasedHereditaryStateDigest`;
- exact `LinkedGameteDigest`;
- exact `LinkedGameteDerivationProvenanceDigest`.

Two contributions may have identical gamete content while remaining historically distinct because their validated derivation receipts carry different parent-role/event/source context.

At the same time, this layer still does not claim persistent parent organism identity unless an external/future authority supplies one.

## Child provenance

`DiploidLinkedOffspringProvenance` binds:

- derivation version;
- exact hereditary-schema digest;
- exact chromosome-map digest;
- exact recombination-profile digest;
- reproduction-event ID;
- exact ParentA contribution evidence;
- exact ParentB contribution evidence;
- exact child phased-state digest.

The provenance digest is domain-separated and semantic; Serde/JSON bytes are transport rather than identity.

## Restoration

Deserializing the child or provenance does not restore parentage authority.

`validate_current(...)` requires all exact current source/profile/gamete/event inputs again. It:

1. validates schema/map/profile authority;
2. checks the external expected event ID;
3. independently revalidates ParentA's gamete derivation as ParentA;
4. independently revalidates ParentB's gamete derivation as ParentB;
5. recomputes contribution evidence;
6. validates the supplied child and child digest;
7. deterministically reassembles the child and provenance;
8. requires exact agreement with restored evidence.

Wrong-role substitution, stale parent phase, changed profile/map/schema, event drift, gamete tampering, contribution tampering, or child tampering must fail closed through these existing authority checks.

## Allele-union theorem

For every mapped diploid locus, forgetting child phase must yield exactly the two allele copies supplied by the ParentA and ParentB linked gametes.

C3B1B introduces no allele, removes no allele, and changes no allele.

Thus this operation is composition, not mutation or selection.

## Selfing / source identity boundary

V1 does not require ParentA and ParentB source phased states to be different objects or different genetic states.

That is intentional: genetic-state equality does not establish organism identity, and some biological reproduction systems permit selfing.

Persistent organism identity, mating-system policy, sex systems, compatibility constraints, and kinship remain external/future authorities.

## Mutation boundary

The old independent-locus reproduction path may combine inheritance and mutation under its own legacy operator contract. This new linked child assembler does not reuse that hidden composition.

For linked genomes, mutation should be an explicit separately versioned operator/receipt so ancestry through recombination and sequence/allele change remain distinguishable causal mechanisms.

## Scientific claims V1 permits

After successful current revalidation, Symtropy may claim:

- these two exact validated linked gamete contributions were assembled under one reproduction event;
- every child chromosome contains exactly one gamete haplotype from each role;
- the canonical phased child state is the exact genetic composition of those contributions;
- the unphased child genotype is exactly the union of their allele copies.

V1 does not claim:

- persistent parent organism identity;
- that child row 0/1 indicates parental origin;
- crossover points;
- mutation;
- gene conversion or structural variants;
- physical DNA sequence;
- viability, development, phenotype, fitness, or survival;
- population membership;
- reproductive isolation or speciation.

## Successor

With the zero-crossover linked reproduction loop closed, CHROM-03C3B2 can add `PoissonCrossoversNoInterferenceV1` gamete derivation while reusing the same `LinkedGamete` and child-assembly surfaces.

C3B2 should change inheritance segments, not child composition semantics.

A separate mutation tranche should operate on phased/linked states with its own exact mutation authority and provenance.

PHYLO-04 can later attach persistent ancestry identities to parent contributions and inheritance segments without changing C2 child canonicalization.

## Qualification boundary

This tranche remains source-reviewed/static until its exact head has Rust `cargo check`, tests, rustfmt, and strict Clippy evidence. Authored fixtures and static review are not executable qualification.
