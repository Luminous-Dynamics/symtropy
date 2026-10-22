# Plant Meristem and Developmental Growth V0

Status: normative Living World design contract. Documentation only.

## Purpose

Define persistent plant growth sites and local developmental decisions so canonical architecture emerges through bud/meristem history rather than direct whole-tree procedural reshaping.

## Core invariant

> New persistent plant structure originates from authorized biological growth sites or an explicitly modeled regeneration process.

A renderer, mesh generator, or global shape function may not append canonical branches directly.

## Growth-site state

A canonical growth site may retain future-bearing state such as:

- stable structural attachment coordinate;
- growth-site kind (apical, axillary, root tip, cambial region, adventitious/regenerative, etc.);
- active / dormant / suppressed / damaged / dead state;
- developmental age or competence;
- local resource/access state when required;
- dominance/suppression state;
- qualified tropism/history accumulators;
- species/development program version.

Exact fields are implementation-specific and versioned.

## Local developmental drivers

Candidate drivers include:

- phototropism / directional light opportunity;
- gravitropism;
- hydrotropism / moisture gradients;
- nutrient opportunity;
- thigmo/mechanical response;
- apical dominance;
- self/neighbor competition;
- wound response;
- phenological gates;
- developmental reaction norms.

Each future-bearing driver follows the Living World stimulus-provenance rules. Renderer visibility or camera direction is never a developmental driver.

## Directional state

Some drivers are directional, not scalar.

Persistent crosswind, light direction, gravity, and moisture gradients cannot be reduced to one unsigned exposure value if direction changes future architecture.

A versioned directional-history representation is required whenever directional memory is biologically relevant.

## Growth proposal

A growth site produces a proposal, not structure.

Conceptually:

`growth-site state + species program + qualified drivers -> growth proposal`

A proposal may describe:

- extension;
- branching;
- bud activation/suppression;
- organ initiation;
- orientation change for new growth;
- cambial thickening request;
- dormancy/deactivation.

Persistent structural changes occur only after the plant growth/resource-settlement transaction authorizes them.

## Existing structure is not continuously re-solved

Tropisms guide **new growth and qualified remodeling**.

A mature branch does not rotate every tick until it points at the brightest current light unless an explicit mechanical/remodeling process exists.

This preserves developmental history and prevents camera/field changes from morphing old structure unrealistically.

## Apical dominance / release

Apical dominance is modeled as biological regulation, not a hardcoded tree-shape shortcut.

Loss/damage of an apical growth site may change suppression state of compatible lateral sites. The resulting crown restructuring is a new developmental sequence and therefore a visible biography signal.

## Deterministic candidate selection

When several growth sites compete for limited resources, selection must be deterministic and policy-driven.

Do not let:

- vector iteration order;
- ECS order;
- hash order;
- renderer distance;
- thread scheduling

choose which bud grows.

Competition may use explicit priority/weights and the canonical scarcity/allocation machinery.

## Collapse

Coarse plant state may aggregate growth sites only when the aggregate retains every statistic required by enabled developmental processes.

If which particular bud is suppressed/damaged/competent affects future crown form and cannot be represented coarsely, the plant requires richer structural state under the existing C1/C2/C3 collapse rules.

## Qualification direction

At minimum test:

1. same hereditary/program state + same driver history -> identical growth-site decisions;
2. adding an unrelated render query cannot change selected growth sites;
3. removal of an apical site can deterministically release eligible lateral growth under an authored fixture;
4. directional light/wind histories produce directionally different **new** growth without rotating old segments;
5. growth-site proposals do not mutate structure before resource settlement;
6. save/reload preserves dormant/suppressed site state and future activation sequence;
7. changing candidate container order cannot change biological outcome;
8. invalid/damaged/dead growth sites cannot silently generate new canonical organs.

## Non-goals

This contract does not define universal botanical meristem physiology, real-world branching parameters, or geometry generation.