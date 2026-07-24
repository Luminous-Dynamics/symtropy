---
title: Shift, Project, Career, Tool, and Handover Runtime
version: 0.1
status: implementation-spec
scope: profession state, work orders, shifts, crews, tools, calibration, hazards, projects, handovers, credentials, compensation, career identity, apprenticeship, and replay
owner: design/gameplay/simulation/economy/engineering
related:
  - ../canon/PROFESSION_SIMULATION_EMBODIED_MASTERY_AND_PUBLIC_RESPONSIBILITY_CONTRACT_V0_1.md
  - NPC_LEARNING_TEACHING_APPRENTICESHIP_AND_SKILL_TRANSMISSION_RUNTIME_V0_1.md
  - ECONOMIC_LEDGER_MARKET_AND_INTEGRITY_RUNTIME_V0_1.md
  - CAUSAL_EXPLANATION_AND_PLAYER_FEEDBACK_RUNTIME_V0_1.md
  - ../ops/NPC_INTELLIGENCE_OBSERVABILITY_EVIDENCE_AND_FAILURE_TRIAGE_STANDARD_V0_1.md
---

# Shift, Project, Career, Tool, and Handover Runtime

## Purpose

This runtime provides shared data and event structures for deep profession gameplay without forcing every profession into the same visible minigame.

## Core Principle

> **Shared infrastructure should preserve causal work history while allowing every profession to expose a different rhythm, interface, and form of mastery.**

# 1. Profession Profile

```text
profession_id
domains and specializations
sensory channels
recognized procedures
tool families
hazards
credential families
public duties
privacy obligations
common institutions
teaching pathways
```

The profile defines capabilities and expectations, not a character class.

# 2. Practitioner State

A practitioner stores:

```text
practice history
learned procedures
perceptual discriminations
tool familiarity
current fatigue and exposures
credentials and jurisdictions
reputation by practice domain
institution memberships
mentors and apprentices
known shortcuts and cautions
injuries and accommodations
practice style tags
unresolved obligations
```

Knowledge, physical ability, authority, and public trust remain separate.

# 3. Work Order

A work order includes:

```text
requester
reported problem or desired service
site and access
urgency
known hazards
affected people and systems
budget or compensation
required authority
privacy scope
available records
promises and deadlines
```

Reported symptoms are not authoritative diagnoses.

# 4. Shift State

```text
shift_id
workplace
scheduled and actual time
crew roster
role assignments
open work orders
resources and supplies
breaks
fatigue and exposure
incidents
handover predecessor and successor
pay and tip state
transport and household constraints
```

A shift can end with unresolved work. The runtime does not force every task to complete before the player leaves.

# 5. Project State

Projects contain:

- milestones;
- dependencies;
- budgets;
- material and labor reservations;
- public or client commitments;
- risk register;
- evidence and decisions;
- stakeholder positions;
- changes of scope;
- inspections;
- accumulated maintenance or technical debt;
- completion and follow-up criteria.

Projects may continue during player absence through assigned authority and simulation level of detail.

# 6. Tool State

```text
tool_id
tool family
owner and access rights
manufacturer and standards
calibration state
wear and damage
contamination
firmware and policy locks
attachments and modifications
known quirks
practitioner familiarity
maintenance history
```

Tool performance combines capability, condition, environment, procedure, and user familiarity.

# 7. Calibration and Verification

Calibration requires:

- reference standard;
- valid environment;
- procedure;
- operator competence;
- timestamp;
- uncertainty;
- evidence.

A calibration certificate can become stale, forged, invalid under current conditions, or politically required beyond its technical value.

# 8. Sensory and Diagnostic Cues

The runtime exposes profession-specific cues through authored or systemic channels:

- sound;
- vibration;
- smell;
- color;
- thermal pattern;
- timing;
- movement;
- social behavior;
- record inconsistency;
- ecological response;
- material feel;
- machine telemetry.

Cues have observability, reliability, and interpretation requirements.

A novice may notice the cue but misclassify it. A master may recognize a pattern but still be wrong.

# 9. Procedure Execution

Procedures define:

```text
preconditions
required and optional tools
roles
steps and flexible regions
hazards
interruptions
quality observations
stop conditions
completion evidence
cleanup
follow-up
```

The system supports procedural variation. It does not reduce every procedure to fixed quick-time events.

# 10. Handover Record

A handover must capture:

```text
completed work
open work
uncertainties
temporary fixes
consumed supplies
unusual observations
hazards and controls
promises made
people affected
next review time
source evidence
receiving person acknowledgement
```

A handover may be oral, written, sensor-backed, ritualized, encrypted, or embodied in a marked object.

Bad handovers can cause delayed failure. Excessive paperwork can consume work capacity and hide crucial information.

# 11. Compensation

The runtime supports:

- wages;
- salaries;
- tips;
- piece rates;
- contracts;
- retainers;
- cooperative shares;
- public service;
- volunteer work;
- care obligations;
- barter;
- informal payment.

Compensation records hours, preparation, cleanup, canceled work, expenses, withheld amounts, and disputes.

# 12. Credentials and Liability

Credentials have:

```text
issuer
scope
jurisdiction
competence evidence
expiry or review
insurance or bond
restrictions
suspension and appeal
historical reason
```

Emergency work may create temporary authority with mandatory review.

Liability attaches through actual duty, control, action, knowledge, and institutional responsibility—not merely because the player touched the system last.

# 13. Occupational Safety

Hazard exposure records:

- type;
- intensity;
- duration;
- protection;
- cumulative burden;
- acute incidents;
- reporting;
- accommodation;
- employer or institutional response.

The runtime supports near misses and unsafe normalization, not only injury events.

# 14. Crew State

Crews have:

- role coverage;
- communication practices;
- trust;
- conflict;
- shared routines;
- leadership;
- informal expertise;
- understaffing;
- cultural norms;
- recent incidents;
- performance under stress.

Crew effectiveness is not the sum of individual skill values.

# 15. Teaching and Apprenticeship

A teaching event records:

- skill or judgment target;
- demonstration;
- learner attempt;
- supervision;
- feedback;
- error and recovery;
- transfer to a new context;
- safety boundary;
- mentor and institutional incentives.

Competence cannot be granted by dialogue alone.

# 16. Career Identity

Career identity emerges from actual history:

```text
practice domains
success and failure patterns
clients and communities served
institutions joined or opposed
signature methods
safety record
teaching lineage
public controversies
injuries and accommodations
unfinished projects
```

Generated titles or reputations must cite supporting events.

# 17. Simulation Levels of Detail

## Full Practice

Detailed tools, cues, named crew, procedure, interruptions, and handover.

## Workplace

Aggregated minor tasks with named critical workers, projects, incidents, compensation, and safety preserved.

## Regional

Labor capacity, credential shortages, project state, institutional reputation, wage pressure, and major failures.

## Long Absence

Career, apprenticeship, injury, workplace ownership, project completion, and handover consequences remain deterministic and inspectable.

# 18. Determinism and Replay

Authoritative outcomes record:

- input snapshot;
- tools and calibration;
- practitioner and crew state;
- procedure version;
- random seed where applicable;
- interruptions;
- observations;
- accepted actions;
- material changes;
- handover and follow-up.

A replay may simplify animation but must reproduce authoritative consequences.

# 19. Acceptance Tests

Required tests include:

- a bad handover causes a specific delayed risk rather than generic failure;
- tool familiarity can matter without overcoming a broken tool;
- certification and competence can disagree;
- a project continues during player absence through assigned workers;
- unpaid preparation is visible in labor accounting;
- a crew with high individual skill can fail through communication;
- fatigue and accommodation affect work without making self-harm optimal;
- teaching requires supervised practice and transfer;
- career reputation is grounded in events;
- reduced-detail simulation preserves named workers, pay, injuries, projects, and obligations.
