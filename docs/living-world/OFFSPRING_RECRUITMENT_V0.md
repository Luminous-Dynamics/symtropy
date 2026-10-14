# Offspring Recruitment V0

## Purpose

Define the authority transition by which a committed reproductive output becomes a counted ecological recruit without duplication across population, propagule, active-individual, and presentation representations.

## Core rule

Recruitment is an authority transfer, not a spawn event.

Conceptually:

`committed reproductive output / propagule -> eligibility + habitat gate -> recruitment transaction -> exactly one ecological owner`

After commit, the recruit is owned by exactly one canonical representation appropriate to its fidelity:

- propagule/dormant bank;
- coarse population/stratum;
- Level-A active refinement;
- Level-I persistent individual where continuity already matters.

It must never simultaneously remain counted in the source bank/output and appear as a newly counted recruit.

## Count authority

Recruitment changes population count only at the canonical commit boundary.

Rendering, animation, particle effects, scene streaming, or object pooling cannot increment/decrement recruit authority.

## Extensive stock

If the ecological model tracks exact stock for the recruit, source reproductive/propagule stock and destination recruit stock must settle coherently.

No recruit may acquire exact stock from both parent investment and a second population-materialization path.

If exact individual resolution is insufficient, recruit a cohort/aggregate compatible with the authority-resolution contract rather than minting zero-stock individuals.

## Heredity

The recruit retains the hereditary result bound to its reproduction event or source propagule class.

Materialization later may derive unresolved phenotype/developmental microstate but cannot reroll canonical hereditary facts.

## Stage

The recruit enters the species-specific developmental stage appropriate to the biological pathway. Coarse `LifeStage` telemetry is a lossy mapping only.

## Habitat / establishment

Recruitment may depend on qualified establishment conditions:

- substrate;
- moisture;
- temperature/history;
- cover;
- host/symbiont availability;
- predation/herbivory pressure;
- parental/nest state;
- density/competition;
- disturbance state;
- species-specific requirements.

Failure to establish has explicit authority/stock consequences where modeled.

## Idempotency

A recruitment transaction has stable request/event identity.

Retry after lost acknowledgement must return the same committed recruit result or fail closed. It cannot create another recruit.

## Regional transfer

If recruitment crosses a region boundary through dispersal/migration, source and destination authority updates are atomic or coordinated through a transaction protocol that prevents transient double ownership.

## Population refinement

Approaching a nest, seedling patch, larval pool, or nursery cannot create more offspring than coarse truth contained.

Leaving the area cannot erase established recruits or restore them to the source bank.

## Qualification

Test:

- source count/stock decreases exactly when destination authority increases;
- retry idempotency;
- no source+destination double ownership;
- no LOD/promote/demote duplication;
- hereditary facts survive recruitment/materialization;
- stage is biologically appropriate and deterministic;
- save/reload equivalence around the commit boundary;
- failed establishment has explicit zero/defined mutation semantics;
- region partitioning does not change total recruitment where partition invariance is claimed.

## Non-claims

V0 does not define universal establishment probability, offspring mortality, carrying capacity, or species calibration.
