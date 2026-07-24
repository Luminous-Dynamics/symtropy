---
title: Construction, Repair, and Structural Transformation Contract
version: 0.1
status: canonical
scope: player construction, physical repair, structural failure, demolition, public works, building rights, and built-world transformation
owner: design/construction/physics/simulation
related:
  - tech/STRUCTURAL_INTEGRITY_CONSTRUCTION_AND_DESTRUCTION_RUNTIME_V0_1.md
  - tech/Symtropy Design Doc - Cybernetic Crafting & Physical Node Assembly.md
  - vision/Symtropy Architecture Design Bible.md
  - canon/PLAYER_AUTHORSHIP_SANDBOX_AND_MODDING_CONTRACT_V0_1.md
  - canon/PROGRESSION_ECONOMY_AND_MASTERY_CONTRACT_V0_1.md
  - tech/DEVICE_BUS_RUNTIME_SAFETY.md
---

# Construction, Repair, and Structural Transformation Contract

## Owned Question

**How can players physically build, repair, expand, repurpose, damage, and dismantle structures while preserving tactile craft, systemic consequences, architectural identity, multiplayer safety, and production feasibility?**

## What This Document Does Not Own

This contract does not define the complete asset catalog, rendering pipeline, every architectural culture, or detailed rigid-body solver. It owns the player-facing construction grammar, structural meaning, authority boundaries, failure principles, and milestone constraints.

## Core Thesis

Construction in Symtropy is not freeform object placement and not a menu that converts resources into finished buildings.

It is the transformation of a real place through:

```text
survey
foundation
materials
assembly
connection
commissioning
use
maintenance
adaptation
decommissioning
```

A built structure becomes part of civilization when bodies can use it, systems can depend on it, maintainers can understand it, and the world can remember what it changed.

> **Buildings are promises made physical. Structural failure is a broken promise with causes.**

## Prime Directives

1. **No construction without place.** Terrain, foundations, access, weather, geology, ecology, and existing infrastructure constrain building.
2. **No major structure from an inventory click.** Significant construction requires staging, physical assembly, machines, labor, and commissioning.
3. **No arbitrary collapse.** Structural failure must propagate from load, damage, material condition, geometry, environment, or maintenance history.
4. **No universal block language.** Modular systems must still express cultural architecture, body assumptions, climate, and construction traditions.
5. **No decorative infrastructure.** Pipes, power, ventilation, access, storage, computing, drainage, and maintenance routes must matter where represented.
6. **No single perfect material tier.** Materials trade strength, weight, corrosion, thermal behavior, repairability, local availability, and environmental cost.
7. **No player creativity held hostage by bureaucracy.** Permissions and registration matter for shared or hazardous systems, not every chair or private wall.
8. **No griefing through realism.** Public life-support, spawn, recovery, and occupied structures receive explicit protection and reversible authority tools.
9. **No destruction as the cheapest universal solution.** Demolition creates debris, contamination, displacement, evidence loss, and rebuilding cost.
10. **No construction system too expensive to populate the world.** The simulation must distinguish structural gameplay objects from visual detail.

# 1. Construction Scales

## 1.1 Hand Repair and Fixtures

```text
patches
brackets
seals
cables
valves
panels
furniture
tool mounts
small sensors
```

These use direct embodied interaction and minimal authority.

## 1.2 Modules and Rooms

```text
habitat cells
workshop bays
greenhouse modules
storage rooms
clinic rooms
vehicle garages
pressure compartments
```

These require anchors, utility connections, access paths, and commissioning.

## 1.3 Buildings and Public Works

```text
bridges
waterworks
power stations
factories
transit stations
clinics
schools
archives
fortifications
```

These require site survey, project planning, logistics, labor, staged construction, inspections, and maintenance plans.

## 1.4 Networks and Megastructures

```text
roads
rail corridors
utility grids
orbital habitats
launch systems
planetary engineering
```

These are multi-project programs whose value comes from connected operation, not one placed object.

# 2. The Construction Loop

```text
1. Read the site.
2. Define purpose and constraints.
3. Select or author a design.
4. Prepare foundations and access.
5. Stage materials, tools, labor, and machines.
6. Assemble physical structure.
7. Connect utilities and control systems.
8. Test and commission.
9. Register shared authority where required.
10. Maintain, adapt, expand, or decommission.
```

Every stage may be compressed for small objects and expanded for public works.

# 3. Design Modes

## 3.1 Repair Mode

Restore or replace damaged elements while preserving as much existing structure and history as possible.

## 3.2 Modular Assembly

Combine authored modules through valid interfaces.

This is the default for Seedworks-scale buildings because it supports reliability, cultural kits, navigation, and performance.

## 3.3 Parametric Construction

Players adjust bounded dimensions, spans, slopes, capacity, openings, and service routes.

## 3.4 Structural Synthesis

Advanced tools generate candidate structures from constraints, but the result must expose materials, loads, maintenance, and failure modes.

## 3.5 Freeform Detail

Decoration, furnishing, signage, gardens, and personal expression may be more permissive because they do not own structural authority.

# 4. Structural Meaning

A structure is evaluated across:

```text
load capacity
stability
serviceability
fatigue
thermal behavior
pressure integrity
weather resistance
fire resistance
corrosion and decay
accessibility
maintainability
utility continuity
escape and rescue
```

The player does not need an engineering degree. Interfaces translate these into readable states, warnings, physical cues, and bounded recommendations.

# 5. Materials and Provenance

Materials carry:

```text
composition
condition
dimensions
strength envelope
mass
thermal behavior
corrosion profile
fabrication history
contamination
repair compatibility
reuse potential
```

Salvaged material can be valuable and uncertain. Certified material can be reliable and politically enclosed. Local biological material may regenerate but require care.

Material quality changes safety margins and maintenance, not merely hit points.

# 6. Foundations and Site Conditions

Site survey may reveal:

```text
bearing capacity
slope
flood risk
subsurface voids
seismic activity
permafrost or thermal cycling
corrosion environment
alien substrate response
protected habitat
buried infrastructure
historical remains
```

Players may stabilize, avoid, bridge, float, suspend, or deliberately accept risk.

# 7. Utilities and Commissioning

A building is not operational merely because its shell exists.

Commissioning verifies:

```text
power
water or process fluids
air and pressure
thermal control
data and Device Bus
waste and drainage
fire or hazard response
access and evacuation
control authority
```

Temporary operation is possible with warnings and maintenance debt.

# 8. Repair Quality

Repairs may be:

```text
emergency
provisional
functional
certified
restorative
transformative
```

Repair quality depends on diagnosis, material compatibility, tool control, access, skill, and testing.

A provisional repair can save lives without pretending to be permanent. It creates a visible future obligation rather than an invisible durability penalty.

# 9. Damage, Destruction, and Salvage

Damage categories:

```text
surface
component
connection
structural member
foundation
utility network
progressive collapse
```

Destruction should produce:

```text
debris
blocked routes
hazard zones
salvage
pollution or dust
lost utilities
injury and displacement
historical evidence
new tactical geometry
```

The player may brace, isolate, evacuate, dismantle, demolish, or rebuild.

# 10. Public Authority and Ownership

Construction rights are bundles:

```text
survey
place
connect
operate
modify
demolish
salvage
exclude
inspect
```

Private expression should be easy. Shared life-support, defense, hazardous industry, transport, and occupied structures require scoped authority.

Emergency construction may proceed under temporary authority with review and expiry.

# 11. Cultural Architecture

Modularity must not flatten culture.

Every construction kit defines:

```text
body assumptions
climate response
materials
span and mass language
public/private thresholds
maintenance visibility
ornament and memory
failure response
```

Two cultures may use the same structural solver and produce radically different places.

# 12. Automation and Labor

Machines may lift, print, weld, inspect, excavate, and route materials. Automation removes repetition but does not eliminate:

```text
site judgment
maintenance
quality control
resource logistics
public decision
adaptation to surprise
```

NPC labor has fatigue, safety, skill, bargaining, and care constraints.

# 13. Multiplayer Safety

Worldline profiles define construction permissions. Minimum protections include:

```text
no deletion of occupied or protected structures without authorized process
no trapping players through unconsented construction
no severing essential utilities through low-authority edits
no invisible structural sabotage
no duplication through rollback or module split/merge
```

Targeted rollback must preserve legitimate downstream work where possible.

# 14. Seedworks Boundary

The representative build should prove:

```text
one tactile repair
one modular structure or bridge segment
one utility connection
one visible load or support problem
one provisional-versus-permanent choice
one construction consequence that changes traversal or settlement capability
one safe co-op role split
```

It does not require freeform cities, megastructures, or full destruction.

# 15. Acceptance Tests

1. A player can understand why a placement is invalid or risky.
2. A completed structure changes at least one physical capability and one world-state consequence.
3. Structural failure follows traceable load or damage paths.
4. A provisional repair remains useful and visibly creates future work.
5. Modular kits preserve distinct cultural and environmental identities.
6. Utility connections can degrade independently of the structural shell.
7. Co-op construction supports meaningful roles without requiring every player to use the same tool.
8. Protected structures resist griefing without making legitimate conflict impossible.
9. Save, rollback, and migration preserve material provenance and construction ancestry.
10. Construction remains within declared entity, physics, and rendering budgets.

## Final Rule

```text
Players should feel that they changed a place—not that they placed a prefab into empty terrain.
```
