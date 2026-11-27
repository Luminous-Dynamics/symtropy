// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Independent 3D OBB SAT reference for transformed GJK.
//!
//! The reference enumerates the 15 separating axes for two oriented boxes:
//! six face normals and nine pairwise edge cross products. It does not use
//! Symtropy support maps, GJK, EPA, or transformed-shape helpers.

use std::f64::consts::PI;

use nalgebra::SVector;
use symtropy_math::{Bivector, HyperBox, Point, Rotor, Transform};
use symtropy_physics::gjk;

const SAT_EPSILON: f64 = 1.0e-10;
const AXIS_EPSILON: f64 = 1.0e-12;

#[derive(Debug, Clone, Copy)]
struct SatResult {
    intersects: bool,
    degenerate_axes: usize,
    tested_axes: usize,
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

fn sample_half_extents(state: &mut u64) -> [f64; 3] {
    [
        sample_range(state, 0.12, 1.8),
        sample_range(state, 0.12, 1.8),
        sample_range(state, 0.12, 1.8),
    ]
}

/// A proper 3D rotation built from three independent plane rotations.
/// The reference oracle reads the resulting axes from the matrix and does
/// not share any of the engine's collision-support implementation.
fn rotor_3d(angles: [f64; 3]) -> Rotor<3> {
    let r01 = Rotor::from_plane_angle(&Bivector::<3>::unit_plane(0, 1), angles[0]);
    let r02 = Rotor::from_plane_angle(&Bivector::<3>::unit_plane(0, 2), angles[1]);
    let r12 = Rotor::from_plane_angle(&Bivector::<3>::unit_plane(1, 2), angles[2]);
    r12.compose(&r02).compose(&r01)
}

fn local_axes(rotation: &Rotor<3>) -> [SVector<f64, 3>; 3] {
    [
        rotation.rotate_vector(&SVector::new(1.0, 0.0, 0.0)),
        rotation.rotate_vector(&SVector::new(0.0, 1.0, 0.0)),
        rotation.rotate_vector(&SVector::new(0.0, 0.0, 1.0)),
    ]
}

/// Independent 15-axis SAT oracle for two 3D oriented boxes.
///
/// Candidate axes are A0..A2, B0..B2 and Ai × Bj for all i,j.
/// Cross-product axes with near-zero magnitude are counted and skipped.
fn obb_sat_reference(
    center_a: [f64; 3],
    half_a: [f64; 3],
    rotation_a: &Rotor<3>,
    center_b: [f64; 3],
    half_b: [f64; 3],
    rotation_b: &Rotor<3>,
) -> SatResult {
    let axes_a = local_axes(rotation_a);
    let axes_b = local_axes(rotation_b);
    let delta = SVector::from([
        center_b[0] - center_a[0],
        center_b[1] - center_a[1],
        center_b[2] - center_a[2],
    ]);
    let mut candidates = Vec::with_capacity(15);
    candidates.extend(axes_a);
    candidates.extend(axes_b);

    for axis_a in axes_a {
        for axis_b in axes_b {
            candidates.push(axis_a.cross(&axis_b));
        }
    }

    let scale = half_a
        .iter()
        .chain(half_b.iter())
        .copied()
        .fold(1.0_f64, f64::max);
    let tolerance = SAT_EPSILON * scale;
    let mut degenerate_axes = 0usize;
    let mut tested_axes = 0usize;

    for mut axis in candidates {
        let norm = axis.norm();
        if norm <= AXIS_EPSILON {
            degenerate_axes += 1;
            continue;
        }
        axis /= norm;
        tested_axes += 1;

        let radius_a = half_a[0] * axes_a[0].dot(&axis).abs()
            + half_a[1] * axes_a[1].dot(&axis).abs()
            + half_a[2] * axes_a[2].dot(&axis).abs();
        let radius_b = half_b[0] * axes_b[0].dot(&axis).abs()
            + half_b[1] * axes_b[1].dot(&axis).abs()
            + half_b[2] * axes_b[2].dot(&axis).abs();

        if delta.dot(&axis).abs() > radius_a + radius_b + tolerance {
            return SatResult {
                intersects: false,
                degenerate_axes,
                tested_axes,
            };
        }
    }

    SatResult {
        intersects: true,
        degenerate_axes,
        tested_axes,
    }
}

fn intersects_transformed(
    half_a: [f64; 3],
    center_a: [f64; 3],
    rotation_a: &Rotor<3>,
    half_b: [f64; 3],
    center_b: [f64; 3],
    rotation_b: &Rotor<3>,
) -> bool {
    let shape_a = HyperBox::<3>::new(half_a);
    let shape_b = HyperBox::<3>::new(half_b);
    let transform_a = Transform {
        translation: Point::new(center_a),
        rotation: rotation_a.clone(),
    };
    let transform_b = Transform {
        translation: Point::new(center_b),
        rotation: rotation_b.clone(),
    };
    gjk::intersects_transformed(&shape_a, &transform_a, &shape_b, &transform_b).intersecting
}

#[test]
fn transformed_3d_obb_gjk_matches_independent_15_axis_sat() {
    const CASES: usize = 8_192;
    const SEED: u64 = 0x3344_5341_5456_3031;

    let mut state = SEED;
    let mut false_positives = 0usize;
    let mut false_negatives = 0usize;
    let mut translation_only_disagreements = 0usize;
    let mut degenerate_axes = 0usize;
    let mut tested_axes = 0usize;
    let mut first_mismatch = None;

    for case_id in 0..CASES {
        let half_a = sample_half_extents(&mut state);
        let half_b = sample_half_extents(&mut state);
        let center_a = [
            sample_range(&mut state, -2.5, 2.5),
            sample_range(&mut state, -2.5, 2.5),
            sample_range(&mut state, -2.5, 2.5),
        ];
        let center_b = [
            sample_range(&mut state, -3.0, 3.0),
            sample_range(&mut state, -3.0, 3.0),
            sample_range(&mut state, -3.0, 3.0),
        ];
        let rotation_a = rotor_3d([
            sample_range(&mut state, -PI, PI),
            sample_range(&mut state, -PI, PI),
            sample_range(&mut state, -PI, PI),
        ]);
        let rotation_b = rotor_3d([
            sample_range(&mut state, -PI, PI),
            sample_range(&mut state, -PI, PI),
            sample_range(&mut state, -PI, PI),
        ]);

        assert!(rotation_a.is_proper_rotation(1.0e-8));
        assert!(rotation_b.is_proper_rotation(1.0e-8));

        let reference = obb_sat_reference(
            center_a,
            half_a,
            &rotation_a,
            center_b,
            half_b,
            &rotation_b,
        );
        degenerate_axes += reference.degenerate_axes;
        tested_axes += reference.tested_axes;

        let observed = intersects_transformed(
            half_a,
            center_a,
            &rotation_a,
            half_b,
            center_b,
            &rotation_b,
        );
        if observed && !reference.intersects {
            false_positives += 1;
            first_mismatch.get_or_insert_with(|| {
                format!(
                    "false-positive case={case_id} half_a={half_a:?} half_b={half_b:?} center_a={center_a:?} center_b={center_b:?}"
                )
            });
        }
        if !observed && reference.intersects {
            false_negatives += 1;
            first_mismatch.get_or_insert_with(|| {
                format!(
                    "false-negative case={case_id} half_a={half_a:?} half_b={half_b:?} center_a={center_a:?} center_b={center_b:?}"
                )
            });
        }

        // Negative control: deliberately use translation-only GJK. The
        // corpus must detect enough cases where ignoring orientation changes
        // the independent SAT classification.
        let shape_a = HyperBox::<3>::new(half_a);
        let shape_b = HyperBox::<3>::new(half_b);
        let translation_only = gjk::intersects(
            &shape_a,
            &SVector::from(center_a),
            &shape_b,
            &SVector::from(center_b),
        )
        .intersecting;
        if translation_only != reference.intersects {
            translation_only_disagreements += 1;
        }
    }

    println!(
        "3D OBB seed: cases={CASES}, false_positives={false_positives}, false_negatives={false_negatives}, translation_only_disagreements={translation_only_disagreements}, degenerate_axes={degenerate_axes}, tested_axes={tested_axes}, first_mismatch={first_mismatch:?}"
    );
    assert_eq!(false_positives, 0, "3D transformed GJK false positives");
    assert_eq!(false_negatives, 0, "3D transformed GJK false negatives; {first_mismatch:?}");
    assert!(
        translation_only_disagreements > CASES / 10,
        "translation-only negative control too weak: {translation_only_disagreements}/{CASES}"
    );
    assert!(tested_axes > CASES * 10, "too few SAT axes were evaluated");
}

fn rotate_translate(point: [f64; 3], rotation: &Rotor<3>, translation: [f64; 3]) -> [f64; 3] {
    let rotated = rotation.rotate_vector(&SVector::from(point));
    [
        rotated[0] + translation[0],
        rotated[1] + translation[1],
        rotated[2] + translation[2],
    ]
}

#[test]
fn transformed_3d_obb_classification_is_common_rigid_motion_invariant() {
    const CASES: usize = 1_024;
    const SEED: u64 = 0x3344_4d45_5441_3031;

    let mut state = SEED;
    for case_id in 0..CASES {
        let half_a = sample_half_extents(&mut state);
        let half_b = sample_half_extents(&mut state);
        let center_a = [
            sample_range(&mut state, -2.0, 2.0),
            sample_range(&mut state, -2.0, 2.0),
            sample_range(&mut state, -2.0, 2.0),
        ];
        let center_b = [
            sample_range(&mut state, -2.5, 2.5),
            sample_range(&mut state, -2.5, 2.5),
            sample_range(&mut state, -2.5, 2.5),
        ];
        let rotation_a = rotor_3d([
            sample_range(&mut state, -PI, PI),
            sample_range(&mut state, -PI, PI),
            sample_range(&mut state, -PI, PI),
        ]);
        let rotation_b = rotor_3d([
            sample_range(&mut state, -PI, PI),
            sample_range(&mut state, -PI, PI),
            sample_range(&mut state, -PI, PI),
        ]);
        let global_rotation = rotor_3d([
            sample_range(&mut state, -PI, PI),
            sample_range(&mut state, -PI, PI),
            sample_range(&mut state, -PI, PI),
        ]);
        let translation = [
            sample_range(&mut state, -10.0, 10.0),
            sample_range(&mut state, -10.0, 10.0),
            sample_range(&mut state, -10.0, 10.0),
        ];

        let base_sat = obb_sat_reference(
            center_a,
            half_a,
            &rotation_a,
            center_b,
            half_b,
            &rotation_b,
        )
        .intersects;
        let base_gjk = intersects_transformed(
            half_a,
            center_a,
            &rotation_a,
            half_b,
            center_b,
            &rotation_b,
        );

        let moved_a = rotate_translate(center_a, &global_rotation, translation);
        let moved_b = rotate_translate(center_b, &global_rotation, translation);
        let moved_rotation_a = global_rotation.compose(&rotation_a);
        let moved_rotation_b = global_rotation.compose(&rotation_b);

        let moved_sat = obb_sat_reference(
            moved_a,
            half_a,
            &moved_rotation_a,
            moved_b,
            half_b,
            &moved_rotation_b,
        )
        .intersects;
        let moved_gjk = intersects_transformed(
            half_a,
            moved_a,
            &moved_rotation_a,
            half_b,
            moved_b,
            &moved_rotation_b,
        );

        assert_eq!(
            moved_sat, base_sat,
            "SAT common-rigid-motion mismatch at case {case_id}"
        );
        assert_eq!(
            moved_gjk, base_gjk,
            "GJK common-rigid-motion mismatch at case {case_id}"
        );
        assert_eq!(
            base_gjk, base_sat,
            "base GJK/SAT mismatch at case {case_id}"
        );
        assert_eq!(
            moved_gjk, moved_sat,
            "moved GJK/SAT mismatch at case {case_id}"
        );
    }

    println!("3D OBB metamorphic seed: cases={CASES}, mismatches=0");
}
