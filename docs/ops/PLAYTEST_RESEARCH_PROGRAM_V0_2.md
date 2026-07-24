---
title: Symtropy Playtest Research Program
version: 0.2
status: implementation-spec
scope: qualitative and quantitative validation of representative gameplay
owner: research/design/qa
supersedes:
  - ops/PLAYTEST_CHECKLIST.md
related:
  - canon/PLAYER_EXPERIENCE_AND_SESSION_RHYTHM_CONTRACT_V0_1.md
  - ops/SEEDWORKS_PRODUCTION_BUDGET_AND_CONTENT_PLAN_V0_1.md
---

# Symtropy Playtest Research Program

## Purpose

The prior checklist verifies an early systems prototype. It does not establish whether Symtropy is understandable, enjoyable, integrated, replayable, or representative of its intended player promise.

This program separates:

```text
functional verification
performance verification
experience research
systems comprehension
longitudinal world response
```

A feature can pass functional checks and still fail as a game.

## Research Principles

### 1. Observe Before Explaining

Do not teach the participant the intended interpretation before observing behavior.

### 2. Ask for Causal Models

The key question is not whether a player liked a number.

Ask what they believe happened, why, and what they expect next.

### 3. Test Roles Separately and Together

A mechanic may feel excellent alone but erase other players in co-op.

### 4. Separate Novelty From Durability

Run first-use, repeat-use, and long-session tests.

### 5. Preserve Failed Outcomes

Do not reset every test at the first failure. Observe whether recovery remains understandable and motivating.

### 6. Accessibility Is Core Validation

Assistance settings must be tested against the same causal world, not postponed as compliance work.

## Test Lanes

### Lane A — Embodied Feel

Questions:

```text
Is movement reliable?
Do tools feel physical and precise?
Is the rover enjoyable?
Is danger readable?
Can players recover from input mistakes?
```

Methods:

```text
5–15 minute focused tasks
input telemetry
video observation
post-task rating and description
```

Measures:

```text
time to successful interaction
mis-target rate
camera correction frequency
tool cancellation/error rate
vehicle collision and recovery rate
combat damage source comprehension
```

Pass evidence:

Players describe controls as trustworthy even when mastery is incomplete.

### Lane B — Onboarding and Comprehension

Questions:

```text
Can players act before reading lore?
Do they understand the opening choices?
Can they distinguish observation from inference?
Do they know what changed after a mission?
```

Measures:

```text
time to first useful action
prompt dependence
time in Field Deck
first thread distribution
incorrect causal explanations
objective abandonment reason
```

Interview prompts:

```text
What is happening in the basin?
Why did you choose that route?
What does the Field Deck know versus guess?
What changed when you returned?
```

### Lane C — Session Rhythm and Delight

Questions:

```text
Does the session vary tempo?
Do players voluntarily explore or rest?
Is work satisfying rather than repetitive?
Does the settlement feel worth inhabiting?
```

Measures:

```text
voluntary detours
unprompted social/leisure participation
longest period without meaningful decision
menu-time percentage
repeated action count
session stop point
```

Interview prompts:

```text
What made you curious?
What felt like a chore?
Where would you spend time without an objective?
What made the settlement feel alive?
```

### Lane D — Systemic Causality

Questions:

```text
Do actions propagate visibly?
Can players form useful models?
Are consequences surprising but explainable?
```

Test cases:

```text
bridge restoration
cargo shortage
signal restoration
factory activation
ecological intervention
partial defense failure
```

Measures:

```text
consequence detection without UI prompt
causal explanation accuracy
time between action and visible response
number of domains affected
```

### Lane E — Combat and Threat

Questions:

```text
Are enemy roles legible?
Do terrain and infrastructure matter?
Are nonlethal or withdrawal options credible?
Does preparation change outcomes?
```

Measures:

```text
damage by source
role identification
flank and cover use
retreat frequency
resource/tool diversity
co-op revive and support contribution
```

Do not evaluate only kill time.

### Lane F — Construction, Economy, and Mastery

Questions:

```text
Does construction change possibility?
Are resource constraints geographical and understandable?
Does mastery reduce tedium?
Do players see multiple acquisition paths?
```

Measures:

```text
planning time
material handling burden
assembly error recovery
alternate solution use
repetition before automation/delegation
post-build utilization
```

### Lane G — NPC Life and Culture

Questions:

```text
Do NPCs seem to have lives beyond missions?
Are relationships remembered?
Is culture experienced rather than described?
Can players enjoy social space without governance?
```

Measures:

```text
NPCs remembered by name or role
unprompted return visits
participation in cultural activity
recognition of routine change
relationship interpretation accuracy
```

### Lane H — Cooperative Role Fairness

Questions:

```text
Does each player have meaningful work?
Does one interface dominate?
Can roles overlap and recover from absence?
```

Measures:

```text
action participation by player
idle/wait time
shared spotlight duration
role switching
communication load
perceived contribution
```

After the session, independently ask each player what every other player contributed.

### Lane I — Solo Viability

Questions:

```text
Can one player complete causal activities without excessive micromanagement?
Do NPC and automation supports preserve meaning?
```

Measures:

```text
task switching burden
AI assistance failures
cargo handling time
threat overload
planning pause use
```

### Lane J — Accessibility and Cognitive Load

Test profiles:

```text
motor assistance
reduced motion
high-contrast/low-vision
hearing support
reduced text density
objective guidance
combat assistance
```

Validate:

```text
same world state
same causal consequence
clear feedback
no humiliation or hidden penalty
```

### Lane K — Persistence and Return

Questions:

```text
Does the world remember correctly?
Can players understand changes after absence?
Do scars motivate return?
```

Test:

```text
save/reload
session reconnect
24-hour simulated absence
branch outcome comparison
source-chain recovery
```

### Lane L — Performance and Reliability

Functional stress scenarios:

```text
major encounter + rover + NPC routines + device network
four-player cargo/construction session
site transition with regional simulation active
save during transformed state
network interruption during Chronicle queue
```

Measure:

```text
frame time
simulation tick stability
network latency and correction
event queue volume
memory
save size and load time
```

## Study Sequence

### Stage 0 — Expert Heuristic Review

Goal:

Catch obvious interaction, readability, scope, and accessibility issues before participant time is spent.

### Stage 1 — Isolated Mechanic Tests

Test:

```text
movement
tools
rover
combat roles
Field Deck basic use
construction
```

A weak isolated mechanic should not be hidden inside a large scenario.

### Stage 2 — Thread Tests

Test individual opening threads end to end.

Goal:

Validate local pacing, consequence, and role identity.

### Stage 3 — Integrated Regional Session

Test 45–90 minute sessions with choice among threads.

Goal:

Validate session rhythm and causal integration.

### Stage 4 — Replay and Alternate Path

Participants repeat with different roles or routes.

Goal:

Separate content novelty from systemic replay.

### Stage 5 — Longitudinal World Test

Multiple sessions in one persistent basin.

Goal:

Validate NPC memory, regional change, progression, maintenance, and return motivation.

## Participant Segments

Recruit across:

```text
survival/building players
action/co-op players
simulation/strategy players
RPG/narrative players
technical sandbox players
players unfamiliar with dense systems
accessibility profiles
```

Do not rely only on developers or people already invested in Symtropy’s philosophy.

## Session Protocol

### Before

Record:

```text
relevant game experience
accessibility needs
preferred play style
prior knowledge of Symtropy
```

### During

Facilitators should:

```text
avoid leading explanations
record confusion moments
note voluntary behavior
mark recoveries and workarounds
capture player communication
```

### After

Use recall before rating:

```text
Tell me what happened.
What did you decide?
What changed?
What would you do next?
```

Then collect scales for:

```text
control trust
clarity
tension
agency
curiosity
connection
chore burden
return motivation
```

## Core Product Metrics

No single metric decides success.

Recommended dashboard:

```text
First Useful Action Time
Voluntary Detour Rate
Causal Recall Accuracy
Distinct Verb Use
Meaningful Decision Interval
Menu Attention Share
Role Contribution Balance
World Change Recognition
Chore Repetition Index
Return Intention
Performance Reliability
```

## Decision Thresholds

### Stop and Fix

```text
Most players cannot explain immediate failure.
One role dominates co-op attention.
Players spend more time in the Field Deck than in the world during ordinary activity.
Routine work becomes repetitive before automation or delegation appears.
Players do not notice persistent changes.
The primary threat is visually impressive but tactically unreadable.
Accessibility modes create different outcomes or hidden penalties.
```

### Continue With Caution

```text
Players understand the system but do not find it enjoyable.
Players enjoy first use but avoid repetition.
Consequences are noticed only after summaries.
The region supports one strong thread but others feel secondary.
```

### Representative Proof

```text
Players choose different opening commitments for positive reasons.
Multiple roles report meaningful contribution.
At least one physical transformation produces visible cross-system effects.
Players voluntarily explore and participate in one non-crisis activity.
Failure creates a comprehensible continuation.
The settlement is remembered as a place, not only a menu hub.
Participants can state a self-directed next goal.
```

## Reporting Format

Each study report should include:

```text
build identifier
research questions
participant profile
scenario
observed behavior
quantitative results
interpretive findings
severity
recommended change
owner
retest condition
```

Separate evidence from interpretation.

## Relationship to Automated Tests

Automated and functional checklists remain necessary for:

```text
correct state transitions
deterministic transactions
save/load
network replication
performance regressions
accessibility setting persistence
```

They do not replace player research.

The former `PLAYTEST_CHECKLIST.md` is retained as a historical prototype verification list.

## Final Rule

```text
We are not testing whether players agree with the design document.
We are testing whether the game produces the intended experience without explanation.
```

# v0.3 Additional Research Lanes

## Lane M — Mission and Event Grammar

Questions:

```text
Do players understand why opportunities exist?
Do different methods produce distinguishable outcomes?
Does failure continuation remain motivating?
Do generated variants feel causal rather than shuffled?
```

Measures:

```text
method diversity
abandonment comprehension
failure-to-reengagement rate
repeated-node fatigue
causal explanation accuracy
```

## Lane N — Scientific Discovery

Questions:

```text
Can players separate observation from inference?
Do failed experiments remain useful?
Does better instrumentation change strategy?
```

Measures:

```text
premature-certainty rate
voluntary replication
instrument selection diversity
correct scope descriptions
```

## Lane O — Authorship and Mod Compatibility

Questions:

```text
Can players create useful distinctive artifacts?
Can they understand provenance and dependencies?
Does missing or outdated content fail safely?
```

## Lane P — Multiplayer Abuse and Recovery

Adversarial scenarios:

```text
spawn camping
mass demolition
inactive-authority lockout
permission race
identity impersonation
malicious mod manifest
worldline profile change
```

Pass evidence requires both prevention and credible targeted recovery.

## Lane Q — Longitudinal Revisitability

Run delayed tests after one in-world day, week, and season where feasible.

Observe whether players notice and correctly attribute physical, social, economic, and ecological change.
