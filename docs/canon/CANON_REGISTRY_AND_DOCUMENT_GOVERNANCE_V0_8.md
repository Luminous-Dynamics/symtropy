---
title: Canon Registry and Document Governance
version: 0.8
status: superseded
scope: documentation lifecycle, canonical spine, ownership, supersession, registry policy
owner: documentation/design
supersedes:
  - CANON_REGISTRY_AND_DOCUMENT_GOVERNANCE_V0_7.md
superseded_by: CANON_REGISTRY_AND_DOCUMENT_GOVERNANCE_V0_9.md
related:
  - ../ops/DOCUMENT_REGISTRY.json
  - ../ops/DOCUMENT_METADATA_MIGRATION_AND_CONSOLIDATION_PLAN_V0_2.md
  - ../ops/V1_2_LIVED_WORLD_SOCIAL_CONSEQUENCE_CAMPAIGN.md
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
canon/SYSTEM_INTERACTION_AND_DEPENDENCY_MAP_V0_5.md
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


## Embodied Social Intelligence

```text
canon/SYMTHAEA_NPC_INTEGRATION_CONTRACT_V0_1.md
canon/NPC_COGNITIVE_RIGHTS_PRIVACY_AND_PLAYER_BOUNDARIES_CONTRACT_V0_1.md
canon/NPC_EMBODIMENT_AFFECT_AND_NONVERBAL_EXPRESSION_CONTRACT_V0_1.md
canon/LIFE_COURSE_HOUSEHOLDS_KINSHIP_AND_EDUCATION_CONTRACT_V0_1.md
tech/SYMTHAEA_NPC_COGNITION_BRIDGE_ARCHITECTURE_V0_1.md
tech/SOCIAL_COGNITION_THEORY_OF_MIND_AND_RELATIONSHIP_RUNTIME_V0_1.md
tech/NPC_MEMORY_CONSOLIDATION_LEARNING_AND_WORLDLINE_CONTINUITY_RUNTIME_V0_1.md
tech/NPC_EMBODIED_AFFECT_PERFORMANCE_AND_VOICE_RUNTIME_V0_1.md
tech/NPC_LEARNING_TEACHING_APPRENTICESHIP_AND_SKILL_TRANSMISSION_RUNTIME_V0_1.md
tech/INSTITUTIONAL_COLLECTIVE_COGNITION_AND_PUBLIC_REASON_RUNTIME_V0_1.md
tech/GROUNDED_DIALOGUE_VOICE_AND_GENERATIVE_SAFETY_RUNTIME_V0_1.md
```

The integration contract owns the boundary between Symtropy authority and optional Symthaea proposals. The embodiment contract owns player-facing bodily and affective expression. The life-course contract owns households, care, education, aging, and intergenerational continuity. Runtime specifications own affect integration, skill transmission, institutional reasoning, and grounded language generation.

No generated cognition or dialogue may establish world truth, expose private cognition, bypass consent, or replace deterministic fallback.


## Lived-World Social Consequence

```text
canon/INFORMATION_ECOLOGY_RUMOR_MEDIA_AND_REPUTATION_CONTRACT_V0_1.md
tech/SOCIAL_SIGNAL_RUMOR_REPUTATION_AND_PUBLIC_OPINION_RUNTIME_V0_1.md
canon/HEALTH_TRAUMA_RECOVERY_AND_CARE_CONTRACT_V0_1.md
tech/BODY_HEALTH_TRAUMA_AND_RECOVERY_RUNTIME_V0_1.md
canon/JUSTICE_HARM_ACCOUNTABILITY_AND_REPAIR_CONTRACT_V0_1.md
canon/RELATIONSHIP_INTIMACY_ROMANCE_AND_BOUNDARIES_CONTRACT_V0_1.md
canon/MIGRATION_DIASPORA_BELONGING_AND_INTEGRATION_CONTRACT_V0_1.md
canon/BELIEF_RITUAL_RELIGION_AND_MEANING_CONTRACT_V0_1.md
ops/LIVED_WORLD_SOCIAL_CONSEQUENCE_BENCHMARK_V0_1.md
```

These documents own the cross-cutting social consequences that remain after an action: who learns about it, how bodies and care change, how harm is investigated and repaired, how adult relationships preserve consent, how migration changes both newcomers and host communities, and how belief becomes lived practice without overriding physical truth or the rights floor.

The information ecology owns claim provenance and social transmission. It does not own world truth. The health runtime owns body and care state. It does not own personhood or consent. Justice owns response to harm under evidence and procedure. It does not replace platform moderation. Relationship and belief contracts own boundaries and meaning, not compulsory player participation.

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



## Living Worlds, Built Worlds, and Movement

```text
canon/LIVING_WORLDS_ECOLOGY_AND_TERRAFORMATION_CONTRACT_V0_1.md
tech/BIOSPHERE_TROPHIC_AND_ECOLOGICAL_SIMULATION_RUNTIME_V0_1.md
canon/CONSTRUCTION_REPAIR_AND_STRUCTURAL_TRANSFORMATION_CONTRACT_V0_1.md
tech/STRUCTURAL_INTEGRITY_CONSTRUCTION_AND_DESTRUCTION_RUNTIME_V0_1.md
canon/MOBILITY_VEHICLES_AND_EXPEDITION_OPERATIONS_CONTRACT_V0_1.md
tech/VEHICLE_SPACECRAFT_PHYSICS_AND_OPERATIONS_RUNTIME_V0_1.md
```

The canonical contracts own player-facing ecological, construction, and mobility meaning. Runtime specifications own authoritative state, LOD, physics approximations, transactions, persistence, and multiplayer boundaries.

## First Contact and Xenotechnics

```text
canon/FIRST_CONTACT_TRANSLATION_AND_XENOTECHNICS_CONTRACT_V0_1.md
tech/XENO_SIGNAL_TRANSLATION_AND_CONTACT_STATE_RUNTIME_V0_1.md
lore/ALIEN_TYPES_AND_FIRST_CONTACT_EC.md
lore/NONHUMAN_GAME_THEORY_AND_AGENCY.md
```

The contract and runtime own uncertainty, evidence, contact phases, boundaries, correspondences, and xenotechnology operation. Lore documents own species and encounter possibility space.

## Player Legibility and Causal Explanation

```text
canon/PLAYER_LEGIBILITY_COMPLEXITY_AND_COGNITIVE_LOAD_CONTRACT_V0_1.md
tech/CAUSAL_EXPLANATION_AND_PLAYER_FEEDBACK_RUNTIME_V0_1.md
tech/FIELD_DECK_INTERFACE_AND_INFORMATION_ARCHITECTURE_BIBLE_V0_2.md
```

The canonical contract owns attention and complexity policy. The runtime owns causal traces, explanation queries, warnings, predictions, action previews, and failure reports.

## Procedural and Content Realization

```text
tech/PROCEDURAL_WORLD_SITE_AND_ACTIVITY_GENERATION_PIPELINE_V0_1.md
ops/CONTENT_AUTHORING_VALIDATION_AND_PROVENANCE_STANDARD_V0_1.md
tech/SIMULATION_SCALE_PERFORMANCE_AND_GRACEFUL_DEGRADATION_BUDGETS_V0_1.md
ops/REPRESENTATIVE_BUILD_PERFORMANCE_CONTENT_AND_STRESS_MATRIX_V0_1.md
```

These documents own reproducible generation, content identity and provenance, validation, performance, LOD, degradation, and combined benchmark evidence.

## Acoustic World

```text
vision/ACOUSTIC_CIVILIZATION_AND_DYNAMIC_MUSIC_BIBLE_V0_1.md
tech/AUDIO_ACOUSTICS_AND_MUSIC_STATE_RUNTIME_V0_1.md
```

The supporting bible owns the experiential and cultural audio direction. The runtime owns semantic sound events, acoustics, machine signatures, dynamic music state, accessibility, synchronization, and budgets.

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


# v0.9 Realization Boundary

v0.9 adds system contracts and runtime specifications for ecology, construction, mobility, first contact, procedural content, causal legibility, audio, and shared performance. These documents do not automatically expand the Seedworks content budget. Each system remains bound by the representative proof scenarios and implementation roadmap.


# v1.0 NPC Intelligence Canon Layer

## NPC Experience and Runtime Ownership

```text
vision/NPC_DAILY_LIFE_RELATIONSHIPS_AND_SOCIAL_MEMORY_BIBLE_V0_2.md
tech/NPC_COGNITION_AGENCY_AND_SIMULATION_RUNTIME_V0_1.md
canon/SYMTHAEA_NPC_INTEGRATION_CONTRACT_V0_1.md
tech/SYMTHAEA_NPC_COGNITION_BRIDGE_ARCHITECTURE_V0_1.md
tech/SOCIAL_COGNITION_THEORY_OF_MIND_AND_RELATIONSHIP_RUNTIME_V0_1.md
tech/NPC_MEMORY_CONSOLIDATION_LEARNING_AND_WORLDLINE_CONTINUITY_RUNTIME_V0_1.md
```

The life bible owns lived experience, ordinary routines, relationships, cultural expression, and social memory as player-facing design.

The general NPC runtime owns authoritative agent state, perception, action selection, simulation tiers, and deterministic fallback.

The Symthaea integration contract owns which cognitive proposals are allowed and where game authority remains.

The cognition bridge owns the technical adapter, transaction schemas, budgets, and observability.

The social cognition runtime owns domain trust, second-order beliefs, deception, attachment, group models, grief, and reconciliation.

The memory runtime owns episodic storage, semantic consolidation, forgetting, contradiction, privacy, death continuity, and worldline migration.

## Cognitive Rights and Player Boundaries

```text
canon/NPC_COGNITIVE_RIGHTS_PRIVACY_AND_PLAYER_BOUNDARIES_CONTRACT_V0_1.md
```

This document owns the prohibition on consciousness-score rights, hidden real-player psychological profiling, unbounded cognitive surveillance, and manipulative dependency optimization.

## Evidence and Authoring

```text
ops/FOUR_NPC_BENCHMARK_HOUSEHOLD_PROTOCOL_V0_1.md
ops/NPC_COGNITION_ABLATION_EVALUATION_AND_PLAYTEST_PROGRAM_V0_1.md
ops/NPC_CONTENT_AUTHORING_AND_GROUNDING_STANDARD_V0_1.md
```

These documents own the first representative benchmark, causal ablations, promotion evidence, grounded claim authoring, and generated-language boundaries.

## Conflict Rule

When NPC documents conflict:

1. cognitive rights and player boundaries;
2. Symthaea integration authority contract;
3. general NPC runtime;
4. social and memory runtimes;
5. lived-experience bible;
6. authoring and benchmark specifications;
7. older recipes and experimental notes.

No Symthaea component may silently supersede authoritative gameplay, privacy, moderation, persistence, or action-validation systems.


# v1.1 Authority Notes

- `canon/NPC_EMBODIMENT_AFFECT_AND_NONVERBAL_EXPRESSION_CONTRACT_V0_1.md` owns observable embodiment and affect boundaries.
- `canon/LIFE_COURSE_HOUSEHOLDS_KINSHIP_AND_EDUCATION_CONTRACT_V0_1.md` owns life stages, households, care, and education.
- `tech/INSTITUTIONAL_COLLECTIVE_COGNITION_AND_PUBLIC_REASON_RUNTIME_V0_1.md` owns decomposable institutional reasoning; it does not create a group mind.
- `tech/GROUNDED_DIALOGUE_VOICE_AND_GENERATIVE_SAFETY_RUNTIME_V0_1.md` owns language rendering and claim validation; it never owns game facts.
- `ops/NPC_SOCIAL_ECOLOGY_LONGITUDINAL_BENCHMARK_V0_1.md` and `ops/NPC_INTELLIGENCE_OBSERVABILITY_EVIDENCE_AND_FAILURE_TRIAGE_STANDARD_V0_1.md` own promotion evidence and failure reproducibility.


# v1.2 Authority Notes

- `canon/INFORMATION_ECOLOGY_RUMOR_MEDIA_AND_REPUTATION_CONTRACT_V0_1.md` owns rumor, media, correction, and domain-reputation design boundaries.
- `tech/SOCIAL_SIGNAL_RUMOR_REPUTATION_AND_PUBLIC_OPINION_RUNTIME_V0_1.md` owns deterministic claim propagation and public-opinion aggregation; it never owns hidden world truth.
- `canon/HEALTH_TRAUMA_RECOVERY_AND_CARE_CONTRACT_V0_1.md` and `tech/BODY_HEALTH_TRAUMA_AND_RECOVERY_RUNTIME_V0_1.md` own health, care, accommodation, trauma-linked state, and recovery while preserving privacy and body sovereignty.
- `canon/JUSTICE_HARM_ACCOUNTABILITY_AND_REPAIR_CONTRACT_V0_1.md` owns in-world harm response, evidence, due process, restitution, and repair. Platform abuse remains subject to out-of-world moderation.
- `canon/RELATIONSHIP_INTIMACY_ROMANCE_AND_BOUNDARIES_CONTRACT_V0_1.md` owns adult relationship and consent boundaries. Generated language never owns consent.
- `canon/MIGRATION_DIASPORA_BELONGING_AND_INTEGRATION_CONTRACT_V0_1.md` owns displacement, arrival, belonging, diaspora, and worldline migration as lived continuity rather than population arithmetic.
- `canon/BELIEF_RITUAL_RELIGION_AND_MEANING_CONTRACT_V0_1.md` owns fictional belief and ritual boundaries while `lore/BELIEF_SYSTEMS_AND_CULTS.md` remains a supporting catalog.
- `ops/LIVED_WORLD_SOCIAL_CONSEQUENCE_BENCHMARK_V0_1.md` owns integrated promotion evidence for the v1.2 campaign.
