// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
use std::convert::Infallible;

use crate::resources::PhysicsWorldRes;
use bevy::prelude::*;
use symtropy_render_bridge::PhysicsBody;

/// Ordinary pre-step rejection reasons that prove the consequential physics step
/// never began.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PhysicsConsequenceRejection {
    InvalidTimestep,
}

/// Typed result of one scheduled physics consequence.
///
/// `Rejected` is reserved for cases such as invalid `dt` where the consequential
/// world step provably did not begin. `AuthorityFault` is a separate fail-stop
/// channel for a future authority-aware solver step that may have entered solver
/// execution before discovering an unrecoverable authority failure.
///
/// There is deliberately no generic conversion of this type to `bool`: an
/// inhabited authority error must be handled explicitly rather than collapsed
/// into ordinary rejection.
#[derive(Debug, PartialEq, Eq)]
pub enum PhysicsConsequenceOutcome<E> {
    Executed,
    Rejected(PhysicsConsequenceRejection),
    AuthorityFault(E),
}

/// Advance the current legacy authoritative 2D world and preserve why a
/// consequence did or did not execute.
///
/// The legacy world API cannot currently return a solver-authority fault, so the
/// error parameter is intentionally [`Infallible`]. Once `PhysicsWorld` exposes
/// the #809 authority-aware step, that path should return the same outcome shape
/// with its typed friction-step/runtime error instead of reusing this function.
pub fn step_physics_world_outcome(
    physics: &mut PhysicsWorldRes,
    dt: f64,
) -> PhysicsConsequenceOutcome<Infallible> {
    if !dt.is_finite() || dt <= 0.0 {
        return PhysicsConsequenceOutcome::Rejected(PhysicsConsequenceRejection::InvalidTimestep);
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

    PhysicsConsequenceOutcome::Executed
}

/// Advance the authoritative 2D physics world by one validated simulation step.
///
/// This compatibility wrapper intentionally exists only over the current
/// [`Infallible`] authority-fault type. It can map invalid-`dt` rejection to
/// `false`, but there is no generic helper that maps an inhabited authority fault
/// to `false`.
///
/// Fixed-tick identity, consequence status, retry behavior, and friction
/// transaction ownership remain outside this legacy helper.
pub fn step_physics_world(physics: &mut PhysicsWorldRes, dt: f64) -> bool {
    match step_physics_world_outcome(physics, dt) {
        PhysicsConsequenceOutcome::Executed => true,
        PhysicsConsequenceOutcome::Rejected(_) => false,
        PhysicsConsequenceOutcome::AuthorityFault(never) => match never {},
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_timestep_is_typed_as_pre_step_rejection() {
        let mut physics = PhysicsWorldRes::default();

        for dt in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert_eq!(
                step_physics_world_outcome(&mut physics, dt),
                PhysicsConsequenceOutcome::Rejected(
                    PhysicsConsequenceRejection::InvalidTimestep
                )
            );
        }
    }

    #[test]
    fn valid_legacy_step_is_executed_not_rejected() {
        let mut physics = PhysicsWorldRes::default();
        assert_eq!(
            step_physics_world_outcome(&mut physics, 1.0 / 60.0),
            PhysicsConsequenceOutcome::Executed
        );
    }

    #[test]
    fn authority_fault_is_structurally_distinct_from_rejection() {
        let outcome: PhysicsConsequenceOutcome<&'static str> =
            PhysicsConsequenceOutcome::AuthorityFault("friction authority failed");
        assert!(matches!(
            outcome,
            PhysicsConsequenceOutcome::AuthorityFault("friction authority failed")
        ));
    }

    #[test]
    fn legacy_bool_wrapper_only_maps_pre_step_rejection() {
        let mut physics = PhysicsWorldRes::default();
        assert!(!step_physics_world(&mut physics, 0.0));
        assert!(step_physics_world(&mut physics, 1.0 / 60.0));
    }
}
