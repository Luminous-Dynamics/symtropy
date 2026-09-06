# Living World Prospective Projection and Realization v0

Status: companion authority contract. Where this document is more specific than earlier population-refinement wording, this document governs Level-P presentation versus Level-A authority transfer.

## Purpose

A non-authoritative presentation projection does not own ecological truth. Therefore Level P does **not** need to subtract count or biomass from canonical coarse population merely to render individual-looking organisms.

However, visible projection can still influence a human player or another observing agent. A player may aim at, follow, avoid, name, photograph, or otherwise act because a projected organism appeared at a particular world-space location.

Living World therefore needs a deterministic boundary between:

- a disposable visual projection;
- a prospective individual-like projection that may later be referenced by interaction; and
- an active canonical microstate that is allowed to affect authoritative history.

## Normative clarification

The earlier rule "materialization requires reservation" applies to **authoritative active refinement**, not to strictly non-authoritative Level-P projection.

The stronger rule is:

```text
Level P projection: no ecological authority is transferred.
Level A refinement: ecological authority must be reserved/transferred exactly once before canonical effects occur.
Level I identity: persistent individual authority must remain excluded from exchangeable coarse authority.
```

A Level-P projection may be generated directly from coarse population state because it is a read-only view and owns no count, biomass, lineage, history, or ecological mutation authority.

## Two presentation classes

### Decorative projection

A decorative projection is pure presentation whose exact identity/location is not exposed as an interactable ecological entity.

Examples:

- distant flock texture/particle impressions;
- canopy micro-motion;
- non-addressable insect specks;
- presentation-only background silhouettes.

Decorative projection may use camera/LOD inputs because no canonical command can refer to a particular projected member.

### Prospective entity projection

A prospective projection depicts an individual-like entity that an observer may distinguish or target.

It is still non-authoritative, but it must be stable enough to support perceptual continuity and safe later realization.

Examples:

- a distant deer visible through binoculars;
- a bird the player can track before entering interaction range;
- a tree proxy that may later become individually damageable;
- a school member that can become addressable when approached.

Prospective projection should be world-space and observer-independent wherever practical. Multiple observers should normally see compatible subsets of the same prospective projection rather than independently inventing different candidate organisms.

## Projection handle

A prospective projection may expose an explicitly non-authoritative handle such as:

```rust
pub struct ProjectionHandle {
    pub source_revision: u64,
    pub projection_epoch: u64,
    pub ordinal: u64,
}
```

The exact representation may differ.

Required properties:

- a projection handle is not a persistent organism ID;
- a projection handle cannot be accepted by ordinary canonical mutation APIs;
- it is scoped to one source population revision/projection epoch;
- stale handles fail closed;
- renderer entity IDs, ECS generations, GPU instance indices, and frame numbers are never authority identifiers.

## Stable candidate microstate

For prospective entities, projection should derive a deterministic candidate microstate from authoritative coarse inputs plus a stable projection seed/epoch.

The candidate may include presentation-relevant unresolved degrees of freedom such as:

- candidate world-space position within canonical occupancy constraints;
- candidate age/condition tuple consistent with represented marginals/strata;
- phenotype variation;
- animation phase or pose seed;
- orientation;
- other bounded unresolved variables.

The candidate is **tentative**, not historical truth.

Re-rendering the same valid projection epoch should recover the same candidate rather than reshuffling visible animals every frame.

## Realization boundary

A canonical operation must never directly mutate or consume a Level-P projection.

If an authoritative command references a prospective projection, the system performs an atomic realization before resolving that command:

```text
prospective projection handle
        |
        | validate source revision / epoch
        v
candidate microstate
        |
        | reserve compatible coarse authority
        | verify count / biomass / strata constraints
        v
Level-A canonical active member
        |
        | only now
        v
resolve canonical interaction
```

Examples include:

- projectile/hit resolution;
- capture;
- harvesting;
- collision that changes canonical world state;
- predation;
- injury;
- tracking/tagging;
- command/AI targeting;
- terrain interaction with ecological consequences.

## Realize what was shown

When a valid prospective projection becomes Level A, the preferred behavior is to commit the **same candidate microstate the observer was shown**, provided it is still compatible with current canonical coarse authority.

This prevents a visible deer from becoming a different age/condition/location merely because interaction crossed the authority boundary.

The realization transition establishes canonical truth from that simulation tick forward. It does not claim the candidate's unresolved individual history was known before realization.

## Source revision and invalidation

A prospective candidate must be bound to a source revision or equivalent authoritative population generation.

If coarse population state changes in a way that invalidates the candidate before realization, the system must not silently apply a stale projection handle.

Possible safe outcomes include:

- reject the stale interaction target;
- deterministically regenerate/reconcile projection before command resolution;
- retain a short qualified projection lease whose reserved feasibility is guaranteed by a higher authority layer.

The v0 requirement is fail-closed stale detection, not a particular UX policy.

## Atomic realize-and-command

Realization plus the triggering canonical command should be transaction-like.

Preflight must establish at least:

1. projection handle is current;
2. candidate satisfies current source constraints;
3. compatible count/biomass/strata can be reserved;
4. active-generation handle can be created without duplication;
5. any ecological/account settlement required by the command is valid.

Only then may the transition commit.

If command validation fails, the system must not leave a half-realized/double-counted organism unless policy explicitly commits realization as a separate authoritative event.

## Human-observer causality

The engine cannot prevent a human from changing behavior because of something rendered.

Instead, Living World guarantees that any machine-recognized canonical action referring to a prospective entity crosses the explicit realization boundary before ecological consequences occur.

This is stronger and more useful than pretending presentation can never influence decisions.

## Multiple observers

Prospective projection should avoid observer-private biological reality.

For shared/federated worlds, a prospective projection policy should derive candidates from shared authoritative inputs or distribute a shared projection roster/epoch.

Two observers must not successfully realize incompatible copies of the same candidate ecological quantity.

Realization authority remains single-owner even when projection is visible to many observers.

## Relationship to current derived materialization

A read-only `DerivedPopulation` whose members cannot directly mutate canonical ecology can be interpreted as a Level-P projection primitive.

Under that interpretation, direct projection from `PopulationState` is not itself a double-authority bug because the derived members explicitly own no ecological quantity.

Before this representation is allowed to drive canonical interactions, Living World still needs the separate Level-A reservation/refinement boundary described by the active-refinement contract.

This distinction allows the current projection work to remain useful without granting it authority it was never designed to hold.

## Qualification requirements

Evidence must establish at least:

1. creating/destroying Level-P projections does not change coarse count, biomass, strata, accounts, or persistent identity;
2. canonical mutation APIs reject projection handles directly;
3. prospective projection is deterministic for a fixed source revision/epoch;
4. re-render/frame-rate changes cannot reshuffle prospective candidates within one epoch;
5. stale projection handles fail closed after incompatible source revision;
6. realization reserves ecological quantity exactly once before canonical command effects;
7. valid realization preserves the candidate microstate the observer was shown;
8. failed realize-and-command cannot leave duplicate/half-transferred authority;
9. two observers cannot realize incompatible copies of one prospective candidate;
10. projection handles cannot masquerade as active or persistent IDs;
11. no-render, low-fidelity, and high-fidelity runs produce identical canonical outcomes for the same authoritative interaction trace once realization inputs are fixed.

## Design principle

**Projection may suggest a possible individual. Realization chooses that individual as present canonical truth. Interaction may only act after that choice is explicit, validated, conservative, and single-owner.**
