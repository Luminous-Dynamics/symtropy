---
title: Firstlight Pacing, Discovery, Cognitive Load, and Return Runtime
version: 0.1
status: implementation-spec
scope: attention management, questless discovery, urgency, player-led pacing, returning-player summaries, session boundaries and opening complexity control
owner: gameplay/ui/ai/narrative/accessibility/research
related:
  - ../canon/PLAYER_LEGIBILITY_COMPLEXITY_AND_COGNITIVE_LOAD_CONTRACT_V0_1.md
  - ../canon/FIRST_40_HOURS_PLAYER_LOVE_AND_RETENTION_CONTRACT_V0_1.md
  - ../ops/FIRSTLIGHT_FIRST_10_AND_40_HOUR_EXPERIENCE_MAP_V0_1.md
  - FIRSTLIGHT_IRIS_COMPANION_AND_NPC_EMOTIONAL_HOOK_RUNTIME_V0_1.md
---

# Firstlight Pacing, Discovery, Cognitive Load, and Return Runtime

## Purpose

Firstlight contains more causal depth than a player can or should process at once.

This runtime controls how opportunities, risks, people, history, and systems become legible without flattening them into a checklist.

> **Complexity belongs in the world. The interface should protect the player's attention rather than demand ownership of every consequence.**

# 1. Attention State

The player-facing system tracks no hidden “engagement” score used to manufacture events.

It may track operational context such as:

- active danger;
- current task;
- recent interruptions;
- cognitive assistance setting;
- unresolved promises;
- nearby opportunities;
- session duration where the player permits wellness prompts;
- whether a tutorial concept has been demonstrated;
- whether the player has requested quiet.

This state changes presentation, not simulation truth.

# 2. Opportunity Classes

## Immediate Hazard

Requires clear presentation because delay changes safety.

## Time-Bounded Obligation

Has an in-world date or window established through an explicit promise, schedule, or physical process.

## Active Project

Can progress, stall, or change without constant alerts.

## Nearby Opportunity

An optional activity discoverable through people, place, sound, or context.

## Background Change

World activity not currently requiring player action.

## Personal Curiosity

Player-marked person, place, question, or practice.

Only immediate hazards may interrupt without consent by default.

# 3. Urgency Rules

Urgency must derive from:

- physical process;
- public schedule;
- known danger;
- person's declared need;
- signed obligation;
- active conflict;
- transport or weather window.

The game may not add urgency merely because:

- the player explored too long;
- session metrics declined;
- a campaign has not advanced;
- a companion has not been visited;
- an optional activity should be showcased.

# 4. Discovery Channels

## Physical Trace

Sound, damage, tracks, altered water, smoke, traffic, missing objects.

## Social Circulation

Conversation, notice wall, rumor, work handover, public record, invitation.

## Routine

Seeing a place or person at a different time.

## Profession Perception

Recognizing a cue through practice.

## IRIS Anomaly

A bounded mismatch, not a revealed answer.

## Map and Transit

Named places, route changes, public schedules, incomplete remote information.

## Deliberate Search

Player asks a person, reviews records, scans, follows a signal, or explores.

No single channel should surface every opportunity.

# 5. Objective Presentation

The opening uses a layered system.

## Current Intention

One player-selected or context-critical intention shown prominently.

## Commitments

Explicit promises, shifts, appointments, hearings, and deadlines.

## Open Questions

Mysteries, hypotheses, and unresolved claims.

## Projects

Longer work with dependencies and current next steps.

## Places and People

Player-pinned memory, not automatically generated task lists.

The UI avoids a long undifferentiated quest log.

# 6. Marker Rules

Markers may identify:

- known public place;
- companion or vehicle whose position is lawfully shared;
- immediate emergency signal;
- player-created pin;
- scheduled meeting point;
- active tool or custody item.

Markers may not identify:

- hidden people;
- private spaces without basis;
- exact evidence not observed;
- hostile actors through walls without sensor support;
- secrets because the content system wants them found.

# 7. IRIS Compression

IRIS can answer:

- “What changed nearby?”
- “What promises are due?”
- “What is urgent?”
- “What do I know about this?”
- “Where did I last see Sera?”
- “Why is this route closed?”
- “What was I doing when I stopped?”

Responses should separate:

- confirmed fact;
- reported claim;
- inference;
- unknown.

# 8. Session Rhythm

## Short Session

The player should be able to:

- perform maintenance;
- visit someone;
- practice;
- make a delivery;
- inspect an outcome;
- attend a small activity;
- stop safely.

## Medium Session

Supports profession case, project stage, social evening, or short danger.

## Long Session

Supports expedition, major event, or hearing plus aftermath.

The game should surface likely safe stopping points after:

- handover;
- rest;
- arrival;
- completed stabilization;
- event phase transition.

# 9. Quiet Mode

The player may request a bounded period with:

- no optional opportunity prompts;
- reduced IRIS speech;
- no companion invitations unless urgent or scheduled;
- ordinary simulation continuing;
- hazards still represented honestly.

Quiet mode is not invulnerability.

# 10. Returning After a Real-World Break

The return flow includes optional layers.

## Ten-Second Orientation

- character;
- location;
- current in-world time;
- immediate safety;
- selected intention.

## One-Minute Summary

- last major action;
- current project;
- named people involved;
- explicit commitments;
- visible world changes;
- known uncertainty.

## Detailed Review

- Chronicle timeline;
- profession notes;
- relationship reminders;
- route changes;
- evidence provenance;
- controls refresher;
- practice access.

No summary may reveal unobserved events as fact.

# 11. Returning After In-World Absence

Distinguish real-world absence from chosen simulation time passage.

For in-world absence:

- simulate affected systems;
- preserve event provenance;
- update people and places;
- show visible changes before exposition;
- present missed public events through records and testimony;
- allow important private events to remain unknown.

# 12. Dynamic Tutorial Support

Tutorial support activates through:

- first use;
- explicit request;
- repeated failure;
- changed tool;
- return after long absence;
- accessibility preference.

It should not:

- seize camera control;
- pause danger without setting permission;
- explain a system already demonstrated unless requested;
- repeat because the player chose a different solution;
- shame the player.

# 13. Cognitive Budget

During ordinary play, the default HUD should foreground no more than:

- immediate body and hazard state;
- held tool state;
- one current intention;
- critical companion communication;
- relevant interaction affordance.

Additional layers are available through Field Deck views.

During danger, reduce narrative and administrative messages.

During analysis, permit dense evidence and system views.

# 14. Pacing Director Boundaries

A pacing director may:

- choose among already valid event opportunities;
- delay nonurgent authored delivery;
- protect quiet recovery windows;
- avoid simultaneous low-priority prompts;
- select which ambient scene is likely to be visible.

It may not:

- alter hidden loyalties;
- harm a companion for drama;
- create scarcity without causal source;
- trigger betrayal because the player is bored;
- change a person's consent;
- erase consequences to restore pace;
- force a major event before prerequisites exist.

# 15. Player-Led Curiosity

Players can mark:

- person;
- place;
- sound;
- object;
- claim;
- route;
- project;
- question.

The Chronicle may then organize relevant observations without converting the curiosity into a promised answer.

# 16. Failure Conditions

The pacing runtime fails if:

- players feel every icon is equally urgent;
- discovery is mostly marker following;
- quiet play is constantly interrupted;
- missed content is treated as failure;
- returning players need external guides;
- IRIS summaries reveal too much;
- active characters know what only the player knows;
- long sessions are required for meaningful progress;
- the game manufactures crises to manage attention.

## Runtime Maxim

> **The player should always have something meaningful to do, but the world should never behave as though everything meaningful must be done by them now.**
