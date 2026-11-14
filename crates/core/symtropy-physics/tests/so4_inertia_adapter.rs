// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Explicit sign- and frame-convention bridge between the physical SO(4)
//! Moser-Veselov reference and production BivectorInertia<4>.

use nalgebra::SMatrix;
use symtropy_math::{Bivector, Rotor};
use symtropy_physics::BivectorInertia;

fn pairing<const D: usize>(left: &Bivector<D>, right: &Bivector<D>) -> f64 {
    (0..D)
        .flat_map(|i| ((i + 1)..D).map(move |j| (i, j)))
        .map(|(i, j)| left.get(i, j) * right.get(i, j))
        .sum()
}

fn fixture() -> (BivectorInertia<4>, Bivector<4>) {
    let second_moment = SMatrix::<f64, 4, 4>::new(
        2.0, 0.2, 0.0, 0.1,
        0.2, 3.0, 0.15, 0.0,
        0.0, 0.15, 4.0, 0.25,
        0.1, 0.0, 0.25, 5.0,
    );
    let inertia = BivectorInertia::<4>::from_second_moment(second_moment)
        .expect("the frozen second-moment matrix is symmetric positive definite");

    let mut physical_angular_velocity = Bivector::<4>::zero();
    physical_angular_velocity.set(0, 1, 0.7);
    physical_angular_velocity.set(0, 2, -0.4);
    physical_angular_velocity.set(0, 3, 0.2);
    physical_angular_velocity.set(1, 2, 1.1);
    physical_angular_velocity.set(1, 3, -0.8);
    physical_angular_velocity.set(2, 3, 0.35);

    // Production's stored B has physical point velocity -B*r.
    let stored_angular_state = physical_angular_velocity.scale(-1.0);
    (inertia, stored_angular_state)
}

#[test]
fn production_inertia_sign_bridge_matches_physical_so4_metric() {
    let (inertia, stored_b) = fixture();
    let second_moment = inertia.second_moment();
    let physical_omega = stored_b.scale(-1.0);

    // Production stores L_store = B*S + S*B = -M_physical.
    let stored_momentum = inertia.angular_momentum(&stored_b);
    let physical_momentum = stored_momentum.scale(-1.0);
    let second_moment = second_moment.clone();
    let physical_omega_matrix = physical_omega.to_matrix();
    let expected_physical_momentum = second_moment * physical_omega_matrix
        + physical_omega_matrix * second_moment;

    assert!(
        (physical_momentum.to_matrix() - expected_physical_momentum).norm() < 1e-11,
        "production stored-sign momentum did not map to the physical SO(4) momentum"
    );

    // Inverting the stored-sign momentum must recover stored B, not physical W.
    let recovered_stored = inertia
        .angular_velocity_from_momentum(&stored_momentum, 1e-12)
        .expect("the full-rank SO(4) inertia operator is invertible");
    assert!(
        recovered_stored.add(&stored_b.scale(-1.0)).norm() < 1e-11,
        "inverse inertia returned a mismatched stored sign"
    );

    // Production kinetic energy is positive and equals 1/2 <M_physical,W_physical>.
    let production_energy = inertia.kinetic_energy(&stored_b);
    let physical_energy = 0.5 * pairing(&physical_momentum, &physical_omega);
    assert!((production_energy - physical_energy).abs() < 1e-11);
}

#[test]
fn production_gyroscopic_rhs_matches_physical_euler_arnold_sign() {
    let (inertia, stored_b) = fixture();
    let zero_torque = Bivector::<4>::zero();
    let stored_b_dot = inertia
        .angular_acceleration(&stored_b, &zero_torque, 1e-12)
        .expect("valid full-rank SO(4) inertia");

    let stored_momentum = inertia.angular_momentum(&stored_b);
    let physical_momentum = stored_momentum.scale(-1.0);
    let physical_omega = stored_b.scale(-1.0);
    let physical_momentum_dot = physical_momentum.to_matrix() * physical_omega.to_matrix()
        - physical_omega.to_matrix() * physical_momentum.to_matrix();

    // Both stored quantities carry the opposite physical sign:
    // I(B_dot) = -M_dot for torque-free motion.
    let stored_momentum_dot = inertia.angular_momentum(&stored_b_dot);
    assert!(
        (stored_momentum_dot.to_matrix() + physical_momentum_dot).norm() < 2e-10,
        "production gyroscopic RHS disagrees with physical Euler-Arnold dynamics"
    );
}

#[test]
fn production_four_dimensional_inertia_is_body_basis_covariant() {
    let (inertia, stored_b) = fixture();
    let r_02 = Rotor::<4>::from_plane_angle(&Bivector::unit_plane(0, 2), 0.41);
    let r_13 = Rotor::<4>::from_plane_angle(&Bivector::unit_plane(1, 3), -0.23);
    let body_rotation = r_02.compose(&r_13);
    let rotation = body_rotation.to_matrix().clone();
    let rotation_t = rotation.transpose();

    let rotated_inertia = inertia.rotated_by(&body_rotation);
    let rotated_stored_b =
        Bivector::from_matrix(&(rotation * stored_b.to_matrix() * rotation_t));
    let rotated_momentum = rotated_inertia.angular_momentum(&rotated_stored_b);
    let expected_momentum = rotation * inertia.angular_momentum(&stored_b).to_matrix()
        * rotation_t;

    assert!((rotated_momentum.to_matrix() - expected_momentum).norm() < 2e-10);
    assert!(
        (rotated_inertia.kinetic_energy(&rotated_stored_b)
            - inertia.kinetic_energy(&stored_b))
        .abs()
            < 2e-10
    );

    let recovered = rotated_inertia
        .angular_velocity_from_momentum(&rotated_momentum, 1e-12)
        .expect("rotated full-rank inertia remains invertible");
    assert!(recovered.add(&rotated_stored_b.scale(-1.0)).norm() < 2e-10);
}

#[test]
fn production_rotor_uses_the_physical_rotation_sign_for_stored_bivectors() {
    let (inertia, stored_b) = fixture();
    let physical_omega = stored_b.scale(-1.0);

    // Rotor::from_bivector exponentiates exp(-B), which is exp(+W_physical).
    // This independently anchors the sign used by the reference step adapter.
    let rotor = Rotor::<4>::from_bivector(&stored_b);
    let reverse_physical = Rotor::<4>::from_bivector(&physical_omega).reverse();
    assert!(
        (rotor.to_matrix() - reverse_physical.to_matrix()).norm() < 2e-10
    );

    let stored_momentum = inertia.angular_momentum(&stored_b);
    let recovered = inertia
        .angular_velocity_from_momentum(&stored_momentum, 1e-12)
        .unwrap();
    assert!(recovered.add(&stored_b.scale(-1.0)).norm() < 1e-11);
}
