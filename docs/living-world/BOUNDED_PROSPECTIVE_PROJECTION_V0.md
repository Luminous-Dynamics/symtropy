# Living World Bounded Prospective Projection v0

Status: companion contract for rendering/observing very large coarse populations without O(N) materialization.

## Problem

A coarse population may represent thousands, millions, or more exchangeable organisms while a nearby/far presentation needs only a bounded number of visible candidate entities.

A projection API that expands one derived member per canonical organism before selecting visible members defeats the purpose of multiscale ecology.

The target complexity for a bounded Level-P projection is therefore proportional to the number of projected candidates and occupied coarse bins/strata, not to total population headcount.

## Core requirement

Given canonical population size `N` and requested projection size `k`:

```text
work ~= O(k × lookup_cost)
memory ~= O(k + coarse-bin-state)
```

with a hard pre-allocation bound.

The implementation must never allocate `O(N)` member storage merely to project `k << N` candidates.

## Deterministic virtual ordinals

A bounded projection may treat the coarse population as a conceptual virtual index space `[0, N)` without actually expanding it.

For each projected ordinal `i in [0, k)`, derive deterministic virtual indices from a stable projection seed/epoch.

Different coarse dimensions should use independent salts/permutations so canonical key ordering does not manufacture strong synthetic age×condition×occupancy correlations.

Conceptually:

```text
age_virtual_index       = P(seed, AGE_SALT, i, N)
condition_virtual_index = P(seed, CONDITION_SALT, i, N)
occupancy_virtual_index = P(seed, CELL_SALT, i, N)
```

where `P` is deterministic and yields distinct indices for `i < k` within each dimension when sampling without replacement is required.

The exact permutation algorithm is an implementation choice and must be frozen/versioned once qualification depends on it.

## Distribution lookup

A virtual index maps into a sparse count distribution by cumulative integer counts.

Example:

```text
juvenile: 30
adult:    50
elder:    20
```

produces cumulative intervals:

```text
[0,30)   -> juvenile
[30,80)  -> adult
[80,100) -> elder
```

No per-organism expansion is necessary.

For many-bin populations, an indexed prefix-sum/search representation may reduce lookup to `O(log bins)` while keeping canonical counts integer and deterministic.

## Stratum-aware projection

Once sparse joint strata become canonical, prospective projection should prefer drawing candidate members from strata rather than independently recombining marginals.

This preserves already-authoritative correlations such as:

- age×disease;
- occupancy×condition;
- genotype×habitat;
- stage×biomass class.

Independent marginal synthesis remains appropriate only for relationships intentionally unresolved by coarse authority.

## Candidate biomass

A Level-P candidate may display an approximate body-size/mass presentation, but that value is not canonical until realization.

If canonical realization occurs, active biomass must be assigned from the reserved coarse/stratum extensive state according to the qualified realization rule.

Where the projection displays size in a way that matters to player expectation, realization should preserve the displayed size candidate when compatible with the reserved stratum biomass model.

## Projection limit

Projection construction accepts an explicit maximum candidate count.

The implementation must fail before candidate allocation if the request exceeds that bound.

This is separate from canonical population count: a population of one billion organisms may legitimately project only 64 prospective individuals.

## Observer independence

For prospective entity projection, candidate generation should normally be keyed by world/population revision and projection epoch rather than render frame or observer-local iteration order.

Different observers requesting overlapping regions should see compatible prospective candidates wherever product semantics expose those candidates as individually distinguishable.

Pure decorative projection may remain observer-specific because it cannot be individually addressed or realized.

## Spatial projection

A future spatial sampler may first restrict canonical occupancy/strata to a world-space query region, then project `k` candidates from that subset.

The query itself may be driven by presentation relevance, but the output remains Level P until explicit realization.

For Level-A activation, the corresponding authoritative reservation uses canonical spatial criteria and exact population/stratum accounting rather than trusting renderer visibility lists.

## Stability

Within one source revision/projection epoch:

- adding unrelated shader features must not reshuffle candidate biological tuples;
- changing render frame rate must not reshuffle candidates;
- requesting fewer candidates should preserve the prefix of a larger request where the chosen algorithm promises prefix stability;
- candidate ordering/identity must not depend on hash-map iteration order.

Prefix stability is strongly preferred because it reduces visible popping when the projection budget changes.

## Schema evolution

Projection randomness should use keyed/counter-based derivation rather than a single sequential RNG stream wherever practical.

Adding a new presentation trait should not silently shift existing age, condition, occupancy, phenotype, or orientation candidates.

Each independently sampled property receives a stable semantic key/salt.

## Qualification requirements

Evidence must establish at least:

1. projecting `k` candidates from a large population does not allocate or iterate one record per canonical organism;
2. hard candidate limits fail closed before allocation;
3. same source revision/epoch/request produces identical candidates;
4. frame rate and renderer configuration do not affect candidate biological state;
5. independent salts prevent deterministic key-order alignment across unresolved marginals;
6. stratum-aware projection preserves canonical joint correlations;
7. projection does not mutate canonical population state;
8. candidate handles remain non-authoritative and cannot directly invoke canonical mutations;
9. stale source revisions invalidate prospective handles;
10. realization can adopt a valid projected candidate without changing the candidate the observer was shown;
11. projection complexity is benchmarked against increasing `N` with fixed `k` and remains bounded by coarse-bin lookup rather than headcount expansion.

## Design principle

**A million-organism population should cost like a million organisms only when ecology truly needs that detail. Showing a few plausible, stable, later-realizable individuals must remain a bounded view over coarse truth.**
