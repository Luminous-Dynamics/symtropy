// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Experimental periodic convex queries and sphere-sphere CCD.
//!
//! This enumerates conservative image candidates, then invokes existing GJK on
//! every candidate. It is restricted to D=2 and D=3 while depending on legacy
//! GJK. An iteration cap is indeterminate, never silently separation.
//! This is a query layer, not a PhysicsWorld/contact-solver integration.

use std::error::Error;
use std::fmt;

use nalgebra::SVector;
use symtropy_math::{LatticeOffset, PeriodicBox, PeriodicDomainError, Shape, Transform};

use crate::broadphase::{support_aabb, Aabb};
use crate::ccd;
use crate::gjk;
use crate::periodic_broadphase::{
    image_offsets_for_aabb_pair, swept_aabb_linear, translate_aabb, PeriodicBroadphaseError,
};
use crate::support_map::TransformedShape;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PeriodicCollisionError {
    UnsupportedDimension { dimension: usize },
    Broadphase(PeriodicBroadphaseError),
    InvalidMotion { field: &'static str },
    NonFiniteGeometry { field: &'static str },
    NumericRange { field: &'static str },
}

impl fmt::Display for PeriodicCollisionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedDimension { dimension } => write!(
                f,
                "periodic GJK query currently supports D=2 or D=3, got D={dimension}"
            ),
            Self::Broadphase(error) => write!(f, "periodic broadphase error: {error}"),
            Self::InvalidMotion { field } => write!(f, "invalid sphere CCD input: {field}"),
            Self::NonFiniteGeometry { field } => {
                write!(f, "non-finite periodic collision geometry: {field}")
            }
            Self::NumericRange { field } => {
                write!(f, "periodic collision numeric range failure: {field}")
            }
        }
    }
}

impl Error for PeriodicCollisionError {}

impl From<PeriodicBroadphaseError> for PeriodicCollisionError {
    fn from(value: PeriodicBroadphaseError) -> Self {
        Self::Broadphase(value)
    }
}

impl From<PeriodicDomainError> for PeriodicCollisionError {
    fn from(value: PeriodicDomainError) -> Self {
        Self::Broadphase(PeriodicBroadphaseError::from(value))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeriodicCollisionStatus {
    /// All enumerated candidates were separated; enumeration was complete
    /// within the configured numeric and image-budget envelope.
    Separated,
    /// At least one periodic image produced a definitive GJK intersection.
    Intersecting,
    /// No definitive hit was found, but at least one GJK query exhausted its
    /// iteration budget. This status is not a separation claim.
    Indeterminate,
}

/// A confirmed intersecting image. The offset translates B relative to A.
/// Different image offsets are retained as different geometric candidates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PeriodicCollisionHit<const D: usize> {
    pub image_offset: LatticeOffset<D>,
    pub gjk_iterations: usize,
}

/// Query result. If hits is non-empty, intersection is confirmed. If unresolved
/// offsets are also present, the complete contact-image set is not known.
#[derive(Clone, Debug)]
pub struct PeriodicCollisionQuery<const D: usize> {
    pub hits: Vec<PeriodicCollisionHit<D>>,
    pub unresolved_offsets: Vec<LatticeOffset<D>>,
    pub image_candidates: usize,
    pub gjk_calls: usize,
}

impl<const D: usize> PeriodicCollisionQuery<D> {
    pub fn status(&self) -> PeriodicCollisionStatus {
        if !self.hits.is_empty() {
            PeriodicCollisionStatus::Intersecting
        } else if !self.unresolved_offsets.is_empty() {
            PeriodicCollisionStatus::Indeterminate
        } else {
            PeriodicCollisionStatus::Separated
        }
    }

    pub fn contact_set_complete(&self) -> bool {
        self.unresolved_offsets.is_empty()
    }
}

/// Query transformed convex shapes over every AABB-overlapping image of B.
///
/// This is currently restricted to D=2/3 and is not integrated into the
/// physics world or contact cache. Existing GJK correctness limits still apply.
pub fn query_periodic_convex<const D: usize>(
    domain: &PeriodicBox<D>,
    shape_a: &dyn Shape<D>,
    transform_a: &Transform<D>,
    shape_b: &dyn Shape<D>,
    transform_b: &Transform<D>,
    max_image_candidates: usize,
) -> Result<PeriodicCollisionQuery<D>, PeriodicCollisionError> {
    if D != 2 && D != 3 {
        return Err(PeriodicCollisionError::UnsupportedDimension { dimension: D });
    }

    let map_a = TransformedShape::new(shape_a, transform_a);
    let map_b = TransformedShape::new(shape_b, transform_b);
    let (a_min, a_max) = support_aabb(&map_a);
    let (b_min, b_max) = support_aabb(&map_b);
    let bounds_a = Aabb { min: a_min, max: a_max };
    let bounds_b = Aabb { min: b_min, max: b_max };

    let offsets =
        image_offsets_for_aabb_pair(domain, &bounds_a, &bounds_b, max_image_candidates)?;
    let candidate_count = offsets.len();
    let mut result = PeriodicCollisionQuery {
        hits: Vec::new(),
        unresolved_offsets: Vec::new(),
        image_candidates: candidate_count,
        gjk_calls: 0,
    };

    for offset in offsets {
        let translated_bounds = translate_aabb(domain, &bounds_b, offset)?;
        // The image enumerator outward-rounds integer ranges to avoid
        // floating-point under-enumeration. That can include a harmless
        // near-boundary false positive; disjoint AABBs are themselves a
        // definitive separation and do not require a GJK call.
        if !bounds_a.overlaps(&translated_bounds) {
            continue;
        }

        let shift = domain.offset_vector(offset)?;
        let mut image_transform_b = transform_b.clone();
        image_transform_b.translation.0 += shift;
        if !image_transform_b
            .translation
            .0
            .iter()
            .all(|value| value.is_finite())
        {
            return Err(PeriodicCollisionError::NumericRange {
                field: "translated transform",
            });
        }

        let collision = gjk::intersects_transformed(
            shape_a,
            transform_a,
            shape_b,
            &image_transform_b,
        );
        result.gjk_calls += 1;

        if collision.intersecting {
            result.hits.push(PeriodicCollisionHit {
                image_offset: offset,
                gjk_iterations: collision.iterations,
            });
        } else if collision.iterations >= gjk::MAX_ITERATIONS {
            result.unresolved_offsets.push(offset);
        }
    }

    Ok(result)
}

/// Earliest periodic sphere-sphere TOI for linear translation and fixed radii.
/// toi_seconds follows the legacy analytic CCD function's actual convention;
/// toi_fraction is separately normalized to [0,1].
#[derive(Clone, Debug)]
pub struct PeriodicSphereCcdHit<const D: usize> {
    pub image_offset: LatticeOffset<D>,
    pub toi_seconds: f64,
    pub toi_fraction: f64,
    pub point: SVector<f64, D>,
    pub normal: SVector<f64, D>,
    pub images_tested: usize,
}

/// Sphere-sphere CCD across periodic images.
///
/// Swept AABBs enumerate every image that could collide over the interval. The
/// earliest hit is returned; exact-time ties choose the lexicographically
/// smaller image offset. Only one first-hit result is produced for a body pair.
/// Enumerate all periodic image sphere-sphere hits during a linear sweep.
///
/// Each image's analytic CCD result is returned separately, sorted by TOI and
/// then lexicographically by image offset. This permits event-driven callers
/// to suppress a just-resolved zero-time image while still considering a later
/// collision with another periodic image of the same body pair.
pub fn sphere_sphere_ccd_periodic_all<const D: usize>(
    domain: &PeriodicBox<D>,
    position_a: &SVector<f64, D>,
    velocity_a: &SVector<f64, D>,
    radius_a: f64,
    position_b: &SVector<f64, D>,
    velocity_b: &SVector<f64, D>,
    radius_b: f64,
    dt: f64,
    max_image_candidates: usize,
) -> Result<Vec<PeriodicSphereCcdHit<D>>, PeriodicCollisionError> {
    if D == 0 {
        return Err(PeriodicCollisionError::UnsupportedDimension { dimension: D });
    }
    if !dt.is_finite() || dt <= 0.0 {
        return Err(PeriodicCollisionError::InvalidMotion { field: "dt" });
    }
    if !radius_a.is_finite() || radius_a <= 0.0 {
        return Err(PeriodicCollisionError::InvalidMotion { field: "radius_a" });
    }
    if !radius_b.is_finite() || radius_b <= 0.0 {
        return Err(PeriodicCollisionError::InvalidMotion { field: "radius_b" });
    }

    for axis in 0..D {
        for (field, value) in [
            ("position_a", position_a[axis]),
            ("velocity_a", velocity_a[axis]),
            ("position_b", position_b[axis]),
            ("velocity_b", velocity_b[axis]),
        ] {
            if !value.is_finite() {
                return Err(PeriodicCollisionError::NonFiniteGeometry { field });
            }
        }
    }

    let end_a = position_a + velocity_a * dt;
    let end_b = position_b + velocity_b * dt;
    if !end_a.iter().all(|value| value.is_finite())
        || !end_b.iter().all(|value| value.is_finite())
    {
        return Err(PeriodicCollisionError::NumericRange {
            field: "swept sphere endpoint",
        });
    }

    let a_start = Aabb::from_sphere(position_a, radius_a);
    let a_end = Aabb::from_sphere(&end_a, radius_a);
    let b_start = Aabb::from_sphere(position_b, radius_b);
    let b_end = Aabb::from_sphere(&end_b, radius_b);
    let swept_a = swept_aabb_linear(&a_start, &a_end);
    let swept_b = swept_aabb_linear(&b_start, &b_end);
    let offsets =
        image_offsets_for_aabb_pair(domain, &swept_a, &swept_b, max_image_candidates)?;
    let images_tested = offsets.len();
    let mut hits = Vec::new();

    for offset in offsets {
        let shift = domain.offset_vector(offset)?;
        let image_position_b = position_b + shift;
        if !image_position_b.iter().all(|value| value.is_finite()) {
            return Err(PeriodicCollisionError::NumericRange {
                field: "translated sphere position",
            });
        }

        if let Some(hit) = ccd::sphere_sphere(
            position_a,
            velocity_a,
            radius_a,
            &image_position_b,
            velocity_b,
            radius_b,
            dt,
        ) {
            let toi = hit.toi;
            if !toi.is_finite() || !(0.0..=dt).contains(&toi) {
                return Err(PeriodicCollisionError::NumericRange {
                    field: "sphere CCD time of impact",
                });
            }

            hits.push(PeriodicSphereCcdHit {
                image_offset: offset,
                toi_seconds: toi,
                toi_fraction: (toi / dt).clamp(0.0, 1.0),
                point: hit.point,
                normal: hit.normal,
                images_tested,
            });
        }
    }

    hits.sort_by(|a, b| {
        a.toi_seconds
            .total_cmp(&b.toi_seconds)
            .then_with(|| a.image_offset.periods.cmp(&b.image_offset.periods))
    });
    Ok(hits)
}

/// First periodic sphere-sphere time of impact found by complete bounded image
/// enumeration. This is a convenience wrapper over the all-image query.
pub fn sphere_sphere_ccd_periodic<const D: usize>(
    domain: &PeriodicBox<D>,
    position_a: &SVector<f64, D>,
    velocity_a: &SVector<f64, D>,
    radius_a: f64,
    position_b: &SVector<f64, D>,
    velocity_b: &SVector<f64, D>,
    radius_b: f64,
    dt: f64,
    max_image_candidates: usize,
) -> Result<Option<PeriodicSphereCcdHit<D>>, PeriodicCollisionError> {
    let mut hits = sphere_sphere_ccd_periodic_all(
        domain,
        position_a,
        velocity_a,
        radius_a,
        position_b,
        velocity_b,
        radius_b,
        dt,
        max_image_candidates,
    )?;
    Ok(if hits.is_empty() {
        None
    } else {
        Some(hits.remove(0))
    })
}

#[cfg(test)]
mod tests {
    use super::{
        PeriodicCollisionStatus, query_periodic_convex, sphere_sphere_ccd_periodic,
        sphere_sphere_ccd_periodic_all,
    };
    use nalgebra::SVector;
    use symtropy_math::{PeriodicBox, Point, Sphere, Transform};

    fn x_periodic_domain() -> PeriodicBox<2> {
        PeriodicBox::new(
            SVector::zeros(),
            SVector::from([10.0, 0.0]),
            [true, false],
        )
        .unwrap()
    }

    #[test]
    fn convex_query_finds_collision_across_periodic_seam() {
        let domain = x_periodic_domain();
        let sphere_a = Sphere::new(Point::origin(), 0.5);
        let sphere_b = Sphere::new(Point::origin(), 0.5);
        let transform_a = Transform::from_translation(Point::new([0.2, 0.0]));
        let transform_b = Transform::from_translation(Point::new([9.6, 0.0]));

        let result = query_periodic_convex(
            &domain, &sphere_a, &transform_a, &sphere_b, &transform_b, 8,
        )
        .unwrap();

        assert_eq!(result.status(), PeriodicCollisionStatus::Intersecting);
        assert_eq!(result.hits.len(), 1);
        assert_eq!(result.hits[0].image_offset.periods, [-1, 0]);
        assert!(result.contact_set_complete());
    }

    #[test]
    fn outward_rounded_aabb_false_positive_is_a_separation_not_error() {
        let domain = x_periodic_domain();
        let sphere_a = Sphere::new(Point::origin(), 0.5);
        let sphere_b = Sphere::new(Point::origin(), 0.5);
        let transform_a = Transform::from_translation(Point::new([0.5, 0.0]));
        let transform_b = Transform::from_translation(Point::new([11.50000000000001, 0.0]));

        let result = query_periodic_convex(
            &domain, &sphere_a, &transform_a, &sphere_b, &transform_b, 8,
        )
        .unwrap();

        assert_eq!(result.status(), PeriodicCollisionStatus::Separated);
        assert_eq!(result.image_candidates, 1);
        assert_eq!(result.gjk_calls, 0);
        assert!(result.hits.is_empty());
    }

    #[test]
    fn convex_query_does_not_wrap_non_periodic_axis() {
        let domain = x_periodic_domain();
        let sphere_a = Sphere::new(Point::origin(), 0.5);
        let sphere_b = Sphere::new(Point::origin(), 0.5);
        let transform_a = Transform::from_translation(Point::new([0.2, 0.0]));
        let transform_b = Transform::from_translation(Point::new([9.6, 2.0]));

        let result = query_periodic_convex(
            &domain, &sphere_a, &transform_a, &sphere_b, &transform_b, 8,
        )
        .unwrap();

        assert_eq!(result.status(), PeriodicCollisionStatus::Separated);
        assert!(result.hits.is_empty());
        assert!(result.unresolved_offsets.is_empty());
    }

    #[test]
    fn sphere_ccd_detects_crossing_of_periodic_seam() {
        let domain = x_periodic_domain();
        let hit = sphere_sphere_ccd_periodic(
            &domain,
            &SVector::from([0.5, 0.0]),
            &SVector::zeros(),
            0.5,
            &SVector::from([9.0, 0.0]),
            &SVector::from([2.0, 0.0]),
            0.5,
            1.0,
            8,
        )
        .unwrap()
        .unwrap();

        assert_eq!(hit.image_offset.periods, [-1, 0]);
        assert!((hit.toi_seconds - 0.25).abs() < 1e-12);
        assert!((hit.toi_fraction - 0.25).abs() < 1e-12);
        assert!(hit.images_tested >= 1);
    }

    #[test]
    fn sphere_ccd_all_images_keeps_later_image_hits() {
        let domain = x_periodic_domain();
        let hits = sphere_sphere_ccd_periodic_all(
            &domain,
            &SVector::from([5.0, 0.0]),
            &SVector::zeros(),
            0.2,
            &SVector::from([0.0, 0.0]),
            &SVector::from([30.0, 0.0]),
            0.2,
            1.0,
            8,
        )
        .unwrap();
        assert!(hits.len() >= 2);
        assert!(hits.windows(2).all(|pair| pair[0].toi_seconds <= pair[1].toi_seconds));
        assert_ne!(hits[0].image_offset, hits[1].image_offset);
    }

    #[test]
    fn sphere_ccd_returns_none_for_separated_sweeps() {
        let domain = x_periodic_domain();
        let hit = sphere_sphere_ccd_periodic(
            &domain,
            &SVector::from([1.0, 0.0]),
            &SVector::zeros(),
            0.5,
            &SVector::from([5.0, 0.0]),
            &SVector::zeros(),
            0.5,
            1.0,
            8,
        )
        .unwrap();
        assert!(hit.is_none());
    }

    #[test]
    fn sphere_ccd_budget_exhaustion_is_an_error_not_a_miss() {
        let domain = PeriodicBox::new(
            SVector::zeros(),
            SVector::from([10.0, 10.0]),
            [true, true],
        )
        .unwrap();
        let result = sphere_sphere_ccd_periodic(
            &domain,
            &SVector::zeros(),
            &SVector::zeros(),
            25.0,
            &SVector::zeros(),
            &SVector::zeros(),
            25.0,
            1.0,
            4,
        );
        assert!(result.is_err());
    }
}
