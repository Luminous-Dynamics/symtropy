# Propagule Bank V0

## Purpose

Define persistent ecological memory for seeds, spores, dormant eggs, cysts, vegetative propagules, and analogous dormant/recruitable life stages.

## Core rule

A region's future biota is not determined only by currently active organisms.

Canonical ecology may retain a versioned propagule bank containing dormant/recruitable biological authority that can survive disturbance, seasonal inactivity, and periods with no active representative organism.

## Examples

Depending on species/system:

- seed bank;
- spore bank;
- dormant egg bank;
- cyst/encysted stage;
- tuber/rhizome/vegetative propagule;
- buried or attached propagules;
- colony fragments;
- nearby dispersal-source summaries where qualified.

## Canonical state

A bank entry may require some combination of:

- species/heredity cluster;
- count;
- exact extensive stock where resolution permits;
- age/dormancy duration;
- viability distribution;
- dormancy stage;
- depth/substrate/spatial patch;
- parent/provenance class when future-relevant;
- environmental history sufficient for germination/hatching;
- disease/pathogen state where relevant.

Do not store only a generic `seed_count` when the enabled future processes depend on these correlations.

## Recruitment

Recruitment follows:

`qualified environmental/developmental cue + viable propagule authority -> recruitment proposal -> validation/settlement -> active/coarse offspring authority`.

A visual sprout/spawn is not recruitment authority.

## Disturbance memory

Propagule banks are a primary mechanism for ecological hysteresis.

After fire, flood, drought, excavation, contamination cleanup, or restoration, recovery should depend on surviving dormant authority and dispersal sources rather than arbitrary respawn tables.

## Dispersal

Dispersal may move propagule authority between regions/patches through explicit migration/flux semantics.

Wind, water, animal carriage, gravity, explosive dispersal, human movement, etc. may produce dispersal intents. Presentation particles are not authority.

## Dormancy

Dormancy/viability changes use authoritative time and qualified environmental history. Region unload cannot pause, reset, or rejuvenate the bank unless the chosen coarse closure explicitly models that effect.

## Fidelity

A bank may be highly aggregated when future processes require only distributions. Notable hereditary lineages or individually important propagules may require richer state.

Promotion/reduction cannot mint or lose propagules.

## Qualification

Test:

- disturbance followed by recruitment depends on surviving bank state;
- save/reload equivalence;
- offscreen coarse catch-up equivalence;
- no duplicate recruitment on retry/promotion;
- regional dispersal conserves propagule count/stock under the declared transfer semantics;
- viability/dormancy histories persist across unload;
- renderer particle count does not alter bank authority;
- identical current habitat with different bank histories can produce different future succession.

## Non-claims

V0 does not define universal seed longevity, germination thresholds, dispersal kernels, or real-species calibration.
