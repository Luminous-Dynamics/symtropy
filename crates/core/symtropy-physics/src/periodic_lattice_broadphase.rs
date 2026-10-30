// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Conservative lattice-image enumeration for Cartesian AABB pairs.
//!
//! Candidate bounds are derived from the reciprocal-coordinate projections of
//! the Minkowski difference A-B. Every overlapping translated image must have
//! its integer lattice coordinate inside every projected interval. Projection
//! intervals are independent, so their Cartesian product can over-generate;
//! exact translated-AABB overlap removes those false positives.
//!
//! This is a query primitive only. It does not replace the existing orthogonal
//! PeriodicBox broadphase and is not wired into PhysicsWorld.

use std::error::Error;
use std::fmt;

use nalgebra::SVector;
use symtropy_math::{LatticeOffset, PeriodicLattice, PeriodicLatticeError};

use crate::broadphase::Aabb;

const PROJECTION_ERROR_FACTOR: f64 = 1.0e-9;

/// Failure to conservatively enumerate images for a triclinic AABB pair.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PeriodicLatticeBroadphaseError {
    InvalidAabb { which: &'static str, axis: usize },
    InvalidCandidateBudget,
    CandidateBudgetExceeded { max_candidates: usize },
    CoordinatePrecisionLimit { axis: usize },
    NumericRange { field: &'static str },
    Lattice(PeriodicLatticeError),
}

impl fmt::Display for PeriodicLatticeBroadphaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAabb { which, axis } => {
                write!(f, "invalid {which} AABB on axis {axis}")
            }
            Self::InvalidCandidateBudget => {
                write!(f, "periodic lattice candidate budget must be positive")
            }
            Self::CandidateBudgetExceeded { max_candidates } => {
                write!(f, "periodic lattice candidate box exceeds budget {max_candidates}")
            }
            Self::CoordinatePrecisionLimit { axis } => {
                write!(f, "periodic lattice candidate index exceeds precision envelope on axis {axis}")
            }
            Self::NumericRange { field } => {
                write!(f, "periodic lattice broadphase numeric-range failure in {field}")
            }
            Self::Lattice(error) => write!(f, "periodic lattice geometry error: {error}"),
        }
    }
}

impl Error for PeriodicLatticeBroadphaseError {}

impl From<PeriodicLatticeError> for PeriodicLatticeBroadphaseError {
    fn from(value: PeriodicLatticeError) -> Self {
        Self::Lattice(value)
    }
}

/// Return all lattice offsets n for which the translated B+n image may overlap
/// A, with exact AABB filtering after a conservative reciprocal-space bound.
///
/// The budget applies to the Cartesian product of integer coordinate ranges
/// before exact overlap filtering. Exhaustion is an error, never truncation.
pub fn lattice_image_offsets_for_aabb_pair<const D: usize>(
    lattice: &PeriodicLattice<D>,
    bounds_a: &Aabb<D>,
    bounds_b: &Aabb<D>,
    max_candidates: usize,
) -> Result<Vec<LatticeOffset<D>>, PeriodicLatticeBroadphaseError> {
    if max_candidates == 0 {
        return Err(PeriodicLatticeBroadphaseError::InvalidCandidateBudget);
    }
    validate_aabb(bounds_a, "A")?;
    validate_aabb(bounds_b, "B")?;

    // A and B+nB intersect iff nB belongs to the Minkowski difference A-B.
    let difference_min = bounds_a.min - bounds_b.max;
    let difference_max = bounds_a.max - bounds_b.min;
    if !difference_min.iter().all(|value| value.is_finite())
        || !difference_max.iter().all(|value| value.is_finite())
    {
        return Err(PeriodicLatticeBroadphaseError::NumericRange {
            field: "AABB Minkowski difference",
        });
    }

    let mut fractional_min = SVector::<f64, D>::from_element(f64::MAX);
    let mut fractional_max = SVector::<f64, D>::from_element(f64::MIN);

    // A linear functional reaches its extrema over an axis-aligned box at a
    // corner. D is restricted by PeriodicLattice to 2 or 3, so this is <=8.
    for corner_mask in 0..(1_usize << D) {
        let mut corner = SVector::<f64, D>::zeros();
        for axis in 0..D {
            corner[axis] = if (corner_mask & (1 << axis)) == 0 {
                difference_min[axis]
            } else {
                difference_max[axis]
            };
        }
        let fractional = lattice.to_fractional_coordinates(&corner)?;
        for axis in 0..D {
            fractional_min[axis] = fractional_min[axis].min(fractional[axis]);
            fractional_max[axis] = fractional_max[axis].max(fractional[axis]);
        }
    }

    let projection_padding =
        PROJECTION_ERROR_FACTOR / lattice.min_relative_qr_pivot();
    if !projection_padding.is_finite() {
        return Err(PeriodicLatticeBroadphaseError::NumericRange {
            field: "reciprocal-coordinate projection padding",
        });
    }

    let mut ranges = [(0_i64, -1_i64); D];
    let mut total_candidates = 1_usize;
    for axis in 0..D {
        let scale = fractional_min[axis]
            .abs()
            .max(fractional_max[axis].abs())
            .max(1.0);
        let padding = projection_padding * scale;
        let lower = (fractional_min[axis] - padding).ceil();
        let upper = (fractional_max[axis] + padding).floor();

        if !lower.is_finite()
            || !upper.is_finite()
            || lower < -(4_503_599_627_370_495.0)
            || upper > 4_503_599_627_370_495.0
        {
            return Err(PeriodicLatticeBroadphaseError::CoordinatePrecisionLimit { axis });
        }
        if lower > upper {
            return Ok(Vec::new());
        }

        let low = lower as i64;
        let high = upper as i64;
        ranges[axis] = (low, high);
        let width = usize::try_from(high - low)
            .ok()
            .and_then(|difference| difference.checked_add(1))
            .ok_or(PeriodicLatticeBroadphaseError::NumericRange {
                field: "integer image interval width",
            })?;
        total_candidates = total_candidates
            .checked_mul(width)
            .ok_or(PeriodicLatticeBroadphaseError::CandidateBudgetExceeded {
                max_candidates,
            })?;
        if total_candidates > max_candidates {
            return Err(PeriodicLatticeBroadphaseError::CandidateBudgetExceeded {
                max_candidates,
            });
        }
    }

    let mut output = Vec::new();
    let mut image = [0_i64; D];
    enumerate_and_filter(
        0,
        &ranges,
        lattice,
        bounds_a,
        bounds_b,
        &mut image,
        &mut output,
    )?;
    Ok(output)
}

fn validate_aabb<const D: usize>(
    bounds: &Aabb<D>,
    which: &'static str,
) -> Result<(), PeriodicLatticeBroadphaseError> {
    for axis in 0..D {
        if !bounds.min[axis].is_finite()
            || !bounds.max[axis].is_finite()
            || bounds.min[axis] > bounds.max[axis]
        {
            return Err(PeriodicLatticeBroadphaseError::InvalidAabb { which, axis });
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn enumerate_and_filter<const D: usize>(
    axis: usize,
    ranges: &[(i64, i64); D],
    lattice: &PeriodicLattice<D>,
    bounds_a: &Aabb<D>,
    bounds_b: &Aabb<D>,
    image: &mut [i64; D],
    output: &mut Vec<LatticeOffset<D>>,
) -> Result<(), PeriodicLatticeBroadphaseError> {
    if axis == D {
        let offset = LatticeOffset::new(*image);
        let translation = lattice.offset_vector(offset)?;
        let translated_b = Aabb {
            min: bounds_b.min + translation,
            max: bounds_b.max + translation,
        };
        if !translated_b.min.iter().all(|value| value.is_finite())
            || !translated_b.max.iter().all(|value| value.is_finite())
        {
            return Err(PeriodicLatticeBroadphaseError::NumericRange {
                field: "translated AABB",
            });
        }
        if bounds_a.overlaps(&translated_b) {
            output.push(offset);
        }
        return Ok(());
    }

    let (lower, upper) = ranges[axis];
    let mut value = lower;
    loop {
        image[axis] = value;
        enumerate_and_filter(
            axis + 1,
            ranges,
            lattice,
            bounds_a,
            bounds_b,
            image,
            output,
        )?;
        if value == upper {
            break;
        }
        value = value.checked_add(1).ok_or(
            PeriodicLatticeBroadphaseError::CoordinatePrecisionLimit { axis },
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        PeriodicLatticeBroadphaseError, lattice_image_offsets_for_aabb_pair,
    };
    use crate::broadphase::Aabb;
    use nalgebra::{SMatrix, SVector};
    use std::collections::BTreeSet;
    use symtropy_math::{LatticeOffset, PeriodicLattice};

    fn aabb(min: [f64; 2], max: [f64; 2]) -> Aabb<2> {
        Aabb {
            min: SVector::from(min),
            max: SVector::from(max),
        }
    }

    fn direct_overlap(a: &Aabb<2>, b: &Aabb<2>) -> bool {
        (0..2).all(|axis| a.max[axis] >= b.min[axis] && a.min[axis] <= b.max[axis])
    }

    fn direct_oracle_2d(
        a: &Aabb<2>,
        b: &Aabb<2>,
        basis: &SMatrix<f64, 2, 2>,
        radius: i64,
    ) -> BTreeSet<[i64; 2]> {
        let mut result = BTreeSet::new();
        for x in -radius..=radius {
            for y in -radius..=radius {
                let offset = [x, y];
                let shift = basis * SVector::from([x as f64, y as f64]);
                let translated = Aabb {
                    min: b.min + shift,
                    max: b.max + shift,
                };
                if direct_overlap(a, &translated) {
                    result.insert(offset);
                }
            }
        }
        result
    }

    fn next_random(state: &mut u64) -> u64 {
        *state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        *state
    }

    fn random_range(state: &mut u64, lower: f64, upper: f64) -> f64 {
        let bits = next_random(state) >> 11;
        lower + (upper - lower) * (bits as f64 / ((1_u64 << 53) as f64))
    }

    #[test]
    fn skewed_2d_candidate_set_matches_direct_integer_image_oracle() {
        let basis = SMatrix::<f64, 2, 2>::from_row_slice(&[3.0, 1.4, 0.0, 2.0]);
        let lattice = PeriodicLattice::new(SVector::zeros(), basis).unwrap();
        let a = aabb([-0.3, -0.2], [0.7, 0.8]);
        let b = aabb([2.1, 1.0], [2.8, 1.6]);

        let actual = lattice_image_offsets_for_aabb_pair(&lattice, &a, &b, 256)
            .unwrap()
            .into_iter()
            .map(|offset| offset.periods)
            .collect::<BTreeSet<_>>();
        let expected = direct_oracle_2d(&a, &b, &basis, 12);

        assert_eq!(actual, expected);
    }

    #[test]
    fn randomized_2d_lattice_aabb_sets_have_no_missed_images() {
        let basis = SMatrix::<f64, 2, 2>::from_row_slice(&[2.0, 0.8, 0.0, 1.5]);
        let lattice = PeriodicLattice::new(SVector::zeros(), basis).unwrap();
        let mut seed = 0x4c41_5454_4943_4542_u64;

        for case in 0..1_000 {
            let ax = random_range(&mut seed, -4.0, 4.0);
            let ay = random_range(&mut seed, -4.0, 4.0);
            let bx = random_range(&mut seed, -4.0, 4.0);
            let by = random_range(&mut seed, -4.0, 4.0);
            let aw = random_range(&mut seed, 0.0, 1.5);
            let ah = random_range(&mut seed, 0.0, 1.5);
            let bw = random_range(&mut seed, 0.0, 1.5);
            let bh = random_range(&mut seed, 0.0, 1.5);
            let a = aabb([ax, ay], [ax + aw, ay + ah]);
            let b = aabb([bx, by], [bx + bw, by + bh]);

            let actual = lattice_image_offsets_for_aabb_pair(&lattice, &a, &b, 1_024)
                .unwrap_or_else(|error| panic!("case={case}: {error}"))
                .into_iter()
                .map(|offset| offset.periods)
                .collect::<BTreeSet<_>>();
            let expected = direct_oracle_2d(&a, &b, &basis, 12);
            assert!(
                actual.is_superset(&expected),
                "missed lattice image in case={case}; actual={actual:?} expected={expected:?}"
            );
        }
    }

    fn aabb3(min: [f64; 3], max: [f64; 3]) -> Aabb<3> {
        Aabb {
            min: SVector::from(min),
            max: SVector::from(max),
        }
    }

    fn direct_overlap_3d(a: &Aabb<3>, b: &Aabb<3>) -> bool {
        (0..3).all(|axis| a.max[axis] >= b.min[axis] && a.min[axis] <= b.max[axis])
    }

    fn direct_oracle_3d(
        a: &Aabb<3>,
        b: &Aabb<3>,
        basis: &SMatrix<f64, 3, 3>,
        radius: i64,
    ) -> BTreeSet<[i64; 3]> {
        let mut result = BTreeSet::new();
        for x in -radius..=radius {
            for y in -radius..=radius {
                for z in -radius..=radius {
                    let offset = [x, y, z];
                    let shift = basis * SVector::from([x as f64, y as f64, z as f64]);
                    let translated = Aabb {
                        min: b.min + shift,
                        max: b.max + shift,
                    };
                    if direct_overlap_3d(a, &translated) {
                        result.insert(offset);
                    }
                }
            }
        }
        result
    }

    #[test]
    fn randomized_3d_lattice_aabb_sets_have_no_missed_images() {
        let basis = SMatrix::<f64, 3, 3>::from_row_slice(&[
            2.0, 0.7, 0.2,
            0.0, 1.6, 0.5,
            0.0, 0.0, 1.3,
        ]);
        let lattice = PeriodicLattice::new(SVector::zeros(), basis.clone()).unwrap();
        let mut seed = 0x3344_4c41_5454_4943_u64;

        for case in 0..80 {
            let ax = random_range(&mut seed, -3.0, 3.0);
            let ay = random_range(&mut seed, -3.0, 3.0);
            let az = random_range(&mut seed, -3.0, 3.0);
            let bx = random_range(&mut seed, -3.0, 3.0);
            let by = random_range(&mut seed, -3.0, 3.0);
            let bz = random_range(&mut seed, -3.0, 3.0);
            let aw = random_range(&mut seed, 0.0, 0.8);
            let ah = random_range(&mut seed, 0.0, 0.8);
            let ad = random_range(&mut seed, 0.0, 0.8);
            let bw = random_range(&mut seed, 0.0, 0.8);
            let bh = random_range(&mut seed, 0.0, 0.8);
            let bd = random_range(&mut seed, 0.0, 0.8);
            let a = aabb3([ax, ay, az], [ax + aw, ay + ah, az + ad]);
            let b = aabb3([bx, by, bz], [bx + bw, by + bh, bz + bd]);

            let actual = lattice_image_offsets_for_aabb_pair(&lattice, &a, &b, 4_096)
                .unwrap_or_else(|error| panic!("3D case={case}: {error}"))
                .into_iter()
                .map(|offset| offset.periods)
                .collect::<BTreeSet<_>>();
            let expected = direct_oracle_3d(&a, &b, &basis, 10);
            assert!(
                actual.is_superset(&expected),
                "missed 3D lattice image in case={case}; actual={actual:?} expected={expected:?}"
            );
        }
    }

    #[test]
    fn touching_faces_are_included_and_budget_exhaustion_is_explicit() {
        let basis = SMatrix::<f64, 2, 2>::identity();
        let lattice = PeriodicLattice::new(SVector::zeros(), basis).unwrap();
        let a = aabb([0.0, 0.0], [1.0, 1.0]);
        let b = aabb([2.0, 0.0], [3.0, 1.0]);

        let touching = lattice_image_offsets_for_aabb_pair(&lattice, &a, &b, 8).unwrap();
        assert!(touching.contains(&LatticeOffset::new([-1, 0])));

        let wide_a = aabb([-50.0, -50.0], [50.0, 50.0]);
        assert_eq!(
            lattice_image_offsets_for_aabb_pair(&lattice, &wide_a, &b, 8),
            Err(PeriodicLatticeBroadphaseError::CandidateBudgetExceeded {
                max_candidates: 8
            })
        );
    }

    #[test]
    fn non_finite_and_inverted_aabbs_are_rejected() {
        let basis = SMatrix::<f64, 2, 2>::identity();
        let lattice = PeriodicLattice::new(SVector::zeros(), basis).unwrap();
        let valid = aabb([0.0, 0.0], [1.0, 1.0]);
        let invalid = aabb([2.0, 0.0], [1.0, 1.0]);

        assert_eq!(
            lattice_image_offsets_for_aabb_pair(&lattice, &invalid, &valid, 8),
            Err(PeriodicLatticeBroadphaseError::InvalidAabb {
                which: "A",
                axis: 0
            })
        );
    }
}
