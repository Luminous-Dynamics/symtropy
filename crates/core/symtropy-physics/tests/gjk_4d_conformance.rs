// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

use nalgebra::{SMatrix, SVector};
use symtropy_physics::gjk::intersects_support_maps;
use symtropy_physics::support_map::WorldSupportMap;

#[derive(Clone)]
struct OrientedBox4 {
    rotation: SMatrix<f64, 4, 4>,
    center: SVector<f64, 4>,
    half_extents: SVector<f64, 4>,
}

impl WorldSupportMap<4> for OrientedBox4 {
    fn support_world(&self, direction: &SVector<f64, 4>) -> SVector<f64, 4> {
        let local_direction = self.rotation.transpose() * direction;
        let local_support = SVector::from_fn(|axis, _| {
            if local_direction[axis] >= 0.0 {
                self.half_extents[axis]
            } else {
                -self.half_extents[axis]
            }
        });
        self.rotation * local_support + self.center
    }

    fn center_world(&self) -> SVector<f64, 4> {
        self.center
    }

    fn bounding_sphere_world(&self) -> (SVector<f64, 4>, f64) {
        (self.center, self.half_extents.norm())
    }
}

fn det3(m: [[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

/// Four-dimensional generalized cross product: a vector orthogonal to u, v, w.
fn normal_4d(
    u: &SVector<f64, 4>,
    v: &SVector<f64, 4>,
    w: &SVector<f64, 4>,
) -> SVector<f64, 4> {
    let mut normal = SVector::<f64, 4>::zeros();
    for omitted in 0..4 {
        let mut minor = [[0.0; 3]; 3];
        for row in 0..3 {
            let vector = match row {
                0 => u,
                1 => v,
                _ => w,
            };
            let mut column = 0;
            for axis in 0..4 {
                if axis == omitted {
                    continue;
                }
                minor[row][column] = vector[axis];
                column += 1;
            }
        }
        normal[omitted] = if omitted % 2 == 0 {
            det3(minor)
        } else {
            -det3(minor)
        };
    }
    normal
}

/// Independent SAT oracle for full-dimensional 4D parallelepipeds.
///
/// A 4D zonotope facet normal is orthogonal to three generators. The Minkowski
/// difference of two 4D boxes has eight edge generators, so checking every
/// independent triple yields the complete finite SAT candidate set. This
/// deliberately does not call GJK or EPA.
fn sat_intersects_4d(a: &OrientedBox4, b: &OrientedBox4) -> bool {
    let mut generators: Vec<SVector<f64, 4>> = Vec::with_capacity(8);
    for axis in 0..4 {
        generators.push(a.rotation.column(axis).into_owned());
        generators.push(b.rotation.column(axis).into_owned());
    }

    let center_delta = b.center - a.center;
    for i in 0..generators.len() {
        for j in (i + 1)..generators.len() {
            for k in (j + 1)..generators.len() {
                let normal = normal_4d(&generators[i], &generators[j], &generators[k]);
                let normal_squared = normal.norm_squared();
                if !normal_squared.is_finite() || normal_squared <= 1e-20 {
                    continue;
                }

                let radius_a: f64 = (0..4)
                    .map(|axis| {
                        a.half_extents[axis]
                            * normal.dot(&a.rotation.column(axis).into_owned()).abs()
                    })
                    .sum();
                let radius_b: f64 = (0..4)
                    .map(|axis| {
                        b.half_extents[axis]
                            * normal.dot(&b.rotation.column(axis).into_owned()).abs()
                    })
                    .sum();
                let signed_distance = center_delta.dot(&normal).abs();
                let tolerance = 1e-10 * normal.norm() * (1.0 + a.center.norm() + b.center.norm());
                if signed_distance > radius_a + radius_b + tolerance {
                    return false;
                }
            }
        }
    }
    true
}

fn plane_rotation_4d(i: usize, j: usize, angle: f64) -> SMatrix<f64, 4, 4> {
    let mut rotation = SMatrix::<f64, 4, 4>::identity();
    let (sin, cos) = angle.sin_cos();
    rotation[(i, i)] = cos;
    rotation[(j, j)] = cos;
    rotation[(i, j)] = -sin;
    rotation[(j, i)] = sin;
    rotation
}

fn rotation_from_seed(seed: &mut u64) -> SMatrix<f64, 4, 4> {
    fn unit(seed: &mut u64) -> f64 {
        *seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((*seed >> 11) as f64) / ((1u64 << 53) as f64)
    }

    let planes = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
    let mut rotation = SMatrix::<f64, 4, 4>::identity();
    for (i, j) in planes {
        let angle = (2.0 * unit(seed) - 1.0) * std::f64::consts::PI;
        rotation *= plane_rotation_4d(i, j, angle);
    }
    rotation
}

fn unit_interval(seed: &mut u64) -> f64 {
    *seed = seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    ((*seed >> 11) as f64) / ((1u64 << 53) as f64)
}

fn fixture_boxes() -> (OrientedBox4, OrientedBox4) {
    // Frozen separated configuration originally found by comparing the GJK
    // classification with a linear-feasibility oracle over the two box interiors.
    // Rounded matrix entries preserve a comfortable separation margin.
    let rotation_a = SMatrix::<f64, 4, 4>::from_row_slice(&[
        -0.79472879, -0.58881908, -0.12927580,  0.07061171,
         0.23518906, -0.44415566,  0.78038874,  0.37202857,
         0.53113898, -0.67465558, -0.38012781, -0.34385184,
        -0.17602172,  0.02928641,  0.47935793, -0.85928728,
    ]);
    let rotation_b = SMatrix::<f64, 4, 4>::from_row_slice(&[
         0.83832231, -0.06495110,  0.48107666,  0.24811753,
         0.33869864, -0.11305963, -0.82823136,  0.43189531,
         0.37901612,  0.63893251, -0.25498371, -0.61894695,
        -0.19709316,  0.75813199,  0.13259474,  0.60729631,
    ]);
    let a = OrientedBox4 {
        rotation: rotation_a,
        center: SVector::zeros(),
        half_extents: SVector::from([1.0, 0.7, 0.6, 0.4]),
    };
    let b = OrientedBox4 {
        rotation: rotation_b,
        center: SVector::from([1.32237486, 0.91625901, 0.65200879, 1.31963214]),
        half_extents: SVector::from([0.8, 0.55, 0.5, 0.3]),
    };
    (a, b)
}

#[test]
fn separated_rotated_hyperboxes_in_4d_are_not_a_false_positive() {
    let (a, b) = fixture_boxes();
    assert!(
        !sat_intersects_4d(&a, &b),
        "independent 4D SAT oracle must confirm the frozen fixture is separated"
    );
    let ab = intersects_support_maps(&a, &b);
    let ba = intersects_support_maps(&b, &a);

    assert!(
        !ab.intersecting,
        "independent feasibility oracle says separated; GJK returned intersecting after {} iterations",
        ab.iterations
    );
    assert_eq!(
        ab.intersecting,
        ba.intersecting,
        "GJK classification must be symmetric under swapping shapes"
    );
}

#[test]
fn gjk_4d_classification_is_translation_invariant() {
    let (a, b) = fixture_boxes();
    let baseline = intersects_support_maps(&a, &b).intersecting;

    let shift = SVector::from([8.0, -4.0, 2.0, 1.0]);
    let mut translated_a = a;
    let mut translated_b = b;
    translated_a.center += shift;
    translated_b.center += shift;

    assert_eq!(
        baseline,
        intersects_support_maps(&translated_a, &translated_b).intersecting
    );
}

#[test]
fn overlapping_rotated_hyperboxes_in_4d_are_detected() {
    let (a, mut b) = fixture_boxes();
    b.center = SVector::zeros();

    assert!(
        sat_intersects_4d(&a, &b),
        "independent 4D SAT oracle must confirm the overlap positive control"
    );
    assert!(
        intersects_support_maps(&a, &b).intersecting,
        "both boxes contain the shared origin and therefore must intersect"
    );
}

#[test]
fn gjk_4d_matches_independent_sat_on_seeded_oriented_boxes() {
    let mut seed = 0x4d47_4a4b_5f53_4154_u64;
    for case_id in 0..256 {
        let rotation_a = rotation_from_seed(&mut seed);
        let rotation_b = rotation_from_seed(&mut seed);
        let half_extents_a = SVector::from_fn(|_, _| 0.2 + unit_interval(&mut seed));
        let half_extents_b = SVector::from_fn(|_, _| 0.2 + unit_interval(&mut seed));
        let center_a = SVector::from_fn(|_, _| 2.0 * unit_interval(&mut seed) - 1.0);
        let center_b = SVector::from_fn(|_, _| 3.0 * unit_interval(&mut seed) - 1.5);
        let a = OrientedBox4 {
            rotation: rotation_a,
            center: center_a,
            half_extents: half_extents_a,
        };
        let b = OrientedBox4 {
            rotation: rotation_b,
            center: center_b,
            half_extents: half_extents_b,
        };

        let expected = sat_intersects_4d(&a, &b);
        let actual = intersects_support_maps(&a, &b).intersecting;
        assert_eq!(
            actual, expected,
            "4D GJK/SAT classification mismatch at seeded case {case_id}"
        );
    }
}
