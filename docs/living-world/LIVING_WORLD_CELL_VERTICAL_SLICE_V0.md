# Living World Cell Vertical Slice V0

## Purpose

Define the first bounded, end-to-end Living World validation scene that integrates canonical ecology, organism development, perception, decomposition, persistence, multiscale fidelity, and one-way causal presentation.

The goal is not a large forest.

The goal is **one small piece of life that is causally coherent enough to scale**.

## Scenario footprint

Use a compact deterministic validation area on the order of 30 m × 30 m (exact dimensions versioned in the scenario manifest).

The scene should be small enough for exhaustive headless evidence yet rich enough to exercise the whole living-world loop.

## Minimum biological cast

V0 should eventually contain:

### Flora

- one mature structural plant/tree archetype;
- one or more juvenile developmental stages;
- ground vegetation or low producer cover;
- one dead structural plant/log or equivalent detrital substrate;
- root/canopy influence on habitat.

### Decomposers / networks

- existing Mycelium integration or equivalent fungal/decomposer actor;
- detrital substrate -> decomposer intent -> nutrient/field feedback;
- explicit distinction between exact stock settlement and heuristic ecology fields.

### Fauna

- one mobile animal archetype using canonical signal perception rather than hidden world queries;
- simple homeostatic needs such as hunger/thirst/fatigue/injury within the qualified model;
- canonical movement/contact state with presentation downstream;
- at least one signal emitted by movement/alarm/interaction that another system could consume.

### Collective/small-life system

- existing Colony or another group-primary actor where useful;
- demonstrates that not every organismal system is individual-primary.

## Environmental gradients

The cell should include controlled, versioned spatial variation in at least:

- light/shade;
- moisture;
- nutrient availability/index;
- obstruction/substrate;
- one mechanical exposure driver such as directional wind;
- one disturbance/damage mechanism.

These are scenario authority, not presentation presets.

## Required causal loops

The vertical slice is incomplete until it demonstrates multiple closed loops.

### Development loop

`habitat history -> developmental accumulator -> reaction norm -> growth/allocation proposal -> paid structural commit -> changed plant structure`

### Competition loop

`plant structure -> canopy/root influence -> habitat/resource pressure -> future allocation/growth`

### Perception/action loop

`canonical event -> signal -> propagation -> receptor -> percept -> bounded action intent -> movement/ecological effect`

### Detritus loop

`death/breakage/organ turnover -> exact detrital authority -> decomposer/substrate signal -> decomposer intent -> reaction/settlement -> nutrient availability -> producers`

The first version may stage some exact chemistry behind explicit placeholders, but no heuristic field may be mislabeled as exact conserved stock.

### Damage/history loop

`disturbance -> canonical damage/physiology -> altered capability/growth -> persistent structure/behavior -> visible causal cue`

## First four-tree fixture

Within or adjacent to the cell, run one hereditary/species baseline under four controlled histories:

1. control;
2. persistent drought;
3. persistent directional crosswind;
4. canopy competition / directional shade.

Compare canonical developmental, allocation, and structural state before rendering.

Presentation then attempts to make those histories legible without scenario-specific cosmetic shortcuts.

## Fauna false-omniscience fixture

Hold a threat/source position fixed while changing only the sensing chain:

- no compatible emission;
- clear signal path;
- occluded/attenuated path;
- impaired receptor/physiology;
- canonical masking/noise.

A hidden source with no qualified signal path must not generate a source-attributable percept.

## Persistence fixture

Run at least:

`0 -> T1 -> save -> reload -> T2`

and compare with uninterrupted:

`0 -> T2`.

Exact authority state must match exactly; approximate biological closures compare using their declared tolerance/error regime.

Persistent history to exercise should include some subset of:

- plant damage/topology;
- developmental exposure;
- phenological state;
- propagule/dormancy state;
- animal injury/memory;
- ecological stock ownership;
- detritus/decomposer progress.

## Fidelity attack fixture

Repeatedly move actors through representation boundaries:

- coarse <-> active ecology;
- low <-> high presentation LOD;
- region activation/deactivation where supported.

Run many cycles, not one.

The process must not accumulate:

- count drift;
- biomass/stock drift;
- duplicated offspring;
- healed damage;
- rerolled heredity;
- reset developmental history;
- reset perception/memory state;
- changed phenology;
- changed canonical plant topology.

## Presentation targets

Presentation should eventually expose biological causes through:

### Flora

- crown/branch architecture;
- age/taper;
- wounds/scars/dead tissue;
- hydration/stress cues;
- phenology;
- leaf/organ age;
- structural wind response.

### Fauna

- gait/contact quality;
- breathing/fatigue;
- injury/limp;
- condition/coat state;
- orientation/attention;
- ears/tail/facial/secondary motion where appropriate;
- group/social cues where present.

None of these presentation systems may mutate canonical biology directly.

## Evidence manifest

Every qualification run records at minimum:

- code/head identity;
- content/scenario version;
- species/development schema versions;
- seed/heredity fixtures;
- authoritative tick range;
- toolchain;
- feature/config identity;
- canonical initial/final state digests when available;
- exact stock/count metrics;
- toleranced observable metrics;
- capture configuration for presentation evidence;
- evidence tier (Q0–Q5).

Do not mix evidence from changed code/content/schema roots under one qualification claim.

## Exit gates

### Gate A — headless causal ecology

Required before visual-realism claims:

- deterministic reference execution;
- no unauthorized authority mutation;
- required causal loops execute;
- save/reload invariance;
- selected fidelity invariance;
- exact stock/count conservation where modeled.

### Gate B — causal presentation

- rendering does not alter Gate-A canonical outputs;
- important canonical state changes produce intended visible cues;
- LOD preserves major causal cues;
- fixed camera/path captures are reproducible enough for comparison.

### Gate C — causal legibility

- controlled-history organisms are distinguishable from canonical evidence first;
- presentation makes the relevant distinctions human/metric-legible;
- random cosmetic variation alone cannot explain the result.

### Gate D — owner playtest

Human validation asks whether the scene feels alive, readable, and coherent during interaction—not merely whether static screenshots look attractive.

## Non-goals

V0 is not:

- a production open world;
- a universal biology simulation;
- a claim of scientific species fidelity;
- a full weather/climate system;
- a finished art asset library;
- a benchmark for maximum population count;
- proof of photorealism.

It is the first integrated proof that Living World authority, biology and presentation can form one coherent loop.

## Relationship

- #170: multiscale population/refinement authority;
- #230: species/development/reproduction/life-cycle biology;
- #235: structural flora and causal rendering;
- #244: causal fauna/animal embodiment;
- #245: Observatory evidence framework;
- #242: first headless PlantStructuralGraph product gate;
- #249: first fauna SignalPerception product gate;
- #214: death/decomposition vertical-slice prerequisites.
