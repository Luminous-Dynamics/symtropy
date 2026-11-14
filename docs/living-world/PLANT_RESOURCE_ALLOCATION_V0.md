# Plant Resource Allocation V0

Status: normative Living World design contract. Documentation only; no claim that Rust currently implements this model.

## Purpose

Define how a canonical plant partitions acquired resources among maintenance, repair, roots, stems, foliage, storage, reproduction, and symbiotic exchange without allowing developmental morphology to mint biomass or bypass ecological scarcity.

The key rule is:

> Plant form is a consequence of settled allocation decisions, not a direct procedural resize.

## Authority boundary

The plant structural graph owns persistent biological structure. External habitat fields and symbionts may propose resource availability; developmental programs may propose allocation preferences; only an atomic growth/maintenance settlement may change canonical plant stocks or structure.

Presentation has no authority over allocation.

## V0 allocation domains

At minimum distinguish these sinks:

- maintenance;
- repair / compartmentalization;
- root extension / thickening;
- stem and branch extension;
- structural thickening / support tissue;
- leaf / photosynthetic tissue;
- storage reserves;
- reproductive structures;
- symbiotic exchange.

A future species schema may omit unsupported sinks or add versioned ones.

## Allocation transaction

Conceptually:

`available exact stocks + physiology + developmental state + structural demand -> allocation intents -> scarcity arbitration -> exact settlement -> structural/physiological commit`

No structural append/thickening operation is committed before the resources that pay for it are committed in the same ecological transaction.

## Priority is biology, not iteration order

Competition among sinks must not be resolved by vector order, ECS system order, hash-map order, or renderer proximity.

Species/development policy may define explicit priorities, minimum maintenance floors, reserve thresholds, reproductive strategy, stress reallocations, and source/sink constraints. Those choices are biological policy and therefore versioned content, not rounding side effects.

## Root–shoot coupling

Root and shoot development are coupled through resource acquisition and allocation but are not forced into one scalar ratio.

Examples:

- chronic drought may increase future root allocation while reducing leaf expansion;
- persistent shade may increase height/internode investment while changing leaf area and reducing lower-crown maintenance;
- wind exposure may increase structural thickening at the expense of extension;
- nutrient abundance may relax root-foraging allocation but does not automatically create shoot biomass.

The exact response is species/content specific.

## Storage and delayed consequences

Storage reserves are canonical future-bearing state when they affect survival, regrowth, reproduction, dormancy, or stress recovery.

A coarse representation may aggregate reserves only if every enabled process remains insensitive to the discarded distribution. Otherwise collapse must enrich the coarse state or preserve identity under the existing Living World information-sufficiency rules.

## Source–sink locality

V0 may initially use plant-level aggregate allocation, but the structural graph must not prevent future local source–sink transport.

When local transport matters, the model may introduce versioned hydraulic/carbon/resource paths along the graph. A renderer-derived branch distance is not an acceptable substitute for canonical transport topology.

## Exact vs modeled quantities

Exact conserved stock ownership and modeled physiological concentrations remain distinct.

A plant may have:

- exact integer carbon/material authority;
- modeled water potential;
- modeled sugar concentration;
- modeled tissue density.

Those values may interact through qualified processes but must not be silently substituted for each other.

## Failure semantics

An allocation plan fails closed if:

- a required source stock is insufficient;
- exact arithmetic overflows;
- an allocation target is stale or no longer exists;
- a structural precondition changed before commit;
- process policy/schema no longer matches;
- a cross-model transfer cannot commit atomically.

Failure leaves plant stocks, structural topology, and allocation history unchanged.

## Qualification direction

At minimum test:

1. total settled sink allocation never exceeds exact available stock;
2. sink order permutations cannot change results absent an explicit policy change;
3. drought-vs-control histories produce deterministic but different future allocation traces under an authored fixture;
4. save/reload preserves storage and subsequent allocation exactly;
5. failed growth settlement leaves both structure and stocks unchanged;
6. changing renderer LOD/FPS cannot change allocation;
7. structural thickening cannot appear without corresponding settled structural investment;
8. local/coarse execution agrees when the declared aggregate allocation state is a qualified sufficient statistic.

## Non-goals

This contract does not define universal plant physiology, photosynthetic chemistry, xylem/phloem mechanics, species calibration values, or render geometry.