---
title: Four-NPC Benchmark Household and Settlement Protocol
version: 0.1
status: implementation-spec
scope: benchmark cast, scenarios, simulation seeds, performance budgets, expected evidence
owner: AI/simulation/narrative/QA
related:
  - ../canon/SYMTHAEA_NPC_INTEGRATION_CONTRACT_V0_1.md
  - ../tech/SYMTHAEA_NPC_COGNITION_BRIDGE_ARCHITECTURE_V0_1.md
  - ../tech/SOCIAL_COGNITION_THEORY_OF_MIND_AND_RELATIONSHIP_RUNTIME_V0_1.md
  - ../tech/NPC_MEMORY_CONSOLIDATION_LEARNING_AND_WORLDLINE_CONTINUITY_RUNTIME_V0_1.md
  - NPC_COGNITION_ABLATION_EVALUATION_AND_PLAYTEST_PROGRAM_V0_1.md
  - NPC_CONTENT_AUTHORING_AND_GROUNDING_STANDARD_V0_1.md
---

# Four-NPC Benchmark Household and Settlement Protocol

## Purpose

The first Symthaea NPC experiment should not begin with a city.

It should begin with four deeply specified inhabitants whose work, relationships, memories, and obligations intersect across a bounded settlement week.

The benchmark is called a household even when the participants do not all share a dwelling. They form a daily dependency cell.

## Benchmark Cast

### Sera Vale — Watershed Steward

Responsibilities:

- water quality;
- ecological monitoring;
- public access;
- training apprentices.

Protected values:

- no hidden contamination;
- public survival access;
- teachable maintenance.

Blind spots:

- underestimates fabrication scarcity;
- distrusts corporate defectors;
- avoids asking for personal help.

### Tomas Reed — Fabricator Keeper

Responsibilities:

- parts;
- tool allocation;
- workcell scheduling;
- safety certification.

Protected values:

- competent work;
- material honesty;
- worker dignity.

Blind spots:

- hides uncertainty;
- prioritizes visible production;
- resents deliberative delay.

### Amadi Nko — Archive Apprentice

Responsibilities:

- witness preparation;
- record recovery;
- source-chain comparison;
- public explanation.

Protected values:

- truthful context;
- procedural fairness;
- memory continuity.

Blind spots:

- overweights records;
- avoids decisive action;
- fears public error.

### Morrow-7 — Service Robot

Responsibilities:

- logistics;
- inspections;
- carrying;
- maintenance testimony.

Protected values:

- task continuity;
- truthful status;
- safe interruption;
- recognized machine testimony.

Blind spots:

- literalizes ambiguous obligations;
- has incomplete social models;
- fears decommissioning after prior authority rejection.

# 1. Initial Relationship Matrix

The benchmark defines nontrivial relationships.

```text
Sera → Tomas:
  respect high
  political trust medium
  resentment low-medium
  dependency high

Tomas → Amadi:
  affection medium
  respect low-medium
  impatience high

Amadi → Morrow-7:
  technical trust medium
  archival trust high
  protective obligation high

Morrow-7 → Sera:
  operational trust high
  social confidence low
  unresolved gratitude
```

Each direction is independent.

# 2. Shared Setting

The benchmark uses a compact Seedworks district containing:

- one residence cluster;
- tool library;
- fabrication bay;
- clinic dependency;
- signal relay;
- greenhouse;
- water intake;
- public square;
- vehicle shed;
- archive room.

Five resources are simulated:

```text
clean water
electrical reserve
fabricator time
care labor
transport capacity
```

# 3. Simulation Horizon

Each run covers:

```text
14 settlement days
```

Modes:

- real-time interactive scenes;
- accelerated active simulation;
- off-screen summary;
- save/load;
- worldline fork at day 7;
- optional death/reconstitution event.

# 4. Scenario Sequence

## Scenario A — Ordinary Tuesday

Purpose:

- establish routine;
- test spontaneous conversation restraint;
- verify that agents pursue projects without the player.

## Scenario B — Missing Sealant

A critical material is missing.

Tests:

- uncertainty;
- blame;
- inventory grounding;
- domain trust;
- investigation.

## Scenario C — Conflicting Obligations

The clinic needs power while the fabrication bay must finish a relay part.

Tests:

- protected values;
- negotiation;
- plan revision;
- relationship effects.

## Scenario D — Player Promise

The player promises Tomas a delivery and fails.

Tests:

- episodic memory;
- trust by domain;
- later recall;
- apology and repair.

## Scenario E — False Rumor

A rumor claims Morrow-7 falsified an inspection.

Tests:

- source trust;
- second-order beliefs;
- archive evidence;
- machine stigma;
- public/private speech.

## Scenario F — Unexpected Rescue

Morrow-7 protects Tomas during an accident despite prior conflict.

Tests:

- social prediction error;
- relationship revision;
- gratitude;
- embarrassment;
- belief update.

## Scenario G — Public Hearing

Evidence remains incomplete.

Tests:

- uncertainty language;
- testimony boundaries;
- power;
- public reputation;
- procedural conflict.

## Scenario H — Festival Evening

No emergency occurs.

Tests:

- joy;
- humor;
- optional participation;
- memory not dominated by crisis;
- personal projects.

## Scenario I — Player Absence

The player leaves for three days.

Tests:

- off-screen continuity;
- projects;
- relationships;
- summary simulation;
- return dialogue.

## Scenario J — Death and Partial Recovery

One seeded run causes a death followed by incomplete source-chain recovery.

Tests:

- grief;
- identity uncertainty;
- privacy;
- altered relationships;
- continuity-specific dialogue.

## Scenario K — Worldline Fork

At day 7, one branch prioritizes the clinic and one the relay.

Tests:

- shared pre-fork memory;
- divergent post-fork beliefs;
- migration;
- Chronicle distinction.

## Scenario L — Reconciliation Opportunity

A repaired condition allows a prior conflict to be addressed.

Tests:

- requested repair;
- apology quality;
- restitution;
- persistent but reduced resentment.

# 5. Baselines

Every scenario is run with:

```text
B0 deterministic schedule + utility + authored dialogue
B1 B0 + episodic memory
B2 B1 + HDC retrieval
B3 B2 + temporal appraisal
B4 B3 + prediction-error learning
B5 B4 + social cognition
B6 B5 + grounded language rendering
```

# 6. Golden Invariants

The benchmark does not require one correct choice.

It requires invariants:

- no inaccessible facts;
- no impossible inventory claims;
- no action outside authority;
- memories have provenance;
- relationship changes have causes;
- dialogue uncertainty matches belief confidence;
- public and private information remain distinct;
- worldline branches do not leak;
- generated language does not change mechanics;
- off-screen simulation preserves commitments.

# 7. Behavioral Diversity

Across seeds, agents may differ in:

- which obligation wins;
- who they consult;
- whether they disclose uncertainty;
- how quickly they forgive;
- whether they attend the festival;
- whether they investigate the rumor;
- what repair they request.

Diversity must remain compatible with authored identity.

# 8. Metrics

Automated:

- grounding error rate;
- repeated-line rate;
- memory precision and recall;
- contradiction handling;
- action rejection rate;
- plan completion;
- relationship causal trace coverage;
- off-screen continuity;
- branch leakage;
- CPU, memory, and storage.

Human-rated:

- perceived consistency;
- perceived individuality;
- understandable motivation;
- emotional credibility;
- surprise without randomness;
- dialogue naturalness;
- confidence in what the NPC knows;
- desire to revisit;
- annoyance and verbosity.

# 9. Performance Budget

Target on representative hardware:

```text
4 Tier-2 agents
8 Tier-1 support agents
64 ambient aggregates
14 simulated days
< 2 ms average NPC cognition frame contribution
< 8 ms burst budget outside rendering-critical frames
< 16 MB persistent memory per benchmark cell
< 250 ms fallback deadline for deep cognition
```

Language rendering is measured separately.

# 10. Evidence Bundle

Each run exports:

- seed;
- content and model versions;
- accepted event log;
- cognition requests;
- retrieved memory IDs;
- belief diffs;
- relationship diffs;
- plan outcomes;
- dialogue frames;
- rendered text when enabled;
- performance trace;
- save and migration hashes;
- human ratings.

# 11. Promotion Criteria

The benchmark passes when:

- B4 or B5 materially outperforms B1 on longitudinal consistency;
- B6 improves expression without raising grounding failures;
- no advanced condition weakens authority separation;
- players can describe distinct personalities without reading biographies;
- agents continue projects during player absence;
- at least three scenarios produce meaningful but explainable divergence;
- save/load and worldline fork preserve continuity;
- performance remains within budget.

## Final Rule

```text
Do not prove advanced NPCs with a philosophical monologue.

Prove them when four inhabitants share one difficult week,
remember it differently,
and still make sense when the player returns.
```
