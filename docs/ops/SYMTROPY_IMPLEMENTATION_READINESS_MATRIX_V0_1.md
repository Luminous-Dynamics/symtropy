---
title: Symtropy Implementation Readiness Matrix
version: 0.1
status: superseded
scope: documentation-based readiness inventory for representative Seedworks and whole-game capabilities
superseded_by:
  - SYMTROPY_IMPLEMENTATION_READINESS_MATRIX_V0_2.md
owner: production/design/engineering
related:
  - ops/DESIGN_TO_CODE_TRACEABILITY_AND_FEATURE_READINESS_STANDARD_V0_1.md
  - ops/GAME_IMPLEMENTATION_ROADMAP_V0_1.md
  - ops/SEEDWORKS_PRODUCTION_BUDGET_AND_CONTENT_PLAN_V0_1.md
---

# Symtropy Implementation Readiness Matrix

## Important Limitation

This matrix assesses **documentation readiness only** unless an evidence bundle is explicitly listed.

The archive does not contain a complete current code audit. Therefore implementation maturity is recorded as `I0 — not assessed` rather than inferred from confident design language or historical roadmap claims.

## Readiness Table

| Capability | Authoritative design | Design | Implementation | Representative proof scenario | Primary next artifact |
|---|---|---:|---:|---|---|
| Embodied movement and tools | Player Feel Bible v0.2 | D3 | I0 | traverse storm route, carry cargo, operate two tools | input/physics prototype evidence |
| Field Deck observation and provenance | Field Deck Bible v0.2 | D3 | I0 | inspect device with uncertainty and contradictory records | mode runtime + comprehension test |
| Device Bus transactions | Device Bus and SymtropyOS specs | D3 | I0 | accepted and rejected local machine write | deterministic transaction harness |
| Physical construction | Cybernetic Crafting doc | D3 | I0 | place, assemble, initialize, and authorize one node | vertical build fixture |
| Resource transformation | Resource Chains doc | D3 | I0 | salvage → process → fabricate → deploy | conservation tests |
| Economy and custody | Economy Integrity Contract + runtime | D3 | I0 | escrowed cargo trade survives disconnect | economic ledger prototype |
| Player progression | Progression and Mastery Contract | D2 | I0 | learn capability through tool, mentor, and infrastructure | progression slice telemetry |
| NPC daily life | NPC Life Bible v0.2 | D3 | I0 | schedules change after route restoration | schedule and relationship prototype |
| NPC cognition runtime | NPC Cognition Runtime Contract | D3 | I0 | competing obligations with grounded explanation | tiered planner harness |
| Combat encounter quality | Combat and Threat design | D3 | I0 | one readable machine encounter with nonlethal outcome | combat graybox evidence |
| Strategic conflict | War/Diplomacy Contract + simulation | D3 | I0 | convoy and bridge alter campaign and ceasefire | campaign summary harness |
| Mission generation | Mission/Event Grammar | D2 | I0 | one source pressure creates three valid activity forms | authored/procedural generator test |
| Settlement metabolism | Settlement/Regional simulation specs | D3 | I0 | power, logistics, NPC routine, and ecology causal chain | minimal causal model |
| Procedural history | Procedural History Engine | D3 | I0 | generated site history changes lock, visuals, repair, and Chronicle | deterministic site generator |
| Faction evolution | Procedural Faction Evolution | D3 | I0 | repeated emergency choices shift posture then identity | faction pressure simulation |
| Science and discovery | Science Contract | D2 | I0 | observation → hypothesis → replicated working model | experiment notebook prototype |
| Civic charters and governance | Social Systems and Charters | D2 | I0 | one scoped rule changes access and survives appeal | charter interpreter |
| Chronicle and durable history | Multiplayer Truth Model + schema | D3 | I0 | important outcome committed; tactical noise omitted | Chronicle backend evidence |
| Real-time co-op | Networking decision + truth model | D3 | I0 | 2–4 players share combat, device, and cargo outcomes | networked vertical slice |
| Multiplayer safety | Social Safety Contract | D3 | I0 | protected infrastructure and recovery after grief attempt | abuse test suite |
| Death and source recovery | Death/Reconstitution design | D3 | I0 | death → limited continuity → recovery path | burden and griefing playtest |
| Vehicles and mobility | Vehicle Bible | D2 | I0 | scout/utility vehicle changes route and cargo capability | handling prototype |
| Ecology and species | Earth Species + simulation | D3 | I0 | species changes water/soil and creates an access conflict | trophic slice |
| Alien translation | Alien contact corpus | D2 | I0 | uncertain signal classified without instant translation | translation-state prototype |
| Player authorship and mods | Authorship/Modding Contract | D2 | I0 | blueprint authored, shared, versioned, and safely loaded | creator tool schema |
| Worldline long horizon | Endgame Contract | D2 | I0 | mature project creates institutional and physical change | strategic prototype |
| Persistence and migration | Worldline Persistence Protocol | D3 | I0 | crash recovery + schema migration + custody reconciliation | persistence harness |
| Delight and everyday life | Delight Bible | D2 | I0 | one social space supports food, music, rest, and expression | ambient-life playtest |
| Accessibility | Player feel, Field Deck, playtest program | D2 | I0 | representative flow completed with alternate input/visual settings | accessibility test plan |

## Highest-Risk Missing Evidence

The following capabilities have high systemic blast radius and should receive evidence before broad content production:

```text
1. persistence and economic custody
2. embodied interaction quality
3. NPC runtime cost and legibility
4. deterministic Device Bus integration
5. regional causal simulation
6. co-op authority and grief recovery
7. content authoring throughput
```

## Representative-Build Critical Path

```text
embodied action
  → local interaction and device transaction
  → one construction / logistics transformation
  → one NPC and settlement consequence
  → save, reload, and visible revisit
  → co-op replication
```

Combat, science, economy, and civic systems should attach to this spine in bounded slices rather than each demanding a separate full game.

## Next Review

Update this matrix only from dated evidence bundles or a fresh code/content audit. Do not promote implementation maturity based on roadmap prose.
