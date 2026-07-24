---
title: Canon Registry and Document Governance
version: 0.4
status: superseded
scope: documentation lifecycle, canonical spine, ownership, supersession, registry policy
superseded_by:
  - CANON_REGISTRY_AND_DOCUMENT_GOVERNANCE_V0_5.md
owner: documentation/design
supersedes:
  - CANON_REGISTRY_AND_DOCUMENT_GOVERNANCE_V0_3.md
related:
  - ../ops/DOCUMENT_REGISTRY.json
  - ../ops/DOCUMENT_METADATA_MIGRATION_AND_CONSOLIDATION_PLAN_V0_2.md
  - ../ops/V0_8_IMPLEMENTATION_HARDENING_CAMPAIGN.md
---

# Canon Registry and Document Governance

## Purpose

The Symtropy corpus is large enough that authority must be explicit.

```text
A newer file is not automatically canonical.
A longer file is not automatically authoritative.
A concept catalog is not a production commitment.
A prototype checklist is not the current product definition.
```

## Status Vocabulary

```text
canonical
canonical-draft
supporting
implementation-spec
experimental
historical
superseded
unclassified
```

- **canonical** defines current product truth. Contradictory documents defer to it.
- **canonical-draft** is intended direction awaiting prototype, review, or consolidation evidence.
- **supporting** expands canonical material without changing product boundaries.
- **implementation-spec** defines a concrete technical, data, runtime, production, or validation contract.
- **experimental** explores a possibility and may not silently create milestone scope.
- **historical** is retained for provenance and earlier implementation truth.
- **superseded** has been replaced by a named document.
- **unclassified** is not canonical by default.

## Conflict Hierarchy

When active documents conflict, use this order:

1. Game Constitution.
2. Canonical player-experience, progression, economy, war, and system-integration contracts.
3. Current milestone and representative-slice specifications.
4. Architecture decisions and implementation specifications.
5. System design bibles.
6. Supporting vision and world documents.
7. Lore catalogs, concept-art prompts, and historical plans.

# Current Canonical Spine

## Product Identity and Play

```text
canon/SYMTROPY_GAME_CONSTITUTION_V0_6.md
canon/PLAYER_EXPERIENCE_AND_SESSION_RHYTHM_CONTRACT_V0_1.md
canon/CORE_GAMEPLAY_PILLARS_AND_VERB_MATRIX_V0_1.md
canon/SCALE_LADDER_AND_PROGRESSION_CONSTITUTION_V0_1.md
canon/PROGRESSION_ECONOMY_AND_MASTERY_CONTRACT_V0_1.md
canon/SYSTEM_INTERACTION_AND_DEPENDENCY_MAP_V0_1.md
canon/MISSION_EVENT_AND_CONTRACT_GRAMMAR_V0_1.md
canon/SCIENCE_RESEARCH_AND_DISCOVERY_CONTRACT_V0_1.md
```

These jointly own what the game is, what players do, what sessions feel like, how capability grows, how systems exchange consequences, and how scale expands.

## Economy and Material Power

```text
canon/ECONOMY_INTEGRITY_MARKETS_LABOR_AND_ANTI_EXPLOIT_CONTRACT_V0_1.md
tech/ECONOMIC_LEDGER_MARKET_AND_INTEGRITY_RUNTIME_V0_1.md
SYMTROPY_RESOURCE_CHAINS_GAME_DOC_V0_1.md
Symtropy Profession Loops and Legibility Progression.md
```

The canonical economy contract owns value, labor, property, market integrity, wealth concentration, and anti-exploit rules. The runtime specification owns custody transitions, transaction envelopes, market state, audit, and conservation invariants.

## War, Diplomacy, Territory, and Peace

```text
canon/WAR_DIPLOMACY_TERRITORY_AND_LOGISTICS_CONTRACT_V0_1.md
tech/STRATEGIC_CONFLICT_CAMPAIGN_AND_OCCUPATION_SIMULATION_V0_1.md
tech/COMBAT_THREAT_AND_SYSTEMIC_ENCOUNTER_DESIGN_V0_1.md
tech/MULTIPLAYER_SOCIAL_SAFETY_GRIEFING_AND_MODERATION_V0_1.md
```

The canonical war contract owns strategic meaning, war aims, civilian protection, territorial capability, diplomacy, and peace. The simulation specification owns force readiness, supply, control vectors, occupations, displacement, and strategic level of detail. Tactical combat remains owned by the encounter bible.

## Living Agents and Society

```text
vision/NPC_DAILY_LIFE_RELATIONSHIPS_AND_SOCIAL_MEMORY_BIBLE_V0_2.md
tech/NPC_COGNITION_AGENCY_AND_SIMULATION_RUNTIME_V0_1.md
vision/CIVILIZATION_DELIGHT_PLAY_AND_EVERYDAY_LIFE_BIBLE_V0_1.md
Symtropy Player Cities & Society.md
lore/SOCIAL_SYSTEMS_AND_CHARTERS.md
tech/PROCEDURAL_HISTORY_ENGINE.md
tech/PROCEDURAL_FACTION_EVOLUTION.md
```

The NPC life bible owns player-facing life, memory, relationships, and ordinary behavior. The runtime contract owns perception, belief, planning, action authority, simulation LOD, dialogue grounding, and causal traces.

## Simulation, Truth, and Durable Worldlines

```text
tech/REGIONAL_PLANETARY_CIVILIZATION_SIMULATION_ARCHITECTURE_V0_1.md
tech/MULTIPLAYER_TRUTH_MODEL.md
tech/NETWORKING_STACK_DECISION.md
tech/CHRONICLE_EVENT_SCHEMA.md
tech/WORLDLINE_MECHANICAL_DELTA_SCHEMA_V0_1.md
tech/WORLDLINE_PERSISTENCE_MIGRATION_AND_DISASTER_RECOVERY_PROTOCOL_V0_1.md
ops/WORLDLINE_BACKUP_RESTORE_AND_UPGRADE_RUNBOOK_V0_1.md
```

The worldline persistence protocol supersedes `tech/WORLD_PERSISTENCE_PROTOCOL.md`. It owns checkpoints, event journals, migrations, mod compatibility, rollback boundaries, forks, backups, retention, and disaster recovery.

## Seedworks Representative Build

```text
ops/SEEDWORKS_REGIONAL_CIVILIZATION_SLICE_V0_2.md
ops/SEEDWORKS_ONBOARDING_AND_FIRST_TEN_HOURS_V0_1.md
ops/SEEDWORKS_PRODUCTION_BUDGET_AND_CONTENT_PLAN_V0_1.md
ops/SEEDWORKS_NEXT_BUILD_PLAN.md
tech/SEEDWORKS_ARCHITECTURE.md
ops/PLAYTEST_RESEARCH_PROGRAM_V0_2.md
```

The Old Waterworks is a supporting authored site, not the whole product proof.

## Embodied Experience and Interface

```text
vision/PLAYER_FEEL_AND_EMBODIED_INTERACTION_BIBLE_V0_2.md
tech/FIELD_DECK_INTERFACE_AND_INFORMATION_ARCHITECTURE_BIBLE_V0_2.md
vision/EXPLORATION_DISCOVERY_AND_AWE_DESIGN_BIBLE_V0_1.md
tech/WORLD_STATE_REVISITABILITY_AND_CONSEQUENCE_PRESENTATION_V0_1.md
```

## Construction, Computing, Robotics, and Mobility

```text
tech/Symtropy Design Doc - Cybernetic Crafting & Physical Node Assembly.md
tech/IN_WORLD_COMPUTING_AND_SYMTROPYOS.md
tech/DEVICE_BUS_RUNTIME_SAFETY.md
ops/ROBOTICS_ROADMAP_TECH_TREE_EXPANSION_V0_3_2.md
tech/Symtropy Vehicle & Mobility Design.md
ops/SEEDWORKS_TECH_TREE_AUDIT_AND_HORIZON_GATES_V0_3_3.md
```

## Threats, Aliens, and Nonhuman Agency

```text
lore/HOSTILE_FACTIONS_AND_THREAT_ECOLOGY.md
lore/ALIEN_TYPES_AND_FIRST_CONTACT_EC.md
lore/NONHUMAN_GAME_THEORY_AND_AGENCY.md
lore/FIRST_CONTACT_ESCALATION_LADDER.md
```

## Player Authorship and Long-Horizon Play

```text
canon/PLAYER_AUTHORSHIP_SANDBOX_AND_MODDING_CONTRACT_V0_1.md
canon/WORLDLINE_LONG_HORIZON_AND_ENDGAME_CONTRACT_V0_1.md
```

These remain canonical drafts until creator tooling and mature-world prototypes provide evidence.

# Document Ownership Rule

Every active document must own one distinct question.

Recommended opening fields:

```text
Owned question
What this document does not own
Canonical dependencies
Scope and deferrals
Acceptance evidence
```

## Scope Vocabulary

Use these exact categories:

```text
implemented
current milestone
architected-deferred
horizon-visible
lore-only
historical
```

Avoid ambiguous “should” statements when the real meaning is “possible someday.”

## Required Front Matter

New canonical and implementation documents require:

```yaml
---
title:
version:
status:
scope:
owner:
related:
---
```

Canonical documents should also specify `supersedes` when replacing active authority.

# Acceptance Rule

A document is not implementation-ready merely because it contains structs or algorithms. It must also define:

```text
authoritative owner
inputs and outputs
failure behavior
persistence class
observability
security or abuse boundaries
performance or simulation budget
acceptance tests
```

# v0.8 Hardening Boundary

The v0.8 campaign adds contracts for NPC cognition, strategic conflict, economy integrity, and durable worldline persistence. These specifications do not add Seedworks content scope by themselves. They constrain architecture and prevent future systems from acquiring contradictory authority.
