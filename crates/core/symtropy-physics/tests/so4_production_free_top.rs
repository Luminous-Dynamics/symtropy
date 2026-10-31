// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Production SO(4) torque-free integration campaign.
//!
//! This is an invariant/convergence comparison, not a finite-step equality
//! assertion against the Moser-Veselov map. Production uses semi-implicit
//! Euler; the independent Moser-Veselov reference is a different map.

use nalgebra::{SMatrix, SVector};
use symtropy_math::{Bivector, Point};
use symtropy_physics::{
    BodyHandle, BivectorInertia, RigidBody,
    integrator::{IntegrationLimits, try_integrate_with_limits},
};

#[derive(Clone, Copy, Debug)]
struct Invariants {
    energy: f64,
    norm_casimir: f64,
    pfaffian: f64,
    spatial_momentum: SMatrix<f64, 4, 4>,
    orthogonality_error: f64,
    determinant_error: f64,
}

#[derive(Clone, Copy, Debug)]
struct DriftMetrics {
    energy: f64,
    norm_casimir: f64,
    pfaffian: f64,
    spatial_momentum: f64,
    orthogonality_error: f64,
    determinant_error: f64,
}

fn pfaffian(matrix: &SMatrix<f64, 4, 4>) -> f64 {
    matrix[(0, 1)] * matrix[(2, 3)]
        - matrix[(0, 2)] * matrix[(1, 3)]
        + matrix[(0, 3)] * matrix[(1, 2)]
}

fn snapshot(body: &RigidBody<4>) -> Invariants {
    let inertia = body
        .rotational_inertia
        .as_ref()
        .expect("fixture carries authoritative inertia")
        .rotated_by(&body.transform.rotation);
    let stored_momentum = inertia.angular_momentum(&body.angular_velocity);
    let physical_momentum = stored_momentum.scale(-1.0).to_matrix();

    Invariants {
        energy: inertia.kinetic_energy(&body.angular_velocity),
        norm_casimir: stored_momentum.norm_squared(),
        pfaffian: pfaffian(&physical_momentum),
        spatial_momentum: physical_momentum,
        orthogonality_error: body.transform.rotation.orthogonality_error(),
        determinant_error: (body.transform.rotation.determinant() - 1.0).abs(),
    }
}

fn relative_scalar_error(initial: f64, current: f64) -> f64 {
    (current - initial).abs() / initial.abs().max(1e-30)
}

fn relative_matrix_error(
    initial: &SMatrix<f64, 4, 4>,
    current: &SMatrix<f64, 4, 4>,
) -> f64 {
    (current - initial).norm() / initial.norm().max(1e-30)
}

fn make_body() -> RigidBody<4> {
    let mut body = RigidBody::<4>::dynamic_sphere(
        BodyHandle(0),
        Point::origin(),
        0.5,
        1.0,
    );
    body.linear_damping = 0.0;
    body.angular_damping = 0.0;

    // Full non-diagonal SPD body-space second moment. This couples all six
    // rotational planes while retaining known mass metadata.
    let second_moment = SMatrix::<f64, 4, 4>::new(
        2.0, 0.2, 0.0, 0.1,
        0.2, 3.0, 0.15, 0.0,
        0.0, 0.15, 4.0, 0.25,
        0.1, 0.0, 0.25, 5.0,
    );
    let inertia = BivectorInertia::<4>::from_second_moment_with_mass(1.0, second_moment)
        .expect("frozen mass second moment must be valid");
    body.try_set_rotational_inertia_operator(inertia)
        .expect("body mass and inertia mass metadata agree");

    // Stored B is the negative of physical angular velocity.
    body.angular_velocity.set(0, 1, 0.7);
    body.angular_velocity.set(0, 2, -0.4);
    body.angular_velocity.set(0, 3, 0.2);
    body.angular_velocity.set(1, 2, 1.1);
    body.angular_velocity.set(1, 3, -0.8);
    body.angular_velocity.set(2, 3, 0.35);
    body
}

fn run(dt: f64, horizon: f64) -> DriftMetrics {
    let steps = (horizon / dt).round() as usize;
    assert!((steps as f64 * dt - horizon).abs() < 1e-13);

    let mut body = make_body();
    let initial = snapshot(&body);
    let limits = IntegrationLimits::unbounded();
    let gravity = SVector::<f64, 4>::zeros();

    let mut maximum_energy_drift = 0.0_f64;
    let mut maximum_norm_casimir_drift = 0.0_f64;
    let mut maximum_pfaffian_drift = 0.0_f64;
    let mut maximum_spatial_momentum_drift = 0.0_f64;

    for step in 0..steps {
        try_integrate_with_limits(&mut body, &gravity, dt, &limits)
            .unwrap_or_else(|error| panic!("dt={dt:e}, step={step}: {error:?}"));
        let current = snapshot(&body);
        maximum_energy_drift = maximum_energy_drift
            .max(relative_scalar_error(initial.energy, current.energy));
        maximum_norm_casimir_drift = maximum_norm_casimir_drift
            .max(relative_scalar_error(initial.norm_casimir, current.norm_casimir));
        maximum_pfaffian_drift = maximum_pfaffian_drift
            .max(relative_scalar_error(initial.pfaffian, current.pfaffian));
        maximum_spatial_momentum_drift = maximum_spatial_momentum_drift
            .max(relative_matrix_error(
                &initial.spatial_momentum,
                &current.spatial_momentum,
            ));
    }

    let final_state = snapshot(&body);
    assert!(final_state.orthogonality_error < 2e-10);
    assert!(final_state.determinant_error < 2e-10);

    DriftMetrics {
        energy: maximum_energy_drift,
        norm_casimir: maximum_norm_casimir_drift,
        pfaffian: maximum_pfaffian_drift,
        spatial_momentum: maximum_spatial_momentum_drift,
        orthogonality_error: final_state.orthogonality_error,
        determinant_error: final_state.determinant_error,
    }
}

#[test]
fn production_so4_free_top_invariant_defects_contract_at_first_order() {
    const HORIZON: f64 = 0.5;
    let coarse = run(0.002, HORIZON);
    let medium = run(0.001, HORIZON);
    let fine = run(0.0005, HORIZON);

    let ratios = [
        coarse.energy / medium.energy,
        medium.energy / fine.energy,
        coarse.norm_casimir / medium.norm_casimir,
        medium.norm_casimir / fine.norm_casimir,
        coarse.pfaffian / medium.pfaffian,
        medium.pfaffian / fine.pfaffian,
        coarse.spatial_momentum / medium.spatial_momentum,
        medium.spatial_momentum / fine.spatial_momentum,
    ];

    for ratio in ratios {
        assert!(
            (1.7..2.3).contains(&ratio),
            "semi-implicit Euler invariant defect should contract at first order; ratio={ratio}"
        );
    }
    assert!(fine.energy < 2e-5);
    assert!(fine.norm_casimir < 2e-5);
    assert!(fine.pfaffian < 5e-5);
    assert!(fine.spatial_momentum < 7e-5);
    assert!(fine.orthogonality_error < 2e-10);
    assert!(fine.determinant_error < 2e-10);
}

#[test]
fn production_so4_free_top_is_covariant_under_world_frame_rotation() {
    let mut direct = make_body();
    let mut rotated = make_body();
    let world_rotation = symtropy_math::Rotor::<4>::from_plane_angle(
        &Bivector::<4>::unit_plane(0, 3),
        0.37,
    )
    .compose(
        &symtropy_math::Rotor::<4>::from_plane_angle(
            &Bivector::<4>::unit_plane(1, 2),
            -0.23,
        ),
    );
    let rotation = world_rotation.to_matrix().clone();
    let rotation_t = rotation.transpose();
    rotated.transform.rotation = world_rotation.compose(&rotated.transform.rotation);
    rotated.angular_velocity = Bivector::from_matrix(
        &(rotation * rotated.angular_velocity.to_matrix() * rotation_t),
    );

    let limits = IntegrationLimits::unbounded();
    let gravity = SVector::<f64, 4>::zeros();
    for _ in 0..200 {
        try_integrate_with_limits(&mut direct, &gravity, 0.001, &limits).unwrap();
        try_integrate_with_limits(&mut rotated, &gravity, 0.001, &limits).unwrap();
    }

    let expected_rotation = world_rotation
        .compose(&direct.transform.rotation)
        .to_matrix();
    assert!(
        (rotated.transform.rotation.to_matrix() - expected_rotation).norm() < 2e-10
    );
    let direct_invariants = snapshot(&direct);
    let rotated_invariants = snapshot(&rotated);
    assert!((direct_invariants.energy - rotated_invariants.energy).abs() < 2e-10);
    assert!(
        (direct_invariants.spatial_momentum.norm() - rotated_invariants.spatial_momentum.norm())
            .abs()
            < 2e-10
    );
}

#[test]
fn production_so4_free_top_rejects_nonfinite_state_transactionally() {
    let mut body = make_body();
    body.angular_velocity.set(0, 1, f64::NAN);
    let before_position = body.transform.translation.0;
    let before_rotation = body.transform.rotation.to_matrix().clone();
    let before_force = body.force_accumulator;
    let before_torque = body.torque_accumulator;

    let error = try_integrate_with_limits(
        &mut body,
        &SVector::<f64, 4>::zeros(),
        0.001,
        &IntegrationLimits::unbounded(),
    )
    .unwrap_err();

    assert_eq!(
        error,
        symtropy_physics::integrator::IntegrationError::NonFiniteInput
    );
    assert_eq!(body.transform.translation.0, before_position);
    assert_eq!(body.transform.rotation.to_matrix(), &before_rotation);
    assert_eq!(body.force_accumulator, before_force);
    assert_eq!(body.torque_accumulator, before_torque);
}
