---
title: Playable History Content Compiler and Worldline Variation Runtime
version: 0.1
status: implementation-spec
scope: compiling causal lore graphs into validated campaign packets, activities, evidence, variants, and deterministic content identities
owner: content-engineering/simulation/narrative
related:
  - HISTORICAL_TEXTURE_LORE_PROVENANCE_AND_GENERATION_RUNTIME_V0_1.md
  - HISTORICAL_PRESSURE_FACTION_CLOCK_AND_CAMPAIGN_STATE_RUNTIME_V0_1.md
  - ../ops/PLAYABLE_HISTORY_CONTENT_PACKET_STANDARD_V0_1.md
  - ../ops/PLAYABLE_HISTORY_REGIONAL_BENCHMARK_V0_1.md
---

# Playable History Content Compiler and Worldline Variation Runtime

## Purpose

The historical-texture generator creates causal entities and relationships. The playable-history compiler turns a bounded subset of that graph into a content package that can be authored, simulated, validated, migrated, and tested.

> **The compiler may propose play. It may not invent authority, erase causality, or guarantee drama by violating the world.**

## Inputs

```text
world seed and worldline ancestry
region and time window
historical entity graph
physical and ecological state
institutions and faction state
named inhabitants and cohorts
available sites and routes
current pressures
content libraries
system capability profile
performance and production budgets
accessibility and localization profile
```

## Compilation Stages

### 1. Scope Selection

Select a bounded region, historical root, time window, and player-experience promise.

Reject scopes that require more systems, named characters, or sites than the declared budget.

### 2. Causal Subgraph Extraction

Extract:

- root events;
- surviving traces;
- dependencies;
- acting institutions;
- affected people;
- open contradictions;
- cultural responses;
- plausible successors.

Every extracted entity retains stable source identity.

### 3. Pressure Derivation

Translate present dependencies into typed pressures. Do not derive conflict merely because two factions exist.

### 4. Actor Binding

Bind objectives and roles to existing agents or institutions. Create new authored agents only when the role cannot be carried by existing inhabitants and the named-character budget permits it.

### 5. Site Binding

Activities bind to physical places with valid access, services, ownership, and traversal.

### 6. Activity Synthesis

Activities are selected or composed from validated grammars:

```text
inspect
repair
transport
care
research
negotiate
witness
publish
construct
evacuate
organize
ritualize
defend
refuse
```

The compiler emits activity candidates, not authoritative completion.

### 7. Ordinary-Life Weave

At least one work, care, meal, play, or cultural activity is attached to each major actor group.

### 8. Resolution Envelope

Generate reachable outcome families based on actual capability. The compiler must not expose a public-takeover ending if no institution can staff or supply it.

### 9. Revisit Matrix

Compile physical, social, acoustic, economic, and knowledge-state presentation tags for major outcomes and absence windows.

### 10. Validation and Packaging

Run schema, graph, rights, performance, localization, persistence, and content-quality validators.

## Stable Content Identity

```rust
struct CompiledCampaignIdentity {
    campaign_id: CampaignId,
    source_graph_hash: Hash,
    compiler_version: Version,
    content_library_versions: Vec<VersionRef>,
    worldline_ancestry: WorldlineAncestry,
    variant_seed: Seed,
    schema_version: Version,
}
```

A different compiler version may produce a new package identity even from the same history.

## Variant Classes

### Surface Variant

Changes names, schedules, weather, minor participants, or presentation without changing causal structure.

### Structural Variant

Changes an institution, dependency, available route, or evidence survival while preserving the same historical root.

### Worldline Variant

Changes the root event or major consequence graph and therefore produces a distinct ancestry.

The compiler must label the class explicitly.

## Worldline Rules

- Shared pre-fork entities retain shared ancestry.
- Post-fork mutations remain branch-local.
- Unique assets may not appear in both branches unless the fork semantics explicitly copy informational state rather than physical state.
- A player-imported authored campaign binds to one worldline package identity.
- Recompilation after schema migration preserves prior campaign events.

## Content Libraries

Libraries may contain:

- activity templates;
- dialogue intents;
- evidence-object patterns;
- site roles;
- ordinary-life behaviors;
- cultural-season components;
- pressure thresholds;
- revisit presentation rules.

Libraries require licensing, provenance, cultural review, localization notes, and compatibility versions.

## Validation Passes

### Causal Graph

- no orphan pressure;
- no activity without preconditions;
- no outcome without capacity;
- no successor without transfer path.

### Authority

- dialogue cannot establish state;
- faction actions use valid mandates;
- private evidence stays scoped;
- consent and rights remain authoritative.

### Material and Spatial

- cargo, labor, power, time, and route requirements exist;
- sites are reachable;
- services persist through transition.

### Character

- no exposition-only named agents;
- independent projects;
- internal faction disagreement;
- absence behavior.

### Historical Texture

- physical trace;
- documentary or living memory;
- cultural response;
- contested interpretation where appropriate.

### Production

- declared budgets;
- cut order;
- reusable asset mapping;
- localization and accessibility coverage;
- performance limits.

## Fallback

If dynamic compilation fails, the runtime may use:

1. last validated compiled packet;
2. deterministic authored minimal packet;
3. region-level pressure summary without new activities.

It may not fabricate state or silently delete the campaign.

## Modding and Authorship

Creator packages must declare:

- source history;
- new stable IDs;
- required systems;
- protected content;
- licensing;
- worldline compatibility;
- migration policy;
- determinism class;
- whether generative rendering is used.

Untrusted packages run through the same validators and authority boundaries.

## Acceptance Tests

1. Identical inputs produce identical package hashes.
2. A removed route removes dependent activity candidates.
3. A destroyed institution prevents unsupported resolutions.
4. Surface variants preserve causal identity.
5. Structural variants declare changed consequence edges.
6. Worldline variants preserve ancestry and uniqueness.
7. Invalid private evidence cannot enter public dialogue.
8. Compilation failure retains last valid state.
9. Save migration preserves completed and failed activity history.
10. A minimal authored packet runs without generative systems.

## Governing Principle

> **Procedural history becomes playable only after compilation proves that people, places, systems, evidence, and consequences still agree.**
