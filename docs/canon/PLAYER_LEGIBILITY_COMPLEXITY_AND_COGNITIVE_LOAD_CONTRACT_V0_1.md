---
title: Player Legibility, Complexity, and Cognitive Load Contract
version: 0.1
status: canonical
scope: information hierarchy, causal legibility, progressive disclosure, warnings, uncertainty, friction, interruption, and player attention
owner: design/UX/accessibility/simulation
related:
  - tech/CAUSAL_EXPLANATION_AND_PLAYER_FEEDBACK_RUNTIME_V0_1.md
  - tech/FIELD_DECK_INTERFACE_AND_INFORMATION_ARCHITECTURE_BIBLE_V0_2.md
  - canon/PLAYER_EXPERIENCE_AND_SESSION_RHYTHM_CONTRACT_V0_1.md
  - canon/SYSTEM_INTERACTION_AND_DEPENDENCY_MAP_V0_1.md
  - tech/WORLD_STATE_REVISITABILITY_AND_CONSEQUENCE_PRESENTATION_V0_1.md
  - vision/PLAYER_FEEL_AND_EMBODIED_INTERACTION_BIBLE_V0_2.md
---

# Player Legibility, Complexity, and Cognitive Load Contract

## Owned Question

**How can Symtropy expose deep physical, ecological, social, economic, civic, and historical systems without turning play into constant dashboard management, bureaucratic interruption, opaque failure, or expert-only comprehension?**

## Core Thesis

Complexity is valuable when it creates meaningful distinctions and discoverable causality.

Complexity is harmful when the player must remember invisible rules, monitor too many equal-priority signals, or stop embodied play to administer every consequence.

> **The world may be deep. The player’s next decision must remain readable.**

## Prime Directives

1. **No equal-priority information wall.** The interface must distinguish immediate danger, current task, emerging pressure, optional depth, and historical context.
2. **No mandatory omniscience.** Players may act with uncertainty and delegate monitoring.
3. **No hidden irreversible consequence.** High-impact actions require readable stakes, uncertainty, and available safeguards.
4. **No warning spam.** Repeated alerts aggregate, escalate, route, or become delegated tasks.
5. **No civic ceremony for trivial action.** Authority friction scales with shared impact, hazard, reversibility, and conflict.
6. **No system that requires external documentation for ordinary play.** Advanced mastery may reward study; core comprehension must emerge in-world.
7. **No single sensory channel for critical state.** Visual, audio, haptic, spatial, and textual cues must have alternatives.
8. **No false certainty.** Observation, inference, claim, prediction, and authority remain visibly distinct.
9. **No explanation without action.** Diagnostics should connect causes to possible responses, not merely describe failure.
10. **No depth tax on every player.** Optional specialists can engage deeper interfaces while teammates receive role-relevant summaries.

# 1. Information Horizons

## 1.1 Immediate

Milliseconds to seconds.

```text
collision
incoming attack
pressure loss
fall risk
tool contact
vehicle instability
```

Delivered through embodied cues with minimal text.

## 1.2 Tactical

Seconds to minutes.

```text
current objective
nearby hazards
team roles
device fault
available route
resource immediately required
```

## 1.3 Operational

Minutes to sessions.

```text
maintenance backlog
convoy readiness
settlement shortage
relationship tension
active investigation
construction project
```

## 1.4 Strategic

Days to years.

```text
faction drift
ecological regime change
wealth concentration
war escalation
terraforming
worldline commitments
```

Strategic information should not demand constant foreground attention. It surfaces at decision windows, milestones, and deliberate review spaces.

# 2. Progressive Disclosure

Information depth follows:

```text
cue
summary
cause
history
model
raw evidence
```

Example:

```text
Cue: pump tone becomes irregular.
Summary: flow unstable.
Cause: intake cavitation likely.
History: debris load rose after upstream collapse.
Model: predicted seal damage in 6–12 hours.
Raw evidence: pressure trace and sensor provenance.
```

Players may stop at the layer needed for their role.

# 3. Attention Budget

Every scene and session has an attention budget.

Critical foreground channels:

```text
one immediate survival problem
one primary current intention
one or two meaningful secondary pressures
```

Additional systems remain ambient, delegated, summarized, or deferred.

A design review must identify which signal wins when multiple systems compete.

# 4. Action Legibility

Before a meaningful action, the player should understand:

```text
what will happen immediately
what may happen later
who or what is affected
what is uncertain
whether the action is reversible
what authority or commitment it creates
```

This may be conveyed through the world, NPCs, tools, planning views, or confirmation—not always a modal dialog.

# 5. Consequence Classes

```text
reversible experiment
recoverable commitment
costly commitment
public precedent
irreversible transformation
```

Interface friction rises by class.

Reversible experiments should be easy. Irreversible transformations deserve staging, witnesses, forecast, or explicit consent.

# 6. Warning Architecture

Warnings have:

```text
severity
confidence
time horizon
scope
owner or responsible role
actionability
suppression and aggregation policy
```

Warning behavior:

```text
inform
recommend
request acknowledgment
interrupt
force safe state
```

Only physically or ethically justified systems may force safe state.

# 7. Uncertainty Language

Use consistent states:

```text
observed
strongly inferred
probable
possible
contested
unknown
unavailable
```

Never use red or green alone to encode certainty or morality.

Predictions state horizon and confidence. A model may be wrong without the game appearing arbitrary if the assumptions and evidence were visible.

# 8. Delegation

Players may delegate:

```text
monitoring
routine maintenance
route following
inventory thresholds
warning triage
schedule optimization
public reporting
```

Delegation creates responsibility boundaries, not invisible automation magic.

A delegated system reports exceptions, summaries, and confidence. Players can inspect and override it.

# 9. Role-Relative Views

A medic, pilot, engineer, ecologist, organizer, and fighter should not receive identical information priority.

Shared truth remains consistent, but presentation emphasizes role-relevant cues.

Players can pin, share, or request other views without stealing control.

# 10. Co-op Communication

Structured sharing includes:

```text
mark
request
warning
hypothesis
route
resource need
handoff
commitment
```

Share the meaning and provenance, not only a screen image.

The system should reduce the need for one player to narrate every UI detail verbally.

# 11. Learning and Mastery

Early encounters teach through:

```text
visible cause and effect
safe experimentation
NPC demonstration
physical analogy
bounded tooltips
post-action reflection
```

Advanced mastery adds better predictions, faster diagnosis, richer models, and more precise control—not permission to understand basic survival.

# 12. Administrative Friction Budget

A normal 60–90 minute session should not require repeated formal approvals for routine personal or low-risk action.

Formal friction is reserved for:

```text
shared survival infrastructure
irreversible ecological release
hazardous public construction
major civic commitments
body or memory consent
worldline forks
```

Reviews, hearings, and votes become meaningful because they are not constant.

# 13. Failure Explanation

After failure, players need:

```text
what failed
immediate cause
important contributors
what evidence was available beforehand
what changed now
possible recovery paths
```

The game should avoid both omniscient autopsy and vague punishment. Explanation depends on surviving sensors, witnesses, records, and knowledge.

# 14. Accessibility

Required options include:

```text
text size and spacing
contrast and color-independent cues
captions and sound descriptions
reduced motion and flash
input remapping and holds/toggles
extended timing windows
simplified interaction assistance
cognitive-load presets
screen-reader compatible structured UI
haptic alternatives
```

Accessibility assists may reveal control affordances without granting hidden world truth.

# 15. Seedworks Boundary

The representative build should prove:

```text
one layered diagnostic
one uncertainty state
one delegated warning
one co-op structured handoff
one high-impact action preview
one failure explanation
one cognitive-load accessibility preset
```

# 16. Acceptance Tests

1. New players identify the next meaningful action without external guidance.
2. Expert players can inspect deeper causes without forcing that depth on everyone.
3. Critical information is available through at least two sensory channels.
4. Repeated warnings aggregate rather than spam.
5. Players distinguish observed fact from inference and prediction.
6. High-impact actions communicate scope, uncertainty, reversibility, and affected parties.
7. Delegated systems surface actionable exceptions.
8. Failure reports improve future decisions without revealing unavailable omniscient truth.
9. Co-op role views preserve shared truth and enable structured handoff.
10. Playtests measure workload, interruption, and comprehension—not only task completion.

## Final Rule

```text
Symtropy should reward attention.
It must not punish the player for being unable to attend to everything at once.
```
