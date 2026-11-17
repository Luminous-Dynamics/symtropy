# Living World Population Spatial Structure v0

Status: companion authority contract. This document defines when coarse occupancy counts are sufficient and when a Living World population must retain richer spatial structure.

## Problem

A coarse population may record exactly that 40 organisms occupy one ecological cell and 60 occupy another.

That does **not** determine how those organisms are arranged inside either cell.

The same occupancy counts can represent:

- a tight herd around water;
- evenly dispersed territorial animals;
- two family groups;
- a nesting colony concentrated at one edge;
- a disease cluster around one contact source;
- saplings concentrated in one canopy gap;
- a fungal front advancing along a moisture boundary;
- mature trees spaced by competition;
- uniform independent points.

Those microstates can produce different predation, disease, mating, pollination, competition, fire, shade, collision, visibility, and resource-access futures.

Therefore **occupancy count is not automatically spatial sufficiency**.

## Core theorem

A spatial representation is sufficient only for the authoritative processes whose future outcomes are insensitive to the spatial information it discards, within an explicit qualified error bound.

If two fine spatial states reduce to the same coarse spatial state but an enabled authoritative process produces materially different futures, the coarse representation is not sufficient for that process.

This is the spatial specialization of `POPULATION_SUFFICIENT_STATISTICS_V0.md` and `PROCESS_INFORMATION_REQUIREMENTS_V0.md`.

## Spatial capability levels

The exact Rust types may differ, but Living World should reason about spatial information capabilities rather than one global "position fidelity" switch.

### S0 — occupancy counts

Stores only exact count per coarse ecological cell.

Suitable for processes whose semantics depend only on cell totals or cross-cell flux after qualification.

Examples may include:

- broad regional abundance;
- coarse migration between cells;
- cell-total grazing pressure under a qualified closure;
- regional biomass accounting.

S0 does not imply any within-cell point arrangement.

### S1 — low-order spatial moments

May retain information such as:

- centroid;
- spread/covariance;
- directional anisotropy;
- edge/center bias;
- vertical/depth band distribution where relevant.

This can represent broad concentration without explicit clusters or individuals.

### S2 — sparse patch/cluster structure

Retains a bounded set of ecologically meaningful spatial components such as:

- herd/group patches;
- nesting colonies;
- vegetation stands;
- canopy gaps;
- nursery patches;
- fungal fronts;
- disease clusters;
- territory regions;
- shoreline/boundary-following populations.

Each component may carry count, exact extensive quantities, moments, and process-specific attributes.

### S3 — explicit relational/spatial microstructure

Retains richer structure when processes require it, for example:

- contact/network edges;
- neighborhood graph;
- explicit patch adjacency;
- territorial ownership graph;
- plant competition neighborhoods;
- burrow/nest topology;
- host-vector contact structure.

S3 still need not mean one fully simulated organism entity per population member.

### S4 — Level-A exact active positions

Temporary canonical active refinement owns exact local microstate needed by authoritative interactions.

Examples include exact positions used for:

- canonical collision/contact;
- pursuit/evasion;
- direct predation;
- player interaction;
- exact local resource use;
- physical disturbance;
- injury.

S4 is Level A authority, not presentation state.

## Spatial state is process-relative

Distance does not determine the required spatial level by itself.

Examples:

- a distant epidemic may require S2/S3 contact structure;
- a nearby decorative flock may remain Level-P projection over S0/S1 truth if it cannot affect ecology;
- a remote fire front may require explicit vegetation patch adjacency;
- a named tracked animal may remain Level I with exact position at arbitrary distance.

The runtime should choose the cheapest representation that satisfies every enabled process contract.

## Level-P projection

Presentation projection may synthesize positions that are not canonical.

However it must label them as projected state and obey the strongest spatial information actually available.

If only S0 occupancy counts exist, a default independent-uniform jitter model is **not automatically biologically true**. It is merely one possible presentation closure.

A spatial projection model should therefore be explicit and versioned, for example conceptually:

```text
SpatialProjectionModel
    - UniformWithinCellV1
    - MomentMatchedV1
    - ClusterDescriptorV1
    - StandProcessV1
    - TerritoryAwareV1
```

The concrete enum may differ.

A model may be used only where its assumptions are acceptable for the presentation or process claim being made.

## Presentation must not create canonical spatial structure

Renderer placement, GPU instance layout, animation avoidance, impostor packing, or camera-dependent visibility cannot become the hidden source of ecological position authority.

A Level-P animal may move visually for animation/avoidance while canonical coarse truth remains unchanged.

If that movement must affect the world, the organism must first cross the explicit Level-A realization boundary.

## Prospective projection and realization

When a visible Level-P candidate is later targeted by a canonical action, realization should preserve the spatial candidate the user/agent interacted with when it remains compatible with current authority.

Conceptually:

```text
projected candidate
    scope + revision + scheme + candidate index
    projected spatial descriptor
            |
            | authoritative interaction
            v
validate source revision and spatial compatibility
            |
reserve compatible coarse/stratum authority
            |
realize matching Level-A position/microstate
            |
resolve canonical interaction
```

If the source has changed such that the candidate is no longer realizable, the interaction must fail/re-resolve explicitly rather than silently targeting a different organism.

## Spatial continuity seed

When exact individual identity is not required, a region/population may still retain a deterministic **spatial continuity seed or latent descriptor** so repeated project/collapse cycles do not visibly reshuffle the world every time an area unloads.

Such a seed:

- is not itself organism identity;
- must derive from authoritative region/population state rather than frame timing;
- may be invalidated or advanced only by explicit canonical spatial changes;
- must be versioned with the spatial projection scheme.

This can preserve perceptual continuity without storing every organism permanently.

## Movement at coarse fidelity

Coarse movement should generally be represented as explicit flux between spatial authority units rather than hidden point motion.

For example:

```text
cell A population/biomass
        |
        | migration flux
        v
cell B population/biomass
```

When sparse strata exist, migration may need to move exact stratum count/biomass rather than only total headcount.

If movement depends on social group, disease cluster, age, genotype, habitat, or another covariance, the corresponding structure must survive or be represented by a qualified closure.

## Spatial ecological processes

### Disease

Cell prevalence alone may be insufficient when transmission depends on clustered contacts.

A process may require S2 cluster burden or S3 contact-network statistics.

### Predation

Predator/prey counts in the same cell do not guarantee encounter probability.

Patch overlap, cover, edge distance, herd formation, and movement structure may matter.

### Plants

Plant competition depends strongly on neighborhood geometry.

The same tree count can represent an evenly spaced stand, a dense thicket, or a canopy-gap cluster with different light, water, root, and fire outcomes.

### Pollination

Flower and pollinator abundance may be insufficient if spatial separation prevents encounters.

### Fire/disturbance

Fuel continuity and adjacency can dominate spread even when total biomass is identical.

### Territorial/social animals

Uniform independent placement is particularly poor when exclusion zones, group cohesion, dens, nests, rookeries, or migration corridors govern position.

## Spatial structure and stratified extensive state

When extensive quantities depend on spatial structure, those quantities belong with the spatial strata/patches that explain them.

Examples:

- biomass per vegetation stand;
- infected biomass/count per disease cluster;
- juvenile biomass per nursery patch;
- detritus/fuel load per fire-relevant patch;
- prey biomass within a predator-accessible patch.

A global total remains derivable but cannot replace required local extensive state.

## Spatial coarsening

Collapsing richer spatial state is an information-loss transition.

Before S4/S3/S2 state collapses to a lower level, the target representation must be sufficient for all processes that will continue running.

If not, Living World must:

1. retain richer spatial state;
2. persist a required latent/patch descriptor;
3. use a separately qualified closure with bounded error; or
4. suspend/promote the process rather than invent unsupported precision.

## Adaptive spatial fidelity

`ADAPTIVE_INFORMATION_FIDELITY_V0.md` applies directly.

Spatial state should promote before a closure leaves its qualified regime.

Examples:

- disease prevalence nears an epidemic threshold -> preserve richer contact/cluster state;
- grazing approaches vegetation-collapse threshold -> preserve patch overlap;
- fire approaches a heterogeneous fuel boundary -> refine patch adjacency;
- predator/prey density enters a regime where encounter closure error grows -> promote spatial structure.

Exact requirements may never be waived by an error budget.

## Qualification fixtures

### Same occupancy, different clustering

Construct two fine states with identical cell counts and biomass:

- state A: one dense cluster;
- state B: dispersed points.

Run a contact-sensitive process.

If futures differ materially, S0 occupancy is not a valid closure for that process.

### Same biomass, different fuel continuity

Construct equal total vegetation biomass with:

- connected fuel patches;
- fragmented gaps.

Fire-spread outcomes should demonstrate whether adjacency must be retained.

### Stand geometry

Construct equal tree count/biomass in:

- uniform spacing;
- clumped gap regeneration.

Compare light competition/growth under fine and coarse models.

### Projection independence

Generate Level-P positions under two presentation schemes from the same canonical spatial state.

Canonical ecological outcomes must remain identical while no realization occurs.

### Realize what was shown

Project one candidate with a spatial descriptor, issue a canonical interaction against its scoped handle, realize it to Level A, and verify the canonical active position matches the projected candidate within the scheme's declared realization tolerance.

### Collapse/reload continuity

Project a population, unload/collapse, reload without canonical spatial change, and verify the same scoped/versioned projection reproduces stable candidates.

### Order/thread invariance

Run spatial flux/contact intent generation in different iteration/thread orders and verify deterministic canonical arbitration/settlement.

## Metrics

Useful Living World Observatory metrics include:

- spatial pair-correlation discrepancy;
- nearest-neighbor distribution discrepancy;
- cluster-size distribution;
- contact-rate error;
- patch-overlap error;
- fuel-connectivity error;
- canopy-gap/stand-structure error;
- projection continuity rate;
- realization displacement;
- fine-vs-coarse process outcome error.

Metrics must be selected according to process semantics rather than treated as universally sufficient.

## Non-goals

This contract does not choose:

- final world cell size;
- final clustering algorithm;
- final navigation system;
- final animal flock/herd model;
- final vegetation point process;
- final spatial persistence encoding;
- final multiplayer authority protocol.

It defines the information and authority boundary those systems must respect.

## Design principle

**A cell count says how much life is present, not how that life is arranged. Spatial structure becomes canonical only when ecology requires it; presentation may suggest geometry, but it may never silently invent the geometry that determines the world's future.**
