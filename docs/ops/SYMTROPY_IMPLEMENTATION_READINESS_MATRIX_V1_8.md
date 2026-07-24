---
title: Symtropy Implementation Readiness Matrix
version: 1.8
status: superseded
superseded_by: SYMTROPY_IMPLEMENTATION_READINESS_MATRIX_V1_9.md
scope: evidence-based maturity for core systems, Atlas travel, multi-character worldlines, houses, succession and political memory
owner: production/design/engineering/research
supersedes:
  - SYMTROPY_IMPLEMENTATION_READINESS_MATRIX_V1_7.md
related:
  - GAME_IMPLEMENTATION_ROADMAP_V1_8.md
  - THREE_GENERATION_INTERSTELLAR_SUCCESSION_BENCHMARK_V0_1.md
---

# Symtropy Implementation Readiness Matrix

## Scale

- **D0:** absent
- **D1:** idea
- **D2:** structured concept
- **D3:** implementation-ready design
- **I0:** no verified implementation
- **I1:** isolated prototype
- **I2:** integrated prototype
- **I3:** benchmarked representative implementation
- **I4:** production-ready

# v2.5 Readiness

| Capability | Design | Implementation | Required proof |
|---|---:|---:|---|
| Multi-character authority contract | D3 | I0 | account cannot bypass character boundaries |
| Worldline-qualified character identity | D3 | I0 | branch collision tests |
| Character roster | D3 | I0 | privacy-aware projection |
| Availability state machine | D3 | I0 | safe and unavailable transitions |
| Perspective handoff | D3 | I0 | deterministic transaction replay |
| Inactive character agency | D3 | I0 | explainable off-screen choices |
| Retirement and later return | D3 | I0 | living non-controlled continuity |
| Multiplayer character custody | D3 | I0 | one authoritative controller |
| Observer envelopes | D3 | I0 | distinct perception fixtures |
| Knowledge provenance | D3 | I0 | source, confidence, freshness and privacy |
| Knowledge-time aging | D3 | I0 | stale remote-state UI |
| Anti-metagaming preconditions | D3 | I0 | blocked impossible precision, allowed investigation |
| Character-specific maps | D3 | I0 | no cross-character leakage |
| IRIS scoped exchange | D3 | I0 | consented redacted transfer |
| House entity | D3 | I0 | internal factions and members |
| Biological and adoptive lineage | D3 | I0 | equal claim processing |
| Seed-vessel lineage | D3 | I0 | mission amendment and descendant autonomy |
| Machine lineage and forks | D3 | I0 | separate post-fork persons |
| Guild and apprenticeship | D3 | I0 | labor, education, safety and succession |
| Civic lineage | D3 | I0 | office continuity without ownership |
| Domain-specific inheritance | D3 | I0 | separate property and office claims |
| Estate custody | D3 | I0 | household protection and privacy |
| Interim public authority | D3 | I0 | bounded service continuity |
| Reconstitution claim adjudication | D3 | I0 | personhood without history rewind |
| Hearing and appeal | D3 | I0 | evidence and remedy replay |
| Adult political relationship guardrails | D3 | I0 | consent and exit proof |
| Adoption and care | D3 | I0 | no ownership semantics |
| Ward and hostage distinction | D3 | I0 | material coercion detection |
| Route-house dependency graph | D3 | I0 | power traced to infrastructure and labor |
| Blockade forms | D3 | I0 | civilian and service consequences |
| Peaceful succession | D3 | I0 | engagement and systemic depth |
| Political intrigue anthology | D3 | I0 | content authoring validation |
| Three-generation benchmark | D3 | I0 | complete evidence bundle |
| Delayed political-memory recall | D3 | I0 | one-week participant study |

# Existing Dependencies

| Dependency | Role in v2.5 | Blocker |
|---|---|---|
| Authoritative world state | bodies, objects, offices, regions | deterministic mutation |
| Source-chain runtime | identity and restoration | evidence and uniqueness |
| Chronicle | branch, roster and event presentation | stable ancestry |
| NPC agency | inactive playable characters | explainable long-horizon behavior |
| Households | care and shared access | persistent membership and privacy |
| Professions | guilds and succession | embodied competence |
| Companion runtime | refusal and relationship continuity | autonomous agency |
| Settlement and civic runtime | offices and public services | mandate and budgets |
| Atlas chronology | interstellar epochs | future-directed ordering |
| Atlas route runtime | route-house dependencies | capacity, clocks, cooling and traffic |
| IRIS | bounded assistance and provenance | privacy and scoped memory |
| Multiplayer truth | seat custody and host migration | no duplication |
| Accessibility | roster, time and knowledge legibility | tested alternatives |
| Mature-content safety | relationships, wards and hostages | subject review |

# Promotion Order

```text
character identity
→ handoff
→ inactive agency
→ knowledge provenance
→ household and house
→ succession and claims
→ reconstitution
→ seed generations
→ route politics
→ branches
→ recall study
```

# Evidence Boundary

All listed v2.5 capabilities remain **I0**.

Documents, fixtures, and exact patch reconstruction do not prove runtime behavior.

## Readiness Maxim

> **A political epic is not ready when its family tree renders. It is ready when every person, office, secret, and inheritance survives switching, absence, death, and replay without becoming the player's property.**
