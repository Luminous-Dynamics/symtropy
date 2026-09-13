// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use nalgebra::SVector;
use symtropy_physics::{BodyHandle, FrictionSolverCoordinates, FrictionTransactionId};

use super::thermodynamic_runtime::{
    ThermodynamicRuntimeError, ThermodynamicTransactionRuntime,
};
use super::thermodynamic_transaction::ThermodynamicConsequenceStatus;

#[test]
fn pending_reservation_ids_are_canonical_and_read_only() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    runtime.begin_next_tick().unwrap();

    let later = runtime
        .reserve_friction_impulse_at(
            BodyHandle(10),
            BodyHandle(11),
            &SVector::<f64, 2>::from([0.0, 0.25]),
            &SVector::<f64, 2>::from([0.1, 0.0]),
            FrictionSolverCoordinates::new(2, 5, 7),
        )
        .unwrap();
    let earlier = runtime
        .reserve_friction_impulse_at(
            BodyHandle(20),
            BodyHandle(21),
            &SVector::<f64, 2>::from([0.0, -0.25]),
            &SVector::<f64, 2>::from([-0.1, 0.0]),
            FrictionSolverCoordinates::new(0, 1, 2),
        )
        .unwrap();

    let expected = vec![
        FrictionTransactionId::new(0, 0, 1, 2),
        FrictionTransactionId::new(0, 2, 5, 7),
    ];
    assert_eq!(runtime.pending_friction_reservation_ids(), expected);
    assert_eq!(runtime.pending_friction_reservation_count(), 2);

    // The diagnostic value is a copy. Mutating it grants no authority over the
    // runtime registry and cannot unblock normal fixed-tick close.
    let mut caller_copy = runtime.pending_friction_reservation_ids();
    caller_copy.clear();
    assert_eq!(runtime.pending_friction_reservation_ids(), expected);
    assert_eq!(
        runtime.prepare_finalize(ThermodynamicConsequenceStatus::Executed),
        Err(ThermodynamicRuntimeError::PendingFrictionReservations { count: 2 })
    );

    runtime.cancel_friction_reservation(earlier).unwrap();
    assert_eq!(
        runtime.pending_friction_reservation_ids(),
        vec![FrictionTransactionId::new(0, 2, 5, 7)]
    );
    runtime.cancel_friction_reservation(later).unwrap();
    assert!(runtime.pending_friction_reservation_ids().is_empty());
    assert_eq!(runtime.pending_friction_reservation_count(), 0);

    // Only the original non-cloneable tokens cleared the registry; normal close
    // becomes admissible afterward.
    runtime
        .prepare_finalize(ThermodynamicConsequenceStatus::Executed)
        .unwrap();
}

#[test]
fn repeated_diagnostics_never_age_out_or_clear_reservations() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    runtime.begin_next_tick().unwrap();
    let reservation = runtime
        .reserve_friction_impulse_at(
            BodyHandle(1),
            BodyHandle(2),
            &SVector::<f64, 2>::zeros(),
            &SVector::<f64, 2>::from([0.25, 0.0]),
            FrictionSolverCoordinates::new(9, 8, 7),
        )
        .unwrap();
    let id = reservation.transaction_id();

    for _ in 0..128 {
        assert_eq!(runtime.pending_friction_reservation_ids(), vec![id]);
        assert_eq!(runtime.pending_friction_reservation_count(), 1);
    }

    assert_eq!(
        runtime.prepare_finalize(ThermodynamicConsequenceStatus::Rejected),
        Err(ThermodynamicRuntimeError::PendingFrictionReservations { count: 1 })
    );
    runtime.cancel_friction_reservation(reservation).unwrap();
}
