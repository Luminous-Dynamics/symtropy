// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! World-space support maps for transformed convex collision queries.
//!
//! `symtropy-math::Shape` deliberately stores geometry in local coordinates.
//! Collision algorithms, however, must query the geometry after the body's full
//! rigid transform has been applied.  This module is the single conversion
//! boundary between those two coordinate systems.

use nalgebra::SVector;
use symtropy_math::{Point, Shape, Transform};

/// Convex support map expressed in world coordinates.
///
/// Keeping GJK, EPA and manifold generation in terms of this small interface
/// prevents individual algorithms from accidentally applying translation but
/// forgetting orientation.
pub trait WorldSupportMap<const D: usize> {
    /// Furthest world-space point in `direction`.
    fn support_world(&self, direction: &SVector<f64, D>) -> SVector<f64, D>;

    /// Representative world-space center used to seed iterative queries.
    fn center_world(&self) -> SVector<f64, D>;

    /// Enclosing world-space sphere. Rigid rotations preserve the radius.
    fn bounding_sphere_world(&self) -> (SVector<f64, D>, f64);
}

/// Borrowed local shape paired with its complete world transform.
#[derive(Clone, Copy)]
pub struct TransformedShape<'a, const D: usize> {
    pub shape: &'a dyn Shape<D>,
    pub transform: &'a Transform<D>,
}

impl<'a, const D: usize> TransformedShape<'a, D> {
    #[inline]
    pub fn new(shape: &'a dyn Shape<D>, transform: &'a Transform<D>) -> Self {
        Self { shape, transform }
    }
}

impl<const D: usize> WorldSupportMap<D> for TransformedShape<'_, D> {
    #[inline]
    fn support_world(&self, direction: &SVector<f64, D>) -> SVector<f64, D> {
        // For an orthogonal transform R, maximizing d·(Rx+t) is equivalent to
        // maximizing (R^T d)·x in local space.
        let local_direction = self.transform.rotation.reverse().rotate_vector(direction);
        let local_support = self.shape.support(&local_direction);
        self.transform.transform_point(&Point(local_support)).0
    }

    #[inline]
    fn center_world(&self) -> SVector<f64, D> {
        let (local_center, _) = self.shape.bounding_sphere();
        self.transform.transform_point(&local_center).0
    }

    #[inline]
    fn bounding_sphere_world(&self) -> (SVector<f64, D>, f64) {
        let (local_center, radius) = self.shape.bounding_sphere();
        (self.transform.transform_point(&local_center).0, radius)
    }
}

/// Tight axis-aligned bounds obtained from support queries along every world
/// axis. This is exact for any bounded convex `Shape`, including rotated boxes,
/// capsules, hulls and compound shapes.
pub fn support_aabb<const D: usize>(
    map: &dyn WorldSupportMap<D>,
) -> (SVector<f64, D>, SVector<f64, D>) {
    let mut min = SVector::<f64, D>::zeros();
    let mut max = SVector::<f64, D>::zeros();

    for axis in 0..D {
        let mut direction = SVector::<f64, D>::zeros();
        direction[axis] = 1.0;
        max[axis] = map.support_world(&direction)[axis];
        min[axis] = map.support_world(&(-direction))[axis];
    }

    (min, max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::FRAC_PI_2;
    use symtropy_math::{Bivector, HyperBox, Rotor};

    #[test]
    fn rotated_box_support_uses_world_orientation() {
        let shape = HyperBox::<3>::new([2.0, 0.5, 0.25]);
        let transform = Transform {
            translation: Point::new([3.0, 0.0, 0.0]),
            rotation: Rotor::from_plane_angle(&Bivector::unit_plane(0, 1), FRAC_PI_2),
        };
        let map = TransformedShape::new(&shape, &transform);
        let x = SVector::from([1.0, 0.0, 0.0]);
        let y = SVector::from([0.0, 1.0, 0.0]);

        let sx = map.support_world(&x);
        let sy = map.support_world(&y);
        assert!((sx[0] - 3.5).abs() < 1e-10, "rotated x extent = {}", sx[0]);
        assert!((sy[1] - 2.0).abs() < 1e-10, "rotated y extent = {}", sy[1]);
    }

    #[test]
    fn support_aabb_tracks_rotated_extents() {
        let shape = HyperBox::<2>::new([2.0, 0.5]);
        let transform = Transform {
            translation: Point::new([1.0, -2.0]),
            rotation: Rotor::from_plane_angle(
                &Bivector::unit_plane(0, 1),
                std::f64::consts::FRAC_PI_2,
            ),
        };
        let map = TransformedShape::new(&shape, &transform);
        let (min, max) = support_aabb(&map);

        assert!((min[0] - 0.5).abs() < 1e-10);
        assert!((max[0] - 1.5).abs() < 1e-10);
        assert!((min[1] + 4.0).abs() < 1e-10);
        assert!((max[1] - 0.0).abs() < 1e-10);
    }
}
