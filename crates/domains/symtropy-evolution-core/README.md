# symtropy-evolution-core

Dependency-light EVO-03 primitives for deterministic heredity and population genetics.

This crate implements runtime beneath the Living World heredity/reproduction contracts. It deliberately does **not** render organisms, infer ecological fitness, create species, or claim open-ended evolution.

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
- zero-count aggregate allele entries are not state in V0: constructors canonicalize them away and raw/restored zero-count entries fail validation;
- authority/state/provenance digests use domain-separated canonical byte grammars rather than Serde bytes;
- every authority-bearing operation revalidates raw/deserialized inputs before use.

## Neutral population reference process

POPGEN-03B adds one deliberately narrow process model:

`NeutralIndependentLocusWrightFisher`.

It advances a fixed-census aggregate population by independently resampling allele copies at each unlinked/unphased locus. It is a **marginal allele-frequency reference model**, not an individual, genotype, haplotype, chromosome, ancestry, or ecology simulator.

Each transition binds:

- exact hereditary schema authority;
- source population identity and exact source-state digest;
- explicit `EvolutionExperimentId` identifying the stochastic realization;
- transition identity and generation coordinate;
- exact population-process profile authority;
- exact destination-state digest.

Independent ensemble replicates must use distinct experiment IDs. Paired counterfactuals may deliberately reuse one experiment ID when common random numbers are desired and all compared assumptions are recorded. Experiment identity is therefore explicit authority rather than an accidental naming convention.

The reference process is intentionally O(number of allele copies × loci) per generation. It exists first for correctness and analytical qualification; accelerated/coarse deep-time models must earn equivalence through the existing Living World fidelity/closure/shadow-validation machinery rather than silently replacing it.

## Qualification fixtures

Static fixtures now cover deterministic replay, transition revalidation, fixed-allele absorption, locus-order independence, exact copy-count conservation, stochastic experiment identity, aggregate canonicalization, and an integration-test ensemble for the one-generation Wright–Fisher mean and variance:

- `E[p'] = p`;
- `Var[p'] = p(1-p)/(2N)` for the diploid reference fixture.

The deterministic ensemble also reruns replicates in reverse execution order to verify host scheduling/order does not change per-replicate outcomes.

These tests are **not yet executable evidence** until an exact-head Rust toolchain run records their results.

## Module boundaries

The crate is split into small authority-focused modules:

- `schema` — hereditary grammar and exact schema identity;
- `heredity` — exact hereditary content bound to that grammar;
- `operators` — mutation/recombination model identity and exact operator authority;
- `reproduction` — deterministic offspring derivation and parentage provenance;
- `population` — aggregate allele-copy state only;
- `population_process` — narrow, versioned population transition models;
- `ids`, `canonical`, and `error` — shared semantic identity, canonical encoding, and fail-closed errors.

This structure is deliberate: chromosome linkage, ancestry, migration, ecology integration, speciation, and deep-time acceleration should extend narrow seams rather than grow a biological mega-module.

## Deliberate limits

V0 does not yet implement chromosome linkage/crossover, quantitative genetics, dominance, epistasis, gene regulation, evo-devo, migration, ecological selection, phylogeny/speciation, ancestry compression, alternative biochemistry, or civilization.

Those arrive as independently reviewable successors. In particular, the `IndependentLoci` models are reference profiles, not claims of universal inheritance or population biology.

## Qualification status

The current stacked implementation is **implemented/static only** until an exact-head Rust toolchain run establishes rustfmt/tests/strict-Clippy/check evidence. Repository workspace registration and `Cargo.lock` changes are intentionally deferred from the structural authority tranche.

See the Living World `HEREDITY_PROVENANCE_V0.md` and `BIOLOGICAL_REPRODUCTION_V0.md` contracts on the parent stack, ASTRO-00 for scientific evidence/assumption semantics, EVO-03A issue #414 for exact-authority hardening, and POPGEN-03B issue #417 for the reference-process qualification plan.
