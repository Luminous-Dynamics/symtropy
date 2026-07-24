---
title: Mission, Event, and Contract Grammar
version: 0.1
status: canonical
scope: activity generation, authored missions, simulation opportunities, objective structure, failure continuation
owner: design/narrative/systems
related:
  - canon/SYMTROPY_GAME_CONSTITUTION_V0_6.md
  - canon/PLAYER_EXPERIENCE_AND_SESSION_RHYTHM_CONTRACT_V0_1.md
  - canon/SYSTEM_INTERACTION_AND_DEPENDENCY_MAP_V0_1.md
  - tech/PROCEDURAL_HISTORY_ENGINE.md
  - tech/PROCEDURAL_FACTION_EVOLUTION.md
  - tech/CHRONICLE_EVENT_SCHEMA.md
---

# Mission, Event, and Contract Grammar

## Owned Question

**How does Symtropy turn a living simulation into clear, varied, replayable activity without reducing the world to quest markers or generated errands?**

## Core Thesis

Symtropy missions are not isolated tasks distributed by static quest givers.

They are **bounded interventions in a changing world**.

```text
pressure creates an opportunity
someone interprets the opportunity
an actor makes a request, claim, warning, or offer
the player chooses a method and accepts exposure
systems react
people remember the result
the world remains changed
```

A mission is successful when it gives the player:

```text
something understandable to attempt
several credible methods
meaningful physical or social resistance
a consequence that survives the objective
```

## What This Document Does Not Own

This document does not define:

```text
individual combat mechanics
all procedural history generation
settlement simulation equations
dialogue prose style
complete faction ideologies
Chronicle storage implementation
```

It defines the contract by which those systems become playable situations.

# 1. Activity Sources

Every activity must declare one primary source.

## 1.1 Authored Arc

Designed sequence with bespoke spaces, characters, and dramatic control.

Use for:

```text
first experiences
major faction turns
first contact
worldline-defining decisions
raids and dungeons with unique mechanics
```

Authored arcs may consume simulation state but must not pretend that every outcome was generated.

## 1.2 Simulation Opportunity

A material or social pressure becomes actionable.

Examples:

```text
route capacity falls below convoy demand
an ecological boundary is crossed
a clinic loses refrigeration
a faction cannot reconcile two valid claims
a Null process begins enforcing an obsolete constraint
```

The simulation proposes conditions. The mission grammar supplies structure and presentation.

## 1.3 Relationship Request

An NPC, household, machine, crew, guild, or alien party asks for help based on memory and relationship.

The request must be grounded in:

```text
what the requester knows
what they value
what they can offer
why they cannot solve it alone
why the player is relevant
```

## 1.4 Player-Declared Project

Players define the objective.

Examples:

```text
establish a mountain route
build a public workshop
survey a storm corridor
recover a lost archive lineage
create a neutral first-contact station
```

The game converts the declaration into requirements, risks, permissions, and milestones without inventing a fake quest giver.

## 1.5 Contract or Bounty

A formal institution offers terms.

Contracts should specify:

```text
requester
beneficiary
scope
success evidence
acceptable methods
prohibited methods
compensation
liability
expiry
appeal path
```

The institution's terms may be biased, incomplete, or exploitative. The UI must distinguish the contract's claim from objective reality.

## 1.6 Encounter Opportunity

An immediate event invites action but does not require acceptance.

Examples:

```text
a rover overturns in a dust front
a swarm crosses a migration route
a rival salvage crew opens a sealed structure
a distress beacon repeats an impossible identity
a public celebration loses power during a storm
```

These opportunities should support observation, withdrawal, intervention, or exploitation.

# 2. Opportunity Lifecycle

```text
latent condition
  → detectable signal
  → interpretation
  → offer or self-declared objective
  → commitment
  → active state
  → complication
  → resolution or abandonment
  → consequence propagation
  → memory and revisit state
```

## 2.1 Latent Condition

The world contains a problem or possibility before the player sees a marker.

## 2.2 Detectable Signal

At least one channel reveals it:

```text
visual change
sound
NPC behavior
Field Deck observation
trade price or cargo absence
weather pattern
radio traffic
rumor
machine fault
alien response
```

## 2.3 Interpretation

Different actors may describe the same condition differently.

```text
A factory calls the event a production interruption.
A settlement calls it toxic exposure.
A machine collective calls it unauthorized disassembly.
A scavenger crew calls it opportunity.
```

## 2.4 Commitment

A player may:

```text
accept explicit terms
declare intent
start physical intervention
promise help
spend public resources
cross a protected boundary
```

Commitment should be legible. The game must not silently turn curiosity into irreversible civic consent.

## 2.5 Complication

Complications arise from existing causes, not arbitrary drama rolls.

Valid sources:

```text
weather
resource shortfall
hostile response
misread agency
mechanical failure
third-party claim
injury or fatigue
time pressure
new evidence
social disagreement
```

## 2.6 Resolution

Resolution records what actually changed, not only whether the objective banner says complete.

## 2.7 Consequence Propagation

Outcomes may affect:

```text
site state
routes
stocks
relationships
faction interpretation
ecological conditions
law or precedent
knowledge
future opportunities
Chronicle history
```

# 3. Mission Family Grammar

Every family below is a reusable dramatic and mechanical structure, not a fixed plot.

## 3.1 Expedition

```text
prepare → travel → observe → penetrate → discover → extract or establish → return
```

Core pressures:

```text
route uncertainty
supply limits
weather
navigation
hazard knowledge
cargo choices
```

Good outcomes may include knowledge, access, relationship, or a new base—not only loot.

## 3.2 Recovery and Rescue

```text
locate → stabilize → choose priority → transport → handoff → account for losses
```

Subjects may be:

```text
people
machines
animals
source chains
cargo
archives
living samples
```

The mission must avoid treating vulnerable beings as interchangeable packages.

## 3.3 Repair, Replacement, and Commissioning

```text
diagnose → choose repair doctrine → acquire → assemble → test → commission → accept obligation
```

The player may restore the old system, bypass it, replace it, or deliberately decommission it.

## 3.4 Convoy and Logistics

```text
plan → allocate → route → move → respond → deliver → reconcile manifest
```

The core question is not merely whether vehicles reach the endpoint. It is what was prioritized, delayed, exposed, damaged, or diverted.

## 3.5 Investigation

```text
observe → collect sources → compare claims → test hypothesis → identify uncertainty → act or publish
```

Investigation must preserve the possibility that the player cannot yet know everything.

## 3.6 Defense and Evacuation

```text
forecast → prepare → hold or redirect → protect movement → absorb loss → recover → review authority
```

Defense should include terrain, logistics, civilians, machines, and infrastructure—not only enemy elimination.

## 3.7 Infiltration, Sabotage, and Liberation

```text
identify dependency → gain access → alter or expose → survive detection → secure aftermath
```

A successful sabotage that leaves a community without life support is not automatically a heroic success.

## 3.8 Negotiation and Mediation

```text
recognize parties → establish standing → surface protected values → create options → verify commitments → monitor compliance
```

Negotiation must interact with material systems. Dialogue alone cannot create water, cargo capacity, safe habitat, or evidence.

## 3.9 Research and Translation

```text
question → observe → hypothesize → instrument → test → replicate → interpret → diffuse or contain
```

This family is governed by the Science, Research, and Discovery Contract.

## 3.10 Cultural and Social Event

```text
prepare → gather → perform or participate → encounter tension → adapt → remember
```

Examples:

```text
festival
memorial
sport
meal
initiation
public art
repair fair
courtship ritual
```

The event must be enjoyable even if no crisis occurs.

## 3.11 Construction Project

```text
survey → design → obtain standing → stage resources → build → connect → commission → inhabit → maintain
```

Large projects should create intermediate functionality rather than demand one enormous material dump.

## 3.12 First Contact

```text
notice → avoid category violence → establish safe observation → test communication → negotiate boundary → create precedent
```

Combat can occur, but extermination must not be the default completion condition.

# 4. Objective Graph

Missions are directed graphs, not linear checklists.

```rust
struct ActivityGraph {
    activity_id: ActivityId,
    source: ActivitySource,
    entry_conditions: Vec<Condition>,
    nodes: Vec<ActivityNode>,
    edges: Vec<ConditionalEdge>,
    role_slots: Vec<RoleSlot>,
    consequence_outputs: Vec<ConsequenceBinding>,
    closure_conditions: Vec<ClosureCondition>,
}
```

Node types:

```text
Observe
Travel
Acquire
Carry
Operate
Construct
Defend
Negotiate
Test
Witness
Decide
Withdraw
Publish
Celebrate
Mourn
```

Rules:

1. No activity longer than twenty minutes should consist of one repeated node type.
2. Optional nodes must change information, risk, method, or consequence—not only reward quantity.
3. Every major activity needs at least one non-default route.
4. A branch must not be presented as meaningful if all branches collapse into the same world state.
5. The graph should allow clean abandonment when the fiction permits it.

# 5. Role Slots and Co-op

Activities define capabilities, not mandatory classes.

Example slots:

```text
navigator
operator
carrier
protector
witness
medic
negotiator
researcher
builder
scout
```

A solo player may combine slots through tools, preparation, automation, NPC assistance, or reduced operational load.

Co-op rules:

```text
no player should spend the climax watching another use a terminal
support roles must create active decisions
important information should be shareable without screen streaming
a late joiner should receive context and a useful role
rewards should reflect shared outcome, not last-hit ownership
```

# 6. Failure Continuation

Failure should create a new state whenever credible.

Examples:

```text
cargo delivered late rather than erased
bridge damaged and route capacity reduced
hostage relationship worsened rather than quest reset
bad scientific conclusion enters circulation and can later be corrected
a retreat saves people but loses equipment
an unauthorized repair works and creates legitimacy debt
```

Hard resets are appropriate for:

```text
unrecoverable technical corruption
explicit challenge modes
consensual competitive matches
states whose continuation would be incoherent
```

## Failure Output Schema

```rust
struct FailureContinuation {
    physical_losses: Vec<StateDelta>,
    survivors: Vec<EntityId>,
    knowledge_gained: Vec<KnowledgeClaim>,
    obligations_created: Vec<Obligation>,
    new_opportunities: Vec<OpportunitySeed>,
    memory_events: Vec<MemoryEvent>,
}
```

# 7. Reward Grammar

Rewards are changes in credible capability or world relationship.

Valid rewards:

```text
materials
access
knowledge
blueprints
trust
route capacity
labor support
new habitat
cultural belonging
machine cooperation
public legitimacy
strategic position
Chronicle recognition
```

Avoid:

```text
currency detached from the local economy
random rarity inflation
universal reputation points
identical reward crates for morally distinct outcomes
```

# 8. Anti-Errand Rules

Reject an activity when:

```text
its only challenge is distance
it asks the player to collect arbitrary counts without systemic meaning
its requester could trivially perform it
its result disappears immediately
its complication is unrelated random hostility
its only choice is cosmetic dialogue
it repeats an earlier activity without new conditions or mastery
```

A simple delivery can remain valid when the cargo, route, timing, recipient, or opportunity cost matters.

# 9. Authored and Procedural Boundary

Procedural generation may select:

```text
actors
sites
pressure sources
available methods
route conditions
resource constraints
complication candidates
consequence bindings
```

Authored libraries should own:

```text
mission family grammar
high-quality complications
voice and cultural interpretation
unique spaces and set pieces
critical moral boundaries
first-use teaching
major worldline turns
```

Generated prose must never invent canon-critical facts that are not represented in structured state.

# 10. Activity Contract Schema

```rust
struct ActivityContract {
    id: ActivityId,
    title: LocalizedText,
    source: ActivitySource,
    issuer: Option<ActorId>,
    beneficiaries: Vec<ActorId>,
    claimed_problem: Vec<ClaimId>,
    known_facts: Vec<EvidenceId>,
    uncertainties: Vec<UncertaintyId>,
    entry_conditions: Vec<Condition>,
    acceptable_methods: Vec<MethodTag>,
    prohibited_methods: Vec<MethodTag>,
    role_slots: Vec<RoleSlot>,
    time_pressure: Option<TimeWindow>,
    offered_exchange: Vec<ExchangeTerm>,
    success_evidence: Vec<EvidenceRequirement>,
    abandonment_terms: Vec<AbandonmentEffect>,
    consequence_bindings: Vec<ConsequenceBinding>,
}
```

The UI should show whose claims and terms are being presented.

# 11. Seedworks Minimum Grammar Set

The representative build should implement six families:

```text
Expedition
Recovery and Rescue
Repair / Replacement / Commissioning
Convoy and Logistics
Investigation
Defense / Evacuation
```

Minimum content:

```text
2 authored activities per family
3 simulation-driven variants per family
1 failure continuation per authored activity
2 meaningful method branches in at least half the activities
1 social or cultural activity with no mandatory crisis
```

# 12. Acceptance Evidence

The grammar is working when:

```text
players can explain why an activity exists in the world
players use multiple methods without being told there is a choice
failed activities create understandable continuation
repeat variants feel causally different rather than textually shuffled
co-op players report active contribution across roles
players revisit sites because consequences remain visible
no single mission family dominates the first five hours
```
