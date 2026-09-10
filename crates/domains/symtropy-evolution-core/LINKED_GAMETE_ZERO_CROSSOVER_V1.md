# Linked gamete zero-crossover derivation V1

Status: CHROM-03C3B1 implemented/static; not executable-qualified.

## Purpose

C3B1 is the first execution layer that turns a validated phased hereditary state into a linked haploid gamete.

It consumes the exact C1 chromosome map, C2 phased hereditary state, and C3A recombination-process profile, but supports only:

`ChromosomeRecombinationModel::NoCrossoversIndependentAssortmentV1`.

No crossover point is generated in V1.

Core theorems:

`gamete state != derivation provenance`.

`zero crossover preserves one complete source haplotype per chromosome`.

`canonical haplotype slot != persistent chromosome ancestry identity`.

## Inputs

A derivation consumes exact current:

- `HereditarySchema`;
- `ChromosomeMap`;
- `PhasedHereditaryState`;
- `ChromosomeRecombinationProfile`;
- `ReproductionEventId`;
- `ParentRole::ParentA` or `ParentRole::ParentB`.

C3B1 requires diploid process authority. `ParentRole::ClonalParent` is rejected.

The Poisson crossover model is rejected by this executor rather than silently degrading to zero crossovers.

## LinkedGamete state authority

`LinkedGamete` contains exactly one complete `ChromosomeHaplotype` for every chromosome in the exact chromosome map.

The state binds exact schema and chromosome-map IDs/digests and validates each allele against the mapped hereditary locus.

The gamete state itself does not claim how it was produced. The same gamete content could arise from different events/processes. Derivation authority lives separately in `LinkedGameteDerivationProvenance`.

## V1 stochastic grammar

For a chromosome whose two source haplotypes differ, source slot selection is a single deterministic semantic bit from SHA-256 over the domain:

`symtropy:evolution:linked-gamete:no-crossovers-independent-assortment:v1`

followed by canonical encodings of:

1. exact reproduction-event ID text;
2. parent-role tag;
3. chromosome ID text.

The low bit of the first digest byte selects canonical local source slot 0 or 1.

No chromosome iteration ordinal is included.

Consequences:

- the same event/role/chromosome coordinate replays exactly;
- `ParentA` and `ParentB` have separate stochastic opportunity streams;
- adding an unrelated chromosome does not shift existing chromosome selections;
- chromosome container/insertion order does not affect existing choices;
- changing map coordinates or process-domain endpoints does not gratuitously reroll the zero-crossover source choice.

Exact provenance nevertheless binds the complete current chromosome-map and recombination-profile digests, so changing those authorities stales the receipt even when the zero-crossover stochastic opportunity is deliberately unchanged.

## Indistinguishable homolog rule

C2 homolog rows are an unlabeled canonical multiset, not persistent chromosome-copy identities.

If the two canonical source haplotypes for a chromosome are allele-for-allele identical, C3B1 canonicalizes the local selected slot to 0 instead of recording a meaningless stochastic distinction between two indistinguishable rows.

This does not claim that a physical organism had only one homolog. It means C3B1 lacks ancestry authority that could distinguish the two equal-content copies and therefore refuses to manufacture such identity.

## Whole-chromosome inheritance evidence

Every gamete chromosome has exactly one `WholeChromosomeInheritanceSegment` covering the chromosome's complete C3A process interval.

The segment binds:

- chromosome ID;
- exact process interval;
- local canonical source-haplotype slot;
- `ChromosomeHaplotypeDigest` of the selected allele content under exact schema/map authority.

The enclosing provenance additionally binds the exact source `PhasedHereditaryStateDigest`.

Therefore `source_haplotype_slot` is interpretable only as a coordinate inside that exact source state. It is not an organism-independent homolog ID, chromosome-lineage ID, or ancestry node.

The haplotype-content digest strengthens the evidence so local slot number is never the sole description of what was inherited. Equal-content source haplotypes intentionally have equal content identity.

PHYLO must introduce or consume separate persistent ancestry identity if it needs to distinguish physically different but allele-identical homolog copies.

## Derivation provenance

`LinkedGameteDerivationProvenance` binds:

- derivation version;
- exact hereditary-schema digest;
- exact chromosome-map digest;
- exact recombination-profile digest;
- exact source phased-state digest;
- reproduction-event ID;
- parent role;
- exact linked-gamete digest;
- canonical chromosome-local inheritance-segment set.

Its semantic digest is domain-separated and independent of Serde/JSON byte representation.

## Restoration / event-context authority

Deserializing a linked gamete or derivation receipt grants no derivation authority.

`validate_current(...)` requires the caller to supply the expected current `ReproductionEventId` and `ParentRole` in addition to schema/map/source/profile/gamete authorities.

The stored event/role must exactly equal those external expectations before deterministic replay.

This prevents a restored receipt from relabeling itself to another event/role that happens by chance to produce the same haplotype choices.

After authority checks, validation deterministically re-derives the gamete and inheritance segments and compares them with the restored evidence.

Tampered source phase, profile, gamete content, event context, source slot, interval, haplotype digest, or other bound evidence must fail closed.

## Scientific claims C3B1 permits

A successfully revalidated C3B1 derivation may claim:

- one linked haploid gamete was derived under the declared zero-crossover reference process;
- each gamete chromosome inherited one complete source haplotype across all modeled loci;
- the selected local source content and full process-domain segment are bound to the exact current source phased state;
- different chromosomes are sampled from chromosome-local independent-assortment opportunity coordinates.

It does not permit claims about:

- a crossover occurring;
- crossover interference;
- mutation during gametogenesis;
- persistent homolog/chromatid identity;
- parent organism identity beyond the supplied reproduction context;
- physical DNA/base-pair sequence;
- gene conversion or structural variants;
- exact ancestry graph nodes;
- child/zygote assembly;
- selection, ecology, reproductive isolation, or speciation.

## Successor design

### C3B1-child assembly

A narrow successor may combine one validated `ParentA` linked gamete and one validated `ParentB` linked gamete into a diploid phased child state, binding both gamete derivation receipts. Mutation should remain an explicit separate operator rather than being hidden inside chromosome segregation.

### C3B2 Poisson crossovers

C3B2 should implement `PoissonCrossoversNoInterferenceV1` with a separately versioned deterministic point-process grammar.

It should emit multiple ordered inheritance segments when crossovers occur while preserving the same source-state/content/interval provenance shape introduced here.

Exact replay should avoid unspecified platform-sensitive floating-point `ln`/`exp` behavior. The integer/fixed-point Poisson and breakpoint realization must be part of the versioned contract.

### PHYLO-04

PHYLO may consume inheritance segments and attach them to persistent ancestry entities. C3B1's canonical source slot is insufficient by itself to distinguish physically different but content-identical homologs.

## Qualification boundary

C3B1 remains source-reviewed/static until the exact head has Rust `cargo check`, tests, rustfmt and strict Clippy evidence. Static reasoning, deterministic hash calculations, and authored fixtures are not executable qualification.
