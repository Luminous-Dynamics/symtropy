---
title: NPC Learning, Teaching, Apprenticeship, and Skill Transmission Runtime
version: 0.1
status: implementation-spec
scope: skill representation, practice, teaching, apprenticeship, tacit knowledge, institutional transmission, learning evidence
owner: AI/simulation/gameplay/narrative/engineering
related:
  - ../canon/LIFE_COURSE_HOUSEHOLDS_KINSHIP_AND_EDUCATION_CONTRACT_V0_1.md
  - ../canon/PROGRESSION_ECONOMY_AND_MASTERY_CONTRACT_V0_1.md
  - NPC_COGNITION_AGENCY_AND_SIMULATION_RUNTIME_V0_1.md
  - NPC_MEMORY_CONSOLIDATION_LEARNING_AND_WORLDLINE_CONTINUITY_RUNTIME_V0_1.md
  - ../ops/NPC_CONTENT_AUTHORING_AND_GROUNDING_STANDARD_V0_1.md
---

# NPC Learning, Teaching, Apprenticeship, and Skill Transmission Runtime

## Purpose

Define how NPCs and players gain, retain, teach, adapt, lose, and transmit practical knowledge without collapsing learning into invisible experience points or instant blueprint unlocks.

## Core Thesis

Skill is not one number.

A person may know what a repair requires, recognize a failure sound, possess the hand control to perform it, understand the safety doctrine, and still lack legal authorization or confidence under pressure.

The runtime therefore separates several dimensions of capability.

# 1. Skill Model

```rust
struct SkillState {
    skill_id: SkillId,
    conceptual_knowledge: Scalar,
    procedural_fluency: Scalar,
    perceptual_discrimination: Scalar,
    calibration: Scalar,
    safety_judgment: Scalar,
    improvisation: Scalar,
    teaching_capacity: Scalar,
    confidence: Scalar,
    fatigue_sensitivity: Scalar,
    context_experience: ContextMap,
    authorization_scopes: Vec<AuthorizationScope>,
    provenance: SkillProvenance,
}
```

These dimensions have different learning sources and failure modes.

## 1.1 Conceptual Knowledge

Understanding principles, terminology, causal models, and documented procedure.

## 1.2 Procedural Fluency

Executing a sequence accurately under ordinary conditions.

## 1.3 Perceptual Discrimination

Recognizing sound, vibration, smell, movement, tissue state, social signal, or visual detail relevant to the skill.

## 1.4 Calibration

Adapting action to a specific tool, body, machine, species, environment, or material.

## 1.5 Safety Judgment

Knowing when not to proceed, when to seek help, and which failure signs matter.

## 1.6 Improvisation

Generating bounded alternatives when normal procedure fails.

## 1.7 Teaching Capacity

Selecting demonstrations, feedback, explanations, and practice appropriate to another learner.

# 2. Knowledge Objects

Skills may depend on explicit knowledge objects:

- manuals;
- blueprints;
- repair cards;
- annotated logs;
- oral histories;
- demonstration records;
- machine testimony;
- ecological observations;
- ritual sequence;
- legal authorization;
- embodied memory;
- translated alien pattern.

Owning a manual does not grant procedural fluency. Destroying a manual does not erase knowledge already carried by people.

# 3. Learning Events

```rust
struct LearningEvent {
    event_id: LearningEventId,
    learner: AgentId,
    skill_id: SkillId,
    mode: LearningMode,
    teacher: Option<AgentId>,
    task_id: Option<TaskId>,
    context: LearningContext,
    outcome: LearningOutcome,
    errors: Vec<ObservedError>,
    feedback: Vec<FeedbackEvent>,
    evidence_ids: Vec<EvidenceId>,
    fatigue: Scalar,
    risk_exposure: Scalar,
    timestamp: ChronicleTick,
}
```

Learning modes include:

- observation;
- explanation;
- guided practice;
- independent practice;
- simulation;
- failure analysis;
- peer collaboration;
- reverse engineering;
- experimentation;
- ritual participation;
- machine-assisted calibration;
- memory recovery;
- teaching another person.

# 4. Practice and Mastery

Practice improves the dimensions actually exercised.

Examples:

- reading raises conceptual knowledge but not necessarily procedural fluency;
- repeated safe operation raises fluency and calibration;
- varied fault cases improve perceptual discrimination and improvisation;
- reviewing near misses improves safety judgment;
- teaching improves conceptual organization and teaching capacity;
- using one standardized tool may not generalize to damaged or alien equipment.

The system should avoid repetitive grind by using diminishing returns for identical low-variance tasks.

Meaningful improvement comes from:

- increased complexity;
- varied contexts;
- feedback;
- reflection;
- responsibility;
- successful transfer to new conditions.

# 5. Error and Feedback

Errors are not generic failure points.

They may be:

- sequence omission;
- timing error;
- force or dosage error;
- misclassification;
- poor tool choice;
- unsafe continuation;
- communication failure;
- authorization mistake;
- context overgeneralization;
- cultural or species misinterpretation.

Feedback may come from:

- teacher;
- machine diagnostics;
- task outcome;
- peer observation;
- self-review;
- Chronicle evidence;
- environmental response;
- injury or near miss;
- later system failure.

Delayed consequences must be able to revise the learner’s confidence and safety judgment.

# 6. Teaching Model

A teacher needs more than high skill.

Teaching quality depends on:

- teaching capacity;
- trust and relationship;
- shared language or translation;
- time;
- patience;
- ability to observe the learner;
- access to safe practice conditions;
- willingness to disclose tacit knowledge;
- cultural norms;
- institutional incentive;
- teacher fatigue.

Teachers choose among:

- demonstration;
- explanation;
- guided handoff;
- error correction;
- question-led discovery;
- paired work;
- simulation;
- supervised real task;
- public lesson;
- private mentoring;
- refusal to teach.

A highly skilled person may be a poor teacher. A moderately skilled person may be excellent at introducing fundamentals.

# 7. Apprenticeship

An apprenticeship is a durable relationship and institutional pathway.

```rust
struct ApprenticeshipState {
    apprenticeship_id: ApprenticeshipId,
    mentor: AgentId,
    apprentice: AgentId,
    skill_domain: SkillDomainId,
    institution: Option<InstitutionId>,
    phase: ApprenticeshipPhase,
    trust: Scalar,
    responsibility_scope: ResponsibilityScope,
    required_experiences: Vec<ExperienceRequirement>,
    completed_evidence: Vec<EvidenceId>,
    safety_incidents: Vec<IncidentId>,
    unresolved_conflicts: Vec<ConflictId>,
    expected_review_tick: ChronicleTick,
}
```

Phases may include:

- observation;
- basic assistance;
- supervised execution;
- independent bounded work;
- public responsibility;
- teaching or certification.

Advancement should require demonstrated evidence, not time served alone.

# 8. Tacit Knowledge

Tacit knowledge includes:

- how a healthy machine sounds;
- when a storm route becomes unsafe;
- how a person prefers to be assisted;
- how a species signals overload;
- how much force a corroded fitting tolerates;
- which public process is formally valid but socially explosive;
- how to calm a frightened animal or machine.

Tacit knowledge is learned through situated experience and trusted transmission.

The runtime may encode it as context-sensitive priors, perceptual thresholds, action affordance weights, or authored recognition tags. It should not be represented only as dialogue text.

# 9. Authorization and Competence

Competence and permission are separate.

A person may be:

- competent but uncertified;
- certified but out of practice;
- authorized only under supervision;
- technically capable but culturally untrusted;
- trusted locally but unrecognized by another charter;
- skilled in a similar system but not this model;
- able to act in emergency but required to undergo later review.

Action validation checks both capability and authority where appropriate.

# 10. Learning Through Symthaea

Symthaea may assist:

- retrieval of relevant prior experiences;
- comparison of current context to learned cases;
- attention to diagnostic features;
- prediction error after unexpected outcome;
- consolidation of repeated patterns;
- selection of questions or explanations;
- learner-model proposals.

It may not directly increment skill state.

Every skill change must be produced by a validated learning event and bounded update rule.

# 11. Player and NPC Mutual Teaching

Players may learn from NPCs and NPCs may learn from players.

Player teaching can involve:

- demonstrating a procedure;
- sharing an authored blueprint;
- supervising a task;
- explaining a discovered pattern;
- creating a training environment;
- correcting a dangerous habit;
- teaching a new tool workflow.

NPCs should evaluate player teaching through actual outcomes, trust, and evidence. They must not automatically accept the player as an expert.

NPC teaching may unlock:

- perceptual cues;
- tool techniques;
- new safe action templates;
- contextual diagnostics;
- cultural interpretation;
- access to supervised work;
- certification opportunities.

# 12. Institutional Knowledge

Institutions preserve and distort knowledge.

They may maintain:

- curricula;
- certification standards;
- public schematics;
- oral traditions;
- safety records;
- examination tasks;
- apprenticeships;
- forbidden methods;
- proprietary locks;
- obsolete doctrine.

Institutional knowledge is not automatically correct. New evidence may generate reform, schism, or suppression.

# 13. Skill Decay and Interruption

Skills may change through:

- long disuse;
- injury;
- body modification;
- tool change;
- environmental change;
- trauma;
- loss of sensory access;
- new assistive technology;
- false confidence;
- institutional isolation.

Conceptual knowledge may remain while calibration declines. Teaching may preserve or even deepen knowledge during physical retirement.

Decay should be slow, legible, and recoverable. It should not punish ordinary player absence with arbitrary loss.

# 14. Machine, Animal, and Nonhuman Learning

The runtime must support different learning structures.

A machine may learn through:

- calibration;
- model update;
- bounded policy revision;
- human or machine testimony;
- simulation;
- supervised deployment.

An animal or uplifted species may learn through:

- conditioning;
- social imitation;
- environmental exploration;
- trust;
- play;
- sensory association.

A collective or alien intelligence may learn through:

- network restructuring;
- habitat response;
- shared signal pattern;
- seasonal memory;
- distributed experiment.

The system must preserve agency and consent boundaries appropriate to each form.

# 15. Simulation LOD

## Local high-depth agents

Simulate task observations, feedback, errors, and context-specific learning.

## Off-screen named agents

Summarize completed practice and teaching sessions from real schedule, resource, and task availability. Preserve significant errors, certifications, and relationship effects.

## Population groups

Track institutional education capacity, skill distribution bands, apprenticeship throughput, and knowledge loss risks.

Aggregate learning must not create named experts from nowhere.

# 16. Anti-Exploit Rules

Prevent:

- infinite skill gains from repeating trivial actions;
- instant copying of tacit knowledge;
- teaching while absent or without time cost;
- skill duplication through save rollback;
- certification without evidence;
- coercive extraction of private or sacred knowledge;
- children used as cheap labor progression units;
- one player monopolizing all knowledge without social consequences;
- generative dialogue creating unearned competence.

# 17. Representative Proof

The first proof should teach an adolescent apprentice to diagnose and repair a degraded vehicle subsystem over several sessions.

The sequence includes:

- observation;
- a mistaken diagnosis;
- teacher feedback;
- safe guided repair;
- independent practice;
- transfer to a different vehicle;
- a moment where the apprentice correctly refuses an unsafe order;
- a later opportunity to explain the method to another person.

The proof must demonstrate that conceptual knowledge, perceptual skill, fluency, safety judgment, confidence, and authorization change differently.

# 18. Tests

Required tests include:

- deterministic skill updates;
- save/load continuity;
- identical-task diminishing returns;
- context transfer and non-transfer;
- false-confidence correction;
- teacher quality effects;
- apprenticeship phase gating;
- authorization separation;
- off-screen learning conservation;
- privacy of restricted knowledge;
- no skill change from dialogue rendering alone;
- player/NPC mutual teaching;
- body-change recalibration.

# 19. Evidence Bundle

Each learning proof records:

- initial skill vector;
- teacher and learner profiles;
- tasks and contexts;
- observations and errors;
- feedback events;
- skill updates with provenance;
- authority changes;
- time and resource cost;
- later transfer test;
- player comprehension;
- performance cost.

## Final Rule

> **Knowledge becomes civilization when it can be practiced, questioned, taught, preserved, and safely transformed by more than one person.**
