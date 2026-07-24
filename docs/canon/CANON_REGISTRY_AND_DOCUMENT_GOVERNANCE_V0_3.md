---
title: Canon Registry and Document Governance
version: 0.3
status: superseded
superseded_by: CANON_REGISTRY_AND_DOCUMENT_GOVERNANCE_V0_4.md
scope: documentation lifecycle, canonical spine, ownership, supersession, registry policy
owner: documentation/design
supersedes:
  - CANON_REGISTRY_AND_DOCUMENT_GOVERNANCE_V0_2.md
related:
  - ../ops/DOCUMENT_REGISTRY.json
  - ../ops/DOCUMENT_METADATA_MIGRATION_AND_CONSOLIDATION_PLAN_V0_2.md
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

### canonical

Defines current product truth. Contradictory documents defer to it.

### canonical-draft

Expected to become canonical after prototype, review, or consolidation evidence.

### supporting

Expands canonical material without changing its product boundaries.

### implementation-spec

Defines a concrete technical, data, runtime, production, or validation contract.

### experimental

Explores a possibility. It may not silently create current milestone scope.

### historical

Preserved for provenance, unique material, and old implementation truth.

### superseded

Replaced by a named document. It may remain useful as a site, module, or rationale source.

### unclassified

Present in the corpus but not yet assigned an explicit lifecycle status.

Unclassified material is not canonical by default.

## Conflict Hierarchy

When active documents conflict, use this order:

1. Game Constitution
2. Canonical player-experience, progression, and system-integration contracts
3. Current milestone and representative-slice specifications
4. Architecture decisions and implementation specifications
5. System design bibles
6. Supporting vision and world documents
7. Lore catalogs, concept-art prompts, and historical plans

## Current Canonical Spine

### Product Identity

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

These documents jointly own:

```text
what the game is
what players do
what a session feels like
how capability grows
how systems meet
how scale expands
```

### Broad Vision

```text
vision/Symtropy Vision Document.md
```

This remains the broad horizon source. The Game Constitution governs product interpretation.

### Seedworks Representative Build

```text
ops/SEEDWORKS_REGIONAL_CIVILIZATION_SLICE_V0_2.md
ops/SEEDWORKS_ONBOARDING_AND_FIRST_TEN_HOURS_V0_1.md
ops/SEEDWORKS_PRODUCTION_BUDGET_AND_CONTENT_PLAN_V0_1.md
ops/SEEDWORKS_NEXT_BUILD_PLAN.md
tech/SEEDWORKS_ARCHITECTURE.md
ops/PLAYTEST_RESEARCH_PROGRAM_V0_2.md
```

The Old Waterworks is a supporting authored site, not the entire product proof.

### Embodied Experience and Interface

```text
vision/PLAYER_FEEL_AND_EMBODIED_INTERACTION_BIBLE_V0_2.md
tech/FIELD_DECK_INTERFACE_AND_INFORMATION_ARCHITECTURE_BIBLE_V0_2.md
tech/COMBAT_THREAT_AND_SYSTEMIC_ENCOUNTER_DESIGN_V0_1.md
vision/EXPLORATION_DISCOVERY_AND_AWE_DESIGN_BIBLE_V0_1.md
```

### Living Civilization

```text
vision/NPC_DAILY_LIFE_RELATIONSHIPS_AND_SOCIAL_MEMORY_BIBLE_V0_2.md
vision/CIVILIZATION_DELIGHT_PLAY_AND_EVERYDAY_LIFE_BIBLE_V0_1.md
Symtropy Player Cities & Society.md
lore/SOCIAL_SYSTEMS_AND_CHARTERS.md
tech/PROCEDURAL_HISTORY_ENGINE.md
tech/PROCEDURAL_FACTION_EVOLUTION.md
```

### Simulation and Persistence

```text
tech/REGIONAL_PLANETARY_CIVILIZATION_SIMULATION_ARCHITECTURE_V0_1.md
tech/MULTIPLAYER_TRUTH_MODEL.md
tech/NETWORKING_STACK_DECISION.md
tech/CHRONICLE_EVENT_SCHEMA.md
tech/WORLD_PERSISTENCE_PROTOCOL.md
```

### Construction, Economy, and Mobility

```text
tech/Symtropy Design Doc - Cybernetic Crafting & Physical Node Assembly.md
SYMTROPY_RESOURCE_CHAINS_GAME_DOC_V0_1.md
Symtropy Profession Loops and Legibility Progression.md
tech/Symtropy Vehicle & Mobility Design.md
ops/SEEDWORKS_TECH_TREE_AUDIT_AND_HORIZON_GATES_V0_3_3.md
```

### Threats, Aliens, and Nonhuman Agency

```text
lore/HOSTILE_FACTIONS_AND_THREAT_ECOLOGY.md
lore/ALIEN_TYPES_AND_FIRST_CONTACT_EC.md
lore/NONHUMAN_GAME_THEORY_AND_AGENCY.md
lore/FIRST_CONTACT_ESCALATION_LADDER.md
```


### Player Authorship and Long-Horizon Play

```text
canon/PLAYER_AUTHORSHIP_SANDBOX_AND_MODDING_CONTRACT_V0_1.md
canon/WORLDLINE_LONG_HORIZON_AND_ENDGAME_CONTRACT_V0_1.md
```

These define intended direction but remain canonical drafts until creator tooling and mature-world prototypes provide evidence.

### Social Safety and Consequence Presentation

```text
tech/MULTIPLAYER_SOCIAL_SAFETY_GRIEFING_AND_MODERATION_V0_1.md
tech/WORLD_STATE_REVISITABILITY_AND_CONSEQUENCE_PRESENTATION_V0_1.md
```

These implementation specifications constrain multiplayer authority, abuse recovery, world-state visibility, and revisitability.

## Document Ownership Rule

Every active document must own one distinct question.

Recommended opening fields:

```text
Owned question
What this document does not own
Canonical dependencies
Scope and deferrals
Acceptance evidence
```

Examples:

```text
Game Constitution:
What is Symtropy?

Player Experience Contract:
What should a session feel like?

Progression Contract:
How does capability grow?

System Interaction Map:
How do domains exchange consequences?

Regional Slice:
What representative experience is built first?

Production Budget:
How much content is enough to prove it?
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
supersedes:
related:
---
```

Empty fields may be omitted, but status, scope, and owner may not.

## Change Protocol

A canonical change must record:

```text
decision
reason
affected documents
scope impact
migration/compatibility impact
new or changed acceptance test
```

## Duplication Rule

Repeated principles may be quoted briefly and linked to their canonical owner.

Do not copy entire conceptual frameworks into several files.

A document may restate a principle only to apply it to its owned question.

## Current Supersession Decisions

```text
SYMTROPY_GAME_CONSTITUTION_V0_5.md
  → superseded by V0_6

CANON_REGISTRY_AND_DOCUMENT_GOVERNANCE_V0_1.md
  → superseded by V0_2

ops/PLAYTEST_CHECKLIST.md
  → historical; superseded by PLAYTEST_RESEARCH_PROGRAM_V0_2.md

ops/SEEDWORKS_PLAYABLE_SLICE_SPEC.md
  → superseded by SEEDWORKS_REGIONAL_CIVILIZATION_SLICE_V0_2.md
  → retained as Old Waterworks implementation reference

lore/SYMTROPY_LITHIC_AND_SUBCRUST_CULTURES_V0_1.md
  → historical; superseded by V0_2

lore/Symtropy_New_Cultures_Compendium_v0_1.md
  → historical; superseded by V0_3
```

## Version Families Still Requiring Consolidation

### Origins

```text
vision/PLAYER_ORIGINS_AND_WORLDLINE_STARTS.md
vision/Symtropy Player Origins Full Des.md
```

Required output:

```text
launch registry
mechanical hook matrix
expanded biography modules
```

### Robotics

Decision owner:

```text
ops/SEEDWORKS_TECH_TREE_AUDIT_AND_HORIZON_GATES_V0_3_3.md
```

Supporting catalogs:

```text
ops/ROBOTICS_ROADMAP_TECH_TREE_EXPANSION_V0_3_2.md
ops/Robotics_Platform_ROADMAP.md
```

### Field Deck Legacy Documents

Decision owner:

```text
tech/FIELD_DECK_INTERFACE_AND_INFORMATION_ARCHITECTURE_BIBLE_V0_2.md
```

Review and classify:

```text
tech/FIELD_DECK_AND_INTERFACE_SOVEREIGNTY.md
tech/FIELD_DECK_INTERACTION_AND_MODES.md
```

### Culture Atlas

Do not combine every culture into one giant file.

Create:

```text
one registry/schema owner
separate deep-dive modules
explicit production tiers
```

## Registry Policy

`ops/DOCUMENT_REGISTRY.json` inventories every Markdown file.

It records explicit metadata when present and flags missing metadata.

The registry does not promote unclassified documents to canon.

Regenerate after structural changes.

## Review Checklist

Before promoting a document:

```text
Is its owned question unique?
Does it conflict with the Game Constitution?
Does it create current production scope?
Are current, deferred, and horizon content separated?
Are player actions identified?
Are system dependencies explicit?
Is there acceptance evidence?
Are superseded files listed?
Are internal links valid?
```

## Archive Policy

Preserve historical files containing:

```text
important reasoning
discarded alternatives
evidence
prior implementation truth
unique worldbuilding
```

Move or mark them so a contributor cannot mistake them for current direction.

## Final Rule

```text
Canon says what governs.
The registry says what exists.
Evidence says what has actually been proven.
```
