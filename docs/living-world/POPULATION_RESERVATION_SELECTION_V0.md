# Living World Population Reservation Selection v0

Status: companion authority contract to `POPULATION_REFINEMENT_AUTHORITY_V0.md`. This document defines how a coarse population subset may be selected for authority transfer without silently biasing ecology or losing partition provenance.

## Why this contract exists

Exact conservation is necessary but not sufficient.

A reservation algorithm can preserve count and biomass perfectly while still introducing systematic ecological bias. For example, selecting the first `n` entries of a canonical `BTreeMap`-ordered age distribution preferentially chooses whichever age bands sort first. Repeating that policy near every player or simulation focus can make materialized ecology statistically different from offscreen ecology even though every split/recombine round trip is numerically exact.

Living World therefore distinguishes:

1. **which subset should be selected**, and
2. **the authority transfer that removes that subset from exchangeable coarse truth**.

Selection policy chooses marginals. Reservation performs the authority transfer.

## Selection theorem

A reservation selection must be explicit about whether it is:

- **representative** — intended to preserve the source population's coarse marginals as closely as integer arithmetic permits; or
- **targeted** — intentionally conditioned on authoritative criteria such as spatial occupancy, developmental band, disease state, or another represented coarse observable.

Canonical key order alone is not an ecological selection policy.

## Representative integer apportionment

For a source marginal with total population `N`, bin count `c_i`, and requested reservation count `n`, the ideal real-valued selected count is:

```text
q_i = n * c_i / N
```

The v0 representative policy should use deterministic largest-remainder apportionment:

1. compute `floor(q_i)` for every bin using widened integer arithmetic;
2. compute each exact remainder `r_i = (n * c_i) mod N`;
3. assign the remaining seats to bins with largest `r_i`;
4. resolve equal remainders using a deterministic tie-break derived from the reservation seed and canonical bin key representation;
5. never allocate more than the source bin contains.

All multiplication used to calculate `n * c_i` must be widened enough to avoid `u64` overflow; `u128` is sufficient for v0 `u64` counts.

Required result:

```text
sum(selected_i) = n
selected_i <= source_i
remainder_i = source_i - selected_i
selected_i + remainder_i = source_i
```

exactly for every bin.

## Why not random sampling

A sequential RNG stream is not acceptable authority because call-order changes can silently alter ecological selection.

If tie-breaking or stochastic-looking selection is useful, it must derive independently from stable authoritative inputs such as:

- source population scope;
- reservation request identity or deterministic seed;
- dimension name;
- bin key.

The same inputs must yield the same reservation plan regardless of unrelated calls elsewhere in the program.

## Targeted reservation

Representative selection is not always correct.

When the player approaches one part of a region, occupancy should generally be selected from that spatial area rather than proportionally from every cell in the regional population.

A targeted reservation therefore supplies or derives an explicit selection plan for one or more coarse dimensions. Examples:

- occupancy cells intersecting an activation volume;
- a diseased cohort selected for a diagnostic simulation;
- a reproductive-age cohort selected for a mating process;
- a migration front selected by directional movement state once such an observable exists.

Targeting must be based on canonical coarse observables, not render visibility, frame timing, nondeterministic physics order, or transient GPU state.

## Mixed policy across marginals

Because the current coarse population stores marginals rather than a joint distribution, different dimensions may use different valid selection policies while still describing only a reserved coarse subset.

For example:

```text
occupancy  -> spatially targeted
age        -> representative apportionment
condition  -> representative apportionment
```

Each selected marginal must total the same reservation count `n`.

This still does not prove cross-dimension correlation. The eventual derived local tuples remain synthetic until crystallization as defined by the refinement authority contract.

## Reservation plan

The implementation should make selected marginals explicit before authority transfer.

Conceptually:

```rust
pub struct PopulationReservationPlan {
    count: u64,
    selected_age: CountDistribution<PopulationAgeBand>,
    selected_condition: CountDistribution<PopulationConditionBand>,
    selected_occupancy: CountDistribution<PopulationCell>,
    biomass_milligrams: u64,
}
```

The exact API may differ. A plan is valid only if:

- every selected marginal totals the same requested count;
- every selected bin count is less than or equal to the corresponding source bin count;
- selected biomass is less than or equal to source biomass;
- zero-count bins remain noncanonical;
- all arithmetic is checked;
- the source population itself verifies before selection.

## Biomass selection

The current coarse state knows total biomass but does not know biomass by age, condition, or occupancy.

Therefore v0 must not pretend to know those correlations.

For representative reservations, exact fixed-point biomass may be apportioned proportionally by count using quotient/remainder arithmetic so:

```text
selected_biomass + remainder_biomass = source_biomass
```

exactly.

For targeted reservations, a different biomass allocation is only authoritative if the coarse model contains an observable that justifies it. Until then, targeted demographic/spatial selection must not fabricate a hidden body-mass correlation.

## Partition provenance

A low-level `merge(a, b)` operation is numerically useful but too weak as the primary authority path: two unrelated populations may happen to have compatible schemas and could be combined accidentally.

The safe reservation API should therefore return an opaque partition object whose private state binds the remainder and reservation to the same authority transition.

Conceptually:

```rust
pub struct PopulationPartition {
    remainder: PopulationState,
    reserved: ReservedPopulation,
    provenance: PartitionProvenance,
}
```

`PartitionProvenance` need not be a global game-state identity in `symtropy-lifesim-core`. Its purpose is local type-level discipline: callers should not be able to construct arbitrary reserved values or claim an unrelated population is the matching remainder through the safe API.

The exact mechanism may use private constructors, consuming methods, sealed state transitions, or another Rust ownership design.

## Consuming authority transitions

Prefer consuming APIs for authority movement.

Conceptually:

```rust
let partition = population.reserve(plan)?;
let active = partition.into_materialized(seed, limit)?;
```

or an equivalent state machine.

The key property is that the same owned population value cannot remain available as canonical exchangeable truth after its selected portion has been transferred into reserved authority.

Where cloning is supported for testing/snapshots, cloning does not create independent ecological authority; authority remains a property of the containing world-state transition.

## Recombination

For an unchanged reservation, safe recombination must recover the exact source state.

For an evolved reservation, recombination must accept an explicit settlement describing what changed rather than blindly adding two arbitrary `PopulationState`s.

Examples of settlement changes include:

- reduced count due to aggregate mortality;
- changed condition marginals;
- changed occupancy marginals due to migration;
- biomass released into detritus;
- one or more members crystallized into persistent identity.

The partition/recombination boundary should make those changes visible and typed.

## No silent population mixing

Species, region, habitat patch, or other semantic population scope may eventually live in a higher-level `symtropy-ecology` wrapper rather than permissive `symtropy-lifesim-core`.

Until that scope exists, low-level generic merge functions must not be treated as proof that two population states belong to the same semantic population.

A higher-level authority layer must bind population state to its semantic scope before cross-region or cross-species recombination can become canonical world state.

## Qualification requirements

Representative reservation is not qualified until tests establish:

1. exact selected count for every request from `0..=N` across fixture distributions;
2. every selected bin is a subset of its source bin;
3. selected + remainder reconstructs every source bin exactly;
4. widened apportionment cannot overflow for `u64` source counts;
5. deterministic tie-breaking for identical seed/input;
6. changing an unrelated sampling call cannot alter a reservation result;
7. long-run repeated reservations do not exhibit canonical-key-order bias;
8. zero/full reservations are exact;
9. targeted plans reject any selected bin not supported by source truth;
10. all selected marginals reconcile to the same reserved count;
11. biomass partitions exactly;
12. unrelated partition halves cannot pass through the safe recombination path as though they shared provenance.

A useful statistical observatory test should repeatedly reserve small representative cohorts from balanced/multimodal distributions using a deterministic seed corpus and verify that aggregate selected frequencies converge toward the source marginals without systematic enum/key-order preference.

## Design principle

**Conservation prevents ecological quantity from appearing or disappearing. Selection integrity prevents ecological composition from drifting merely because detail was requested.**

Living World needs both.
