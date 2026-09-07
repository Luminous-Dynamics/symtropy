# Plant Phenology V0

Status: normative Living World design contract. Documentation only.

## Purpose

Define seasonal/developmental plant-state transitions as biological process state rather than texture swaps or wall-clock animation.

Phenology controls when a plant can invest in buds, leaves, flowers, fruit, seed, dormancy, senescence, and related processes.

## Core invariant

> Phenological state is driven by qualified biological/environmental history on authoritative simulation time.

Renderer time, frame count, local machine date, or visual season settings cannot directly advance canonical phenology.

## Candidate V0 phases

Species may support a versioned subset of:

- Dormant;
- BudSwelling;
- LeafOut;
- Vegetative;
- Flowering;
- Fruiting;
- SeedMaturation;
- Senescing;
- Leafless / post-senescence.

These are biological phase labels, not necessarily universal botanical truth. Species definitions own supported phases and transitions.

## Driver classes

Phenology may depend on qualified drivers such as:

- accumulated warmth / degree-time;
- chilling exposure;
- photoperiod or qualified astronomical day-length;
- drought / water stress history;
- nutrient/energy reserves;
- developmental age/stage;
- disturbance/damage state;
- species/genotype-specific thresholds.

Every driver that affects future state follows the developmental-stimulus provenance rules: source authority, units/normalization, integration semantics, cadence/window, and calibration version are explicit.

## Transition semantics

Transitions are evaluated from canonical state and committed deterministically.

A transition may be:

- threshold crossing;
- hysteretic threshold pair;
- cumulative exposure gate;
- conjunction/disjunction of qualified conditions;
- explicit disturbance-driven transition.

Transition grammar is versioned. A future change in threshold/ordering semantics is not silently applied to an existing saved biological lineage without migration policy.

## Hysteresis

Phenology often requires hysteresis. The same current temperature/moisture does not imply the same state if the preceding history differs.

A plant that has completed chilling and accumulated spring warmth may be in LeafOut while another plant under identical instantaneous weather remains Dormant.

Therefore current HabitatSample alone is not generally a sufficient phenological state.

## Structural and allocation effects

Phenology gates future actions rather than directly mutating visuals.

Examples:

- LeafOut may authorize creation/expansion of new leaf organs when resources settle;
- Flowering may enable reproductive allocation and pollination affordances;
- Fruiting may enable fruit growth and later dispersal;
- Senescence may transfer/remobilize resources before leaf abscission;
- Dormancy may suppress extension growth while preserving maintenance processes.

No leaf/flower/fruit appears canonically unless the corresponding biological action commits.

## Ecology interaction

Phenology changes ecosystem affordances:

- nectar/pollen availability;
- fruit/seed food availability;
- canopy light interception;
- transpiration demand;
- leaf-litter timing;
- pollinator interaction windows;
- seed dispersal timing;
- habitat/cover quality.

Thus seasonal visual change should emerge from canonical ecological state rather than being a separate rendering-only seasonal controller.

## Coarse execution

Offscreen phenology may use coarse catch-up only when the retained history/statistics are qualified as sufficient for the species' transition grammar.

If an order-sensitive event or threshold crossing could be lost by coarse aggregation, the representation must retain the required statistic/event boundary or remain at richer fidelity.

## Save/reload and replay

Saved state must retain enough phenological phase and driver-history state that uninterrupted execution and save/reload produce the same future transition sequence.

Re-entering a region cannot restart chilling, degree-time, flowering duration, or senescence progress unless that reset is explicit biology.

## Presentation

Rendering consumes canonical phase plus organ/structural state to derive:

- bud appearance;
- leaf emergence and maturation;
- flower display;
- fruit maturation;
- senescence color/material changes;
- leaf fall;
- dormant silhouette.

Presentation may interpolate visual transitions smoothly, but cannot create canonical reproductive availability or advance phase authority.

## Qualification direction

At minimum test:

1. same history -> identical phenological transition ticks;
2. same instantaneous weather but different accumulated history -> different phase where expected;
3. fine execution == qualified coarse catch-up for cumulative driver fixtures;
4. save/reload preserves the next transition exactly;
5. renderer/FPS/local date cannot affect phase;
6. flowering/fruit affordances exist only in authorized phases;
7. resource starvation can delay/suppress organ production without rewriting the phenological driver history;
8. a seasonal cycle produces structural/ecological consequences, not only visual swaps.

## Non-goals

This contract does not define universal plant-calendar parameters, real-world species calibration, astronomical climate simulation, pollination implementation, or rendering shaders.