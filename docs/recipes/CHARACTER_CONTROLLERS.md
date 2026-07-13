# Character Controllers

For robust, generic 3D character control (walking, jumping, slopes, floating), we recommend [`bevy_tnua`](https://github.com/idanarye/bevy-tnua). It acts as a kinematic wrapper over the rigid body system, allowing tight control while still participating in collisions and Symtropy's coupling fields.

## 1. Dependencies

```toml
[dependencies]
bevy_tnua = "0.19" # Verify Bevy 0.18 compatibility
bevy_tnua_rapier3d = "0.19" # If using rapier3d bridge
```

## 2. Plugin Setup

```rust
use bevy::prelude::*;
use bevy_tnua::prelude::*;
use bevy_tnua_rapier3d::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TnuaRapier3dPlugin)
        .add_plugins(TnuaControllerPlugin)
        .run();
}
```

## 3. Spawning the Character

Attach the `TnuaControllerBundle` alongside your physics components.

```rust
fn spawn_player(mut commands: Commands) {
    commands.spawn((
        // Standard Bevy Transform/Visibility
        TransformBundle::from(Transform::from_xyz(0.0, 10.0, 0.0)),
        
        // Physics Body (Symtropy or Rapier)
        RigidBody::Dynamic,
        Collider::capsule_y(0.5, 0.5),
        
        // Tnua specifics
        TnuaRapier3dIOBundle::default(), // Hook into physics
        TnuaControllerBundle::default(), // The main controller
    ));
}
```

## 4. Applying Movement

Read input and apply it to the `TnuaController` component.

```rust
fn player_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut TnuaController>,
) {
    let mut controller = query.single_mut();
    
    let mut direction = Vec3::ZERO;
    if keyboard.pressed(KeyCode::KeyW) { direction -= Vec3::Z; }
    if keyboard.pressed(KeyCode::KeyS) { direction += Vec3::Z; }
    if keyboard.pressed(KeyCode::KeyA) { direction -= Vec3::X; }
    if keyboard.pressed(KeyCode::KeyD) { direction += Vec3::X; }

    direction = direction.normalize_or_zero();

    controller.basis(TnuaBuiltinWalk {
        desired_velocity: direction * 5.0,
        float_height: 1.0, // Hover height
        ..Default::default()
    });

    if keyboard.pressed(KeyCode::Space) {
        controller.action(TnuaBuiltinJump {
            height: 2.0,
            ..Default::default()
        });
    }
}
```
