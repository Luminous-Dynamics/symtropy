# Plant Presentation LOD V0

Status: normative Living World design contract. Documentation only.

## Purpose

Define plant-specific presentation fidelity so geometry/detail may scale aggressively without changing plant biology, causal appearance, or interaction authority.

## Core invariant

> Plant presentation LOD changes representation cost, not plant truth.

Changing camera distance, GPU budget, screen resolution, frame rate, occlusion, or renderer backend cannot alter canonical plant structure, stocks, damage, phenology, developmental history, or exact ecological influence.

## Candidate presentation tiers

A renderer may implement tiers such as:

- **P0 Aggregate** — canopy/stand proxy or distant landscape representation;
- **P1 Impostor/Cluster** — trunk/crown proxy, clustered foliage, causal material summary;
- **P2 Structured** — explicit major branch structure, instanced foliage clusters;
- **P3 Interactive** — detailed branch/organ presentation, high-quality material state, local deformation;
- **P4 Hero** — maximum supported geometry/material/secondary-motion detail.

Names and thresholds are implementation-specific. These are presentation tiers, not ecological fidelity levels.

## Distinguish presentation from ecological fidelity

A plant may be visually P0 while remaining ecologically rich if an active process requires its detailed structural state.

Conversely, a nearby decorative plant may be visually detailed while ecological processes operate on a qualified coarse closure.

Do not collapse these two axes into one `lod` integer.

## Causal cue retention

LOD simplification should preserve visually important causal state where perceptually relevant, including:

- major asymmetry;
- missing limbs;
- dead crown regions;
- severe scars/wounds;
- phenological phase;
- large fruit/flower masses;
- disease/senescence regions;
- landmark/ancient-tree silhouette;
- major wetness/fire/deadwood state.

Fine bark pores or individual leaves may disappear long before these cues do.

## Transition stability

LOD transitions must not reshuffle stable plant presentation unnecessarily.

For a fixed canonical source revision and renderer scheme, moving across an LOD boundary should preserve:

- major branch attachment relationships;
- silhouette anchors;
- causal damage locations;
- material-state classes;
- phase/seed coordinates used by secondary motion.

Cross-fade/dither/geometry morphing are presentation techniques only.

## Interaction promotion

If the player targets/cuts/inspects a plant while a coarse visual representation is active, the canonical interaction resolves against current plant authority, not the impostor pixels.

The renderer may request richer presentation before interaction, but visual promotion cannot create new branches or heal/remove damage.

## Ecological influence independence

Canopy/root competition, habitat effects, resource uptake, wind loading, and other canonical/qualified ecological processes do not derive from whichever render LOD is currently active.

An impostor tree casts ecological shade only through the canonical/derived canopy authority pipeline, not because its billboard happens to cover pixels.

## Budget policy

Presentation fidelity may depend on:

- projected screen size;
- distance;
- salience;
- interaction likelihood;
- landmark importance;
- visual budget;
- capture/cinematic mode.

These may change cost/quality, never canonical biology.

## Qualification direction

At minimum test/measure:

1. repeated P0→P4→P0 cycles leave canonical plant state byte-for-byte/hash-equivalent;
2. major silhouette/damage anchors remain spatially coherent across tiers;
3. interaction promotion cannot change topology until a canonical command commits;
4. headless ecology and all presentation tiers produce identical canonical simulation results;
5. ecological canopy/root influence is invariant to render tier;
6. severe causal material cues remain represented at qualified distant tiers;
7. renderer backend/quality setting changes cannot change plant development;
8. LOD thrashing cannot accumulate presentation-state drift that feeds back into canonical state.

## Non-goals

This contract does not choose exact distance thresholds, polygon budgets, billboard technology, meshlet strategy, or renderer-specific transition effects.