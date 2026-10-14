# Projection Source Authority Stamp V0

## Status

Normative Living World authority contract. This document strengthens the future `Level P -> Level A` boundary. It does not claim the current Level-P projection implementation already carries every field defined here.

## Problem

A prospective candidate needs enough context to avoid accidental cross-population or stale realization. Scope, numeric revision, projection scheme version, and seed are necessary, but numeric revision alone is not globally self-describing.

The value `revision = 7` can mean different things if it belongs to:

- a marginal `PopulationState`;
- a stratified canonical population;
- a region snapshot;
- a post-rollback branch;
- a migrated schema version;
- a reloaded authority epoch that reused a local counter.

A realization boundary must never infer those meanings from an untyped integer.

## Core rule

Every projection handle that may later be presented to a canonical realization API must bind to an explicit **source authority stamp**.

Conceptually:

```text
ProjectionSourceAuthorityStamp {
    scope,
    authority_kind,
    authority_schema_version,
    generation,
    source_state_token,
}
```

The exact Rust representation is non-normative. The semantic fields are normative.

## Authority kind

`authority_kind` identifies which representation established the candidate's known facts.

Examples include:

```text
MarginalPopulation
StratifiedPopulation
ActivePopulationRefinement
PersistentOrganism
```

V0 Level-P marginal projection should bind specifically to the marginal-population authority kind.

A future strata-aware targetable projection must use a distinct authority kind and projection scheme/version. A marginal candidate must never be silently reinterpreted as though it came from joint strata.

## Authority schema version

The source structural schema version is independent from the projection algorithm version.

These answer different questions:

- source schema version: **what ecological facts were available?**
- projection scheme version: **how were candidate details selected from those facts?**

Changing either can change candidate semantics.

A realization implementation must validate both.

## Generation

`generation` is a source-authority freshness coordinate whose namespace and monotonicity are defined by the higher authoritative layer.

It must not be treated as globally meaningful without the associated scope/kind/schema.

If generations can be reused after rollback, restore, branch switching, migration, or reconstruction, generation alone is insufficient for realization.

## Source state token

The source stamp therefore includes an opaque `source_state_token` representing the exact authority state or authority epoch from which the projection was made.

The token may eventually be backed by:

- a canonical state digest;
- a qualified snapshot digest;
- a causal event-head digest;
- an opaque non-reused authority-epoch identifier;
- another higher-layer token with equivalent uniqueness semantics.

`lifesim-core` need not own cryptography to carry such a token. The higher authoritative layer may create it and the projection layer may treat it as opaque routing/freshness evidence.

The required semantic property is:

> Two semantically different source authority states must not accidentally share the complete realization stamp merely because they reuse the same local revision number.

## Projection context

The future complete context should conceptually bind:

```text
source authority stamp
+ projection scheme version
+ projection seed/family
+ candidate index
```

That complete tuple identifies a **prospective candidate occurrence**, not a persistent organism.

It grants no mutation authority by itself.

## Realization validation

Before a Level-P candidate can become Level A, realization must check at least:

1. scope matches the requested/current population;
2. authority kind matches the realization adapter;
3. authority schema version is understood;
4. generation is current or otherwise explicitly admissible;
5. source state token matches the canonical source lineage/state expected by the realization policy;
6. projection scheme version is understood;
7. projected known facts are still compatible with current canonical authority;
8. the candidate handle has not already been consumed by an exclusive realization transaction when exclusivity is required.

Failure is stale/invalid realization and must not reserve count or extensive authority.

## Rollback and replay

A particularly important anti-alias case is:

```text
state A: scope = P, generation = 7
     |
     v
state B: generation = 8
     |
  rollback/reload
     v
state C: scope = P, generation = 7
```

If state C is not semantically identical to state A, a candidate projected from A must not become valid against C merely because the numeric generation matches again.

A non-reused source-state token or canonical digest distinguishes them.

## Source schema upgrade

Suppose a candidate was shown from marginal authority and the population later promotes to sparse joint strata.

The new strata may prove the synthetic marginal tuple impossible.

The old candidate does not automatically inherit strata-aware validity.

Allowed responses include:

- reject as stale and reproject;
- explicitly bridge through a qualified compatibility adapter that proves the shown tuple remains realizable;
- retain the old source authority long enough to finish a transaction if the higher layer has explicitly reserved that authority.

Silent reinterpretation is forbidden.

## Multiple observers

Different observers may hold projections from the same source stamp and different seeds/families.

That does not multiply ecological authority.

The realization transaction arbitrates canonical ownership. Projection handles are proposals, not reservations.

## Canonical event integration

After CRK/event identity is sealed and adopted by the higher ecology layer, a causal event-head digest may be one component of `source_state_token` or provenance.

This contract does not require that dependency today and does not make Level-P projection depend on `symtropy-game-state`.

## Qualification fixtures

Minimum future executable evidence should include:

1. same scope/generation but different authority kinds cannot cross-realize;
2. same kind/generation but different source schema versions cannot cross-realize;
3. same scope/kind/schema/generation but different source-state tokens cannot cross-realize;
4. rollback/reuse of a numeric generation does not revive an old candidate;
5. changing only presentation seed changes the projection family but does not alter source authority identity;
6. changing only projection scheme version requires an explicitly supported scheme;
7. stale validation consumes no count or extensive share;
8. duplicate realization retry is idempotent according to the active transaction contract;
9. marginal-source candidate cannot silently realize through a strata-only adapter;
10. source-state token remains opaque to low-level presentation code.

## Non-goals

This contract does not define:

- the cryptographic algorithm for source-state tokens;
- persistent organism identity;
- CRK-02 world transactions;
- network authentication;
- projection rendering placement;
- exact serialization bytes.

It freezes the semantic requirement that **a realizable projection must name not only where it came from and when, but what kind/version of ecological authority it came from and which exact authority state that freshness coordinate refers to**.
