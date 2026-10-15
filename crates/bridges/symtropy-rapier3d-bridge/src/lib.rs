// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Bridge between Rapier3D and Symtropy's state-coupling framework.

pub mod manipulator;

use std::fmt;

use ::nalgebra::SVector;
use bevy::prelude::*;
use rapier3d::prelude::*;
use symtropy_physics::body::BodyHandle;
use symtropy_physics::contact::CollisionEvent;
use symtropy_physics::world::PhysicsCallback;

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

/// Errors returned when a Rapier bridge step cannot be applied safely.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RapierStepError {
    /// The requested timestep is non-finite or not positive.
    InvalidDelta,
    /// The requested gravity vector contains a non-finite component.
    InvalidGravity,
    /// The callback produced a non-finite force for the identified body.
    NonFiniteModulatedForce { body: BodyHandle },
}

impl fmt::Display for RapierStepError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDelta => write!(f, "Rapier timestep must be finite and positive"),
            Self::InvalidGravity => write!(f, "Rapier gravity must be finite"),
            Self::NonFiniteModulatedForce { body } => {
                write!(f, "force callback produced a non-finite force for {body:?}")
            }
        }
    }
}

impl std::error::Error for RapierStepError {}

/// Observable, engine-local body state captured after a Rapier step.
///
/// Quaternion order is explicitly scalar-first: [w, x, y, z].
#[derive(Debug, Clone, PartialEq)]
pub struct RapierBodyState {
    pub handle: RigidBodyHandle,
    pub translation: [Real; 3],
    pub rotation_wxyz: [Real; 4],
    pub linear_velocity: [Real; 3],
    pub angular_velocity: [Real; 3],
    pub is_sleeping: bool,
}

/// The Rapier-based physics bridge.
///
/// The pipeline and gravity policy belong to the bridge and are reused across steps.
/// Rapier remains a reference backend; this bridge does not designate its output as
/// canonical gameplay truth.
pub struct RapierPhysicsBridge<C: PhysicsCallback<3>> {
    callback: C,
    pipeline: PhysicsPipeline,
    gravity: Vector<Real>,
}

impl<C: PhysicsCallback<3>> RapierPhysicsBridge<C> {
    pub fn new(callback: C) -> Self {
        Self {
            callback,
            pipeline: PhysicsPipeline::new(),
            gravity: vector![0.0, -9.81, 0.0],
        }
    }

    /// Set world gravity. A rejected value leaves the previous gravity unchanged.
    pub fn set_gravity(&mut self, gravity: Vector<Real>) -> Result<(), RapierStepError> {
        if !gravity.iter().all(|value| value.is_finite()) {
            return Err(RapierStepError::InvalidGravity);
        }
        self.gravity = gravity;
        Ok(())
    }

    /// Return the configured gravity vector.
    pub fn gravity(&self) -> &Vector<Real> {
        &self.gravity
    }

    /// Step the persistent Rapier pipeline and apply the Symtropy force callback once.
    ///
    /// `dt` is authoritative for this call: the supplied integration parameters are
    /// copied and their timestep replaced, leaving the caller's settings unchanged.
    /// The raw user-force accumulator is restored after stepping so a non-idempotent
    /// callback cannot recursively modulate its own previous output on later frames.
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
    ) -> Result<(), RapierStepError> {
        if !dt.is_finite() || dt <= 0.0 {
            return Err(RapierStepError::InvalidDelta);
        }
        if !self.gravity.iter().all(|value| value.is_finite()) {
            return Err(RapierStepError::InvalidGravity);
        }

        let mut parameters = *integration_parameters;
        parameters.dt = dt;

        // Compute every modulated force before mutating any body. A bad callback output
        // fails the whole step before a partial set of bodies is changed.
        let mut force_changes = Vec::new();
        for (handle, body) in rigid_body_set.iter() {
            let original_force = body.user_force();
            let body_handle = BodyHandle(handle.into_raw_parts().0 as usize);
            let force64 = SVector::<f64, 3>::new(
                original_force.x as f64,
                original_force.y as f64,
                original_force.z as f64,
            );
            let modulated64 = self.callback.modulate_force(body_handle, &force64);
            let modulated_force = vector![
                modulated64[0] as Real,
                modulated64[1] as Real,
                modulated64[2] as Real
            ];
            if !modulated_force.iter().all(|value| value.is_finite()) {
                return Err(RapierStepError::NonFiniteModulatedForce { body: body_handle });
            }
            // Avoid waking otherwise-idle bodies just to reset/re-add a zero force.
            if original_force.iter().all(|value| *value == 0.0)
                && modulated_force.iter().all(|value| *value == 0.0)
            {
                continue;
            }
            force_changes.push((handle, original_force, modulated_force));
        }

        for (handle, _, modulated_force) in &force_changes {
            if let Some(body) = rigid_body_set.get_mut(*handle) {
                // Clearing a source force is bookkeeping; only a nonzero transformed
                // force should wake a sleeping body.
                body.reset_forces(false);
                let should_wake = modulated_force.iter().any(|value| *value != 0.0);
                body.add_force(*modulated_force, should_wake);
            }
        }

        self.pipeline.step(
            &self.gravity,
            &parameters,
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

        // Rapier intentionally retains user forces between steps. Restore the raw
        // accumulator so the same force is modulated from the same source each frame.
        for (handle, original_force, _) in force_changes {
            if let Some(body) = rigid_body_set.get_mut(handle) {
                // Restoring the source accumulator is bookkeeping, not a new impulse;
                // never wake a body solely because its prior force is being preserved.
                body.reset_forces(false);
                body.add_force(original_force, false);
            }
        }

        Ok(())
    }

    /// Read back the latest body state after `step`; no cached/stale copy is used.
    pub fn post_step(&self, bodies: &RigidBodySet) -> Vec<RapierBodyState> {
        bodies
            .iter()
            .map(|(handle, body)| {
                let translation = body.translation();
                let rotation = body.rotation().quaternion();
                let linear_velocity = body.linvel();
                let angular_velocity = body.angvel();
                RapierBodyState {
                    handle,
                    translation: [translation.x, translation.y, translation.z],
                    rotation_wxyz: [rotation.w, rotation.i, rotation.j, rotation.k],
                    linear_velocity: [linear_velocity.x, linear_velocity.y, linear_velocity.z],
                    angular_velocity: [angular_velocity.x, angular_velocity.y, angular_velocity.z],
                    is_sleeping: body.is_sleeping(),
                }
            })
            .collect()
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
    use symtropy_physics::world::NoOpCallback;

    struct TestWorld {
        bodies: RigidBodySet,
        colliders: ColliderSet,
        parameters: IntegrationParameters,
        islands: IslandManager,
        broad_phase: BroadPhase,
        narrow_phase: NarrowPhase,
        impulse_joints: ImpulseJointSet,
        multibody_joints: MultibodyJointSet,
        ccd_solver: CCDSolver,
    }

    impl TestWorld {
        fn new() -> Self {
            Self {
                bodies: RigidBodySet::new(),
                colliders: ColliderSet::new(),
                parameters: IntegrationParameters::default(),
                islands: IslandManager::new(),
                broad_phase: BroadPhase::new(),
                narrow_phase: NarrowPhase::new(),
                impulse_joints: ImpulseJointSet::new(),
                multibody_joints: MultibodyJointSet::new(),
                ccd_solver: CCDSolver::new(),
            }
        }

        fn step<C: PhysicsCallback<3>>(
            &mut self,
            bridge: &mut RapierPhysicsBridge<C>,
            dt: f32,
        ) -> Result<(), RapierStepError> {
            bridge.step(
                dt,
                &mut self.bodies,
                &mut self.colliders,
                &self.parameters,
                &mut self.islands,
                &mut self.broad_phase,
                &mut self.narrow_phase,
                &mut self.impulse_joints,
                &mut self.multibody_joints,
                &mut self.ccd_solver,
                &(),
                &(),
            )
        }
    }

    struct ScaleForceCallback(f64);

    impl PhysicsCallback<3> for ScaleForceCallback {
        fn modulate_force(&self, _: BodyHandle, force: &SVector<f64, 3>) -> SVector<f64, 3> {
            *force * self.0
        }

        fn modulate_impulse(&self, impulse: f64, _: &SVector<f64, 3>) -> f64 {
            impulse
        }

        fn friction_multiplier(&self, _: &SVector<f64, 3>, _: BodyHandle) -> f64 {
            1.0
        }

        fn on_collision(&mut self, _: &CollisionEvent<3>) {}
        fn record_dissipation(&mut self, _: f64) {}
        fn record_work(&mut self, _: BodyHandle, _: f64) {}
        fn apply_trauma(&mut self, _: &CollisionEvent<3>) {}
    }

    fn y_after_one_step(dt: f32) -> (f32, f32) {
        let mut bridge = RapierPhysicsBridge::new(NoOpCallback);
        let mut world = TestWorld::new();
        let handle = world.bodies.insert(
            RigidBodyBuilder::dynamic()
                .translation(vector![0.0, 10.0, 0.0])
                .additional_mass(1.0)
                .build(),
        );
        let caller_dt = world.parameters.dt;

        world.step(&mut bridge, dt).expect("valid fixed step");
        let position_y = bridge
            .post_step(&world.bodies)
            .into_iter()
            .find(|state| state.handle == handle)
            .expect("body state readback")
            .translation[1];
        assert_eq!(world.parameters.dt, caller_dt);
        (position_y, world.parameters.dt)
    }

    #[test]
    fn bridge_step_advances_a_falling_body_and_reads_back_current_state() {
        let mut bridge = RapierPhysicsBridge::new(NoOpCallback);
        let mut world = TestWorld::new();
        let handle = world.bodies.insert(
            RigidBodyBuilder::dynamic()
                .translation(vector![0.0, 10.0, 0.0])
                .additional_mass(1.0)
                .build(),
        );

        world.step(&mut bridge, 1.0 / 60.0).expect("valid fixed step");
        let state = bridge.post_step(&world.bodies)
            .into_iter()
            .find(|state| state.handle == handle)
            .expect("body state readback");

        assert!(state.translation[1] < 10.0);
        assert!(state.linear_velocity[1] < 0.0);
        assert!((state.rotation_wxyz[0] - 1.0).abs() < 1.0e-6);
        assert!(!state.is_sleeping);
    }

    #[test]
    fn bridge_step_uses_requested_dt_without_mutating_caller_parameters() {
        let (short_dt_y, short_parameters_dt) = y_after_one_step(1.0 / 60.0);
        let (long_dt_y, long_parameters_dt) = y_after_one_step(1.0 / 30.0);
        let short_drop = 10.0 - short_dt_y;
        let long_drop = 10.0 - long_dt_y;

        assert_eq!(short_parameters_dt, 1.0 / 60.0);
        assert_eq!(long_parameters_dt, 1.0 / 60.0);
        assert!(short_drop > 0.0);
        assert!(long_drop > short_drop * 2.0);
    }

    #[test]
    fn invalid_delta_and_gravity_are_rejected_without_stepping() {
        let mut bridge = RapierPhysicsBridge::new(NoOpCallback);
        let mut world = TestWorld::new();
        let handle = world.bodies.insert(
            RigidBodyBuilder::dynamic()
                .translation(vector![0.0, 10.0, 0.0])
                .additional_mass(1.0)
                .build(),
        );
        assert_eq!(
            world.step(&mut bridge, f32::NAN),
            Err(RapierStepError::InvalidDelta)
        );
        assert_eq!(world.bodies.get(handle).unwrap().translation().y, 10.0);
        assert_eq!(
            bridge.set_gravity(vector![0.0, f32::INFINITY, 0.0]),
            Err(RapierStepError::InvalidGravity)
        );
        assert_eq!(*bridge.gravity(), vector![0.0, -9.81, 0.0]);
    }

    #[test]
    fn zero_force_step_does_not_wake_an_idle_body() {
        let mut bridge = RapierPhysicsBridge::new(NoOpCallback);
        let mut world = TestWorld::new();
        let handle = world.bodies.insert(
            RigidBodyBuilder::dynamic()
                .translation(vector![0.0, 10.0, 0.0])
                .additional_mass(1.0)
                .sleeping(true)
                .build(),
        );

        assert!(world.bodies.get(handle).expect("body inserted").is_sleeping());
        world.step(&mut bridge, 1.0 / 60.0).expect("valid fixed step");

        let state = bridge
            .post_step(&world.bodies)
            .into_iter()
            .find(|state| state.handle == handle)
            .expect("body state readback");
        assert!(state.is_sleeping, "zero-force bookkeeping must not wake idle bodies");
        assert_eq!(state.linear_velocity, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn zeroed_modulated_force_does_not_wake_a_sleeping_body() {
        let mut bridge = RapierPhysicsBridge::new(ScaleForceCallback(0.0));
        let mut world = TestWorld::new();
        let handle = world.bodies.insert(
            RigidBodyBuilder::dynamic()
                .translation(vector![0.0, 10.0, 0.0])
                .additional_mass(1.0)
                .sleeping(true)
                .build(),
        );
        let body = world.bodies.get_mut(handle).expect("body inserted");
        body.add_force(vector![1.0, 0.0, 0.0], false);
        body.sleep();

        world.step(&mut bridge, 1.0 / 60.0).expect("valid fixed step");
        let body = world.bodies.get(handle).expect("body retained");
        assert!(body.is_sleeping(), "a callback-suppressed force must not wake the body");
        assert_eq!(body.user_force(), vector![0.0, 0.0, 0.0]);
        assert_eq!(body.linvel(), &vector![0.0, 0.0, 0.0]);
    }

    #[test]
    fn non_finite_callback_output_fails_before_mutating_body_state() {
        let mut bridge = RapierPhysicsBridge::new(ScaleForceCallback(f64::NAN));
        let mut world = TestWorld::new();
        let handle = world.bodies.insert(
            RigidBodyBuilder::dynamic()
                .additional_mass(1.0)
                .build(),
        );
        world
            .bodies
            .get_mut(handle)
            .expect("body inserted")
            .add_force(vector![1.0, 0.0, 0.0], true);

        assert!(matches!(
            world.step(&mut bridge, 0.1),
            Err(RapierStepError::NonFiniteModulatedForce { .. })
        ));
        let body = world.bodies.get(handle).expect("body retained");
        assert!((body.user_force().x - 1.0).abs() < 1.0e-6);
        assert_eq!(body.linvel().x, 0.0);
    }

    #[test]
    fn force_callback_does_not_compound_its_previous_output() {
        let mut bridge = RapierPhysicsBridge::new(ScaleForceCallback(2.0));
        let mut world = TestWorld::new();
        let handle = world.bodies.insert(
            RigidBodyBuilder::dynamic()
                .additional_mass(1.0)
                .build(),
        );
        world
            .bodies
            .get_mut(handle)
            .expect("body inserted")
            .add_force(vector![1.0, 0.0, 0.0], true);
        world
            .bodies
            .get_mut(handle)
            .expect("body inserted")
            .add_torque(vector![0.0, 0.0, 0.75], true);

        world.step(&mut bridge, 0.1).expect("first step");
        world.step(&mut bridge, 0.1).expect("second step");

        let body = world.bodies.get(handle).expect("body retained");
        assert!((body.user_force().x - 1.0).abs() < 1.0e-6);
        assert!((body.user_torque().z - 0.75).abs() < 1.0e-6);
        assert!((body.linvel().x - 0.4).abs() < 0.04);
    }
}
