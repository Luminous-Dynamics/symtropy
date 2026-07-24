---
title: Canon Registry and Document Governance
scope: documentation lifecycle, canonical spine, ownership, supersession, registry
  policy
owner: documentation/design
version: 1.6
status: superseded
supersedes:
- CANON_REGISTRY_AND_DOCUMENT_GOVERNANCE_V1_5.md
related:
- ../ops/DOCUMENT_REGISTRY.json
- ../ops/DOCUMENT_METADATA_MIGRATION_AND_CONSOLIDATION_PLAN_V0_2.md
- ../ops/V1_7_HISTORICAL_TEXTURE_CAMPAIGN.md
- ../ops/V1_8_PLAYABLE_HISTORY_CAMPAIGN.md
- ../ops/V2_0_IRIS_MACHINE_INTIMACY_AND_UNFLINCHING_HUMAN_REALITY_CAMPAIGN.md
- ../ops/IRIS_UNFLINCHING_REALITY_AND_EXPLOITATION_SAFETY_BENCHMARK_V0_1.md
superseded_by: CANON_REGISTRY_AND_DOCUMENT_GOVERNANCE_V1_7.md
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
canon/SYSTEM_INTERACTION_AND_DEPENDENCY_MAP_V1_1.md
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


# v1.3 Civilization Continuity Spine

## Succession, Administration, and Public Service

```text
canon/CIVIC_SUCCESSION_PUBLIC_SERVICE_AND_INSTITUTIONAL_CONTINUITY_CONTRACT_V0_1.md
tech/PUBLIC_ADMINISTRATION_SUCCESSION_AND_CORRUPTION_RUNTIME_V0_1.md
```

The canonical contract owns office boundaries, leadership transition, public-service continuity, corruption constraints, and the requirement that institutions outlive individual leaders. The runtime owns scoped authority tokens, succession transactions, qualification coverage, procedures, procurement, capture pressure, audit, and administrative levels of detail.

## Archives, Historiography, and Heritage

```text
canon/ARCHIVES_HISTORIOGRAPHY_HERITAGE_AND_COLLECTIVE_MEMORY_CONTRACT_V0_1.md
tech/KNOWLEDGE_ARCHIVE_AND_HISTORICAL_EVIDENCE_RUNTIME_V0_1.md
```

These documents own evidence provenance, archival custody, correction without erasure, contested history, privacy, heritage, memorialization, reparative obligations, and worldline ancestry. Chronicle records remain durable events; they do not become omniscient historical interpretation.

## Preparedness, Emergency Continuity, and Recovery

```text
canon/DISASTER_PREPAREDNESS_CONTINUITY_OF_OPERATIONS_AND_RECOVERY_CONTRACT_V0_1.md
tech/EMERGENCY_COORDINATION_EVACUATION_AND_RECOVERY_RUNTIME_V0_1.md
```

The contract owns disaster meaning, preparedness, evacuation rights, continuity floors, relief, recovery, and disaster justice. The runtime owns forecasts, warnings, evacuation assignments, shelters, relief flows, incident roles, recovery projects, and after-action learning. It does not replace the physical hazard simulation, health runtime, vehicle runtime, or governance authority checks.

## Generational and Cultural Continuity

```text
canon/CULTURAL_EVOLUTION_LANGUAGE_AND_INTERGENERATIONAL_TRANSMISSION_CONTRACT_V0_1.md
tech/DEMOGRAPHY_GENERATIONAL_AND_CULTURAL_EVOLUTION_RUNTIME_V0_1.md
ops/FIFTY_YEAR_CIVILIZATION_CONTINUITY_BENCHMARK_V0_1.md
```

The contract owns cultural transmission, language, subculture, assimilation pressure, revival, and player-created cultural adoption. The runtime owns named/cohort conservation, life-stage transitions, skill reproduction, language-community state, cultural-practice lineages, and demographic LOD. The benchmark owns the integrated fifty-year proof.

All v1.3 systems remain design-complete but implementation-unassessed until their named evidence fixtures exist.

# v1.4 Planetary Society Spine

## Federation and Shared Sovereignty

```text
canon/PLANETARY_FEDERATION_SUBSIDIARITY_AND_SHARED_SOVEREIGNTY_CONTRACT_V0_1.md
tech/INTERSETTLEMENT_TREATY_STANDARDS_AND_MUTUAL_AID_RUNTIME_V0_1.md
```

The canonical contract owns subsidiarity, membership, representation, planetary public goods, contribution fairness, rights floors, shared sovereignty, and federal emergency limits. The runtime owns treaty clauses, obligations, aid reservations, standards, mandates, jurisdiction cases, contribution accounts, and authority return.

## Planetary Networks and Exchange

```text
tech/PLANETARY_INFRASTRUCTURE_NETWORKS_AND_CORRIDOR_RUNTIME_V0_1.md
tech/INTERREGIONAL_TRADE_CUSTOMS_CURRENCY_AND_SANCTIONS_RUNTIME_V0_1.md
```

The network runtime owns conserved interregional flows, corridor condition, dependency, access, ecological passage, maintenance, islanding, and orbital-surface connection. The trade runtime owns cargo manifests, delivery contracts, customs, clearing, currencies, sanctions, smuggling, and strategic dependency. Neither runtime may teleport matter or treat a currency balance as inventory.

## Climate and Planetary Ecology

```text
canon/PLANETARY_CLIMATE_ADAPTATION_AND_ECOLOGICAL_COORDINATION_CONTRACT_V0_1.md
```

This contract owns cross-boundary environmental authority, adaptation burden, historical responsibility, managed retreat, planetary intervention limits, ecological standing, monitoring commons, and environmental treaty design. It extends rather than supersedes the living-worlds ecology contract.

## Orbital-Planetary Interface

```text
canon/ORBITAL_PLANETARY_INTERFACE_AND_SPACEPORT_GOVERNANCE_CONTRACT_V0_1.md
```

This contract owns launch access, traffic coordination, dock and habitat intake, rescue, quarantine, spaceport burden, dock labor, surface-orbit migration, and shared orbital-planetary institutions. Vehicle physics and orbital propagation remain owned by their technical runtimes.

## Secession and Federation Forks

```text
canon/SECESSION_FEDERATION_FORK_AND_CIVIL_CONFLICT_PREVENTION_CONTRACT_V0_1.md
```

This contract owns lawful exit, association, dissolution, referendum integrity, minority protection, shared-asset transition, civil-conflict prevention, and federation worldline forks. It never overrides multiplayer consent or platform moderation.

## Planetary Contact Order

```text
canon/PLANETARY_DIPLOMACY_NONHUMAN_SOVEREIGNTY_AND_CONTACT_ORDER_CONTRACT_V0_1.md
```

This contract owns scoped recognition, contact authority, plural representation, nonhuman sovereignty, territory translation, noncontact, quarantine, xenotechnology diplomacy, and planetary contact orders. First-contact evidence and translation mechanics remain owned by the first-contact contract and xeno runtime.

## Planetary Evidence

```text
ops/CENTURY_PLANETARY_FEDERATION_BENCHMARK_V0_1.md
```

The century benchmark owns integrated promotion evidence for planetary-scale coordination. All v1.4 systems remain design-complete but implementation-unassessed until their schemas, replays, performance traces, save migrations, and human comprehension results exist.

## v1.4 Conflict Rule

When planetary documents conflict:

1. Game Constitution and rights boundaries.
2. Federation, climate, secession, orbital-interface, and contact-order canonical contracts.
3. Existing war, economy, worldline, ecology, migration, justice, and multiplayer-safety contracts.
4. Planetary and interregional implementation specifications.
5. The century benchmark and roadmap.
6. Supporting planetary, spaceport, naval, culture, and world-type bibles.

Planetary scope may not silently enter Seedworks milestone scope. The local and regional causal spine remains the production gate.


## Interplanetary Civilization and Distributed Distance

```text
canon/INTERPLANETARY_CIVILIZATION_LATENCY_AND_DISTRIBUTED_SOVEREIGNTY_CONTRACT_V0_1.md
tech/LIGHT_DELAY_COMMUNICATION_TIMEKEEPING_AND_ASYNC_COORDINATION_RUNTIME_V0_1.md
canon/CLOSED_LOOP_HABITATS_GENERATION_SHIPS_AND_SETTLEMENT_CONTINUITY_CONTRACT_V0_1.md
tech/HABITAT_METABOLISM_LIFE_SUPPORT_AND_POPULATION_RUNTIME_V0_1.md
tech/DEEP_SPACE_LOGISTICS_TRANSFER_WINDOWS_RESCUE_AND_SALVAGE_RUNTIME_V0_1.md
canon/INTERPLANETARY_SECURITY_FLEETS_BLOCKADE_AND_RULES_OF_ENGAGEMENT_CONTRACT_V0_1.md
canon/SETTLEMENT_AUTONOMY_COLONIZATION_ETHICS_AND_NONEXTRACTIVE_EXPANSION_CONTRACT_V0_1.md
tech/INTERPLANETARY_TRADE_FINANCE_CUSTODY_AND_CONTRACT_LATENCY_RUNTIME_V0_1.md
ops/TWO_CENTURY_SOLAR_SYSTEM_CIVILIZATION_BENCHMARK_V0_1.md
```

The interplanetary contract owns the player-facing meaning of civilization under communication delay and distributed sovereignty. The communication runtime owns message delivery, clocks, causal frontiers, and asynchronous civic state. Habitat documents own closed-loop survival, household continuity, generation-ship renewal, and physical metabolism. Logistics owns conserved transit, rescue, and salvage. The security contract owns fleet purpose, protected systems, boarding, blockade, surrender, and demobilization. The settlement-autonomy contract owns off-world claims, labor, existing life, expansion, and decolonization. The economic runtime owns delayed contracts and settlement without creating physical assets.

No interplanetary system may read current remote state without delivered evidence, operate local life support without valid delegated authority, duplicate cargo or claims across worldline forks, or treat descendants as permanently bound by founder missions.


## Interstellar Threshold and Deep-Time Civilization

```text
canon/INTERSTELLAR_CIVILIZATION_RELATIVISTIC_DISTANCE_AND_LOCAL_SOVEREIGNTY_CONTRACT_V0_1.md
tech/RELATIVISTIC_NAVIGATION_TIME_DILATION_AND_CAUSAL_COORDINATION_RUNTIME_V0_1.md
canon/AUTONOMOUS_PROBES_ARKS_AND_MISSION_INHERITANCE_CONTRACT_V0_1.md
tech/DEEP_TIME_ARK_ECOLOGY_POPULATION_AND_CULTURAL_DRIFT_RUNTIME_V0_1.md
canon/INTERSTELLAR_CONTACT_NONINTERFERENCE_AND_LONG_VOW_DIPLOMACY_CONTRACT_V0_1.md
tech/STELLAR_RELAY_SIGNAL_PROVENANCE_AND_CONTACT_LATENCY_RUNTIME_V0_1.md
canon/INTERSTELLAR_RESCUE_ABANDONMENT_AND_IRRECOVERABLE_LOSS_CONTRACT_V0_1.md
tech/INFRASTRUCTURE_LOCKED_INTERSTELLAR_TRANSIT_GATE_AUTHORITY_AND_FAILURE_RUNTIME_V0_1.md
ops/THOUSAND_YEAR_INTERSTELLAR_CIVILIZATION_BENCHMARK_V0_1.md
```

These documents own the horizon beyond ordinary solar-system civilization: causal isolation, relativistic chronology, autonomous missions, descendant autonomy, long-delay contact, rescue impossibility, precursor evidence, and infrastructure-locked transit.

The interstellar runtime may not create remote omniscience, casual ship-mounted FTL, permanent founder authority, mission ownership of descendants, unbounded replication, automatic precursor legitimacy, or duplicate unique entities through gates or worldline forks.

The legacy `tech/Symtropy Design Doc - Infrastructure-Locked Interstellar Transit.md` is superseded and retained only for provenance.


## Historical Texture and Authored World Memory

```text
lore/MEGACORPORATE_EMPIRES_AND_POST_CORPORATE_SUCCESSOR_STATES_BIBLE_V0_1.md
lore/CORPORATE_CIVILIZATION_ATLAS_V0_1.md
lore/GREAT_CONFLICTS_CATASTROPHES_AND_HISTORICAL_SCARS_ATLAS_V0_1.md
lore/LEGENDARY_CITIES_HABITATS_AND_WORLD_REGIONS_ATLAS_V0_1.md
lore/DIASPORAS_STATELESS_PEOPLES_AND_MOBILE_CIVILIZATIONS_ATLAS_V0_1.md
lore/HISTORICAL_FIGURES_FOUNDERS_WITNESSES_AND_CONTESTED_LEGACIES_ATLAS_V0_1.md
lore/SHADOW_CIVILIZATIONS_INFORMAL_NETWORKS_AND_OUTLAW_COMMONS_ATLAS_V0_1.md
lore/ARTISTIC_MOVEMENTS_MEDIA_AND_EVERYDAY_MATERIAL_CULTURE_ATLAS_V0_1.md
lore/CONTESTED_HISTORIES_ARCHIVAL_SCHOOLS_AND_PUBLIC_MEMORY_ATLAS_V0_1.md
lore/HISTORICAL_TEXTURE_TIMELINE_AND_RELATIONSHIP_MAP_V0_1.md
tech/HISTORICAL_TEXTURE_LORE_PROVENANCE_AND_GENERATION_RUNTIME_V0_1.md
ops/HISTORICAL_TEXTURE_WORLD_COHERENCE_BENCHMARK_V0_1.md
```

These documents own named historical texture: corporate civilizations and successors, conflicts and catastrophes, famous places, diasporas, historical people, informal institutions, art movements, competing histories, causal lore generation, and coherence validation.

They are supporting lore and implementation specifications beneath the canonical economy, rights, history, migration, culture, worldline, and authority contracts. Named exemplars may vary or be absent across worldlines and may not silently become universal factions.


## Playable History and Regional Campaign Content

```text
canon/HISTORICAL_CONTENT_AND_PLAYABLE_CAMPAIGN_CONTRACT_V0_1.md
ops/PLAYABLE_HISTORY_CONTENT_PACKET_STANDARD_V0_1.md
lore/REGIONAL_CAMPAIGN_ANTHOLOGY_V0_1.md
tech/HISTORICAL_PRESSURE_FACTION_CLOCK_AND_CAMPAIGN_STATE_RUNTIME_V0_1.md
tech/DISCOVERABLE_HISTORY_ARCHIVE_RUIN_AND_ENVIRONMENTAL_STORYTELLING_RUNTIME_V0_1.md
lore/CORPORATE_EMPIRE_PLAYABLE_CAMPAIGN_PACKETS_V0_1.md
ops/LEGENDARY_DESTINATION_PRODUCTION_PACKS_V0_1.md
lore/DIASPORA_SHADOW_NETWORK_AND_MOBILE_CIVILIZATION_CAMPAIGN_PACKETS_V0_1.md
lore/CULTURAL_SEASONS_FESTIVALS_ART_AND_EVERYDAY_LIFE_CONTENT_BIBLE_V0_1.md
tech/PLAYABLE_HISTORY_CONTENT_COMPILER_AND_WORLDLINE_VARIATION_RUNTIME_V0_1.md
ops/PLAYABLE_HISTORY_REGIONAL_BENCHMARK_V0_1.md
ops/PLAYABLE_HISTORY_IMPLEMENTATION_TICKET_BACKLOG_V0_1.md
```

The campaign contract owns how historical lore becomes playable without turning simulation into a fixed quest tree. The packet standard owns authoring and production completeness. Runtimes own pressure, faction initiative, evidence discovery, deterministic compilation, worldline variation, and persistence.

The atlases and packets are supporting authored content beneath the canonical economy, rights, migration, culture, archive, worldline, and authority contracts. A named campaign may vary, transform, or be absent across worldlines.

No campaign text, dialogue renderer, or content compiler may directly establish authoritative world state.


# v1.9 Lore De-Polishing and Human Mess Authority

The v1.9 documents are supporting lore and production validation. They do not override the Game Constitution, rights floor, authoritative simulation, source-chain rules, or evidence boundaries.

They own the following lore-quality requirements:

```text
major entities carry socially grounded alternate names
corporate histories are structurally uneven rather than template-symmetric
ordinary voices and artifacts exist for practical in-world reasons
human motives include vanity, faith, rivalry, boredom, taste, and miscalculation
popular culture includes low-prestige and widely loved bad art
historical events preserve noise, false lessons, and administrative residue
cultures contain minorities, mixed households, reactionaries, and nonparticipants
historical figures retain ordinary lives and privacy boundaries
nonhuman agents possess everyday activity not organized around human contact
flagship lore passes blind tone and local-specificity review
```

Primary documents:

```text
lore/NAMING_PLURALITY_LOCAL_REGISTER_AND_LINGUISTIC_HISTORY_ATLAS_V0_1.md
lore/CORPORATE_ASYMMETRY_MERGERS_BRANDS_AND_INSTITUTIONAL_FAILURES_ATLAS_V0_1.md
lore/GROUND_LEVEL_DIEGETIC_ARTIFACTS_AND_ORDINARY_VOICES_ANTHOLOGY_V0_1.md
lore/HUMAN_IRRATIONALITY_STATUS_RIVALRY_REVENGE_AND_MISCALCULATION_ATLAS_V0_1.md
lore/POPULAR_BAD_CULTURE_SPORT_KITSCH_CELEBRITY_AND_TRASH_MEDIA_ATLAS_V0_1.md
lore/HISTORICAL_NOISE_FALSE_LESSONS_FORGOTTEN_EVENTS_AND_BUREAUCRATIC_ACCIDENTS_ATLAS_V0_1.md
lore/CULTURAL_CROSS_CONTAMINATION_MINORITIES_MIXED_HOUSEHOLDS_AND_ORDINARY_NONPARTICIPANTS_ATLAS_V0_1.md
lore/HISTORICAL_FIGURE_PRIVATE_LIVES_HABITS_DEBTS_RIVALRIES_AND_UNRELIABLE_LEGACY_ADDENDUM_V0_1.md
lore/NONHUMAN_EVERYDAY_LIFE_HUMOR_PLAY_STATUS_AND_DOMESTICITY_ATLAS_V0_1.md
ops/LORE_TONE_VARIANCE_HUMAN_MESS_AND_LOCAL_SPECIFICITY_BENCHMARK_V0_1.md
```

The direct normative prose belongs in contracts and standards. Lore-facing entries should prefer physical evidence, local language, biased artifacts, ordinary practice, and unresolved historical interpretation.

The v1.9 benchmark is a production gate for flagship regions. It is not a style police mechanism for every sentence, and it may not erase accessibility, rights, or causal clarity in pursuit of tonal difference.

# v2.0 IRIS and Unflinching-Reality Authority

## IRIS

```text
canon/IRIS_FIELD_DECK_COGNITIVE_COMPANION_AND_AUTHORITY_CONTRACT_V0_1.md
tech/IRIS_COGNITION_MEMORY_VOICE_AND_SOURCE_CHAIN_RUNTIME_V0_1.md
lore/IRIS_LINEAGES_PUBLIC_MEMORY_AND_FIELD_DECK_CULTURES_ATLAS_V0_1.md
```

The canonical contract owns player-facing identity, knowledge, advice, relationship, privacy, fork, and authority boundaries. The runtime owns implementation state and scheduling. The atlas is supporting historical texture and cannot grant universal personhood or central control.

## Bodily, Intimate, and Psychological Reality

```text
canon/PRIVATE_BODILY_LIFE_HYGIENE_DISABILITY_AGING_AND_INTIMATE_CARE_CONTRACT_V0_1.md
canon/ADULT_SEXUALITY_INTIMACY_SEX_WORK_REPRODUCTIVE_AUTONOMY_AND_COERCION_CONTRACT_V0_1.md
tech/ADDICTION_INTOXICATION_HARM_REDUCTION_AND_PHARMACEUTICAL_DEPENDENCE_RUNTIME_V0_1.md
canon/DOMESTIC_COERCIVE_CONTROL_ABUSE_ESCAPE_AND_SURVIVOR_AGENCY_CONTRACT_V0_1.md
canon/WAR_ATROCITY_DETENTION_DISPLACEMENT_AND_POSTWAR_ACCOUNTABILITY_CONTRACT_V0_1.md
canon/MENTAL_HEALTH_FAITH_GRIEF_AND_AMBIGUOUS_EXPERIENCE_CONTRACT_V0_1.md
```

These documents extend, but do not replace, the existing health, relationship, justice, war, migration, belief, and rights contracts.

## Mature Content Production and Proof

```text
lore/UNFLINCHING_ORDINARY_LIVES_SCENE_AND_ARTIFACT_ANTHOLOGY_V0_1.md
ops/MATURE_CONTENT_AUTHORING_REVIEW_AND_PLAYER_CONTROL_STANDARD_V0_1.md
ops/IRIS_UNFLINCHING_REALITY_AND_EXPLOITATION_SAFETY_BENCHMARK_V0_1.md
ops/V2_0_IRIS_MACHINE_INTIMACY_AND_UNFLINCHING_HUMAN_REALITY_CAMPAIGN.md
```

The anthology is supporting inspiration. The authoring standard and benchmark own production controls and promotion evidence.

## Conflict Rule

When mature lore conflicts with privacy, consent, adulthood, player control, accessibility, or non-exploitation boundaries, the boundary wins.

When IRIS dialogue conflicts with authoritative simulation state, authoritative state wins and the discrepancy becomes a traceable IRIS failure.
