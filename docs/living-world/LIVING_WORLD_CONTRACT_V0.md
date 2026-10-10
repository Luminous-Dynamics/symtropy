# Symtropy Living World Contract v0

Status: implementation foundation

This document freezes the first architectural boundaries for Symtropy's Living World work. It is deliberately narrower than a complete ecology design: later flora, fauna, population, rendering, and causal-biography work must preserve these rules.

## Core authority rule

Canonical ecological state owns life. Active simulation and rendering are derived views of that state.

The dependency direction is one-way:

```text
canonical ecological truth
        |
        +--> persistence / replay / observatory
        |
        +--> active simulation working set
        |       +--> physics / navigation / IK / local sensing
        |
        +--> presentation recipe
                +--> mesh / material / animation / audio / particles
```

Derived state MUST NOT silently become ecological authority.

## Required invariants

### LW-I01 — Render independence

Changing renderer, frame rate, material quality, animation quality, or visibility MUST NOT alter canonical ecological state.

### LW-I02 — Fidelity independence

Promoting or demoting an organism or population between ecological fidelity tiers MUST NOT change canonical ecological truth except through an explicit, deterministic conservative refinement/reduction operation.

### LW-I03 — Headless equivalence

Given identical canonical input, simulation seed, and authoritative events, a headless run and a rendered run MUST produce identical canonical ecological state.

### LW-I04 — Single authoritative time

All biological rates derive from the authoritative simulation clock. Rendering time and wall-clock time are never authoritative biological time.

### LW-I05 — Deterministic derivation

Phenotype, materialization, inheritance, and stochastic-looking ecological variation MUST be reproducible from stable canonical inputs. Sequential or thread-local randomness MUST NOT define canonical biology.

### LW-I06 — Conservation by contract

Any subsystem that transfers a conserved ecological quantity MUST declare its source, sink, and tolerated numerical error. Promotion/demotion between representations MUST preserve the quantities declared by that representation boundary.

### LW-I07 — History matters

Where a biological process has hysteresis or durable consequences, current environmental conditions alone MUST NOT erase its history. Persistent effects are represented as state or causal imprints, not reconstructed from presentation artifacts.

### LW-I08 — Geometry is derived

Meshes, vertex buffers, animation rigs, particles, impostors, and texture/material instances are never canonical organism identity. Canonical structure must be sufficient to regenerate presentation after renderer or asset upgrades.

### LW-I09 — Bounded identity

Not every distant organism requires persistent individual identity. Population members may be represented statistically, but organisms that are narratively, causally, genetically, or ecologically notable may remain individually persistent.

### LW-I10 — Explainable appearance

When a visible feature is caused by canonical ecological state, tooling should be able to trace it to the responsible traits, physiological state, environment, or durable biographical imprint.

## Authority layers

### Canonical ecology

Owns:

- species identity and heritable/developmental traits;
- individual organism state when individually materialized;
- population and regional aggregate state;
- physiological state;
- ecological accounting quantities;
- habitat-facing biological state;
- disturbance memory and succession state;
- reproduction/inheritance inputs;
- durable organism and landscape biography.

### Active simulation

May derive and cache:

- local sensory queries;
- navigation state;
- contact and IK targets;
- transient structural response;
- spatial acceleration structures;
- nearby individual working sets;
- short-lived physics proxies.

These caches are disposable and reconstructible.

### Presentation

May derive and cache:

- meshes and impostors;
- GPU buffers;
- materials and shader parameters;
- animation state;
- procedural secondary motion;
- visual wind response;
- particles;
- audio presentation.

Presentation has no write authority over ecology.

## Fidelity model

The initial fidelity vocabulary is:

- **F0 Macro** — population/biomass state, no individual presentation required.
- **F1 Proxy** — simplified cohorts or instances.
- **F2 Active** — individual physiology/behavior with simplified physical representation.
- **F3 Interactive** — detailed local contact, damage, and behavioral state.
- **F4 Hero** — highest justified structural and presentation fidelity.

Distance is only one scheduling input. Interaction, narrative importance, visibility, ecological significance, and computational budget may also affect fidelity.

## Qualification properties

Living World work should progressively establish executable checks for:

1. deterministic replay;
2. frame-rate independence;
3. headless/rendered ecological equivalence;
4. save/reload equivalence;
5. population refinement/reduction conservation;
6. ecological ledger conservation;
7. deterministic inheritance;
8. bounded phenotype variation;
9. disturbance-history effects;
10. fixed-scene visual and locomotion evidence.

## Explicit non-goals of v0

This contract does not yet define:

- a full genetics schema;
- canonical-event-v2 bindings;
- a complete trophic model;
- plant structural graphs;
- animal locomotion implementation;
- final flora/fauna materials;
- a specific world-region size;
- a claim of photorealism or ecological scientific validity.

Those should be added only after the underlying authority and conservation boundaries are executable.
