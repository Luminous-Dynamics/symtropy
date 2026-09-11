# Modeled Gamete Ancestry V1

Status: implemented/source-reviewed only until exact-head Rust qualification executes.

## Purpose

PHYLO-04B bridges validated linked-gamete genetics into persistent ancestry identity at exactly the resolution currently modeled by the chromosome map: hereditary loci.

It does not add or infer ancestry for physical sequence between modeled loci.

## Core separation

V1 treats the following as distinct authorities:

- gamete allele state;
- gamete genetic derivation evidence;
- source persistent ancestry-copy identity;
- resulting modeled-locus ancestry;
- hidden physical crossover history.

Therefore:

`exact modeled-locus ancestry != exact hidden breakpoint history`.

## State

`ModeledGameteAncestry` binds one exact `LinkedGamete` and stores, for each mapped chromosome, one `ModeledLocusAncestry` record per mapped locus in exact chromosome-map order.

Each record contains only:

- exact `LocusId`;
- persistent source `AncestryCopyId`.

No interval endpoint, physical coordinate, crossover count, or unmodeled-sequence ancestry is claimed.

## Process authority

`GameteAncestryDerivationProvenance` binds:

- exact hereditary-schema digest;
- exact chromosome-map digest;
- exact source phased-genetic-state digest;
- exact source phased-ancestry-state digest;
- exact parent-specific recombination-profile digest;
- exact closed linked-gamete-derivation-evidence digest;
- exact linked-gamete digest;
- reproduction event ID;
- parent role;
- exact modeled-gamete-ancestry digest.

Serde restoration is evidence-shaped data only. Current authority is regained only by full deterministic replay against all exact current inputs.

## Distinct genetic homologs

When a diploid chromosome has two distinct complete haplotype contents, PHYLO-04A contains two singleton ancestry classes.

Existing validated genetic gamete evidence already determines which genetic content supplied each modeled locus. V1 therefore adds no genealogical RNG for those loci: the singleton class determines the persistent ancestry copy.

For zero-crossover evidence the selected complete haplotype's singleton ancestry copy supplies every modeled locus on that chromosome.

For marker-marginal Poisson evidence, the validated initial/source-after slots determine the singleton ancestry class at each modeled locus.

## Allele-identical ancestry copies

When both source homologs have identical complete genetic content, PHYLO-04A retains one content class with two persistent ancestry-copy IDs.

The genetic executor intentionally canonicalizes the local homolog slot because either homolog yields exactly the same allele state. That canonical slot is not genealogy.

V1 therefore uses a separate genealogical semantic bit for the initial persistent ancestry copy.

Frozen domain:

`symtropy:evolution:gamete-ancestry:initial-copy:v1\0`

Key coordinates:

- reproduction event ID;
- parent role;
- chromosome ID.

No mutable RNG state, chromosome iteration ordinal, locus iteration ordinal, or C2 row identity enters this choice.

The bit indexes the canonically ordered two-copy ancestry class.

## Marker parity with identical genetics

For marker-marginal Poisson evidence on allele-identical homologs:

- even parity preserves the current persistent ancestry copy;
- odd parity toggles to the other persistent ancestry copy.

This rule applies even though the genetic executor continues to report canonical source slot 0 and the allele sequence does not change.

Thus a chromosome may exhibit:

`same genetic state + different modeled-locus ancestry`.

That is expected and is the reason genetic and genealogical authorities are separate.

## Determinism

The genealogical choice is semantic-addressed by event/role/chromosome. Adding or reordering unrelated chromosomes must not reroll an existing ambiguous chromosome's initial ancestry-copy choice.

For distinct-content homologs no extra genealogical draw occurs.

## Validation

`ModeledGameteAncestry::validate_current(...)` proves exact schema/map/gamete binding and exact modeled-locus sequence coverage.

`GameteAncestryDerivationProvenance::validate_current(...)` additionally revalidates:

- source phased genetics;
- source PHYLO-04A ancestry authority;
- recombination profile;
- closed genetic gamete derivation evidence;
- event and parent role;
- deterministic ancestry replay.

A process-agnostic ancestry state alone is not proof of its derivation.

## Scope

V1 is diploid-only, matching the currently implemented linked-gamete execution models.

V1 does not define:

- organism or parent individual identity;
- mutation ancestry;
- full pedigree/lineage graph edges;
- exact physical breakpoint coordinates;
- ancestry for unmodeled sequence;
- genealogy simplification;
- tskit wire compatibility;
- species or population ancestry semantics.

## Successor

PHYLO-04C can convert validated modeled-locus ancestry into explicit parent/child ancestry graph edges. Canonical run compression may then be added as a representation optimization over exact modeled-locus origins, without changing the underlying authority or implying physical breakpoint precision.
