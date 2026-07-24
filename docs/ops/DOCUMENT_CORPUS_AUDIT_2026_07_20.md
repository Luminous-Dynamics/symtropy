---
title: Symtropy Document Corpus Audit
date: 2026-07-20
version: 1.0
status: implementation-spec
scope: documentation structure, overlap, gaps, canonicalization
owner: documentation/design
---

# Symtropy Document Corpus Audit — 2026-07-20

## Executive Finding

The corpus is not short on imagination.

It contains **145 Markdown documents** and approximately **362,026 words** after this improvement pass.

The primary risk is no longer missing lore. It is loss of product shape through:

```text
repeated principles
parallel version families
water-centered first-playable framing
weak separation between canon, horizon, and current milestone
insufficient ownership of exploration, combat feel, and scale progression
```

## What Is Already Strong

### Civilizational Identity

The corpus consistently treats infrastructure, history, ecology, technology, and culture as connected.

### Diegetic Systems

The Field Deck, Device Bus, Chronicle, SymtropyOS, source chains, physical crafting, vehicles, and architecture form a coherent world grammar.

### Ethical and Cultural Breadth

The alien, cultural, body-sovereignty, threat-ecology, and charter documents are unusually rich and avoid simple species or ideology morality.

### Technical Restraint

The multiplayer truth model and networking decisions correctly separate real-time action from durable history.

### Production Seeds

The Old Waterworks, Earth Atlas patches, robotics roadmap, Infrastructure Loom, and foundry documents contain many implementable modules.

## Highest-Risk Problems

### 1. The Example Became the Product

The first playable specification was explicitly titled:

```text
The First Pump Teaches the Whole Game
```

The pump is a good tutorial site, but the repeated framing narrowed the apparent genre toward civic utility repair.

Correction:

```text
Old Waterworks remains a site.
Firstlight Basin becomes the product proof.
```

### 2. Missing Canonical Layer

The prior root index linked categories but did not establish authority when documents conflicted.

Correction:

```text
canon/SYMTROPY_GAME_CONSTITUTION_V0_5.md
canon/CORE_GAMEPLAY_PILLARS_AND_VERB_MATRIX_V0_1.md
canon/SCALE_LADDER_AND_PROGRESSION_CONSTITUTION_V0_1.md
canon/CANON_REGISTRY_AND_DOCUMENT_GOVERNANCE_V0_1.md
```

### 3. Exploration Was Under-Owned

Exploration appeared across vehicles, planets, architecture, aliens, and vision, but no document owned the full discovery loop.

Correction:

```text
vision/EXPLORATION_DISCOVERY_AND_AWE_DESIGN_BIBLE_V0_1.md
```

### 4. Combat Had Meaning but Needed a Feel Contract

The threat and dungeon documents are philosophically strong, but combat required a dedicated tactical grammar.

Correction:

```text
tech/COMBAT_THREAT_AND_SYSTEMIC_ENCOUNTER_DESIGN_V0_1.md
```

### 5. Simulation Scales Were Fragmented

Settlement state, history, factions, ecology, persistence, and worldlines existed in separate documents without one fidelity architecture.

Correction:

```text
tech/REGIONAL_PLANETARY_CIVILIZATION_SIMULATION_ARCHITECTURE_V0_1.md
```

## High-Overlap Families

Semantic review found the strongest overlap in:

```text
PLAYER_ORIGINS_AND_WORLDLINE_STARTS
Player Origins Full Design

Lithic and Subcrust Cultures v0.1
Lithic and Subcrust Cultures v0.2

New Cultures Compendium v0.1
New Cultures Compendium v0.3

Multiplayer Truth Model
Networking Stack Decision

Robotics Platform Roadmap
Robotics Roadmap + Tech Tree Expansion
Tech Tree Audit and Horizon Gates
```

Overlap is not automatically bad. The next pass should name one owner question per file and mark earlier versions historical.

## Link Health

Before the pass, three internal links were broken:

```text
tech/GAME_ENGINE_COMPONENTS.md -> ../ROADMAP.md
ops/ROADMAP.md -> docs/GAME_ENGINE_COMPONENTS.md
ops/ROADMAP.md -> ./LICENSING.md
```

All three were corrected.

Current internal Markdown link audit:

```text
checked links: 37
broken links: 0
```

## New and Improved Documents

### New Canonical Product Layer

```text
canon/SYMTROPY_GAME_CONSTITUTION_V0_5.md
canon/CORE_GAMEPLAY_PILLARS_AND_VERB_MATRIX_V0_1.md
canon/SCALE_LADDER_AND_PROGRESSION_CONSTITUTION_V0_1.md
canon/CANON_REGISTRY_AND_DOCUMENT_GOVERNANCE_V0_1.md
```

### New Production and System Documents

```text
ops/SEEDWORKS_REGIONAL_CIVILIZATION_SLICE_V0_2.md
vision/EXPLORATION_DISCOVERY_AND_AWE_DESIGN_BIBLE_V0_1.md
tech/COMBAT_THREAT_AND_SYSTEMIC_ENCOUNTER_DESIGN_V0_1.md
tech/REGIONAL_PLANETARY_CIVILIZATION_SIMULATION_ARCHITECTURE_V0_1.md
```

### Expanded Production Bibles

```text
vision/PLAYER_FEEL_AND_EMBODIED_INTERACTION_BIBLE_V0_2.md
tech/FIELD_DECK_INTERFACE_AND_INFORMATION_ARCHITECTURE_BIBLE_V0_2.md
vision/NPC_DAILY_LIFE_RELATIONSHIPS_AND_SOCIAL_MEMORY_BIBLE_V0_2.md
```

### Updated Existing Canon

```text
vision/Symtropy Vision Document.md
ops/SEEDWORKS_PLAYABLE_SLICE_SPEC.md
lore/OLD_WATERWORKS_LEVEL_BIBLE.md
tech/GAME_ENGINE_COMPONENTS.md
ops/ROADMAP.md
README.md
```

## Recommended Next Consolidation Wave

### A. Origins

Create one canonical origin registry with:

```text
Seedworks launch origins
horizon origins
mechanical hook matrix
unique dialogue burden
```

Archive duplicated prose after extracting unique material.

### B. Culture Atlas

Consolidate:

```text
Subcultural Society Atlas
Cultural Logic Atlas
New Cultures Compendium
Dark Cultures Codex
Societies of Desire, Restraint, Care, and Control
```

Do not merge all prose into one giant file.

Create:

```text
one schema
one registry
separate deep-dive modules
```

### C. Robotics and Tech Tree

Make the v0.3.3 audit the decision owner.

Separate:

```text
current playable nodes
dependency model
platform catalog
future horizon
```

### D. Field Deck

Reconcile the two older Field Deck documents with the v0.2 information-architecture bible.

Preserve:

```text
physical interaction model
mode semantics
epistemic classes
activity context profiles
```

### E. Operations

Distinguish:

```text
current build plan
roadmap
historical stabilization reports
implementation tickets
```

Old status reports should remain in archive only.

## Documentation Quality Gates

Future canonical docs should contain:

```text
status and version
owned question
scope and deferrals
player actions
system dependencies
acceptance evidence
superseded files
```

## Final Assessment

Symtropy already has enough worldbuilding to sustain a very large game.

The next gains will come from:

```text
consolidation
playable contracts
cross-system ownership
prototype evidence
```

The corpus should now expand only where a new document owns a genuinely missing design problem.
