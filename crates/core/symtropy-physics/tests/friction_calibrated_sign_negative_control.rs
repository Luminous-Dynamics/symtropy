// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Negative control for #1050.
//!
//! This test deliberately records the current boundary mismatch rather than
//! claiming it is desired production behavior: stable transition-local ΔK is
//! physically interpreted only after `MechanicalUnitCalibration`, while the
//! convenience `FrictionTransitionDelta2d` classifier currently applies a fixed
//! epsilon in solver-energy coordinates before that calibration.

use symtropy_math::Point;
use symtropy_physics::{
    BodyHandle, FrictionTransitionDelta2d, MechanicalUnitCalibration, RigidBody,
    capture_friction_pair_transition_basis_2d_checked,
    classify_friction_pair_transition_2d_checked,
};

fn pair(velocity_a_x: f64) -> (RigidBody<2>, RigidBody<2>) {
    let mut a = RigidBody::<2>::dynamic_sphere(BodyHandle(1), Point::origin(), 0.5, 1.0);
    let b = RigidBody::<2>::dynamic_sphere(BodyHandle(2), Point::origin(), 0.5, 1.0);
    a.linear_velocity[0] = velocity_a_x;
    (a, b)
}

#[test]
fn solver_space_neutral_classification_can_hide_order_one_calibrated_joules() {
    let (mut a, b) = pair(2.0e-8);
    let before = capture_friction_pair_transition_basis_2d_checked(&a, &b).unwrap();

    a.linear_velocity[0] = 1.0e-8;
    let after = capture_friction_pair_transition_basis_2d_checked(&a, &b).unwrap();
    let transition = classify_friction_pair_transition_2d_checked(before, after).unwrap();

    // Stable factored ΔK is approximately -1.5e-16 solver-energy units: smaller
    // than #1034's current fixed 1e-15 convenience-classification epsilon.
    assert!(transition.pair_delta_solver < 0.0);
    assert!(transition.pair_delta_solver.abs() < 1.0e-15);
    assert_eq!(transition.delta, FrictionTransitionDelta2d::Neutral);

    // A valid calibration can make that same nonzero signed transition order-one
    // in SI. The conversion preserves sign rather than erasing it.
    let calibration = MechanicalUnitCalibration::new(1.0e16, 1.0, 1.0).unwrap();
    let physical_delta_joules = calibration
        .signed_energy_to_joules(transition.pair_delta_solver)
        .unwrap();
    assert!(physical_delta_joules < -1.0);
    assert!((physical_delta_joules + 1.5).abs() < 1.0e-12);
}

#[test]
fn equal_si_energy_scale_can_receive_different_precalibration_classes() {
    // Representation A: SI-identity units, -1.5 solver units == -1.5 J.
    let (mut si_a, si_b) = pair(2.0);
    let si_before = capture_friction_pair_transition_basis_2d_checked(&si_a, &si_b).unwrap();
    si_a.linear_velocity[0] = 1.0;
    let si_after = capture_friction_pair_transition_basis_2d_checked(&si_a, &si_b).unwrap();
    let si_transition =
        classify_friction_pair_transition_2d_checked(si_before, si_after).unwrap();
    let si_calibration = MechanicalUnitCalibration::new(1.0, 1.0, 1.0).unwrap();
    let si_joules = si_calibration
        .signed_energy_to_joules(si_transition.pair_delta_solver)
        .unwrap();
    assert!(matches!(
        si_transition.delta,
        FrictionTransitionDelta2d::DissipationCandidate { .. }
    ));

    // Representation B: a 1e16 J/solver-energy calibration with velocities scaled
    // down by 1e-8 gives the same -1.5 J signed energy change, but the fixed
    // solver-space epsilon classifies it as Neutral before calibration.
    let (mut scaled_a, scaled_b) = pair(2.0e-8);
    let scaled_before =
        capture_friction_pair_transition_basis_2d_checked(&scaled_a, &scaled_b).unwrap();
    scaled_a.linear_velocity[0] = 1.0e-8;
    let scaled_after =
        capture_friction_pair_transition_basis_2d_checked(&scaled_a, &scaled_b).unwrap();
    let scaled_transition =
        classify_friction_pair_transition_2d_checked(scaled_before, scaled_after).unwrap();
    let scaled_calibration = MechanicalUnitCalibration::new(1.0e16, 1.0, 1.0).unwrap();
    let scaled_joules = scaled_calibration
        .signed_energy_to_joules(scaled_transition.pair_delta_solver)
        .unwrap();

    assert_eq!(scaled_transition.delta, FrictionTransitionDelta2d::Neutral);
    assert!((si_joules + 1.5).abs() < 1.0e-12);
    assert!((scaled_joules + 1.5).abs() < 1.0e-12);
    assert!((si_joules - scaled_joules).abs() < 1.0e-12);
}
