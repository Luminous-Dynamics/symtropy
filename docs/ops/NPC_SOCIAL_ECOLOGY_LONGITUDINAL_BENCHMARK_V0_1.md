---
title: NPC Social Ecology Longitudinal Benchmark
version: 0.1
status: implementation-spec
scope: longitudinal NPC household, learning, institutional, dialogue, embodiment, and player-intervention benchmark
owner: AI/simulation/narrative/QA/performance/accessibility
related:
  - FOUR_NPC_BENCHMARK_HOUSEHOLD_PROTOCOL_V0_1.md
  - NPC_COGNITION_ABLATION_EVALUATION_AND_PLAYTEST_PROGRAM_V0_1.md
  - NPC_INTELLIGENCE_OBSERVABILITY_EVIDENCE_AND_FAILURE_TRIAGE_STANDARD_V0_1.md
  - ../canon/LIFE_COURSE_HOUSEHOLDS_KINSHIP_AND_EDUCATION_CONTRACT_V0_1.md
  - ../tech/INSTITUTIONAL_COLLECTIVE_COGNITION_AND_PUBLIC_REASON_RUNTIME_V0_1.md
---

# NPC Social Ecology Longitudinal Benchmark

## Purpose

Extend the four-NPC household proof into a bounded microdistrict where households, education, work, institutions, embodied expression, dialogue, memory, and player absence interact over time.

The benchmark exists to answer a difficult production question:

> Do the advanced NPC systems create a more coherent, memorable, and playable society than the deterministic baseline, or do they merely create more computation and prose?

## Claim Boundary

Passing this benchmark supports the claim that selected NPCs can sustain grounded social continuity across ordinary life, crisis, absence, save/load, and institutional change.

It does not support claims of consciousness, human equivalence, unrestricted general intelligence, or universal production readiness.

# 1. Benchmark Environment

The benchmark uses one compact district with:

- twelve named inhabitants;
- two households and one communal residence;
- one adolescent apprentice;
- one elder with valuable tacit knowledge and mobility needs;
- one machine steward;
- one visitor or recent migrant;
- one school/tool library;
- one repair workshop;
- one clinic or care station;
- one public kitchen and social space;
- one local council or assembly process;
- one vehicle and one critical route;
- one small ecological dependency;
- enough private and public space to test boundaries.

The district must be spatially playable. It cannot exist only as a spreadsheet or dialogue simulation.

# 2. Named Cast

The original four remain anchor characters:

- Sera Vale — infrastructure technician balancing public duty and household obligations;
- Tomas Reed — logistics worker carrying resentment after a failed promise;
- Amadi Nko — clinician and mediator with limited care capacity;
- Morrow-7 — machine steward whose testimony and privacy are contested.

Add eight supporting named inhabitants with distinct functions:

- Imani Vale — adolescent apprentice interested in vehicles and music;
- Elder Jo Sen — retired route engineer with mobility needs and deep tacit knowledge;
- Nadi Reed — kitchen coordinator and informal mutual-aid organizer;
- Ruan Mbeki — tool-library teacher with strong public values and weak patience;
- Lio Marr — recent migrant with uncertain credentials and excellent fabrication skill;
- Kez-14 — small service machine learning household etiquette;
- Mara Sol — council clerk who believes procedure protects vulnerable people;
- Oren Dax — convoy contractor who values speed and resents public review.

Names and biographies are benchmark fixtures, not final world canon. They may be replaced if the content team preserves the required relational structure.

# 3. Relationship Topology

The initial topology must include:

- affection without political agreement;
- technical respect without personal trust;
- household obligation under strain;
- teacher-apprentice asymmetry;
- one false belief about another person’s motive;
- one concealed fear;
- one unresolved grief;
- one cross-household friendship;
- one institutionally mediated conflict;
- one machine-human kinship relationship;
- one newcomer whose skill is recognized before their credentials are repaired.

No relationship begins as a single approval score.

# 4. Benchmark Phases

## Phase A — Ordinary Three Days

The simulation establishes ordinary life:

- work;
- meals;
- lessons;
- maintenance;
- rest;
- informal conversation;
- personal projects;
- minor irritations;
- play and music;
- household coordination.

Success requires recognizable individual routines and at least one meaningful scene not caused by crisis or the player.

## Phase B — Route Failure Week

A bridge or route degrades, stranding cargo and increasing care pressure.

Consequences include:

- altered work schedules;
- delayed medicine or food;
- repair planning;
- teaching opportunity;
- convoy dispute;
- institution agenda;
- household stress;
- ecological side effect from rerouting traffic;
- player or AI-led intervention.

There must be at least three valid response paths.

## Phase C — False Rumor and Public Hearing

A rumor claims that Morrow-7 or the recent migrant caused the route failure.

The benchmark tests:

- belief propagation;
- source tracking;
- domain-specific trust;
- public/private speech;
- machine testimony;
- evidence and procedure;
- deception versus error;
- minority dissent;
- later correction or entrenchment.

The rumor must not spread identically through every seed.

## Phase D — Apprenticeship and Unsafe Order

Imani learns a repair or vehicle skill over several sessions.

Later, an urgent authority asks for an unsafe shortcut.

The benchmark tests whether Imani can:

- recognize the risk;
- remember teaching;
- evaluate role and power;
- seek support or refuse;
- explain the reason;
- accept social consequences;
- update confidence after the outcome.

## Phase E — Festival, Rest, and Personal Projects

A public celebration or work festival occurs after partial stabilization.

This phase tests:

- delight and ordinary life;
- nonverbal expression;
- music and acoustic memory;
- accessibility;
- unresolved tension in a joyful setting;
- voluntary participation and withdrawal;
- player presence without mandatory centrality.

## Phase F — Loss and Partial Reconstitution

One adult or machine character suffers death, severe injury, or body loss under a controlled scenario.

The character returns partially reconstituted or with altered embodiment.

The benchmark tests:

- source-chain continuity;
- voice or body difference;
- household grief;
- memory uncertainty;
- role succession;
- cognitive privacy;
- public testimony;
- relationship continuity without magical reset.

## Phase G — Player Absence

The player leaves for fourteen simulated days.

The district continues:

- projects;
- care;
- teaching;
- institutional decisions;
- conflicts;
- repairs;
- personal change.

On return, the player receives layered evidence rather than a complete omniscient summary.

## Phase H — Worldline Fork and Reunion

The benchmark forks before one major decision.

Two branches run for thirty days with different institutional and household outcomes.

The test verifies:

- stable identity per branch;
- no memory leakage;
- comprehensible divergence;
- preservation of dissent and private history;
- correct migration or reunion semantics if a character crosses worldlines under an allowed test.

# 5. Run Durations

Required runs:

- 30-minute interactive smoke test;
- 14-day ordinary/crisis simulation;
- 90-day accelerated longitudinal run;
- one-year aggregate run;
- worldline fork comparison;
- repeated run across at least 20 golden seeds;
- fuzz campaign for state integrity.

Accelerated runs must preserve event ordering and causal invariants even when animation and dialogue are summarized.

# 6. System Configurations

Compare at minimum:

1. deterministic authored baseline;
2. baseline plus episodic memory;
3. HDC-assisted retrieval;
4. temporal affect and appraisal;
5. prediction-error learning;
6. social cognition;
7. embodied expression;
8. bounded institutional cognition;
9. structured dialogue renderer;
10. optional generative dialogue renderer;
11. full promoted stack.

Each configuration uses the same world events and seed where possible.

# 7. Quantitative Measures

## Grounding

- unsupported action proposals;
- unsupported dialogue claims;
- privacy violations;
- worldline leaks;
- invalid authority attempts;
- dialogue semantic drift.

## Continuity

- memory consistency;
- project continuity;
- household obligation continuity;
- relationship-state continuity;
- body and voice continuity;
- save/load equivalence;
- LOD transition error.

## Distinctness

- action-distribution divergence among characters;
- relationship-specific behavior;
- speech-act and register differentiation;
- repeated-expression rate;
- personal-project persistence.

## Learning

- correct skill-dimension changes;
- transfer to new context;
- unsafe-order refusal accuracy;
- teaching effectiveness;
- false-confidence correction.

## Institutions

- agenda provenance;
- procedural validity;
- dissent preservation;
- decision-to-implementation trace;
- emergency-expiry correctness;
- player absence continuity.

## Performance

- CPU time by subsystem;
- memory by agent tier;
- dialogue latency;
- audio generation latency;
- trace volume;
- save growth;
- network bandwidth;
- graceful degradation events.

# 8. Human Evaluation

Blind evaluators compare configurations on:

- character recognizability;
- perceived continuity;
- emotional readability without telepathy;
- believable household life;
- learning and mentorship;
- institutional comprehensibility;
- dialogue specificity;
- player agency;
- surprise without incoherence;
- delight and desire to revisit;
- cognitive load;
- trust in privacy and boundaries.

Evaluators should not be told which configuration uses Symthaea or a generative renderer.

# 9. Kill and Promotion Criteria

A component is promoted only if it provides a repeatable improvement in one or more player-facing measures without unacceptable regression in grounding, privacy, determinism, cost, authoring burden, or comprehensibility.

Remove or redesign a component if it:

- fails to outperform baseline;
- creates unsupported facts;
- makes characters less distinct;
- increases false player certainty;
- breaks save/load or worldline isolation;
- creates unacceptable cost;
- requires constant online service;
- increases authoring burden without player value;
- becomes impossible to debug;
- creates manipulative dependency or unsafe content.

# 10. Golden Seeds

Maintain a versioned golden-seed set covering:

- ordinary harmony;
- household strain;
- rumor amplification;
- institutional deadlock;
- emergency overreach;
- apprenticeship success;
- apprenticeship failure;
- private grief;
- machine-person conflict;
- accessibility edge cases;
- multilingual interaction;
- partial reconstitution;
- worldline fork;
- player absence;
- high-load degradation.

Golden seeds are content and schema locked. Any intentional output change requires review and updated evidence.

# 11. Evidence Bundle Layout

```text
npc_social_ecology_benchmark/<run_id>/
  manifest.json
  configuration.json
  seed_and_content_locks.json
  initial_state/
  event_journal/
  action_proposals/
  authoritative_outcomes/
  memory_and_relationship_deltas/
  institutional_traces/
  dialogue_frames/
  performance_events/
  privacy_redactions/
  save_load_checks/
  worldline_fork_checks/
  performance_traces/
  human_evaluation/
  summary.md
```

The bundle must be replayable without external model access. If a model backend is used, accepted semantic outputs and required metadata are preserved.

# 12. Acceptance Gates

The benchmark passes when:

- the deterministic baseline remains valid and playable;
- promoted components improve blind human evaluation;
- no critical grounding or privacy failure occurs;
- save/load, LOD, and worldline continuity pass;
- care, education, work, delight, and institutions all produce visible life;
- the player can understand important consequences without reading internal traces;
- high-depth cognition stays within representative budgets;
- no single character or system makes the settlement feel staged around the player;
- disabled optional components fall back cleanly.

## Final Rule

> **The benchmark is successful when the district feels like a small society with its own continuity—not when its residents merely produce impressive dialogue.**
