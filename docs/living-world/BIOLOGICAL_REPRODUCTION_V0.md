# Biological Reproduction V0

## Purpose

Define reproduction as a canonical biological/ecological transition rather than an entity-spawn convenience.

A child, seed, spore, egg, propagule, bud, clone, or other recruit may enter canonical ecology only through a qualified reproduction/recruitment path whose heredity, resource cost, timing, and authority are explicit.

## Core rule

Reproduction is not:

`spawn(entity_prefab)`

It is conceptually:

`eligible reproductive state + qualified mate/parent source(s) + reproductive investment + reproduction-event identity + heredity/recombination scheme + mutation scheme -> offspring/propagule authority`

followed by explicit ecological recruitment.

## Parent modes

The contract must support different biological strategies without forcing sexual diploid assumptions onto every species:

- biparental sexual reproduction;
- multi-parent or unusual ploidy where later required;
- selfing;
- asexual budding/fission;
- clonal/vegetative propagation;
- spore production;
- seed/egg production;
- colony/network propagules;
- phase-dependent reproduction.

Each mode is a versioned species-biological policy.

## Heredity provenance

Offspring heredity must bind canonical parent/hereditary content and an explicit reproduction event coordinate.

Conceptually:

`parent hereditary state(s) + reproduction_event + recombination_v + mutation_v -> offspring hereditary state`

Sequential runtime RNG, ECS iteration order, wall-clock entropy, renderer timing, or spawn order are not authoritative heredity inputs.

The existing `PhenotypeSeed` remains a derivation coordinate and is not silently promoted into genome, ancestry, parentage, or organism identity.

## Reproduction event identity

The reproduction event coordinate must be stable enough that retries cannot create multiple genetically distinct offspring from the same committed reproductive event.

Idempotent retry of one committed event must resolve to the same recruitment result or fail closed; it must never mint an additional offspring because an acknowledgement was lost.

## Resource-paid reproduction

Reproductive output requires explicit biological investment appropriate to the species/model:

- carbon/biomass;
- water;
- nutrients;
- stored energy;
- gamete/seed/egg tissue;
- parental physiological cost;
- nest/den substrate;
- gestation/incubation/lactation or analogous investment;
- symbiotic contributions where modeled.

A reproduction proposal may be denied or reduced by scarcity. Presentation state cannot bypass these costs.

## Timing

Eligibility and commitment occur on authoritative simulation time and may depend on:

- developmental stage;
- phenology;
- physiological condition;
- season/climate history;
- social/mate state;
- pollination/fertilization state;
- resource availability;
- dormancy/diapause;
- species-specific reproductive windows.

Local wall clock, UI date, animation completion, or renderer frame rate cannot create reproductive authority.

## Atomicity

Any transition that changes parent investment, offspring/propagule count, hereditary authority, exact stock ownership, or recruitment state must commit atomically across the participating authorities or not mutate.

Do not implement:

`subtract parent resources -> spawn child -> later fix population count`.

## Death / failed reproduction

Failed fertilization, seed abortion, egg loss, miscarriage, propagule mortality, etc. must have explicit biological/stock semantics when modeled. They are not ordinary object deletion.

## Qualification

At minimum test:

- exact replay of heredity under identical parent/event inputs;
- different event identity can produce different allowed recombination without sequential RNG;
- retry idempotency;
- no offspring from unpaid reproduction;
- stage/phenology gating;
- parent resource + offspring/propagule settlement atomicity;
- save/reload equivalence;
- population refinement/LOD cannot duplicate offspring;
- renderer absence does not prevent or change reproduction.

## Non-claims

V0 does not choose a universal genome representation, Mendelian model, mutation rate, mating system, fertility curve, or species calibration.
