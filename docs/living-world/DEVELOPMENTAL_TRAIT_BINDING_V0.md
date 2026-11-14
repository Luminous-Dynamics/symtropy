# Developmental Trait Binding V0

## Status

Normative design contract for composing hereditary baselines, developmental stimuli, reaction norms, physiology and structural state. This document does not claim a complete organism-development implementation exists yet.

## Goal

A developmental trait should answer more than:

> what value does this slider have right now?

It should define:

- what is inherited;
- what environmental/history driver can modify it;
- when that modification is permitted;
- whether the result is reversible, persistent, hysteretic or structurally irreversible;
- what state must survive fidelity collapse and reload;
- how the final value can be explained causally.

## Core model

For one trait:

```text
hereditary baseline
+ developmental stage
+ qualified stimulus history
+ versioned reaction norm
+ retained developmental state
-> canonical developmental trait state
```

Presentation derives from the resulting state; presentation does not write it.

## Trait identity

Every developmental trait requires a stable portable key under a species/development schema.

Examples:

```text
plant.branch.internode_length
plant.branch.wood_allocation
plant.leaf.area_scale
plant.root.allocation
animal.body.frame_scale
animal.coat.density
animal.limb.proportion.fore
```

The key is semantic identity, not merely a UI label.

Changing the meaning/unit of an existing key requires schema/version migration rather than silent reinterpretation.

## Hereditary baseline

The baseline may be derived from explicit genotype content and/or a deterministic keyed phenotype variation rule.

The baseline is not the same as current developed state.

For example:

```text
inherited branch-angle tendency = 42 degrees
current developed branch geometry = function of tendency + light + wind + damage + age
```

## Response binding

A reaction norm is bound to exactly one named developmental driver specification or to an explicitly versioned composition of several drivers.

The binding identifies:

- reaction-norm scheme/version;
- stimulus provenance/calibration;
- response units/scale;
- developmental stage(s) where active;
- update cadence;
- state-retention class.

## State-retention classes

V0 should distinguish at least four categories.

### R0 — transient reversible

Current environment can change the value and it can return without persistent developmental memory.

Examples may include short-term leaf orientation or temporary physiological presentation proxies.

R0 state can often be recomputed from current canonical inputs.

### R1 — slowly reversible / adaptive

State relaxes over time and requires a retained accumulator or state variable.

Examples may include acclimation or seasonal coat-density adaptation.

Collapse requires preservation of the sufficient adaptive state.

### R2 — cumulative developmental

History accumulates and changes future structure/development.

Examples:

- shade-induced internode history;
- accumulated mechanical conditioning;
- developmental nutrient limitation;
- growth-stage thermal time.

The relevant accumulator/history is future-bearing canonical state.

### R3 — structural / effectively irreversible

The response changes a structure that cannot be recreated from current environment alone.

Examples:

- branch topology already grown;
- trunk taper/wood already allocated;
- limb loss;
- permanent scar/callus;
- root branch path already established.

R3 effects require explicit repair/remodeling events to reverse. Current environment must not simply overwrite them.

## Critical anti-pattern

Do not implement mature morphology as:

```text
current_mesh_scale = reaction_norm(current_environment)
```

for traits whose biology is cumulative or structural.

That would make a drought-scarred tree instantly become an undroughted tree when rain returns and would make offscreen history disappear.

Instead:

```text
current environment
    -> developmental driver update
    -> retained developmental state
    -> structural/growth action
    -> persistent morphology
```

## Stage gating

A developmental response may be active only during particular species-specific stages.

Examples:

- limb proportion may be highly plastic during juvenile growth and mostly fixed in adulthood;
- branch topology expands during growth but mature trees may only add secondary growth/remodeling;
- flowering response depends on mature/reproductive state;
- coat density may remain seasonally adaptive in adults.

Species-specific stage definitions map to the coarse shared `LifeStage` telemetry but should not be replaced by it.

## Rate / cadence

Developmental updates derive from authoritative simulation time.

No wall-clock, renderer frame delta, or animation tick may directly advance canonical development.

Long offscreen catch-up may use aggregated sufficient statistics only where the development process qualifies that reduction.

## Structural actions

For R2/R3 traits, reaction-norm output often should influence a **growth action** rather than directly set final geometry.

Examples:

```text
shade response -> next internode length tendency
wind response -> next wood-allocation / stiffness action
root-zone moisture -> next root allocation/growth direction weighting
thermal development -> stage-progress increment
```

This is preferable to rewriting already-grown structure.

## Constraints / tradeoffs

Multiple traits may compete for finite biological resources.

A future growth system should not independently apply every positive response if doing so creates mass/energy from nowhere.

Preferred architecture:

```text
trait/development responses
    -> proposed growth/allocation intents
    -> resource/physiology constraints
    -> canonical arbitration/settlement
    -> structural update
```

This connects developmental realism to the Living World stock/scarcity contracts.

## Causal provenance

A future developmental state should preserve enough information to explain material trait differences.

Example:

```text
Tree branch segment 91
  internode length = 143 mm
  baseline tendency = 112 mm
  + shade response = +28 mm
  + water-stress response = -7 mm
  + structural constraint = +10 mm effective allocation
  committed at growth tick 4200
```

The exact telemetry shape is open, but causal legibility is a design goal.

## Collapse / persistence

For each trait binding, define the minimum state that must survive:

- active -> coarse collapse;
- region unload;
- save/reload;
- world migration;
- renderer destruction/recreation.

R0 may be recomputable.

R1/R2 generally require sufficient retained accumulators.

R3 requires the resulting structural state or an equivalent lossless structural summary.

If the destination coarse representation cannot retain required information, the existing C1/C2/C3 collapse rules apply.

## Qualification fixtures

Future executable evidence should include:

1. same genotype/baseline + same developmental history -> same trait state;
2. same baseline + different qualified history -> expected directional divergence;
3. removing current stress does not erase R2/R3 history;
4. R0 state can return under reversed current conditions when biology says so;
5. stage-inactive drivers cannot mutate the gated trait;
6. renderer/FPS changes cannot alter development;
7. save/reload preserves R1/R2/R3 future trajectory;
8. coarse/fine execution agrees where the coarse sufficient statistic is qualified;
9. finite-resource growth constraints prevent independent trait responses from minting growth;
10. causal telemetry can identify the driver/binding that produced a committed structural change.

## First flora benchmark

Use one hereditary tree baseline under four deterministic histories:

```text
A: control
B: drought
C: persistent crosswind
D: canopy competition / shade
```

The target is not merely four visually different random trees.

The target is:

> the final structural differences are stable, replayable, directionally characteristic, and explainable from the known histories.

That becomes the first serious test of Symtropy's causal photorealism thesis.

## Non-goals

V0 does not define:

- exact botanical coefficients;
- full structural plant graph implementation;
- mesh generation;
- multivariate neural development models;
- complete resource-allocation physiology;
- biomechanics FEM.

It freezes the rule that **development is a history-bearing biological process with explicit reversibility and persistence semantics, not a real-time shader-like remapping from today's environment to today's appearance**.
