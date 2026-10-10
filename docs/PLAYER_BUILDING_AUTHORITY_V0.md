# Player Building Authority v0.1

Status: **normative design contract; no runtime implementation claim**

This document freezes the first authority and gameplay contract for player-authored construction in Symtropy. It is intentionally upstream of the active Fabrication/Construction qualification stack and does not create a second physical-truth system.

The central design goal is:

> Give players as much expressive freedom as the simulated world can coherently support, while keeping physical realization, conserved matter, structural consequences, permissions, and historical provenance owned by their existing authorities.

A player should eventually be able to begin with an improvised shelter and, through the same family of construction semantics, participate in building homes, workshops, bases, settlements, cities, vehicles, spacecraft, shipyards, and orbital habitats. Scale may change representation and tooling; it must not silently change the meaning of physical reality.

## 1. Core authority theorem

Player input is intent, not physical truth.

```text
Player gesture / tool / blueprint / planner request
    -> ConstructionIntent
    -> deterministic plan compilation
    -> advisory constraint analysis
    -> exact construction/fabrication work requests
    -> authoritative physical execution
    -> structure / matter / structural / utility state
    -> read-only gameplay and presentation projections
```

The following are deliberately distinct:

```text
preview != plan
plan != reservation
reservation != execution
execution != successful structure
structure != commissioned system
inhabitable space != home
home != ownership
ownership != civic permission
```

No UI, blueprint, semantic-space classifier, settlement projection, NPC desire, Symthaea proposal, or renderer may manufacture a fact that belongs to Conserved Matter, Fabrication, Construction, structural physics, utilities, commissioning, ownership, or civic authority.

The existing `symtropy-construction` authority boundary remains intact: it owns site/work/staging/continuity/temporary-work orchestration truth and explicitly does not own conserved matter, structural physics, fabrication process truth, technical commissioning, Device Bus registration, or civic authorization.

## 2. One construction language, many scales

Symtropy should not develop unrelated house-builder, base-builder, city-builder, vehicle-builder, and station-builder authorities.

Instead, player-facing tools compile into one scale-independent construction language whose physical operations are consumed by the appropriate existing authorities.

A useful conceptual decomposition is:

```text
material + geometry + connection + treatment + function + provenance
```

A wall, hull plate, bridge member, pressure bulkhead, road slab, pipe support, or furniture panel may differ in material model, geometry, connection semantics, analysis requirements, and commissioning needs. They do not require unrelated notions of "placing an object into reality."

Specialized editors may exist. A ship designer can expose pressure, mass, thrust, thermal, and systems information that a hand-building interface does not. These are specialized projections and planning tools over shared lower-level authority, not parallel worlds.

## 3. Proposed player-building IR

The first runtime successor should introduce a dependency-light proposal layer. Names below are normative in role but may be adjusted if a later code audit finds a materially better fit.

### `ConstructionIntent`

Represents what an actor is trying to accomplish. It is non-authoritative and safe to create, edit, preview, discard, serialize as draft data, or generate from different interfaces.

It should bind at minimum:

- actor or proposing principal reference;
- target site/scope reference when one exists;
- requested geometry or operation family;
- material requirements as requirements, never invented inventory;
- optional anchors/connections;
- optional tolerances and alignment preferences;
- optional design/blueprint provenance;
- deterministic intent identity/version when persisted or networked.

### `ConstructionPlan`

A deterministic compilation of an intent against an explicitly identified planning context.

A plan should contain an ordered/dependency-resolved graph of proposed physical work. It must bind every authority input that affected compilation strongly enough that stale plans can be rejected rather than silently replayed against a changed site, changed design, changed material authority, changed capability set, or changed construction policy.

A plan is still not execution authority.

### `ConstructionOperation`

A small vocabulary of composable work intentions such as placement/assembly, joining, fastening, deposition/pour, cutting/machining, excavation/backfill, compaction, treatment, repair, removal/dismantling, and connection to owned utility/system authorities.

The vocabulary should map onto existing Construction/Fabrication/Universal Matter operations rather than duplicate them.

### `ConstraintReport`

Separates hard invariant failures from advisories.

Hard failures are reserved for conditions that genuinely cannot enter authoritative execution, for example malformed/non-finite geometry, missing authority identity, an impossible identifier relation, invalid operation topology, or an authority-owned denial.

Gameplay or engineering concerns that are physically possible but risky should normally be advisories: inadequate support, excessive estimated load, poor weather exposure, insufficient insulation, low ventilation, undesirable noise, high predicted cost, or lack of redundancy.

The player may be allowed to proceed when the world can coherently simulate the consequences.

### `BuildProjection`

Read-only preview derived from intent/plan/current world state. It may show ghost geometry, snap hints, dimensions, predicted material demand, estimated support/load concerns, utility routing hints, work sequence, or accessibility concerns.

A projection must never mutate canonical state.

## 4. Freedom contract

Maximum player freedom is achieved by removing arbitrary authoring restrictions, not by removing physical consequences.

### 4.1 Continuous placement is fundamental

The canonical authoring model must not require a world grid or mandatory sockets.

Players may construct off-grid, non-orthogonal, irregular, asymmetrical, curved, improvised, partially embedded, and adaptive structures when underlying geometry/physics authorities can represent them.

Grid placement may exist as a tool preference.

### 4.2 Snapping is assistive, never ontological

The interaction model should support a continuum:

```text
free placement
<-> inferred alignment
<-> explicit snap constraints
<-> parametric/pattern placement
```

Snap candidates are deterministic suggestions produced from explicit current context. Turning snapping off must not invalidate the construction language.

A snap relation should compile to the same meaningful geometry/connection result that could have been authored manually.

### 4.3 Player-authored dimensions

Where representation permits, players should be able to specify dimensions rather than select only prefabricated size variants.

Reusable parametric families should describe constraints and generation rules, not become magic physical objects.

### 4.4 Partial and improvised construction is valid state

The engine must support unfinished structures as first-class persistent states.

Examples include exposed framing, temporary shoring, incomplete roofs, trenches, scaffolding, staged materials, temporary weather protection, partially connected utilities, unfinished surfaces, and occupied buildings that continue to evolve.

A structure does not need to transition atomically from `absent` to `finished` merely for UI convenience.

### 4.5 Tools amplify rather than replace freedom

Player capability should progress approximately as:

```text
hand
-> tool
-> jig/template
-> reusable blueprint
-> machine
-> crew
-> automated fabrication
-> infrastructure planner
```

Every higher level should reduce repetitive work while preserving the ability to intervene at lower levels.

Large settlements must not require manually placing millions of parts. Delegation and hierarchical planning solve scale; they do not justify replacing physical construction with resource deletion plus instant geometry.

## 5. Construction identity and composition

A physically realized structure should be composed from stable existing physical/construction identities rather than receiving a magical new truth source.

A later player-building runtime may introduce an assembly/place reference that groups constituent construction identities, but grouping cannot erase or replace their provenance.

The following should remain queryable where the owning authority retains them:

- constituent physical/construction identities;
- source material provenance;
- fabrication provenance;
- joining/repair history;
- salvage generation;
- structural relationships;
- construction work lineage;
- temporal history relevant to persistence/replay.

Grouping should support arbitrary nesting:

```text
component
-> assembly
-> furnishing / machine / room-scale system
-> building
-> site
-> block / district
-> settlement
```

and independently:

```text
component
-> subsystem
-> module
-> vehicle
-> spacecraft
-> station / shipyard
```

These are organizational views over authority, not an excuse to lose exact lower-level state when fidelity requires it.

## 6. Place and home identity

A home is intentionally **not** created by placing a `HomeCore`, setting `is_home = true`, or selecting a predefined room recipe.

Symtropy should distinguish at least:

```text
physical structure
!= enclosed/usable space
!= socially recognized place
!= inhabited home
!= ownership/title
```

### 6.1 `PlaceIdentity`

A future place layer should provide persistent identity for locations/assemblies that acquire social or gameplay meaning. It may refer to a stable set or lineage of physical construction identities without becoming their physical authority.

A place should be capable of retaining:

- stable place identity;
- optional player/NPC-given names and rename history;
- constituent structure/site references and lineage;
- creation/founding time when known;
- builder/contributor references when evidenced;
- repairs, expansions, partial destruction, salvage, and rebuilding references;
- inhabitant/visitor associations as observations or records;
- meaningful events that occurred there;
- inherited provenance when a rebuilt place intentionally continues an older place.

### 6.2 `SpaceSnapshot`

Rooms and usable volumes should preferably be derived from geometry, openings, connectivity, environmental boundaries, and current physical state.

A player should not need to paint `Kitchen` onto a closed polygon for the world to understand that a space exists.

Derived space identity must tolerate irregular geometry, caves, vehicle interiors, connected modules, damaged walls, open-plan layouts, courtyards, and changing construction.

### 6.3 Use semantics are derived, not prescribed

A kitchen may emerge because food is repeatedly stored/prepared there and relevant equipment/utilities exist. A bedroom may emerge from repeated sleep/privacy/storage behavior. A workshop may emerge from workbench/tool/process activity.

Classifiers may produce multiple overlapping uses and uncertainty. They are projections/interpretations, not physical truth.

This avoids forbidding unconventional but valid player arrangements.

### 6.4 Home emergence

A `HomeAssociation` should be supported by persistent evidence such as repeated habitation, sleeping, personal storage, routines, social recognition, attachment to the place, or an explicit character intention. Different characters may disagree about whether the same place is home.

The system must allow:

- one character to have several homes;
- several characters to share one home;
- a nomadic/mobile home;
- an improvised shelter becoming a long-term home;
- a former home retaining historical meaning after abandonment;
- a destroyed home continuing as a place lineage after rebuilding;
- ownership without emotional/home association;
- home association without formal ownership.

No universal scalar `home_score` is required for authority.

## 7. Architectural time

Player attachment requires buildings to accumulate history instead of remaining interchangeable geometry.

The rendering/simulation stack should eventually be able to derive visible and audible consequences from authoritative history and environment, including wear, weathering, repairs, soot/scorching, material aging, traffic paths, vegetation growth, accumulated possessions, and repaired damage.

These effects must not corrupt underlying provenance. Presentation may summarize or bake distant history for performance while retaining enough canonical state to reconstruct meaningful facts.

The sound and light identity of a place are first-class presentation goals: rain on a particular roof, floor resonance, machinery hum, wind leakage, room reverb, window orientation, seasonal sunlight, and other persistent environmental characteristics can make a home recognizable without a HUD label.

## 8. Destruction and rebuilding

Destruction should preserve story whenever the physical authorities can preserve matter/provenance.

The preferred lifecycle is:

```text
constructed state
-> damage
-> partial failure
-> stabilization opportunity
-> fracture / debris / rubble where applicable
-> salvage
-> repair or reconstruction
-> continued or successor place identity
```

Catastrophic failure should not normally erase the place into a generic loot table.

A repaired original beam, reused hull plate, salvaged wall stone, rebuilt foundation, or inherited furnishing can remain part of the place's biography.

Gameplay should expose progressive warnings and emergency stabilization opportunities where physically appropriate so a complex home is not routinely destroyed by an opaque single-frame threshold crossing.

## 9. Scaling to bases, cities, spacecraft, and stations

The construction language must scale by changing planning and simulation resolution, not by changing physical semantics.

### Homes and bases

Direct player placement, small crews, stockpiles, temporary works, repair, utilities, and local structural feedback are appropriate.

### Settlements and cities

The player increasingly authors goals, corridors, parcels, blueprints, requirements, infrastructure networks, and priorities. NPCs/organizations/machines turn those intentions into construction work through normal material/logistics/authority paths.

A city is not one object. It is a multiscale composition of places, structures, networks, institutions, logistics, and inhabitants.

### Vehicles and spacecraft

Vehicle construction should be the first proof that the language is not secretly architecture-specific. A small cart/rover is a better initial bridge than a starship.

Later spacecraft add specialized requirements such as pressure boundaries, thermal control, mass distribution, propulsion mounts, life support, power, avionics, docking, and commissioning. These specialized analyses constrain physical realizations; they do not create a separate placement authority.

### Orbital stations

Stations naturally compose modules and can validate the hierarchical model:

```text
outpost
-> habitat/station
-> industrial station
-> shipyard
-> orbital settlement
```

Vacuum boundary, pressure integrity, thermal rejection, power, docking, logistics, radiation environment, and emergency isolation become domain-specific requirements over shared construction/fabrication reality.

## 10. Hierarchical fidelity contract

Large structures cannot remain fully simulated at maximum microscopic detail at all times.

Symtropy should support deterministic authority-preserving coarse representations of inactive/distant construction, with explicit promotion/refinement when a process requires finer information.

At minimum, coarse construction state must not silently lose facts needed by surviving semantics such as:

- conserved material quantity;
- stable place/assembly identity;
- important provenance;
- structural capacity/damage state at the chosen representation;
- utility connectivity/capacity relevant at that scale;
- pressure/environmental boundary state where relevant;
- unresolved failures or hazards;
- ownership/permission references when required by consumers.

Renderer distance and frame rate are never valid reasons to change canonical physical outcomes.

## 11. First vertical acceptance theorem: "the shelter that became home"

The first player-facing runtime milestone should deliberately be small enough to qualify rigorously.

One player must be able to:

1. acquire or recover real construction material through existing authority paths;
2. select a building operation without choosing a complete prefab house;
3. place at least foundation/support, frame/wall, opening, and roof-relevant geometry using free placement;
4. optionally enable deterministic snapping/alignment without making it mandatory;
5. persist an incomplete structure, leave, reload, and continue work;
6. receive advisory structural/environmental feedback distinct from hard authority failures;
7. finish an enclosed/usable space without manually assigning a room type;
8. place/use sleeping and personal-storage functions through their owning gameplay authorities;
9. allow the place to acquire persistent identity and a character-relative home association through habitation/use;
10. damage and repair at least one original constituent while preserving identity/provenance;
11. save/load and deterministic-replay the complete sequence;
12. dismantle or salvage part of the structure without silently creating/deleting conserved matter.

Passing this theorem is more important than shipping hundreds of decorative building pieces.

## 12. Required hostile tests

Runtime successors should include adversarial fixtures for at least:

- off-grid crooked but valid construction;
- snap enabled/disabled producing equivalent physical semantics when geometry is equivalent;
- snapping never creating execution authority;
- non-finite or malformed geometry failing closed;
- stale plan after site/material/capability authority drift failing closed;
- duplicate/replayed execution request not duplicating matter or work;
- incomplete structure surviving save/load;
- rendering disabled or frame cadence changed without changing canonical construction outcome;
- presentation/semantic-space classifiers being unable to mutate physical authority;
- same physical structure supporting different character-relative home associations;
- ownership and home association remaining independent;
- destroyed/rebuilt place retaining explicit lineage rather than silently becoming the same untouched structure;
- salvage/rebuild preserving available provenance;
- blueprint/delegated construction consuming the same lower authority paths as direct player work;
- large-scale/coarse representation not losing an authority fact required by an active process.

## 13. Recommended implementation sequence

This contract deliberately prioritizes integration over feature count.

### PB-00 — this contract

Freeze the authority, freedom, home/place, and scaling semantics. Documentation only.

### PB-01 — proposal IR

Add dependency-light `ConstructionIntent`, deterministic `ConstructionPlan` proposal identity, operation graph, planning-context binding, advisory/hard constraint separation, and read-only preview structures.

PB-01 must not execute matter or construction itself.

### PB-02 — exact Construction adapter

Compile accepted PB-01 operations into the current exact `symtropy-construction` / Fabrication / matter-authority ingress once those exact product heads are integration-qualified. No second executor.

### PB-03 — free placement + deterministic snap projection

Implement continuous transforms, optional deterministic snap candidate generation, measurement aids, constraint overlays, and a no-render deterministic planning fixture.

### PB-04 — semantic space projection

Derive persistent-enough space snapshots from realized geometry/connectivity. Read-only with respect to physical authority.

### PB-05 — place identity + home association

Add persistent place lineage and character-relative home associations. Keep ownership, civic status, physical construction, and emotional/social meaning separate.

### PB-06 — shelter acceptance cell

Ship the complete "shelter that became home" deterministic vertical with save/load, damage/repair, salvage, and replay evidence.

### PB-07 — blueprint and delegation

Allow arbitrary player-authored plans/assemblies to become reusable design inputs and delegated work, while consuming real materials/capabilities and preserving exact execution authority.

### PB-08 — hierarchical construction fidelity

Add authority-preserving coarse/refined construction representation required before city or station scale.

### PB-09 — rover construction proof

Build a small physically functional vehicle from the same construction/fabrication language. This is the anti-overfitting gate before spacecraft work.

### PB-10 — settlement infrastructure composition

Compose roads, utilities, logistics, structures, places, and delegated construction without introducing a monolithic city object.

### PB-11 — pressure-bound habitat/orbital proof

Only after PB-08/PB-09: prove modular pressure boundary, utilities, repair/isolation, docking-relevant structure, and hierarchical persistence for an orbital habitat cell.

## 14. Deliberate non-goals of v0.1

This contract does not claim or introduce:

- a finished player building UI;
- runtime freeform CSG/CAD;
- a new conserved-matter implementation;
- a new structural solver;
- a new Fabrication or Construction executor;
- automatic engineering certification;
- arbitrary NPC autonomy over construction;
- instant city generation;
- complete spacecraft engineering;
- universal real-world building-code compliance;
- legal ownership or civic permission inferred from physical possession;
- a scalar "home quality" or universal definition of home;
- runtime qualification of any active construction branch.

## 15. Merge gate

PB-00 is documentation-only and may merge independently if the authority contract is accepted.

PB-01 and later must not claim executable qualification merely because this document exists. Each runtime tranche must state its exact base/head and earn the appropriate format/check/test/strict-Clippy/replay/conservation evidence for that exact lineage.

Most importantly:

> No player-building successor may bypass or duplicate the exact physical authority stack merely to make the UX easier to implement.

The UX should become extremely expressive. Reality should remain singular.