// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Projection from ND physics space to screen/render coordinates.

use bevy::prelude::{Reflect, Resource};
use nalgebra::SVector;
use symtropy_math::{Hyperplane, Point};

/// Trait for projecting ND points to Bevy-compatible 2D/3D coordinates.
pub trait Projector<const D: usize>: Send + Sync {
    /// Project an ND point to a 3D position (x, y, z) for Bevy rendering.
    /// Returns (x, y, z) — for 2D rendering, z is used for sprite ordering.
    fn project(&self, point: &Point<D>) -> [f32; 3];

    /// Whether the point passes this projector's slice/domain and numeric-validity checks.
    ///
    /// This is not viewport/frustum culling; the host camera still decides whether a
    /// finite projected point lies inside the rendered viewport.
    fn is_visible(&self, point: &Point<D>) -> bool;

    /// Scale factor: how many render pixels per physics unit.
    fn scale(&self) -> f32;
}

/// 2D projector: direct 1:1 mapping from physics (x, y) to Bevy (x, y, 0).
pub struct Projector2D {
    /// Pixels per physics unit.
    pub pixels_per_unit: f32,
    /// Camera center in physics space.
    pub camera_x: f64,
    pub camera_y: f64,
}

impl Projector2D {
    pub fn new(pixels_per_unit: f32) -> Self {
        Self {
            pixels_per_unit,
            camera_x: 0.0,
            camera_y: 0.0,
        }
    }

    fn configuration_is_valid(&self) -> bool {
        self.pixels_per_unit.is_finite()
            && self.pixels_per_unit > 0.0
            && self.camera_x.is_finite()
            && self.camera_y.is_finite()
    }
}

impl Projector<2> for Projector2D {
    fn project(&self, point: &Point<2>) -> [f32; 3] {
        if !self.configuration_is_valid() {
            return [f32::NAN; 3];
        }
        let x = (point.coord(0) - self.camera_x) * f64::from(self.pixels_per_unit);
        let y = (point.coord(1) - self.camera_y) * f64::from(self.pixels_per_unit);
        representable_xyz([x, y, 0.0])
    }

    fn is_visible(&self, point: &Point<2>) -> bool {
        self.project(point).iter().all(|value| value.is_finite())
    }

    fn scale(&self) -> f32 {
        self.pixels_per_unit
    }
}

/// 3D projector: direct mapping from physics (x, y, z) to Bevy (x, y, z).
pub struct Projector3D {
    pub scale_factor: f32,
}

impl Projector3D {
    pub fn new(scale_factor: f32) -> Self {
        Self { scale_factor }
    }
}

impl Projector<3> for Projector3D {
    fn project(&self, point: &Point<3>) -> [f32; 3] {
        if !self.scale_factor.is_finite() || self.scale_factor <= 0.0 {
            return [f32::NAN; 3];
        }
        let scale = f64::from(self.scale_factor);
        representable_xyz([
            point.coord(0) * scale,
            point.coord(1) * scale,
            point.coord(2) * scale,
        ])
    }

    fn is_visible(&self, point: &Point<3>) -> bool {
        self.project(point).iter().all(|value| value.is_finite())
    }

    fn scale(&self) -> f32 {
        self.scale_factor
    }
}

/// Legacy W-axis projector for 4D points and body centers.
///
/// Filters points by distance from the hyperplane w = w_slice and projects their
/// first three coordinates. It does not compute exact intersections of extended
/// geometry with the hyperplane; use a dedicated mesh/collider slicing adapter for
/// actual continuous cross-sections. Moving the slice position reveals different
/// sets of point/body-center samples.
#[derive(Resource, Reflect, Debug, Clone, Copy)]
pub struct Projector4D {
    /// Position of the slicing hyperplane along the W axis.
    pub w_slice: f64,
    /// Thickness of the slice (objects within this distance of the hyperplane are visible).
    pub slice_thickness: f64,
    /// Scale factor for rendering.
    pub scale_factor: f32,
}

impl Projector4D {
    pub fn new(w_slice: f64, slice_thickness: f64, scale_factor: f32) -> Self {
        Self {
            w_slice,
            slice_thickness,
            scale_factor,
        }
    }

    /// True only when the W-slice configuration has finite values and positive
    /// thickness/scale. The constructor remains source-compatible; consumers should
    /// check this before wiring the projector into a render system.
    pub fn configuration_is_valid(&self) -> bool {
        self.w_slice.is_finite()
            && self.slice_thickness.is_finite()
            && self.slice_thickness > 0.0
            && self.scale_factor.is_finite()
            && self.scale_factor > 0.0
    }

    /// The slicing hyperplane: { (x,y,z,w) : w = w_slice }.
    pub fn slice_plane(&self) -> Hyperplane<4> {
        Hyperplane::new(SVector::from([0.0, 0.0, 0.0, 1.0]), self.w_slice)
    }

    /// Distance of a 4D point from the slicing hyperplane (along W).
    /// Returns infinity for non-finite W coordinates or an invalid slice position.
    pub fn w_distance(&self, point: &Point<4>) -> f64 {
        if !self.w_slice.is_finite() || !point.coord(3).is_finite() {
            return f64::INFINITY;
        }
        (point.coord(3) - self.w_slice).abs()
    }

    /// Alpha (opacity) based on distance from slice plane. Invalid configurations
    /// and non-finite points fail closed as fully transparent.
    pub fn alpha(&self, point: &Point<4>) -> f32 {
        if !self.configuration_is_valid() || (0..4).any(|axis| !point.coord(axis).is_finite()) {
            return 0.0;
        }
        let dist = self.w_distance(point);
        if !dist.is_finite() || dist >= self.slice_thickness {
            return 0.0;
        }
        (1.0 - dist / self.slice_thickness) as f32
    }
}

impl Projector<4> for Projector4D {
    fn project(&self, point: &Point<4>) -> [f32; 3] {
        if !self.configuration_is_valid() {
            return [f32::NAN; 3];
        }
        let scale = f64::from(self.scale_factor);
        // Project (x, y, z, w) to (x, y, z), retaining W in slice visibility.
        representable_xyz([
            point.coord(0) * scale,
            point.coord(1) * scale,
            point.coord(2) * scale,
        ])
    }

    fn is_visible(&self, point: &Point<4>) -> bool {
        self.configuration_is_valid()
            && self.w_distance(point) < self.slice_thickness
            && self.project(point).iter().all(|value| value.is_finite())
    }

    fn scale(&self) -> f32 {
        self.scale_factor
    }
}

/// Converts f64 projected coordinates to f32 only when every coordinate is
/// representable. Non-finite/overflowing output uses NaNs so callers can fail closed.
fn representable_xyz(coordinates: [f64; 3]) -> [f32; 3] {
    if coordinates
        .iter()
        .all(|value| value.is_finite() && value.abs() <= f64::from(f32::MAX))
    {
        [
            coordinates[0] as f32,
            coordinates[1] as f32,
            coordinates[2] as f32,
        ]
    } else {
        [f32::NAN; 3]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projector_2d_basic() {
        let proj = Projector2D::new(32.0);
        let p = Point::new([1.0, 2.0]);
        let [x, y, z] = proj.project(&p);
        assert!((x - 32.0).abs() < 1e-5);
        assert!((y - 64.0).abs() < 1e-5);
        assert!((z - 0.0).abs() < 1e-5);
    }

    #[test]
    fn projector_2d_with_camera_offset() {
        let mut proj = Projector2D::new(32.0);
        proj.camera_x = 5.0;
        proj.camera_y = 10.0;
        let p = Point::new([6.0, 12.0]);
        let [x, y, _] = proj.project(&p);
        assert!((x - 32.0).abs() < 1e-5); // (6-5) * 32
        assert!((y - 64.0).abs() < 1e-5); // (12-10) * 32
    }

    #[test]
    fn projector_3d_basic() {
        let proj = Projector3D::new(1.0);
        let p = Point::new([1.0, 2.0, 3.0]);
        let [x, y, z] = proj.project(&p);
        assert!((x - 1.0).abs() < 1e-5);
        assert!((y - 2.0).abs() < 1e-5);
        assert!((z - 3.0).abs() < 1e-5);
    }

    #[test]
    fn projector_4d_on_slice_visible() {
        let proj = Projector4D::new(5.0, 1.0, 1.0);
        let p = Point::new([1.0, 2.0, 3.0, 5.0]); // w = 5.0, exactly on slice
        assert!(proj.is_visible(&p));
        assert!((proj.alpha(&p) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn projector_4d_off_slice_invisible() {
        let proj = Projector4D::new(5.0, 1.0, 1.0);
        let p = Point::new([1.0, 2.0, 3.0, 10.0]); // w = 10.0, far from slice
        assert!(!proj.is_visible(&p));
        assert!((proj.alpha(&p) - 0.0).abs() < 1e-5);
    }

    #[test]
    fn projector_4d_edge_fades() {
        let proj = Projector4D::new(5.0, 2.0, 1.0);
        let p = Point::new([1.0, 2.0, 3.0, 6.0]); // w = 6.0, distance 1.0 from slice
        assert!(proj.is_visible(&p));
        // Alpha = 1.0 - 1.0/2.0 = 0.5
        assert!((proj.alpha(&p) - 0.5).abs() < 1e-5);
    }

    #[test]
    fn projector_4d_projects_xyz() {
        let proj = Projector4D::new(0.0, 1.0, 2.0);
        let p = Point::new([1.0, 2.0, 3.0, 0.5]);
        let [x, y, z] = proj.project(&p);
        assert!((x - 2.0).abs() < 1e-5); // 1.0 * 2.0
        assert!((y - 4.0).abs() < 1e-5); // 2.0 * 2.0
        assert!((z - 6.0).abs() < 1e-5); // 3.0 * 2.0
    }

    #[test]
    fn projector_4d_moving_slice_reveals() {
        let mut proj = Projector4D::new(0.0, 0.5, 1.0);

        // Hidden room at w=5.0
        let hidden = Point::new([1.0, 2.0, 3.0, 5.0]);

        // Not visible from w=0.0
        assert!(!proj.is_visible(&hidden));

        // Move slice to w=5.0 — now visible
        proj.w_slice = 5.0;
        assert!(proj.is_visible(&hidden));
    }

    #[test]
    fn planar_projector_fails_closed_for_invalid_scales_and_coordinates() {
        let invalid_scale = Projector2D::new(f32::NAN);
        let point = Point::new([1.0, 2.0]);
        assert!(!invalid_scale.is_visible(&point));
        assert!(
            !invalid_scale
                .project(&point)
                .iter()
                .all(|value| value.is_finite())
        );

        let overflow = Projector3D::new(f32::MAX);
        let huge_point = Point::new([f64::MAX, 0.0, 0.0]);
        assert!(!overflow.is_visible(&huge_point));
    }

    #[test]
    fn legacy_four_d_projector_rejects_invalid_slice_and_nonfinite_points() {
        let invalid = Projector4D::new(0.0, 0.0, 1.0);
        let point = Point::new([1.0, 2.0, 3.0, 0.0]);
        assert!(!invalid.configuration_is_valid());
        assert!(!invalid.is_visible(&point));
        assert_eq!(invalid.alpha(&point), 0.0);

        let valid = Projector4D::new(0.0, 1.0, 1.0);
        let nonfinite_w = Point::new([1.0, 2.0, 3.0, f64::NAN]);
        let nonfinite_xyz = Point::new([f64::NAN, 2.0, 3.0, 0.0]);
        assert!(!valid.is_visible(&nonfinite_w));
        assert_eq!(valid.alpha(&nonfinite_w), 0.0);
        assert!(!valid.is_visible(&nonfinite_xyz));
        assert_eq!(valid.alpha(&nonfinite_xyz), 0.0);
    }

    #[test]
    fn slice_plane_correct() {
        let proj = Projector4D::new(3.0, 1.0, 1.0);
        let plane = proj.slice_plane();
        // Point at w=3.0 should be on the plane
        let on = Point::new([0.0, 0.0, 0.0, 3.0]);
        assert!(plane.signed_distance(&on).abs() < 1e-10);
    }
}
