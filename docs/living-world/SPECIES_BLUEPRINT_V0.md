# Species Blueprint V0

## Status

Normative design contract for future flora/fauna species content. This document does not claim a Rust `SpeciesBlueprint` implementation already exists.

## Goal

A species blueprint should describe **biological rules**, not a render prefab.

The canonical simulation must be able to change renderer, meshes, materials, LOD strategy, or animation system without changing what species an organism is or how its biology behaves.

## Core split

Keep three layers distinct:

```text
Species biology
    -> canonical traits / development / ecological capabilities

Presentation profile
    -> meshes / materials / rigs / animation / audio

World instance state
    -> one population / organism / lineage / developmental history
```

Presentation assets may reference a species biology definition, but presentation data does not become ecological authority.

## Identity

A blueprint requires a stable namespaced `SpeciesId` independent from display/localized name.

Conceptually:

```text
SpeciesId(namespace, canonical_name)
```

Examples:

```text
symtropy.firstlight:canopy_tree_a
symtropy.firstlight:grazer_a
```

The exact Rust representation is open.

Renaming UI text must not change species identity.

## Version / content identity

Future-bearing world state must bind to the exact biological content that produced it.

A species definition therefore needs at least:

- schema version;
- biological content version or digest;
- species ID;
- explicit migration policy when semantics change.

A save created under one biological blueprint must not silently load under a materially different blueprint merely because the display species name is unchanged.

## V0 biological sections

A future species definition should be able to bind the following independently.

### 1. Development plan

Species-specific stages and stage transitions.

The existing cross-domain `LifeStage` remains a coarse telemetry/interop category. Species-specific stages map *to* it rather than forcing every species into one identical lifecycle.

Examples:

```text
plant: seed -> germination -> seedling -> juvenile -> mature -> senescent
insect: egg -> larva -> pupa -> adult
mammal: neonate -> juvenile -> subadult -> adult -> elder
```

### 2. Metabolic traits

Bind stable capability/tolerance values compatible with `MetabolicTraits` or a qualified successor.

Mutable physiology remains organism state rather than blueprint state.

### 3. Hereditary trait definitions

Specify which traits are inherited/generative and their authored biological ranges/constraints.

Do not encode the complete genome as one opaque random seed without versioned heredity semantics.

### 4. Developmental trait bindings

Bind hereditary baselines to qualified developmental stimulus specifications and reaction norms.

A trait definition should identify whether a response is:

- reversible/physiological;
- slowly reversible;
- cumulative developmental;
- hysteretic;
- structurally irreversible without explicit repair/remodeling.

### 5. Ecological capabilities / affordances

Prefer compositional capabilities over hardcoded species-pair rules.

Examples:

- can consume fruit;
- can browse foliage;
- can pollinate compatible flowers;
- produces decomposable litter;
- can form a mycorrhizal association;
- can excavate/burrow;
- provides nesting substrate;
- produces toxic defense.

A species may participate in several ecological roles simultaneously and roles may change with stage.

### 6. Reproduction policy

Bind reproductive mode, eligibility, parental contribution, propagule type, and recombination/mutation scheme version.

Reproduction is a canonical authority transition; it must not be a render/spawn side effect.

### 7. Mortality / detrital class

Specify the biological class needed by death/decomposition without pretending to encode full chemistry.

Examples may distinguish woody tissue, soft animal tissue, leaf litter, seed/fruit material, etc.

Exact stock/property accounting remains governed by the Living World conservation contracts.

## Presentation separation

Do **not** put authoritative Bevy handles, mesh IDs, shader values, animation graph state, or LOD state into the biological blueprint.

A separate presentation profile may map:

```text
species + phenotype/development state
    -> body plan / structural geometry recipe
    -> rig / gait / animation profile
    -> material channels
    -> audio profile
    -> presentation LOD resources
```

Presentation content may be replaced without invalidating canonical ecology unless the replacement changes gameplay-affecting physical/contact semantics through an explicitly authoritative adapter.

## Structural biology rather than triangles

For long-lived organisms, canonical state should preserve biological structure instead of rendered triangles.

Examples:

```text
plant structural graph
animal body plan / skeleton topology
shell/exoskeleton segments
root system abstraction
```

Render geometry is derived from those structures.

## Species validation

A species definition should fail closed when required biological information is invalid or ambiguous.

Minimum checks should eventually include:

- valid stable species ID;
- supported schema version;
- deterministic content identity;
- unique trait keys within their namespace;
- valid authored ranges;
- valid development-stage graph;
- no impossible/ambiguous stage transitions;
- every reaction norm bound to a qualified stimulus specification;
- every exact/physical unit explicitly declared where authority depends on it;
- reproduction and mortality policy versions understood;
- no presentation-only resource required to execute headless canonical ecology.

## Headless requirement

A species must remain simulatable in a renderer-free/headless run.

Therefore:

> deleting every mesh, shader and animation asset must not erase the canonical organism's biology.

This is a core qualification criterion.

## Change classes

Not every content change has the same authority impact.

### Presentation-only

Examples: texture improvement, fur shader, non-authoritative mesh detail.

May keep biological version unchanged.

### Biological parameter change

Examples: drought tolerance, reaction norm, metabolic rate, reproduction eligibility.

Requires biological content version change and may require save/world migration policy.

### Schema/semantic change

Examples: new interpretation of trait units, changed development-stage grammar, changed heredity algorithm.

Requires explicit schema/version migration; never reinterpret old state silently.

## Qualification fixtures

Future executable evidence should include:

1. same blueprint + same canonical inputs -> same biological outputs;
2. changing presentation profile leaves canonical state unchanged;
3. headless simulation does not require render assets;
4. biological content change changes its content identity;
5. unsupported old biological version fails/migrates explicitly;
6. duplicate trait keys fail;
7. species-specific stage transitions map deterministically to coarse `LifeStage` telemetry;
8. developmental responses reference qualified stimulus semantics;
9. render LOD changes cannot mutate species traits;
10. save/reload binds to the same biological blueprint identity.

## Non-goals

V0 does not define:

- a universal taxonomy database;
- exact real-world species parameters;
- complete genome encoding;
- neural behavior models;
- Bevy asset format;
- a chemistry engine;
- persistent organism identity.

It freezes the architectural rule that **a Symtropy species is a versioned biological program whose presentation is replaceable, whose development is causal, and whose canonical meaning survives rendering changes**.
