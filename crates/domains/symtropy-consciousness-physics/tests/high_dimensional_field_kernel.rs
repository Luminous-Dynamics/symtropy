// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Regression tests for the dimension-dependent, Plummer-softened harmony-field kernel.
//!
//! These tests specify the implementation contract. They do not assert that this
//! phenomenological influence rule is a fundamental physical field law.

use symtropy_consciousness_physics::{HarmonyField, HarmonySource};
use symtropy_math::Point;

const NUM_HARMONIES: usize = 9;
const SOFTENING_EPSILON: f64 = 1.0;

fn field_with_source<const D: usize>() -> HarmonyField<D> {
    let mut activations = [0.0; NUM_HARMONIES];
    activations[0] = 1.0;
    let source = HarmonySource {
        position: Point::origin(),
        activations,
        strength: 1.0,
        radius: 100.0,
        created_at: 0.0,
        propagation_speed: f64::MAX,
    };
    let mut field = HarmonyField::new();
    field.sources.push(source);
    field
}

fn assert_falloff<const D: usize>(near: Point<D>, far: Point<D>, dimension: usize) {
    let field = field_with_source::<D>();
    let near_value = field.sample(&near)[0];
    let far_value = field.sample(&far)[0];
    let exponent = (dimension as f64 - 1.0).max(1.0);
    let expected_ratio = ((1.0 + SOFTENING_EPSILON.powi(2))
        / (9.0 + SOFTENING_EPSILON.powi(2)))
        .powf(exponent / 2.0);
    let observed_ratio = near_value / far_value;

    assert!(
        (observed_ratio - expected_ratio).abs() <= 1e-12,
        "D={dimension}: observed ratio={observed_ratio:.15}, expected={expected_ratio:.15}"
    );
}

#[test]
fn softened_radial_falloff_matches_the_declared_exponent_in_dimensions_one_through_five() {
    // Distances are r=1 and r=3. Plummer softening epsilon=1 makes the expected
    // near/far ratio ((1^2+epsilon^2)/(3^2+epsilon^2))^(p_D/2), p_D=max(D-1,1).
    assert_falloff(Point::<1>::new([1.0]), Point::<1>::new([3.0]), 1);
    assert_falloff(Point::<2>::new([1.0, 0.0]), Point::<2>::new([3.0, 0.0]), 2);
    assert_falloff(Point::<3>::new([1.0, 0.0, 0.0]), Point::<3>::new([3.0, 0.0, 0.0]), 3);
    assert_falloff(
        Point::<4>::new([1.0, 0.0, 0.0, 0.0]),
        Point::<4>::new([3.0, 0.0, 0.0, 0.0]),
        4,
    );
    assert_falloff(
        Point::<5>::new([1.0, 0.0, 0.0, 0.0, 0.0]),
        Point::<5>::new([3.0, 0.0, 0.0, 0.0, 0.0]),
        5,
    );
}

#[test]
fn documented_nonnegative_harmonies_never_produce_friction_amplification() {
    let field = field_with_source::<3>();
    let aligned = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    let orthogonal = [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];

    for point in [
        Point::<3>::new([1.0, 0.0, 0.0]),
        Point::<3>::new([3.0, 0.0, 0.0]),
    ] {
        let aligned_multiplier = field.friction_multiplier(&point, &aligned);
        let orthogonal_multiplier = field.friction_multiplier(&point, &orthogonal);
        let aligned_impulse = field.impulse_multiplier(&point, &aligned);
        let orthogonal_impulse = field.impulse_multiplier(&point, &orthogonal);
        assert!(
            (0.5..=1.0).contains(&aligned_multiplier),
            "aligned friction multiplier out of documented range: {aligned_multiplier}"
        );
        assert!(
            (0.5..=1.0).contains(&orthogonal_multiplier),
            "orthogonal friction multiplier out of documented range: {orthogonal_multiplier}"
        );
        assert!(
            (0.5..=1.0).contains(&aligned_impulse),
            "aligned impulse multiplier out of documented range: {aligned_impulse}"
        );
        assert!(
            (0.5..=1.0).contains(&orthogonal_impulse),
            "orthogonal impulse multiplier out of documented range: {orthogonal_impulse}"
        );
        assert!(
            (orthogonal_multiplier - 1.0).abs() < 1e-12,
            "orthogonal nonnegative harmonies should be friction-neutral: {orthogonal_multiplier}"
        );
        assert!(
            (orthogonal_impulse - 1.0).abs() < 1e-12,
            "orthogonal nonnegative harmonies should be impulse-neutral: {orthogonal_impulse}"
        );
    }
}
