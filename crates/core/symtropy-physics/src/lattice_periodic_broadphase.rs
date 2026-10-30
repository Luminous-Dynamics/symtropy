// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Conservative broadphase image enumeration for skewed periodic lattices.
//!
//! For two world-space AABBs A and B, a lattice translation s=B*n can
//! produce overlap only if s lies in the component-wise interval
//! [A.min - B.max, A.max - B.min]. Applying the inverse lattice basis to this
//! interval gives conservative integer-coordinate ranges for n. The ranges
//! are enumerated under an explicit budget and then filtered by exact
//! translated-AABB overlap.
//!
//! This module is a broadphase candidate generator only. A caller must still
//! run the intended narrow phase for every returned image. Budget exhaustion
//! is an error, never a separation result.

use std::error::Error;
use std::fmt;

use nalgebra::SVector;
use symtropy_math::{LatticeDomainError, LatticePeriodicDomain};

use crate::broadphase::Aabb;

const MAX_EXACT_LATTICE_INDEX: f64 = 4_503_599_627_370_496.0; // 2^52
const OUTWARD_ROUNDING_EPSILON_MULTIPLIER: f64 = 32.0;

/// Failure reason for bounded skew-lattice AABB image enumeration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LatticeBroadphaseError {
    Domain(LatticeDomainError),
    InvalidBounds { axis: usize },
    NonFiniteBounds { axis: usize },
    NumericRange { axis: usize },
    CandidateCountOverflow,
    CandidateBudgetExceeded { limit: usize },
}

impl fmt::Display for LatticeBroadphaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Domain(error) => write!(f, "lattice domain error: {error}"),
            Self::InvalidBounds { axis } => write!(f, "invalid AABB bounds on axis {axis}"),
            Self::NonFiniteBounds { axis } => write!(f, "non-finite AABB bound on axis {axis}"),
            Self::NumericRange { axis } => {
                write!(f, "skew-lattice candidate range exceeded numeric envelope on axis {axis}")
            }
            Self::CandidateCountOverflow => write!(f, "lattice image candidate count overflow"),
            Self::CandidateBudgetExceeded { limit } => {
                write!(f, "lattice image candidate budget exceeded (limit {limit})")
            }
        }
    }
}

impl Error for LatticeBroadphaseError {}

impl From<LatticeDomainError> for LatticeBroadphaseError {
    fn from(value: LatticeDomainError) -> Self {
        Self::Domain(value)
    }
}

fn validate_aabb<const D: usize>(bounds: &Aabb<D>) -> Result<(), LatticeBroadphaseError> {
    for axis in 0..D {
        if !bounds.min[axis].is_finite() || !bounds.max[axis].is_finite() {
            return Err(LatticeBroadphaseError::NonFiniteBounds { axis });
        }
        if bounds.min[axis] > bounds.max[axis] {
            return Err(LatticeBroadphaseError::InvalidBounds { axis });
        }
    }
    Ok(())
}

/// Enumerate every integer lattice offset whose translated B AABB overlaps A.
///
/// The returned offset n translates B by basis*n relative to A. The output is
/// lexicographically deterministic, includes boundary-touching cases, and is
/// exact-filtered after conservative inverse-basis interval enumeration.
///
/// Non-periodic basis axes are fixed to zero. The search budget applies to the
/// conservative integer product before exact AABB filtering; therefore a
/// highly skewed cell may fail closed even when few final overlaps exist.
pub fn image_offsets_for_aabb_pair_lattice<const D: usize>(
    domain: &LatticePeriodicDomain<D>,
    a: &Aabb<D>,
    b: &Aabb<D>,
    max_candidates: usize,
) -> Result<Vec<[i64; D]>, LatticeBroadphaseError> {
    validate_aabb(a)?;
    validate_aabb(b)?;

    if max_candidates == 0 {
        return Err(LatticeBroadphaseError::CandidateBudgetExceeded { limit: 0 });
    }

    let mut fractional_ranges = [(0_i64, 0_i64); D];
    let mut total = 1_u128;

    for lattice_axis in 0..D {
        if !domain.periodic_axes()[lattice_axis] {
            continue;
        }

        // Necessary world-space translation interval for AABB overlap.
        let mut world_lower = SVector::<f64, D>::zeros();
        let mut world_upper = SVector::<f64, D>::zeros();
        for world_axis in 0..D {
            world_lower[world_axis] = a.min[world_axis] - b.max[world_axis];
            world_upper[world_axis] = a.max[world_axis] - b.min[world_axis];
            if !world_lower[world_axis].is_finite()
                || !world_upper[world_axis].is_finite()
            {
                return Err(LatticeBroadphaseError::NumericRange { axis: world_axis });
            }
        }

        // Fractional coordinate n_i = row_i(B^-1) dot translation.
        // Interval arithmetic bounds n_i for every translation in the world
        // AABB-difference box. This can over-generate but cannot omit a
        // translation contained in that box (modulo the explicit float margin).
        let inverse_row = domain.inverse_basis().row(lattice_axis);
        let mut fractional_lower = 0.0;
        let mut fractional_upper = 0.0;

        for world_axis in 0..D {
            let coefficient = inverse_row[world_axis];
            let low = world_lower[world_axis];
            let high = world_upper[world_axis];
            if coefficient >= 0.0 {
                fractional_lower += coefficient * low;
                fractional_upper += coefficient * high;
            } else {
                fractional_lower += coefficient * high;
                fractional_upper += coefficient * low;
            }
        }

        if !fractional_lower.is_finite() || !fractional_upper.is_finite() {
            return Err(LatticeBroadphaseError::NumericRange {
                axis: lattice_axis,
            });
        }

        let scale = fractional_lower
            .abs()
            .max(fractional_upper.abs())
            .max(1.0);
        let padding = OUTWARD_ROUNDING_EPSILON_MULTIPLIER * f64::EPSILON * scale;
        let lower_real = fractional_lower - padding;
        let upper_real = fractional_upper + padding;
        if !lower_real.is_finite()
            || !upper_real.is_finite()
            || lower_real.abs() >= MAX_EXACT_LATTICE_INDEX
            || upper_real.abs() >= MAX_EXACT_LATTICE_INDEX
        {
            return Err(LatticeBroadphaseError::NumericRange {
                axis: lattice_axis,
            });
        }

        let lower = lower_real.ceil() as i64;
        let upper = upper_real.floor() as i64;
        if lower > upper {
            return Ok(Vec::new());
        }
        fractional_ranges[lattice_axis] = (lower, upper);

        let count = (upper as i128 - lower as i128 + 1) as u128;
        total = total
            .checked_mul(count)
            .ok_or(LatticeBroadphaseError::CandidateCountOverflow)?;
        if total > max_candidates as u128 {
            return Err(LatticeBroadphaseError::CandidateBudgetExceeded {
                limit: max_candidates,
            });
        }
    }

    let mut offsets = Vec::with_capacity(total as usize);
    offsets.push([0_i64; D]);
    for axis in 0..D {
        let (lower, upper) = fractional_ranges[axis];
        let count = (upper as i128 - lower as i128 + 1) as usize;
        let mut next = Vec::with_capacity(offsets.len() * count);
        for existing in offsets {
            for value in lower..=upper {
                let mut candidate = existing;
                candidate[axis] = value;
                next.push(candidate);
            }
        }
        offsets = next;
    }

    let mut overlapping = Vec::new();
    for offset in offsets {
        let shift = domain.offset_vector(offset)?;
        let translated_min = b.min + shift;
        let translated_max = b.max + shift;
        for axis in 0..D {
            if !translated_min[axis].is_finite() || !translated_max[axis].is_finite() {
                return Err(LatticeBroadphaseError::NumericRange { axis });
            }
        }
        let translated = Aabb {
            min: translated_min,
            max: translated_max,
        };
        if a.overlaps(&translated) {
            overlapping.push(offset);
        }
    }

    Ok(overlapping)
}

#[cfg(test)]
mod tests {
    use super::{
        image_offsets_for_aabb_pair_lattice, LatticeBroadphaseError,
    };
    use crate::broadphase::Aabb;
    use nalgebra::{SMatrix, SVector};
    use std::collections::HashSet;
    use symtropy_math::LatticePeriodicDomain;

    fn bounds(min: [f64; 2], max: [f64; 2]) -> Aabb<2> {
        Aabb {
            min: SVector::from(min),
            max: SVector::from(max),
        }
    }

    fn skew_domain() -> LatticePeriodicDomain<2> {
        LatticePeriodicDomain::new(
            SVector::zeros(),
            SMatrix::new(10.0, 3.0, 0.0, 8.0),
            [true, true],
            8192,
        )
        .unwrap()
    }

    fn brute_force<const D: usize>(
        domain: &LatticePeriodicDomain<D>,
        a: &Aabb<D>,
        b: &Aabb<D>,
        span: i64,
    ) -> HashSet<[i64; D]> {
        let mut result = HashSet::new();
        let mut offsets = vec![[0_i64; D]];
        for axis in 0..D {
            if !domain.periodic_axes()[axis] {
                continue;
            }
            let mut next = Vec::new();
            for existing in offsets {
                for image in -span..=span {
                    let mut candidate = existing;
                    candidate[axis] = image;
                    next.push(candidate);
                }
            }
            offsets = next;
        }

        for offset in offsets {
            // Independent integer-image enumeration; direct B*n translation
            // avoids using the production image-range calculation.
            let mut coeffs = SVector::<f64, D>::zeros();
            for axis in 0..D {
                coeffs[axis] = offset[axis] as f64;
            }
            let shift = domain.basis() * coeffs;
            let translated = Aabb {
                min: b.min + shift,
                max: b.max + shift,
            };
            if a.overlaps(&translated) {
                result.insert(offset);
            }
        }
        result
    }

    #[test]
    fn skew_seam_contact_uses_non_axis_aligned_lattice_image() {
        let domain = skew_domain();
        let a = bounds([0.0, 0.0], [1.0, 1.0]);
        // B is one second basis vector above A: the required image is (0,-1).
        let b = bounds([3.0, 8.0], [4.0, 9.0]);

        let offsets = image_offsets_for_aabb_pair_lattice(&domain, &a, &b, 128).unwrap();
        assert_eq!(offsets, vec![[0, -1]]);
    }

    #[test]
    fn skew_lattice_candidate_set_matches_direct_integer_image_oracle() {
        let domain = skew_domain();
        let fixtures = [
            (bounds([0.0, 0.0], [1.0, 1.0]), bounds([3.0, 8.0], [4.0, 9.0])),
            (bounds([-2.0, 1.0], [1.5, 4.0]), bounds([8.0, -7.0], [12.0, -2.0])),
            (bounds([0.0, 0.0], [22.0, 17.0]), bounds([-8.0, -7.0], [14.0, 12.0])),
            (bounds([2.0, -3.0], [2.5, -2.5]), bounds([24.0, 5.0], [25.0, 6.0])),
        ];

        for (a, b) in fixtures {
            let actual: HashSet<_> = image_offsets_for_aabb_pair_lattice(
                &domain, &a, &b, 8192,
            )
            .unwrap()
            .into_iter()
            .collect();
            let oracle = brute_force(&domain, &a, &b, 8);
            assert_eq!(actual, oracle);
        }
    }

    #[test]
    fn mixed_periodic_rank_never_translates_nonperiodic_basis_axis() {
        let domain = LatticePeriodicDomain::new(
            SVector::zeros(),
            SMatrix::new(10.0, 3.0, 0.0, 8.0),
            [true, false],
            1024,
        )
        .unwrap();
        let a = bounds([0.0, 0.0], [1.0, 1.0]);
        let b = bounds([10.0, 0.0], [11.0, 1.0]);

        let offsets = image_offsets_for_aabb_pair_lattice(&domain, &a, &b, 128).unwrap();
        assert_eq!(offsets, vec![[-1, 0]]);
    }

    #[test]
    fn large_objects_keep_all_overlapping_skew_images() {
        let domain = skew_domain();
        let a = bounds([-15.0, -12.0], [16.0, 14.0]);
        let b = bounds([-15.0, -12.0], [16.0, 14.0]);
        let actual: HashSet<_> = image_offsets_for_aabb_pair_lattice(
            &domain, &a, &b, 8192,
        )
        .unwrap()
        .into_iter()
        .collect();
        let oracle = brute_force(&domain, &a, &b, 8);
        assert_eq!(actual, oracle);
        assert!(actual.len() > 1);
    }

    #[test]
    fn candidate_budget_exhaustion_is_not_separation() {
        let domain = LatticePeriodicDomain::new(
            SVector::zeros(),
            SMatrix::new(10.0, 3.0, 0.0, 8.0),
            [true, true],
            8192,
        )
        .unwrap();
        let a = bounds([-100.0, -100.0], [100.0, 100.0]);
        let b = bounds([-100.0, -100.0], [100.0, 100.0]);

        assert_eq!(
            image_offsets_for_aabb_pair_lattice(&domain, &a, &b, 1),
            Err(LatticeBroadphaseError::CandidateBudgetExceeded { limit: 1 })
        );
    }

    #[test]
    fn invalid_aabb_is_rejected() {
        let domain = skew_domain();
        let a = bounds([1.0, 0.0], [0.0, 1.0]);
        let b = bounds([0.0, 0.0], [1.0, 1.0]);
        assert_eq!(
            image_offsets_for_aabb_pair_lattice(&domain, &a, &b, 64),
            Err(LatticeBroadphaseError::InvalidBounds { axis: 0 })
        );
    }
}
