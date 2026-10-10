# Living World Population Sufficient Statistics v0

Status: companion architecture contract. This document defines what information a coarse population representation is allowed to discard.

## Motivation

A coarse population can conserve headcount and biomass exactly and still become ecologically wrong if it discards correlations needed by future dynamics.

Consider two populations with identical marginals:

```text
50% juvenile
50% mature
50% healthy
50% infected
```

Population A may have infection concentrated entirely in juveniles. Population B may have infection concentrated entirely in mature organisms.

Their independent age and infection marginals are identical, but an age-dependent disease model can produce different futures.

Likewise:

- juveniles may be concentrated in nursery habitat;
- stressed organisms may cluster around a toxin source;
- genotype may correlate with elevation;
- reproductive state may correlate with age and season;
- predator exposure may correlate with occupancy and condition.

If Living World collapses those relationships merely because the player moved away, LOD has changed ecological truth.

## Sufficiency theorem

A canonical coarse population state must retain enough information to evaluate every authoritative coarse transition that may occur while that population remains coarse.

In statistical language, the coarse state should contain sufficient statistics for the transition model it is expected to support.

In simulation language:

> If two microscopic populations map to the same canonical coarse state, every authoritative process allowed to run at that coarse fidelity must either treat those populations equivalently or the coarse representation is missing state.

This is the **coarse-state sufficiency contract**.

## Marginals are views, not always sufficient authority

Independent marginal distributions are useful and cheap:

- age distribution;
- condition distribution;
- occupancy distribution;
- disease prevalence;
- genotype frequencies.

But they do not preserve covariance.

Therefore a marginal-only `PopulationState` is sufficient only for processes whose authoritative transition rules depend on those marginals independently.

When a process depends on a relationship among dimensions, that relationship must remain represented canonically at the fidelity where the process executes.

## Sparse strata

The preferred mechanism for preserving important correlations is a sparse stratum/cohort representation rather than a full Cartesian tensor or one row per organism.

Conceptually:

```rust
pub struct PopulationStratum<K> {
    pub key: K,
    pub count: u64,
    pub biomass_milligrams: u64,
}

pub struct StratifiedPopulation<K> {
    strata: BTreeMap<K, PopulationStratum<K>>,
}
```

The concrete low-level API may differ. The important properties are:

- only occupied strata are stored;
- counts and biomass remain exact integers;
- canonical marginals are derived by summing strata, not maintained as competing authorities;
- dimensions represented in `K` are chosen because their joint relationship affects future authoritative dynamics;
- sparse representation avoids paying for impossible/unoccupied combinations.

A higher-level ecology crate may own semantic stratum keys while `symtropy-lifesim-core` provides generic exact sparse-accounting primitives.

## Examples of useful strata

A terrestrial animal population might temporarily need joint state resembling:

```text
(cell, age_band, condition_band)
```

A disease model might require:

```text
(cell, age_band, disease_stage)
```

A plant population might require:

```text
(cell, development_stage, drought_memory_band)
```

A genetic adaptation model might require:

```text
(habitat_patch, genotype_cluster, development_stage)
```

These examples are not a fixed universal ontology. They illustrate that the canonical macrostate should preserve the relationships the active ecological model actually consumes.

## No universal full tensor

Living World should not allocate every possible combination of every biological dimension.

That would reproduce individual-scale cost in another form.

Instead:

1. store sparse occupied strata;
2. preserve only correlations required by authoritative dynamics or durable observables;
3. aggregate dimensions that are conditionally irrelevant at the current model boundary;
4. qualify every aggregation against the processes that remain enabled afterward.

## Correlation budget

Every population model should declare a **correlation budget**: the set of joint relationships guaranteed to survive coarse simulation and LOD transitions.

For v0 this can be explicit documentation/typing rather than a runtime reflection system.

Examples:

```text
preserve: occupancy × condition
preserve: age × reproductive_state
aggregate: cosmetic coat variation
aggregate: presentation-only gait phase
```

A later model may promote an aggregated dimension into the correlation budget when new gameplay or ecology requires it.

## Markov-closure test

A practical way to test coarse sufficiency is to ask whether the coarse process is approximately Markovian in the retained state.

Given two microscopic fixtures `A` and `B` that reduce to the same candidate coarse state `C`:

```text
reduce(A) = C
reduce(B) = C
```

run the authoritative process from both microscopic fixtures and compare the observables promised by the coarse model.

If their expected next-state behavior differs materially because of information discarded by `C`, then `C` is not sufficient for that process.

The remedy is one of:

- retain the missing correlation/state;
- disable that process at this coarse fidelity;
- replace it with a validated coarse closure model that explicitly accounts for the lost information.

Silently assuming independence is not an acceptable remedy when it changes authoritative ecology.

## Relationship to derived materialization

A sparse stratum can support more truthful materialization than independent marginals because a derived member can be sampled from a stratum whose joint properties are actually known.

However a stratum still does not imply persistent individual identity.

For example, a stratum may canonically know:

```text
17 juvenile + stressed organisms in cell A
```

Materialization may derive 17 local working members with those properties without inventing the correlation.

It still does not know which persistent organism was which unless identity has been crystallized separately.

Thus:

```text
joint coarse truth != individual identity
```

and the causal crystallization contract remains required.

## Relationship to reservation

Reservation selection should operate on the richest canonical population state available.

When strata exist, a targeted or representative reservation should split exact stratum counts and biomass first. Marginals should then be derived from the selected/remainder strata.

This avoids independently selecting incompatible marginal subsets.

If only marginals exist, the selection contract in `POPULATION_RESERVATION_SELECTION_V0.md` applies and any cross-dimension tuple remains explicitly synthetic.

## Relationship to LOD

LOD may reduce compute, update frequency, geometry, animation, or local behavioral detail.

It may not discard an ecological correlation that an enabled future process requires unless that loss is part of an explicit, qualified coarse closure transition.

Therefore:

```text
render fidelity change != ecological information destruction
```

and:

```text
ecological aggregation = explicit authority transition
```

## Hysteresis and latent state

Current observable marginals may also be insufficient because history can affect future response.

Examples:

- previously drought-stressed plants respond differently after rainfall;
- recently disturbed animals retain threat memory;
- recovering soil differs from never-degraded soil at the same instantaneous moisture;
- prior disease exposure changes susceptibility.

Such latent/history state must either:

- remain in strata/coarse memory variables;
- remain in persistent individuals;
- or be represented by a validated closure variable.

The sufficient-statistics contract therefore includes both covariance and hysteresis.

## Canonical vs telemetry marginals

Once a stratified representation exists, common marginals should generally become derived telemetry/query views rather than duplicated canonical storage.

This prevents two authorities such as:

```text
canonical strata count = 1,000
canonical age marginal count = 999
```

from coexisting.

The preferred rule is:

> Keep one canonical representation of each promised ecological fact; derive convenient summaries from it.

Redundant representations may exist only when they are continuously verified and serve a justified performance/evidence role.

## Qualification requirements

Before a stratified/coarse population model is considered sufficient for a process, evidence should establish:

1. exact stratum count and biomass conservation;
2. derived marginals exactly equal sums over strata;
3. sparse aggregation/dissolution never creates zero-count canonical strata;
4. split + recombine recovers exact strata;
5. materialization cannot claim correlations absent from the canonical representation;
6. LOD does not discard any declared correlation-budget dimension without an explicit aggregation transition;
7. microscopic fixtures with different hidden covariance but identical candidate coarse state are used as adversarial tests;
8. if those fixtures produce materially different authoritative futures, the representation is rejected as insufficient or enriched;
9. history-sensitive processes have explicit latent/hysteresis state or are disabled at fidelities that cannot represent it;
10. performance measurements show sparse strata remain materially cheaper than permanent individual simulation for target populations.

## Observatory metric: information-loss sensitivity

Living World Observatory should maintain paired fixtures that intentionally share marginals while differing in covariance.

Examples:

- disease concentrated in juveniles vs adults;
- stressed organisms clustered vs spatially uniform;
- genotype adapted to one habitat patch vs randomly distributed.

Run both through coarse and fine models over a defined horizon.

Measure divergence in promised observables such as:

- survival;
- reproduction;
- disease prevalence;
- migration;
- biomass;
- spatial occupancy.

A coarse representation passes only within an explicit tolerance appropriate to the model.

This gives Symtropy a measurable answer to the question:

> **How much biological information may we discard before the future changes?**

## Design principle

**A coarse model is not good because it is small. It is good when it is the smallest state that still carries the ecological information needed to produce the right future.**
