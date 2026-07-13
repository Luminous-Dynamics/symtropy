# NPC AI: When to use `big-brain` vs `symthaea-bevy-brain`

Symtropy provides two paths for NPC behavior. Choosing the right one depends on the importance of the NPC to the thermodynamic and social simulation.

## 1. Utility AI (`big-brain`)

Use [`big-brain`](https://github.com/zkat/big-brain) for **ambient, low-impact NPCs** (e.g., background wildlife, simple drones, crowd extras).

**Why?** It is fast, highly scalable, and excellent for simple state machines based on utility curves.

**Recipe:**
```rust
use bevy::prelude::*;
use big_brain::prelude::*;

#[derive(Clone, Component, Debug, ActionBuilder)]
pub struct MoveToWaterPump;

fn move_to_water_pump_action_system(
    mut action_query: Query<(&Actor, &mut ActionState, &MoveToWaterPump)>,
    mut transforms: Query<&mut Transform>,
    time: Res<Time>,
) {
    for (Actor(actor), mut state, _) in &mut action_query {
        if let Ok(mut tf) = transforms.get_mut(*actor) {
            // Move logic here
            *state = ActionState::Success;
        }
    }
}
```

## 2. The Cognitive Loop (`symthaea-bevy-brain`)

Use `symthaea-bevy-brain` for **core simulation actors** (e.g., Faction leaders, key outpost crew, hostile intelligences).

**Why?** It integrates directly with the physics engine. The agent's perception is driven by Hyperdimensional Computing (HDC) and its behavior minimizes Variational Free Energy (FEP). Crucially, its motor authority is modulated by its **Phi (Φ) level**.

**Recipe:**
```rust
use symthaea_bevy_brain::{CognitiveBrain, SymthaeaBrainPlugin};

fn spawn_conscious_npc(mut commands: Commands) {
    commands.spawn((
        // Standard Bevy components
        TransformBundle::default(),
        
        // Add the Cognitive Brain
        CognitiveBrain::new(
            8, // HDC State dimensions
            6, // Observation dimensions (e.g., Energy, Stress, Danger, Caution, Water, Power)
        ),
    ));
}
```

### The Difference in Practice

A `big-brain` NPC *always* moves at its set speed.
A `symthaea` NPC's movement speed and force application drops to zero if its internal harmonic resonance (consciousness) collapses. They are thermodynamic entities.
