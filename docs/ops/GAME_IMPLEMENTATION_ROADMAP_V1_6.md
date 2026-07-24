---
title: Symtropy Game Implementation Roadmap
version: 1.6
status: superseded
superseded_by: GAME_IMPLEMENTATION_ROADMAP_V1_7.md
scope: staged implementation gates from representative region through player-founded settlement, succession, absence, and living legacy
owner: production/design/engineering/research
supersedes:
  - GAME_IMPLEMENTATION_ROADMAP_V1_5.md
related:
  - ../canon/CANON_REGISTRY_AND_DOCUMENT_GOVERNANCE_V1_9.md
  - PLAYER_FOUNDED_CIVILIZATION_IMPLEMENTATION_TICKET_BACKLOG_V0_1.md
  - TWENTY_YEAR_PLAYER_FOUNDED_REGION_BENCHMARK_V0_1.md
---

# Symtropy Game Implementation Roadmap

## Strategy

Do not build universal civilization generation first.

Build one South Cut founding proof on top of the existing Firstlight region, profession, companion, IRIS, infrastructure, and history contracts.

# Gate 0 — Dependency Truth

Required:

- authoritative resource flows;
- households and NPC identity;
- professions and shifts;
- companion requests and refusal;
- IRIS evidence boundaries;
- Chronicle persistence;
- multiplayer authority envelope.

**Exit:** dependency tests are green or explicitly mocked without making implementation claims.

# Gate 1 — People Before Buildings

Implement twelve named residents, four households, care obligations, work, intended homes, and reasons to stay or leave.

**Exit:** residents exist and act before permanent construction.

# Gate 2 — Site and Prior Claims

Implement wetland boundary, old utility easement, displaced-household claim, corporate title, and uncertainty.

**Exit:** scanning does not collapse conflicts into ownership.

# Gate 3 — Provisional Compact

Implement three compact paths with rights floor, emergency authority, review date, exit, and public record.

**Exit:** emergency authority expires correctly.

# Gate 4 — Seven Public Services

Implement water, clinic, sanitation, workshop, food, transport, and records.

**Exit:** services consume labor and resources, expose capacity, and accumulate maintenance debt.

# Gate 5 — Charter and Institutions

Implement clause data, ratification, three offices, four institutional forms, budget, challenge, and amendment ancestry.

**Exit:** no construction or host shortcut can issue civic authority.

# Gate 6 — Promise, Office, and Attribution

Implement commitments, term handover, three constituency reputation views, and plural project credit.

**Exit:** former office-holder cannot act; public history may disagree with work records.

# Gate 7 — Emergent Campaigns

Implement causal clustering, storylet proposals, privacy gates, NPC initiation, and suppression.

**Exit:** two campaigns progress without the player; one private case remains private.

# Gate 8 — Place and Culture

Implement provenance, repairs, local names, one landmark, one ritual or work practice, adoption, mutation, and rejection.

**Exit:** player designation alone creates neither landmark nor culture.

# Gate 9 — Five-Year Absence

Run bounded simulation for services, households, offices, projects, culture, ecology, and external relations.

**Exit:** the return packet explains change through causality, not arbitrary time skips.

# Gate 10 — Death, Reconstitution, and Successor

End the player's office through death or legal absence, run succession, and return a verified reconstituted player.

**Exit:** identity continuity does not restore expired authority.

# Gate 11 — Twenty-Year Worldlines

Generate durable commons, technical republic, and mobile successor branches.

**Exit:** branches preserve common ancestry and distinct authority.

# Gate 12 — Multiplayer Founding

Two players found, disagree, hold different roles, disconnect, migrate host, return, and fork.

**Exit:** host state never becomes civic state.

# Gate 13 — Research Proof

Run delayed recall, founder-ownership, outgrown-founder, accessibility, safety, and performance studies.

**Exit:** players remember people, services, promises, and changes; successors feel independently legitimate.

# Gate 14 — Expansion Decision

Only after proof decide whether to expand to:

- additional settlement archetypes;
- player-led mobile civilizations;
- orbital habitats;
- machine-founded settlements;
- mixed human–alien polities;
- planetary charter networks;
- large-scale procedural founding.

# Cut Order

Preserve first:

1. residents and households;
2. prior claims;
3. services;
4. compact;
5. charter and succession;
6. five-year absence;
7. return;
8. multiplayer authority.

Cut first:

- large architecture catalog;
- decorative government variants;
- cinematic founder ceremonies;
- broad procedural culture generation;
- hundreds of residents;
- planetary-scale UI.

# Production Maxim

> **Prove that one settlement can live without its founder before promising that players can build civilizations across the stars.**
