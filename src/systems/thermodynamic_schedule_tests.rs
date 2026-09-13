// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use nalgebra::SVector;
use symtropy_math::Point;
use symtropy_physics::BodyHandle;
use symtropy_render_bridge::PhysicsBody;

use super::thermodynamic::ThermodynamicHudState;
use super::thermodynamic_cadence::THERMODYNAMIC_V01_FIXED_TIMESTEP;
use super::thermodynamic_runtime::ThermodynamicTransactionRuntime;
use super::thermodynamic_schedule::{
    ThermodynamicScheduleFaultKind, ThermodynamicScheduleState,
    transactional_physics_finalize_system, transactional_thermodynamic_begin_system,
};
use super::thermodynamic_transaction::ThermodynamicConsequenceStatus;
use crate::components::Player;
use crate::resources::{GamePhase, PhysicsWorldRes};

fn world_for_phase(phase: GamePhase) -> World {
    let mut world = World::new();
    let mut fixed = Time::<Fixed>::from_duration(THERMODYNAMIC_V01_FIXED_TIMESTEP);
    fixed.advance_by(THERMODYNAMIC_V01_FIXED_TIMESTEP);
    world.insert_resource(fixed);
    world.insert_resource(State::new(phase));
    world.insert_resource(PhysicsWorldRes::default());
    world.insert_resource(ThermodynamicHudState::default());
    world.insert_resource(ThermodynamicTransactionRuntime::new());
    world.insert_resource(ThermodynamicScheduleState::default());
    world
}

fn add_registered_player(world: &mut World, velocity_x: f64) -> (BodyHandle, Entity) {
    let handle = {
        let mut physics = world.resource_mut::<PhysicsWorldRes>();
        let handle = physics.world.add_sphere(Point::origin(), 0.5, 1.0);
        physics.consciousness.register(handle, 100.0, 10.0);
        physics.world.body_mut(handle).unwrap().linear_velocity =
            SVector::from([velocity_x, 0.0]);
        handle
    };
    let entity = world
        .spawn((Player, PhysicsBody::new(handle, 0.5), Transform::default()))
        .id();
    (handle, entity)
}

fn body_x(world: &World, handle: BodyHandle) -> f64 {
    world
        .resource::<PhysicsWorldRes>()
        .world
        .body(handle)
        .unwrap()
        .position()[0]
}

fn entity_x(world: &World, entity: Entity) -> f32 {
    world.entity(entity).get::<Transform>().unwrap().translation.x
}

#[test]
fn duplicate_body_handle_rejects_before_begin_or_counter_reset() {
    let mut world = world_for_phase(GamePhase::Playing);
    let (handle, _first) = add_registered_player(&mut world, 0.0);
    world.spawn((Player, PhysicsBody::new(handle, 0.5), Transform::default()));

    {
        let mut physics = world.resource_mut::<PhysicsWorldRes>();
        let entity = physics.consciousness.entities.get_mut(&handle).unwrap();
        let _ = entity.energy.consume(1.0);
        assert!(entity.energy.consumed_this_tick > 0.0);
    }

    world
        .run_system_once(transactional_thermodynamic_begin_system)
        .unwrap();

    assert_eq!(
        world
            .resource::<ThermodynamicTransactionRuntime>()
            .open_tick_id(),
        None
    );
    let schedule = world.resource::<ThermodynamicScheduleState>();
    assert_eq!(
        schedule.last_fault().unwrap().kind,
        ThermodynamicScheduleFaultKind::BeginCensus
    );
    assert!(schedule.active_handles().is_none());
    assert!(
        world
            .resource::<PhysicsWorldRes>()
            .consciousness
            .entities
            .get(&handle)
            .unwrap()
            .energy
            .consumed_this_tick
            > 0.0,
        "rejected begin must not reset an existing operational interval",
    );
}

#[test]
fn live_census_drift_blocks_physics_until_control_volume_matches_again() {
    let mut world = world_for_phase(GamePhase::Playing);
    let (a, _entity_a) = add_registered_player(&mut world, 1.0);

    world
        .run_system_once(transactional_thermodynamic_begin_system)
        .unwrap();
    assert_eq!(
        world
            .resource::<ThermodynamicScheduleState>()
            .active_handles(),
        Some(&[a][..])
    );

    let (_b, entity_b) = add_registered_player(&mut world, 0.0);
    world
        .run_system_once(transactional_physics_finalize_system)
        .unwrap();

    assert_eq!(body_x(&world, a), 0.0, "census drift must block consequence");
    {
        let schedule = world.resource::<ThermodynamicScheduleState>();
        assert!(schedule.has_pending_transaction());
        assert_eq!(
            schedule.last_fault().unwrap().kind,
            ThermodynamicScheduleFaultKind::CensusDriftBeforeConsequence
        );
    }

    world.despawn(entity_b);
    world
        .run_system_once(transactional_physics_finalize_system)
        .unwrap();

    assert!(body_x(&world, a) > 0.0);
    let schedule = world.resource::<ThermodynamicScheduleState>();
    let receipt = schedule.last_receipt().unwrap();
    assert_eq!(
        receipt.committed.operational_close.canonical_handles,
        vec![a]
    );
    assert!(!schedule.has_pending_transaction());
    assert!(schedule.active_handles().is_none());
}

#[test]
fn prepared_close_retry_after_phase_exit_does_not_step_physics_twice() {
    let mut world = world_for_phase(GamePhase::Playing);
    let (handle, entity) = add_registered_player(&mut world, 1.0);

    world
        .run_system_once(transactional_thermodynamic_begin_system)
        .unwrap();
    world
        .resource_mut::<ThermodynamicHudState>()
        .energy_consumed_accumulator = f64::NAN;

    world
        .run_system_once(transactional_physics_finalize_system)
        .unwrap();
    let x_after_first_consequence = body_x(&world, handle);
    assert!(x_after_first_consequence > 0.0);
    assert_eq!(entity_x(&world, entity), 0.0, "export waits for commit");
    assert!(
        world
            .resource::<ThermodynamicScheduleState>()
            .has_pending_transaction()
    );

    // The finalizer itself is not gameplay-gated in the production registration;
    // replacing the State resource demonstrates that its prepared continuation is
    // independent of the later phase value.
    world.insert_resource(State::new(GamePhase::MainMenu));
    world
        .resource_mut::<ThermodynamicHudState>()
        .energy_consumed_accumulator = 0.0;
    world
        .run_system_once(transactional_physics_finalize_system)
        .unwrap();

    assert_eq!(
        body_x(&world, handle),
        x_after_first_consequence,
        "prepared retry must not execute physics a second time",
    );
    assert!((entity_x(&world, entity) as f64 - x_after_first_consequence).abs() < 1e-6);
    let schedule = world.resource::<ThermodynamicScheduleState>();
    assert!(!schedule.has_pending_transaction());
    assert_eq!(
        schedule
            .last_receipt()
            .unwrap()
            .committed
            .transaction
            .consequence_status,
        ThermodynamicConsequenceStatus::Executed
    );
}

#[test]
fn kinematic_3d_tick_commits_without_exporting_2d_world_position() {
    let mut world = world_for_phase(GamePhase::Playing3D);
    let (handle, entity) = add_registered_player(&mut world, 1.0);
    world.entity_mut(entity).get_mut::<Transform>().unwrap().translation.x = 7.0;

    world
        .run_system_once(transactional_thermodynamic_begin_system)
        .unwrap();
    world
        .run_system_once(transactional_physics_finalize_system)
        .unwrap();

    assert_eq!(body_x(&world, handle), 0.0, "3D slice must not step 2D physics");
    assert_eq!(entity_x(&world, entity), 7.0, "3D transform remains kinematic");
    assert_eq!(
        world
            .resource::<ThermodynamicScheduleState>()
            .last_receipt()
            .unwrap()
            .committed
            .transaction
            .consequence_status,
        ThermodynamicConsequenceStatus::IntentionallyAbsent
    );
}
