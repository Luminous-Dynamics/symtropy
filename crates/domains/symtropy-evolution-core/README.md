# symtropy-evolution-core

Dependency-light EVO-03 primitives for deterministic heredity and population genetics.

This crate implements runtime beneath the Living World heredity/reproduction contracts. It deliberately does **not** render organisms, infer ecological fitness, create species, or claim open-ended evolution.

## V0 authority rules

- `PhenotypeSeed`, hereditary content, parentage, reproduction-event identity, lineage, and persistent organism identity remain distinct;
- hereditary state binds both a semantic schema ID and the exact canonical schema digest, so same-name/different-content schemas cannot reinterpret old state;
- evolution-operator authority binds the complete versioned mutation and recombination profile rather than trusting labels alone;
- reproduction returns child hereditary content together with a revalidatable provenance record binding exact schema authority, exact operator authority, reproduction event, role-typed parent hereditary digests, reproduction mode, and child digest;
- deserialized provenance is data until `validate_current` succeeds against the exact current parents/schema/operators/child;
- semantic identity invariants survive Serde restoration: string-backed IDs deserialize only through their validated constructors rather than unchecked derived field construction;
- malformed empty/whitespace IDs therefore fail at the wire boundary, including when nested inside hereditary schemas;
- stochastic choices are keyed from semantic identities rather than mutable/global RNG state;
- adding an unrelated locus does not consume or shift another locus's random stream;
- thread/ECS/iteration order cannot alter offspring derivation;
- mutation-rate sweeps reuse the same underlying keyed mutation variate while still changing exact operator authority;
- V0 supports clonal reproduction and an intentionally simple independent-locus biparental model;
- aggregate population state records allele-copy counts and exact schema authority without fabricating historical individual genomes;
- zero-count aggregate allele entries are not state in V0: constructors canonicalize them away and raw/restored zero-count entries fail validation;
- authority/state/provenance digests use domain-separated canonical byte grammars rather than Serde bytes;
- every authority-bearing operation revalidates raw/deserialized inputs before use.

## Wire-safe semantic identity

Semantic IDs are not plain trusted strings. `HereditarySchemaId`, `LocusId`, `AlleleId`, `PopulationId`, `ReproductionEventId`, `OperatorProfileId`, `EvolutionExperimentId`, `PopulationTransitionId`, `PopulationProcessProfileId`, and `PopulationStructureProfileId` all share one constructor-enforced identity grammar.

V0 preserves the existing wire representation (a Serde newtype encoded as a string) but implements custom deserialization that routes restored text through `new(...)`. This closes the gap where derived `Deserialize` could construct whitespace-only IDs without invoking validation.

The rule is centralized in the semantic-ID macro so future containing structures do not need a second ad-hoc identity check. Later identity-grammar changes should remain centralized and versioned rather than being scattered across population, reproduction, persistence, trajectory, process-profile, or demography code.

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

## Trajectory authority

POPGEN-03B1 makes the temporal/stochastic position explicit:

`population state != that population state at generation G in experiment E`.

A `PopulationTrajectoryPoint` binds:

- exact hereditary schema authority;
- population identity;
- exact aggregate population-state digest;
- `EvolutionExperimentId`;
- `PopulationGeneration`.

Equal allele-count states may recur in different experiments or generations and therefore produce different trajectory-point identities.

Ordinary neutral continuation consumes a validated source trajectory point and returns both the destination population state and a new destination point at exactly `G + 1`. Transition provenance binds the source and destination point digests as well as the state digests, process profile, transition identity, and generation interval.

`PopulationTrajectoryPoint::declare_reference_start(...)` is deliberately named as an explicit free-standing/reference experiment start. It does **not** prove canonical world-history time. A future adapter to Living World / continuation authority must bind simulation time and inactive-world catch-up separately.

The static chain fixture advances the same trajectory continuously for 20 generations and as `0 -> 7 -> checkpoint -> 20`; both paths must produce the same final state and trajectory point. This is the first direct bridge toward bounded deterministic inactive-world catch-up without making work-budget chunking part of biology.

Trajectory points and their digests are canonical simulation identities/provenance, not cryptographic authorization tokens.

## Structured-population authority

DEMOG-04A1 adds the demographic contract that a later structured Wright-Fisher process can consume. It deliberately adds **no migration runtime yet**.

The central boundary is:

`aggregate parental gene-pool migration != exact organism migration`.

At the current population fidelity, Symtropy has marginal allele-copy counts. The structure authority can therefore describe how much of a destination population's next-generation parental gene pool comes from each other population, but it cannot establish which exact organisms moved, their genotypes, haplotypes, kin relationships, sex-biased dispersal, or individual ancestry.

`PopulationStructureProfile` binds:

- a wire-safe semantic profile ID and explicit version;
- a typed `PopulationStructureModel` describing matrix interpretation;
- an exact canonical declared population set;
- off-diagonal parental-source probabilities in integer ppm;
- an implicit self/stay probability equal to one minus the destination row's off-diagonal sum;
- a domain-separated canonical digest independent of declaration/insertion order.

`ParentalSourceEdge` names `destination` and `source` explicitly rather than relying on ambiguous `(from, to, rate)` tuples. Its meaning is always: a next-generation allele copy in `destination` chooses its parental gene pool from `source` with `rate_ppm` probability.

The validated constructor rejects duplicate population declarations and duplicate edges before ordered collections can erase that input history. It canonicalizes zero-rate input edges away. Raw/restored profiles containing zero-rate entries or empty migration rows fail validation, so one demographic model has one canonical representation. Self entries are forbidden because stay probability is implicit, and each off-diagonal row must total at most 1,000,000 ppm.

All future structured transition code must validate the complete profile before using convenience accessors or selecting a zero-migration fast path. A zero-migration runtime successor should delegate to the existing neutral reference process so the zero-migration theorem is exact/pathwise rather than merely distributional.

## Metapopulation source-snapshot authority

DEMOG-04A2 adds the missing simultaneity boundary between population structure and future structured stepping:

`a set of population states != one simultaneous metapopulation source state`.

`MetapopulationSnapshot` is a compact manifest, not another owner of biological state. It binds:

- exact hereditary-schema authority;
- exact population-structure authority;
- one explicit `EvolutionExperimentId`;
- one exact `PopulationGeneration`;
- the complete declared population set;
- for each population, its exact aggregate state digest and trajectory-point digest.

Capture requires the population-state map and trajectory-point map to match the structure profile exactly. Every embedded state identity must match its map key, every trajectory-point population identity must match its map key, every point must revalidate against the exact current population, and all points must occupy the same experiment and generation.

The snapshot digest is canonical over the ordered manifest, so source-map insertion order cannot change identity. Raw/restored snapshots remain evidence-shaped data until `validate_current(...)` succeeds against the exact current schema, structure profile, population states, and trajectory points. A restored manifest with altered digest bytes therefore cannot regain authority merely because Serde accepted its representation.

This provides the future structured transition with one immutable logical generation-G source cut. All generation-G+1 destinations must be derived from that cut before any destination can become a parental source for another destination. Iteration order, batching, or thread scheduling must never create within-generation causality.

This remains reference-trajectory authority, not canonical world-time authority. A later Living World continuation adapter must bind it to world time separately.

## Qualification fixtures

The neutral reference lane now has both one-step and long-run analytical fixtures.

One-generation ensemble checks cover:

- `E[p'] = p`;
- `Var[p'] = p(1-p)/(2N)` for the diploid reference fixture.

Long-run deterministic ensembles additionally check:

- neutral martingale behavior, `E[p_t] = p_0`, after multiple generations;
- expected heterozygosity decay, `E[H_t] = H_0 (1 - 1/(2N))^t`;
- ultimate neutral fixation probability matching initial allele frequency;
- absorbing allele-frequency boundaries remaining absorbing while trajectory time advances;
- replicate execution-order invariance for both one-step and multi-generation runs.

The martingale acceptance bound is derived from the analytical finite-generation Wright-Fisher variance and four standard errors for the declared ensemble. Fixation uses the analytical binomial sampling error around the neutral fixation probability. The heterozygosity fixture uses a deliberately conservative fixed band for the declared finite deterministic corpus.

These are deterministic qualification corpora: experiment IDs fix the realized sample set, so host scheduling cannot change the result. They remain regression evidence for this explicit reference implementation/profile, not proof that all later evolutionary models are correct.

Static fixtures also cover deterministic replay, transition revalidation, fixed-allele absorption, locus-order independence, exact copy-count conservation, stochastic experiment identity, aggregate canonicalization, trajectory-state revalidation, generation-sensitive trajectory identity, checkpoint/chunking invariance, semantic-ID JSON round-trip/rejection, structure-profile insertion-order invariance, migration-rate authority sensitivity, zero-edge canonicalization, duplicate-input rejection, row-sum bounds, implicit-stay arithmetic, complete metapopulation source-cut capture, source-set mismatch, mixed experiment/generation rejection, state/trajectory map-key mismatch rejection, structure/population staleness, source-map order invariance, and restored-manifest tamper rejection.

These tests are **not yet executable evidence** until an exact-head Rust toolchain run records their results.

## Module boundaries

The crate is split into small authority-focused modules:

- `schema` — hereditary grammar and exact schema identity;
- `heredity` — exact hereditary content bound to that grammar;
- `operators` — mutation/recombination model identity and exact operator authority;
- `reproduction` — deterministic offspring derivation and parentage provenance;
- `population` — aggregate allele-copy state only;
- `population_process` — narrow, versioned population transition models;
- `population_structure` — canonical metapopulation/parental-gene-pool structure authority;
- `population_trajectory` — revalidatable experiment/generation/state trajectory positions;
- `metapopulation` — compact simultaneous multi-population source-snapshot authority;
- `ids`, `canonical`, and `error` — shared semantic identity, canonical encoding, and fail-closed errors.

This structure is deliberate: chromosome linkage, ancestry, structured transition execution, ecological selection, speciation, and deep-time acceleration should extend narrow seams rather than grow a biological mega-module.

## Deliberate limits

V0 does not yet implement chromosome linkage/crossover, quantitative genetics, dominance, epistasis, gene regulation, evo-devo, structured migration execution, organism migration, ecological selection, phylogeny/speciation, ancestry compression, alternative biochemistry, world-time authority, rewind/branch DAGs, or civilization.

Those arrive as independently reviewable successors. In particular, the `IndependentLoci` models are reference profiles, not claims of universal inheritance or population biology, and `PopulationStructureProfile` is a demographic authority rather than evidence that any literal organism moved.

Population split/merge, pulse admixture, extinction/recolonization, and variable census should become explicit demographic-event authorities rather than being overloaded into the continuous parental-source matrix.

The current identity grammar only enforces the pre-existing non-empty/non-whitespace invariant. It does not yet impose Unicode normalization, case folding, global namespace policy, cryptographic authenticity, or a repository-wide ID standard.

## Qualification status

The current stacked implementation is **implemented/static only** until an exact-head Rust toolchain run establishes rustfmt/tests/strict-Clippy/check evidence. Repository workspace registration and `Cargo.lock` changes are intentionally deferred from the structural authority tranche.

See the Living World `HEREDITY_PROVENANCE_V0.md` and `BIOLOGICAL_REPRODUCTION_V0.md` contracts on the parent stack, ASTRO-00 for scientific evidence/assumption semantics, EVO-03A issue #414 for exact-authority hardening, #433 for validation-preserving semantic IDs, POPGEN-03B issue #417 for the reference-process qualification plan, POPGEN-03B1 issue #429 for trajectory-cursor hardening, POPGEN-03B3 issue #464 for long-run neutral qualification, DEMOG-04A issue #484 for structured-population evolution, and DEMOG-04A2 issue #490 for simultaneous metapopulation source authority.
