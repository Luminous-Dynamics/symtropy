# Derived Ecological Field Authority V0

## Status

Normative Living World authority contract. This document clarifies the role of current `FieldGrid`, Basin, Colony, and Mycelium scalar fields relative to future exact ecological stocks and settlements. It does not invalidate their current deterministic simulation use.

## Problem

A field can be deterministic without being a physically conserved quantity.

Current Living World-adjacent systems contain channels/states such as:

- `FieldLayer::Biomass`;
- `FieldLayer::Nutrient`;
- `FieldLayer::Toxin`;
- moisture, heat, oxygen, disease, signals;
- Basin water/soil/toxin indices;
- Mycelium nutrient/biomass activity;
- pheromone and danger fields.

Many are normalized, heuristic, domain-scaled, or behavior-oriented `f32` values. They are extremely useful, but a value named `Biomass` or `Nutrient` must not automatically be interpreted as exact kilograms/moles/milligrams of canonical matter.

## Field classes

Every future field that can influence canonical ecology should declare one of three semantic classes.

### F-S — signal field

Carries information/intensity rather than material authority.

Examples:

- pheromone intensity;
- bird alarm;
- signal noise;
- danger cue;
- fungal pulse.

Diffusion/decay may destroy signal intensity without implying loss of physical matter.

### F-I — ecological index / derived availability field

Carries a normalized or domain-scaled estimate used for behavior, suitability, stress, or approximate rates.

Examples may include:

- nutrient availability index;
- heuristic biomass/activity density;
- toxin pressure/index;
- habitat quality;
- decomposer capacity.

Its numerical value is not canonical stock ownership unless an explicit adapter says otherwise.

### F-P — physical field

Carries a dimensioned physical quantity/concentration with declared units, geometry, and conservation semantics.

A physical field must define at least:

- quantity/property represented;
- unit;
- cell geometry/volume/area semantics;
- boundary conditions;
- numerical update rule;
- conservation/error tolerance;
- relationship to exact stock/ledger authority;
- quantization/residual policy when crossing exact integer boundaries.

Only F-P fields may directly claim physical stock/property authority, and only within their qualified numerical contract.

## Default interpretation of current V0 fields

Until a stronger typed adapter explicitly upgrades them, current shared `FieldGrid` channels and Basin/Mycelium `f32` state are treated as F-S or F-I according to domain meaning—not as exact canonical matter ownership.

In particular:

- `MyceliumWorld::add_dead_biomass(..., nutrients)` currently adds to `FieldLayer::Nutrient`; it must not be used as an exact corpse-mass settlement API;
- Mycelium toxin buffering and nutrient release are heuristic ecological transforms, not yet a stoichiometrically qualified reaction ledger;
- field biomass decay does not by itself prove where exact matter went;
- Basin `MetabolicFlux` values are ecological process/index telemetry until units and settlement semantics are explicit.

This is a clarification of authority, not a criticism of the current deterministic reference models.

## One-way derived-field pattern

Preferred integration with canonical stock authority:

```text
canonical stocks / habitat / exact settlement
                 |
                 v
      derived ecological fields
                 |
       perception / rate proposal
                 |
                 v
          typed flux intents
                 |
                 v
       canonical arbitration
                 |
                 v
        atomic exact settlement
                 |
                 v
          canonical stocks
                 |
                 +----> refresh derived fields
```

The field helps organisms decide what to do. The field does not independently mint or destroy canonical matter.

## Feedback rule

A derived field may influence canonical state only through an explicit process boundary that converts the field observation into a typed intent and then settles against canonical authority.

Forbidden pattern:

```text
read heuristic nutrient field
-> directly add exact organism biomass
```

Required pattern:

```text
read nutrient availability index
-> propose uptake demand
-> arbitrate against exact nutrient/substrate authority
-> commit exact/qualified stock reaction
-> update organism stock
-> refresh field
```

## Death and decomposition seam

The first exact mortality path should not call current heuristic `add_dead_biomass` with an integer corpse mass and assume conservation.

Instead:

```text
Level-A living exact stock
      -> exact detritus/carrion stock
      -> derived substrate/odor/nutrient-availability fields
      -> scavenger/decomposer intents
      -> qualified reaction settlement
```

This preserves the useful Mycelium/Basin behavior models while preventing them from becoming accidental matter authorities.

## Numerical decay

For F-S/F-I fields, diffusion and decay are semantic signal/index operations. No physical sink is required unless the field claims a physical quantity.

For F-P fields, any numerical decay term must correspond to one of:

- transfer into another modeled pool/property;
- explicit boundary output;
- a declared reaction product;
- an error term bounded and accounted by the numerical qualification contract.

Physical quantity may not simply disappear because a convenient field update uses `decay > 0`.

## Cross-domain coupling

Copying/scaling one field into another domain does not transfer canonical stock authority.

For example, Basin nutrient index feeding Mycelium nutrient index is a derived coupling unless a separate settlement transaction moves actual nutrient stock.

This permits cheap multiscale ecological signaling without double-spending material resources.

## Units and names

Names such as `Biomass`, `Nutrient`, `Water`, or `Toxin` are insufficient to establish physical units.

Future APIs should prefer names/types that expose semantics where ambiguity matters, for example:

- `NutrientAvailabilityIndex`;
- `MycelialActivityDensity`;
- `ToxinPressure`;
- or typed `Concentration<Q, Unit>` for qualified physical fields.

A migration can be additive; current field names need not be broken immediately.

## Qualification fixtures

Minimum future evidence should include:

1. F-S/F-I field diffusion/decay cannot directly alter exact stock totals;
2. changing only a derived field without settling an intent cannot create exact organism biomass;
3. exact corpse mass transfers to detritus before any heuristic field pulse is emitted;
4. two domains may observe/copy one derived field without duplicating canonical material authority;
5. a physical field declares unit and cell geometry;
6. F-P numerical update satisfies its explicit conservation/error bound;
7. exact-to-field and field-to-intent adapters have deterministic/versioned semantics;
8. failed canonical settlement leaves both authority and published next-generation fields consistent with the pre-commit state;
9. field update order does not determine same-tick scarce exact allocation;
10. rendering may read any field class but never becomes its authority owner.

## Non-goals

This contract does not immediately redesign `FieldGrid`, Basin, Colony, or Mycelium. It does not select SI units for every ecological variable or require exact arithmetic for presentation/signal fields.

It freezes the rule that **deterministic ecological fields are observations/signals/approximations unless they explicitly earn physical authority through units, geometry, and conservation qualification**.
