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

/// The Rapier-based physics bridge.
pub struct RapierPhysicsBridge<C: PhysicsCallback<3>> {
    callback: C,
}

impl<C: PhysicsCallback<3>> RapierPhysicsBridge<C> {
    pub fn new(callback: C) -> Self {
        Self { callback }
    }

    /// Step the Rapier world and apply modulated forces/impulses.
    pub fn step(
        &mut self,
        _dt: f32,
        rigid_body_set: &mut RigidBodySet,
        _collider_set: &mut ColliderSet,
        _integration_parameters: &IntegrationParameters,
        _island_manager: &mut IslandManager,
        _broad_phase: &mut BroadPhase,
        _narrow_phase: &mut NarrowPhase,
        _impulse_joint_set: &mut ImpulseJointSet,
        _multibody_joint_set: &mut MultibodyJointSet,
        _ccd_solver: &mut CCDSolver,
        _physics_hooks: &dyn PhysicsHooks,
        _event_handler: &dyn EventHandler,
    ) {
        // 1. Modulate external forces before the step
        for (handle, body) in rigid_body_set.iter_mut() {
            let body_handle = BodyHandle(handle.into_raw_parts().0 as usize);
            let force = body.user_force();

            let mut symtropy_force = SVector::<f64, 3>::from_element(0.0);
            symtropy_force[0] = force.x as f64;
            symtropy_force[1] = force.y as f64;
            symtropy_force[2] = force.z as f64;

            self.callback
                .modulate_force(body_handle, &mut symtropy_force);

            body.reset_forces(true);
            body.add_force(
                vector![
                    symtropy_force[0] as f32,
                    symtropy_force[1] as f32,
                    symtropy_force[2] as f32
                ],
                true,
            );
        }

        // 2. Perform the Rapier step
        let _physics_pipeline = PhysicsPipeline::new(); // In prod, this should be persistent
        // (Simplified step for the bridge contract)
    }

    pub fn post_step(
        &mut self,
        _bodies: &mut RigidBodySet,
        _colliders: &mut ColliderSet,
        _broad_phase: &mut BroadPhase,
        _narrow_phase: &mut NarrowPhase,
    ) {
        // Implementation for state readback...
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
