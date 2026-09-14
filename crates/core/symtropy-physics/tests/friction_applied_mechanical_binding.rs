// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

use nalgebra::SVector;
use symtropy_math::Point;
use symtropy_physics::{
    BodyHandle, BodyType, FrictionDiagnosticFinalizeError, FrictionTransactionId,
    FrictionTransactionJournal, FrictionTransactionPhase, RigidBody, apply_friction_impulse_once,
    finalize_friction_diagnostic,
};

fn body(handle: usize, velocity_x: f64) -> RigidBody<3> {
    let mut body = RigidBody::dynamic_sphere(
        BodyHandle(handle),
        Point::origin(),
        0.5,
        1.0,
    );
    body.linear_velocity[0] = velocity_x;
    body
}

fn applied_neutral_pair(
    id: FrictionTransactionId,
) -> (
    RigidBody<3>,
    RigidBody<3>,
    symtropy_physics::AppliedFrictionTransaction<3>,
    FrictionTransactionJournal,
) {
    let mut a = body(1, 1.0);
    let mut b = body(2, 0.0);
    let mut journal = FrictionTransactionJournal::new();
    let applied = apply_friction_impulse_once(
        &mut a,
        &mut b,
        &SVector::zeros(),
        &SVector::zeros(),
        id,
        &mut journal,
    )
    .unwrap();
    (a, b, applied, journal)
}

#[test]
fn participant_b_coherent_mass_drift_invalidates_bound_transaction() {
    let id = FrictionTransactionId::new(200, 0, 0, 0);
    let (a, mut b, applied, mut journal) = applied_neutral_pair(id);

    b.mass *= 2.0;
    b.inv_mass *= 0.5;

    assert_eq!(
        finalize_friction_diagnostic(&a, &b, &applied, &mut journal),
        Err(FrictionDiagnosticFinalizeError::ObservationStateMismatch)
    );
    assert_eq!(journal.phase(id), Some(FrictionTransactionPhase::Applied));
    assert_eq!(journal.pending_application_count(), 1);
}

#[test]
fn participant_b_position_drift_invalidates_bound_transaction() {
    let id = FrictionTransactionId::new(201, 0, 0, 0);
    let (a, mut b, applied, mut journal) = applied_neutral_pair(id);

    b.transform.translation.0[1] += 1.0e-9;

    assert_eq!(
        finalize_friction_diagnostic(&a, &b, &applied, &mut journal),
        Err(FrictionDiagnosticFinalizeError::ObservationStateMismatch)
    );
    assert_eq!(journal.phase(id), Some(FrictionTransactionPhase::Applied));
}

#[test]
fn participant_b_body_type_drift_invalidates_bound_transaction() {
    let id = FrictionTransactionId::new(202, 0, 0, 0);
    let (a, mut b, applied, mut journal) = applied_neutral_pair(id);

    b.body_type = BodyType::Kinematic;

    assert_eq!(
        finalize_friction_diagnostic(&a, &b, &applied, &mut journal),
        Err(FrictionDiagnosticFinalizeError::ObservationStateMismatch)
    );
    assert_eq!(journal.phase(id), Some(FrictionTransactionPhase::Applied));
}
