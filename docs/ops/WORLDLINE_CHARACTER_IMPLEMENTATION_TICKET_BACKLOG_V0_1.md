---
title: Worldline Character Implementation Ticket Backlog
version: 0.1
status: implementation-spec
scope: implementation sequencing for multi-character play, knowledge envelopes, houses, succession, claims, benchmark and evidence
owner: production/engineering/design/QA
related:
  - THREE_GENERATION_INTERSTELLAR_SUCCESSION_BENCHMARK_V0_1.md
  - ../canon/MULTI_CHARACTER_WORLDLINE_AND_PERSPECTIVE_AUTHORITY_CONTRACT_V0_1.md
  - ../tech/WORLDLINE_CHARACTER_ROSTER_HANDOFF_AND_CONTINUITY_RUNTIME_V0_1.md
  - ../tech/CHARACTER_KNOWLEDGE_OBSERVER_ENVELOPE_AND_ANTI_METAGAMING_RUNTIME_V0_1.md
---

# Worldline Character Implementation Ticket Backlog

## Program Rule

Do not begin with a galaxy-wide dynasty simulator.

Prove three characters, one handoff, one private secret, one office succession, and one preserved branch before expanding the roster.

# Phase A — Identity and Roster

## WC-001 Worldline-Qualified Character IDs

Implement stable character identity bound to worldline ancestry.

**Acceptance:** counterparts in different branches cannot collide.

## WC-002 Character Record Store

Persist embodiment, location, proper time, custody, availability, knowledge reference, households, institutions, and source chain.

## WC-003 Roster Projection

Build privacy-aware roster entries separate from authoritative character records.

## WC-004 Playability Grants

Implement scenario, account, household, succession, and multiplayer access bases.

## WC-005 Availability State Machine

Implement safe, contextual, busy, private, missing, dead, pending, retired, and archived states.

# Phase B — Handoff

## WC-010 Handoff Transaction

Validate source, target, worldline, custody, availability, and uniqueness.

## WC-011 Departure Closure

Persist current work, tools, obligations, risk, and autonomous intention.

## WC-012 Arrival Initialization

Load character-specific body, HUD, map, IRIS, permissions, language, and obligations.

## WC-013 Transition Presentation

Support safe switch, shift handover, message switch, death continuation, deep-time advance, and branch selection.

## WC-014 Handoff Replay

Reproduce switch timing and state exactly.

# Phase C — Inactive Agency

## WC-020 Autonomous Continuation Policy

Record protected obligations, refusal boundaries, risk tolerance, and planned activity.

## WC-021 Multi-LOD Character Simulation

Support full, reduced, event-stepped, and Chronicle presentation levels.

## WC-022 Irreversible Branch Interrupts

Wake or notify the player before protected irreversible decisions where policy requires it.

## WC-023 Retirement and Return

Allow living characters to leave and later re-enter play.

## WC-024 Offline Character Audit

Export why an inactive character made each major choice.

# Phase D — Knowledge

## WC-030 Knowledge Item Store

Implement provenance, confidence, worldline, freshness, privacy, admissibility, and contradiction.

## WC-031 Observer Envelope Builder

Project world state through location, senses, profession, language, institutions, and relationships.

## WC-032 Character-Specific Maps

Separate discovered, received, rumored, predicted, and stale map information.

## WC-033 Anti-Metagaming Preconditions

Require causal knowledge for precision actions without blocking plausible investigation.

## WC-034 “How Do I Know This?” UI

Expose provenance accessibly.

## WC-035 IRIS Data Transfer

Implement scoped, consented, redacted exchange among IRIS instances.

## WC-036 Knowledge-Time Decay

Age remote information and display predicted versus confirmed state.

# Phase E — Houses and Institutions

## WC-040 House Entity

Implement type, membership, assets, obligations, offices, factions, archives, and legitimacy.

## WC-041 Membership Events

Support birth, adoption, partnership, apprenticeship, resignation, expulsion, fork recognition, and ceremonial affiliation.

## WC-042 Shared Asset Access

Implement household, guild, cooperative, corporate, public, and trust accounts.

## WC-043 Internal Factions

Prevent one-house-one-agent simplification.

## WC-044 Founder and Lineage Memory

Track public myth, archive contradiction, and audience-specific prestige.

# Phase F — Succession and Claims

## WC-050 Domain-Specific Claims

Separate identity, property, office, credential, custody, debt, and source-chain claims.

## WC-051 Estate Custody

Protect household continuity, private records, care obligations, and unresolved assets.

## WC-052 Office Tenure

Move authority through mandate and succession rather than inventory transfer.

## WC-053 Interim Authority

Create bounded public-service continuity during disputes.

## WC-054 Reconstitution Claims

Recognize personhood without automatic restoration of office, relationship, or distributed property.

## WC-055 Machine Fork Claims

Support shared ancestry and distinct post-fork persons.

## WC-056 Hearing and Appeal

Implement evidence, findings, interim orders, remedy, privacy, and review.

# Phase G — Relationship Guardrails

## WC-060 Adult Relationship Consent

Bind marriage and partnership to participant consent.

## WC-061 Adoption and Care

Implement care and kinship without ownership.

## WC-062 Apprenticeship Contract

Track education, labor, compensation, safety, assessment, and exit.

## WC-063 Ward Review

Implement advocate, communication, review, and exit conditions.

## WC-064 Hostage Material-State Detection

Detect coercive confinement despite euphemistic labels.

# Phase H — Political Geography

## WC-070 Route House Dependency Graph

Derive influence from cooling, clocks, capacity, manifests, finance, labor, and destination consent.

## WC-071 Capacity and Blockade Policies

Model physical, financial, documentation, technical, labor, and quarantine restrictions.

## WC-072 Diplomatic Message Age

Bind offers and authority to emission and receipt dates.

## WC-073 Worker Refusal

Permit safety, antiwar, wage, and political refusal with service consequences.

## WC-074 Public Institution Counterweights

Add transparent quotas, split authority, public audits, and worker vetoes.

# Phase I — Benchmark Content

## WC-080 Seven-Character Roster

Author Amara, IRI-17/Blue, Del, Hana, Sefu, Mina, and Morrow-17/After.

## WC-081 Muni Seventeen Generational Events

Implement departure, transit refusal, fork, arrival, death, and founding.

## WC-082 Far Station Succession

Implement estate, utility, adopted apprentice, and office claims.

## WC-083 Reconnection Politics

Implement deed, passage, labor, route vote, and privacy conflicts.

## WC-084 Amara Reconstitution

Test personal recognition and separate legal domains.

## WC-085 Three Branches

Generate Public Pairing, House Compact, and Closed Route Crisis.

# Phase J — Research and QA

## WC-090 Delayed Recall Study

Test character, relationship, boundary, and political-memory recall after one week.

## WC-091 Anti-Metagaming Usability

Confirm restrictions feel causal rather than arbitrary.

## WC-092 Inactive Agency Trust

Measure whether players understand and accept off-screen choices.

## WC-093 Peaceful Succession Engagement

Compare engagement and systemic depth against violent branch.

## WC-094 Accessibility Review

Ensure provenance, roster graph, time, and branch presentation are usable.

## WC-095 Safety Review

Review marriage, adoption, children, wardship, hostage, coercion, and privacy content.

# Release Gates

A v2.5 implementation claim requires:

- deterministic replay;
- no duplicate characters or source chains;
- no cross-branch asset transfer;
- no cross-character knowledge leakage;
- correct inactive simulation;
- domain-specific succession;
- privacy review;
- multiplayer custody proof;
- delayed recall evidence;
- performance evidence.

## Backlog Maxim

> **Implement the handoff before the dynasty, the knowledge boundary before the intrigue, and the household before the throne.**
