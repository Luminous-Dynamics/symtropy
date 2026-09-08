# Heredity Provenance V0

## Status

Normative design contract for future genetics/reproduction. This document deliberately keeps PR #225's `PhenotypeSeed` as a deterministic derivation primitive rather than promoting it into genome or organism identity.

## Core distinction

The following are different concepts and must not silently alias:

```text
persistent organism identity
reproduction-event identity
genome / genotype content
lineage ancestry
phenotype derivation seed
presentation seed
```

A compact integer seed may help derive deterministic variation, but it is not automatically any of the other identities above.

## PhenotypeSeed boundary

The existing `PhenotypeSeed { lineage, individual }` is suitable as stable seed material for keyed deterministic trait variation.

It must **not** by itself prove:

- persistent organism identity;
- unique genome content;
- ancestry;
- parentage;
- causal birth history;
- cryptographic uniqueness.

Its current deterministic hash is intentionally a procedural variation mechanism, not an identity/security primitive.

## Genome authority

A future genotype should have explicit semantic content under a versioned genome schema.

Possible V0 representations include:

- a compact vector/map of named alleles/trait loci;
- a versioned generative genotype plus explicit parameters;
- another typed representation whose inheritance semantics are reviewable and deterministic.

The exact encoding is open.

What is not acceptable is:

```text
random_u64 == genome
```

with inheritance/recombination semantics hidden inside mutable implementation code.

## Genome identity

A genotype needs a stable semantic identity derived from or bound to its actual versioned content.

Conceptually:

```text
GenomeIdentity {
    genome_schema_version,
    content_identity,
}
```

The content identity may later be a canonical digest or another collision-resistant higher-layer identifier. `lifesim-core` does not need to own cryptography.

## Reproduction derivation

For sexual/recombinant reproduction, child genotype derivation should conceptually bind:

```text
parent A genome identity/content
+ parent B genome identity/content
+ reproduction-event identity
+ recombination scheme version
+ mutation scheme version
-> child genotype
```

For clonal/asexual reproduction, the parent set/mode changes but the same principle applies: reproduction is a versioned deterministic transformation, not unscoped runtime RNG.

## Reproduction-event identity

Two births from the same parents must be able to produce different deterministic offspring without relying on call order.

Therefore the derivation needs a stable birth/reproduction-event coordinate.

Before CRK/Level-I event identity is integrated, a higher layer may provide a provisional canonical reproduction coordinate. Once canonical event identity is available, it may become part of this provenance.

The reproduction event must not be frame number, ECS entity number, wall time, or renderer spawn order.

## Mutation

Mutation decisions are keyed to explicit reproduction/genome provenance and a versioned mutation scheme.

Adding a new independently keyed locus must not shift mutation decisions for existing loci.

Changing mutation probability or distribution is a biological model/version change.

## Environment vs heredity

Environmental development must not rewrite genotype merely because phenotype changed.

Keep the causal split explicit:

```text
genotype
   + developmental history
   + physiology
   + environment
-> phenotype
```

If epigenetic inheritance is later modeled, it becomes an explicit heritable channel with its own state, reset rules and reproduction semantics rather than an accidental copy of current phenotype.

## Parent phenotype boundary

Do not infer child genotype from a parent's current rendered/body phenotype when the underlying hereditary state exists.

Examples of non-heritable acquired state unless explicitly modeled otherwise:

- scar;
- broken limb;
- drought-induced branch loss;
- current hydration;
- temporary coat wetness;
- learned fear;
- current infection.

## Population coarse state

Far/coarse populations need not retain every exact genome if no active process requires it.

A qualified coarse representation may use genotype/allele distributions or other sufficient statistics.

Promotion from coarse genetic distributions to active individuals must be deterministic, conservative with respect to declared population statistics, and provenance-aware. It must not pretend reconstructed micro-genomes were historically observed.

Notable/persistent lineages may retain exact genotype even offscreen.

## Schema evolution

Changing any of the following requires explicit versioning/migration:

- locus meaning;
- allele encoding;
- recombination scheme;
- mutation scheme;
- genotype-to-phenotype interpretation;
- unit/scale of hereditary parameters;
- epigenetic inheritance semantics.

Old world state must not be reinterpreted under a new heredity grammar merely because field names still deserialize.

## Qualification fixtures

Future executable evidence should include:

1. same parent genotype(s) + same reproduction-event identity + same scheme -> same child genotype;
2. same parents + different reproduction-event identity can deterministically produce different offspring;
3. adding unrelated keyed loci does not shift existing-locus derivation;
4. current environmental phenotype does not mutate parent genotype;
5. save/reload preserves exact offspring derivation;
6. renderer/ECS ordering cannot alter child genotype;
7. recombination/mutation scheme-version mismatch fails closed;
8. genotype content identity changes when biological genotype changes;
9. coarse genetic materialization preserves declared aggregate statistics without claiming reconstructed micro-history;
10. persistent/notable lineage genotype survives fidelity collapse.

## Non-goals

V0 does not define:

- real chromosome biology;
- Mendelian vs polygenic parameter values;
- cryptographic digest format;
- persistent organism IDs;
- epigenetics implementation;
- mate choice;
- population genetics equations.

It freezes the rule that **heredity is explicit versioned biological provenance, while procedural phenotype seeds remain derivation coordinates rather than fake genomes or identities**.
