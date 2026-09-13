# SEL-10E1A2 Open Species-Concept Descriptor V1

## Purpose

This crate provides an open, family-neutral descriptor waist above concrete species-concept authorities.

Core theorem:

`family-specific authority != open descriptor != current scientific authority != cross-model robustness`.

The descriptor exists so future concept families can participate in common model-set, semantic-relation, and robustness machinery without forcing all of their evidence criteria into the strict biological-species enums owned by `symtropy-evolution-core`.

## Layering

V1 intentionally uses this dependency direction:

`symtropy-evolution-core -> symtropy-species-concept`

Future concrete species-concept implementations may project into `symtropy-species-concept`, while the low-level evolution evidence crate remains unchanged.

The open descriptor must not create a dependency from `symtropy-evolution-core` back to higher-level concept families.

## Open conceptual identity

`OpenSpeciesConceptIdentity` contains only:

- stable family ID;
- family version;
- semantic content digest.

The strict-BSC projection preserves the exact family string, version, and concept-content digest bytes from SEL-10E1.

Qualification and validity-domain drift therefore remain outside conceptual identity.

## Opaque source authority

`SpeciesConceptSourceAuthorityRef` is intentionally family-neutral. It binds:

- source-authority kind ID/version;
- exact source-authority digest;
- source qualification provenance;
- exact source validity-domain digest;
- source adapter/derivation-rule authority.

The open waist does **not** know how to prove arbitrary source-family semantics from that reference.

This is deliberate.

### Local descriptor validity

Local validation proves structural/canonical integrity only:

- version validity;
- canonical capability order and uniqueness;
- canonical schema order and term uniqueness;
- schema-content digest integrity;
- distinct evidence/domain schema roles;
- exact open-waist descriptor rule.

It does **not** prove that an arbitrary opaque source-authority digest is scientifically current or truthful.

Changing only the opaque source digest can therefore produce a different locally canonical descriptor representation.

### Current scientific authority

Current authority requires a source-specific replay path.

V1 provides:

`ValidatedSpeciesConceptFamilyDescriptor::validate_current_strict_biological(...)`

This recomputes the entire descriptor from a fresh current SEL-10E1 `ValidatedSpeciesConceptAuthority<'_>` and requires exact equality.

Thus source-authority drift, source qualification drift, source validity-domain drift, schema drift, capability drift, or adapter-rule drift all stale the old descriptor even if its serialized envelope remains structurally valid.

Future concept families must add equivalently source-specific current replay rather than gaining authority from the generic envelope constructor.

## Open schemas

Evidence requirements and validity/domain constraints are represented as open canonical schemas:

- `SpeciesConceptSchemaId`;
- schema revision;
- sorted unique `SpeciesConceptSchemaTerm` values;
- exact schema content digest.

Each term is identified by an open ID, revision, and content digest.

This avoids a global enum that must accumulate criteria from every future species concept.

Schema terms are family-owned contracts. Their digests establish exact identity, not scientific truth by themselves.

## Generic capabilities

Downstream capabilities are represented as open capability references with:

- capability ID;
- revision;
- content digest.

The strict-BSC projection currently maps exactly:

- `CurrentSpeciesStatus` -> `current-species-status`;
- `HistoricalTransitionInterval` -> `historical-transition-interval`.

Capability lists are sorted and duplicate IDs fail closed.

## Strict-BSC projection

The V1 strict-BSC projection consumes only current SEL-10E1 authority.

It preserves:

- exact conceptual family identity;
- exact SEL-10E1 current authority digest as source authority identity;
- exact SEL-10E1 source qualification provenance;
- exact SEL-10E1 validity-domain digest;
- exact SEL-10E1 adapter-rule authority;
- exact capability surface;
- deterministic evidence/domain schema projections.

Strict-BSC evidence schema terms are:

1. `explicit-qualified-model-applicability`;
2. `current-complete-reproductive-barrier`;
3. `persistent-or-recontact-lineage-history`;
4. `historical-transition-temporal-evidence`.

Its domain schema currently contains:

- `reproductive-isolation-biologically-meaningful`.

No SEL-10E1 canonical identity is changed by this projection.

## Canonicalization

Generic declaration canonicalizes schema terms and capability references before persistence.

Restored serialized descriptors must already be in canonical order. Reordering a persisted vector therefore fails local validation rather than silently re-normalizing historical bytes.

This keeps representation identity deterministic while permitting caller input order to be non-semantic at construction time.

## Identity versus currentness

A descriptor digest means:

**these exact descriptor bytes and bindings under this exact open-waist rule**.

It does not mean:

- the source concept is scientifically correct;
- the source qualification remains current;
- the source evidence remains current;
- the species concept is independent of another concept;
- any concrete species result is true.

This distinction is mandatory for SEL-10E1A3 semantic relations and SEL-10E2 robustness.

## Semantic independence is out of scope

Different `OpenSpeciesConceptIdentity` values are not automatically independent.

Semantic dependence/generalization/refinement is handled by SEL-10E1A3. Evidence-source and qualification-process dependence are separate downstream fault-domain questions.

The open descriptor therefore exposes identity and provenance, not an `independent=true` bit.

## Adversarial requirements

V1 must demonstrate:

- strict-BSC projection preserves conceptual identity exactly;
- exact current source authority digest is preserved;
- qualification drift changes descriptor/source-authority identity without manufacturing conceptual diversity;
- validity-domain drift changes descriptor/source-domain identity without manufacturing conceptual diversity;
- generic constructor canonicalizes caller ordering;
- persisted capability reordering fails local validation;
- persisted schema reordering fails local validation;
- duplicate capability IDs fail closed;
- duplicate schema term IDs fail closed;
- schema digest tampering fails closed;
- descriptor-rule tampering fails closed;
- source-authority digest tampering changes representation identity but cannot obtain current strict-BSC replay authority;
- stale replay after source requalification fails;
- wire shape contains no lineage-specific current status or transition outcome.

## Explicit non-claims

SEL-10E1A2 does not:

- implement a second species concept;
- decide any species boundary;
- establish semantic independence;
- establish evidence independence;
- establish cross-model robustness;
- perform majority voting;
- infer speciation time;
- make taxonomic nomenclature decisions;
- establish universal taxonomy truth.

Its positive claim is narrower:

**This exact family-neutral descriptor canonically represents this exact concrete source authority, conceptual identity, capability surface, evidence schema, domain schema, and derivation provenance; where a source-specific replay exists, this descriptor exactly replays from the current concrete authority.**
