---
title: Document Metadata Migration and Consolidation Plan
version: 0.2
status: implementation-spec
scope: corpus metadata, duplicate families, canonical ownership, archive migration
owner: documentation/design
related:
  - canon/CANON_REGISTRY_AND_DOCUMENT_GOVERNANCE_V0_2.md
  - ops/DOCUMENT_REGISTRY.json
---

# Document Metadata Migration and Consolidation Plan

## Executive Finding

The corpus contains strong material but remains difficult to govern because most files predate the canonical metadata standard.

Current baseline before this pass:

```text
147 Markdown documents
100 without YAML front matter
several parallel version families
mixed naming conventions
old prototype plans adjacent to current canon
```

The solution is not to rewrite every document at once.

Use a staged registry-first migration.

## Phase 1 — Registry Coverage

Create a machine-readable registry for every Markdown file with:

```text
path
title
category
front-matter presence
status
version
owned question
canonical owner
superseded-by
review state
```

Files without explicit metadata receive:

```text
status: unclassified
review_state: metadata-needed
```

Do not infer canon solely from filename or date.

## Phase 2 — Critical Spine Metadata

Add or validate front matter for:

```text
all canon documents
current Seedworks operations documents
architecture decisions
current implementation specifications
```

Required fields:

```yaml
title:
version:
status:
scope:
owner:
supersedes:
related:
```

## Phase 3 — Duplicate Family Resolution

### Origins

Candidate owner:

```text
vision/PLAYER_ORIGINS_AND_WORLDLINE_STARTS.md
```

Required consolidation output:

```text
one launch-origin registry
one mechanical hook matrix
separate expanded biographies
```

### Lithic Cultures

Candidate owner:

```text
lore/SYMTROPY_LITHIC_AND_SUBCRUST_CULTURES_V0_2.md
```

Mark v0.1 historical after unique-content comparison.

### New Cultures Compendium

Candidate owner:

```text
lore/Symtropy_New_Cultures_Compendium_v0_3.md
```

Mark v0.1 historical after extraction.

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

The audit owns current playable scope. Catalogs own platform detail and horizon.

### Field Deck

Decision owner:

```text
tech/FIELD_DECK_INTERFACE_AND_INFORMATION_ARCHITECTURE_BIBLE_V0_2.md
```

Extract unique material from:

```text
tech/FIELD_DECK_AND_INTERFACE_SOVEREIGNTY.md
tech/FIELD_DECK_INTERACTION_AND_MODES.md
```

Then mark them supporting or historical.

### Seedworks Slice

Decision owner:

```text
ops/SEEDWORKS_REGIONAL_CIVILIZATION_SLICE_V0_2.md
```

The older pump-centered playable slice becomes historical/supporting. The Old Waterworks remains a level bible.

## Phase 4 — Ownership Headers

Every active document should state near the top:

```text
Owned question
What this document does not own
Canonical dependencies
```

This reduces repeated principles and accidental scope expansion.

## Phase 5 — Archive Migration

Move or clearly mark:

```text
old stabilization reports
superseded slice plans
obsolete implementation status
older validated version families
```

Preserve history and links through redirect stubs or registry aliases.

## Naming Migration

New files should use:

```text
UPPER_SNAKE_CASE_VX_Y.md for operations and implementation specs
Title Case.md for long-form bibles only when stable and non-versioned
```

Existing filenames with spaces do not need immediate renaming unless touched for substantive revision.

Avoid churn that breaks external references.

## Link Policy

Internal links should be relative.

Validation must check:

```text
missing target
case mismatch
URL-encoded spaces
moved/superseded alias
anchor existence for critical links
```

## Promotion Gates

A document may become canonical only when:

```text
its owned question is unique
scope and deferrals are explicit
contradictions are resolved or recorded
player actions are identified
system dependencies are named
acceptance evidence exists
superseded files are listed
```

## Immediate Actions Completed in v0.6 Pass

```text
new Game Constitution v0.6
new Player Experience contract
new System Interaction map
new Progression/Economy contract
new Onboarding plan
new Production Budget
new Playtest Research Program
new Delight/Everyday Life bible
updated root README
updated canon registry
machine-readable document registry
```

## Next Recommended Metadata Batch

Prioritize these 20 files:

```text
vision/Symtropy Vision Document.md
tech/ARCHITECTURE.md
tech/ENGINE.md
tech/SEEDWORKS_ARCHITECTURE.md
tech/MULTIPLAYER_TRUTH_MODEL.md
tech/NETWORKING_STACK_DECISION.md
tech/PROCEDURAL_HISTORY_ENGINE.md
tech/PROCEDURAL_FACTION_EVOLUTION.md
tech/IN_WORLD_COMPUTING_AND_SYMTROPYOS.md
tech/CHRONICLE_EVENT_SCHEMA.md
tech/WORLD_PERSISTENCE_PROTOCOL.md
ops/ROADMAP.md
ops/SEEDWORKS_NEXT_BUILD_PLAN.md
ops/SEEDWORKS_TECH_TREE_AUDIT_AND_HORIZON_GATES_V0_3_3.md
vision/PLAYER_ORIGINS_AND_WORLDLINE_STARTS.md
vision/Symtropy Architecture Design Bible.md
lore/HOSTILE_FACTIONS_AND_THREAT_ECOLOGY.md
lore/SOCIAL_SYSTEMS_AND_CHARTERS.md
SYMTROPY_RESOURCE_CHAINS_GAME_DOC_V0_1.md
Symtropy Profession Loops and Legibility Progression.md
```

## Final Rule

```text
The registry identifies what exists.
Front matter identifies what governs.
Archive policy preserves why earlier decisions existed.
```
