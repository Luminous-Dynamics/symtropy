// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

use nalgebra::SVector;
use symtropy_physics::gjk::intersects_support_maps;
use symtropy_physics::support_map::WorldSupportMap;
use symtropy_physics::GjkTermination;

#[derive(Clone)]
struct AxisAlignedBox<const D: usize> {
    center: SVector<f64, D>,
    half_extents: SVector<f64, D>,
}

impl<const D: usize> WorldSupportMap<D> for AxisAlignedBox<D> {
    fn support_world(&self, direction: &SVector<f64, D>) -> SVector<f64, D> {
        let local = SVector::from_fn(|axis, _| {
            if direction[axis] >= 0.0 {
                self.half_extents[axis]
            } else {
                -self.half_extents[axis]
            }
        });
        self.center + local
    }

    fn center_world(&self) -> SVector<f64, D> {
        self.center
    }

    fn bounding_sphere_world(&self) -> (SVector<f64, D>, f64) {
        (self.center, self.half_extents.norm())
    }
}

fn aabb_oracle<const D: usize>(a: &AxisAlignedBox<D>, b: &AxisAlignedBox<D>) -> bool {
    (0..D).all(|axis| {
        (a.center[axis] - b.center[axis]).abs()
            <= a.half_extents[axis] + b.half_extents[axis]
    })
}

fn box_at<const D: usize>(center: SVector<f64, D>, half: f64) -> AxisAlignedBox<D> {
    AxisAlignedBox {
        center,
        half_extents: SVector::from_element(half),
    }
}

fn assert_nd_contract<const D: usize>() {
    let a = box_at(SVector::<f64, D>::zeros(), 1.0);
    let separated = box_at(
        SVector::from_fn(|axis, _| if axis == 0 { 3.5 } else { 0.0 }),
        1.0,
    );
    assert!(!aabb_oracle(&a, &separated));
    let separated_result = intersects_support_maps(&a, &separated);
    assert_eq!(
        separated_result.termination,
        GjkTermination::Separated,
        "D={D}: separated hyperboxes should be classified as separated"
    );
    assert!(!separated_result.intersecting);

    let overlapping = box_at(SVector::from_element(0.5), 1.0);
    assert!(aabb_oracle(&a, &overlapping));
    let overlap_result = intersects_support_maps(&a, &overlapping);
    assert_eq!(
        overlap_result.termination,
        GjkTermination::Intersecting,
        "D={D}: overlapping hyperboxes should be classified as intersecting"
    );
    assert!(overlap_result.intersecting);
    assert!(
        overlap_result.simplex.len() <= D + 1,
        "D={D}: the terminal simplex cannot have more than D+1 vertices"
    );

    let swapped = intersects_support_maps(&overlapping, &a);
    assert_eq!(
        swapped.termination,
        GjkTermination::Intersecting,
        "D={D}: swapping support maps must preserve overlap classification"
    );

    let translation = SVector::from_element(32.0);
    let translated_a = box_at(translation, 1.0);
    let translated_overlapping = box_at(translation + SVector::from_element(0.5), 1.0);
    let translated_result = intersects_support_maps(&translated_a, &translated_overlapping);
    assert_eq!(
        translated_result.termination,
        GjkTermination::Intersecting,
        "D={D}: common translation must preserve classification"
    );
}

#[test]
fn active_set_gjk_handles_five_dimensions() {
    assert_nd_contract::<5>();
}

#[test]
fn active_set_gjk_handles_six_dimensions() {
    assert_nd_contract::<6>();
}

#[test]
fn active_set_gjk_handles_eight_dimensions() {
    assert_nd_contract::<8>();
}

#[test]
fn five_dimensional_classification_is_uniform_scale_invariant() {
    let base_a = box_at(SVector::<f64, 5>::zeros(), 1.0);
    let base_b = box_at(SVector::from([3.5, 0.0, 0.0, 0.0, 0.0]), 1.0);
    assert!(!aabb_oracle(&base_a, &base_b));

    for scale in [1e-12, 1e-9, 1.0, 1e9, 1e12] {
        let a = AxisAlignedBox {
            center: base_a.center * scale,
            half_extents: base_a.half_extents * scale,
        };
        let b = AxisAlignedBox {
            center: base_b.center * scale,
            half_extents: base_b.half_extents * scale,
        };
        assert!(!aabb_oracle(&a, &b));
        let result = intersects_support_maps(&a, &b);
        assert_eq!(
            result.termination,
            GjkTermination::Separated,
            "D=5: scale {scale:e} changed classification"
        );
    }
}
