# Living World Population Refinement Authority v0

Status: architecture contract for implementation and qualification. This document does not claim the runtime has already satisfied these requirements.

## Purpose

Living World must support a very large biosphere without requiring every organism to remain individually simulated. Coarse population state is therefore authoritative for exchangeable background organisms, while local individuals may be derived temporarily for perception, animation, physics, and nearby simulation.

The dangerous boundary is refinement: a derived local organism must not silently become a second copy of population truth, and synthetic relationships introduced only for rendering or local simulation must not be mistaken for historical fact.

This contract freezes the authority rules before editable local ecology, persistent organism identity, reproduction, or rendering integration are added.

## Core authority theorem

For exchangeable organisms, canonical ecological truth is the coarse population state until an explicit authority transition says otherwise.

A representation may become more detailed without becoming more authoritative.

Therefore:

1. `PopulationState` is canonical coarse truth.
2. Materialization requires an explicit reservation removed from that coarse truth.
3. A derived local member is an ephemeral projection, not a persistent organism identity.
4. Synthetic cross-dimension relationships created during materialization remain non-authoritative.
5. A causal event that would make one synthetic tuple historically consequential requires explicit identity crystallization first.
6. Death, reproduction, injury, persistent learning, lineage, naming, ownership, player interaction, or other durable history may not be committed through an uncrystallized derived tuple.
7. Rendering, ECS lifetime, distance, LOD, and frame rate never create or destroy canonical organisms.

## Required type boundary

The API must make the safe path easier than the unsafe path.

Direct materialization from unrestricted `PopulationState` is not sufficient because a caller can accidentally materialize organisms while leaving the same count and biomass authoritative in the coarse population.

The target shape is conceptually:

```rust
pub struct PopulationPartition {
    remainder: PopulationState,
    reserved: ReservedPopulation,
}

pub struct ReservedPopulation {
    state: PopulationState,
}

impl PopulationState {
    pub fn reserve_prefix(
        self,
        count: u64,
    ) -> Result<PopulationPartition, PopulationError>;
}

impl ReservedPopulation {
    pub fn materialize_derived(
        self,
        seed: MaterializationSeed,
        max_individuals: usize,
    ) -> Result<DerivedPopulation, PopulationError>;
}
```

The concrete names may change, but the authority property may not: only population mass/count that has been explicitly removed from the coarse remainder may enter a local individual working set.

## Conservation boundary

For a reservation:

```text
source_count = reserved_count + remainder_count
source_biomass = reserved_biomass + remainder_biomass
```

Every canonical marginal represented by the coarse model must also split and recombine exactly.

Materialization does not create additional ecological quantity. It only changes representation of the reserved partition.

## Marginals are not a joint history

A coarse state may know independently that:

- 30% of the population is juvenile;
- 20% is stressed;
- 40% occupies cell A.

It does not therefore know that a particular juvenile was the stressed organism in cell A.

When a local working set combines those marginal values into member tuples, that association is a deterministic synthetic microstate unless authoritative joint history already exists elsewhere.

This distinction must remain visible in APIs, documentation, persistence, telemetry, and tests.

## Derived-member authority

A derived member may safely be used for reversible presentation and local computation whose result does not make its synthetic tuple historically consequential.

Examples include:

- rendering an animal body;
- choosing animation phase;
- local avoidance that is discarded when the representation collapses;
- presentation-only fur, feather, leaf, or gait variation;
- temporary physics proxies whose state does not become canonical ecological history.

A derived ordinal is scoped only to one materialization result. It is not a stable organism identifier and must not be persisted as one.

## Causal crystallization

When the world needs to preserve a particular member's joint state or biography, the member must cross an explicit authority boundary.

Conceptually:

```text
coarse population
    |
    | reserve
    v
reserved population
    |
    | derive reversible local microstate
    v
derived member
    |
    | durable causal contact
    v
crystallization
    |
    +--> persistent organism authority
    |
    +--> updated coarse remainder
```

Crystallization means that the world intentionally chooses and records the formerly derived tuple as canonical individual state from that point forward. It is not a claim that the tuple was historically known before crystallization.

The crystallization event establishes the boundary after which future history belongs to that persistent organism.

## Events requiring crystallization

At minimum, a derived member must crystallize before committing outcomes that require durable individual identity or persistent cross-dimension relationships, including:

- death or permanent removal when the exact individual matters locally;
- injury, scarring, limb loss, infection, or other lasting condition;
- reproduction or parent/offspring lineage;
- persistent learning or threat memory;
- taming, companionship, ownership, naming, tagging, tracking, or player attachment;
- quest/narrative participation;
- durable relocation when identity must follow the organism;
- capture, release, rescue, hunting, or harvesting that records an individual history;
- any event incorporated into a canonical organism biography.

A domain may crystallize earlier if needed, but never later than the first durable individual consequence.

## Observation is policy-sensitive

Merely entering render range does not require crystallization.

Whether visual observation alone crystallizes an organism is a product/world policy decision. A strict simulation may crystallize only on material interaction; a narrative world may crystallize organisms that the player closely observes or tracks.

Whatever policy is chosen must be deterministic from authoritative inputs. Camera frame rate and nondeterministic renderer timing are not valid authority inputs.

## Population-wide outcomes

Not every ecological event needs individual identity.

Population-scale mortality, migration, birth, disease progression, or resource consumption may operate directly on coarse distributions when the event semantics are permutation-invariant and do not claim a specific persistent organism.

For example, a coarse drought model may reduce a population and move biomass into detritus without fabricating identities for every death.

The implementation must distinguish:

- aggregate ecological outcome; and
- individual biographical outcome.

The first may remain coarse. The second requires persistent individual authority.

## Typed active outcomes

Editable local ecology should not expose an unrestricted mutable vector as the canonical mutation interface.

Prefer typed operations/outcomes such as:

```rust
pub enum ActivePopulationOutcome {
    ConditionTransition { /* bounded fields */ },
    Relocation { /* bounded fields */ },
    AggregateMortality { /* bounded fields */ },
    Crystallize { /* promotion evidence */ },
}
```

The exact API may differ, but every mutation must state which canonical observable it changes and how conservation/reconciliation is proven.

## Death settlement

Death never means deleting an entity and forgetting its ecological quantity.

A canonical mortality settlement must account for at least:

```text
living population biomass
    -> carrion / detritus
    -> decomposer pathway
    -> nutrient / soil / mycelial pathways
```

Any biomass leaving the Living compartment must be represented as an explicit ecological transfer or external output. The population reduction and ecological ledger must agree on the amount released.

## Reproduction settlement

Birth likewise cannot create unexplained organisms or biomass.

Future reproduction work must define explicitly:

- source population or persistent parents;
- inherited/genetic state authority;
- reproductive event authority;
- offspring count;
- biomass/resource cost or modeled external contribution;
- destination population or persistent-offspring identity.

Sequential RNG order is not valid inheritance authority. Deterministic keyed derivation should be used where stochastic biological variation is represented.

## Persistent identity budget

Not every organism should become permanently persistent merely because it was once rendered.

Persistent identity is reserved for organisms whose individual history has become semantically meaningful.

Examples include:

- notable or player-interacted animals;
- companions;
- organisms carrying lasting wounds or memories;
- tracked lineages;
- landmark/ancient trees;
- organisms participating in canonical causal events.

The system may later define safe demotion/compaction rules, but demotion must not erase consequences required by future gameplay, ecology, causality, or replay.

## Demotion rule

A persistent organism may only return to aggregate population representation if all authoritative information that must survive demotion is either:

1. conserved in the target population observables; or
2. preserved separately as durable summary/biography state.

If exact individual identity remains semantically required, demotion is forbidden.

## LOD theorem

For an unchanged world state:

```text
F0 coarse population
 -> reserve
 -> derived local representation
 -> collapse
 -> recombine
```

must recover the original canonical population exactly for every observable promised by the representation.

Changing visual/physical fidelity without an authoritative ecological event must not change:

- population count;
- living biomass;
- age distribution;
- condition distribution;
- spatial occupancy distribution;
- genetic observables once added;
- disease observables once added;
- persistent-organism state.

## Qualification requirements

Before the population-refinement boundary is considered qualified, automated evidence must establish at least:

1. Materialization is impossible through the public safe API without an explicit reservation type.
2. Reservation conserves exact count, biomass, and every represented marginal.
3. Unchanged reservation -> materialization -> reduction -> recombination recovers exact source state.
4. Different materialization seeds may change only synthetic tuple arrangement, never canonical coarse observables.
5. Derived ordinals cannot be accepted where persistent organism identity is required.
6. Render/ECS lifetime changes cannot mutate canonical population state.
7. Frame rate cannot alter population refinement or crystallization decisions.
8. Crystallization removes exactly one corresponding contribution from exchangeable coarse authority before persistent identity becomes authoritative.
9. No organism may be simultaneously authoritative in both persistent-individual and exchangeable-population representations.
10. Mortality settlement conserves released living biomass into an explicit destination or external output.
11. Qualification includes exact-head, clean-worktree, pinned-toolchain evidence consistent with repository policy.

## Non-goals for this contract

This document does not define:

- the final persistent-organism identity format;
- canonical event-v2 bindings;
- species schemas;
- reproduction genetics;
- flora structural graphs;
- fauna locomotion;
- renderer/ECS components;
- persistence serialization format;
- final demotion heuristics.

Those layers must obey this authority boundary when implemented.

## Design principle

The player approaching an ecosystem may reveal detail, but proximity must never create truth.

The world may choose a previously unresolved microstate when durable causal contact requires one. That choice must happen explicitly, deterministically, conservatively, and exactly once.
