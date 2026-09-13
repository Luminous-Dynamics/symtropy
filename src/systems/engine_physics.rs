// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
use crate::resources::PhysicsWorldRes;
use bevy::prelude::*;
use symtropy_render_bridge::PhysicsBody;

/// Advance the authoritative 2D physics world by one validated simulation step.
///
/// This is intentionally narrower than scheduler authority: it validates only the
/// timestep and performs exactly one world step. Fixed-tick identity, consequence
/// status, retry behavior, and friction transaction ownership remain outside this
/// helper.
///
/// With `consciousness-runtime`, the integration field remains the physics callback
/// so existing force/impulse/friction coupling and collision feedback execute inside
/// the same authoritative step. Standalone builds retain the ordinary physics step.
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

/// Sync physics body positions back to Bevy Transforms.
pub fn physics_sync_transforms(
    physics: Res<PhysicsWorldRes>,
    mut query: Query<(&PhysicsBody, &mut Transform)>,
) {
    for (body_comp, mut transform) in &mut query {
        if let Some(body) = physics.world.body(body_comp.handle) {
            let pos: nalgebra::SVector<f64, 2> = body.position();
            transform.translation.x = pos[0] as f32;
            transform.translation.y = pos[1] as f32;
        }
    }
}
