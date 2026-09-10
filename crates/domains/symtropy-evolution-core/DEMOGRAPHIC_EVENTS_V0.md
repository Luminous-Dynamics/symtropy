# Demographic Events V0

Status: implemented/static authority contract; no event execution is claimed.

## Purpose

DEMOG-04B1 separates discrete population history from continuous gene-pool migration.

The central rule is:

`continuous migration != discrete demographic history`.

`PopulationStructureProfile` continues to own per-generation parental-source fractions. `DemographicEventDeclaration` records one explicit intervention such as a bottleneck, founder event, split, pulse admixture, extinction, or recolonization against one exact metapopulation source snapshot.

## Fidelity boundary

V0 operates on aggregate allele-copy population state. It may establish population-level demographic semantics, but it cannot establish:

- exact founder or migrant organisms;
- individual pedigrees or kinship;
- haplotypes or chromosome ancestry;
- sex-, age-, or stage-biased movement;
- migrant survival during transit;
- ecological causes;
- speciation by itself.

Population absence is structural. `PopulationGeneticState` remains non-empty. An extinction successor should remove the population from successor structure/state rather than fabricate a census-zero population.

## Event timing

All V0 declarations use `BeforeReproductionFromSourceGenerationV1`.

An event consumes an exact generation-G `MetapopulationSnapshot` and denotes an instantaneous intervention after that source cut has been established but before reproduction from generation G.

A future executor should therefore produce a post-event source cut still located at generation G. The ordinary population process then advances that post-event cut to generation G+1.

This gives an explicit order:

`source snapshot G -> demographic event(s) -> post-event snapshot G -> reproduction/migration -> G+1`.

If multiple events are allowed at the same generation in a later tranche, each event must bind the exact output snapshot of the preceding event. Event ordering must never be inferred from container or ECS iteration order.

## Event kinds

### Census resize

Changes the declared census of one existing population. Target census must be positive. Runtime semantics must distinguish stochastic downsampling from deterministic expansion rather than inventing allele copies silently.

### Founder event

Creates a new population identity from one existing source population. The founded population must not already exist and founder census must be positive. At aggregate fidelity this will mean sampling a new allele-count population from a source gene pool, not identifying literal founders.

### Population split

Replaces one existing source population with at least two new daughter population identities. Daughters are semantically unordered and therefore canonicalized by `PopulationId`; restored noncanonical ordering is rejected before canonical hashing/current validation.

A split declaration does not itself claim reproductive isolation or speciation.

### Pulse admixture

Declares an instantaneous one-time source contribution into an existing destination population. Source and destination must be distinct existing populations. The source fraction is fixed-point ppm in `1..=1_000_000`.

Pulse admixture is intentionally distinct from continuous migration.

### Extinction

Declares removal of an existing population from successor structure. It does not create a zero-census `PopulationGeneticState`.

### Recolonization

Creates a previously absent population identity from one current source population with a positive founder census. It is represented separately from a generic founder event so later historical queries can distinguish colonization of a new niche from return to a previously occupied region when higher-level world authority supplies that meaning.

## Source authority

A declaration binds:

- wire-safe `DemographicEventId`;
- declaration version;
- timing convention;
- complete event kind and parameters;
- exact hereditary-schema digest;
- exact population-structure digest;
- exact source-metapopulation snapshot digest;
- source experiment identity;
- source generation.

Changing any bound event parameter or source authority changes the exact declaration identity or makes current revalidation fail.

## Canonical representation

Validated construction canonicalizes unordered split daughters before authority minting and rejects duplicate daughter IDs before sorting can erase the conflict.

Local canonical validation is required before `canonical_digest()` will hash restored event-shaped data. This prevents malformed/noncanonical wire representations from acquiring a canonical digest merely because Serde accepted their shape.

Current authority is stronger than local canonicality: `validate_current(...)` also revalidates the exact schema, structure, source population states, trajectory points, and source snapshot.

## Runtime successors

Execution should be staged rather than implemented as one large demographic switch:

1. census resize / bottleneck reference operator;
2. founder/recolonization reference sampling;
3. pulse-admixture reference operator;
4. extinction and split with explicit successor-structure authority;
5. batch/event-chain authority for multiple ordered events at one generation.

Every executor should return revalidatable provenance binding its declaration digest, exact source snapshot, exact successor state/structure, stochastic experiment coordinates where sampling occurs, and post-event snapshot.

## Deep-time role

Discrete demographic events are permanent causal landmarks even when ordinary generations are later compressed or accelerated. A future phylogeny/provenance layer should be able to answer questions such as:

- which founder event created this population;
- which bottleneck removed most diversity;
- which split isolated these daughter populations;
- which pulse reintroduced alleles;
- when a population disappeared;
- whether recolonization came from the same or a different source lineage.

Those answers should be derived from event provenance, not reconstructed from present-day frequencies alone.

## Deliberate non-claims

DEMOG-04B1 does not execute events, alter a structure profile, create successor population state, prove canonical world time, infer geological/ecological causes, establish speciation, create ancestry, or validate accelerated deep-time closures.

It establishes only the event grammar and exact source-bound authority needed for those successors.
