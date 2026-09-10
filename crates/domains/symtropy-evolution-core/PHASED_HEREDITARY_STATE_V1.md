# Phased hereditary-state authority V1

Status: CHROM-03C2 implemented/static; not executable-qualified.

## Purpose

`PhasedHereditaryState` is an optional exact sidecar that records which alleles occur together on the same chromosome homolog across the loci declared by `ChromosomeMap`.

It does not change the semantics of the existing unphased `HereditaryState`.

Core theorems:

`phased linkage state != unphased allele-copy state`.

`phase forgetting is deterministic; phase reconstruction is not`.

## Authority dependencies

A phased state binds simultaneously to:

- exact `HereditarySchema` ID and digest;
- exact `ChromosomeMap` ID and digest;
- the complete chromosome/haplotype content of the state.

Both upstream authorities must revalidate before the phased state or its digest can regain current authority.

## Representation

- `ChromosomeHaplotype` contains one allele per mapped locus, in chromosome-map locus order.
- `PhasedChromosomeState` contains one chromosome ID and an unlabeled multiset of complete haplotypes.
- `PhasedHereditaryState` contains one exact phased chromosome state for every chromosome in the map.
- `PhasedHereditaryStateDigest` is the domain-separated semantic identity.

C2 is euploid: every mapped chromosome has exactly `HereditarySchema::ploidy` haplotypes. Aneuploidy, chromosome-specific copy number, sex-chromosome systems, and copy-number variation are not represented by this authority.

## Canonical homolog rule

Phase is the association of alleles along one chromosome homolog. It is **not** an arbitrary identity for homolog row 0 versus homolog row 1.

Therefore validated construction sorts complete haplotype rows lexicographically as an unlabeled multiset.

Swapping two whole homolog rows does not change phased identity.

Moving an allele between homolog rows does change phased identity when it changes which alleles travel together, even if every locus still has the same unphased allele-copy multiset.

Alleles inside one haplotype are never sorted. Their vector order is fixed by `ChromosomeDefinition::loci`.

## Coupling/repulsion theorem

For two diploid loci A and B, these phased states are distinct:

- coupling: `[A0,B0] + [A1,B1]`
- repulsion: `[A0,B1] + [A1,B0]`

Their unphased projection is identical at both loci, but their phased digests differ.

This is precisely the information C2 adds over `HereditaryState`.

## Validation

Current validation requires:

- supported phased-state version;
- exact hereditary-schema authority;
- exact chromosome-map authority;
- phased chromosome set exactly equal to map chromosome set;
- state map key equals embedded chromosome ID;
- exactly `schema.ploidy` haplotypes per chromosome;
- canonical lexicographic whole-haplotype ordering;
- exactly one allele per mapped locus in every haplotype;
- every allele allowed by the corresponding hereditary locus.

Restored data is transport-shaped data only. Noncanonical homolog order, malformed chromosome keys, wrong copy counts, wrong allele counts, unknown alleles, changed maps, or changed schemas fail revalidation.

## Phase-forgetting projection

`PhasedHereditaryState::to_unphased(...)` deterministically forgets homolog associations and returns the existing `HereditaryState`.

For every mapped locus, it gathers the allele at that locus from each complete haplotype and delegates final unphased canonicalization to `HereditaryState::new`.

This projection preserves all information expressible by the older unphased authority, but necessarily discards phase information.

No inverse authority is provided. An unphased state generally corresponds to multiple phased states, so Symtropy must never invent one automatically.

## Semantic digest

The phased-state digest binds:

- phased-state version;
- hereditary-schema ID and exact digest;
- chromosome-map ID and exact digest;
- canonical chromosome set;
- canonical whole-haplotype multiset for each chromosome;
- allele sequence within each haplotype in map-locus order.

Serde/JSON byte layout is not semantic identity.

## Scientific claims C2 permits

A currently validated C2 state may claim that certain allele combinations coexist on the same homolog within one modeled hereditary state.

It does **not** establish:

- parental origin of a homolog;
- persistent chromosome-copy identity across generations;
- which homolog will enter a gamete;
- crossover count or position;
- recombination fraction;
- physical base-pair sequence;
- chromosome-specific copy-number variation;
- ancestry intervals;
- population haplotype frequencies unless separately represented;
- selection, ecology, reproductive isolation, or speciation.

## C3 successor

CHROM-03C3 should introduce an explicitly versioned reference meiosis/gamete operator. The first such operator may be diploid-only even though C2 storage supports the schema's broader euploid ploidy range.

C3 should consume exact schema + chromosome map + phased state + recombination authority and emit both a gamete and interval inheritance provenance suitable for PHYLO-04.

The operator must keep stochastic draw coordinates semantic and chromosome-local so adding an unrelated chromosome does not shift existing chromosome outcomes.

## Qualification boundary

C2 remains source-reviewed/static until the exact head has Rust `cargo check`, tests, rustfmt and strict Clippy evidence. Static reasoning is not executable qualification.
