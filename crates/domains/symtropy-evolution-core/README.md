# symtropy-evolution-core

Dependency-light EVO-03 primitives for deterministic heredity and population genetics.

This crate implements the first runtime beneath the Living World heredity/reproduction contracts. It deliberately does **not** render organisms, infer fitness, simulate ecology, create species, or claim open-ended evolution.

## V0 authority rules

- `PhenotypeSeed`, hereditary content, parentage, reproduction-event identity, lineage, and persistent organism identity remain distinct;
- hereditary state binds both a semantic schema ID and the exact canonical schema digest, so same-name/different-content schemas cannot reinterpret old state;
- evolution-operator authority binds the complete versioned mutation and recombination profile rather than trusting labels alone;
- reproduction returns child hereditary content together with a revalidatable provenance record binding exact schema authority, exact operator authority, reproduction event, role-typed parent hereditary digests, reproduction mode, and child digest;
- deserialized provenance is data until `validate_current` succeeds against the exact current parents/schema/operators/child;
- stochastic choices are keyed from semantic identities rather than mutable/global RNG state;
- adding an unrelated locus does not consume or shift another locus's random stream;
- thread/ECS/iteration order cannot alter offspring derivation;
- mutation-rate sweeps reuse the same underlying keyed mutation variate while still changing exact operator authority;
- V0 supports clonal reproduction and an intentionally simple independent-locus biparental model;
- aggregate population state records allele-copy counts and exact schema authority without fabricating historical individual genomes;
- authority/state/provenance digests use domain-separated canonical byte grammars rather than Serde bytes;
- every authority-bearing operation revalidates raw/deserialized inputs before use.

## Module boundaries

The crate is split into small authority-focused modules:

- `schema` — hereditary grammar and exact schema identity;
- `heredity` — exact hereditary content bound to that grammar;
- `operators` — mutation/recombination model identity and exact operator authority;
- `reproduction` — deterministic offspring derivation and parentage provenance;
- `population` — aggregate allele-copy state only;
- `ids`, `canonical`, and `error` — shared semantic identity, canonical encoding, and fail-closed errors.

This structure is deliberate: chromosome linkage, population stepping, ancestry, and ecology integration should extend narrow seams rather than grow a biological mega-module.

## Deliberate limits

V0 does not yet implement chromosome linkage/crossover, quantitative genetics, dominance, epistasis, gene regulation, evo-devo, demographic drift, migration, ecological selection, phylogeny/speciation, ancestry compression, alternative biochemistry, or civilization.

Those arrive as independently reviewable successors. In particular, the `IndependentLoci` model is a reference inheritance profile, not a claim of universal recombination biology.

## Qualification status

The current stacked implementation is **implemented/static only** until an exact-head Rust toolchain run establishes rustfmt/tests/strict-Clippy/check evidence. Repository workspace registration and `Cargo.lock` changes are intentionally deferred from the structural authority tranche.

See the Living World `HEREDITY_PROVENANCE_V0.md` and `BIOLOGICAL_REPRODUCTION_V0.md` contracts on the parent stack, ASTRO-00 for scientific evidence/assumption semantics, and EVO-03A issue #414 for the exact-authority hardening contract.
