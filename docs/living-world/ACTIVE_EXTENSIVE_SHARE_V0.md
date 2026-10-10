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

When no richer per-member extensive distribution is canonical, the preferred v0 allocator is a **telescoping cumulative partition**.

For canonical slot ordinal `i` with `0 <= i < N`:

```text
prefix(i) = floor(B * i / N)

slot_share(i) =
    floor(B * (i + 1) / N)
  - floor(B * i / N)
```

The products MUST be evaluated in widened integer arithmetic (`u128` is sufficient for `u64` B, N, and i) before division.

This formulation is equivalent to quotient/remainder allocation but freezes the residual placement without a secondary random or ordering rule.

Let:

```text
q = floor(B / N)
r = B mod N
```

Then every slot share is exactly either:

```text
q
```

or:

```text
q + 1 mg
```

with exactly `r` slots receiving the extra milligram.

## Telescoping conservation theorem

For all valid `N > 0`, `B >= 0`, and `0 <= k <= N`:

```text
sum(slot_share(i), i = 0..k-1)
    = floor(B * k / N)
```

Therefore for the complete partition:

```text
sum(slot_share(i), i = 0..N-1)
    = floor(B * N / N)
    = B
```

exactly.

No accumulation loop is required to compute an individual slot share, and no remainder pool can become orphaned.

## Prefix proportionality bound

For every prefix length `k`:

```text
ideal_prefix = B * k / N
actual_prefix = floor(B * k / N)
```

so:

```text
0 <= ideal_prefix - actual_prefix < 1 mg
```

This is stronger than merely saying that each individual share differs by at most one milligram.

It means a prefix-stable authority reservation sequence can expand or contract its active budget without front-loading an arbitrary rounding residue. The aggregate exact authority assigned to its first `k` slots is always within strictly less than one milligram of the mathematically proportional ideal.

This does **not** make the slots biological body-mass measurements. It only makes the accounting partition maximally balanced under the information available.

## Slot ordering is authority semantics

The cumulative rule removes a separate residual-placement lottery, but slot ordinal still matters and therefore MUST be canonical and versioned.

The slot ordering must not be derived from:

- renderer entity IDs;
- frame number;
- projection-array order;
- wall-clock time;
- thread/ECS iteration order;
- nondeterministic map order.

For representative Level-A refinement, the preferred relationship is:

```text
qualified representative reservation sequence
    -> canonical stratum-local slot ordinal
    -> cumulative exact authority share
```

so the same nested reservation order also receives the prefix proportionality guarantee above.

Targeted reservation may choose non-prefix slot subsets, but every selected slot still owns its frozen exact share and total conservation remains exact.

## Allocation versioning

The cumulative partition itself should be frozen under an explicit allocation scheme version, for example conceptually:

```text
CUMULATIVE_EXACT_SHARE_V1
```

Changing any of the following is a semantic version change:

- authority slot ordering;
- integer unit;
- cumulative-share formula;
- source extensive-state interpretation;
- handling of richer per-member extensive evidence.

A stale Level-A handle bound to another allocation version must fail closed at settlement/recombination.

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
    -> validate current projection context/source authority
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

The cumulative aggregate allocator is valid only when aggregate stratum biomass is the strongest canonical extensive information available.

If a future representation stores richer authoritative extensive information, for example:

- biomass sub-bands;
- body-size classes;
- measured persistent individuals;
- exact per-member extensive state;

then a process requiring those facts must use the richer representation rather than collapsing back to aggregate equal-share accounting.

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

- `N == 0` for a requested slot allocation;
- slot ordinal `i >= N`;
- stale population/source revision;
- wrong population scope;
- incompatible projection scheme/context/source authority;
- selected stratum no longer has sufficient count;
- selected authority slot already owned;
- arithmetic overflow or invalid widened arithmetic path;
- allocation scheme mismatch;
- process requires richer extensive information than aggregate-share allocation provides;
- atomic conservation settlement cannot commit.

## Qualification requirements

The Living World Observatory should establish at least:

1. exhaustive small `(N, B)` fixtures prove all slot shares sum exactly to `B`;
2. exactly `N` slot shares are defined for each valid partition;
3. each share is exactly `floor(B/N)` or `floor(B/N)+1`;
4. for every small `k <= N`, the first `k` shares sum exactly to `floor(B*k/N)`;
5. prefix proportionality error is always `< 1 mg`;
6. `u128` intermediates handle adversarial `u64` boundary values without overflow;
7. `N == 0` and `i >= N` fail before mutation;
8. projection seed/renderer order cannot alter exact authority allocation;
9. reservation of any valid subset plus remainder conserves count and biomass exactly;
10. nested prefix expansion from `k` to `k+1` preserves every already-owned slot/share;
11. reservation followed by collapse recovers exact source authority;
12. repeated/multi-observer realization cannot duplicate the same authority ownership;
13. stale/scope/source/scheme/allocation-version mismatches fail before mutation;
14. modeled body-mass changes cannot mutate exact extensive authority;
15. explicit growth/death fluxes change the exact share by exactly the settled amount;
16. richer canonical mass information prevents fallback to aggregate-only allocation when a process requires that richer information.

## Design principle

**Exact aggregate conservation can be partitioned with sub-milligram proportional discrepancy across canonical authority slots, without pretending those accounting shares are historically measured individual body masses.**
