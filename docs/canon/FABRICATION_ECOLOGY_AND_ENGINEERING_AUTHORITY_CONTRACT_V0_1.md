---
title: Fabrication Ecology and Engineering Authority Contract
version: 0.1
status: canonical-draft
scope: fabrication authority, workpieces, engineering intent, interfaces, joints, processes, workmanship, commissioning, repair, dismantling, and integration with Universal Matter
owner: design/fabrication/construction/physics/simulation
related:
  - canon/CONSTRUCTION_REPAIR_AND_STRUCTURAL_TRANSFORMATION_CONTRACT_V0_1.md
  - tech/Symtropy Design Doc - Cybernetic Crafting & Physical Node Assembly.md
  - tech/STRUCTURAL_INTEGRITY_CONSTRUCTION_AND_DESTRUCTION_RUNTIME_V0_1.md
  - canon/PLAYER_AUTHORSHIP_SANDBOX_AND_MODDING_CONTRACT_V0_1.md
  - canon/PROGRESSION_ECONOMY_AND_MASTERY_CONTRACT_V0_1.md
---

# Fabrication Ecology and Engineering Authority Contract

## Owned Question

**How can Symtropy let players intentionally turn persistent matter into useful artifacts without recipes, spawned finished objects, duplicate physics truth, arbitrary quality scores, or construction-specific exceptions?**

## Canonical Decision

Fabrication is the intentional transformation of already-authoritative matter into persistent capability.

```text
intent
  + matter
  + geometry
  + process capability
  + interfaces
  + assembly
  + evidence
  -> persistent artifact state
```

The fabrication system does not create matter, replace structural physics, decide civic permission, or declare a device operational merely because assembly steps were completed.

> **Making is the intentional conversion of matter, knowledge, and labor into persistent capability.**

A structure is therefore not something a player places. It is a persistent network of matter, interfaces, joints, services, obligations, and history that the player caused to exist.

# 1. Authority Partition

Symtropy must preserve four distinct truths.

## 1.1 Matter Truth

Owned by Universal Matter or the current authoritative physical substrate.

It owns:

```text
material identity
quantity / conserved inventory
physical geometry
fragment identity
material condition
structural bond truth
physical damage
physical persistence
```

Fabrication may request typed matter transformations. It may never mint, delete, or silently rewrite authoritative matter.

## 1.2 Fabrication Truth

Owned by the fabrication domain.

It owns:

```text
workpiece identity
intentional process history
manufactured features
semantic interfaces
joint semantics
assembly topology
workmanship evidence
manufacturing provenance
known process defects
```

Fabrication describes **how intentional work changed matter**. It does not become a second physics engine.

## 1.3 Functional Truth

Owned by engineering/function evaluators.

It owns:

```text
requirements
constraints
performance envelopes
fitness for intended service
derating
failure-mode interpretation
commissioning predicates
```

A fabrication record cannot claim that an artifact works. Functional truth is derived from current physical and assembly state.

## 1.4 Civilizational Truth

Owned by Device Bus, civic/authority, ownership, and Chronicle systems as appropriate.

It owns:

```text
identity on shared systems
operator authority
ownership / stewardship
certification / registration
liability
public consequences
durable history
```

Physical possibility and civic permission are deliberately separate.

## 1.5 Authority Theorem

```text
physics decides whether it can exist
fabrication records how it was made
engineering decides what it can safely do
society decides how it may be used
history records what mattered
```

No layer may counterfeit another.

# 2. The Fabrication Lifecycle

The canonical lifecycle is:

```text
matter
  -> workpiece
  -> process
  -> manufactured feature
  -> interface / joint
  -> assembly
  -> functional evaluation
  -> commissioning
  -> operation
  -> wear / damage
  -> diagnosis
  -> repair / modification
  -> dismantling / salvage
  -> matter or reusable workpiece
```

There is no privileged `crafted` state between matter and artifact.

# 3. Workpieces

A **workpiece** is matter currently carrying intentional manufacturing identity.

A workpiece must reference authoritative matter; it must not duplicate the conserved inventory as an unrelated count.

A workpiece may carry:

```text
stable workpiece identity
matter allocation reference(s)
current manufactured geometry/features
semantic interfaces
process history
workmanship state
provenance
assembly membership
lifecycle state
```

Workpiece lifecycle states should remain small and structural:

```text
staged
in_process
available
installed
removed
retired
```

These states describe fabrication participation, not whether the underlying matter still exists.

## 3.1 Stable Identity

Workpiece identity survives:

```text
save / load
installation
removal
repair
representation LOD
runtime entity recreation
```

A Bevy entity, rigid body handle, mesh handle, or solver particle is never workpiece identity.

## 3.2 Matter Binding

Until the advanced Universal Matter lineage is integrated into the live standalone repository, the fabrication kernel must use an explicit opaque matter-allocation reference rather than inventing a replacement material store.

The future adapter must prove:

```text
referenced allocation exists
allocation revision is current
claimed matter is not double-bound incompatibly
transformation commits atomically with fabrication history
released matter returns to physical authority
```

# 4. Designs Are Constraints, Not Recipes

A conventional recipe says:

```text
4 steel + 2 copper -> pump
```

Symtropy should instead express:

```text
required function
  -> constraints
  -> interfaces
  -> admissible material/process envelopes
  -> candidate manufacturing plans
```

Three concepts remain distinct.

## 4.1 Design Specification

Declares what must be true for an intended function.

Examples:

```text
minimum pressure capacity
minimum flow area
allowed leakage
mechanical load envelope
thermal envelope
chemical compatibility
electrical isolation
connection geometry
service access
```

## 4.2 Manufacturing Plan

Declares one known sequence of processes that can attempt to satisfy a design.

A plan is advice and procedure, not ontology. Alternate plans may satisfy the same design.

## 4.3 Blueprint

A provenance-bearing artifact that may package:

```text
design specification
manufacturing plan
tolerances
commissioning tests
known failure modes
allowed substitutions
firmware
certification evidence
license / authority metadata
```

Possessing a blueprint can improve knowledge, trust, reproducibility, and authorization without becoming magical permission to create the artifact.

# 5. Interfaces

Parts become interoperable through explicit interfaces rather than matching item names.

Initial interface families:

```text
mechanical
structural
fluid
electrical
thermal
data/control
```

An interface describes an exposed contract such as:

```text
geometry / mating form
orientation
capacity envelope
medium or signal class
tolerance
required joining process
service accessibility
```

Compatibility must be evaluable from typed state.

A hardcoded list such as `pipe_A connects_to pipe_B` is not the long-term authority model.

# 6. Joints

Connections are first-class fabrication entities.

A joint represents the semantic meaning of a physical connection while underlying structural/matter authority remains responsible for physical truth.

Joint families may include:

```text
fastened
welded
brazed / soldered
bonded
clamped
press-fit
threaded
hinged / bearing
sealed fluid coupling
electrical termination
```

Joint state may include, where relevant:

```text
member identities
interface identities
joining process evidence
alignment
preload / clamp state
seal state
thermal history
contamination
corrosion
fatigue / damage reference
inspection evidence
```

A joint must never own a second independent strength value that can drift away from structural authority. It may cache or interpret physical evidence, but the authoritative physical bond remains below it.

# 7. Process Algebra

Fabrication is built from a bounded vocabulary of typed transformations rather than thousands of recipes.

Initial process families:

```text
prepare
separate
shape
join
treat
connect
configure
inspect / test
```

Initial useful operations include:

```text
clean
cut
drill
grind
bend
form
clamp
fasten
weld
seal
splice
terminate
coat
heat-treat
calibrate
pressure-test
continuity-test
inspect
```

Each process declares:

```text
admissible inputs
required capability envelope
preconditions
intended state transition
produced evidence
possible deterministic defect classes
abort behavior
```

A process operates on current state. It does not exchange ingredients for a prefab result.

# 8. Capability Envelopes

Tools, machines, fixtures, and operators provide capabilities rather than arbitrary crafting bonuses.

Examples:

```text
available force / torque
heat or energy range
control precision
measurement resolution
accessible geometry
supported material families
working envelope
duty cycle
calibration state
condition
```

Better tools primarily expand controllability, repeatability, precision, and observability.

They should not merely multiply `crafting_speed`.

# 9. Workmanship Is a Vector

There is no canonical universal `quality: 0.82` field.

An artifact carries only the dimensions relevant to its fabrication and service.

Possible dimensions include:

```text
dimensional accuracy
alignment
surface preparation
joint integrity
seal integrity
contamination
residual stress
fastener preload
electrical termination quality
calibration error
```

Functional evaluators determine which dimensions matter for a given design.

A crooked table and a misaligned turbine therefore have different consequences without needing different crafting ontologies.

# 10. Failure Must Be Causal

Fabrication may introduce defects, but defects arise from state and process conditions rather than opaque random failure rolls.

Example:

```text
poor preparation
  -> incomplete weld fusion
  -> reduced joint margin
  -> cyclic loading
  -> crack growth
  -> leak
```

Uncertainty may exist in observation and diagnosis. It must not be used as permission for inexplicable failure.

# 11. Observation and Knowledge Boundary

Authoritative physical state is not automatically player knowledge.

```text
reality state != observation state != actor belief state
```

Tools and expertise improve the evidence available to the actor.

Progression should therefore preferentially increase:

```text
measurement resolution
diagnostic discrimination
process control
substitution knowledge
failure recognition
planning quality
```

rather than granting magical fabrication tiers.

# 12. Assembly Graphs

Complex artifacts are persistent graphs of:

```text
parts / workpieces
interfaces
joints
service connections
subassemblies
```

The graph supports:

```text
construction order
dismantling order
accessibility
maintenance
load-path interpretation
failure propagation
replacement
upgrade
salvage
```

A building, bridge, machine, rover, and pressure vessel are different scales of the same assembly concept.

# 13. Construction Is Fabrication at Larger Scale

Construction does not receive a second crafting ontology.

The same semantics scale through:

```text
field repair
workshop fabrication
machine assembly
building construction
public works
industrial production
```

What changes is orchestration and representation fidelity.

Suggested simulation scales:

```text
micro       - seam, seal, bearing, termination
assembly    - pump, door, panel, structural bay
structure   - building, bridge, machine hall
facility    - waterworks, factory, settlement network
```

Representation LOD may aggregate dormant detail, but must preserve identity, material accounting, topology, condition, and meaningful history.

# 14. Construction Sites and Temporary State

Large construction is persistent work, not a single placement event.

A future construction-site authority should represent:

```text
survey / design reference
staged materials
completed operations
pending work orders
temporary supports
access constraints
hazards
utility state
commissioning state
```

Incomplete states are valid world states:

```text
excavated but flooded
positioned but unbraced
braced but not self-supporting
installed but untested
physically complete but uncommissioned
commissioned but not authorized for public service
```

Temporary works such as scaffolding, shoring, jigs, bypass plumbing, temporary power, and falsework are ordinary fabricated assemblies and may matter structurally.

# 15. Commissioning Is Evidence

An artifact is not operational merely because its assembly graph is complete.

A commissioning plan names required tests and the predicates those tests establish.

Examples:

```text
continuity
insulation
free rotation
low-power spin
pressure hold
leak rate
vibration envelope
load test
calibration
Device Bus initialization
```

Commissioning produces evidence. Functional and civic systems consume that evidence according to their own authority.

# 16. Repair and Dismantling Symmetry

Repair modifies the actual damaged causal state. It does not reset an abstract health bar.

Dismantling must be the controlled inverse of assembly where physically possible.

```text
assembly -> use -> damage -> diagnosis -> repair
assembly -> dismantling -> reusable parts / salvage -> new fabrication
```

Destructive removal is valid but may reduce recoverability, destroy evidence, contaminate material, or create hazards.

# 17. Provenance and Chronicle

Fabrication history may retain detailed local process evidence for explanation and replay.

Chronicle and civic history should record only consequential transitions, for example:

```text
public bridge commissioned
emergency conduit installed
uncertified pressure vessel accepted under emergency authority
machine failure traced to historic misalignment
public infrastructure dismantled
```

Every tool stroke does not become durable civic history.

# 18. Player-Facing Complexity Rule

> **Fabrication complexity must purchase agency.**

A simulated property earns its place only when it creates at least one of:

```text
a meaningful decision
an understandable failure
an alternate solution
transferable knowledge
emergent consequence
sensory satisfaction
```

If detail produces only repetitive labor, it should be abstracted.

Repeated known-safe work may compress after the player has established the procedure; compression must preserve resulting state and causal evidence.

# 19. Foundation Runtime Boundary

The first implementation should be a pure deterministic domain crate with no Bevy, Rapier, rendering, input, or UI dependencies.

```text
symtropy-game-state
        |
        v
symtropy-fabrication
        |
        +--> future Universal Matter adapter
        +--> future engineering evaluator
        +--> future construction orchestration
        +--> future diagnostics
        +--> presentation consumers
```

The initial fabrication kernel should own only:

```text
stable workpieces
opaque matter bindings
interfaces
joints
process specifications
process evidence
workmanship dimensions
assembly topology primitives
```

It must remain impossible for the kernel to silently create physical material.

# 20. F0-F4 Qualification Gates

## F0 — Authority Contract

Pass when:

```text
matter / fabrication / function / civic authority are non-overlapping
recipes are explicitly non-authoritative
runtime entity identity is explicitly non-persistent
Universal Matter integration is an adapter seam, not duplicated storage
```

## F1 — Workpiece Identity

Pass when:

```text
workpieces round-trip through serialization
stable identity survives reconstruction
matter bindings are explicit and revisioned
retirement does not imply matter deletion
```

## F2 — Interfaces

Pass when:

```text
typed interface families exist
compatibility is deterministic and symmetric where appropriate
capacity/tolerance mismatch is explainable
interfaces reference workpieces rather than runtime entities
```

## F3 — Joints

Pass when:

```text
joints connect compatible interfaces only
joint identity is persistent
joint semantics do not duplicate structural truth
removal preserves a causal record
```

## F4 — Process Algebra

Pass when:

```text
process preconditions are deterministic
capability requirements are explicit
process completion emits evidence rather than a prefab item
process abort is a valid state transition
at least one repair workflow composes several process types
```

# 21. First Cross-System Proof

The first generalized proof remains the Patch Conduit, but its implementation must no longer be a special recipe.

```text
inspect fracture
  -> bind salvage/new material as workpieces
  -> prepare surfaces
  -> align interfaces
  -> clamp / fasten
  -> seal or weld
  -> inspect joint
  -> pressure-test
  -> evaluate service envelope
  -> initialize Device Bus
  -> request temporary authority
```

The same primitives must later support a load-bearing footbridge and a multi-part pump/generator rebuild without changing the foundation ontology.

# 22. Anti-Goals

F0-F4 do not implement:

```text
full base building
factory logistics
NPC work scheduling
automated design synthesis
finite-element solvers
civic permit workflows
Device Bus runtime integration
Universal Matter persistence itself
multiplayer construction rights
```

Those systems consume the fabrication kernel after its authority boundaries are proven.

# 23. Final Invariants

1. **No recipe is physical truth.**
2. **No workpiece without a matter binding.**
3. **No fabrication operation may silently mint or delete matter.**
4. **No runtime ECS or solver handle is persistent fabrication identity.**
5. **No universal quality scalar is authoritative.**
6. **No joint may become a second structural-physics oracle.**
7. **No finished assembly implies functional success.**
8. **No functional success implies civic permission.**
9. **No failure without a recoverable causal explanation.**
10. **No simulated complexity that buys only repetition.**
11. **Construction, repair, modification, dismantling, and salvage share one fabrication vocabulary.**
12. **The world remembers consequential making without recording every transient gesture forever.**

> **A repair is not a progress bar. It is a chain of evidence attached to matter.**
