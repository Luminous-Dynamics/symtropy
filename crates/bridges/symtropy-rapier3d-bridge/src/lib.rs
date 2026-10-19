// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Bridge between Rapier3D and Symtropy's state-coupling framework.

pub mod manipulator;

use ::nalgebra::SVector;
use rapier3d::prelude::*;
use symtropy_physics::body::BodyHandle;
use symtropy_physics::world::PhysicsCallback;

use bevy::prelude::*;

pub use manipulator::{
    Manipulator8D, ManipulatorTarget, manipulator_motor_system, spawn_manipulator_8d,
};

/// Wrapper to make Rapier RigidBody a Bevy Component.
#[derive(Component)]
pub struct RapierRigidBody(pub RigidBody);

/// Wrapper to make Rapier Collider a Bevy Component.
#[derive(Component)]
pub struct RapierCollider(pub Collider);

#[derive(Resource, Default)]
pub struct RapierRigidBodySet(pub RigidBodySet);

#[derive(Resource, Default)]
pub struct RapierColliderSet(pub ColliderSet);

#[derive(Resource, Default)]
pub struct RapierImpulseJointSet(pub ImpulseJointSet);

#[derive(Resource, Default)]
pub struct RapierIslandManager(pub IslandManager);

#[derive(Resource, Default)]
pub struct RapierMultibodyJointSet(pub MultibodyJointSet);

/// Machine-readable body observation captured from Rapier after a completed step.
///
/// Quaternion values use `[w, i, j, k]` order. The Rapier handle is retained
/// rather than collapsing it to a slot index, because Rapier reuses slots with
/// generations and a slot index alone is not a stable identity.
#[derive(Clone, Debug, PartialEq)]
pub struct RapierBodyState {
    pub handle: RigidBodyHandle,
    pub translation: [f32; 3],
    pub rotation_wijk: [f32; 4],
    pub linear_velocity: [f32; 3],
    pub angular_velocity: [f32; 3],
    pub sleeping: bool,
}

/// Errors that prevent a Rapier step from being executed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RapierBridgeStepError {
    InvalidTimeStep,
    NonFiniteGravity,
    NonFiniteModulatedForce,
}

impl std::fmt::Display for RapierBridgeStepError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTimeStep => write!(f, "dt must be finite and strictly positive"),
            Self::NonFiniteGravity => write!(f, "gravity must contain only finite components"),
            Self::NonFiniteModulatedForce => {
                write!(f, "force modulation produced a non-finite or unrepresentable force")
            }
        }
    }
}

impl std::error::Error for RapierBridgeStepError {}

/// The Rapier-based physics bridge.
///
/// The pipeline is retained across steps so its temporary buffers can be reused.
/// Gravity is supplied explicitly at each call; the bridge does not silently
/// choose a gravity convention for the caller.
pub struct RapierPhysicsBridge<C: PhysicsCallback<3>> {
    callback: C,
    physics_pipeline: PhysicsPipeline,
    last_body_states: Vec<RapierBodyState>,
}

impl<C: PhysicsCallback<3>> RapierPhysicsBridge<C> {
    pub fn new(callback: C) -> Self {
        Self {
            callback,
            physics_pipeline: PhysicsPipeline::new(),
            last_body_states: Vec::new(),
        }
    }

    /// Step Rapier, applying Symthaea/Symtropy force modulation before integration.
    ///
    /// The supplied `dt` is copied into a cloned `IntegrationParameters` value
    /// and that exact f32 is passed to Rapier. Invalid inputs or unrepresentable
    /// modulated forces are rejected before any rigid-body force is mutated.
    #[allow(clippy::too_many_arguments)]
    pub fn step(
        &mut self,
        dt: f32,
        gravity: &Vector<Real>,
        rigid_body_set: &mut RigidBodySet,
        collider_set: &mut ColliderSet,
        integration_parameters: &IntegrationParameters,
        island_manager: &mut IslandManager,
        broad_phase: &mut BroadPhase,
        narrow_phase: &mut NarrowPhase,
        impulse_joint_set: &mut ImpulseJointSet,
        multibody_joint_set: &mut MultibodyJointSet,
        ccd_solver: &mut CCDSolver,
        query_pipeline: Option<&mut QueryPipeline>,
        physics_hooks: &dyn PhysicsHooks,
        event_handler: &dyn EventHandler,
    ) -> Result<(), RapierBridgeStepError> {
        // Clear observations first so a rejected step cannot leave a previous
        // successful state snapshot looking like evidence for this attempt.
        self.last_body_states.clear();

        if !dt.is_finite() || dt <= 0.0 {
            return Err(RapierBridgeStepError::InvalidTimeStep);
        }
        if !gravity.iter().all(|value| value.is_finite()) {
            return Err(RapierBridgeStepError::NonFiniteGravity);
        }

        let mut parameters = integration_parameters.clone();
        parameters.dt = dt;

        // Preflight every body before applying any rewritten force. This keeps
        // invalid modulation from leaving half the set mutated.
        let mut modulated_forces = Vec::new();
        for (handle, body) in rigid_body_set.iter() {
            let rapier_force = body.user_force();
            let symtropy_force = SVector::<f64, 3>::from([
                rapier_force.x as f64,
                rapier_force.y as f64,
                rapier_force.z as f64,
            ]);
            let modulated = self.callback.modulate_force(
                BodyHandle(handle.into_raw_parts().0 as usize),
                &symtropy_force,
            );
            let force = vector![
                modulated[0] as f32,
                modulated[1] as f32,
                modulated[2] as f32
            ];
            if !modulated.iter().all(|value| value.is_finite())
                || !force.iter().all(|value| value.is_finite())
            {
                return Err(RapierBridgeStepError::NonFiniteModulatedForce);
            }
            modulated_forces.push((handle, force));
        }

        for (handle, force) in modulated_forces {
            let body = rigid_body_set
                .get_mut(handle)
                .expect("body handles cannot change during this exclusive step call");
            body.reset_forces(true);
            body.add_force(force, true);
        }

        self.physics_pipeline.step(
            gravity,
            &parameters,
            island_manager,
            broad_phase,
            narrow_phase,
            rigid_body_set,
            collider_set,
            impulse_joint_set,
            multibody_joint_set,
            ccd_solver,
            query_pipeline,
            physics_hooks,
            event_handler,
        );

        self.post_step(rigid_body_set);
        Ok(())
    }

    /// Refresh the readback snapshot from the current Rapier body set.
    ///
    /// Normally called by `step` after a successful simulation. It is public
    /// so callers that make additional Rapier-side state changes can explicitly
    /// refresh observations afterward.
    pub fn post_step(&mut self, bodies: &RigidBodySet) {
        self.last_body_states = bodies
            .iter()
            .map(|(handle, body)| {
                let translation = body.translation();
                let rotation = body.rotation().quaternion();
                let linear_velocity = body.linvel();
                let angular_velocity = body.angvel();

                RapierBodyState {
                    handle,
                    translation: [translation.x, translation.y, translation.z],
                    rotation_wijk: [rotation.w, rotation.i, rotation.j, rotation.k],
                    linear_velocity: [
                        linear_velocity.x,
                        linear_velocity.y,
                        linear_velocity.z,
                    ],
                    angular_velocity: [
                        angular_velocity.x,
                        angular_velocity.y,
                        angular_velocity.z,
                    ],
                    sleeping: body.is_sleeping(),
                }
            })
            .collect();
    }

    /// Return the observations from the most recent successful step.
    pub fn last_body_states(&self) -> &[RapierBodyState] {
        &self.last_body_states
    }
}

/// Helper to add a sphere to the Rapier world.
pub fn add_sphere_to_rapier(
    rigid_body_set: &mut RigidBodySet,
    collider_set: &mut ColliderSet,
    translation: Vec3,
    radius: f32,
    mass: f32,
) -> RigidBodyHandle {
    let rigid_body = RigidBodyBuilder::dynamic()
        .translation(vector![translation.x, translation.y, translation.z])
        .additional_mass(mass)
        .build();
    let handle = rigid_body_set.insert(rigid_body);
    let collider = ColliderBuilder::ball(radius).build();
    collider_set.insert_with_parent(collider, handle, rigid_body_set);
    handle
}
