---
title: Symtropy Game Implementation Roadmap
version: 1.8
status: superseded
superseded_by: GAME_IMPLEMENTATION_ROADMAP_V1_9.md
scope: staged implementation from existing regional and Atlas proofs through multi-character handoff, observer knowledge, houses, succession and three-generation political memory
owner: production/design/engineering/research
supersedes:
  - GAME_IMPLEMENTATION_ROADMAP_V1_7.md
related:
  - ../canon/CANON_REGISTRY_AND_DOCUMENT_GOVERNANCE_V2_1.md
  - WORLDLINE_CHARACTER_IMPLEMENTATION_TICKET_BACKLOG_V0_1.md
  - THREE_GENERATION_INTERSTELLAR_SUCCESSION_BENCHMARK_V0_1.md
---

# Symtropy Game Implementation Roadmap

## Strategy

Do not build a galaxy-wide dynasty generator first.

Add one alternate playable perspective to the existing Firstlight proof, then one seed-voyage generation, one succession, and one branch. Generalize only after character separation and knowledge provenance survive replay.

# Gate A — Existing Embodied Character

Required:

- authoritative body and inventory;
- source chain;
- profession;
- household or companion relationships;
- IRIS permissions;
- current obligations;
- Chronicle events.

**Exit:** one character can be saved, resumed, killed, and restored without ambiguity.

# Gate B — Second Same-Worldline Character

Add a second playable adult with:

- different profession;
- separate inventory;
- separate map;
- separate IRIS or no IRIS;
- independent schedule;
- relationship not centered on the first character.

**Exit:** switching cannot transfer assets, authority, or knowledge.

# Gate C — Authoritative Handoff

Implement:

- availability;
- departure closure;
- autonomous intention;
- target initialization;
- elapsed time;
- replay.

**Exit:** source character continues acting.

# Gate D — Observer Knowledge

Add:

- one private observation;
- one rumor;
- one public record;
- one stale remote report;
- one IRIS inference;
- “How do I know this?” UI.

**Exit:** impossible precision actions are blocked while plausible investigation remains available.

# Gate E — Household and Shared Assets

Implement one household account and one public or cooperative account.

**Exit:** access derives from membership and role, not account ownership.

# Gate F — One House With Internal Difference

Implement one seed, machine, professional, or civic lineage containing:

- four members;
- two internal factions;
- one shared obligation;
- one person uninterested in house politics;
- one public reputation.

**Exit:** house hostility does not make every member hostile.

# Gate G — One Succession

Implement:

- death or retirement;
- private estate;
- office vacancy;
- interim authority;
- two claimants;
- hearing;
- peaceful remedy;
- appeal.

**Exit:** private property and public office resolve independently.

# Gate H — Reconstitution Claim

Restore the former office-holder.

**Exit:** personhood can be accepted while office restoration is rejected through current law.

# Gate I — Seed-Generation Expansion

Add reduced Muni Seventeen sequence:

- launch character;
- transit-born character;
- machine fork;
- arrival character;
- inactive progression;
- mission amendments.

**Exit:** three generations remain distinct.

# Gate J — Route-House Politics

Add Echo Two and Far Station dependencies:

- cooling cooperative;
- clock assembly;
- passage board;
- logistics house;
- route labor.

**Exit:** political influence can be traced to actual systems.

# Gate K — Divergent Branch

Preserve one pre-succession branch.

**Exit:** no inventory, knowledge, character, source-chain, or office transfer across worldlines.

# Gate L — Three-Generation Benchmark

Run the full v2.5 benchmark with:

- seven characters;
- 118 years;
- five handoffs;
- one reconstitution;
- one peaceful succession;
- one violent branch;
- three worldlines;
- delayed recall.

**Exit:** evidence bundle complete.

# Gate M — Scale Expansion

Only after Gate L:

- add more houses;
- add alien playable perspectives;
- add wider route politics;
- add authored political campaigns;
- support longer Chronicle rosters.

# Implementation Order

```text
single character truth
→ second perspective
→ handoff
→ observer knowledge
→ shared institutions
→ house
→ succession
→ reconstitution
→ seed generations
→ route politics
→ worldline branch
→ benchmark
```

# Explicit Non-Goals

Before benchmark promotion, do not claim:

- arbitrary NPC possession;
- universal character roster;
- galaxy-wide dynasty simulation;
- alternate-worldline travel;
- implemented hostage politics;
- production-safe child playability;
- implemented alien kinship;
- production-ready political generation.

## Roadmap Maxim

> **First prove that two people remain different when the player switches between them. Everything larger depends on that truth.**
