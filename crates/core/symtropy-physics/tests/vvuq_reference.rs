// SPDX-License-Identifier: AGPL-3.0-or-later
//! Small analytical and independent-reference VVUQ seed for `symtropy-physics`.
//!
//! The suite is intentionally narrower than the engine's public API:
//! - uniform-gravity semi-implicit Euler convergence against a closed form;
//! - transformed 2D OBB overlap against an independent SAT implementation.
//!
//! This does not qualify 3D/4D collisions, general CCD, full rigid-body
//! validation, experimental physics fidelity, or cross-platform determinism.

use std::f64::consts::PI;

use nalgebra::SVector;
use symtropy_math::{Bivector, HyperBox, Point, Rotor, Transform};
use symtropy_physics::{PhysicsWorld, gjk};

fn free_fall_errors(dt: f64) -> (f64, f64, f64) {
    let gravity = -9.81_f64;
    let duration = 1.0_f64;
    let steps = (duration / dt).round() as usize;
    let actual_duration = steps as f64 * dt;

    let initial = Point::new([0.0, 100.0, 0.0]);
    let mut world = PhysicsWorld::<3>::new(SVector::from([0.0, gravity, 0.0]));
    let handle = world.add_sphere(initial, 0.5, 1.0);
    {
        let body = world.body_mut(handle).expect("body was just inserted");
        body.linear_damping = 0.0;
        body.angular_damping = 0.0;
    }

    for _ in 0..steps {
        world.step(dt);
    }

    let body = world.body(handle).expect("body remains in world");
    let expected_y = initial.coord(1) + 0.5 * gravity * actual_duration * actual_duration;
    let expected_vy = gravity * actual_duration;
    let position_error = body.transform.translation.coord(1) - expected_y;
    let velocity_error = body.linear_velocity[1] - expected_vy;

    // For v[n+1] = v[n] + g*dt and x[n+1] = x[n] + v[n+1]*dt,
    // the exact discrete position offset from continuous free fall is
    // g*T*dt/2. This is not a fitted tolerance: it is the scheme's
    // analytically derived truncation error for constant acceleration.
    let exact_discrete_position_error = 0.5 * gravity * actual_duration * dt;
    (
        position_error,
        velocity_error,
        exact_discrete_position_error,
    )
}

#[test]
fn semi_implicit_euler_matches_its_exact_discrete_error_and_first_order_rate() {
    let timesteps = [1.0 / 30.0, 1.0 / 60.0, 1.0 / 120.0, 1.0 / 240.0];
    let mut position_errors = Vec::new();

    for dt in timesteps {
        let (position_error, velocity_error, expected_discrete_error) = free_fall_errors(dt);

        assert!(
            (position_error - expected_discrete_error).abs() <= 1.0e-9,
            "dt={dt}: observed position error {position_error:.15e}, expected exact discrete error {expected_discrete_error:.15e}"
        );
        assert!(
            velocity_error.abs() <= 1.0e-10,
            "dt={dt}: constant-acceleration velocity should be exact, error={velocity_error:.15e}"
        );
        position_errors.push(position_error.abs());
    }

    for pair in position_errors.windows(2) {
        let ratio = pair[0] / pair[1];
        assert!(
            (ratio - 2.0).abs() <= 1.0e-8,
            "position error should converge at first order when dt halves; ratio={ratio:.12}"
        );
    }
}

fn next_u64(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state
}

fn unit_interval(state: &mut u64) -> f64 {
    let mantissa = next_u64(state) >> 11;
    mantissa as f64 / ((1_u64 << 53) as f64)
}

fn sample_range(state: &mut u64, min: f64, max: f64) -> f64 {
    min + (max - min) * unit_interval(state)
}

fn rotor_2d(angle: f64) -> Rotor<2> {
    Rotor::from_plane_angle(&Bivector::unit_plane(0, 1), angle)
}

fn local_axes(angle: f64) -> [[f64; 2]; 2] {
    let (sin, cos) = angle.sin_cos();
    [[cos, sin], [-sin, cos]]
}

fn dot(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

/// Independent 2D OBB reference using the separating-axis theorem.
///
/// It consumes only centers, half-extents, and angles. It does not call
/// Symtropy support maps, GJK, EPA, or transformed-shape helpers.
fn obb_sat_reference(
    center_a: [f64; 2],
    half_a: [f64; 2],
    angle_a: f64,
    center_b: [f64; 2],
    half_b: [f64; 2],
    angle_b: f64,
) -> bool {
    let axes_a = local_axes(angle_a);
    let axes_b = local_axes(angle_b);
    let delta = [center_b[0] - center_a[0], center_b[1] - center_a[1]];

    for axis in axes_a.into_iter().chain(axes_b) {
        let radius_a =
            half_a[0] * dot(axes_a[0], axis).abs() + half_a[1] * dot(axes_a[1], axis).abs();
        let radius_b =
            half_b[0] * dot(axes_b[0], axis).abs() + half_b[1] * dot(axes_b[1], axis).abs();
        if dot(delta, axis).abs() > radius_a + radius_b + 1.0e-12 {
            return false;
        }
    }
    true
}

#[test]
fn transformed_obb_gjk_matches_seeded_independent_sat_reference() {
    const CASES: usize = 4_096;
    const SEED: u64 = 0x4551_4356_5655_3031;

    let half_a = [1.0, 0.5];
    let half_b = [0.75, 0.35];
    let shape_a = HyperBox::<2>::new(half_a);
    let shape_b = HyperBox::<2>::new(half_b);
    let center_a = [0.0, 0.0];

    let mut state = SEED;
    let mut false_positives = 0usize;
    let mut false_negatives = 0usize;
    let mut translation_only_disagreements = 0usize;

    for case_id in 0..CASES {
        let angle_a = sample_range(&mut state, -PI, PI);
        let angle_b = sample_range(&mut state, -PI, PI);
        let center_b = [
            sample_range(&mut state, -3.0, 3.0),
            sample_range(&mut state, -2.0, 2.0),
        ];

        let transform_a = Transform {
            translation: Point::new(center_a),
            rotation: rotor_2d(angle_a),
        };
        let transform_b = Transform {
            translation: Point::new(center_b),
            rotation: rotor_2d(angle_b),
        };

        let expected = obb_sat_reference(center_a, half_a, angle_a, center_b, half_b, angle_b);
        let observed = gjk::intersects_transformed(&shape_a, &transform_a, &shape_b, &transform_b)
            .intersecting;

        if observed && !expected {
            false_positives += 1;
        }
        if !observed && expected {
            false_negatives += 1;
        }

        // Negative control: deliberately drop both rotations. The corpus must
        // contain cases where this incorrect implementation disagrees with
        // the independent reference; otherwise the test would not exercise
        // transform awareness.
        let translation_only = gjk::intersects(
            &shape_a,
            &SVector::from(center_a),
            &shape_b,
            &SVector::from(center_b),
        )
        .intersecting;
        if translation_only != expected {
            translation_only_disagreements += 1;
        }
    }

    assert_eq!(
        false_positives, 0,
        "transformed GJK false positives across {CASES} cases"
    );
    assert_eq!(
        false_negatives, 0,
        "transformed GJK false negatives across {CASES} cases"
    );
    assert!(
        translation_only_disagreements > CASES / 10,
        "negative control was too weak: only {translation_only_disagreements}/{CASES} translation-only cases disagreed"
    );
}
