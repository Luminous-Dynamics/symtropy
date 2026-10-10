# Plant Canopy and Root Competition V0

Status: normative Living World design contract. Documentation only.

## Purpose

Define how plant structure produces spatial competition for light, water, nutrients, and physical space without requiring all-pairs organism interactions or letting render geometry become ecological authority.

## Core invariant

> Canonical plant structure contributes qualified ecological influence; habitat/competition fields are derived summaries, not independent plant truth.

## Canopy influence

Canonical foliage/branch state may contribute to spatial summaries such as:

- light interception / shading;
- canopy occupancy;
- rain interception where modeled;
- wind shelter/exposure;
- local transpiration demand;
- habitat/cover structure.

A rendering mesh, shadow map, screen-space occlusion buffer, or camera-visible leaf count is not canonical canopy authority.

## Root-zone influence

Canonical root structure may contribute to spatial summaries such as:

- root occupancy/density;
- water uptake demand;
- nutrient uptake demand;
- soil engineering/compaction interactions;
- mycorrhizal/symbiotic interfaces;
- physical root competition.

The first implementation may use root-zone aggregates rather than individual fine roots, provided enabled processes remain insensitive to the discarded microstructure.

## Derived field lifecycle

Conceptually:

`canonical plant structure + qualified process state -> derived canopy/root influence -> HabitatSampler / competition query -> organism process intents`

Derived influence is rebuildable from canonical source state. It cannot independently mint plant biomass, roots, leaves, or developmental history.

## Avoiding all-pairs competition

Plants should normally compete through shared spatial summaries and local habitat queries rather than O(N^2) direct pair tests.

Candidate representations include:

- surface/canopy grids;
- sparse patches;
- spatial hashes;
- hierarchical tiles;
- local influence kernels;
- canopy height/leaf-area summaries;
- root-zone occupancy summaries.

The representation is an optimization only if it preserves the process-required observables within its qualified error bounds.

## Self-shading and neighbor shading

Self-competition and neighbor competition are both biological.

A plant's own crown may shade lower foliage; neighboring plants may create directional gaps. When directional light opportunity affects architecture, the retained field must preserve enough directionality for the corresponding meristem/tropism process.

A single scalar light value is insufficient for directional crown development if direction changes future growth.

## Resource scarcity

Spatial uptake fields may generate exact/approximate process intents, but contested finite stock is settled through the canonical scarcity/stock authority system.

Multiple overlapping root zones cannot each consume the full same water/nutrient quantity because they read the same habitat value.

## Feedback loop

The intended loop is:

`plant structure -> competition fields -> habitat opportunity -> developmental/physiological response -> resource-paid growth -> new plant structure`

The loop advances on authoritative ecology cadence, not renderer frames.

## LOD / collapse

A distant forest may use aggregate canopy/root summaries if those summaries remain sufficient for every active landscape process.

Promotion to detailed plants must not change total canonical canopy/root influence merely because more geometry appears.

Likewise, demotion must not erase competitive history that changes succession or future growth.

## Qualification direction

At minimum test:

1. identical canonical plant structure produces identical competition summaries independent of renderer state;
2. adding a plant changes local light/root opportunity in the expected spatial region;
3. removing/severing foliage/root structure invalidates derived influence deterministically;
4. overlapping resource demands cannot double-consume exact finite stock;
5. fine and coarse canopy/root representations preserve declared observables within qualified bounds;
6. promotion/demotion leaves canonical plant stocks and developmental history unchanged;
7. a canopy-gap fixture produces deterministic directional opportunity and later crown response;
8. field rebuild after save/reload reproduces the same habitat query results for a frozen implementation version.

## Non-goals

This contract does not define a universal radiative-transfer model, full soil-root hydraulics, individual-root collision simulation, or species-specific competition coefficients.