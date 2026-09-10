# symtropy-evolution-core

Dependency-light EVO-03 primitives for deterministic heredity and population genetics.

This crate implements the first runtime beneath the Living World heredity/reproduction contracts. It deliberately does **not** render organisms, infer fitness, simulate ecology, create species, or claim open-ended evolution.

## V0 rules

- `PhenotypeSeed`, genome content, parentage, reproduction-event identity, lineage, and persistent organism identity remain distinct;
- heredity is explicit semantic state under a versioned schema;
- stochastic choices are keyed from semantic identities rather than mutable/global RNG state;
- adding an unrelated locus does not consume or shift another locus's random stream;
- thread/ECS/iteration order cannot alter offspring derivation;
- mutation and recombination policies are explicit and versioned;
- V0 supports clonal reproduction and an intentionally simple independent-locus biparental model;
- aggregate population state records allele-copy counts without fabricating historical individual genomes;
- authority/state digests use a domain-separated canonical byte grammar rather than Serde bytes;
- every authority-bearing operation revalidates raw/deserialized inputs before use.

## Deliberate limits

V0 does not yet implement chromosome linkage/crossover, quantitative genetics, dominance, epistasis, gene regulation, evo-devo, speciation, ancestry compression, ecological selection, alternative biochemistry, or civilization.

Those should arrive as independently reviewable successors after the deterministic heredity substrate is qualified.

See the Living World `HEREDITY_PROVENANCE_V0.md` and `BIOLOGICAL_REPRODUCTION_V0.md` contracts on the parent stack, plus ASTRO-00 for scientific evidence/assumption semantics.
