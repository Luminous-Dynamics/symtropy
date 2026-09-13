// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

use nalgebra::SVector;
use symtropy_math::Point;
use symtropy_physics::{
    BodyHandle, EnergyTransferLedger, FrictionEvidenceRegime, FrictionMechanicalDelta,
    HeatPartition, RigidBody, ThermalBody, ThermalMaterial, ThermalState,
    apply_friction_impulse_measured, apply_friction_impulse_with_heat,
};

fn body(handle: usize, velocity_x: f64, thermal: bool) -> RigidBody<3> {
    let mut body = RigidBody::dynamic_sphere(
        BodyHandle(handle),
        Point::origin(),
        0.5,
        1.0,
    );
    body.linear_velocity[0] = velocity_x;
    if thermal {
        body.set_thermal(
            ThermalBody::new(
                ThermalMaterial::new(1_000.0, 1.0, 0.5).unwrap(),
                ThermalState::new(300.0).unwrap(),
                1.0,
            )
            .unwrap(),
        );
    }
    body
}

#[test]
fn centered_measurement_matches_audited_friction_reference() {
    let contact = SVector::zeros();
    let impulse = SVector::from([0.5, 0.0, 0.0]);

    let mut measured_a = body(1, 1.0, false);
    let mut measured_b = body(2, 0.0, false);
    let observation = apply_friction_impulse_measured(
        &mut measured_a,
        &mut measured_b,
        &contact,
        &impulse,
    )
    .unwrap();

    assert_eq!(
        observation.regime,
        FrictionEvidenceRegime::CenteredClosedDynamicPair
    );
    assert_eq!(
        observation.delta,
        FrictionMechanicalDelta::DissipationCandidate { joules: 0.25 }
    );

    let mut audited_a = body(1, 1.0, true);
    let mut audited_b = body(2, 0.0, true);
    let mut ledger = EnergyTransferLedger::new();
    let audited = apply_friction_impulse_with_heat(
        &mut audited_a,
        &mut audited_b,
        &contact,
        &impulse,
        HeatPartition::equal(),
        &mut ledger,
    )
    .unwrap();

    assert_eq!(observation.centered_promotable_loss_candidate_joules(), Some(0.25));
    assert!((audited.dissipated_joules - 0.25).abs() < 1.0e-12);
    assert!((audited.dissipated_joules + observation.pair_delta_joules).abs() < 1.0e-12);

    // The measurement seam and the audited #9 transaction must represent the
    // exact same mechanical transition before the latter adds thermal state.
    assert_eq!(measured_a.linear_velocity, audited_a.linear_velocity);
    assert_eq!(measured_b.linear_velocity, audited_b.linear_velocity);
    assert_eq!(measured_a.angular_velocity, audited_a.angular_velocity);
    assert_eq!(measured_b.angular_velocity, audited_b.angular_velocity);

    assert!((audited.heat_to_a_joules + audited.heat_to_b_joules - 0.25).abs() < 1.0e-12);
    assert_eq!(ledger.net_external_joules(), 0.0);
}

#[test]
fn injected_measurement_has_no_audited_heat_equivalent() {
    let contact = SVector::zeros();
    let impulse = SVector::from([2.0, 0.0, 0.0]);

    let mut measured_a = body(1, 1.0, false);
    let mut measured_b = body(2, 0.0, false);
    let observation = apply_friction_impulse_measured(
        &mut measured_a,
        &mut measured_b,
        &contact,
        &impulse,
    )
    .unwrap();

    assert_eq!(
        observation.delta,
        FrictionMechanicalDelta::SolverInjection { joules: 2.0 }
    );
    assert_eq!(observation.centered_promotable_loss_candidate_joules(), None);

    let mut audited_a = body(1, 1.0, true);
    let mut audited_b = body(2, 0.0, true);
    let before_a = audited_a.linear_velocity;
    let before_b = audited_b.linear_velocity;
    let mut ledger = EnergyTransferLedger::new();

    let error = apply_friction_impulse_with_heat(
        &mut audited_a,
        &mut audited_b,
        &contact,
        &impulse,
        HeatPartition::equal(),
        &mut ledger,
    )
    .unwrap_err();

    assert_eq!(error, symtropy_physics::DissipationError::NonDissipativeImpulse);
    assert_eq!(audited_a.linear_velocity, before_a);
    assert_eq!(audited_b.linear_velocity, before_b);
    assert!(ledger.transfers().is_empty());
}
