// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
use crate::point::Point;
use nalgebra::SVector;

/// A finite, conservative axis-aligned enclosure in a shape's local frame.
///
/// A LocalAabb is a geometric enclosure, not a transformed/world-space
/// collision certificate. The endpoints are immutable after construction.
/// Providers must return None whenever their shape data is invalid,
/// unbounded, or not covered by a conservative formula.
///
/// Sphere and capsule constructors widen their single-operation endpoint
/// arithmetic by one representable binary64 value in each outward direction.
/// This relies on ordinary IEEE-754 round-to-nearest binary64 operations.
/// Shapes whose local bounds require unsupported arithmetic must return None.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LocalAabb<const D: usize> {
    min: SVector<f64, D>,
    max: SVector<f64, D>,
}

impl<const D: usize> LocalAabb<D> {
    /// Construct a local AABB from already-conservative finite endpoints.
    ///
    /// Callers are responsible for ensuring that the supplied endpoints
    /// enclose their shape. This constructor validates representation only.
    pub fn from_conservative_endpoints(min: SVector<f64, D>, max: SVector<f64, D>) -> Option<Self> {
        if min.iter().any(|value| !value.is_finite())
            || max.iter().any(|value| !value.is_finite())
            || (0..D).any(|axis| min[axis] > max[axis])
        {
            return None;
        }
        Some(Self { min, max })
    }

    /// Construct the exact represented local bounds of an origin-centred
    /// hyperbox with finite non-negative half-extents.
    pub fn from_half_extents(half_extents: [f64; D]) -> Option<Self> {
        if half_extents
            .iter()
            .any(|extent| !extent.is_finite() || *extent < 0.0)
        {
            return None;
        }

        let min = SVector::from_fn(|axis, _| -half_extents[axis]);
        let max = SVector::from_fn(|axis, _| half_extents[axis]);
        Some(Self { min, max })
    }

    /// Construct an outward local enclosure of a represented sphere.
    ///
    /// Each endpoint is one correctly rounded subtraction/addition followed
    /// by one representable step outward. Overflow, invalid radius, or
    /// non-finite center values return None.
    pub fn from_center_radius(center: &Point<D>, radius: f64) -> Option<Self> {
        if !radius.is_finite() || radius < 0.0 || center.0.iter().any(|value| !value.is_finite()) {
            return None;
        }

        let mut min = SVector::<f64, D>::zeros();
        let mut max = SVector::<f64, D>::zeros();
        for axis in 0..D {
            let lower = center.0[axis] - radius;
            let upper = center.0[axis] + radius;
            if !lower.is_finite() || !upper.is_finite() {
                return None;
            }
            min[axis] = next_down(lower);
            max[axis] = next_up(upper);
            if !min[axis].is_finite() || !max[axis].is_finite() {
                return None;
            }
        }

        Some(Self { min, max })
    }

    /// Construct the exact coordinate-wise enclosure of represented points.
    ///
    /// No arithmetic is performed beyond comparisons, so the extremal input
    /// coordinates themselves are conservative endpoints for their convex
    /// hull. Empty or non-finite point sets return None.
    pub fn from_points(points: &[SVector<f64, D>]) -> Option<Self> {
        let first = *points.first()?;
        if first.iter().any(|value| !value.is_finite()) {
            return None;
        }

        let mut min = first;
        let mut max = first;
        for point in &points[1..] {
            if point.iter().any(|value| !value.is_finite()) {
                return None;
            }
            for axis in 0..D {
                min[axis] = min[axis].min(point[axis]);
                max[axis] = max[axis].max(point[axis]);
            }
        }

        Some(Self { min, max })
    }

    /// Construct a one-operation outward enclosure from finite ordered
    /// rounded endpoints. Intended for simple built-in analytic formulas.
    pub(crate) fn from_rounded_endpoints_outward(
        rounded_min: SVector<f64, D>,
        rounded_max: SVector<f64, D>,
    ) -> Option<Self> {
        if rounded_min.iter().any(|value| !value.is_finite())
            || rounded_max.iter().any(|value| !value.is_finite())
            || (0..D).any(|axis| rounded_min[axis] > rounded_max[axis])
        {
            return None;
        }

        let min = rounded_min.map(next_down);
        let max = rounded_max.map(next_up);
        if min.iter().any(|value| !value.is_finite()) || max.iter().any(|value| !value.is_finite()) {
            return None;
        }

        Some(Self { min, max })
    }

    /// Inclusive local-space lower endpoint.
    #[inline]
    pub fn min(&self) -> &SVector<f64, D> {
        &self.min
    }

    /// Inclusive local-space upper endpoint.
    #[inline]
    pub fn max(&self) -> &SVector<f64, D> {
        &self.max
    }

    /// Whether this enclosure contains the supplied finite local point.
    #[inline]
    pub fn contains(&self, point: &SVector<f64, D>) -> bool {
        point.iter().all(|value| value.is_finite())
            && (0..D).all(|axis| self.min[axis] <= point[axis] && point[axis] <= self.max[axis])
    }
}

#[inline]
fn next_up(value: f64) -> f64 {
    if value.is_nan() || value == f64::INFINITY {
        return value;
    }
    if value == 0.0 {
        return f64::from_bits(1);
    }
    let bits = value.to_bits();
    if value > 0.0 {
        f64::from_bits(bits + 1)
    } else {
        f64::from_bits(bits - 1)
    }
}

#[inline]
fn next_down(value: f64) -> f64 {
    if value.is_nan() || value == f64::NEG_INFINITY {
        return value;
    }
    if value == 0.0 {
        return f64::from_bits((1_u64 << 63) | 1);
    }
    let bits = value.to_bits();
    if value > 0.0 {
        f64::from_bits(bits - 1)
    } else {
        f64::from_bits(bits + 1)
    }
}

/// Trait for D-dimensional convex shapes compatible with GJK collision detection.
///
/// The support function is the only requirement for GJK — given a direction,
/// return the point on the shape's boundary furthest in that direction.
/// This generalizes naturally to any dimension.
pub trait Shape<const D: usize>: Send + Sync {
    /// Support function: furthest point on the boundary in the given direction.
    fn support(&self, direction: &SVector<f64, D>) -> SVector<f64, D>;

    /// Bounding sphere: (center, radius).
    ///
    /// This is a geometric convenience API. It is not, by itself, a certified
    /// numerical enclosure suitable for broadphase pruning.
    fn bounding_sphere(&self) -> (Point<D>, f64);

    /// Optional conservative axis-aligned bounds in the shape's local frame.
    ///
    /// Some(bounds) promises that the finite represented shape is contained
    /// by the returned LocalAabb. None means unsupported, unbounded, invalid,
    /// or otherwise not qualified for geometric pruning. Consumers must fail
    /// open for None; they must not substitute bounding_sphere() or infer
    /// separation.
    ///
    /// This method is additive and defaults to None so existing third-party
    /// shapes remain source-compatible and unprunable until they explicitly
    /// provide a conservative local enclosure.
    fn local_aabb(&self) -> Option<LocalAabb<D>> {
        None
    }

    /// Downcast to concrete type for specialized collision dispatch.
    fn as_any(&self) -> &dyn std::any::Any;

    /// Clone this shape into a new box.
    fn clone_box(&self) -> Box<dyn Shape<D>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Capsule, ConvexHull, HalfSpace, HyperBox, Sphere};

    #[test]
    fn half_extents_are_exact_and_invalid_extents_are_unavailable() {
        let bounds = LocalAabb::<3>::from_half_extents([1.0, 2.0, 0.0]).unwrap();
        assert_eq!(*bounds.min(), SVector::from([-1.0, -2.0, 0.0]));
        assert_eq!(*bounds.max(), SVector::from([1.0, 2.0, 0.0]));
        assert!(LocalAabb::<2>::from_half_extents([1.0, f64::NAN]).is_none());
        assert!(LocalAabb::<2>::from_half_extents([1.0, -0.1]).is_none());
    }

    #[test]
    fn center_radius_bounds_expand_outward_and_reject_overflow() {
        let center = Point::new([1.0, -2.0]);
        let bounds = LocalAabb::from_center_radius(&center, 0.5).unwrap();
        assert!(bounds.min()[0] < 0.5);
        assert!(bounds.max()[0] > 1.5);
        assert!(bounds.min()[1] < -2.5);
        assert!(bounds.max()[1] > -1.5);
        assert!(LocalAabb::from_center_radius(&Point::<2>::origin(), f64::INFINITY).is_none());
        assert!(LocalAabb::from_center_radius(&Point::new([f64::MAX]), f64::MAX).is_none());
    }

    #[test]
    fn point_bounds_are_exact_for_finite_vertices() {
        let points = [
            SVector::from([1.0, 4.0]),
            SVector::from([-2.0, 3.0]),
            SVector::from([0.5, -1.0]),
        ];
        let bounds = LocalAabb::from_points(&points).unwrap();
        assert_eq!(*bounds.min(), SVector::from([-2.0, -1.0]));
        assert_eq!(*bounds.max(), SVector::from([1.0, 4.0]));
        assert!(LocalAabb::<2>::from_points(&[]).is_none());
    }

    #[test]
    fn built_in_shape_local_bound_providers_enclose_support_points() {
        let sphere = Sphere::new(Point::new([1.0, 2.0, 3.0]), 0.5);
        let sphere_bounds = sphere.local_aabb().unwrap();

        let capsule = Capsule::<3>::y_aligned(2.0, 0.5);
        let capsule_bounds = capsule.local_aabb().unwrap();

        let hyperbox = HyperBox::<3>::new([1.0, 2.0, 3.0]);
        let hyperbox_bounds = hyperbox.local_aabb().unwrap();
        assert_eq!(
            hyperbox_bounds,
            LocalAabb::from_half_extents([1.0, 2.0, 3.0]).unwrap()
        );

        let directions = [
            SVector::from([1.0, 0.0, 0.0]),
            SVector::from([-1.0, 0.0, 0.0]),
            SVector::from([0.0, 1.0, 0.0]),
            SVector::from([0.0, -1.0, 0.0]),
            SVector::from([0.0, 0.0, 1.0]),
            SVector::from([0.0, 0.0, -1.0]),
        ];
        for direction in directions {
            assert!(sphere_bounds.contains(&sphere.support(&direction)));
            assert!(capsule_bounds.contains(&capsule.support(&direction)));
            assert!(hyperbox_bounds.contains(&hyperbox.support(&direction)));
        }

        let hull = ConvexHull::<2>::new(vec![
            SVector::from([-1.0, 2.0]),
            SVector::from([3.0, -4.0]),
            SVector::from([0.0, 1.0]),
        ]);
        let hull_bounds = hull.local_aabb().unwrap();
        assert_eq!(hull_bounds, LocalAabb::from_points(&hull.vertices).unwrap());
        assert!(
            hull.vertices
                .iter()
                .all(|vertex| hull_bounds.contains(vertex))
        );

        let plane = HalfSpace::<3>::ground(1, 0.0);
        assert!(plane.local_aabb().is_none());
    }

    #[test]
    fn invalid_mutable_builtin_shape_data_has_no_local_bound() {
        let mut sphere = Sphere::<3>::unit();
        sphere.radius = -1.0;
        assert!(sphere.local_aabb().is_none());

        let capsule = Capsule::<3>::new(1.0, 0.5, 3);
        assert!(capsule.local_aabb().is_none());

        let hyperbox = HyperBox::<3>::new([1.0, f64::NAN, 2.0]);
        assert!(hyperbox.local_aabb().is_none());

        let hull = ConvexHull::<2>::new(vec![
            SVector::from([0.0, 0.0]),
            SVector::from([f64::INFINITY, 1.0]),
        ]);
        assert!(hull.local_aabb().is_none());
    }
}
