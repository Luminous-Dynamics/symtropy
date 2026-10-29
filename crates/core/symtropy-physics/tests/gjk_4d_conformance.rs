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
        intersects_support_maps(&a, &b).intersecting,
        "both boxes contain the shared origin and therefore must intersect"
    );
}
