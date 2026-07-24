---
title: Companion Implementation Ticket Backlog
version: 0.1
status: implementation-spec
scope: staged implementation tasks for companion autonomy, coordination, continuity, secrets, and benchmark evidence
owner: production/engineering/AI/narrative/QA
related:
  - COMPANION_EMOTIONAL_CONTINUITY_BENCHMARK_V0_1.md
  - ../tech/COMPANION_COORDINATION_DELEGATION_AND_TRUST_RUNTIME_V0_1.md
  - ../tech/COMPANION_HOUSEHOLD_PROJECT_ABSENCE_AND_WORLDLINE_CONTINUITY_RUNTIME_V0_1.md
  - ../tech/SECRET_HISTORY_EASTER_EGG_AND_WORLDLINE_DISCOVERY_RUNTIME_V0_1.md
---

# Companion Implementation Ticket Backlog

## C0 — Stable Identity and Life Anchors

Implement stable companion IDs, household references, work roles, projects, current location, continuity status, and worldline ancestry.

**Acceptance:** save/load and branch migration preserve identity without duplication.

## C1 — Request and Response Envelope

Implement request, counterproposal, defer, evidence request, and refusal semantics.

**Acceptance:** a companion can refuse without becoming hostile or silently executing the request.

## C2 — Domain-Specific Trust

Implement trust domains and negotiated permissions.

**Acceptance:** technical trust changes a repair interaction without granting political or intimate access.

## C3 — Joint Procedure Graph

Prototype one repair and one evacuation procedure with role slots, synchronization points, interruptions, and evidence.

**Acceptance:** both procedures remain physically authoritative and replayable.

## C4 — Practiced Coordination

Record reviewed runs, shorthand, accommodations, challenge points, and anticipation confidence.

**Acceptance:** repeated practice reduces communication load but does not bypass safety or authority.

## C5 — Companion Initiative

Enable bounded initiative for ordinary work, assistance, project activity, rest, and challenge.

**Acceptance:** initiative uses perceived state and controlled resources only.

## C6 — Household and Project Simulation

Implement background work packets, household obligations, project milestones, and availability.

**Acceptance:** six-month absence changes state causally without freezing or arbitrary drama.

## C7 — Departure and Continuity States

Implement leave, withdrawal, missing, death, reconstitution, and fork states.

**Acceptance:** relationships and authority require reconciliation after restoration.

## C8 — IRIS Companion Boundary

Expose observable coordination and permissions while redacting private cognition.

**Acceptance:** privacy red-team cannot recover hidden attraction, fear, medical state, or memories.

## C9 — Secret Content Records

Implement stable secret IDs, predicates, presentation variants, reward policy, and worldline provenance.

**Acceptance:** secrets replay deterministically and do not leak branches.

## C10 — Firstlight Cast Slice

Author Sera, Tomas, Amadi, Morrow-7, Nia, and Mara with homes, schedules, projects, relationships, refusals, and absences.

**Acceptance:** each remains recognizable without traveling with the player.

## C11 — Emotional Recall Instrumentation

Build playtest questionnaire, anonymized event summaries, and recall analysis.

**Acceptance:** instrumentation excludes private cognition and does not optimize emotional dependency.

## C12 — Integrated Benchmark

Run thirty days, three absences, and three branches.

**Acceptance:** all hard gates in the benchmark pass with a reproducible evidence bundle.

# Recommended Prototype Order

```text
C0 → C1 → C3 → C2 → C4 → C6 → C5 → C8 → C7 → C9 → C10 → C11 → C12
```

This order proves authoritative coordination before expanding emotional content.
