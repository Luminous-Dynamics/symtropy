---
title: Playable History Content Packet Standard
version: 0.1
status: implementation-spec
scope: authoring, review, packaging, budgeting, validation, and release evidence for historical campaign content
owner: narrative/production/content-engineering
related:
  - ../canon/HISTORICAL_CONTENT_AND_PLAYABLE_CAMPAIGN_CONTRACT_V0_1.md
  - ../canon/MISSION_EVENT_AND_CONTRACT_GRAMMAR_V0_1.md
  - ../tech/HISTORICAL_TEXTURE_LORE_PROVENANCE_AND_GENERATION_RUNTIME_V0_1.md
  - ../tech/WORLD_STATE_REVISITABILITY_AND_CONSEQUENCE_PRESENTATION_V0_1.md
---

# Playable History Content Packet Standard

## Purpose

This standard turns a lore idea into a reviewable production unit.

A packet is complete only when narrative, simulation, environment, NPC, audio, accessibility, persistence, test, and cut-scope information travel together.

## Packet Directory

Recommended shape:

```text
campaign_id/
  campaign.yaml
  history_graph.json
  state_graph.json
  cast.yaml
  factions.yaml
  sites.yaml
  activities/
  evidence/
  dialogue/
  audio/
  localization/
  accessibility.yaml
  budgets.yaml
  tests/
  README.md
```

The format may vary in implementation, but the information boundaries are normative.

## Stable Identity

Every packet requires:

```text
campaign_id
content_version
worldline_compatibility_version
historical_root_ids
region_id
primary_site_ids
named_agent_ids
institution_ids
required_system_versions
```

Renaming display text must not change stable identity.

## Campaign Header

Required fields:

- player promise;
- historical premise;
- current pressure;
- ordinary-life promise;
- intended duration;
- recommended entry vectors;
- blocked entry vectors;
- required capabilities;
- content rating and sensitive themes;
- minimum viable slice;
- canonical, optional, and horizon elements.

## Historical Root Graph

The graph must identify:

- originating events;
- surviving physical traces;
- documentary evidence;
- current dependencies;
- institutions and successors;
- affected populations;
- cultural responses;
- open contradictions;
- worldline variation points.

Every active campaign pressure must trace to at least one root.

## Cast Packet

Each named character requires:

```text
identity and origin
current home and work
relationships
personal project
campaign objective
protected values
blind spots
known evidence
incorrect or uncertain beliefs
claims they may make
private information
absence behavior
injury, death, replacement, and reconstitution handling
```

A character whose only role is delivering campaign information fails review.

## Faction and Institution Packet

Each faction requires:

- public purpose;
- real service or protection;
- decision procedure;
- resource base;
- internal blocs;
- obligations;
- capacity limits;
- ordinary members;
- abuses or risks;
- independent initiatives;
- successor and splinter possibilities.

## Site Packet

Each major site requires:

- silhouette and traversal identity;
- material systems;
- ownership and access;
- work and domestic uses;
- historical layers;
- current occupants;
- acoustic profile;
- ordinary and crisis states;
- accessibility paths;
- revisit variants;
- destruction and repair limits;
- evidence objects;
- performance budget.

## Activity Packet

Every activity declares:

```text
trigger
participants
authoritative preconditions
player affordances
NPC initiatives
material inputs
information inputs
possible interruptions
validated outputs
failure outputs
Chronicle candidates
revisit effects
```

Activities may be authored, systemic, or hybrid. Generated language never establishes their authoritative outputs.

## Pressure and Clock Packet

A pressure is a causal accumulation, not a hidden countdown.

Required fields:

- measured variables;
- contributing events;
- decay or recovery;
- thresholds;
- visible indicators;
- affected actors;
- independent actions;
- player influence limits;
- off-screen cadence;
- rollback and replay requirements.

## Evidence Packet

Evidence objects require:

- provenance;
- custody;
- access scope;
- integrity state;
- interpretation options;
- privacy and consent;
- possible damage or loss;
- whether the evidence can be copied;
- worldline ancestry.

## Dialogue Packet

Dialogue content must separate:

```text
semantic intent
permitted claims
belief confidence
relationship context
public or private setting
performance variation
localization notes
voice rights and provenance
```

No dialogue line may silently mutate inventory, law, relationship, campaign state, or historical truth.

## Ordinary-Life Packet

Every campaign requires content for:

- meals;
- rest;
- work rhythm;
- social gathering;
- play, art, or sport;
- care work;
- children, elders, machines, animals, or visitors as appropriate;
- weather or environmental routine;
- one local joke, dispute, or custom not created by the central crisis.

## Revisit Matrix

At minimum:

```text
baseline
pressure escalated
partial repair
institutional victory A
institutional victory B
managed failure
player absence
post-campaign ordinary life
worldline variant
```

Each row identifies physical, social, economic, acoustic, visual, and knowledge-state differences.

## Production Budget

Required budget classes:

- authored words;
- voiced words;
- named agents;
- ambient population variants;
- bespoke animations;
- reusable animations;
- major sites;
- minor sites;
- interactable devices;
- evidence objects;
- music and sound assets;
- cinematics;
- localization strings;
- QA paths;
- save migrations;
- peak simulation cost.

## Cut Order

Every packet defines what is removed first without breaking the causal spine.

A recommended order is:

1. bespoke cinematics;
2. rare dialogue permutations;
3. minor decorative sites;
4. optional side histories;
5. secondary worldline variants;
6. nonessential bespoke animation.

Never cut:

- authoritative state transitions;
- rights and privacy boundaries;
- evidence provenance;
- ordinary-life minimum;
- accessibility paths;
- deterministic fallback;
- persistence and migration tests.

## Review Gates

### Narrative Gate

- historical causality;
- competing values;
- distinct character voices;
- no protagonist exceptionalism;
- no exposition-only characters.

### Systems Gate

- authoritative transitions;
- conservation;
- NPC initiative;
- LOD and off-screen progression;
- worldline uniqueness.

### Production Gate

- budgets;
- cut lines;
- reuse plan;
- localization;
- accessibility;
- test ownership.

### Ethics and Rights Gate

- consent;
- privacy;
- children and dependents;
- disability;
- belief;
- migration;
- labor;
- medical information;
- nonhuman agency.

### Playtest Gate

- comprehension;
- emotional distinctiveness;
- ordinary-life attachment;
- consequence readability;
- replay variation;
- absence behavior.

## Release Evidence

A packet may be called implementation-ready only with:

```text
schema validation
content lint
deterministic replay traces
save/load and migration evidence
performance capture
rights and privacy tests
accessibility review
localization smoke test
playtest report
known limitations
```

## Governing Principle

> **The packet is the smallest unit in which lore, systems, people, production, and proof remain inseparable.**
