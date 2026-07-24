---
title: Symtropy v0.7 Integration Patch Campaign
version: 0.7
status: implementation-spec
scope: patch grouping, prototype gates, adoption order, deferrals
owner: documentation/design/engineering
---

# Symtropy v0.7 Integration Patch Campaign

## Campaign Goal

Close the largest remaining gaps between the civilizational vision and a durable playable platform.

This campaign deliberately avoids adding new factions, planets, species, or vehicle catalogs.

## Patch Set A — Playable Activity

Adds:

```text
canon/MISSION_EVENT_AND_CONTRACT_GRAMMAR_V0_1.md
tech/WORLD_STATE_REVISITABILITY_AND_CONSEQUENCE_PRESENTATION_V0_1.md
```

Prototype gate:

```text
one simulation pressure creates an opportunity
one activity supports two methods
one failure continues rather than resets
one delayed revisit makes the outcome visible
```

## Patch Set B — Knowledge and Discovery

Adds:

```text
canon/SCIENCE_RESEARCH_AND_DISCOVERY_CONTRACT_V0_1.md
```

Prototype gate:

```text
observation → hypothesis → instrumented test → failed or successful replication → applied consequence
```

## Patch Set C — Authorship and Long-Horizon Purpose

Adds:

```text
canon/PLAYER_AUTHORSHIP_SANDBOX_AND_MODDING_CONTRACT_V0_1.md
canon/WORLDLINE_LONG_HORIZON_AND_ENDGAME_CONTRACT_V0_1.md
```

These remain canonical drafts. They prevent near-term architecture from making future creator tools, save migration, mature worlds, and worldline forks impossible.

## Patch Set D — Multiplayer Safety

Adds:

```text
tech/MULTIPLAYER_SOCIAL_SAFETY_GRIEFING_AND_MODERATION_V0_1.md
```

Prototype gate:

```text
declared conflict profile
scoped permissions
protected essential infrastructure
targeted abuse rollback
inactive-authority recovery
```

## Patch Set E — Canon Integration

Updates:

```text
README
Canon Registry
System Interaction Map
Progression Contract
Player Experience Contract
Playtest Program
Roadmap
Document Registry and audits
```

## Deferrals

This campaign does not require Seedworks v0.1 to ship:

```text
public mod workshop
full late-game planetary simulation
open PvP worldlines
full player mission editor
inter-world Confluence
```

It requires the architecture and canon not to contradict those futures.
