# Living World Active Extensive Share v0

Status: companion authority contract.

## Purpose

Level-A realization eventually has to transfer exact conserved quantity out of a coarse population stratum. A stratum may know exactly:

- member count `N`;
- total living biomass `B` in integer milligrams;

without knowing the historically measured biomass of each microscopic member.

That information boundary must not be hidden by inventing an "exact individual body mass" during realization.

This contract separates **exact conservation ownership** from **modeled biological phenotype/state**.

## Core distinction

For an active organism, these are different concepts:

1. **extensive authority share** — the exact integer conserved quantity transferred out of aggregate authority and owned by the Level-A refinement;
2. **modeled body-mass estimate** — a biological/phenotypic estimate used only by processes whose information requirements permit it;
3. **measured individual mass** — an individual fact only when canonical evidence/state actually establishes it.

An aggregate stratum total does not imply that microscopic member masses were equal, nor that the engine knows which member owned which historical milligram.

Therefore:

> deriving an exact accounting share from aggregate state is an authority partition, not a claim that the share equals historically measured individual body mass.

## Exact partition requirement

For a stratum with:

```text
count = N > 0
biomass = B mg
```

any set of Level-A authority shares plus the remaining stratum authority must satisfy exactly:

```text
sum(active_shares) + remaining_stratum_biomass = B
```

and:

```text
active_count + remaining_stratum_count = N
```

at every committed authority boundary.

No floating-point conversion participates in this equality.

## Canonical authority slots

A realization/reservation scheme should transfer authority through **canonical stratum-local authority slots**, not renderer entity IDs, projection array indices, ECS order, thread order, or wall-clock timing.

Conceptually:

```text
StratumAuthoritySlot {
    population_scope,
    source_revision/generation,
    stratum_key,
    slot_ordinal,
    allocation_scheme_version,
}
```

The exact final type may live above `symtropy-lifesim-core`, but the semantics are normative.

A slot ordinal is not a persistent organism identity. It is a generation-scoped ownership coordinate used to make exact reservation/recombination deterministic.

## Aggregate-only v0 share allocation

When no richer per-member extensive distribution is canonical, a v0 allocator may partition `B` across `N` authority slots using exact quotient/remainder arithmetic:

```text
q = B / N
r = B % N

slot_share(slot) = q or q + 1 mg
```

with exactly `r` slots receiving the extra milligram.

The allocator MUST freeze how those `r` slots are selected. It must not depend on:

- projection seed;
- render order;
- active-budget request order;
- nondeterministic map iteration;
- CPU/thread timing.

A versioned deterministic residue-placement rule is required.

## Residual placement and selection bias

Assigning all `+1 mg` residuals to low numeric slot ordinals is exact but can correlate accounting residue with a representative reservation sequence that also prefers low ordinals.

The v0 residue-placement scheme should therefore be independently keyed/versioned from projection and reservation ordering, or the qualification suite must prove that the chosen composition cannot create a meaningful systematic authority bias.

This is an accounting concern, not a claim that one milligram changes visible phenotype.

## No projection-derived mass

A Level-P candidate carries no canonical biomass authority.

Its:

- candidate index;
- projection seed;
- projected age/condition/cell tuple;
- renderer handle;

must never directly determine exact mass ownership.

At realization:

```text
Level-P candidate
    -> validate current projection context
    -> identify compatible current canonical stratum
    -> reserve one canonical authority slot
    -> compute that slot's exact extensive share under the frozen authority allocator
    -> create Level-A refinement
```

Only then does exact biomass authority leave the stratum.

## Candidate preservation vs authority allocation

`realize what was shown` means the canonical Level-A refinement preserves every projection fact that remains compatible with current canonical source state.

It does **not** mean the projection pre-owned an authority slot.

Multiple observers may show the same candidate proposal. The first successful canonical realization acquires one compatible slot under the authority allocator. A second attempt against the same single-owner realization key must fail closed or resolve to the already-authoritative refinement according to the higher-layer transaction contract.

## Richer extensive state supersedes aggregate allocation

The quotient/remainder allocator is valid only when aggregate stratum biomass is the strongest canonical extensive information available.

If a future representation stores richer authoritative extensive information, for example:

- biomass sub-bands;
- body-size classes;
- measured persistent individuals;
- exact per-member extensive state;

then a process requiring those facts must use the richer representation rather than collapsing back to equal-share accounting.

The process-information matrix decides whether aggregate-share allocation is sufficient.

## Modeled body mass is not conservation authority

A Level-A organism may need a continuous modeled mass for animation, locomotion, energetic demand, or approximate physiology.

If that value is not canonically measured, it must remain distinguishable from the exact extensive authority share.

For example:

```text
ActiveRefinement {
    exact_biomass_authority_mg: 124_031,
    modeled_body_mass_kg: 0.1267,
    ...
}
```

is semantically acceptable only if downstream processes know which field is authoritative for conserved settlement.

A process may not silently overwrite exact conserved authority from the modeled float.

## Growth, consumption, excretion, injury, and death

Once Level A owns an exact extensive share, future canonical biomass changes occur through explicit exact flux/settlement operations.

Examples:

```text
food/resource -> Living(active)
Living(active) -> waste
Living(active) -> detached tissue
Living(active) -> Detritus on death
```

The authority share changes only through such committed transfers.

No animation scale change, phenotype recalculation, rendering LOD transition, or approximate physiology update may silently create/destroy exact biomass.

## Collapse back to coarse state

When a Level-A refinement can safely return to coarse authority:

```text
active exact share + remaining stratum exact biomass
    -> recombined canonical stratum biomass
```

must be exact.

The collapse also has to preserve every joint statistic still required by enabled processes. Exact biomass conservation alone is not sufficient permission to discard Level-A information.

## Persistent identity

Promotion from Level A to Level I moves the exact extensive authority share from temporary active ownership to persistent individual ownership without creating or destroying biomass.

Conceptually:

```text
Living / active refinement
    -> Living / persistent organism
```

is an ownership transfer, not an ecological source/sink.

## Fail-closed cases

Realization/settlement must not mutate canonical state when any of these fail:

- stale population/source revision;
- wrong population scope;
- incompatible projection scheme/context;
- selected stratum no longer has sufficient count;
- selected authority slot already owned;
- arithmetic overflow;
- allocation scheme mismatch;
- process requires richer extensive information than aggregate-share allocation provides;
- atomic conservation settlement cannot commit.

## Qualification requirements

The Living World Observatory should establish at least:

1. for exhaustive small `(N, B)` fixtures, slot shares sum exactly to `B`;
2. exactly `N` slot shares are defined;
3. every share is integer-exact and differs from `floor(B/N)` by at most one milligram for the aggregate-only v0 allocator;
4. residue placement is deterministic and versioned;
5. projection seed/renderer order do not alter exact authority allocation;
6. reservation of any valid subset plus remainder conserves count and biomass exactly;
7. reservation followed by collapse recovers exact source authority;
8. repeated/multi-observer realization cannot duplicate the same authority ownership;
9. stale/scope/scheme/allocation-version mismatches fail before mutation;
10. modeled body-mass changes cannot mutate exact extensive authority;
11. explicit growth/death fluxes change the exact share by exactly the settled amount;
12. `u64` boundary/adversarial arithmetic remains fail-closed;
13. richer canonical mass information prevents fallback to aggregate-only allocation when a process requires that richer information.

## Design principle

**Aggregate conservation can tell us exactly how much matter authority must move without pretending it told us the historically measured body mass of an unresolved individual.**
