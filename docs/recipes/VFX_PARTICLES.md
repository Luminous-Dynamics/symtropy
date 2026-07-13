# VFX Particles

For high-performance GPU particle systems (sparks from repairing a junction, dust in the dungeon, magical "Phi" auras), we recommend [`bevy_hanabi`](https://github.com/djeedai/bevy_hanabi).

## 1. Dependencies

```toml
[dependencies]
bevy_hanabi = "0.12" # Check Bevy 0.18 compatibility
```

## 2. Plugin Setup

```rust
use bevy::prelude::*;
use bevy_hanabi::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(HanabiPlugin)
        .run();
}
```

## 3. Creating an Effect

Here is a recipe for a "Spark" effect when repairing infrastructure.

```rust
fn setup_sparks(mut commands: Commands, mut effects: ResMut<Assets<EffectAsset>>) {
    // Define a color gradient
    let mut gradient = Gradient::new();
    gradient.add_key(0.0, Vec4::new(1.0, 1.0, 0.0, 1.0)); // Yellow
    gradient.add_key(1.0, Vec4::new(1.0, 0.0, 0.0, 0.0)); // Red, fading out

    // Define the particle effect
    let effect = EffectAsset::new(32768, Spawner::once(100.0.into(), true), writer.finish())
        .with_name("Sparks")
        .init(InitPositionSphereModifier {
            center: Vec3::ZERO,
            radius: 0.1,
            dimension: ShapeDimension::Surface,
        })
        .init(InitVelocitySphereModifier {
            center: Vec3::ZERO,
            speed: 5.0.into(),
        })
        .update(AccelModifier::constant(Vec3::new(0.0, -9.81, 0.0))) // Gravity
        .render(ColorOverLifetimeModifier { gradient });

    // Save the asset handle to spawn later
    commands.insert_resource(SparkEffectHandle(effects.add(effect)));
}
```

## 4. Spawning the Effect

When an interaction happens (e.g., player clicks the Power Junction), spawn the emitter.

```rust
fn repair_system(
    mut commands: Commands,
    spark_handle: Res<SparkEffectHandle>,
    // ... other queries
) {
    // On repair action:
    commands.spawn((
        ParticleEffectBundle::new(spark_handle.0.clone())
            .with_spawner(Spawner::once(50.0.into(), true)),
        TransformBundle::from(Transform::from_translation(junction_position)),
    ));
}
```
