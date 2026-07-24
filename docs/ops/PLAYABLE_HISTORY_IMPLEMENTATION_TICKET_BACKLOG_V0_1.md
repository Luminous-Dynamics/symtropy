---
title: Playable History Implementation Ticket Backlog
version: 0.1
status: implementation-spec
scope: staged engineering and content tickets for the first playable-history regional proof
owner: production/engineering/narrative
related:
  - PLAYABLE_HISTORY_REGIONAL_BENCHMARK_V0_1.md
  - PLAYABLE_HISTORY_CONTENT_PACKET_STANDARD_V0_1.md
  - ../tech/PLAYABLE_HISTORY_CONTENT_COMPILER_AND_WORLDLINE_VARIATION_RUNTIME_V0_1.md
---

# Playable History Implementation Ticket Backlog

## P0 — Data and Determinism

### PH-001 Stable campaign identities

Implement stable IDs, content versions, package hashes, and worldline ancestry.

**Exit:** identical fixtures hash identically; renamed display text does not change identity.

### PH-002 Campaign event journal

Append validated campaign actions and threshold transitions.

**Exit:** replay reconstructs state after crash and save restore.

### PH-003 Pressure state

Implement multidimensional fixed-point pressure and causal contributions.

**Exit:** no direct narrative flag mutation.

### PH-004 Faction initiative envelope

Bind candidate actions to knowledge, resources, authority, routes, and capability.

**Exit:** invalid initiatives fail with inspectable reasons.

## P1 — First Region

### PH-010 Nine Pumps fixture

Author the utility, two communities, cast, ordinary-life schedule, drought pressure, and three successor outcomes.

### PH-011 Service continuity

Model pump capacity, maintenance, parts, labor, water allocation, and failure.

### PH-012 Evidence trail

Create debt ledger, maintenance records, worker testimony, meter history, and one damaged physical trace.

### PH-013 Revisit matrix

Implement baseline, drought, reform, creditor return, managed failure, six-month absence, and fork variants.

## P2 — Discovery and Social Propagation

### PH-020 Historical trace interaction

Observe, access, copy, protect, return, publish, or destroy evidence under permissions.

### PH-021 Claim circulation

Connect evidence publication to rumor, media, reputation, and institutional response.

### PH-022 Private-information tests

Prove that medical, intimate, child, and source-chain data do not enter public campaign state.

## P3 — Ordinary Life and Culture

### PH-030 Shift and household schedules

Named inhabitants continue work, meals, care, and personal projects.

### PH-031 Maintenance festival

Implement preparation, repair tasks, music, food, accessibility, cleanup, and post-event memory.

### PH-032 Player-free participation

NPCs attend, refuse, organize, and adapt the event without player initiation.

## P4 — Compilation and Tools

### PH-040 Packet schema validator

Validate campaign, cast, faction, site, activity, evidence, budget, and revisit files.

### PH-041 Graph visualizer

Show historical roots, pressure contributions, activities, consequences, and worldline variation.

### PH-042 Budget linter

Reject packets above declared content or performance limits unless explicitly waived.

### PH-043 Deterministic minimal compiler

Compile one authored history graph into a validated packet without generative systems.

## P5 — Evidence and Playtest

### PH-050 Replay bundle exporter

Export package hash, event journal, threshold traces, conservation, and migration evidence.

### PH-051 Blind consequence playtest

Test whether players understand causes and revisit differences without designer explanation.

### PH-052 Absence study

Compare active, six-month absent, and five-year absent player runs.

### PH-053 Cut-scope pass

Remove optional assets while preserving ordinary life and causal spine.

## Dependency Order

```text
PH-001 → PH-002 → PH-003 → PH-004
PH-010 → PH-011 → PH-012 → PH-013
PH-020 → PH-021 → PH-022
PH-030 → PH-031 → PH-032
PH-040 → PH-041 → PH-042 → PH-043
all prior tracks → PH-050 → PH-051 → PH-052 → PH-053
```

## Non-Goals

- galaxy-scale procedural campaign generation;
- unrestricted generative dialogue;
- fully voiced variant explosion;
- cinematic branch trees;
- all legendary destinations;
- interstellar historical campaign proof.

The first result is one region that remains coherent under play, absence, replay, and fork.
