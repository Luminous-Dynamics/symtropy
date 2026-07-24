---
title: Firstlight Action Vertical-Slice Scenario Pack
version: 0.1
status: implementation-spec
scope: bounded action scenarios for movement, combat, rescue, destruction, companion coordination, aftermath and replay
owner: gameplay/level-design/combat/physics/narrative/qa
related:
  - ../canon/FIRSTLIGHT_CORE_ACTION_DANGER_COMBAT_RESCUE_AND_DESTRUCTION_CONTRACT_V0_1.md
  - FIRSTLIGHT_FIRST_10_AND_40_HOUR_EXPERIENCE_MAP_V0_1.md
---

# Firstlight Action Vertical-Slice Scenario Pack

## Purpose

The slice uses four scenarios rather than a broad enemy catalogue.

# Scenario A — The Bent Feeder

**Window:** opening ten minutes.

**Threats:** moving slope, water, live electrical cabinet, hand injury.

**Player verbs:** brace, inspect, drain, isolate, stabilize, carry, communicate.

**Companions:** Sera and Bram.

**Persistence:** temporary repair, injury state, slope monitor, later load test.

**Failure continuation:** secondary trip isolates a wider district; no instant death unless the player deliberately enters declared lethal contact.

# Scenario B — Bridge Seven

**Window:** hours two to three.

**Composition:** a weakened service bridge fails as a cargo crawler and pedestrians occupy it. A damaged utility machine begins enforcing an expired exclusion protocol.

**Player choices:**

- rescue trapped pedestrian;
- stop crawler movement;
- isolate bridge power;
- fight or disable the machine;
- communicate valid authority;
- open lower escape route;
- withdraw and wait for specialist support.

**Combat possibility:** the machine uses industrial restraint and impact tools. One opportunistic scavenger cell may arrive depending on world state.

**Aftermath:** route detour, clinic follow-up, investigation, repair estimate, rumor.

# Scenario C — Flooded Annex

**Window:** hours ten to eighteen, optional.

**Location:** Lower Works and corporate Annex 4C.

**Threats:** rising water, contamination, power, hostile recovery team, unstable partition, trapped maintenance worker.

**Approaches:**

- stealth through service ducts;
- negotiation at controlled entry;
- disable systems;
- direct combat;
- technical rerouting;
- rescue and withdraw without securing evidence.

**Destruction:** selected walls, doors, cable trays, pipe segments, and catwalk supports.

**Consequences:** corporate claim, recovered evidence, worker survival, contamination, route shortcut, repair debt.

# Scenario D — Firstlight Night

**Window:** hours twenty to thirty.

**Situation:** festival and market load coincide with a prepared infrastructure weakness and an opportunistic intervention.

**Concurrent fronts:**

- crowd and transit bottleneck;
- power instability;
- clinic overload;
- Lower Works breach;
- contaminated batch or false alert;
- fire or structural damage;
- evidence custody.

The player selects one primary front and may influence a second. Companions and institutions act on the others.

**Possible climax:**

- defend an evacuation route;
- restart a pump under attack;
- carry an injured person through a failing venue;
- hold a bulkhead while Sera completes isolation;
- secure evidence while Mara refuses to expose private records;
- stop a false evacuation machine without disabling legitimate safety systems.

**Aftermath state:** physically persistent for at least five in-world days and one absence transition.

# Cross-Scenario Requirements

Each scenario must support:

- at least one noncombat solution;
- at least one action-forward solution;
- a meaningful retreat;
- companion initiative;
- profession-specific advantage;
- persistent physical state;
- evidence and record state;
- ordinary-life aftermath;
- deterministic replay from authoritative inputs.

# Replay Variants

Variability may come from:

- weather;
- who is present;
- prior repair quality;
- route availability;
- equipment;
- trust and practiced coordination;
- faction objective;
- public authority;
- information available.

Do not randomize core causality merely to make replay unpredictable.

# QA Evidence

Capture:

- input-to-motion latency;
- interaction completion and abandonment;
- damage causes;
- rescue priority choices;
- companion autonomous actions;
- route and structural state deltas;
- player-reported excitement;
- perceived fairness;
- aftermath comprehension;
- replay desire.

## Scenario Maxim

> **Four excellent situations with persistent consequences are more valuable than forty disposable encounters.**
