---
title: Firstlight Catastrophe Escalation, Evacuation, and Survivor State Runtime
version: 0.1
status: implementation-spec
scope: catastrophe simulation, warning propagation, threshold crossing, evacuation routes, survivor outcomes, evidence and replay
owner: simulation/gameplay/narrative/networking
related:
  - ../canon/BREAKING_OF_FIRSTLIGHT_LOSS_CONTINUITY_AND_PLAYER_AGENCY_CONTRACT_V0_1.md
  - ../ops/BREAKING_OF_FIRSTLIGHT_120_MINUTE_EXPERIENCE_MAP_V0_1.md
  - EMERGENCY_COORDINATION_EVACUATION_AND_RECOVERY_RUNTIME_V0_1.md
  - KNOWLEDGE_ARCHIVE_AND_HISTORICAL_EVIDENCE_RUNTIME_V0_1.md
  - ../canon/ATLAS_METRIC_ENGINEERING_FTL_AND_CAUSALITY_CONTRACT_V0_1.md
---

# Firstlight Catastrophe Escalation, Evacuation, and Survivor State Runtime

## Purpose

Provide deterministic, inspectable state for the Breaking of Firstlight so the opening catastrophe is causal, replayable, multiplayer-safe, and meaningfully shaped by player and NPC action.

## Runtime Thesis

> **The Breaking is a coupled-system threshold event, not a cinematic timer.**

The campaign guarantees that Firstlight loses its previous stable civic form. The runtime determines how the transition occurs and what persists.

# 1. Authoritative State Domains

The catastrophe runtime owns or consumes authoritative state for:

- weather;
- watershed and terrain;
- power and thermal networks;
- structures;
- transport routes;
- communications;
- medical capacity;
- public warnings;
- institutional authority;
- crowd movement;
- households;
- unique people;
- vehicles;
- cargo;
- archives and evidence;
- machine continuity;
- hidden metric or alien anomaly;
- hostile actors;
- player actions;
- worldline time.

No single `catastrophe_stage` variable may replace these domains.

# 2. Scenario Family

Each worldline selects a validated `breaking_scenario_family`.

Reference families include:

## 2.1 Watershed Resonance

A severe storm overloads drainage and public power while a buried metric structure changes local gradients.

## 2.2 Industrial Field Cascade

A private metric or energy experiment couples to municipal infrastructure and produces cascading clock, power, and structural faults.

## 2.3 Hostile Seizure Under Anomaly

An armed or corporate force attempts to secure the anomaly during a natural emergency, blocking routes and diverting resources.

## 2.4 Null Continuity Event

Corrupted machine or archive processes issue conflicting emergency authority and cause infrastructure to operate against present safety.

## 2.5 Deep-Time Contact

An alien system interprets human infrastructure as a dormant endpoint or signal surface and begins a process humans cannot stop quickly.

Each family defines bounded parameters, not a fixed scene order.

# 3. Escalation Variables

The runtime tracks continuous or ordinal values including:

- environmental load;
- network stress;
- structural margin;
- route throughput;
- public trust;
- warning penetration;
- authority conflict;
- medical load;
- hostile pressure;
- anomaly coherence;
- clock divergence;
- evacuation demand;
- vehicle readiness;
- archive custody integrity;
- communications reach;
- responder fatigue.

Variables are linked through explicit causal edges.

Example:

```text
storm intensity
  → drainage load
  → substation flooding
  → power instability
  → pump loss
  → district flooding
  → route closure
  → evacuation congestion
```

The anomaly may add new edges rather than simply increasing a damage scalar.

# 4. Warning Events

Warnings are observations with provenance.

A warning contains:

- phenomenon;
- observer;
- emission time;
- confidence;
- measurement method;
- predicted consequence;
- affected area;
- authority status;
- communication channels;
- distortion or suppression history.

Residents react according to received warnings, trust, practice, obligations, and current state.

No NPC may act on global simulation truth they did not observe.

# 5. Preparation Effects

Before threshold crossing, actions can alter:

- route capacity;
- vehicle readiness;
- supply placement;
- household awareness;
- medical staging;
- power isolation;
- archive duplication;
- drainage;
- public trust;
- responder fatigue;
- hostile opportunity;
- anomaly evidence.

Preparation does not prevent the campaign premise. It changes the response envelope.

Examples:

- repairing crawler brakes permits a steeper route;
- moving medicine creates a functioning mobile triage point;
- warning one household changes where several people begin the evacuation;
- isolating a feeder prevents one district fire but removes power from a shelter;
- copying evidence preserves truth but consumes time and storage;
- following the rival reveals a private escape route but creates dependency.

# 6. Threshold Crossing

The Breaking occurs when the active scenario reaches a validated irreversibility condition.

Reference conditions include:

- two or more essential civic metabolisms become mutually unrecoverable within available repair time;
- anomaly coherence exceeds safe shutdown capacity;
- route graph loses sufficient throughput for full population support;
- contamination or structural risk makes long-term occupation impossible;
- authority conflict prevents coordinated restoration before cascading failure;
- a persistent metric scar forms.

The condition must be inspectable in replay evidence.

The runtime writes a `breaking_threshold_event` containing:

- causal parents;
- observations available before crossing;
- responsible systems and institutions;
- affected regions;
- first irreversible consequences;
- uncertainty.

# 7. Region Decomposition

Firstlight is divided into operational cells rather than one destruction mesh.

Each cell tracks:

- population;
- structures;
- utilities;
- routes;
- hazards;
- local authority;
- response capacity;
- evacuation demand;
- survivor groups;
- evidence;
- later accessibility.

A cell may become:

- stable refuge;
- temporary refuge;
- evacuation corridor;
- isolated;
- hazardous;
- collapsed;
- flooded;
- occupied;
- quarantined;
- metric-unstable;
- unknown.

# 8. Route Graph

Evacuation uses an authoritative directed multigraph.

Nodes represent:

- neighborhoods;
- shelters;
- transfer points;
- vehicle yards;
- bridges;
- tunnels;
- wilderness exits;
- private routes;
- medical facilities;
- convoy destinations.

Edges contain:

- physical capacity;
- travel time;
- accessibility profile;
- vehicle limits;
- hazard;
- authority requirements;
- current congestion;
- visibility;
- evidence confidence;
- failure threshold.

Routes may change while occupants are moving.

# 9. Evacuation Groups

People normally move as groups shaped by:

- household;
- care dependency;
- work team;
- neighborhood;
- vehicle access;
- trust;
- institutional assignment;
- emergent mutual aid.

The runtime may split groups only through a recorded event:

- route failure;
- medical handover;
- deliberate choice;
- arrest or seizure;
- vehicle capacity;
- lost communication;
- rescue transfer;
- death;
- refusal.

# 10. Person Decision Cycle

At bounded decision points, a person evaluates:

1. immediate hazard;
2. dependents and commitments;
3. received warnings;
4. known routes;
5. available transport;
6. professional duty;
7. trust in authorities and player;
8. personal attachment;
9. physical capacity;
10. alternatives.

They may:

- evacuate;
- prepare;
- help another group;
- remain;
- refuse;
- seek evidence;
- join the player;
- take another route;
- challenge an order;
- conceal their plan.

The player does not own their decision.

# 11. Rescue and Triage

Rescue candidates are prioritized by declared procedures and local judgment, not protagonist status.

State includes:

- survivability;
- time sensitivity;
- access difficulty;
- responder risk;
- required equipment;
- transport needs;
- consent or known wishes;
- number of people affected;
- alternative responders.

The system must support imperfect but defensible choices.

# 12. Survivor Ledger

Every observed or high-salience person receives one authoritative outcome state.

Required fields:

- person ID;
- last confirmed location;
- last confirmed time;
- current status;
- evidence level;
- route or group;
- injuries;
- continuity state;
- possessions or evidence carried;
- witness claims;
- later update hooks.

Statuses include:

- confirmed_safe;
- departed_other_route;
- remaining_by_choice;
- unable_to_evacuate;
- missing;
- confirmed_dead;
- recovery_possible;
- forked_claim;
- unknown_unobserved.

`missing` may not silently age into `confirmed_dead`.

# 13. Cargo and Continuity Loading

The Continuance Crawler and other vehicles expose actual capacity.

Cargo objects include:

- mass;
- volume;
- shape;
- restraint requirements;
- power;
- cooling;
- hazard;
- ownership;
- custody;
- access need;
- cultural significance;
- replaceability;
- dependencies.

People are not cargo objects. Their transport requirements interact with capacity through seats, bunks, stretchers, privacy, life support, adaptations, and consent.

# 14. Mobile-Base Readiness

Crawler readiness tracks:

- propulsion;
- energy;
- brakes;
- steering;
- tires, tracks, legs, or suspension;
- structural integrity;
- water;
- sanitation;
- medical berth;
- workshop;
- communications;
- route clearance;
- crew competence;
- cargo balance.

A player who ignored the crawler may still escape through a degraded configuration, another convoy, or rival route. The campaign must not hard-lock because one optional preparation was missed.

# 15. Hostile and Institutional Actors

Actors pursue concrete objectives such as:

- securing anomaly data;
- protecting one district;
- preserving corporate assets;
- controlling evacuation records;
- seizing a vehicle;
- preventing public panic;
- opening or closing a route;
- extracting a person;
- destroying evidence.

They operate under the same observation and route constraints as other entities.

# 16. Persistence After Departure

Firstlight continues at a lower simulation level.

The runtime advances:

- remaining residents;
- fire, flood, contamination, or metric state;
- route closures;
- occupation;
- rescue attempts;
- broadcasts;
- evidence custody;
- machine operations;
- ecological transformation;
- later settlements.

The player may receive delayed or unverified updates.

# 17. Multiplayer Determinism

The authoritative host or server owns catastrophe state.

Clients submit actions with timestamps and receive event confirmations.

Critical events require:

- stable event IDs;
- causal parents;
- worldline ID;
- authoritative time;
- affected entities;
- deterministic resolution inputs;
- replay hashes.

Host migration must preserve the catastrophe ledger and survivor state.

# 18. Failure Handling

The opening may continue when:

- the player is injured;
- the preferred route closes;
- the crawler is damaged;
- a companion refuses;
- evidence is lost;
- medical supplies are absent;
- combat is lost;
- the player is separated from the main group.

Fallback paths must remain causal and costly.

The system may end the current character through death, but reconstitution and worldline continuity rules remain authoritative.

# 19. Telemetry and Evidence

Development builds record:

- warning exposure;
- preparation actions;
- threshold cause graph;
- routes opened and closed;
- groups split;
- rescues attempted;
- survivor states;
- cargo loaded;
- crawler readiness;
- time spent;
- player confusion markers;
- accessibility mode;
- deterministic replay checksum.

Telemetry must not include private generative NPC cognition beyond declared research consent and minimized summaries.

# 20. Validation Gates

The runtime passes only if:

- fixed seeds replay identically;
- different actions create different survivor and continuity outcomes;
- no person teleports between routes;
- vehicle capacity is conserved;
- route throughput is conserved;
- warnings respect knowledge propagation;
- missing and dead remain distinct;
- Firstlight continues after player departure;
- multiplayer clients agree on outcomes;
- save/load preserves event ancestry;
- at least three scenario families remain completable through multiple practice combinations;
- no single rescue or cargo solution dominates all test worldlines.

## Runtime Maxim

> **A catastrophe becomes history when every survivor, route, object, failure, and disputed record can answer how it got there.**
