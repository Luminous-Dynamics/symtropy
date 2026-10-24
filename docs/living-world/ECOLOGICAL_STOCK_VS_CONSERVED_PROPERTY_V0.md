# Ecological Stock vs Conserved Property V0

## Status

Normative Living World accounting contract. This document constrains the future death, decomposition, metabolism, feeding, growth, respiration, and biogeochemical settlement layers. It does not require an immediate breaking change to the current `EcologicalLedger` API.

## Problem

An ecological **stock** is not automatically a **conserved property**.

For example, an organism's body biomass may pass through:

```text
living tissue
   -> carrion / detritus
   -> decomposer biomass
   -> dissolved/soil material
   -> atmospheric gases
```

The matter has not vanished, but it has stopped belonging to the semantic category "living biomass" or even "organic biomass" at several points.

Therefore a model must not infer:

```text
biomass stock is conserved forever
```

merely because death can initially transfer body mass exactly from `Living` to `Detritus`.

## Core distinction

Living World accounting separates at least three concepts.

### 1. Stock variable

A stock answers:

> How much material/state currently belongs to this ecological form or compartment?

Examples:

- living body biomass;
- detrital organic matter;
- fungal biomass;
- plant structural biomass;
- dissolved nutrient stock;
- soil organic matter.

Stocks may be consumed or produced by explicit reactions/processes.

### 2. Conserved property

A conserved property answers:

> Which quantity must balance across every process admitted by this accounting domain, except explicit boundary input/output?

Examples may include total mass or particular elemental masses such as carbon under an appropriate non-nuclear ecological reaction domain.

A property is called conserved only relative to an explicitly declared process boundary. The name alone is not proof of conservation.

### 3. Composition / carried property

A stock may carry overlapping conserved properties or modeled composition, for example:

```text
body stock
  + exact total mass ownership
  + carbon mass
  + water content
  + nutrient-element content
```

Those properties describe the same material from different accounting perspectives and MUST NOT simply be added together as if they were disjoint masses.

## Current ledger interpretation

The existing `EcologicalLedger` is a useful v0 transfer ledger. Until reaction semantics are introduced, quantities such as `BiomassMass`, `WaterMass`, `CarbonMass`, and `MineralNutrientMass` can be checked independently for transfer-style accounting.

However:

- `BiomassMass` must not be treated as proof that biochemical reactions conserve the semantic category biomass;
- `WaterMass` may be transfer-conserved in hydrological processes without claiming arbitrary chemistry can never create/consume water molecules;
- aggregate nutrient categories are conserved only if their defined elemental/species boundary supports that claim;
- no code may sum overlapping quantity axes into a fake "total matter" without a composition model proving they are disjoint.

A future reaction-capable ledger should make these semantics explicit rather than relying on enum names.

## Death is a transfer before it is a reaction

For the first Living World mortality loop, death can be modeled conservatively as an ownership/form transition that preserves exact body-mass authority:

```text
Living / active organism
      |
      | exact body-mass share
      v
Detritus / corpse or carrion
```

At the instant of death, absent explicit emissions/fluids/fragmentation modeled in the same transaction:

```text
exact mass before = exact mass after
```

This does **not** imply later decomposition must preserve a `BiomassMass` stock unchanged.

## Decomposition is a reaction/flux network

Decomposition should eventually be represented conceptually as:

```text
reactant stocks + environment + process
          |
          v
validated flux/reaction plan
          |
          +--> product stocks
          +--> gases / dissolved pools
          +--> decomposer assimilation
          +--> heat / external outputs where modeled
          |
          v
conserved-property balance checks
          |
          v
atomic commit
```

A decomposition implementation must not fake cross-category chemistry using a simple same-quantity `transfer(...)` when the semantic form of matter actually changes.

## Exact population biomass

The current integer `population biomass_milligrams` is best interpreted as exact ownership of a body-mass stock at the population/stratum authority layer.

It is valuable because death, migration, reservation, collapse, and active ownership can transfer that exact quantity without rounding drift.

It is **not by itself** a complete biochemical composition vector.

Therefore:

- exact body-mass ownership may move from living to detrital authority;
- composition may initially remain modeled/coarse;
- future carbon/water/nutrient settlement requires explicit composition or a qualified closure;
- modeled composition cannot silently rewrite the exact body-mass ownership total;
- exact and approximate representations must declare their quantization/residual boundary.

## Reaction closure

Every process that changes stock categories should declare which properties it is required to conserve.

Conceptually:

```text
ReactionContract {
    reactant_stock_kinds,
    product_stock_kinds,
    conserved_properties,
    allowed_boundary_fluxes,
    quantization_policy,
}
```

Examples:

- pure migration: same stock + same exact mass, different region owner;
- death: living stock -> detrital stock, body mass preserved;
- feeding: food stock decreases, consumer stock/metabolic outputs increase;
- respiration: organic stock/property routing includes atmospheric output rather than pretending biomass category is conserved;
- decomposition: detrital stock decreases while conserved elemental/mass properties route into decomposers, soil/water pools, and atmospheric outputs.

## Atomicity

A reaction/stock conversion must be planned and validated before mutation.

The transaction should prove:

1. all reactant authority exists;
2. no reactant is double-owned;
3. product arithmetic is finite/exact as required;
4. every declared conserved property balances after explicit boundary fluxes;
5. quantization residuals have an owner;
6. no stock becomes negative;
7. the full plan can commit atomically.

Failure leaves all stocks/properties unchanged.

## Open-system accounting

An ecosystem region is generally open.

Inputs/outputs such as:

- animal migration;
- river transport;
- atmospheric gas exchange;
- harvested biomass;
- sediment export;
- imported food/fertilizer;

must be explicit boundary fluxes when they affect a property under conservation qualification.

A balance report should distinguish internal transformation from true boundary exchange.

## Avoiding double counting

The following is invalid unless a composition model explicitly says the terms are disjoint:

```text
body mass + carbon mass + water mass + nutrient mass = total mass
```

Carbon and water contents may already be components/properties of the body mass.

The ledger must distinguish overlapping properties from disjoint material stocks.

## Qualification fixtures

Minimum future evidence should include:

1. pure death transfers exact body-mass ownership Living -> Detritus without loss;
2. migration changes owner/location but not global exact mass;
3. decomposition can reduce detrital biomass stock while still balancing declared conserved properties;
4. a reaction that loses carbon/mass without explicit boundary output fails;
5. a reaction that overproduces a conserved property fails;
6. open-system input/output reconciles exactly/within the declared numerical contract;
7. overlapping composition properties are never summed as disjoint stock mass;
8. quantization residuals are explicit and cannot vanish;
9. failed reaction validation mutates nothing;
10. same reaction intents produce the same settlement independent of execution/thread order when paired with the qualified arbitration policy.

## Non-goals

This contract does not yet choose:

- a full elemental chemistry model;
- exact nitrogen/phosphorus species;
- microbial stoichiometric equations;
- thermodynamic free-energy accounting;
- a universal exact unit for every property;
- the final persistence schema.

It freezes the prerequisite distinction that **ecological forms/stocks may transform, while only explicitly declared physical/chemical properties are required to survive those transformations as conserved quantities**.
