---
title: Companion Household, Project, Absence, and Worldline Continuity Runtime
version: 0.1
status: implementation-spec
scope: companion life outside party, households, independent projects, schedules, absence, migration, death, forks, persistence
owner: simulation/narrative/persistence
related:
  - ../canon/COMPANION_SHARED_POWER_AND_AUTONOMOUS_AGENCY_CONTRACT_V0_1.md
  - ../canon/COMPANION_CONSENT_REFUSAL_DEPARTURE_AND_DEPENDENCY_CONTRACT_V0_1.md
  - NPC_MEMORY_CONSOLIDATION_LEARNING_AND_WORLDLINE_CONTINUITY_RUNTIME_V0_1.md
  - WORLDLINE_PERSISTENCE_MIGRATION_AND_DISASTER_RECOVERY_PROTOCOL_V0_1.md
---

# Companion Household, Project, Absence, and Worldline Continuity Runtime

## Purpose

A companion must remain a person when not rendered beside the player.

The runtime preserves:

- household membership;
- work and income;
- care responsibilities;
- independent projects;
- institutional roles;
- friendships and rivalries;
- location and travel;
- injuries and recovery;
- political change;
- source-chain identity;
- worldline ancestry.

# 1. Life Anchor

```rust
struct CompanionLifeAnchor {
    agent_id: AgentId,
    home: Option<SiteId>,
    household: Option<HouseholdId>,
    primary_work: Option<WorkRoleId>,
    institutions: Vec<InstitutionMembershipId>,
    care_obligations: Vec<ObligationId>,
    independent_projects: Vec<ProjectId>,
    preferred_places: Vec<SiteId>,
    protected_relationships: Vec<AgentId>,
    current_region: RegionId,
}
```

Travel does not delete these anchors. It creates absence, substitution, delay, or strain.

# 2. Independent Projects

Every flagship companion requires at least one project not created solely by the player.

Examples:

- restore a wetland channel;
- reopen a tool library;
- secure machine-witness standing;
- reconcile a family archive;
- train apprentices;
- organize a transport cooperative;
- compose an unfinished route song;
- investigate a historic workplace injury.

Project state includes:

```text
purpose
beneficiaries
resources
milestones
risks
opponents
collaborators
failure states
player relevance
continuation without player
```

The player may help, harm, ignore, inherit, or misunderstand the project.

# 3. Availability

Companions are not always available.

Availability is derived from:

- work schedules;
- household needs;
- care obligations;
- recovery;
- institution sessions;
- travel time;
- personal projects;
- relationship state;
- current danger;
- chosen rest.

The game may let the player request future coordination or seek another person. It should not solve availability by instant summoning.

# 4. Absence Simulation

For each background interval, simulate:

```text
scheduled work
critical household events
project progress
resource consumption
major relationship interactions
health and recovery
institutional decisions
communications
travel
unexpected pressures
```

Only causally important events become Chronicle candidates. Ordinary continuity still updates state.

# 5. Return Reconciliation

When the player returns, reconciliation presents:

- what changed;
- what the companion knows;
- messages sent and received;
- promises missed;
- work completed;
- injuries or care changes;
- relationship developments;
- project state;
- new availability;
- unresolved questions.

IRIS may summarize only information available through legitimate channels.

# 6. Companion Travel

Travel requires:

- transport capacity;
- supplies;
- destination accommodation;
- role justification when institutional resources are used;
- household or work handover;
- consent;
- return or migration plan.

Species-, body-, and machine-specific needs remain physical.

# 7. Career and Skill Growth

Companions learn through practice, teaching, study, failure, and institutional access.

They may become more capable without the player. They may also lose fluency through injury, disuse, missing tools, or changed body.

Career growth can reduce availability, increase political responsibility, or change the relationship from follower-like travel to peer coordination.

# 8. Household Consequences

Travel may create:

- lost wages;
- childcare or elder-care substitution;
- household conflict;
- loneliness;
- relief;
- new opportunities;
- risk to dependents;
- changes in housing or status.

These are not automatic punishments. They make repeated absence socially legible.

# 9. Departure, Missing, and Death States

```rust
#[derive(Clone, Copy)]
enum CompanionContinuityStatus {
    Active,
    TravelingElsewhere,
    OnLeave,
    Withdrawn,
    Missing,
    Captured,
    DeadUnverified,
    DeadVerified,
    ReconstitutionPending,
    RestoredDisputed,
    Forked,
    Irrecoverable,
}
```

Each state controls evidence, communication, authority, household response, and available actions.

# 10. Worldline Forks

At a worldline fork:

- companion identities receive branch ancestry;
- unique bodies and assets are not duplicated into the same authority domain;
- relationship histories diverge after the fork;
- messages retain branch provenance;
- one branch cannot reveal private events from another;
- reunification, when possible, is a contact event between distinct histories.

# 11. Save and Migration

Persist:

```text
life anchor
current commitments
household state references
projects
availability
practiced procedures
permissions
conflicts
messages
continuity status
source-chain roots
worldline ancestry
```

Migration must preserve stable IDs and create explicit conversion evidence for changed schemas.

# 12. Failure Conditions

Fail if:

- companions freeze while absent;
- project progress ignores resources or collaborators;
- a traveling companion remains simultaneously at work;
- careers advance only when accompanying the player;
- households have no response to prolonged absence;
- restored companions inherit relationships without negotiation;
- forks leak knowledge;
- death deletes unfinished obligations;
- background simulation changes intimate state without causal events;
- IRIS summarizes events it could not know.
