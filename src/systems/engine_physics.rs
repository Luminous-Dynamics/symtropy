// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
use crate::resources::{GamePhase, PhysicsWorldRes};
use bevy::prelude::*;
use symtropy_physics::BodyHandle;
use symtropy_render_bridge::PhysicsBody;

const MOTOR_EPSILON: f64 = 1e-10;

/// Hard physical limits for one launcher-local planar motor controller.
///
/// These are limits, not gameplay tuning defaults. Callers must deliberately choose
/// values for the body/experience they own rather than inheriting an unrelated mode's
/// movement constants.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlanarMotorLimits {
    /// Maximum requested planar speed before integration-field motor authority is applied.
    pub max_speed: f64,
    /// Maximum requested acceleration magnitude before motor authority is applied.
    pub max_acceleration: f64,
}

impl PlanarMotorLimits {
    fn is_valid(self) -> bool {
        self.max_speed.is_finite()
            && self.max_speed >= 0.0
            && self.max_acceleration.is_finite()
            && self.max_acceleration >= 0.0
    }
}

/// Result of one bounded planar motor-control request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlanarMotorOutcome {
    /// A non-zero velocity correction was applied.
    Applied {
        motor_gain: f64,
        delta_speed: f64,
        positive_work_joules: f64,
        braking_dissipation_joules: f64,
    },
    /// The request and current state already agree within the controller tolerance.
    NoChange { motor_gain: f64 },
    /// The registered entity has no current self-propelled motor authority.
    NoAuthority,
    /// The command contained an invalid timestep, vector, or physical limit.
    InvalidRequest,
    /// The body handle does not resolve in the authoritative physics world.
    MissingBody,
    /// The resolved body is not a finite positive-mass dynamic body.
    NonDynamicBody,
    /// A self-propelled body has no registered consciousness/energy authority state.
    MissingMotorState,
    /// The registered motor state itself contains non-finite authority/energy values.
    InvalidMotorState,
}

fn finite_planar(vector: &nalgebra::SVector<f64, 2>) -> bool {
    vector.iter().all(|value| value.is_finite())
}

fn clamp_vector_magnitude(
    vector: nalgebra::SVector<f64, 2>,
    max_magnitude: f64,
) -> nalgebra::SVector<f64, 2> {
    let magnitude = vector.norm();
    if magnitude <= max_magnitude || magnitude <= MOTOR_EPSILON {
        vector
    } else {
        vector * (max_magnitude / magnitude)
    }
}

/// Split the mechanical work of a constant-direction velocity correction into
/// positive motor work and negative-work braking dissipation.
///
/// The correction direction is the force/impulse axis. Only the component of body
/// velocity parallel to that axis contributes mechanical power. If the correction
/// crosses through zero parallel velocity during the step, braking-to-zero and
/// acceleration-away-from-zero are accounted separately instead of cancelling out.
fn actuator_work_components(
    velocity_before: &nalgebra::SVector<f64, 2>,
    delta_velocity: &nalgebra::SVector<f64, 2>,
    mass: f64,
) -> (f64, f64) {
    let delta_speed = delta_velocity.norm();
    if delta_speed <= MOTOR_EPSILON || mass <= 0.0 {
        return (0.0, 0.0);
    }

    let axis = delta_velocity / delta_speed;
    let speed_before = velocity_before.dot(&axis);
    let speed_after = speed_before + delta_speed;

    if speed_before >= 0.0 {
        (
            0.5 * mass * (speed_after * speed_after - speed_before * speed_before),
            0.0,
        )
    } else if speed_after <= 0.0 {
        (
            0.0,
            0.5 * mass * (speed_before * speed_before - speed_after * speed_after),
        )
    } else {
        (
            0.5 * mass * speed_after * speed_after,
            0.5 * mass * speed_before * speed_before,
        )
    }
}

/// Return the largest [0, 1] scale that keeps positive actuator work within the
/// currently extractable work budget. Negative-work braking is intentionally free of
/// *positive* motor-energy demand here; its removed kinetic energy is recorded as heat.
fn energy_limited_delta_scale(
    velocity_before: &nalgebra::SVector<f64, 2>,
    delta_velocity: &nalgebra::SVector<f64, 2>,
    mass: f64,
    available_work: f64,
) -> f64 {
    let delta_speed = delta_velocity.norm();
    if delta_speed <= MOTOR_EPSILON {
        return 1.0;
    }

    let (full_positive_work, _) =
        actuator_work_components(velocity_before, delta_velocity, mass);
    if full_positive_work <= available_work + MOTOR_EPSILON {
        return 1.0;
    }

    let axis = delta_velocity / delta_speed;
    let speed_before = velocity_before.dot(&axis);
    let energy = available_work.max(0.0);

    let scale = if speed_before >= 0.0 {
        let reachable_parallel_speed =
            (speed_before * speed_before + 2.0 * energy / mass).sqrt();
        (reachable_parallel_speed - speed_before) / delta_speed
    } else {
        // Braking from a negative parallel velocity to zero is negative work and
        // therefore does not consume the positive motor-work budget. Any acceleration
        // beyond zero must be paid for from the remaining energy.
        let paid_speed_after_zero = (2.0 * energy / mass).sqrt();
        (-speed_before + paid_speed_after_zero) / delta_speed
    };

    scale.clamp(0.0, 1.0)
}

/// Read the current motor gain and the amount of work this actuator may extract.
///
/// The reservoir is a hard authority boundary independent of the cached safety tier:
/// stale Green/Yellow state cannot authorize self-propulsion after energy reaches zero.
/// Under the full consciousness runtime, the mechanical work budget is Helmholtz free
/// energy (`U - T*S`) via `EnergyBudget::available_work()`. The standalone launcher
/// stub has no thermal/entropy state, so its documented fallback is finite raw reservoir
/// energy.
fn motor_authority_and_work_budget(
    physics: &PhysicsWorldRes,
    handle: BodyHandle,
) -> Option<(f64, f64)> {
    let entity = physics.consciousness.entities.get(&handle)?;
    let reservoir_energy = entity.energy.available;

    if !reservoir_energy.is_finite() || reservoir_energy < 0.0 {
        return Some((f64::NAN, f64::NAN));
    }
    if entity.energy.is_collapsed() || reservoir_energy <= MOTOR_EPSILON {
        return Some((0.0, 0.0));
    }

    #[cfg(feature = "consciousness-runtime")]
    let work_budget = {
        if !entity.energy.temperature.is_finite()
            || entity.energy.temperature <= 0.0
            || !entity.energy.entropy.is_finite()
            || entity.energy.entropy < 0.0
            || !entity.energy.heat_capacity.is_finite()
            || entity.energy.heat_capacity <= 0.0
        {
            return Some((f64::NAN, f64::NAN));
        }
        entity.energy.available_work()
    };

    #[cfg(not(feature = "consciousness-runtime"))]
    let work_budget = reservoir_energy;

    #[cfg(feature = "consciousness-runtime")]
    let gain = entity.effective_motor_gain();

    #[cfg(not(feature = "consciousness-runtime"))]
    let gain = {
        let tier_gain = match entity.safety_tier {
            crate::resources::SafetyTier::Green => 1.0,
            crate::resources::SafetyTier::Yellow => 0.6,
            crate::resources::SafetyTier::Red => 0.0,
        };
        tier_gain * entity.motor_precision
    };

    Some((gain, work_budget))
}

fn charge_positive_motor_work(physics: &mut PhysicsWorldRes, handle: BodyHandle, work_joules: f64) {
    if work_joules <= MOTOR_EPSILON {
        return;
    }

    #[cfg(feature = "consciousness-runtime")]
    {
        use symtropy_physics::PhysicsCallback;
        PhysicsCallback::record_work(&mut physics.consciousness, handle, work_joules);
    }

    #[cfg(not(feature = "consciousness-runtime"))]
    if let Some(entity) = physics.consciousness.entities.get_mut(&handle) {
        let _ = entity.energy.consume(work_joules);
    }
}

fn record_braking_dissipation(
    physics: &mut PhysicsWorldRes,
    handle: BodyHandle,
    dissipated_joules: f64,
) {
    if dissipated_joules <= MOTOR_EPSILON {
        return;
    }

    #[cfg(feature = "consciousness-runtime")]
    if let Some(entity) = physics.consciousness.entities.get_mut(&handle) {
        entity.energy.dissipate_heat(dissipated_joules);
    }

    physics
        .consciousness
        .ledger
        .record_dissipation(dissipated_joules);
}

/// Apply one self-propelled planar target-velocity request to an authoritative body.
///
/// This is deliberately an actuator boundary rather than a direct transform writer:
///
/// 1. requested speed/acceleration are finite and hard-bounded;
/// 2. the body's registered motor authority scales both target speed and acceleration;
/// 3. positive mechanical work is limited by the entity's current extractable-work budget;
/// 4. braking energy is dissipated rather than magically regenerated;
/// 5. the resulting bounded velocity correction is applied to the physics body;
/// 6. collision impulses and other external velocity changes remain distinct from this
///    command and are never inferred as locomotion work.
///
/// A zero motor gain or exhausted reservoir returns [`PlanarMotorOutcome::NoAuthority`]
/// without damping or cancelling existing velocity. That matters: a collapsed/Red entity
/// cannot propel itself, but an external collision is still allowed to move it.
pub fn apply_planar_motor_target(
    physics: &mut PhysicsWorldRes,
    handle: BodyHandle,
    desired_velocity: nalgebra::SVector<f64, 2>,
    limits: PlanarMotorLimits,
    dt: f64,
) -> PlanarMotorOutcome {
    if !dt.is_finite() || dt <= 0.0 || !limits.is_valid() || !finite_planar(&desired_velocity) {
        return PlanarMotorOutcome::InvalidRequest;
    }

    let (velocity_before, mass) = {
        let Some(body) = physics.world.body(handle) else {
            return PlanarMotorOutcome::MissingBody;
        };
        if !body.is_dynamic() || !body.mass.is_finite() || body.mass <= 0.0 {
            return PlanarMotorOutcome::NonDynamicBody;
        }
        if !finite_planar(&body.linear_velocity) {
            return PlanarMotorOutcome::InvalidRequest;
        }
        (body.linear_velocity, body.mass)
    };

    let Some((motor_gain, work_budget)) = motor_authority_and_work_budget(physics, handle) else {
        return PlanarMotorOutcome::MissingMotorState;
    };
    if !motor_gain.is_finite() || !work_budget.is_finite() || work_budget < 0.0 {
        return PlanarMotorOutcome::InvalidMotorState;
    }

    let motor_gain = motor_gain.clamp(0.0, 1.0);
    if motor_gain <= MOTOR_EPSILON {
        return PlanarMotorOutcome::NoAuthority;
    }

    let requested_velocity = clamp_vector_magnitude(desired_velocity, limits.max_speed);
    let effective_target_velocity = requested_velocity * motor_gain;
    let requested_delta_velocity = effective_target_velocity - velocity_before;
    let max_delta_speed = limits.max_acceleration * motor_gain * dt;
    let bounded_delta_velocity =
        clamp_vector_magnitude(requested_delta_velocity, max_delta_speed);

    if bounded_delta_velocity.norm() <= MOTOR_EPSILON {
        return PlanarMotorOutcome::NoChange { motor_gain };
    }

    let energy_scale = energy_limited_delta_scale(
        &velocity_before,
        &bounded_delta_velocity,
        mass,
        work_budget,
    );
    let applied_delta_velocity = bounded_delta_velocity * energy_scale;

    if applied_delta_velocity.norm() <= MOTOR_EPSILON {
        return PlanarMotorOutcome::NoChange { motor_gain };
    }

    let (positive_work_joules, braking_dissipation_joules) =
        actuator_work_components(&velocity_before, &applied_delta_velocity, mass);

    if let Some(body) = physics.world.body_mut(handle) {
        body.linear_velocity += applied_delta_velocity;
        body.wake();
    } else {
        return PlanarMotorOutcome::MissingBody;
    }

    charge_positive_motor_work(physics, handle, positive_work_joules);
    record_braking_dissipation(physics, handle, braking_dissipation_joules);

    PlanarMotorOutcome::Applied {
        motor_gain,
        delta_speed: applied_delta_velocity.norm(),
        positive_work_joules,
        braking_dissipation_joules,
    }
}

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

/// Only the 2D `Playing` slice currently grants this adapter authority to integrate
/// `PhysicsWorld`. `Playing3D` owns movement kinematically in `rendering_3d` and still
/// shares physics-body handles for representation, so stepping it here would silently
/// introduce a second movement/collision authority.
fn phase_uses_authoritative_2d_step(phase: GamePhase) -> bool {
    phase == GamePhase::Playing
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

/// Advance authoritative 2D physics when that phase owns integration, then export body
/// positions to Bevy transforms.
///
/// This function keeps the historical `physics_sync_transforms` name because it is already
/// registered in the launcher's `FixedUpdate` chain for both 2D and 3D modes. In 2D, a
/// transform is published only after the corresponding physics step. In 3D, the existing
/// kinematic controller remains authoritative and this function only mirrors its shared
/// physics-body representation back to the visual transform.
pub fn physics_sync_transforms(
    mut physics: ResMut<PhysicsWorldRes>,
    time: Res<Time>,
    phase: Res<State<GamePhase>>,
    mut query: Query<(&PhysicsBody, &mut Transform)>,
) {
    if phase_uses_authoritative_2d_step(*phase.get())
        && !step_physics_world(&mut physics, f64::from(time.delta_secs()))
    {
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

    fn registered_body(energy: f64) -> (PhysicsWorldRes, BodyHandle) {
        let mut physics = PhysicsWorldRes::default();
        let handle = physics
            .world
            .add_sphere(symtropy_math::Point::origin(), 1.0, 1.0);
        physics.consciousness.register(handle, energy, 10.0);
        (physics, handle)
    }

    fn generous_limits() -> PlanarMotorLimits {
        PlanarMotorLimits {
            max_speed: 100.0,
            max_acceleration: 100.0,
        }
    }

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

    #[test]
    fn only_2d_playing_phase_owns_world_integration_here() {
        assert!(phase_uses_authoritative_2d_step(GamePhase::Playing));
        assert!(!phase_uses_authoritative_2d_step(GamePhase::Playing3D));
        assert!(!phase_uses_authoritative_2d_step(GamePhase::Loading));
        assert!(!phase_uses_authoritative_2d_step(GamePhase::MainMenu));
    }

    #[test]
    fn planar_motor_rejects_non_finite_commands_without_mutation() {
        let (mut physics, handle) = registered_body(100.0);
        let before = physics.world.body(handle).unwrap().linear_velocity;
        let outcome = apply_planar_motor_target(
            &mut physics,
            handle,
            nalgebra::SVector::from([f64::NAN, 0.0]),
            generous_limits(),
            1.0 / 64.0,
        );
        assert_eq!(outcome, PlanarMotorOutcome::InvalidRequest);
        assert_eq!(physics.world.body(handle).unwrap().linear_velocity, before);
    }

    #[test]
    fn planar_motor_bounds_acceleration_and_charges_positive_work() {
        let (mut physics, handle) = registered_body(100.0);
        let limits = PlanarMotorLimits {
            max_speed: 10.0,
            max_acceleration: 4.0,
        };
        let outcome = apply_planar_motor_target(
            &mut physics,
            handle,
            nalgebra::SVector::from([100.0, 0.0]),
            limits,
            0.5,
        );

        let PlanarMotorOutcome::Applied {
            delta_speed,
            positive_work_joules,
            ..
        } = outcome
        else {
            panic!("expected applied motor request, got {outcome:?}");
        };
        assert!((delta_speed - 2.0).abs() < 1e-9);
        assert!((positive_work_joules - 2.0).abs() < 1e-9);
        assert!((physics.world.body(handle).unwrap().linear_velocity[0] - 2.0).abs() < 1e-9);
        let remaining = physics.consciousness.entities.get(&handle).unwrap().energy.available;
        assert!((remaining - 98.0).abs() < 1e-9);
    }

    #[test]
    fn planar_motor_is_energy_limited_before_motion_is_applied() {
        let (mut physics, handle) = registered_body(0.5);
        let outcome = apply_planar_motor_target(
            &mut physics,
            handle,
            nalgebra::SVector::from([10.0, 0.0]),
            generous_limits(),
            1.0,
        );
        assert!(matches!(outcome, PlanarMotorOutcome::Applied { .. }));
        let velocity = physics.world.body(handle).unwrap().linear_velocity[0];
        assert!((velocity - 1.0).abs() < 1e-9, "velocity={velocity}");
        let entity = physics.consciousness.entities.get(&handle).unwrap();
        assert!(entity.energy.available <= 1e-9);
        assert!(entity.energy.is_collapsed());
    }

    #[test]
    fn braking_dissipates_energy_without_regenerative_credit() {
        let (mut physics, handle) = registered_body(100.0);
        physics.world.body_mut(handle).unwrap().linear_velocity =
            nalgebra::SVector::from([2.0, 0.0]);
        let energy_before = physics.consciousness.entities.get(&handle).unwrap().energy.available;

        let outcome = apply_planar_motor_target(
            &mut physics,
            handle,
            nalgebra::SVector::zeros(),
            generous_limits(),
            1.0,
        );
        let PlanarMotorOutcome::Applied {
            positive_work_joules,
            braking_dissipation_joules,
            ..
        } = outcome
        else {
            panic!("expected braking request, got {outcome:?}");
        };
        assert!(positive_work_joules <= 1e-9);
        assert!((braking_dissipation_joules - 2.0).abs() < 1e-9);
        assert!(physics.world.body(handle).unwrap().linear_velocity.norm() <= 1e-9);
        let energy_after = physics.consciousness.entities.get(&handle).unwrap().energy.available;
        assert!((energy_after - energy_before).abs() < 1e-9);
    }

    #[test]
    fn reversal_does_not_cancel_paid_work_against_prior_braking() {
        let (mut physics, handle) = registered_body(100.0);
        physics.world.body_mut(handle).unwrap().linear_velocity =
            nalgebra::SVector::from([-1.0, 0.0]);
        let before = physics.consciousness.entities.get(&handle).unwrap().energy.available;

        let outcome = apply_planar_motor_target(
            &mut physics,
            handle,
            nalgebra::SVector::from([1.0, 0.0]),
            generous_limits(),
            1.0,
        );
        let PlanarMotorOutcome::Applied {
            positive_work_joules,
            braking_dissipation_joules,
            ..
        } = outcome
        else {
            panic!("expected reversal request, got {outcome:?}");
        };
        assert!((positive_work_joules - 0.5).abs() < 1e-9);
        assert!((braking_dissipation_joules - 0.5).abs() < 1e-9);
        let after = physics.consciousness.entities.get(&handle).unwrap().energy.available;
        assert!((before - after - 0.5).abs() < 1e-9);
    }

    #[test]
    fn red_motor_authority_does_not_cancel_external_velocity() {
        let (mut physics, handle) = registered_body(100.0);
        physics.world.body_mut(handle).unwrap().linear_velocity =
            nalgebra::SVector::from([3.0, 0.0]);
        physics
            .consciousness
            .entities
            .get_mut(&handle)
            .unwrap()
            .safety_tier = crate::resources::SafetyTier::Red;

        let outcome = apply_planar_motor_target(
            &mut physics,
            handle,
            nalgebra::SVector::zeros(),
            generous_limits(),
            1.0,
        );
        assert_eq!(outcome, PlanarMotorOutcome::NoAuthority);
        assert!((physics.world.body(handle).unwrap().linear_velocity[0] - 3.0).abs() < 1e-9);
    }

    #[test]
    fn zero_reservoir_blocks_stale_cached_motor_authority_without_damping() {
        let (mut physics, handle) = registered_body(100.0);
        physics.world.body_mut(handle).unwrap().linear_velocity =
            nalgebra::SVector::from([3.0, 0.0]);
        let entity = physics.consciousness.entities.get_mut(&handle).unwrap();
        entity.energy.available = 0.0;
        entity.safety_tier = crate::resources::SafetyTier::Green;

        let outcome = apply_planar_motor_target(
            &mut physics,
            handle,
            nalgebra::SVector::from([10.0, 0.0]),
            generous_limits(),
            1.0,
        );

        assert_eq!(outcome, PlanarMotorOutcome::NoAuthority);
        assert!((physics.world.body(handle).unwrap().linear_velocity[0] - 3.0).abs() < 1e-9);
    }

    #[cfg(feature = "consciousness-runtime")]
    #[test]
    fn collapsed_reservoir_blocks_green_cached_tier_even_with_positive_internal_energy() {
        let (mut physics, handle) = registered_body(100.0);
        physics.world.body_mut(handle).unwrap().linear_velocity =
            nalgebra::SVector::from([2.0, 0.0]);
        let entity = physics.consciousness.entities.get_mut(&handle).unwrap();
        entity.energy.collapsed = true;
        entity.safety_tier = crate::resources::SafetyTier::Green;

        let outcome = apply_planar_motor_target(
            &mut physics,
            handle,
            nalgebra::SVector::from([10.0, 0.0]),
            generous_limits(),
            1.0,
        );

        assert_eq!(outcome, PlanarMotorOutcome::NoAuthority);
        assert!((physics.world.body(handle).unwrap().linear_velocity[0] - 2.0).abs() < 1e-9);
    }

    #[cfg(feature = "consciousness-runtime")]
    #[test]
    fn helmholtz_available_work_caps_positive_motor_work() {
        let (mut physics, handle) = registered_body(100.0);
        {
            let entity = physics.consciousness.entities.get_mut(&handle).unwrap();
            entity.energy.temperature = 310.0;
            entity.energy.entropy = (100.0 - 0.5) / 310.0;
            entity.energy.heat_capacity = 100.0;
            assert!((entity.energy.available_work() - 0.5).abs() < 1e-9);
        }

        let outcome = apply_planar_motor_target(
            &mut physics,
            handle,
            nalgebra::SVector::from([10.0, 0.0]),
            generous_limits(),
            1.0,
        );
        let PlanarMotorOutcome::Applied {
            positive_work_joules,
            ..
        } = outcome
        else {
            panic!("expected extractable-work-limited request, got {outcome:?}");
        };

        assert!((positive_work_joules - 0.5).abs() < 1e-9);
        assert!((physics.world.body(handle).unwrap().linear_velocity[0] - 1.0).abs() < 1e-9);
    }

    #[cfg(feature = "consciousness-runtime")]
    #[test]
    fn non_finite_thermal_state_fails_closed_before_motor_mutation() {
        let (mut physics, handle) = registered_body(100.0);
        physics.world.body_mut(handle).unwrap().linear_velocity =
            nalgebra::SVector::from([2.0, 0.0]);
        physics
            .consciousness
            .entities
            .get_mut(&handle)
            .unwrap()
            .energy
            .temperature = f64::NAN;

        let outcome = apply_planar_motor_target(
            &mut physics,
            handle,
            nalgebra::SVector::from([10.0, 0.0]),
            generous_limits(),
            1.0,
        );

        assert_eq!(outcome, PlanarMotorOutcome::InvalidMotorState);
        assert!((physics.world.body(handle).unwrap().linear_velocity[0] - 2.0).abs() < 1e-9);
    }

    #[test]
    fn motor_precision_reduces_target_speed_and_acceleration_authority() {
        let (mut physics, handle) = registered_body(100.0);
        physics
            .consciousness
            .entities
            .get_mut(&handle)
            .unwrap()
            .motor_precision = 0.5;

        let outcome = apply_planar_motor_target(
            &mut physics,
            handle,
            nalgebra::SVector::from([10.0, 0.0]),
            generous_limits(),
            1.0,
        );
        let PlanarMotorOutcome::Applied { motor_gain, .. } = outcome else {
            panic!("expected applied motor request, got {outcome:?}");
        };
        assert!((motor_gain - 0.5).abs() < 1e-9);
        assert!((physics.world.body(handle).unwrap().linear_velocity[0] - 5.0).abs() < 1e-9);
    }
}
