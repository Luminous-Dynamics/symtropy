---
title: NPC AI Runtime Selection Recipe
version: 0.2
status: supporting
scope: implementation guidance for selecting reactive, utility, planner, and Symthaea-backed NPC runtimes
owner: AI/engineering
related:
  - tech/NPC_COGNITION_AGENCY_AND_SIMULATION_RUNTIME_V0_1.md
  - vision/NPC_DAILY_LIFE_RELATIONSHIPS_AND_SOCIAL_MEMORY_BIBLE_V0_2.md
---

# NPC AI Runtime Selection Recipe

Use the cheapest runtime that preserves the agent's required consequences.

Do not divide the population into “simple AI” and “conscious AI.” Symtropy uses several interchangeable decision policies behind one validated action contract.

## Runtime Selection

| Agent need | Recommended policy | Typical cadence | Persistence |
|---|---|---:|---|
| Immediate hazard response | reactive policy | frame/event | none or compact |
| Ambient work and routines | utility arbitration (`big-brain` or equivalent) | 1–4 Hz | schedule + need summary |
| Named citizen with obligations | bounded goal planner + memory | 0.2–1 Hz / event | persistent beliefs and relationships |
| Complex machine or nonhuman agent | specialized viability policy | event-driven | protected values and boundary state |
| Research hero agent | Symthaea-backed proposals behind validated actions | configurable | explicit and auditable |

## Shared Action Boundary

Every runtime should emit requests such as:

```rust
pub enum AgentActionRequest {
    MoveTo(Entity),
    TransferCargo { item: Entity, destination: Entity },
    OperateDevice { device: Entity, operation: OperationId },
    Speak(DialogueFrame),
    Refuse { obligation: ObligationId, reason: ReasonCode },
}
```

The owning gameplay system validates the request. An AI package must never mutate inventory, permissions, Chronicle history, or civic state directly.

## Utility Example: Ambient Cargo Worker

```rust
use bevy::prelude::*;
use big_brain::prelude::*;

#[derive(Clone, Component, Debug, ActionBuilder)]
pub struct MoveCrateToDepot;

fn move_crate_action_system(
    mut actions: Query<(&Actor, &mut ActionState), With<MoveCrateToDepot>>,
    workers: Query<&AssignedCargo>,
    depots: Query<&DepotState>,
) {
    for (Actor(actor), mut state) in &mut actions {
        let Ok(cargo) = workers.get(*actor) else {
            *state = ActionState::Failure;
            continue;
        };

        if depots.get(cargo.destination).is_err() {
            *state = ActionState::Failure;
            continue;
        }

        // Submit a validated movement / transfer request here.
        *state = ActionState::Success;
    }
}
```

Scores should include travel cost, fatigue, role obligation, danger, available equipment, and social interruption—not only distance to a target.

## Named Citizen Example

A named courier may generate these intents:

```text
deliver medicine before refrigeration fails
help sibling evacuate a flooded district
return a borrowed rover
avoid a checkpoint controlled by a distrusted faction
```

The planner selects a short route through authored actions. If a bridge is destroyed or permission is denied, it replans or asks for help instead of teleporting the result.

## Symthaea-Backed Runtime

Use Symthaea only when the feature being tested requires it. Candidate uses include attention, prediction error, memory retrieval, uncertainty, or motor-intent proposals.

```rust
#[derive(Component)]
pub struct ExperimentalCognitivePolicy {
    pub enabled: bool,
    pub fallback: FallbackPolicy,
    pub observation_schema: ObservationSchemaId,
    pub action_schema: ActionSchemaId,
}
```

Requirements:

```text
feature flag
a baseline policy for comparison
recorded inputs and outputs
safe fallback
no direct world mutation
no claim that a scalar proves consciousness or personhood
```

## Simulation LOD

When an agent leaves the active area:

```text
full navigation → schedule encounter summary → regional work outcome
```

Do not continue frame-level pathfinding off-screen. Preserve obligations, relationship encounters, injuries, travel time, and resource accounting.

## Debugging Checklist

For each named agent, developers should be able to inspect:

```text
what it perceived
what it currently believes
what it needs
what it promised
which intents were considered
why one intent won
what action was requested
why the world accepted or rejected it
what memory changed afterward
```

The canonical requirements are defined in [NPC Cognition, Agency, and Simulation Runtime Contract](../tech/NPC_COGNITION_AGENCY_AND_SIMULATION_RUNTIME_V0_1.md).
