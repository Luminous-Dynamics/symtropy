# SEL-10E1 Species-Concept Authority V1

## Purpose

SEL-10E1 introduces a model-family-neutral authority boundary around species concepts without weakening or rewriting the existing strict biological-species model.

Its core theorem is:

`one species concept != all species concepts != universal taxonomy`.

The first and only V1 adapter is the existing strict biological-species model. E1 does **not** invent additional species concepts merely to increase apparent model diversity.

## Claim boundary

E1 is a model-authority abstraction, not a species classifier.

It may describe:

- which species-concept family is represented;
- which exact semantic content defines that family instance;
- which validity domain applies;
- which source model and source qualification provenance are bound;
- which evidence requirements are declared;
- which downstream capabilities are supported.

It does not contain or decide:

- lineage A or lineage B;
- current species status;
- historical transition status;
- exact speciation time;
- taxonomic nomenclature;
- universal species truth.

Those remain downstream, evidence-bound claims.

## Two identity layers

E1 deliberately separates conceptual identity from authority identity.

### `SpeciesConceptIdentity`

Conceptual identity is:

- concept family ID;
- family version;
- semantic concept-content digest.

It excludes validity-domain and qualification identity.

Therefore:

- changing only source qualification does not create a new species concept;
- changing only validity domain does not create a new species concept;
- changing only a model instance ID does not create conceptual diversity unless semantic family/content changes.

This identity is orderable so downstream preregistered model sets can canonicalize and count distinct concepts without counting qualification/domain variants as independent concepts.

### `SpeciesConceptAuthorityDigest`

Authority identity additionally binds:

- model ID;
- exact validity-domain digest;
- exact embedded source model and source-model digest;
- source qualification provenance;
- exact capability/evidence/domain surfaces;
- exact adapter derivation-rule authority.

Qualification or domain drift therefore changes authority identity while preserving conceptual identity when semantic concept content is unchanged.

## Strict biological-species adapter

V1 adapts `ValidatedBiologicalSpeciesModel<'_>` into `SpeciesConceptAuthority`.

The adapter preserves the strict model's semantic content rather than broadening it.

The strict-BSC family exposes exactly two capabilities:

1. `CurrentSpeciesStatus`
2. `HistoricalTransitionInterval`

These are separate capabilities. Supporting current status does not collapse historical-transition evidence into the same theorem.

The frozen evidence requirements are:

1. explicit qualified model applicability;
2. current complete reproductive-barrier evidence;
3. persistent-or-recontact lineage history;
4. historical transition temporal evidence.

The V1 domain constraint is:

- reproductive isolation must be biologically meaningful for the target/domain.

An asexual or otherwise out-of-domain target must not be silently routed through this reproductive-isolation concept.

## Adapter rule versus source qualification

The generic adapter has its own built-in derivation-rule authority:

`strict-biological-species-concept-adapter-v1`.

The source model's qualification authority is preserved as provenance, but E1 does **not** claim that the source qualifier directly signed or endorsed the generic adapter transformation.

This separation prevents authority laundering across abstraction layers.

## Persisted versus current authority

`SpeciesConceptAuthority` is serializable evidence/representation.

Local validation recomputes:

- embedded source-model identity;
- family/version;
- capability surface;
- evidence requirements;
- domain constraints;
- semantic concept-content digest;
- concept validity-domain digest;
- source qualification binding;
- adapter-rule identity.

A restored serialized value cannot become current authority by hashing itself.

Current authority is carried only by non-Serde `ValidatedSpeciesConceptAuthority<'_>`, which requires fresh replay against a current `ValidatedBiologicalSpeciesModel<'_>`.

A requalified source model therefore invalidates current replay of an archived adapter even when conceptual identity is unchanged.

## Raw capability inspection

`SpeciesConceptAuthority::supports()` is a non-authoritative convenience inspection and is order-independent.

Authority validation still requires the exact frozen canonical capability vector. Reordering, deleting, or adding capabilities causes local validation failure even when raw membership inspection can still see a capability.

Downstream authoritative code should use `ValidatedSpeciesConceptAuthority::require_capability()` or `supports_capability()`.

## Diversity semantics for SEL-10E2

E1 intentionally provides the identity needed for future model-robustness reasoning.

Five authorities that differ only by:

- qualifier;
- validity domain;
- model instance identity;

but share the same strict-BSC family/version/content still count as **one conceptual species model**.

E2 must not treat authority multiplicity as conceptual independence.

Conceptual diversity and fault-domain independence remain separate requirements.

## Adversarial corpus

V1 covers:

- strict-BSC adaptation and exact current replay;
- qualification drift changes authority, not concept identity;
- validity-domain drift changes authority, not concept identity;
- qualifier/domain variants canonicalize to one `SpeciesConceptIdentity`;
- raw reordered capability inspection remains order-independent but authority validation fails;
- capability removal fails closed;
- adapter-rule tampering fails closed;
- stale replay against a requalified source fails;
- evidence-requirement tampering fails closed;
- domain-constraint tampering fails closed;
- wire shape contains no target status or historical outcome.

## Explicit non-claims

SEL-10E1 V1 does not:

- determine which philosophical species concept is correct;
- establish that the strict biological-species concept is universally applicable;
- provide a second independent species concept;
- establish cross-model robustness;
- perform majority voting;
- classify a concrete lineage pair;
- infer historical speciation by itself;
- create scientific truth from digests or authority references.

Its positive claim is narrower:

**This exact current qualified concrete species model maps, under this exact adapter rule, to this exact model-family authority, conceptual identity, validity-domain binding, capability surface, and evidence-requirement surface.**
