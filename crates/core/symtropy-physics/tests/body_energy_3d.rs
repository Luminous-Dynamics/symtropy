// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

use nalgebra::SVector;
use symtropy_math::{Point, Rotor, Sphere, Transform};
use symtropy_physics::{
    AngularDynamicsError, BodyHandle, BodyType, RigidBody, RigidBodyEnergyError,
    angular_vector_to_bivector,
};

fn anisotropic_body(inertia: [f64; 3], mass: f64) -> RigidBody<3> {
    RigidBody::new(
        BodyHandle(0),
        BodyType::Dynamic,
        Transform::<3>::identity(),
        Box::new(Sphere::new(Point::origin(), 1.0)),
        mass,
        SVector::from(inertia),
    )
}

#[test]
fn checked_body_energy_matches_direct_anisotropic_formula() {
    let mut body = anisotropic_body([1.0, 4.0, 9.0], 2.0);
    body.linear_velocity = SVector::from([3.0, 4.0, 0.0]);
    body.angular_velocity = angular_vector_to_bivector(&SVector::from([1.0, 2.0, 3.0]));

    // Linear: 0.5 * 2 * (3^2 + 4^2) = 25 J.
    // Angular at identity: 0.5 * (1*1^2 + 4*2^2 + 9*3^2) = 49 J.
    let expected = 74.0;
    let exact = body.kinetic_energy_3d_checked().unwrap();
    assert!((exact - expected).abs() < 1.0e-12, "exact={exact}");

    // The historical generic path uses mean inertia and must not accidentally
    // be treated as the anisotropic reference for this body.
    let compatibility = body.kinetic_energy();
    assert!(
        (compatibility - expected).abs() > 1.0,
        "mean-inertia compatibility path unexpectedly matched exact energy: {compatibility}"
    );
}

#[test]
fn body_orientation_changes_exact_energy_for_fixed_world_omega() {
    let mut body = anisotropic_body([1.0, 4.0, 9.0], 1.0);
    body.angular_velocity = angular_vector_to_bivector(&SVector::from([1.0, 0.0, 0.0]));

    let identity_energy = body.kinetic_energy_3d_checked().unwrap();
    assert!((identity_energy - 0.5).abs() < 1.0e-12);

    let quarter_turn = Rotor::from_bivector(&angular_vector_to_bivector(&SVector::from([
        0.0,
        0.0,
        std::f64::consts::FRAC_PI_2,
    ])));
    body.transform.rotation = quarter_turn;
    let rotated_energy = body.kinetic_energy_3d_checked().unwrap();

    assert!(
        (rotated_energy - identity_energy).abs() > 1.0,
        "anisotropic body orientation must change energy for fixed world omega"
    );
    assert!((rotated_energy - 2.0).abs() < 1.0e-10, "rotated={rotated_energy}");
}

#[test]
fn static_body_returns_zero_without_requiring_dynamic_inertia() {
    let body = RigidBody::<3>::static_body(
        BodyHandle(1),
        Point::origin(),
        Box::new(Sphere::new(Point::origin(), 1.0)),
    );
    assert_eq!(body.kinetic_energy_3d_checked().unwrap(), 0.0);
}

#[test]
fn malformed_live_body_state_fails_closed() {
    let mut bad_mass = anisotropic_body([1.0, 2.0, 3.0], 1.0);
    bad_mass.mass = f64::NAN;
    assert_eq!(
        bad_mass.kinetic_energy_3d_checked(),
        Err(RigidBodyEnergyError::InvalidMass)
    );

    let mut bad_velocity = anisotropic_body([1.0, 2.0, 3.0], 1.0);
    bad_velocity.linear_velocity[0] = f64::INFINITY;
    assert_eq!(
        bad_velocity.kinetic_energy_3d_checked(),
        Err(RigidBodyEnergyError::NonFiniteLinearVelocity)
    );

    let mut bad_inertia = anisotropic_body([1.0, 2.0, 3.0], 1.0);
    bad_inertia.inertia[1] = 0.0;
    assert_eq!(
        bad_inertia.kinetic_energy_3d_checked(),
        Err(RigidBodyEnergyError::Angular(
            AngularDynamicsError::InvalidPrincipalMoment { axis: 1 }
        ))
    );
}

#[test]
fn finite_linear_inputs_that_overflow_energy_are_rejected() {
    let mut body = anisotropic_body([1.0, 1.0, 1.0], f64::MAX);
    body.linear_velocity = SVector::from([2.0, 0.0, 0.0]);
    assert_eq!(
        body.kinetic_energy_3d_checked(),
        Err(RigidBodyEnergyError::UnrepresentableLinearEnergy)
    );
}

#[test]
fn finite_angular_inputs_that_overflow_energy_are_rejected() {
    let mut body = anisotropic_body([f64::MAX, 1.0, 1.0], 1.0);
    body.angular_velocity = angular_vector_to_bivector(&SVector::from([2.0, 0.0, 0.0]));
    assert_eq!(
        body.kinetic_energy_3d_checked(),
        Err(RigidBodyEnergyError::Angular(
            AngularDynamicsError::UnrepresentableRotationalEnergy
        ))
    );
}
