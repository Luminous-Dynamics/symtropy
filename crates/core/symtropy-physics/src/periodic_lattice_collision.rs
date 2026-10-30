// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Experimental triclinic convex collision queries and sphere-sphere CCD.
//!
//! The query layer enumerates conservative skew-lattice image candidates, then
//! runs the existing Euclidean narrow phase on each image. It is not wired into
//! PhysicsWorld, ContactCache, constraints, or impulse response.
//!
//! GJK currently reports iteration count but no typed termination field.
//! Therefore the adapter treats exactly 64 iterations as unresolved, matching
//! the private cap in gjk.rs. Replace this compatibility check if GJK exposes a
//! public termination enum.

use std::error::Error;
use std::fmt;

use nalgebra::SVector;
use symtropy_math::{LatticeOffset, PeriodicLattice, PeriodicLatticeError, Shape, Transform};

use crate::broadphase::{support_aabb, Aabb};
use crate::ccd;
use crate::gjk;
use crate::periodic_lattice_broadphase::{
    PeriodicLatticeBroadphaseError, lattice_image_offsets_for_aabb_pair,
};
use crate::support_map::TransformedShape;

const LEGACY_GJK_ITERATION_LIMIT: usize = 64;

/// Failure reason for a triclinic periodic collision query.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PeriodicLatticeCollisionError {
    UnsupportedDimension { dimension: usize },
    Broadphase(PeriodicLatticeBroadphaseError),
    InvalidMotion { field: &'static str },
    NonFiniteGeometry { field: &'static str },
    NumericRange { field: &'static str },
}

impl fmt::Display for PeriodicLatticeCollisionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedDimension { dimension } => write!(
                f,
                "triclinic convex collision currently supports D=2 or D=3, got D={dimension}"
            ),
            Self::Broadphase(error) => write!(f, "triclinic periodic broadphase error: {error}"),
            Self::InvalidMotion { field } => write!(f, "invalid triclinic sphere CCD input: {field}"),
            Self::NonFiniteGeometry { field } => {
                write!(f, "non-finite triclinic collision geometry: {field}")
            }
            Self::NumericRange { field } => {
                write!(f, "triclinic collision numeric-range failure in {field}")
            }
        }
    }
}

impl Error for PeriodicLatticeCollisionError {}

impl From<PeriodicLatticeBroadphaseError> for PeriodicLatticeCollisionError {
    fn from(value: PeriodicLatticeBroadphaseError) -> Self {
        Self::Broadphase(value)
    }
}

impl From<PeriodicLatticeError> for PeriodicLatticeCollisionError {
    fn from(value: PeriodicLatticeError) -> Self {
        Self::Broadphase(PeriodicLatticeBroadphaseError::from(value))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeriodicLatticeCollisionStatus {
    /// Every conservative image candidate was definitively separated by GJK.
    Separated,
    /// At least one lattice image produced a definitive GJK intersection.
    Intersecting,
    /// No definitive hit was found, but at least one GJK call exhausted its
    /// iteration cap. This is not a separation claim.
    Indeterminate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PeriodicLatticeCollisionHit<const D: usize> {
    /// Lattice image translation applied to body B relative to body A.
    pub image_offset: LatticeOffset<D>,
    pub gjk_iterations: usize,
}

#[derive(Clone, Debug)]
pub struct PeriodicLatticeSphereCcdHit<const D: usize> {
    pub image_offset: LatticeOffset<D>,
    pub toi_seconds: f64,
    pub toi_fraction: f64,
    pub point: SVector<f64, D>,
    pub normal: SVector<f64, D>,
    pub images_tested: usize,
}

#[derive(Clone, Debug)]
pub struct PeriodicLatticeCollisionQuery<const D: usize> {
    pub hits: Vec<PeriodicLatticeCollisionHit<D>>,
    pub unresolved_offsets: Vec<LatticeOffset<D>>,
    /// Number of conservative image offsets returned by the broadphase.
    pub image_candidates: usize,
    /// Number of GJK calls made for conservative image candidates.
    pub gjk_calls: usize,
}

impl<const D: usize> PeriodicLatticeCollisionQuery<D> {
    pub fn status(&self) -> PeriodicLatticeCollisionStatus {
        if !self.hits.is_empty() {
            PeriodicLatticeCollisionStatus::Intersecting
        } else if !self.unresolved_offsets.is_empty() {
            PeriodicLatticeCollisionStatus::Indeterminate
        } else {
            PeriodicLatticeCollisionStatus::Separated
        }
    }

    pub fn contact_set_complete(&self) -> bool {
        self.unresolved_offsets.is_empty()
    }
}

/// Query two transformed convex shapes over the conservative image superset
/// returned by the triclinic reciprocal-projection AABB broadphase.
///
/// Extra broadphase candidates are rejected by the Cartesian AABB overlap test
/// before GJK. Every remaining candidate is sent through transformed GJK.
/// Iteration exhaustion is retained as unresolved.
pub fn query_triclinic_convex<const D: usize>(
    lattice: &PeriodicLattice<D>,
    shape_a: &dyn Shape<D>,
    transform_a: &Transform<D>,
    shape_b: &dyn Shape<D>,
    transform_b: &Transform<D>,
    max_image_candidates: usize,
) -> Result<PeriodicLatticeCollisionQuery<D>, PeriodicLatticeCollisionError> {
    if D != 2 && D != 3 {
        return Err(PeriodicLatticeCollisionError::UnsupportedDimension { dimension: D });
    }

    let map_a = TransformedShape::new(shape_a, transform_a);
    let map_b = TransformedShape::new(shape_b, transform_b);
    let (a_min, a_max) = support_aabb(&map_a);
    let (b_min, b_max) = support_aabb(&map_b);
    let bounds_a = Aabb { min: a_min, max: a_max };
    let bounds_b = Aabb { min: b_min, max: b_max };

    let offsets = lattice_image_offsets_for_aabb_pair(
        lattice,
        &bounds_a,
        &bounds_b,
        max_image_candidates,
    )?;
    let image_candidates = offsets.len();
    let mut result = PeriodicLatticeCollisionQuery {
        hits: Vec::new(),
        unresolved_offsets: Vec::new(),
        image_candidates,
        gjk_calls: 0,
    };

    for offset in offsets {
        let shift = lattice.offset_vector(offset)?;
        // The lattice broadphase deliberately returns a conservative
        // superset and performs no floating-point AABB post-filter. We run GJK
        // for every candidate so a near-touching image cannot be lost by a
        // second rounded translation/overlap comparison.
        let mut image_transform_b = transform_b.clone();
        image_transform_b.translation.0 += shift;
        if !image_transform_b
            .translation
            .0
            .iter()
            .all(|value| value.is_finite())
        {
            return Err(PeriodicLatticeCollisionError::NumericRange {
                field: "translated shape transform",
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
            result.hits.push(PeriodicLatticeCollisionHit {
                image_offset: offset,
                gjk_iterations: collision.iterations,
            });
        } else if collision.iterations >= LEGACY_GJK_ITERATION_LIMIT {
            result.unresolved_offsets.push(offset);
        }
    }

    Ok(result)
}

/// Enumerate every analytic sphere-sphere collision TOI over the conservative
/// triclinic image set for linear translation and fixed radii.
///
/// Swept AABBs use the union of both endpoint sphere bounds, which is
/// conservative for fixed-radius linear translation. Returned hits are sorted
/// by TOI then lattice offset. Rotating convex CCD is not supported here.
pub fn sphere_sphere_ccd_triclinic_all<const D: usize>(
    lattice: &PeriodicLattice<D>,
    position_a: &SVector<f64, D>,
    velocity_a: &SVector<f64, D>,
    radius_a: f64,
    position_b: &SVector<f64, D>,
    velocity_b: &SVector<f64, D>,
    radius_b: f64,
    dt: f64,
    max_image_candidates: usize,
) -> Result<Vec<PeriodicLatticeSphereCcdHit<D>>, PeriodicLatticeCollisionError> {
    if D != 2 && D != 3 {
        return Err(PeriodicLatticeCollisionError::UnsupportedDimension { dimension: D });
    }
    if !dt.is_finite() || dt <= 0.0 {
        return Err(PeriodicLatticeCollisionError::InvalidMotion { field: "dt" });
    }
    for (field, radius) in [("radius_a", radius_a), ("radius_b", radius_b)] {
        if !radius.is_finite() || radius <= 0.0 {
            return Err(PeriodicLatticeCollisionError::InvalidMotion { field });
        }
    }
    for axis in 0..D {
        for (field, value) in [
            ("position_a", position_a[axis]),
            ("velocity_a", velocity_a[axis]),
            ("position_b", position_b[axis]),
            ("velocity_b", velocity_b[axis]),
        ] {
            if !value.is_finite() {
                return Err(PeriodicLatticeCollisionError::NonFiniteGeometry { field });
            }
        }
    }

    let end_a = position_a + velocity_a * dt;
    let end_b = position_b + velocity_b * dt;
    if !end_a.iter().all(|value| value.is_finite())
        || !end_b.iter().all(|value| value.is_finite())
    {
        return Err(PeriodicLatticeCollisionError::NumericRange {
            field: "sphere swept endpoint",
        });
    }

    let a_start = Aabb::from_sphere(position_a, radius_a);
    let a_end = Aabb::from_sphere(&end_a, radius_a);
    let b_start = Aabb::from_sphere(position_b, radius_b);
    let b_end = Aabb::from_sphere(&end_b, radius_b);
    let swept_a = a_start.union(&a_end);
    let swept_b = b_start.union(&b_end);

    let offsets = lattice_image_offsets_for_aabb_pair(
        lattice,
        &swept_a,
        &swept_b,
        max_image_candidates,
    )?;
    let images_tested = offsets.len();
    let mut hits = Vec::new();

    for offset in offsets {
        let shift = lattice.offset_vector(offset)?;
        let image_position_b = position_b + shift;
        if !image_position_b.iter().all(|value| value.is_finite()) {
            return Err(PeriodicLatticeCollisionError::NumericRange {
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
            if !hit.toi.is_finite() || !(0.0..=dt).contains(&hit.toi) {
                return Err(PeriodicLatticeCollisionError::NumericRange {
                    field: "sphere time of impact",
                });
            }
            if !hit.point.iter().all(|value| value.is_finite())
                || !hit.normal.iter().all(|value| value.is_finite())
            {
                return Err(PeriodicLatticeCollisionError::NumericRange {
                    field: "sphere CCD contact geometry",
                });
            }

            hits.push(PeriodicLatticeSphereCcdHit {
                image_offset: offset,
                toi_seconds: hit.toi,
                toi_fraction: (hit.toi / dt).clamp(0.0, 1.0),
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

#[cfg(test)]
mod tests {
    use super::{
        PeriodicLatticeCollisionStatus, query_triclinic_convex,
        sphere_sphere_ccd_triclinic_all,
    };
    use crate::{ccd, gjk};
    use nalgebra::{SMatrix, SVector};
    use symtropy_math::{LatticeOffset, PeriodicLattice, Point, Sphere, Transform};

    fn skewed_lattice() -> PeriodicLattice<2> {
        let basis = SMatrix::<f64, 2, 2>::from_row_slice(&[
            10.0, 4.0,
            0.0, 8.0,
        ]);
        PeriodicLattice::new(SVector::zeros(), basis).unwrap()
    }

    fn sphere(radius: f64) -> Sphere<2> {
        Sphere::new(Point::origin(), radius)
    }

    #[test]
    fn convex_spheres_collide_through_a_skew_lattice_image() {
        let lattice = skewed_lattice();
        let shape_a = sphere(0.5);
        let shape_b = sphere(0.5);
        let transform_a = Transform::from_translation(Point::new([0.5, 0.5]));
        let transform_b = Transform::from_translation(Point::new([10.2, 0.5]));

        let result = query_triclinic_convex(
            &lattice,
            &shape_a,
            &transform_a,
            &shape_b,
            &transform_b,
            64,
        )
        .unwrap();

        assert_eq!(result.status(), PeriodicLatticeCollisionStatus::Intersecting);
        assert!(result.hits.iter().any(|hit| hit.image_offset == LatticeOffset::new([-1, 0])));
        assert!(result.contact_set_complete());
    }

    #[test]
    fn mixed_periodic_mask_does_not_invent_nonperiodic_collision_images() {
        let basis = SMatrix::<f64, 2, 2>::from_row_slice(&[
            10.0, 4.0,
            0.0, 8.0,
        ]);
        let lattice =
            PeriodicLattice::new_with_periodic_axes(SVector::zeros(), basis, [true, false])
                .unwrap();
        let shape_a = sphere(0.5);
        let shape_b = sphere(0.5);
        let transform_a = Transform::from_translation(Point::new([0.5, 0.5]));
        let transform_b = Transform::from_translation(Point::new([10.2, 8.5]));

        let result = query_triclinic_convex(
            &lattice,
            &shape_a,
            &transform_a,
            &shape_b,
            &transform_b,
            64,
        )
        .unwrap();

        assert_eq!(result.status(), PeriodicLatticeCollisionStatus::Separated);
        assert!(result.hits.is_empty());
        assert!(result.unresolved_offsets.is_empty());
        assert_eq!(result.image_candidates, 0);
        assert_eq!(result.gjk_calls, 0);
    }

    #[test]
    fn randomized_triclinic_convex_image_hits_match_direct_gjk_oracle() {
        let lattice = skewed_lattice();
        let mut seed = 0x474a_4b54_5249_434c_u64;
        let mut next = || {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            seed >> 11
        };
        let mut range = |low: f64, high: f64| {
            low + (high - low) * (next() as f64 / ((1_u64 << 53) as f64))
        };

        for case in 0..160 {
            let position_a = SVector::from([range(-12.0, 12.0), range(-12.0, 12.0)]);
            let position_b = SVector::from([range(-12.0, 12.0), range(-12.0, 12.0)]);
            let radius_a = range(0.15, 1.0);
            let radius_b = range(0.15, 1.0);
            let shape_a = sphere(radius_a);
            let shape_b = sphere(radius_b);
            let transform_a = Transform::from_translation(Point(position_a));
            let transform_b = Transform::from_translation(Point(position_b));

            let actual = query_triclinic_convex(
                &lattice,
                &shape_a,
                &transform_a,
                &shape_b,
                &transform_b,
                1_024,
            )
            .unwrap_or_else(|error| panic!("periodic GJK query failed in case {case}: {error}"));
            let actual_hits = actual
                .hits
                .iter()
                .map(|hit| hit.image_offset.periods)
                .collect::<std::collections::BTreeSet<_>>();
            let actual_unresolved = actual
                .unresolved_offsets
                .iter()
                .map(|offset| offset.periods)
                .collect::<std::collections::BTreeSet<_>>();

            let mut oracle_hits = std::collections::BTreeSet::new();
            let mut oracle_unresolved = std::collections::BTreeSet::new();
            for first in -10_i64..=10 {
                for second in -10_i64..=10 {
                    let offset = LatticeOffset::new([first, second]);
                    let shift = lattice.offset_vector(offset).unwrap();
                    let mut image_transform_b = transform_b.clone();
                    image_transform_b.translation.0 += shift;
                    let result = gjk::intersects_transformed(
                        &shape_a,
                        &transform_a,
                        &shape_b,
                        &image_transform_b,
                    );
                    if result.intersecting {
                        oracle_hits.insert([first, second]);
                    } else if result.iterations >= 64 {
                        oracle_unresolved.insert([first, second]);
                    }
                }
            }

            assert_eq!(actual_hits, oracle_hits, "GJK hit image mismatch in case {case}");
            assert_eq!(
                actual_unresolved,
                oracle_unresolved,
                "GJK unresolved image mismatch in case {case}"
            );
            assert_eq!(
                actual.status(),
                if !oracle_hits.is_empty() {
                    PeriodicLatticeCollisionStatus::Intersecting
                } else if !oracle_unresolved.is_empty() {
                    PeriodicLatticeCollisionStatus::Indeterminate
                } else {
                    PeriodicLatticeCollisionStatus::Separated
                },
                "GJK aggregate status mismatch in case {case}"
            );
        }
    }

    #[test]
    fn triclinic_sphere_ccd_detects_a_seam_impact() {
        let lattice = skewed_lattice();
        let hits = sphere_sphere_ccd_triclinic_all(
            &lattice,
            &SVector::from([0.5, 0.5]),
            &SVector::zeros(),
            0.5,
            &SVector::from([9.0, 0.5]),
            &SVector::from([2.0, 0.0]),
            0.5,
            1.0,
            64,
        )
        .unwrap();

        assert!(!hits.is_empty());
        assert_eq!(hits[0].image_offset, LatticeOffset::new([-1, 0]));
        assert!((hits[0].toi_seconds - 0.25).abs() < 1e-12);
        assert!((hits[0].toi_fraction - 0.25).abs() < 1e-12);
    }

    #[test]
    fn triclinic_sphere_ccd_matches_direct_shifted_euclidean_oracle() {
        let lattice = skewed_lattice();
        let position_a = SVector::from([1.0, 1.0]);
        let position_b = SVector::from([9.0, 2.0]);
        let velocity_a = SVector::from([0.1, 0.0]);
        let velocity_b = SVector::from([-1.0, 0.2]);
        let radius_a = 0.7;
        let radius_b = 0.6;
        let dt = 1.5;

        let actual = sphere_sphere_ccd_triclinic_all(
            &lattice,
            &position_a,
            &velocity_a,
            radius_a,
            &position_b,
            &velocity_b,
            radius_b,
            dt,
            256,
        )
        .unwrap();

        let mut oracle = Vec::new();
        for x in -8_i64..=8 {
            for y in -8_i64..=8 {
                let shift = lattice
                    .basis()
                    * SVector::from([x as f64, y as f64]);
                let shifted_b = position_b + shift;
                if let Some(hit) = ccd::sphere_sphere(
                    &position_a,
                    &velocity_a,
                    radius_a,
                    &shifted_b,
                    &velocity_b,
                    radius_b,
                    dt,
                ) {
                    oracle.push((LatticeOffset::new([x, y]), hit.toi));
                }
            }
        }
        oracle.sort_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.periods.cmp(&b.0.periods)));

        assert_eq!(actual.len(), oracle.len());
        for (actual_hit, (offset, toi)) in actual.iter().zip(oracle) {
            assert_eq!(actual_hit.image_offset, offset);
            assert!((actual_hit.toi_seconds - toi).abs() < 1e-10);
        }
    }

    #[test]
    fn randomized_triclinic_ccd_image_sets_match_direct_euclidean_oracle() {
        let lattice = skewed_lattice();
        let mut seed = 0x5452_4943_4343_4421_u64;
        let mut next = || {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            seed >> 11
        };
        let mut range = |low: f64, high: f64| {
            low + (high - low) * (next() as f64 / ((1_u64 << 53) as f64))
        };

        for case in 0..300 {
            let position_a = SVector::from([range(-15.0, 15.0), range(-15.0, 15.0)]);
            let position_b = SVector::from([range(-15.0, 15.0), range(-15.0, 15.0)]);
            let velocity_a = SVector::from([range(-8.0, 8.0), range(-8.0, 8.0)]);
            let velocity_b = SVector::from([range(-8.0, 8.0), range(-8.0, 8.0)]);
            let radius_a = range(0.1, 1.0);
            let radius_b = range(0.1, 1.0);
            let dt = range(0.05, 1.5);

            let actual_hits = sphere_sphere_ccd_triclinic_all(
                &lattice,
                &position_a,
                &velocity_a,
                radius_a,
                &position_b,
                &velocity_b,
                radius_b,
                dt,
                1_024,
            )
            .unwrap_or_else(|error| panic!("production CCD failed in case {case}: {error}"));

            assert!(
                actual_hits.windows(2).all(|pair| {
                    pair[0].toi_seconds < pair[1].toi_seconds
                        || (pair[0].toi_seconds == pair[1].toi_seconds
                            && pair[0].image_offset.periods <= pair[1].image_offset.periods)
                }),
                "production CCD order mismatch in case {case}"
            );

            let mut actual = std::collections::BTreeMap::new();
            for hit in actual_hits {
                actual.insert(hit.image_offset.periods, hit.toi_seconds);
            }

            // Independent image-set oracle: enumerate a substantially wider
            // integer cube, construct each Cartesian translation directly
            // from the basis, and call only the nonperiodic analytic CCD.
            let mut oracle = std::collections::BTreeMap::new();
            for first in -12_i64..=12 {
                for second in -12_i64..=12 {
                    let shifted_b = position_b
                        + lattice.basis()
                            * SVector::from([first as f64, second as f64]);
                    if let Some(hit) = ccd::sphere_sphere(
                        &position_a,
                        &velocity_a,
                        radius_a,
                        &shifted_b,
                        &velocity_b,
                        radius_b,
                        dt,
                    ) {
                        oracle.insert([first, second], hit.toi);
                    }
                }
            }

            assert_eq!(
                actual.keys().copied().collect::<std::collections::BTreeSet<_>>(),
                oracle.keys().copied().collect::<std::collections::BTreeSet<_>>(),
                "per-image CCD hit set mismatch in case {case}"
            );
            for (offset, oracle_toi) in oracle {
                let actual_toi = actual[&offset];
                assert!(
                    (actual_toi - oracle_toi).abs() <= 1.0e-10,
                    "TOI mismatch in case {case} offset={offset:?}: {actual_toi} vs {oracle_toi}"
                );
            }
        }
    }

    #[test]
    fn triclinic_ccd_fails_closed_on_invalid_inputs_and_budget() {
        let lattice = skewed_lattice();
        let invalid = sphere_sphere_ccd_triclinic_all(
            &lattice,
            &SVector::zeros(),
            &SVector::zeros(),
            -1.0,
            &SVector::zeros(),
            &SVector::zeros(),
            0.5,
            1.0,
            64,
        );
        assert!(invalid.is_err());

        let budget = sphere_sphere_ccd_triclinic_all(
            &lattice,
            &SVector::zeros(),
            &SVector::zeros(),
            20.0,
            &SVector::zeros(),
            &SVector::zeros(),
            20.0,
            1.0,
            1,
        );
        assert!(budget.is_err());
    }
}
