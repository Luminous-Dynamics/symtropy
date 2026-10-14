# Living World Stratified Extensive Population State v0

Status: companion architecture contract for the next coarse-population generation.

## Purpose

Headcount marginals plus one population-wide biomass total are an intentionally small v0 representation, but they are not sufficient for every realistic ecological process.

A population can contain the same number of juveniles and adults under two very different biomass allocations. If growth, starvation, predation yield, decomposition, reproductive capacity, movement cost, or resource demand depends on body size/stage, one global biomass total erases future-relevant information.

Living World therefore distinguishes:

- **extensive quantities** that add across members/strata, such as count and living biomass;
- **intensive/state descriptors** such as condition class or temperature response;
- **correlations/strata** needed to bind extensive quantities to the ecological categories that explain their future behavior.

## Core rule

If an authoritative process consumes or produces an extensive quantity conditional on a coarse ecological category, that extensive quantity must be attributable at a coarse resolution sufficient for that process.

Example:

```text
juvenile: 80 members, 120 kg living biomass
adult:    20 members, 900 kg living biomass
```

is materially different from:

```text
juvenile: 80 members, 900 kg living biomass
adult:    20 members, 120 kg living biomass
```

even though both have identical age headcounts and total biomass.

If stage-specific metabolism/predation is enabled, a representation preserving only total biomass is insufficient.

## Sparse stratum model

A future coarse representation should be able to express sparse joint strata conceptually like:

```rust
pub struct PopulationStratum<K> {
    pub key: K,
    pub count: u64,
    pub living_biomass_milligrams: u64,
}
```

A stratum key may contain only the dimensions required by enabled dynamics, for example:

```text
(cell, age_band)
(cell, age_band, disease_stage)
(habitat_patch, development_stage, genotype_cluster)
```

This does **not** require a full Cartesian cube. Only occupied/required combinations need records.

## Marginals become views

When a canonical sparse stratum representation exists, age/condition/occupancy headcount marginals should preferably be derived views of that authority rather than separately mutable stores.

Likewise:

```text
population total biomass = sum(stratum biomass)
population total count   = sum(stratum count)
```

Redundant cached marginals/totals may exist for performance, but validation must detect disagreement and they must not become independent mutation authorities.

## Why biomass belongs in strata

Stratified biomass enables physically/ecologically meaningful operations such as:

- juvenile versus adult resource demand;
- size-selective predation;
- stage-specific mortality yield to detritus;
- growth transitions that move count/biomass between developmental strata;
- plant age/size structure;
- forest stand biomass by patch/development class;
- disease wasting concentrated in infected strata;
- migration of a subset carrying its actual living biomass contribution;
- decomposition accounting after cohort mortality.

Without it, active realization has to invent an individual's mass from an unrelated global total.

## Refinement allocation

When a stratum of:

```text
count = N
biomass = B
```

is refined into `k` active members, their canonical biomass assignments must sum exactly to the reserved stratum biomass assigned to those `k` members.

For a homogeneous unresolved stratum, deterministic quotient/remainder allocation is acceptable as a v0 closure:

```text
base      = B / N
remainder = B % N
```

but the allocation rule must be deterministic and versioned/stable for qualification.

If biomass variance or size-body correlation materially affects enabled dynamics, the coarse model must preserve a richer size/biomass distribution instead of pretending equal allocation is sufficient.

## Growth is a transfer, not a field edit

Increasing living biomass requires an ecological source.

Conceptually:

```text
resource / nutrient / prey / stored reserve
        -> Living biomass
```

A growth update must therefore reconcile population/stratum biomass with the ecological conservation/accounting layer.

Likewise wasting, respiration, excretion, shedding, harvest, and death route quantity to explicit destinations or external fluxes according to the modeled accounting scope.

## Stage transition

A stage change is not necessarily creation/destruction of biomass.

Example:

```text
juvenile stratum
    -> adult stratum
```

moves one organism plus its attributed living biomass and any other required extensive state from source to destination.

If maturation consumes external resources during the step, those flows are explicit settlement inputs rather than implicit biomass creation.

## Mortality

For coarse mortality of one stratum:

```text
Living / population / stratum
        -> Detritus / carrion
```

The mortality operation must select/reduce both count and corresponding living biomass coherently.

It is invalid to reduce headcount while leaving the dead members' biomass in the living stratum.

## Reproduction

Offspring count and offspring biomass are separate quantities.

A birth event may add many tiny offspring while consuming parent/resource reserves. Future reproduction models must not infer offspring biomass merely from population mean mass unless that closure has been explicitly qualified for the species/process.

## Plants and modular organisms

For plants, fungi, colonial organisms, and modular life, "individual count" may be less informative than biomass/module/ramet/genet measures.

The stratum architecture therefore treats count as one extensive dimension, not the universal measure of biological significance.

Species/domain adapters may later add qualified extensive observables such as:

- ramet count;
- leaf area;
- basal area;
- root biomass;
- fungal network biomass;
- colony worker biomass;
- reproductive propagule count.

The same single-authority/conservation rules apply.

## Sufficient-statistics rule

A stratum schema is not automatically sufficient because it contains more columns.

For every enabled coarse process, qualification must ask whether two fine populations with identical retained strata/extensive totals can still produce materially different futures because of discarded information.

If yes, either:

- refine the stratum key/state;
- add a closure/latent statistic;
- or restrict the process at that coarse fidelity.

## Qualification requirements

Evidence must establish at least:

1. sum of stratum counts equals canonical population count;
2. sum of stratum living biomass equals canonical living population biomass;
3. derived marginals agree exactly with strata;
4. reservation/recombination conserves count and biomass per represented stratum;
5. active realization allocates reserved biomass exactly with no creation/loss;
6. stage/condition/occupancy transitions move extensive quantities coherently;
7. coarse mortality removes matching living count and biomass and settles biomass to an explicit destination;
8. growth cannot increase living biomass without corresponding modeled input/transfer;
9. overflow/underflow failures are atomic;
10. information-loss sensitivity fixtures detect when a proposed stratum schema is insufficient for enabled dynamics.

## Design principle

**Count tells us how many living units exist. Biomass tells us how much living matter exists. Strata tell us which ecological state owns those quantities. Realistic coarse ecology needs all three to remain mutually consistent.**
