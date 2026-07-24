---
title: Atlas Metric Engineering Implementation Ticket Backlog
version: 0.1
status: implementation-spec
scope: ordered engineering, design, content, research and validation work for v2.4 Atlas metric and FTL systems
owner: production/engineering/design/research
related:
  - FIRST_ATLAS_RECONNECTION_AND_GALACTIC_REACH_BENCHMARK_V0_1.md
  - ../tech/ATLAS_TIME_PROPER_TIME_KNOWLEDGE_TIME_AND_CAUSAL_GRAPH_RUNTIME_V0_1.md
  - ../tech/METRIC_TRIM_SAIL_ANCHOR_CORRIDOR_AND_BRIDGE_RUNTIME_V0_1.md
---

# Atlas Metric Engineering Implementation Ticket Backlog

## Delivery Principle

Implement chronology and uniqueness before spectacle.

A beautiful gate with invalid time, duplicated players, or decorative seed history is not progress.

# Foundation

## AT-001 Fixed-Point Atlas Instant

Implement worldline-scoped monotonic event time.

**Proof:** deterministic ordering and serialization.

## AT-002 Causal Event Envelope

Implement parent ancestry, hashes, region, authority, and visibility.

**Proof:** replay of a multi-parent route event.

## AT-003 Proper-Time Integrator

Integrate proper time for representative trajectories.

**Proof:** bounded numerical error and no double aging.

## AT-004 Local Calendar Projection

Project Atlas events into two civic calendars and one ship calendar.

**Proof:** stable round-trip references.

## AT-005 Knowledge Envelope

Separate confirmed observation from prediction.

**Proof:** stale destination UI and delayed update test.

# Causality

## AT-010 Route Edge Store

Implement directed edges with positive latency bounds.

## AT-011 Incremental Cycle Validator

Reject nonpositive route cycles.

## AT-012 Unknown Route Quarantine

Represent alien route uncertainty without integration.

## AT-013 Worldline Branch Ancestry

Persist branch root and unique-asset policy.

## AT-014 Temporal Network Envelope

Validate earliest reception and branch identity.

# Seed Voyage

## AT-020 Seed Vessel Population Fixture

Instantiate 480 residents and 126 households at scalable LOD.

## AT-021 Voyage Temporal Modes

Implement active, accelerated, and deep-time advancement.

## AT-022 Mandatory Interruption Rules

Interrupt for branch events and protected promises.

## AT-023 Mission Amendment Runtime

Support descendant-led mission change.

## AT-024 Suspension and Reconstitution Schedule

Preserve proper, biological, and legal continuity.

## AT-025 Apprenticeship Continuity

Carry one critical profession through two generations.

# Metric Engineering

## AT-030 Metric Measurement Probe

Implement gravimetry, clocks, and route-weather observations.

## AT-031 Metric Trim Prototype

Integrate bounded vehicle acceleration shaping.

## AT-032 Metric Sail Mission

Prove sublight cruise with power, propulsion, braking, and heat.

## AT-033 Anchor Component Graph

Model clocks, arrays, power, cooling, authority, and quarantine.

## AT-034 Metric Debt State

Accumulate and remediate structured operational burden.

## AT-035 Route Weather Runtime

Derive capacity changes from observed conditions.

# Transit

## AT-040 Pairing Stage Machine

Implement staged evidence from field echo through crew certification.

## AT-041 Manifest Freeze and Reservation

Freeze unique objects and destination capacity.

## AT-042 Transit Commit Boundary

Guarantee exactly one ownership state.

## AT-043 Positive-Latency Transit

Advance route-owned manifest and proper time.

## AT-044 Arrival Reconciliation

Activate destination assets and close origin locks.

## AT-045 Crash Recovery Harness

Recover unprepared, prepared, committed, and arrived states.

# Multiplayer

## AT-050 Player Temporal Location Registry

Enforce one authoritative location per avatar.

## AT-051 Forward Epoch Migration

Move one player into a future voyage region.

## AT-052 Asynchronous Region Advancement

Advance origin and destination independently.

## AT-053 Future Join Disclosure

Present time, continuity, and return consequences.

## AT-054 Host Migration During Transit

Preserve snapshots, event logs, and reservations.

## AT-055 Branch Reconnection Refusal

Prevent silent merge of divergent regions.

# Route Emergency

## AT-060 Fault Dependency Graph

Implement causal fault propagation.

## AT-061 Recovery State Machine

Assign one recovery state to a committed transit.

## AT-062 Rescue Capacity and Triage

Conserve rescue mass, beds, crews, and time.

## AT-063 Atlas Scar Persistence

Create and evolve one scar for five years.

## AT-064 Investigation Evidence Bundle

Preserve technical, authority, and worker-action evidence.

# Content and UX

## AT-070 Echo Two Endpoint Slice

Build one worker-centered endpoint district.

## AT-071 Muni Seventeen Voyage Cast

Author minimum benchmark characters and relationships.

## AT-072 Far Station Founding Packet

Integrate prior claims, charter, services, and route debate.

## AT-073 Four-Time UI

Expose Atlas, civic, proper, and knowledge time clearly.

## AT-074 Route Decision UI

Expose capacity, latency, weather, authority, quarantine, and abort.

## AT-075 IRIS Chronology Explanations

Provide bounded explanations without false certainty.

# Research and Gates

## AT-080 Physics-Literacy Study

Test whether players understand time distinctions without prior relativity knowledge.

## AT-081 Galaxy-Scale Perception Study

Test whether FTL preserves distance and wonder.

## AT-082 Emotional Recall Study

Test memory of voyage people, ordinary life, and route consequences.

## AT-083 Multiplayer Irreversibility Study

Test future-migration disclosure and regret prevention.

## AT-084 Accessibility Review

Review visual, auditory, cognitive, and motion effects.

## AT-085 Full Benchmark Evidence Run

Produce every required artifact.

# Ordering

```text
AT-001..005
→ AT-010..014
→ AT-020..025
→ AT-030..035
→ AT-040..045
→ AT-050..055
→ AT-060..064
→ AT-070..075
→ AT-080..085
```

# Cut Order

Preserve first:

1. Atlas Time and knowledge separation;
2. proper time;
3. seed voyage;
4. unique transit commit;
5. region advancement;
6. one route emergency;
7. player-facing chronology.

Cut first:

- many route visual styles;
- large galactic map;
- arbitrary personal FTL;
- dozens of alien route systems;
- cinematic bridge opening;
- procedural route cities;
- military fleet scale.

# Production Maxim

> **Build the clock, the voyage, and the commit boundary before building the ring in the sky.**
