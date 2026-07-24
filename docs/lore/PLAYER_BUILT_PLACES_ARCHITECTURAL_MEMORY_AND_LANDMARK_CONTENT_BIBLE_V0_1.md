---
title: Player-Built Places, Architectural Memory, and Landmark Content Bible
version: 0.1
status: canonical-draft
scope: constructed places, repair history, adaptive reuse, landmarks, demolition, ruins, local names, social memory
owner: world/narrative/environment/art/design
related:
  - ../canon/CONSTRUCTION_REPAIR_AND_STRUCTURAL_TRANSFORMATION_CONTRACT_V0_1.md
  - ../canon/PLAYER_FOUNDED_CIVILIZATION_SETTLEMENT_LEGACY_AND_WORLDLINE_CONTRACT_V0_1.md
  - LEGENDARY_CITIES_HABITATS_AND_WORLD_REGIONS_ATLAS_V0_1.md
  - CULTURAL_SEASONS_FESTIVALS_ART_AND_EVERYDAY_LIFE_CONTENT_BIBLE_V0_1.md
  - ../tech/PLAYER_CREATED_CULTURE_RITUAL_ART_AND_SYMBOL_RUNTIME_V0_1.md
---

# Player-Built Places, Architectural Memory, and Landmark Content Bible

## Purpose

This bible defines how player construction becomes inhabited place, public memory, ordinary inconvenience, contested heritage, or ruin.

A building is not historically meaningful because the player placed it. Meaning emerges through use, repair, dependency, stories, conflict, adaptation, and time.

> **The player builds structures. Inhabitants make places. History decides what becomes a landmark.**

# 1. Place Layers

Every durable place may accumulate:

```text
site history
original intent
design authorship
material provenance
construction labor
first use
routine use
repair generations
accidents
private memories
public events
renaming
adaptive reuse
accessibility changes
ownership disputes
demolition proposals
ruin and salvage
```

The simulation should preserve several of these as evidence and visual state.

# 2. Construction Attribution

Record:

- who proposed the structure;
- who designed it;
- who supplied materials;
- who performed labor;
- who financed it;
- whose land or claim was affected;
- who maintains it;
- who uses it;
- who was excluded;
- who altered it later.

A player may be the initiator without being the builder, owner, maintainer, or primary beneficiary.

# 3. Material Memory

Materials carry history:

- salvaged hull plates;
- local stone;
- imported composite;
- repaired timber;
- corporate standardized panels;
- alien-grown material;
- contaminated but encapsulated structure;
- pieces recovered from a destroyed home;
- recycled festival scaffolding;
- machine-fabricated parts with obsolete signatures.

Material provenance may affect:

- maintenance;
- appearance;
- symbolic meaning;
- legal claims;
- contamination;
- cultural acceptance;
- salvage value;
- preservation disputes.

# 4. Repair Generations

A place should visibly record repair:

- patch plates;
- mismatched tiles;
- replaced seals;
- changed door geometry;
- handrails added later;
- flood marks;
- rewired lighting;
- locally fabricated brackets;
- inaccessible old stairs bypassed by ramps;
- temporary structures that became permanent;
- repairs made by named crews.

A pristine asset can be less beloved than a repeatedly repaired one.

# 5. Local Names

Places may have:

- legal name;
- construction code;
- sponsor name;
- worker nickname;
- resident name;
- hostile name;
- youth slang;
- old name;
- map abbreviation;
- memorial name;
- name used only by one household.

The UI should select names by speaker, context, and knowledge rather than enforcing one universal label.

# 6. Landmark Emergence

A place becomes a landmark when several conditions converge:

- repeated use;
- shared orientation;
- memorable event;
- distinctive form;
- cultural adoption;
- public dependency;
- contested interpretation;
- longevity;
- representation in art or media;
- connection to a person or movement.

```rust
struct LandmarkState {
    entity_id: EntityId,
    recognition_by_group: Map<ConstituencyId, f32>,
    names: Vec<NameRecord>,
    associated_events: Vec<ChronicleEventId>,
    cultural_artifacts: Vec<ArtifactId>,
    maintenance_state: MaintenanceState,
    heritage_status: Vec<HeritageStatus>,
    access_conflicts: Vec<ConflictId>,
}
```

There is no universal `landmark=true` flag that every society instantly shares.

# 7. Heritage and Use

Preservation may conflict with:

- housing;
- accessibility;
- safety;
- ecological restoration;
- public-service capacity;
- private grief;
- sacred boundaries;
- economic need;
- political repudiation.

A building can be historically significant and still need alteration or demolition.

The game should not treat preservation as automatically virtuous.

# 8. Adaptive Reuse

Possible transformations:

- pump house to public bath;
- barracks to housing;
- casino to school;
- founder residence to clinic;
- warehouse to theater;
- launch gantry to memorial garden;
- corporate office to cooperative workshop;
- detention facility to evidence archive;
- chapel to multi-use shelter;
- reactor hall to protected ruin;
- abandoned ship to neighborhood.

Reuse preserves some history and destroys other history.

# 9. Demolition

Demolition requires decisions about:

- safety;
- ownership;
- public need;
- displacement;
- hazardous material;
- salvage;
- records;
- memorial objects;
- ecological effect;
- labor;
- who can object.

The player should sometimes watch a beloved structure be removed through a legitimate process.

# 10. Ruins

Player-built structures may become ruins through:

- abandonment;
- disaster;
- war;
- ecological change;
- migration;
- insolvency;
- obsolete technology;
- deliberate decommissioning;
- failed expansion;
- worldline divergence.

Ruins retain:

- material traces;
- access hazards;
- salvage;
- records;
- stories;
- legal claims;
- ecological adaptation;
- contested memory.

They do not revert to empty build tiles.

# 11. Twenty-Four Place Archetypes

## 11.1 The First Shared Pump

Originally a temporary pump assembled from three incompatible systems. Later enclosed by a public utility building. The original patch remains visible because workers refuse to cover the names scratched into it.

Possible futures:

- protected technical landmark;
- embarrassing symbol of poor early planning;
- still-operating emergency backup;
- removed during modernization;
- falsely credited to the founder alone.

## 11.2 South Cut Clinic

Built too small during the founding winter. Expanded five times. Every addition uses different corridor widths and privacy standards.

Its history appears through:

- reused doors;
- memorial tiles;
- an old triage window;
- a private garden added by patients;
- arguments over whether to move to a modern site.

## 11.3 Workshop Number Two

The first workshop burned. The replacement acquired the number two as a joke and later became the center of an apprentice movement.

## 11.4 The Unfinished Council Room

Residents began building a ceremonial chamber but diverted materials to housing. Meetings happen under an exposed roof frame for twelve years. Some later oppose completion because the unfinished state became a symbol of priorities.

## 11.5 Temporary Block C

Emergency housing intended for six months survives for forty years. Residents improve it, politicians promise replacement, and heritage advocates later romanticize conditions the original residents hated.

## 11.6 The Shared Kitchen

Created because private cooking fuel was scarce. Becomes a food market, childcare exchange, gossip center, and political organizing site.

## 11.7 Founder's House

The player may build or occupy it. Later possibilities include:

- ordinary private home;
- inherited household;
- public office;
- museum;
- clinic;
- ruin;
- demolished structure;
- site residents refuse to commemorate.

## 11.8 Bridge 4A

A plain bridge that matters because it remained open during evacuation. The engineer's name is forgotten; the night crew's improvised brace becomes a local emblem.

## 11.9 The Quiet Steps

An accessible route originally added after public protest. Later a common meeting place. Its landmark status derives from use, not architectural grandeur.

## 11.10 Old Battery Yard

A contaminated industrial site converted into storage, then art space, then housing. Each reuse leaves unresolved health and heritage arguments.

## 11.11 The Rain Court

A drainage basin that becomes a sports field during dry months and flood infrastructure during storms. Attempts to build permanent seating threaten its hydraulic function.

## 11.12 The Wrong Statue

A memorial sculptor uses the face of the wrong technician. The mistake becomes known, disputed, and eventually part of the monument's meaning.

## 11.13 Five-Meter Market

A strip of informal stalls allowed under a temporary five-meter setback exemption. The exemption expires; the market does not.

## 11.14 The Repaired Wall

A defensive wall becomes unnecessary but remains because it contains generations of repair inscriptions and family memorials.

## 11.15 East Lift

An unreliable public elevator that connects a steep district. Residents hate it, depend on it, decorate it, and oppose closure.

## 11.16 The Borrowed Dome

A corporate habitat shell leased for twenty years. When the contract ends, residents argue that a structure containing homes cannot be repossessed as ordinary equipment.

## 11.17 Morrow's Alcove

A service robot's preferred maintenance corner becomes a small social place. Later restorations disagree whether the alcove belongs to the original Morrow-7, a fork, the workshop, or no one.

## 11.18 The Broken Clock

A public clock stops during a disaster. It remains stopped for years, then is repaired against the wishes of memorial groups. Different displays preserve both times.

## 11.19 The Seed Library

Begins as a cabinet in a workshop. Expands into a climate-controlled institution. Its most important artifact is an ugly handwritten inventory that prevented a lineage from being lost.

## 11.20 The Noisy Roof

A cheap roof resonates in wind. Musicians incorporate the sound into local performance, while residents beneath it campaign for quiet replacement.

## 11.21 The Empty Foundation

A planned prestige building is cancelled. The foundation becomes play space, market, flood basin, and eventually protected evidence of a rejected political era.

## 11.22 River Door 3

A maintenance access gate associated with smuggling, rescue, court evidence, and adolescent dares. Official maps use a different name.

## 11.23 The Last Corporate Sign

Residents remove most branding after buyout but retain one sign because it gives directions and has become absurdly familiar.

## 11.24 The Common Repair Table

A movable worktable outlives three buildings. Its scratches and modifications record the settlement's technical history more accurately than the founder monument.

# 12. Visual Production Requirements

Environment art should support:

- layered materials;
- nonuniform repairs;
- local additions;
- accessibility retrofits;
- signage history;
- weathering by real exposure;
- resident decoration;
- utility routing;
- conflicting uses;
- demolition and salvage states.

Procedural variation must remain traceable to authored or simulated causes.

# 13. Player Interaction

Players may:

- inspect provenance;
- repair;
- alter;
- petition for preservation;
- document;
- rename through valid process;
- donate artifacts;
- oppose demolition;
- salvage;
- adapt use;
- create private memories;
- discover they lack authority.

Landmarks should not become checklist collectibles.

# Closing Principle

> **The most important building may be the one the player barely remembers placing and the settlement cannot imagine living without.**
