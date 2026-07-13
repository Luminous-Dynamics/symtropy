# Save/Load and Identity

Symtropy leverages **Mycelix** for decentralized identity (DIDs) and secure, cryptographic data storage. For local, immediate-mode game saving in Bevy, we recommend starting with [`bevy_save`](https://github.com/hankjordan/bevy_save) and bridging that state into the Mycelix vault.

## 1. Local Bevy Save

`bevy_save` allows you to serialize the ECS world (or specific components) to disk.

```toml
[dependencies]
bevy_save = "0.13" # Check Bevy 0.18 compatibility
```

### Setup

```rust
use bevy::prelude::*;
use bevy_save::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SavePlugins)
        // Register types you want to save
        .register_type::<Player>()
        .register_type::<SettlementMetrics>()
        .run();
}
```

### Triggering Saves

```rust
fn save_game_system(
    world: &mut World,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::KeyS) {
        // Save the current state to the default workspace
        world.save("save_01").expect("Failed to save");
    }
}
```

## 2. Bridging to Mycelix

In the full Symtropy vision, local saves are merely "cache". The true state resides in the Mycelix personal cluster vault.

When the `mycelix` feature is active, you should wrap your local save data and commit it to the DHT via the Holochain conductor.

```rust
#[cfg(feature = "mycelix")]
use symtropy_mycelix_bridge::MycelixClient;

#[cfg(feature = "mycelix")]
fn sync_to_vault(
    client: Res<MycelixClient>,
    metrics: Res<SettlementMetrics>,
) {
    // Construct a payload
    let payload = serde_json::json!({
        "power": metrics.power,
        "water": metrics.water,
        "trust": metrics.trust,
    });
    
    // Commit to the decentralized vault
    client.commit_to_vault("settlement_state", payload);
}
```
