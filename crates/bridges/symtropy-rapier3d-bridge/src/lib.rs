// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Bridge between Rapier3D and Symtropy's state-coupling framework.
//!
//! Rapier is a differential/reference backend here, not a source of canonical
//! Symtropy physical truth. The bridge keeps forcing, timestep, identity mapping,
//! and post-step observations explicit so later validation code can compare
//! independent implementations without silently changing gameplay authority.

pub mod manipulator;

use std::collections::BTreeMap;

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

/// Dependency-light post-step state used by differential validation.
///
/// The raw Rapier handle keeps both arena index and generation so measurement
/// identity is not accidentally weakened to an index-only view.
#[derive(Clone, Debug, PartialEq)]
pub struct RapierBodyObservation {
    pub raw_handle: (u32, u32),
    pub translation: [f32; 3],
    pub linear_velocity: [f32; 3],
    pub angular_velocity: [f32; 3],
    pub sleeping: bool,
}

/// The Rapier-based physics bridge.
///
/// `PhysicsPipeline` is persistent because Rapier uses it as reusable scratch
/// storage. Gravity defaults to zero so this adapter never invents an Earth
/// boundary condition; reference scenarios must opt into gravity explicitly.
///
/// The consciousness callback's `BodyHandle` identity is mapped explicitly from
/// a full Rapier `(index, generation)` handle. Unmapped bodies are intentionally
/// not sent through the callback: arena indices are not valid cross-engine
/// identities and can be reused after removal.
pub struct RapierPhysicsBridge<C: PhysicsCallback<3>> {
    callback: C,
    physics_pipeline: PhysicsPipeline,
    gravity: Vec3,
    callback_body_map: BTreeMap<(u32, u32), BodyHandle>,
    last_observations: Vec<RapierBodyObservation>,
    step_count: u64,
}

impl<C: PhysicsCallback<3>> RapierPhysicsBridge<C> {
    pub fn new(callback: C) -> Self {
        Self {
            callback,
            physics_pipeline: PhysicsPipeline::new(),
            gravity: Vec3::ZERO,
            callback_body_map: BTreeMap::new(),
            last_observations: Vec::new(),
            step_count: 0,
        }
    }

    /// Construct a reference bridge with an explicit gravity vector.
    pub fn with_gravity(callback: C, gravity: Vec3) -> Self {
        let mut bridge = Self::new(callback);
        bridge.set_gravity(gravity);
        bridge
    }

    /// Replace the reference gravity vector.
    ///
    /// Gravity is scenario configuration, never inferred from renderer/world
    /// presentation state.
    pub fn set_gravity(&mut self, gravity: Vec3) {
        assert!(
            gravity.x.is_finite() && gravity.y.is_finite() && gravity.z.is_finite(),
            "Rapier reference gravity must be finite"
        );
        self.gravity = gravity;
    }

    pub fn gravity(&self) -> Vec3 {
        self.gravity
    }

    pub fn step_count(&self) -> u64 {
        self.step_count
    }

    /// Bind one full Rapier handle to the Symtropy body identity expected by
    /// `PhysicsCallback`.
    ///
    /// A reused Rapier arena index with a new generation does not inherit the
    /// previous binding because the complete raw handle is the map key.
    pub fn bind_callback_body(
        &mut self,
        rapier_handle: RigidBodyHandle,
        symtropy_handle: BodyHandle,
    ) -> Option<BodyHandle> {
        self.callback_body_map
            .insert(rapier_handle.into_raw_parts(), symtropy_handle)
    }

    /// Remove a callback identity binding for a Rapier body.
    pub fn unbind_callback_body(&mut self, rapier_handle: RigidBodyHandle) -> Option<BodyHandle> {
        self.callback_body_map.remove(&rapier_handle.into_raw_parts())
    }

    /// Resolve the callback identity bound to a Rapier body.
    pub fn callback_body(&self, rapier_handle: RigidBodyHandle) -> Option<BodyHandle> {
        self.callback_body_map
            .get(&rapier_handle.into_raw_parts())
            .copied()
    }

    /// Observations captured immediately after the latest successful step or
    /// explicit `post_step` refresh.
    pub fn last_observations(&self) -> &[RapierBodyObservation] {
        &self.last_observations
    }

    /// Step the Rapier world and apply Symtropy callback force modulation.
    ///
    /// The supplied integration profile is copied so all caller-selected Rapier
    /// settings remain intact while `dt` is made explicit at this bridge
    /// boundary. No query pipeline is maintained yet; differential scene-query
    /// coverage belongs in a later tranche.
    #[allow(clippy::too_many_arguments)]
    pub fn step(
        &mut self,
        dt: f32,
        rigid_body_set: &mut RigidBodySet,
        collider_set: &mut ColliderSet,
        integration_parameters: &IntegrationParameters,
        island_manager: &mut IslandManager,
        broad_phase: &mut BroadPhase,
        narrow_phase: &mut NarrowPhase,
        impulse_joint_set: &mut ImpulseJointSet,
        multibody_joint_set: &mut MultibodyJointSet,
        ccd_solver: &mut CCDSolver,
        physics_hooks: &dyn PhysicsHooks,
        event_handler: &dyn EventHandler,
    ) {
        assert!(
            dt.is_finite() && dt >= 0.0,
            "Rapier reference dt must be finite and non-negative"
        );

        // 1. Modulate explicit user forces only when the caller has supplied an
        // explicit cross-engine identity binding. Never reinterpret Rapier's
        // recyclable arena index as a Symtropy body identity.
        for (handle, body) in rigid_body_set.iter_mut() {
            let Some(body_handle) = self.callback_body_map.get(&handle.into_raw_parts()).copied()
            else {
                continue;
            };

            let force = body.user_force();
            let mut symtropy_force = SVector::<f64, 3>::from_element(0.0);
            symtropy_force[0] = force.x as f64;
            symtropy_force[1] = force.y as f64;
            symtropy_force[2] = force.z as f64;

            let symtropy_force = self.callback.modulate_force(body_handle, &symtropy_force);

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

        // 2. Preserve every caller-selected Rapier integration setting except
        // the timestep, which is the explicit argument of this bridge step.
        let mut step_parameters = *integration_parameters;
        step_parameters.dt = dt;
        let gravity = vector![self.gravity.x, self.gravity.y, self.gravity.z];

        // 3. Advance the actual Rapier simulation. Rapier 0.18 accepts an
        // optional QueryPipeline; this first differential tranche keeps query
        // state out of the authority surface and passes None deliberately.
        self.physics_pipeline.step(
            &gravity,
            &step_parameters,
            island_manager,
            broad_phase,
            narrow_phase,
            rigid_body_set,
            collider_set,
            impulse_joint_set,
            multibody_joint_set,
            ccd_solver,
            None,
            physics_hooks,
            event_handler,
        );

        self.step_count = self.step_count.saturating_add(1);
        self.capture_observations(rigid_body_set);
    }

    /// Refresh post-step measurements without advancing the simulation.
    ///
    /// The additional arguments are retained for source compatibility with the
    /// existing bridge surface. Future differential contact/query observations
    /// may consume them explicitly.
    pub fn post_step(
        &mut self,
        bodies: &mut RigidBodySet,
        _colliders: &mut ColliderSet,
        _broad_phase: &mut BroadPhase,
        _narrow_phase: &mut NarrowPhase,
    ) {
        self.capture_observations(bodies);
    }

    fn capture_observations(&mut self, bodies: &RigidBodySet) {
        self.last_observations.clear();
        self.last_observations.reserve(bodies.len());

        for (handle, body) in bodies.iter() {
            let translation = body.translation();
            let linear_velocity = body.linvel();
            let angular_velocity = body.angvel();

            self.last_observations.push(RapierBodyObservation {
                raw_handle: handle.into_raw_parts(),
                translation: [translation.x, translation.y, translation.z],
                linear_velocity: [linear_velocity.x, linear_velocity.y, linear_velocity.z],
                angular_velocity: [angular_velocity.x, angular_velocity.y, angular_velocity.z],
                sleeping: body.is_sleeping(),
            });
        }

        // Arena iteration is normally stable, but evidence should not depend on
        // an undocumented container traversal order.
        self.last_observations
            .sort_by_key(|observation| observation.raw_handle);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use symtropy_physics::contact::CollisionEvent;
    use symtropy_physics::world::NoOpCallback;

    fn empty_rapier_state() -> (
        RigidBodySet,
        ColliderSet,
        IntegrationParameters,
        IslandManager,
        BroadPhase,
        NarrowPhase,
        ImpulseJointSet,
        MultibodyJointSet,
        CCDSolver,
    ) {
        (
            RigidBodySet::new(),
            ColliderSet::new(),
            IntegrationParameters::default(),
            IslandManager::new(),
            BroadPhase::new(),
            NarrowPhase::new(),
            ImpulseJointSet::new(),
            MultibodyJointSet::new(),
            CCDSolver::new(),
        )
    }

    #[test]
    fn default_reference_gravity_is_zero() {
        let mut bridge = RapierPhysicsBridge::new(NoOpCallback);
        assert_eq!(bridge.gravity(), Vec3::ZERO);

        let (
            mut bodies,
            mut colliders,
            parameters,
            mut islands,
            mut broad_phase,
            mut narrow_phase,
            mut impulse_joints,
            mut multibody_joints,
            mut ccd,
        ) = empty_rapier_state();
        let handle = add_sphere_to_rapier(
            &mut bodies,
            &mut colliders,
            Vec3::new(0.0, 10.0, 0.0),
            0.5,
            1.0,
        );

        bridge.step(
            1.0 / 60.0,
            &mut bodies,
            &mut colliders,
            &parameters,
            &mut islands,
            &mut broad_phase,
            &mut narrow_phase,
            &mut impulse_joints,
            &mut multibody_joints,
            &mut ccd,
            &(),
            &(),
        );

        assert_eq!(bodies[handle].translation().y, 10.0);
        assert_eq!(bodies[handle].linvel().y, 0.0);
    }

    #[test]
    fn rapier_bridge_step_actually_advances_under_explicit_gravity() {
        let (
            mut bodies,
            mut colliders,
            parameters,
            mut islands,
            mut broad_phase,
            mut narrow_phase,
            mut impulse_joints,
            mut multibody_joints,
            mut ccd,
        ) = empty_rapier_state();

        let handle = add_sphere_to_rapier(
            &mut bodies,
            &mut colliders,
            Vec3::new(0.0, 10.0, 0.0),
            0.5,
            1.0,
        );
        let mut bridge =
            RapierPhysicsBridge::with_gravity(NoOpCallback, Vec3::new(0.0, -9.81, 0.0));

        bridge.step(
            1.0 / 60.0,
            &mut bodies,
            &mut colliders,
            &parameters,
            &mut islands,
            &mut broad_phase,
            &mut narrow_phase,
            &mut impulse_joints,
            &mut multibody_joints,
            &mut ccd,
            &(),
            &(),
        );

        assert!(bodies[handle].translation().y < 10.0);
        assert!(bodies[handle].linvel().y < 0.0);
        assert_eq!(bridge.step_count(), 1);
        assert_eq!(bridge.last_observations().len(), 1);
        assert_eq!(
            bridge.last_observations()[0].raw_handle,
            handle.into_raw_parts()
        );
    }

    #[test]
    fn bridge_dt_controls_reference_trajectory() {
        fn step_once(dt: f32) -> f32 {
            let (
                mut bodies,
                mut colliders,
                parameters,
                mut islands,
                mut broad_phase,
                mut narrow_phase,
                mut impulse_joints,
                mut multibody_joints,
                mut ccd,
            ) = empty_rapier_state();
            let handle = add_sphere_to_rapier(
                &mut bodies,
                &mut colliders,
                Vec3::new(0.0, 10.0, 0.0),
                0.5,
                1.0,
            );
            let mut bridge =
                RapierPhysicsBridge::with_gravity(NoOpCallback, Vec3::new(0.0, -9.81, 0.0));

            bridge.step(
                dt,
                &mut bodies,
                &mut colliders,
                &parameters,
                &mut islands,
                &mut broad_phase,
                &mut narrow_phase,
                &mut impulse_joints,
                &mut multibody_joints,
                &mut ccd,
                &(),
                &(),
            );

            bodies[handle].translation().y
        }

        let y_short = step_once(1.0 / 120.0);
        let y_long = step_once(1.0 / 30.0);
        assert!(y_long < y_short);
    }

    #[test]
    #[should_panic(expected = "Rapier reference dt must be finite and non-negative")]
    fn non_finite_dt_is_rejected_before_stepping() {
        let (
            mut bodies,
            mut colliders,
            parameters,
            mut islands,
            mut broad_phase,
            mut narrow_phase,
            mut impulse_joints,
            mut multibody_joints,
            mut ccd,
        ) = empty_rapier_state();
        let mut bridge = RapierPhysicsBridge::new(NoOpCallback);

        bridge.step(
            f32::NAN,
            &mut bodies,
            &mut colliders,
            &parameters,
            &mut islands,
            &mut broad_phase,
            &mut narrow_phase,
            &mut impulse_joints,
            &mut multibody_joints,
            &mut ccd,
            &(),
            &(),
        );
    }

    struct CountingCallback {
        force_calls: Cell<usize>,
        last_body: Cell<Option<BodyHandle>>,
    }

    impl PhysicsCallback<3> for CountingCallback {
        fn modulate_force(&self, body: BodyHandle, force: &SVector<f64, 3>) -> SVector<f64, 3> {
            self.force_calls.set(self.force_calls.get() + 1);
            self.last_body.set(Some(body));
            *force
        }

        fn modulate_impulse(&self, impulse: f64, _contact_point: &SVector<f64, 3>) -> f64 {
            impulse
        }

        fn friction_multiplier(&self, _contact_point: &SVector<f64, 3>, _body: BodyHandle) -> f64 {
            1.0
        }

        fn on_collision(&mut self, _event: &CollisionEvent<3>) {}
        fn record_dissipation(&mut self, _energy: f64) {}
        fn record_work(&mut self, _body: BodyHandle, _work_joules: f64) {}
        fn apply_trauma(&mut self, _event: &CollisionEvent<3>) {}
    }

    #[test]
    fn unmapped_rapier_body_does_not_fabricate_callback_identity() {
        let (
            mut bodies,
            mut colliders,
            parameters,
            mut islands,
            mut broad_phase,
            mut narrow_phase,
            mut impulse_joints,
            mut multibody_joints,
            mut ccd,
        ) = empty_rapier_state();

        let handle = add_sphere_to_rapier(&mut bodies, &mut colliders, Vec3::ZERO, 0.5, 1.0);
        bodies[handle].add_force(vector![3.0, 0.0, 0.0], true);
        let callback = CountingCallback {
            force_calls: Cell::new(0),
            last_body: Cell::new(None),
        };
        let mut bridge = RapierPhysicsBridge::new(callback);

        bridge.step(
            1.0 / 60.0,
            &mut bodies,
            &mut colliders,
            &parameters,
            &mut islands,
            &mut broad_phase,
            &mut narrow_phase,
            &mut impulse_joints,
            &mut multibody_joints,
            &mut ccd,
            &(),
            &(),
        );

        assert_eq!(bridge.callback.force_calls.get(), 0);
        assert_eq!(bridge.callback.last_body.get(), None);
        assert!(bodies[handle].linvel().x > 0.0);
    }

    #[test]
    fn force_modulation_uses_explicit_body_mapping_once_before_the_step() {
        let (
            mut bodies,
            mut colliders,
            parameters,
            mut islands,
            mut broad_phase,
            mut narrow_phase,
            mut impulse_joints,
            mut multibody_joints,
            mut ccd,
        ) = empty_rapier_state();

        let handle = add_sphere_to_rapier(&mut bodies, &mut colliders, Vec3::ZERO, 0.5, 1.0);
        bodies[handle].add_force(vector![3.0, 0.0, 0.0], true);

        let callback = CountingCallback {
            force_calls: Cell::new(0),
            last_body: Cell::new(None),
        };
        let mut bridge = RapierPhysicsBridge::new(callback);
        let expected_body = BodyHandle(42);
        assert_eq!(bridge.bind_callback_body(handle, expected_body), None);
        assert_eq!(bridge.callback_body(handle), Some(expected_body));

        bridge.step(
            1.0 / 60.0,
            &mut bodies,
            &mut colliders,
            &parameters,
            &mut islands,
            &mut broad_phase,
            &mut narrow_phase,
            &mut impulse_joints,
            &mut multibody_joints,
            &mut ccd,
            &(),
            &(),
        );

        assert_eq!(bridge.callback.force_calls.get(), 1);
        assert_eq!(bridge.callback.last_body.get(), Some(expected_body));
        assert!(bodies[handle].linvel().x > 0.0);
        assert_eq!(bridge.unbind_callback_body(handle), Some(expected_body));
        assert_eq!(bridge.callback_body(handle), None);
    }

    #[test]
    fn rapier_handle_generation_prevents_callback_binding_alias() {
        let mut bodies = RigidBodySet::new();
        let mut colliders = ColliderSet::new();
        let mut islands = IslandManager::new();
        let mut impulse_joints = ImpulseJointSet::new();
        let mut multibody_joints = MultibodyJointSet::new();

        let first = add_sphere_to_rapier(&mut bodies, &mut colliders, Vec3::ZERO, 0.5, 1.0);
        let callback = CountingCallback {
            force_calls: Cell::new(0),
            last_body: Cell::new(None),
        };
        let mut bridge = RapierPhysicsBridge::new(callback);
        let first_symtropy = BodyHandle(7);
        bridge.bind_callback_body(first, first_symtropy);

        bodies
            .remove(
                first,
                &mut islands,
                &mut colliders,
                &mut impulse_joints,
                &mut multibody_joints,
                true,
            )
            .expect("first Rapier body should exist");

        let replacement = add_sphere_to_rapier(&mut bodies, &mut colliders, Vec3::ZERO, 0.5, 1.0);
        assert_eq!(first.into_raw_parts().0, replacement.into_raw_parts().0);
        assert_ne!(first.into_raw_parts().1, replacement.into_raw_parts().1);
        assert_eq!(bridge.callback_body(first), Some(first_symtropy));
        assert_eq!(bridge.callback_body(replacement), None);
    }

    #[test]
    fn post_step_observations_are_sorted_by_full_rapier_handle() {
        let mut bodies = RigidBodySet::new();
        let mut colliders = ColliderSet::new();
        let first = add_sphere_to_rapier(&mut bodies, &mut colliders, Vec3::ZERO, 0.5, 1.0);
        let second = add_sphere_to_rapier(
            &mut bodies,
            &mut colliders,
            Vec3::new(1.0, 0.0, 0.0),
            0.5,
            1.0,
        );
        let mut broad_phase = BroadPhase::new();
        let mut narrow_phase = NarrowPhase::new();
        let mut bridge = RapierPhysicsBridge::new(NoOpCallback);

        bridge.post_step(
            &mut bodies,
            &mut colliders,
            &mut broad_phase,
            &mut narrow_phase,
        );

        assert_eq!(bridge.last_observations().len(), 2);
        assert_eq!(
            bridge.last_observations()[0].raw_handle,
            first.into_raw_parts()
        );
        assert_eq!(
            bridge.last_observations()[1].raw_handle,
            second.into_raw_parts()
        );
    }
}
