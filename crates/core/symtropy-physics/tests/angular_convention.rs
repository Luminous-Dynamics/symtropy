// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

use nalgebra::{SMatrix, SVector};
use symtropy_math::{Bivector, Rotor};
use symtropy_physics::{
    AngularDynamicsError, PrincipalInertia3, angular_vector_to_bivector,
    angular_velocity_at_offset, world_angular_momentum,
};

#[test]
fn rotor_finite_difference_matches_physical_point_velocity() {
    let omega_world = SVector::from([0.4, -0.3, 1.2]);
    let angular_velocity = angular_vector_to_bivector(&omega_world);
    let offset = SVector::from([0.8, -0.2, 0.5]);

    let expected = angular_velocity_at_offset(&angular_velocity, &offset)
        .expect("finite reference point velocity");

    // Differentiate the actual Symtropy Rotor convention rather than relying on
    // a cross-product identity alone. `Rotor::from_bivector` computes exp(-B),
    // so the derivative at dt=0 must be -B*r.
    let dt = 1.0e-7;
    let delta = Rotor::from_bivector(&angular_velocity.scale(dt));
    let moved = delta.rotate_vector(&offset);
    let finite_difference = (moved - offset) / dt;

    let error = (finite_difference - expected).norm();
    assert!(
        error < 1.0e-6,
        "Rotor finite-difference velocity must match -B*r: error={error:e}, finite_difference={finite_difference:?}, expected={expected:?}"
    );
}

#[test]
fn arbitrary_finite_matrix_is_not_accepted_as_reference_rotation() {
    let mut reflection = SMatrix::<f64, 3, 3>::identity();
    reflection[(0, 0)] = -1.0;
    let rotation = Rotor::from_matrix(reflection);
    let inertia = PrincipalInertia3::new([1.0, 2.0, 3.0]).unwrap();

    assert!(matches!(
        world_angular_momentum(&rotation, &Bivector::<3>::zero(), inertia),
        Err(AngularDynamicsError::InvalidRotation)
    ));
}
