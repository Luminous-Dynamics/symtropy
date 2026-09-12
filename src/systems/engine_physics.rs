// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
use crate::resources::PhysicsWorldRes;
use bevy::prelude::*;
use symtropy_render_bridge::PhysicsBody;

/// Advance the authoritative 2D physics world by one validated simulation step.
///
/// With `consciousness-runtime`, the integration field is the physics callback so
/// force/impulse/friction coupling and collision feedback execute inside the same
/// authoritative step. Standalone builds retain the ordinary physics step rather
/// than silently turning this system into a no-op.
pub fn step_physics_world(physics: &mut PhysicsWorldRes, dt: f64) -> bool {
    if !dt.is_finite() || dt <= 0.0 {
        return false;
    }

    #[cfg(feature = "consciousness-runtime")]
    {
        let PhysicsWorldRes {
            world,
            consciousness,
        } = physics;
        world.step_with_callback(dt, consciousness);
    }

    #[cfg(not(feature = "consciousness-runtime"))]
    {
        physics.world.step(dt);
    }

    true
}

pub fn update_physics_consciousness(
    mut physics: ResMut<PhysicsWorldRes>,
    query: Query<(&PhysicsBody, &crate::components::HarmonyComponent)>,
) {
    for (body_comp, harmony) in query.iter() {
        // Sync harmony activations
        if let Some(entity) = physics.consciousness.entities.get_mut(&body_comp.handle) {
            entity.harmony_activations = [
                harmony.activations[0] as f64,
                harmony.activations[1] as f64,
                harmony.activations[2] as f64,
                harmony.activations[3] as f64,
                harmony.activations[4] as f64,
                harmony.activations[5] as f64,
                harmony.activations[6] as f64,
                harmony.activations[7] as f64,
                0.0, // Index 8
            ];
        }
    }
}

/// Advance authoritative physics once, then export body positions to Bevy transforms.
///
/// This function keeps the historical `physics_sync_transforms` name because it is already
/// registered in the launcher's `FixedUpdate` chain. The important authority contract is
/// now explicit: a transform is published only *after* the corresponding physics step.
/// Generic `Time` resolves to Bevy's fixed clock when this system runs in `FixedUpdate`.
pub fn physics_sync_transforms(
    mut physics: ResMut<PhysicsWorldRes>,
    time: Res<Time>,
    mut query: Query<(&PhysicsBody, &mut Transform)>,
) {
    if !step_physics_world(&mut physics, f64::from(time.delta_secs())) {
        return;
    }

    for (body_comp, mut transform) in &mut query {
        if let Some(body) = physics.world.body(body_comp.handle) {
            let pos: nalgebra::SVector<f64, 2> = body.position();
            transform.translation.x = pos[0] as f32;
            transform.translation.y = pos[1] as f32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physics_step_rejects_invalid_time() {
        let mut physics = PhysicsWorldRes::default();
        assert!(!step_physics_world(&mut physics, 0.0));
        assert!(!step_physics_world(&mut physics, -0.1));
        assert!(!step_physics_world(&mut physics, f64::NAN));
        assert!(!step_physics_world(&mut physics, f64::INFINITY));
    }

    #[test]
    fn physics_step_integrates_existing_velocity() {
        let mut physics = PhysicsWorldRes::default();
        let handle = physics
            .world
            .add_sphere(symtropy_math::Point::origin(), 1.0, 1.0);
        physics
            .world
            .body_mut(handle)
            .expect("new body exists")
            .linear_velocity = nalgebra::SVector::from([2.0, 0.0]);

        assert!(step_physics_world(&mut physics, 0.5));

        let x = physics
            .world
            .body(handle)
            .expect("body survives step")
            .position()[0];
        assert!(x > 0.0, "authoritative physics step must integrate velocity");
    }
}
