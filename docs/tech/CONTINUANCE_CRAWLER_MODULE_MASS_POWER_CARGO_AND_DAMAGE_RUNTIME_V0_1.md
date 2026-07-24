---
title: Continuance Crawler Module, Mass, Power, Cargo, and Damage Runtime
version: 0.1
status: implementation-spec
scope: mobile base simulation, module graph, mass and balance, power and heat, water and waste, cargo, damage, repair and deployment
owner: vehicle-simulation/physics/gameplay/networking
related:
  - ../canon/MOBILE_CONTINUANCE_BASE_HOME_VEHICLE_AND_CREW_SOVEREIGNTY_CONTRACT_V0_1.md
  - VEHICLE_SPACECRAFT_PHYSICS_AND_OPERATIONS_RUNTIME_V0_1.md
  - ../SYMTROPY_RESOURCE_CHAINS_GAME_DOC_V0_1.md
  - PLANETARY_INFRASTRUCTURE_NETWORKS_AND_CORRIDOR_RUNTIME_V0_1.md
  - ../ops/BREAKING_OF_FIRSTLIGHT_AND_CONTINUANCE_BASE_BENCHMARK_V0_1.md
---

# Continuance Crawler Module, Mass, Power, Cargo, and Damage Runtime

## Purpose

Define the authoritative physical and operational model for the first mobile base and its later modular descendants.

The runtime must make the crawler satisfying to drive, inhabit, repair, load, damage, and transform while conserving matter, capacity, energy, labor, authority, and people.

# 1. Entity Model

The crawler is an assembly graph.

```text
crawler
  ├── chassis
  ├── mobility assemblies
  ├── power network
  ├── thermal network
  ├── water and sanitation network
  ├── atmosphere and environmental control
  ├── structural modules
  ├── cargo restraints
  ├── workstations
  ├── sensors and communication
  ├── resident spaces
  └── attached vehicles or trailers
```

Each component has:

- unique ID;
- owner or custodian;
- mass;
- geometry;
- mount points;
- material state;
- operating envelope;
- dependencies;
- maintenance history;
- damage;
- authority requirements;
- provenance.

# 2. Chassis State

The chassis tracks:

- frame geometry;
- structural members;
- suspension or leg geometry;
- wheel, track, or foot contact;
- steering;
- brakes;
- propulsion;
- ground clearance;
- turning envelope;
- traction;
- axle or support loads;
- rollover margin;
- fatigue;
- corrosion;
- contamination;
- current mass distribution.

Vehicle handling derives from these states rather than a tier number.

# 3. Mass and Center of Gravity

Every installed component, resident, consumable, and cargo object contributes mass at a location.

The runtime calculates:

- total mass;
- center of gravity;
- per-axle or support load;
- pitch and roll moments;
- braking demand;
- route pressure;
- suspension travel;
- stability margin.

Loading a heavy archive core high on the rear deck should alter handling.

Passengers may move during travel only within declared safety states.

# 4. Route Compatibility

Routes expose:

- width;
- height;
- turning radius;
- surface strength;
- grade;
- traction;
- water depth;
- overhead hazards;
- bridge load;
- noise restrictions;
- legal access;
- environmental limits.

The crawler evaluates compatibility using current configuration.

A route may be physically possible but unsafe, illegal, ecologically destructive, or politically unacceptable.

# 5. Power Network

Power sources may include:

- batteries;
- fuel cells;
- combustion or turbine generator;
- solar;
- grid connection;
- external tow or beam power;
- advanced metric or alien systems later in progression.

Loads include:

- propulsion;
- steering and brakes;
- workshop;
- medical systems;
- lighting;
- heating and cooling;
- pumps;
- sanitation;
- computing;
- sensors;
- communications;
- charging machine bodies;
- personal equipment.

The network tracks:

- voltage and frequency class;
- capacity;
- peak demand;
- breaker and isolation state;
- cable limits;
- conversion loss;
- source health;
- stored energy;
- fault state.

# 6. Thermal Network

Heat sources and sinks are explicit.

Sources include:

- propulsion;
- batteries;
- generators;
- workshop tools;
- medical sterilization;
- electronics;
- residents;
- environment;
- fire;
- metric equipment.

Sinks include:

- radiators;
- air exchange;
- water loops;
- ground coupling;
- evaporation;
- external coolant.

Thermal overload may reduce power, injure occupants, damage materials, force route or schedule changes, or require visible radiator deployment.

# 7. Water and Sanitation

The crawler tracks:

- potable water;
- technical water;
- greywater;
- blackwater;
- contamination;
- filters;
- treatment capacity;
- tank geometry and slosh;
- pipes;
- pumps;
- hygiene demand;
- medical demand;
- food preparation;
- ecological cultures.

Water is not one generic resource number.

Reduced-detail simulation may aggregate state while preserving contamination, total mass, service availability, and critical dependencies.

# 8. Atmosphere and Shelter

For sealed or climate-controlled configurations, track:

- temperature;
- humidity;
- ventilation;
- smoke;
- toxins;
- pathogens;
- pressure where relevant;
- noise;
- occupancy;
- isolation zones.

Most early Earth crawlers are not spacecraft, but medical isolation, smoke, chemical hazards, extreme weather, and later planetary environments require bounded environmental control.

# 9. Module Interface

A module declares:

- structural mounts;
- power inputs and outputs;
- thermal interfaces;
- water interfaces;
- data interfaces;
- access doors;
- safety clearances;
- occupancy;
- ownership;
- required certifications;
- deployment geometry.

Installation requires tools, labor, time, space, compatible structure, and authority.

Modules cannot appear instantly through a menu.

# 10. Reference Modules

## 10.1 Cockpit and Navigation

Requires sightlines, control interfaces, route data, driver accommodation, communication, and emergency overrides.

## 10.2 Workshop

Provides tool storage, benches, fabrication, lifting, calibration, spares, fire risk, noise, and waste.

## 10.3 Medical Berth

Provides cleanable surfaces, privacy, stabilization, storage, power, water, isolation, and handover records.

## 10.4 Habitation

Provides bunks, seats, personal storage, ventilation, lighting, privacy, restraint, and social space.

## 10.5 Kitchen and Commons

Provides food storage, preparation, water, waste, heat, seating, and cultural life.

## 10.6 Archive Locker

Provides physical and cryptographic custody, environmental control, access logging, and disputed authority.

## 10.7 Ecology Module

Carries seeds, microbes, plants, animals, water cultures, or habitat samples with distinct conditions.

## 10.8 Defense Module

Provides sensors, armor, concealment, drones, countermeasures, or weapons with legal and social constraints.

## 10.9 Deployable Utility Module

Provides temporary power, water, communications, bridge support, clinic, or workshop service outside the crawler.

# 11. Cargo Model

Cargo objects track:

- mass;
- dimensions;
- center of mass;
- fragility;
- restraint;
- stackability;
- environmental needs;
- hazard class;
- ownership;
- custody;
- access priority;
- replacement cost;
- cultural significance;
- dependencies.

Cargo placement is physical.

An emergency object buried behind other cargo is not instantly accessible.

# 12. Resident Transport Requirements

Residents require:

- seat, bunk, standing area, stretcher, or specialized support;
- restraint appropriate to movement;
- air and temperature;
- water and sanitation;
- privacy where needed;
- medication or care;
- accessibility route;
- machine power or maintenance;
- safe social conditions.

The system must distinguish a person's rights and needs from cargo math while still representing physical constraints.

# 13. Damage Model

Damage types include:

- deformation;
- fracture;
- puncture;
- wear;
- corrosion;
- overheating;
- electrical fault;
- contamination;
- fire;
- water ingress;
- sensor failure;
- software or authority fault;
- metric or alien interference.

Damage propagates through dependencies.

A roof strike may deform a hatch, sever wiring, block a corridor, and prevent module deployment.

# 14. Repair

Repair actions specify:

- diagnosis;
- isolation;
- access;
- tools;
- materials;
- labor;
- time;
- safety;
- certification;
- temporary versus permanent result;
- inspection.

Repairs leave history.

A field patch may remain visible, alter later fatigue, become a beloved feature, or be criticized by another technician.

# 15. Cannibalization and Salvage

The player may recover components from:

- abandoned vehicles;
- ruined infrastructure;
- purchased equipment;
- donated modules;
- enemy assets under lawful salvage;
- their own damaged systems.

Salvage records provenance and possible claims.

Removing a component may damage the source or require tools and time.

# 16. Deployment

The crawler can deploy systems while parked:

- leveling supports;
- solar or thermal arrays;
- awnings and shelter;
- water intake and treatment;
- workshop space;
- medical area;
- communications mast;
- observation platform;
- defensive sensors;
- market or commons.

Deployment changes silhouette, power, vulnerability, route readiness, and local land use.

Emergency departure may require abandoning deployed systems.

# 17. Convoy

A convoy is a coordination graph among independent vehicles.

State includes:

- membership;
- route plan;
- spacing;
- communication;
- towing;
- shared services;
- rescue obligations;
- fuel and water exchange;
- authority;
- departure rights.

No vehicle becomes a module of the player's base merely by joining the convoy.

# 18. Automation and IRIS

IRIS may:

- summarize state;
- identify uncertainty;
- propose load plans;
- warn of imbalance;
- assist navigation;
- coordinate diagnostics;
- preserve maintenance history;
- support accessibility.

IRIS may not:

- move owned cargo without authority;
- assign residents as labor;
- override lawful refusal;
- conceal damage to protect player confidence;
- optimize away cultural or personal objects without explicit criteria.

# 19. Simulation Levels

## Full Detail

Used during driving, repair, loading, combat, deployment, and inhabited interior play.

## Operational

Used for nearby convoy travel and routine systems.

## Historical

Used while the crawler is distant or inactive, preserving route, maintenance, residents, projects, incidents, and resource totals.

Promotion to full detail must reconstruct from conserved state rather than generate a fresh vehicle.

# 20. Network Authority

In multiplayer:

- one authority owns physical vehicle state;
- stations and actions use leases or bounded control tokens;
- residents and cargo have independent identity;
- simultaneous module work resolves through explicit locks and physical conflicts;
- host migration preserves component graph and event history;
- no client predicts authoritative inventory duplication.

# 21. Validation Gates

The runtime passes only if:

- mass and center of gravity are conserved;
- cargo cannot duplicate;
- resident capacity is enforced without objectifying people;
- module dependencies fail legibly;
- damage persists through save/load;
- repairs alter future state;
- multiple loadouts create different route options;
- the player cannot build a universal no-tradeoff configuration;
- crawler operation remains playable under accessibility assistance;
- convoy vehicles retain independent ownership and authority;
- reduced-detail simulation returns to full detail without unexplained state change.

## Runtime Maxim

> **Every kilogram, watt, liter, person, claim, and repair must have somewhere to be and a history explaining why it is there.**
