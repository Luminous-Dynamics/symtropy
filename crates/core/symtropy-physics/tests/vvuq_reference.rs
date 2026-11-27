// SPDX-License-Identifier: Apache-2.0 OR MIT
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

fn dot3(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn rotor_3d(xy: f64, yz: f64, xz: f64) -> Rotor<3> {
    let rotate_xy = Rotor::from_plane_angle(&Bivector::unit_plane(0, 1), xy);
    let rotate_yz = Rotor::from_plane_angle(&Bivector::unit_plane(1, 2), yz);
    let rotate_xz = Rotor::from_plane_angle(&Bivector::unit_plane(0, 2), xz);
    rotate_xz.compose(&rotate_yz.compose(&rotate_xy))
}

fn axes_3d(rotation: &Rotor<3>) -> [[f64; 3]; 3] {
    std::array::from_fn(|index| {
        let mut basis = SVector::<f64, 3>::zeros();
        basis[index] = 1.0;
        let world = rotation.rotate_vector(&basis);
        [world[0], world[1], world[2]]
    })
}

/// Independent 3D OBB reference using all 15 separating axes:
/// three face normals from each box and nine pairwise edge cross products.
/// It does not use Symtropy support maps, GJK, EPA, or its OBB routines.
fn obb_sat_3d_reference(
    center_a: [f64; 3],
    half_a: [f64; 3],
    rotation_a: &Rotor<3>,
    center_b: [f64; 3],
    half_b: [f64; 3],
    rotation_b: &Rotor<3>,
) -> bool {
    let axes_a = axes_3d(rotation_a);
    let axes_b = axes_3d(rotation_b);
    let delta = [
        center_b[0] - center_a[0],
        center_b[1] - center_a[1],
        center_b[2] - center_a[2],
    ];

    let mut candidates = Vec::with_capacity(15);
    candidates.extend(axes_a);
    candidates.extend(axes_b);
    for axis_a in axes_a {
        for axis_b in axes_b {
            candidates.push(cross3(axis_a, axis_b));
        }
    }

    for axis in candidates {
        let norm = dot3(axis, axis).sqrt();
        if norm < 1.0e-12 {
            continue;
        }

        let radius_a = half_a[0] * dot3(axes_a[0], axis).abs()
            + half_a[1] * dot3(axes_a[1], axis).abs()
            + half_a[2] * dot3(axes_a[2], axis).abs();
        let radius_b = half_b[0] * dot3(axes_b[0], axis).abs()
            + half_b[1] * dot3(axes_b[1], axis).abs()
            + half_b[2] * dot3(axes_b[2], axis).abs();

        if dot3(delta, axis).abs() > radius_a + radius_b + 1.0e-12 * norm {
            return false;
        }
    }
    true
}

fn transform_point_3d(rotation: &Rotor<3>, point: [f64; 3], translation: [f64; 3]) -> [f64; 3] {
    let rotated = rotation.rotate_vector(&SVector::from(point));
    [
        rotated[0] + translation[0],
        rotated[1] + translation[1],
        rotated[2] + translation[2],
    ]
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

    for _ in 0..CASES {
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
    println!(
        "OBB seed: cases={CASES}, false_positives={false_positives}, false_negatives={false_negatives}, translation_only_disagreements={translation_only_disagreements}"
    );
}

#[test]
fn transformed_obb_3d_gjk_matches_independent_fifteen_axis_sat() {
    const CASES: usize = 4_096;
    const SEED: u64 = 0x3344_4f42_4256_5655;

    let half_a = [1.0, 0.55, 0.3];
    let half_b = [0.8, 0.45, 0.25];
    let shape_a = HyperBox::<3>::new(half_a);
    let shape_b = HyperBox::<3>::new(half_b);
    let center_a = [0.0, 0.0, 0.0];

    let mut state = SEED;
    let mut false_positives = 0usize;
    let mut false_negatives = 0usize;
    let mut translation_only_disagreements = 0usize;

    for _ in 0..CASES {
        let rotation_a = rotor_3d(
            sample_range(&mut state, -PI, PI),
            sample_range(&mut state, -PI, PI),
            sample_range(&mut state, -PI, PI),
        );
        let rotation_b = rotor_3d(
            sample_range(&mut state, -PI, PI),
            sample_range(&mut state, -PI, PI),
            sample_range(&mut state, -PI, PI),
        );
        // Concentrate cases inside the unrotated AABB-overlap band. The
        // translation-only baseline therefore predicts overlap for every
        // pair; any rotated SAT separation exposes the ignored orientation.
        let center_b = [
            sample_range(&mut state, -1.62, 1.62),
            sample_range(&mut state, -0.90, 0.90),
            sample_range(&mut state, -0.495, 0.495),
        ];

        let expected =
            obb_sat_3d_reference(center_a, half_a, &rotation_a, center_b, half_b, &rotation_b);
        let observed = gjk::intersects_transformed(
            &shape_a,
            &Transform {
                translation: Point::new(center_a),
                rotation: rotation_a.clone(),
            },
            &shape_b,
            &Transform {
                translation: Point::new(center_b),
                rotation: rotation_b.clone(),
            },
        )
        .intersecting;

        if observed && !expected {
            false_positives += 1;
        }
        if !observed && expected {
            false_negatives += 1;
        }

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
        "3D transformed GJK false positives across {CASES} cases"
    );
    assert_eq!(
        false_negatives, 0,
        "3D transformed GJK false negatives across {CASES} cases"
    );
    assert!(
        translation_only_disagreements > CASES / 10,
        "3D translation-only negative control was too weak: {translation_only_disagreements}/{CASES}"
    );
    println!(
        "3D OBB seed: cases={CASES}, false_positives={false_positives}, false_negatives={false_negatives}, translation_only_disagreements={translation_only_disagreements}"
    );
}

#[test]
fn transformed_obb_3d_classification_is_invariant_under_common_rigid_motion() {
    const CASES: usize = 512;
    const SEED: u64 = 0x3344_4d45_5441_4d4f;

    let half_a = [1.0, 0.55, 0.3];
    let half_b = [0.8, 0.45, 0.25];
    let shape_a = HyperBox::<3>::new(half_a);
    let shape_b = HyperBox::<3>::new(half_b);
    let center_a = [0.0, 0.0, 0.0];
    let mut state = SEED;

    for _ in 0..CASES {
        let rotation_a = rotor_3d(
            sample_range(&mut state, -PI, PI),
            sample_range(&mut state, -PI, PI),
            sample_range(&mut state, -PI, PI),
        );
        let rotation_b = rotor_3d(
            sample_range(&mut state, -PI, PI),
            sample_range(&mut state, -PI, PI),
            sample_range(&mut state, -PI, PI),
        );
        let center_b = [
            sample_range(&mut state, -1.6, 1.6),
            sample_range(&mut state, -0.9, 0.9),
            sample_range(&mut state, -0.5, 0.5),
        ];
        let global_rotation = rotor_3d(
            sample_range(&mut state, -PI, PI),
            sample_range(&mut state, -PI, PI),
            sample_range(&mut state, -PI, PI),
        );
        let translation = [
            sample_range(&mut state, -10.0, 10.0),
            sample_range(&mut state, -10.0, 10.0),
            sample_range(&mut state, -10.0, 10.0),
        ];

        let base_expected =
            obb_sat_3d_reference(center_a, half_a, &rotation_a, center_b, half_b, &rotation_b);
        let base_observed = gjk::intersects_transformed(
            &shape_a,
            &Transform {
                translation: Point::new(center_a),
                rotation: rotation_a.clone(),
            },
            &shape_b,
            &Transform {
                translation: Point::new(center_b),
                rotation: rotation_b.clone(),
            },
        )
        .intersecting;

        let moved_center_a = transform_point_3d(&global_rotation, center_a, translation);
        let moved_center_b = transform_point_3d(&global_rotation, center_b, translation);
        let moved_rotation_a = global_rotation.compose(&rotation_a);
        let moved_rotation_b = global_rotation.compose(&rotation_b);

        let moved_expected = obb_sat_3d_reference(
            moved_center_a,
            half_a,
            &moved_rotation_a,
            moved_center_b,
            half_b,
            &moved_rotation_b,
        );
        let moved_observed = gjk::intersects_transformed(
            &shape_a,
            &Transform {
                translation: Point::new(moved_center_a),
                rotation: moved_rotation_a,
            },
            &shape_b,
            &Transform {
                translation: Point::new(moved_center_b),
                rotation: moved_rotation_b,
            },
        )
        .intersecting;

        assert_eq!(
            moved_expected, base_expected,
            "independent 3D SAT changed under common rigid motion"
        );
        assert_eq!(
            moved_observed, base_observed,
            "transformed 3D GJK changed under common rigid motion"
        );
        assert_eq!(
            base_observed, base_expected,
            "base 3D transformed GJK disagreed with independent SAT"
        );
        assert_eq!(
            moved_observed, moved_expected,
            "moved 3D transformed GJK disagreed with independent SAT"
        );
    }

    println!("3D OBB metamorphic seed: common rigid-motion cases={CASES}, mismatches=0");
}

#[test]
fn transformed_obb_2d_boundary_envelope_matches_sat_and_is_symmetric() {
    const ANGLES: [f64; 8] = [
        0.0,
        PI / 8.0,
        -PI / 6.0,
        PI / 4.0,
        -PI / 2.0,
        3.0 * PI / 4.0,
        PI,
        -0.91,
    ];
    const MARGINS: [f64; 3] = [-1.0e-6, 0.0, 1.0e-6];

    let half_a = [1.0, 0.5];
    let half_b = [0.75, 0.35];
    let shape_a = HyperBox::<2>::new(half_a);
    let shape_b = HyperBox::<2>::new(half_b);
    let mut cases = 0usize;

    for angle in ANGLES {
        let rotation = rotor_2d(angle);
        let (sin, cos) = angle.sin_cos();
        for margin in MARGINS {
            let separation = half_a[0] + half_b[0] + margin;
            let center_a = [0.0, 0.0];
            let center_b = [separation * cos, separation * sin];
            let transform_a = Transform {
                translation: Point::new(center_a),
                rotation: rotation.clone(),
            };
            let transform_b = Transform {
                translation: Point::new(center_b),
                rotation: rotation.clone(),
            };

            let expected = obb_sat_reference(
                center_a, half_a, angle, center_b, half_b, angle,
            );
            let observed_ab =
                gjk::intersects_transformed(&shape_a, &transform_a, &shape_b, &transform_b)
                    .intersecting;
            let observed_ba =
                gjk::intersects_transformed(&shape_b, &transform_b, &shape_a, &transform_a)
                    .intersecting;

            assert_eq!(expected, margin <= 0.0, "SAT boundary convention changed");
            assert_eq!(
                observed_ab, expected,
                "2D transformed GJK disagreed at angle={angle}, margin={margin}"
            );
            assert_eq!(
                observed_ba, expected,
                "2D GJK argument order changed at angle={angle}, margin={margin}"
            );
            cases += 1;
        }
    }

    println!("2D OBB boundary envelope: cases={cases}, mismatches=0");
}

#[test]
fn transformed_obb_3d_boundary_envelope_matches_fifteen_axis_sat_and_is_symmetric() {
    const ROTATIONS: [[f64; 3]; 6] = [
        [0.0, 0.0, 0.0],
        [PI / 8.0, 0.0, 0.0],
        [0.0, -PI / 6.0, 0.0],
        [0.0, 0.0, PI / 4.0],
        [PI / 5.0, -PI / 7.0, PI / 9.0],
        [-PI / 3.0, PI / 8.0, -PI / 5.0],
    ];
    const MARGINS: [f64; 3] = [-1.0e-6, 0.0, 1.0e-6];

    let half_a = [1.0, 0.55, 0.3];
    let half_b = [0.8, 0.45, 0.25];
    let shape_a = HyperBox::<3>::new(half_a);
    let shape_b = HyperBox::<3>::new(half_b);
    let mut cases = 0usize;

    for [xy, yz, xz] in ROTATIONS {
        let rotation = rotor_3d(xy, yz, xz);
        for margin in MARGINS {
            let separation = half_a[0] + half_b[0] + margin;
            let local_offset = SVector::from([separation, 0.0, 0.0]);
            let world_offset = rotation.rotate_vector(&local_offset);
            let center_a = [0.0, 0.0, 0.0];
            let center_b = [world_offset[0], world_offset[1], world_offset[2]];
            let transform_a = Transform {
                translation: Point::new(center_a),
                rotation: rotation.clone(),
            };
            let transform_b = Transform {
                translation: Point::new(center_b),
                rotation: rotation.clone(),
            };

            let expected = obb_sat_3d_reference(
                center_a, half_a, &rotation, center_b, half_b, &rotation,
            );
            let observed_ab =
                gjk::intersects_transformed(&shape_a, &transform_a, &shape_b, &transform_b)
                    .intersecting;
            let observed_ba =
                gjk::intersects_transformed(&shape_b, &transform_b, &shape_a, &transform_a)
                    .intersecting;

            assert_eq!(expected, margin <= 0.0, "3D SAT boundary convention changed");
            assert_eq!(
                observed_ab, expected,
                "3D transformed GJK disagreed at rotation=({xy},{yz},{xz}), margin={margin}"
            );
            assert_eq!(
                observed_ba, expected,
                "3D GJK argument order changed at rotation=({xy},{yz},{xz}), margin={margin}"
            );
            cases += 1;
        }
    }

    println!("3D OBB boundary envelope: cases={cases}, mismatches=0");
}

fn rotate_point(point: [f64; 2], angle: f64) -> [f64; 2] {
    let (sin, cos) = angle.sin_cos();
    [
        cos * point[0] - sin * point[1],
        sin * point[0] + cos * point[1],
    ]
}

#[test]
fn transformed_obb_classification_is_invariant_under_common_rigid_motion() {
    const CASES: usize = 1_024;
    const SEED: u64 = 0x4d45_5441_4d4f_5250;

    let half_a = [1.0, 0.5];
    let half_b = [0.75, 0.35];
    let shape_a = HyperBox::<2>::new(half_a);
    let shape_b = HyperBox::<2>::new(half_b);
    let center_a = [0.0, 0.0];
    let mut state = SEED;

    for _ in 0..CASES {
        let angle_a = sample_range(&mut state, -PI, PI);
        let angle_b = sample_range(&mut state, -PI, PI);
        let center_b = [
            sample_range(&mut state, -2.5, 2.5),
            sample_range(&mut state, -2.0, 2.0),
        ];
        let global_rotation = sample_range(&mut state, -PI, PI);
        let translation = [
            sample_range(&mut state, -10.0, 10.0),
            sample_range(&mut state, -10.0, 10.0),
        ];

        let base_expected = obb_sat_reference(center_a, half_a, angle_a, center_b, half_b, angle_b);
        let base_observed = gjk::intersects_transformed(
            &shape_a,
            &Transform {
                translation: Point::new(center_a),
                rotation: rotor_2d(angle_a),
            },
            &shape_b,
            &Transform {
                translation: Point::new(center_b),
                rotation: rotor_2d(angle_b),
            },
        )
        .intersecting;

        let moved_center_a = rotate_point(center_a, global_rotation);
        let moved_center_b = rotate_point(center_b, global_rotation);
        let moved_center_a = [
            moved_center_a[0] + translation[0],
            moved_center_a[1] + translation[1],
        ];
        let moved_center_b = [
            moved_center_b[0] + translation[0],
            moved_center_b[1] + translation[1],
        ];
        let moved_angle_a = angle_a + global_rotation;
        let moved_angle_b = angle_b + global_rotation;

        let moved_expected = obb_sat_reference(
            moved_center_a,
            half_a,
            moved_angle_a,
            moved_center_b,
            half_b,
            moved_angle_b,
        );
        let moved_observed = gjk::intersects_transformed(
            &shape_a,
            &Transform {
                translation: Point::new(moved_center_a),
                rotation: rotor_2d(moved_angle_a),
            },
            &shape_b,
            &Transform {
                translation: Point::new(moved_center_b),
                rotation: rotor_2d(moved_angle_b),
            },
        )
        .intersecting;

        assert_eq!(
            moved_expected, base_expected,
            "independent SAT changed under common rigid motion"
        );
        assert_eq!(
            moved_observed, base_observed,
            "transformed GJK changed under common rigid motion"
        );
        assert_eq!(
            base_observed, base_expected,
            "base transformed GJK disagreed with independent SAT"
        );
        assert_eq!(
            moved_observed, moved_expected,
            "moved transformed GJK disagreed with independent SAT"
        );
    }

    println!("OBB metamorphic seed: common rigid-motion cases={CASES}, mismatches=0");
}
