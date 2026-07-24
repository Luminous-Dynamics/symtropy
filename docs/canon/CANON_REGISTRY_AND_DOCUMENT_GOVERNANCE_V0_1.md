---
title: Canon Registry and Document Governance
version: 0.1
status: superseded
superseded_by: canon/CANON_REGISTRY_AND_DOCUMENT_GOVERNANCE_V0_2.md
scope: documentation lifecycle
---

# Canon Registry and Document Governance

## Purpose

The Symtropy corpus is now large enough that document status must be explicit.

A newer file is not automatically canonical.
A longer file is not automatically authoritative.
A concept-art catalog is not automatically a production requirement.

## Status Vocabulary

Every active design document should declare one status:

```text
canonical
canonical-draft
supporting
implementation-spec
experimental
historical
superseded
```

### canonical

Defines current product truth. Contradictory documents must defer to it.

### canonical-draft

Expected to become canonical, but still open to structural change.

### supporting

Expands a canonical concept without changing its boundaries.

### implementation-spec

Defines a concrete technical or production contract.

### experimental

Explores a possibility. Must not silently create scope.

### historical

Preserved for provenance, prior decisions, or recovered ideas.

### superseded

Replaced by a named document. May remain useful as a module.

## Canonical Hierarchy

When documents conflict, use this order:

1. Game Constitution
2. Canonical product and progression documents
3. Current milestone and slice specifications
4. Technical architecture decisions
5. System design bibles
6. Lore modules and catalogs
7. Concept art prompts and historical plans

## Current Canonical Spine

### Product

```text
canon/SYMTROPY_GAME_CONSTITUTION_V0_5.md
canon/CORE_GAMEPLAY_PILLARS_AND_VERB_MATRIX_V0_1.md
canon/SCALE_LADDER_AND_PROGRESSION_CONSTITUTION_V0_1.md
vision/Symtropy Vision Document.md
```

The Vision Document remains the broad source. The Game Constitution governs interpretation when the vision appears narrower in a specific example.

### Seedworks

```text
ops/SEEDWORKS_REGIONAL_CIVILIZATION_SLICE_V0_2.md
ops/SEEDWORKS_NEXT_BUILD_PLAN.md
tech/SEEDWORKS_ARCHITECTURE.md
```

The Old Waterworks remains a valid site and tutorial module, not the whole-game thesis.

### World and Society

```text
Symtropy Player Cities & Society.md
lore/SOCIAL_SYSTEMS_AND_CHARTERS.md
tech/PROCEDURAL_HISTORY_ENGINE.md
tech/PROCEDURAL_FACTION_EVOLUTION.md
```

### Computing and Truth

```text
tech/IN_WORLD_COMPUTING_AND_SYMTROPYOS.md
tech/MULTIPLAYER_TRUTH_MODEL.md
tech/NETWORKING_STACK_DECISION.md
tech/CHRONICLE_EVENT_SCHEMA.md
```

### Physical Construction

```text
tech/Symtropy Design Doc - Cybernetic Crafting & Physical Node Assembly.md
SYMTROPY_RESOURCE_CHAINS_GAME_DOC_V0_1.md
tech/Symtropy Vehicle & Mobility Design.md
```

## Known Version Families

The following families require explicit consolidation rather than silent deletion:

```text
PLAYER_ORIGINS_AND_WORLDLINE_STARTS.md
Symtropy Player Origins Full Des.md

SYMTROPY_LITHIC_AND_SUBCRUST_CULTURES_V0_1.md
SYMTROPY_LITHIC_AND_SUBCRUST_CULTURES_V0_2.md

Symtropy_New_Cultures_Compendium_v0_1.md
Symtropy_New_Cultures_Compendium_v0_3.md

Robotics_Platform_ROADMAP.md
ROBOTICS_ROADMAP_TECH_TREE_EXPANSION_V0_3_2.md
SEEDWORKS_TECH_TREE_AUDIT_AND_HORIZON_GATES_V0_3_3.md
```

Recommended rule:

```text
keep the newest validated version canonical
mark older versions historical
extract any unique material before archiving
```

## Required Front Matter

New canonical and implementation documents should begin with:

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

## Scope Language

Documents must distinguish:

```text
present in current build
targeted for current milestone
architected but deferred
horizon concept
lore-only possibility
```

Avoid using “should” where the real meaning is “could someday.”

## Change Protocol

A canonical change should record:

```text
decision
reason
affected documents
migration or compatibility impact
new acceptance test
```

## Naming Rules

Prefer:

```text
UPPER_SNAKE_CASE for implementation and operations specifications
Title Case for long-form design bibles
explicit version suffix for revisable design families
```

Do not create a new file solely because the title changed.

## Duplication Rule

Repeated principles may be quoted briefly, then linked to their canonical definition.

Each document should own a distinct question.

Examples:

```text
Game Constitution:
What is Symtropy?

Verb Matrix:
What does the player do?

Regional Slice:
What is built first?

Simulation Architecture:
How do scales communicate?

Lore Atlas:
What might exist in the world?
```

## Review Checklist

Before promoting a document to canonical:

```text
Does it conflict with the Game Constitution?
Does it create new production scope?
Does it identify actual player actions?
Does it connect to existing systems?
Does it define what is deferred?
Does it name acceptance evidence?
Does it supersede another file?
Are all internal links valid?
```

## Archive Policy

Historical files should be preserved when they contain:

```text
important reasoning
discarded alternatives
evidence
old implementation state
unique worldbuilding
```

They should be moved or marked so that a new contributor cannot mistake them for current direction.
