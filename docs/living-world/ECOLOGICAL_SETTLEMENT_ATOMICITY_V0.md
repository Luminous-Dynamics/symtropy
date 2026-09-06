# Living World Ecological Settlement Atomicity v0

Status: companion authority contract. This document defines how population state, persistent organisms, and conserved ecological quantities compose without creating duplicate authority or partial updates.

## Problem

Living World now has two useful but potentially dangerous representations of biological quantity:

- population state can carry exact living biomass;
- the ecological conservation ledger can carry biomass in a `Living` compartment.

If both are treated as independent canonical totals, the same matter can be counted twice or drift apart.

A mortality operation can also fail halfway:

```text
population count/biomass reduced
        X
ledger transfer to detritus fails
```

or the reverse.

A realistic ecosystem cannot tolerate those partial states.

## Single-authority accounting theorem

The same ecological quantity must not have two independent canonical balances.

When detailed biological state and a conservation ledger are composed, one of the following must hold:

1. the ledger total is derived from authoritative subaccounts such as populations/persistent organisms; or
2. detailed state is transactionally bound to corresponding ledger accounts so both change as one atomic authority transition.

A periodically reconciled pair of independent mutable numbers is not sufficient authority.

## Hierarchical living accounts

The preferred long-term model is hierarchical accounting.

Conceptually:

```text
BiomassMass
|
+-- Living
|   |
|   +-- exchangeable population A
|   +-- exchangeable population B
|   +-- persistent organism P
|   +-- persistent organism Q
|
+-- Detritus
+-- Soil
+-- WaterColumn
+-- Storage
+-- external boundary
```

The exact account identity probably belongs in a higher-level `symtropy-ecology` authority layer rather than the permissive low-level life core.

The invariant is independent of implementation location:

```text
Living total = sum(all authoritative living subaccounts)
```

not a separately mutable duplicate.

## Crystallization is an ownership transfer

Promoting one derived/reserved member into persistent organism identity must not create biomass.

Conceptually:

```text
Living / exchangeable population
    -- exact member biomass -->
Living / persistent organism
```

The ecological compartment remains `Living`; only the authority owner changes.

Required invariants:

```text
population_count decreases by 1
population_biomass decreases by member_biomass
persistent_organism_count increases by 1
persistent_organism_biomass = member_biomass
regional_living_biomass unchanged
```

The transition must become authoritative exactly once.

## Demotion is the reverse transfer

If a persistent organism can safely lose individual authority, demotion is:

```text
Living / persistent organism
    -->
Living / exchangeable population
```

again with no living-mass creation or destruction.

Demotion is forbidden when required individual history cannot be represented by the target population/summary state.

## Mortality settlement

Death changes both biological organization and ecological compartment.

For an exchangeable coarse mortality event:

```text
Living / population
    -- count reduction + released biomass -->
Detritus / carrion-or-litter pathway
```

For a persistent organism:

```text
Living / persistent organism
    -->
Detritus / identified remains or aggregate detritus
```

The identity/history record may persist after death, but its living biomass authority does not.

The amount removed from living biological state must exactly equal the amount deposited into detritus or explicitly exported across the modeled system boundary.

## Reproduction settlement

Birth changes count and creates a new living allocation, but it must not create unexplained matter/energy.

Depending on model fidelity, offspring biomass may come from:

- explicit parent/reproductive reserves;
- egg/seed/propagule storage already represented as living or stored biomass;
- modeled resource assimilation over gestation/growth;
- an explicit external biological input at a world boundary.

The settlement must identify the source. A bare `count += 1` with unexplained biomass is not canonical reproduction.

## Migration settlement

Movement between regions changes account ownership/location rather than global biomass.

Conceptually:

```text
Region A / Living / population
    -->
Region B / Living / population
```

For a closed two-region accounting boundary:

```text
global living count unchanged
global living biomass unchanged
A decreases exactly as B increases
```

If a population crosses the modeled world boundary, the transfer becomes explicit external output/input instead.

## Atomic settlement

Any operation spanning multiple canonical structures must be prepared and validated before mutation is committed.

Conceptually:

```rust
pub struct EcologicalSettlement {
    population_delta: PopulationDelta,
    account_transfers: Vec<AccountTransfer>,
    persistent_delta: PersistentOrganismDelta,
}
```

The exact types may differ, but execution follows:

```text
construct proposed settlement
        |
        v
validate all source balances
validate count/biomass arithmetic
validate destination capacity/schema
validate authority exclusivity
validate causal preconditions
        |
        +---- failure ---> no canonical mutation
        |
        v
commit all state changes once
```

No externally observable canonical intermediate state may represent only half of the settlement.

## Functional update preference

Where practical, prefer building new validated values and swapping/committing them as a unit over mutating multiple structures incrementally.

For example:

```text
(old_population, old_accounts)
        |
        | pure/preflight calculation
        v
(new_population, new_accounts, receipt)
        |
        | commit
        v
canonical world state
```

This naturally supports fail-closed behavior and deterministic replay.

## Settlement receipts

A successful settlement should eventually yield a compact receipt suitable for causal/persistence integration.

A receipt may include:

- authoritative simulation tick;
- settlement kind;
- source/destination account scopes;
- count delta;
- exact typed quantity deltas;
- crystallized/demoted identity references when applicable;
- pre-state and post-state summary digests once canonical digest infrastructure exists.

This low-level contract does not bind receipts to game-state event v2 yet. That integration remains deferred until the relevant canonical-event work is qualified.

## No renderer settlement authority

Rendering/ECS systems may request presentation changes but cannot originate ecological account transfers merely because an entity spawned/despawned.

Therefore:

```text
Bevy entity despawn != organism death
LOD demotion != population transfer
mesh destruction != biomass destruction
animation state != physiology authority
```

A renderer observes a committed settlement; it does not constitute one.

## Relationship to terrain/destruction

Physical disturbance may trigger ecological settlement, but the causal chain must remain explicit.

For example:

```text
ExcavationEvent
 -> root damage
 -> plant mortality settlement
 -> Living biomass decreases
 -> Detritus biomass increases
 -> later decomposition
 -> soil/mineral pathways
```

The excavation renderer or voxel-remesh operation itself is not the mortality authority.

## Account scope

Low-level `EcologicalCompartment` is useful for typed conservation but does not currently distinguish every population, organism, region, or species account.

Before cross-population settlement becomes product-authoritative, the higher ecology layer should add semantic account scope.

A conceptual account key might include:

```text
quantity
region
compartment
owner-kind
owner-id-or-population-scope
```

The final representation must avoid forcing game-state identity dependencies into reusable low-level crates unless that dependency is explicitly justified.

## Numerical representation

Population biomass currently uses integer milligrams while the general conservation ledger uses `f64` quantities.

Composition must define a lossless or explicitly bounded conversion rule before one can qualify the other.

For biomass settlement, prefer an exact integer accounting path at the population/persistent-organism layer and derive/convert higher-level telemetry as needed.

Do not silently round an organism out of existence or create biomass through repeated float<->integer conversion.

A future ledger revision may introduce fixed-point typed quantities or exact subaccount balances for biological mass.

## Cross-model reconciliation invariant

At every qualified checkpoint, authoritative biological allocations must reconcile with conservation accounting.

For a region with exchangeable populations `P`, persistent living organisms `I`, and any other explicitly modeled living pools `O`:

```text
regional_living_biomass
=
sum(biomass(P))
+ sum(biomass(I))
+ sum(biomass(O))
```

within the exact representation or an explicitly declared conversion tolerance.

If that equality fails, the world state is not qualified.

## Qualification requirements

Before ecological settlement is considered qualified, evidence must establish:

1. failed settlement preflight leaves every participating state byte/semantically unchanged;
2. crystallization preserves regional living biomass exactly;
3. demotion preserves regional living biomass exactly;
4. mortality removes exactly the biomass deposited into detritus/export;
5. reproduction names and balances its biomass source;
6. closed-boundary migration conserves global count and biomass;
7. no persistent organism remains simultaneously counted in its source exchangeable population;
8. no settlement can overflow count or fixed-point biomass arithmetic;
9. repeated settlement/reversal fixtures do not accumulate conversion drift;
10. renderer/ECS spawn/despawn cannot invoke canonical accounting mutation through presentation-only APIs;
11. save/reload between completed settlements yields the same authoritative balances as uninterrupted execution;
12. eventual causal receipts replay to identical post-state balances.

## Observatory scenarios

Living World Observatory should include at least:

### Crystallization round trip

```text
coarse population
 -> reserve
 -> materialize
 -> crystallize one member
 -> persistent organism
 -> safe demotion
 -> recombine
```

With no other ecological event, all promised coarse observables and regional living biomass must recover exactly.

### Mortality chain

```text
living member
 -> death
 -> detritus
 -> decomposition
 -> nutrient transfer
```

No biomass may vanish merely because representation changes.

### Migration chain

Move a cohort A -> B -> A and prove exact global count/biomass conservation plus deterministic regional recovery.

### Failure atomicity

Intentionally make the destination settlement invalid after constructing a valid source change. The entire transaction must fail without mutating source or destination authority.

## Design principle

**Ecological conservation is not only about totals. It is about ownership of conserved quantity across authority transitions. A living world becomes trustworthy when every organism, population, corpse, nutrient pool, and migration can answer both “what changed?” and “where did the matter go?”**
