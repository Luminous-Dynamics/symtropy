// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Duration;

use bevy::prelude::{Fixed, Time};

use super::thermodynamic::ThermodynamicHudState;
use super::thermodynamic_cadence::{
    CadenceBoundBeginError, CadenceBoundCommitError, THERMODYNAMIC_V01_FIXED_TIMESTEP,
    ThermodynamicCadenceError, admit_thermodynamic_cadence,
    begin_thermodynamic_tick_at_admitted_cadence,
    finalize_thermodynamic_tick_at_admitted_cadence,
};
use super::thermodynamic_runtime::ThermodynamicTransactionRuntime;
use super::thermodynamic_transaction::ThermodynamicConsequenceStatus;
use crate::resources::PhysicsWorldRes;
use symtropy_math::Point;

fn fixed(timestep: Duration) -> Time<Fixed> {
    Time::<Fixed>::from_duration(timestep)
}

#[test]
fn exact_bevy_default_cadence_is_admitted() {
    let time = fixed(Duration::from_micros(15_625));
    let receipt = admit_thermodynamic_cadence(&time).unwrap();
    assert_eq!(receipt.timestep, THERMODYNAMIC_V01_FIXED_TIMESTEP);
}

#[test]
fn sixty_hz_is_rejected_instead_of_mislabeling_hud_rates() {
    let time = Time::<Fixed>::from_hz(60.0);
    assert_eq!(
        admit_thermodynamic_cadence(&time),
        Err(ThermodynamicCadenceError::UnsupportedFixedTimestep {
            expected: THERMODYNAMIC_V01_FIXED_TIMESTEP,
            actual: time.timestep(),
        })
    );
}

#[test]
fn unsupported_cadence_rejects_before_begin_mutates_runtime() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    let bad = Time::<Fixed>::from_hz(60.0);

    assert!(matches!(
        begin_thermodynamic_tick_at_admitted_cadence(&mut runtime, &bad),
        Err(CadenceBoundBeginError::Cadence(
            ThermodynamicCadenceError::UnsupportedFixedTimestep { .. }
        ))
    ));
    assert_eq!(runtime.open_tick_id(), None);
    assert_eq!(runtime.next_tick_id(), Some(0));
    assert!(runtime.friction_journal().is_empty());
}

#[test]
fn cadence_change_before_close_fails_before_operational_close() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    let good = fixed(THERMODYNAMIC_V01_FIXED_TIMESTEP);
    begin_thermodynamic_tick_at_admitted_cadence(&mut runtime, &good).unwrap();

    let mut physics = PhysicsWorldRes::default();
    let handle = physics.world.add_sphere(Point::origin(), 1.0, 1.0);
    physics.consciousness.register(handle, 100.0, 10.0);
    let mut hud = ThermodynamicHudState::default();
    let bad = Time::<Fixed>::from_hz(60.0);

    assert!(matches!(
        finalize_thermodynamic_tick_at_admitted_cadence(
            &mut runtime,
            &mut physics,
            &mut hud,
            &[handle],
            ThermodynamicConsequenceStatus::Executed,
            &bad,
        ),
        Err(CadenceBoundCommitError::Cadence(
            ThermodynamicCadenceError::UnsupportedFixedTimestep { .. }
        ))
    ));
    assert_eq!(runtime.open_tick_id(), Some(0));
    assert_eq!(runtime.next_tick_id(), Some(0));
    assert_eq!(hud.ticks_accumulated, 0);

    let receipt = finalize_thermodynamic_tick_at_admitted_cadence(
        &mut runtime,
        &mut physics,
        &mut hud,
        &[handle],
        ThermodynamicConsequenceStatus::Executed,
        &good,
    )
    .unwrap();
    assert_eq!(receipt.cadence.timestep, THERMODYNAMIC_V01_FIXED_TIMESTEP);
    assert_eq!(receipt.committed.transaction.tick_id, 0);
    assert_eq!(runtime.next_tick_id(), Some(1));
}

#[test]
fn committed_receipt_binds_the_admitted_cadence() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    let time = fixed(THERMODYNAMIC_V01_FIXED_TIMESTEP);
    begin_thermodynamic_tick_at_admitted_cadence(&mut runtime, &time).unwrap();

    let mut physics = PhysicsWorldRes::default();
    let handle = physics.world.add_sphere(Point::origin(), 1.0, 1.0);
    physics.consciousness.register(handle, 100.0, 10.0);
    let mut hud = ThermodynamicHudState::default();

    let receipt = finalize_thermodynamic_tick_at_admitted_cadence(
        &mut runtime,
        &mut physics,
        &mut hud,
        &[handle],
        ThermodynamicConsequenceStatus::IntentionallyAbsent,
        &time,
    )
    .unwrap();

    assert_eq!(receipt.cadence.timestep, Duration::from_micros(15_625));
    assert_eq!(
        receipt.committed.transaction.consequence_status,
        ThermodynamicConsequenceStatus::IntentionallyAbsent
    );
}