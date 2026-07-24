---
title: Player-Founded Civilization Implementation Ticket Backlog
version: 0.1
status: implementation-spec
scope: implementation sequence, acceptance criteria, evidence, dependencies, risks for v2.3 systems
owner: production/design/engineering/research
related:
  - TWENTY_YEAR_PLAYER_FOUNDED_REGION_BENCHMARK_V0_1.md
  - PLAYER_FOUNDED_SETTLEMENT_CAMPAIGN_PACKET_STANDARD_V0_1.md
  - ../tech/SETTLEMENT_FOUNDING_CHARTER_INSTITUTION_AND_PUBLIC_SERVICE_RUNTIME_V0_1.md
  - ../tech/EMERGENT_CAMPAIGN_DETECTION_CAUSAL_STORYLET_AND_PLAYER_HISTORY_RUNTIME_V0_1.md
  - ../tech/PLAYER_PROMISE_OFFICE_REPUTATION_AND_LEGACY_RUNTIME_V0_1.md
---

# Player-Founded Civilization Implementation Ticket Backlog

## Program Rule

Do not implement a general city generator first.

Implement one bounded South Cut region with twelve residents, real services, three compact paths, one charter, one succession, one long absence, and one worldline fork.

# Milestone A — Founding Truth

## PF-001 Prior-Claim Registry

Implement claim records for residency, watershed dependency, seasonal use, public easement, and corporate title.

**Acceptance:** conflicting claims remain visible; no scan silently selects a winner.

## PF-002 Founding Cohort State

Represent twelve named residents and four households before permanent buildings exist.

**Acceptance:** households retain location, care, work, and exit intentions.

## PF-003 Provisional Compact

Implement three compact variants and public review date.

**Acceptance:** emergency authority expires and cannot renew without a recorded action.

## PF-004 Rights Floor

Implement bodily autonomy, survival access, due process, practical exit, and privacy as explicit rules.

**Acceptance:** any violating action produces authoritative refusal or harm state, not only flavor text.

# Milestone B — Services and Institutions

## PF-005 Public-Service Entity

Implement water, clinic, sanitation, workshop, food, transport, and records services.

**Acceptance:** each conserves labor, resources, capacity, and maintenance debt.

## PF-006 Institution Entity

Implement assembly, cooperative, department, and household-compact forms.

**Acceptance:** same service can operate under at least two forms with different authority and costs.

## PF-007 Office and Authority Token

Implement three offices with terms, duties, conflicts, recall, and succession.

**Acceptance:** former holder cannot use expired token.

## PF-008 Budget Cycle

Implement one annual budget with taxes, fees, labor contribution, subsidy, and maintenance deferral.

**Acceptance:** UI exposes distributional consequences.

# Milestone C — Charter

## PF-009 Charter Schema and Parser

Load clauses, rights floor, amendment, emergency, and dissolution rules from validated data.

## PF-010 Ratification Runtime

Support resident vote, household assembly, and worker-cooperative ratification.

**Acceptance:** participation and exclusion evidence is stored.

## PF-011 Amendment Ancestry

Preserve version chain and branch conditions.

**Acceptance:** every amendment can be replayed from parent charter and authoritative vote.

## PF-012 Constitutional Challenge

Implement one case contesting emergency authority or access.

# Milestone D — Commitments and Legacy

## PF-013 Commitment Ledger

Implement personal promise, contract, office duty, maintenance guarantee, and care obligation.

## PF-014 Constituency Reputation

Implement three separate reputation views without a global score.

## PF-015 Attribution Claims

Allow workers, public records, and media to credit the same project differently.

## PF-016 Founder Legacy State

Persist material, institutional, legal, cultural, and harmful traces.

# Milestone E — Emergent Campaigns

## PF-017 Causal Cluster Detector

Cluster events by dependency, people, place, obligation, and institution.

## PF-018 Storylet Candidate Compiler

Produce bounded opportunities with initiating NPC, privacy gate, and possible player roles.

## PF-019 NPC Initiative

Allow two campaigns to resolve or progress without the player.

## PF-020 Anti-Drama Suppression

Add repetition, privacy, arbitrary-targeting, and low-evidence suppressions.

# Milestone F — Place and Culture

## PF-021 Place Provenance

Track design, labor, material, maintenance, use, and renaming.

## PF-022 Landmark Emergence

Compute constituency recognition from use and history, not player designation.

## PF-023 Cultural Artifact

Support name, signal, ritual, festival, song, flag, and work practice.

## PF-024 Adoption and Mutation

Implement awareness, use, identity, variants, opposition, and commercialization.

# Milestone G — Absence and Return

## PF-025 Absence Summary

Simulate 30-day, two-year, five-year, and twenty-year horizons using bounded LOD.

## PF-026 Return Packet

Generate changed people, places, services, permissions, promises, and ordinary scene.

## PF-027 Reconstitution and Office Separation

Verify identity without restoring expired authority.

## PF-028 Successor Government

Implement one successor that improves a service while rejecting founder control.

# Milestone H — Worldline and Multiplayer

## PF-029 Worldline Fork

Fork charter, population, obligations, and authority ancestry.

## PF-030 Multiplayer Founding

Allow two players to hold different roles and disagree.

## PF-031 Host Migration

Prove civic authority survives host migration without reassignment.

## PF-032 Replay Bundle

Export Chronicle events, state hashes, and branch ancestry.

# Milestone I — Research and Production

## PF-033 Delayed Recall Study

Test resident, place, service, promise, and charter recall after seven days.

## PF-034 Founder Ownership Study

Measure whether players describe the settlement as "mine" in a property sense or "ours"/"theirs" in a civic sense.

## PF-035 Outgrown Founder Study

Test whether players can experience legitimate successor leadership as emotionally satisfying.

## PF-036 Accessibility Review

Review charter meetings, construction, records, return summaries, and cultural interactions.

## PF-037 Performance Gate

Profile 12, 50, 200, and 1,000 resident LOD modes.

## PF-038 Safety and Exploitation Review

Audit forced labor, settlement displacement, coercive authority, reproductive politics, and colonial framing.

# Cut Order

If scope must shrink, preserve:

1. prior claims;
2. households;
3. services;
4. compact and charter;
5. office succession;
6. five-year absence;
7. return packet;
8. one cultural mutation;
9. one emergent campaign;
10. one worldline fork.

Cut first:

- large building catalogs;
- decorative monuments;
- dozens of government forms;
- broad procedural naming;
- full planetary settlement generation;
- prestige cinematics.

# Promotion Gate

No feature moves above **D3 / I0** until its ticket has:

- code;
- automated tests;
- deterministic replay evidence;
- multiplayer authority evidence where relevant;
- performance evidence;
- accessibility review;
- player-facing validation.
