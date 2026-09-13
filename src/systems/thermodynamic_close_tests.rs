// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::thermodynamic::ThermodynamicHudState;
use super::thermodynamic_close::{
    OperationalThermodynamicCloseError, close_operational_thermodynamic_tick,
};
use crate::resources::{PhysicsWorldRes, SafetyTier};
use symtropy_math::Point;
use symtropy_physics::BodyHandle;

fn registered_agent(energy: f64) -> (PhysicsWorldRes, BodyHandle) {
    let mut physics = PhysicsWorldRes::default();
    let handle = physics.world.add_sphere(Point::origin(), 1.0, 1.0);
    physics.consciousness.register(handle, energy, 10.0);
    (physics, handle)
}

#[test]
fn consequence_time_counters_are_sampled_in_the_same_close() {
    let (mut physics, handle) = registered_agent(100.0);
    let entity = physics.consciousness.entities.get_mut(&handle).unwrap();
    entity.energy.tick_reset();
    let _ = entity.energy.consume(5.0);

    let mut hud = ThermodynamicHudState::default();
    let receipt = close_operational_thermodynamic_tick(&mut physics, &mut hud, &[handle]).unwrap();

    assert_eq!(receipt.canonical_handles, vec![handle]);
    assert_eq!(receipt.sampled_consumed, 5.0);
    assert_eq!(receipt.sampled_regenerated, 0.0);
    assert_eq!(receipt.legacy_tick_count_before, 0);
    assert_eq!(receipt.legacy_tick_count_after, 1);
    assert_eq!(hud.ticks_accumulated, 1);
    assert_eq!(hud.energy_consumed_accumulator, 5.0);
}

#[test]
fn newly_exhausted_entity_is_red_at_close() {
    let (mut physics, handle) = registered_agent(1.0);
    {
        let entity = physics.consciousness.entities.get_mut(&handle).unwrap();
        entity.energy.tick_reset();
        let _ = entity.energy.consume(1.0);
        entity.safety_tier = SafetyTier::Green;
    }

    let mut hud = ThermodynamicHudState::default();
    let receipt = close_operational_thermodynamic_tick(&mut physics, &mut hud, &[handle]).unwrap();
    assert_eq!(receipt.collapsed_handles, vec![handle]);
    assert_eq!(
        physics.consciousness.entities.get(&handle).unwrap().safety_tier,
        SafetyTier::Red
    );
}

#[test]
fn duplicate_handle_fails_before_hud_or_safety_mutation() {
    let (mut physics, handle) = registered_agent(1.0);
    {
        let entity = physics.consciousness.entities.get_mut(&handle).unwrap();
        entity.energy.tick_reset();
        let _ = entity.energy.consume(1.0);
        entity.safety_tier = SafetyTier::Green;
    }
    let mut hud = ThermodynamicHudState::default();

    assert_eq!(
        close_operational_thermodynamic_tick(&mut physics, &mut hud, &[handle, handle]),
        Err(OperationalThermodynamicCloseError::DuplicateHandle(handle))
    );
    assert_eq!(hud.ticks_accumulated, 0);
    assert_eq!(physics.consciousness.ledger.tick_count, 0);
    assert_eq!(
        physics.consciousness.entities.get(&handle).unwrap().safety_tier,
        SafetyTier::Green
    );
}

#[test]
fn missing_operational_entity_fails_closed() {
    let mut physics = PhysicsWorldRes::default();
    let handle = physics.world.add_sphere(Point::origin(), 1.0, 1.0);
    let mut hud = ThermodynamicHudState::default();

    assert_eq!(
        close_operational_thermodynamic_tick(&mut physics, &mut hud, &[handle]),
        Err(OperationalThermodynamicCloseError::MissingOperationalEntity(handle))
    );
    assert_eq!(hud.ticks_accumulated, 0);
    assert_eq!(physics.consciousness.ledger.tick_count, 0);
}

#[test]
fn invalid_counter_fails_before_close_mutation() {
    let (mut physics, handle) = registered_agent(100.0);
    {
        let entity = physics.consciousness.entities.get_mut(&handle).unwrap();
        entity.energy.tick_reset();
        entity.energy.consumed_this_tick = f64::NAN;
        entity.safety_tier = SafetyTier::Green;
    }
    let mut hud = ThermodynamicHudState::default();

    assert_eq!(
        close_operational_thermodynamic_tick(&mut physics, &mut hud, &[handle]),
        Err(OperationalThermodynamicCloseError::InvalidEntityCounters(handle))
    );
    assert_eq!(hud.ticks_accumulated, 0);
    assert_eq!(physics.consciousness.ledger.tick_count, 0);
    assert_eq!(
        physics.consciousness.entities.get(&handle).unwrap().safety_tier,
        SafetyTier::Green
    );
}

#[test]
fn malformed_legacy_ledger_fails_before_close_mutation() {
    let (mut physics, handle) = registered_agent(100.0);
    physics.consciousness.ledger.energy_in = f64::NAN;
    let mut hud = ThermodynamicHudState::default();

    assert_eq!(
        close_operational_thermodynamic_tick(&mut physics, &mut hud, &[handle]),
        Err(OperationalThermodynamicCloseError::InvalidLegacyLedgerState)
    );
    assert_eq!(hud.ticks_accumulated, 0);
    assert_eq!(physics.consciousness.ledger.tick_count, 0);
    assert!(physics.consciousness.ledger.energy_in.is_nan());
}

#[test]
fn exhausted_legacy_tick_counter_is_rejected_before_close() {
    let (mut physics, handle) = registered_agent(100.0);
    physics.consciousness.ledger.tick_count = u64::MAX;
    let mut hud = ThermodynamicHudState::default();

    assert_eq!(
        close_operational_thermodynamic_tick(&mut physics, &mut hud, &[handle]),
        Err(OperationalThermodynamicCloseError::LegacyTickCounterExhausted)
    );
    assert_eq!(hud.ticks_accumulated, 0);
    assert_eq!(physics.consciousness.ledger.tick_count, u64::MAX);
}

#[test]
fn canonical_handle_order_makes_receipt_order_independent() {
    let mut physics = PhysicsWorldRes::default();
    let first = physics.world.add_sphere(Point::origin(), 1.0, 1.0);
    let second = physics.world.add_sphere(Point::origin(), 1.0, 1.0);
    physics.consciousness.register(first, 100.0, 10.0);
    physics.consciousness.register(second, 100.0, 10.0);
    let mut hud = ThermodynamicHudState::default();

    let receipt = close_operational_thermodynamic_tick(
        &mut physics,
        &mut hud,
        &[second, first],
    )
    .unwrap();
    assert_eq!(receipt.canonical_handles, vec![first, second]);
}

#[test]
fn rate_window_receipt_matches_existing_sixty_four_hz_policy() {
    let (mut physics, handle) = registered_agent(100.0);
    let entity = physics.consciousness.entities.get_mut(&handle).unwrap();
    entity.energy.tick_reset();
    let _ = entity.energy.consume(1.0);

    let mut hud = ThermodynamicHudState {
        energy_consumed_accumulator: 15.0,
        energy_regenerated_accumulator: 0.0,
        ticks_accumulated: 15,
        consumed_per_sec: 0.0,
        regenerated_per_sec: 0.0,
    };
    let receipt = close_operational_thermodynamic_tick(&mut physics, &mut hud, &[handle]).unwrap();

    assert!(receipt.rate_window_completed);
    assert_eq!(receipt.hud_ticks_before, 15);
    assert_eq!(receipt.hud_ticks_after, 0);
    assert!((receipt.consumed_per_sec_after - 64.0).abs() < 1.0e-12);
    assert!((hud.consumed_per_sec - 64.0).abs() < 1.0e-12);
    assert_eq!(hud.energy_consumed_accumulator, 0.0);
}