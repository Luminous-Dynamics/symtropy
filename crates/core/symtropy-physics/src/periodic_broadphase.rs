// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Conservative periodic-image candidate generation for axis-aligned bounds.
//!
//! This module is intentionally a bounded broadphase primitive, not a complete
//! periodic collision pipeline. It enumerates every lattice translation of B
//! whose AABB can overlap A, subject to an explicit image budget. Callers must
//! still run the configured narrow phase on each candidate and deduplicate
//! contact resolution by stable body/image identity.

use std::error::Error;
use std::fmt;

use nalgebra::SVector;
use symtropy_math::{LatticeOffset, PeriodicBox, PeriodicDomainError};

use crate::broadphase::Aabb;

const MAX_EXACT_PERIOD_INDEX: f64 = 4_503_599_627_370_496.0; // 2^52

/// Failure reason for bounded periodic-image enumeration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeriodicBroadphaseError {
    Domain(PeriodicDomainError),
    InvalidBounds { axis: usize },
    NonFiniteBounds { axis: usize },
    CoordinatePrecisionLimit { axis: usize },
    CandidateCountOverflow,
    CandidateBudgetExceeded { limit: usize },
}

impl fmt::Display for PeriodicBroadphaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Domain(error) => write!(f, "periodic domain error: {error}"),
            Self::InvalidBounds { axis } => write!(f, "invalid AABB bounds on axis {axis}"),
            Self::NonFiniteBounds { axis } => write!(f, "non-finite AABB bound on axis {axis}"),
            Self::CoordinatePrecisionLimit { axis } => {
                write!(f, "periodic image range exceeds precision envelope on axis {axis}")
            }
            Self::CandidateCountOverflow => write!(f, "periodic candidate count overflow"),
            Self::CandidateBudgetExceeded { limit } => {
                write!(f, "periodic image candidate budget exceeded (limit {limit})")
            }
        }
    }
}

impl Error for PeriodicBroadphaseError {}

impl From<PeriodicDomainError> for PeriodicBroadphaseError {
    fn from(value: PeriodicDomainError) -> Self {
        Self::Domain(value)
    }
}

fn validate_aabb<const D: usize>(
    bounds: &Aabb<D>,
) -> Result<(), PeriodicBroadphaseError> {
    for axis in 0..D {
        if !bounds.min[axis].is_finite() || !bounds.max[axis].is_finite() {
            return Err(PeriodicBroadphaseError::NonFiniteBounds { axis });
        }
        if bounds.min[axis] > bounds.max[axis] {
            return Err(PeriodicBroadphaseError::InvalidBounds { axis });
        }
    }
    Ok(())
}

/// Enumerate all integer lattice offsets n for which B + n * periods can
/// overlap A, including boundary-touching AABBs.
///
/// An offset translates B relative to A. Periodic axes may return more than
/// one offset when bodies are large relative to the box; this is deliberate.
/// Non-periodic axes must overlap without translation. The output is
/// deterministic and lexicographically ordered by axis-0-first generation.
///
/// If a finite search cannot be represented safely or the product of per-axis
/// image ranges exceeds max_candidates, the function returns an error. It
/// never reports an empty list when image enumeration was truncated.
pub fn image_offsets_for_aabb_pair<const D: usize>(
    domain: &PeriodicBox<D>,
    a: &Aabb<D>,
    b: &Aabb<D>,
    max_candidates: usize,
) -> Result<Vec<LatticeOffset<D>>, PeriodicBroadphaseError> {
    validate_aabb(a)?;
    validate_aabb(b)?;

    let mut ranges = [(0_i64, 0_i64); D];
    let mut total = 1_u128;

    for axis in 0..D {
        if domain.is_periodic(axis) == Some(false) {
            if a.max[axis] < b.min[axis] || a.min[axis] > b.max[axis] {
                return Ok(Vec::new());
            }
            ranges[axis] = (0, 0);
            continue;
        }

        let period = domain.periods()[axis];
        let low_delta = a.min[axis] - b.max[axis];
        let high_delta = a.max[axis] - b.min[axis];
        if !low_delta.is_finite() || !high_delta.is_finite() {
            return Err(PeriodicBroadphaseError::CoordinatePrecisionLimit { axis });
        }

        let low_ratio = low_delta / period;
        let high_ratio = high_delta / period;
        if !low_ratio.is_finite()
            || !high_ratio.is_finite()
            || low_ratio.abs() >= MAX_EXACT_PERIOD_INDEX
            || high_ratio.abs() >= MAX_EXACT_PERIOD_INDEX
        {
            return Err(PeriodicBroadphaseError::CoordinatePrecisionLimit { axis });
        }

        // Bias the integer interval outward by a small ratio-scaled margin.
        // The narrow phase removes harmless extra candidates; under-enumerating
        // a boundary-touching image would be a false negative.
        let ratio_scale = low_ratio.abs().max(high_ratio.abs()).max(1.0);
        let rounding_pad = 8.0 * f64::EPSILON * ratio_scale;
        let lower = (low_ratio - rounding_pad).ceil();
        let upper = (high_ratio + rounding_pad).floor();
        if lower > upper {
            return Ok(Vec::new());
        }

        // All offsets are within the exactly represented integer range here.
        let lower_i = lower as i64;
        let upper_i = upper as i64;
        ranges[axis] = (lower_i, upper_i);

        let count = (upper_i as i128 - lower_i as i128 + 1) as u128;
        total = total
            .checked_mul(count)
            .ok_or(PeriodicBroadphaseError::CandidateCountOverflow)?;
        if total > max_candidates as u128 {
            return Err(PeriodicBroadphaseError::CandidateBudgetExceeded {
                limit: max_candidates,
            });
        }
    }

    if total > max_candidates as u128 {
        return Err(PeriodicBroadphaseError::CandidateBudgetExceeded {
            limit: max_candidates,
        });
    }

    let mut candidates = Vec::with_capacity(total as usize);
    candidates.push(LatticeOffset::zero());

    for axis in 0..D {
        let (lower, upper) = ranges[axis];
        let mut next = Vec::with_capacity(candidates.len() * (upper - lower + 1) as usize);
        for existing in candidates {
            for count in lower..=upper {
                let mut image = existing.periods;
                image[axis] = count;
                next.push(LatticeOffset::new(image));
            }
        }
        candidates = next;
    }

    Ok(candidates)
}

/// Translate an AABB by a declared lattice image offset.
pub fn translate_aabb<const D: usize>(
    domain: &PeriodicBox<D>,
    bounds: &Aabb<D>,
    offset: LatticeOffset<D>,
) -> Result<Aabb<D>, PeriodicBroadphaseError> {
    validate_aabb(bounds)?;
    let shift = domain.offset_vector(offset)?;
    let min = bounds.min + shift;
    let max = bounds.max + shift;
    for axis in 0..D {
        if !min[axis].is_finite() || !max[axis].is_finite() {
            return Err(PeriodicBroadphaseError::CoordinatePrecisionLimit { axis });
        }
    }
    Ok(Aabb { min, max })
}

/// Conservative union of two endpoint AABBs.
///
/// This is a valid swept bound for pure linear translation with fixed
/// orientation. It is not sufficient for arbitrary rotation; such callers
/// must supply a rotation-aware swept bound before periodic image generation.
pub fn swept_aabb_linear<const D: usize>(start: &Aabb<D>, end: &Aabb<D>) -> Aabb<D> {
    start.union(end)
}

#[cfg(test)]
mod tests {
    use super::{
        image_offsets_for_aabb_pair, swept_aabb_linear, translate_aabb,
        PeriodicBroadphaseError,
    };
    use crate::broadphase::Aabb;
    use nalgebra::SVector;
    use std::collections::HashSet;
    use symtropy_math::{LatticeOffset, PeriodicBox};

    fn bounds(min: [f64; 2], max: [f64; 2]) -> Aabb<2> {
        Aabb {
            min: SVector::from(min),
            max: SVector::from(max),
        }
    }

    fn x_periodic_domain() -> PeriodicBox<2> {
        PeriodicBox::new(
            SVector::zeros(),
            SVector::from([10.0, 0.0]),
            [true, false],
        )
        .unwrap()
    }

    #[test]
    fn emits_seam_crossing_image_candidate() {
        let domain = x_periodic_domain();
        let a = bounds([0.0, 0.0], [0.3, 1.0]);
        let b = bounds([9.8, 0.0], [10.2, 1.0]);

        let offsets = image_offsets_for_aabb_pair(&domain, &a, &b, 8).unwrap();
        assert_eq!(offsets, vec![LatticeOffset::new([-1, 0])]);

        let shifted_b = translate_aabb(&domain, &b, offsets[0]).unwrap();
        assert!(a.overlaps(&shifted_b));
    }

    #[test]
    fn matches_an_independent_bounded_image_enumerator() {
        let domain = x_periodic_domain();
        let fixtures = [
            (bounds([0.0, 0.0], [0.3, 1.0]), bounds([9.8, 0.0], [10.2, 1.0])),
            (bounds([1.0, 0.0], [2.0, 1.0]), bounds([3.0, 0.0], [4.0, 1.0])),
            (bounds([-1.0, 0.0], [1.0, 1.0]), bounds([8.0, 0.0], [12.0, 1.0])),
        ];

        for (a, b) in fixtures {
            let actual: HashSet<_> = image_offsets_for_aabb_pair(&domain, &a, &b, 32)
                .unwrap()
                .into_iter()
                .collect();
            let mut oracle = HashSet::new();
            for n in -4_i64..=4 {
                let offset = LatticeOffset::new([n, 0]);
                if a.overlaps(&translate_aabb(&domain, &b, offset).unwrap()) {
                    oracle.insert(offset);
                }
            }
            assert_eq!(actual, oracle);
        }
    }

    #[test]
    fn large_bodies_can_have_multiple_overlapping_images() {
        let domain = x_periodic_domain();
        let a = bounds([0.0, 0.0], [25.0, 1.0]);
        let b = bounds([0.0, 0.0], [25.0, 1.0]);
        let offsets = image_offsets_for_aabb_pair(&domain, &a, &b, 8).unwrap();
        let x_offsets: Vec<_> = offsets.iter().map(|o| o.periods[0]).collect();
        assert_eq!(x_offsets, vec![-2, -1, 0, 1, 2]);
    }

    #[test]
    fn non_periodic_axis_can_reject_a_pair() {
        let domain = x_periodic_domain();
        let a = bounds([0.0, 0.0], [1.0, 1.0]);
        let b = bounds([9.0, 2.0], [11.0, 3.0]);
        assert!(
            image_offsets_for_aabb_pair(&domain, &a, &b, 8)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn candidate_budget_exhaustion_is_not_reported_as_separation() {
        let domain = PeriodicBox::new(
            SVector::zeros(),
            SVector::from([10.0, 10.0]),
            [true, true],
        )
        .unwrap();
        let a = bounds([0.0, 0.0], [30.0, 30.0]);
        let b = bounds([0.0, 0.0], [30.0, 30.0]);
        assert_eq!(
            image_offsets_for_aabb_pair(&domain, &a, &b, 10),
            Err(PeriodicBroadphaseError::CandidateBudgetExceeded { limit: 10 })
        );
    }

    #[test]
    fn invalid_aabb_is_rejected() {
        let domain = x_periodic_domain();
        let a = bounds([1.0, 0.0], [0.0, 1.0]);
        let b = bounds([0.0, 0.0], [1.0, 1.0]);
        assert_eq!(
            image_offsets_for_aabb_pair(&domain, &a, &b, 8),
            Err(PeriodicBroadphaseError::InvalidBounds { axis: 0 })
        );
    }

    #[test]
    fn image_offsets_are_deterministic_and_unique() {
        let domain = PeriodicBox::new(
            SVector::zeros(),
            SVector::from([10.0, 10.0]),
            [true, true],
        )
        .unwrap();
        let a = bounds([0.0, 0.0], [15.0, 15.0]);
        let b = bounds([0.0, 0.0], [15.0, 15.0]);
        let x = image_offsets_for_aabb_pair(&domain, &a, &b, 16).unwrap();
        let y = image_offsets_for_aabb_pair(&domain, &a, &b, 16).unwrap();
        assert_eq!(x, y);
        assert_eq!(x.len(), x.iter().copied().collect::<HashSet<_>>().len());
    }

    #[test]
    fn linear_swept_bounds_include_both_endpoints() {
        let start = bounds([0.0, -1.0], [1.0, 1.0]);
        let end = bounds([9.0, -1.0], [10.0, 1.0]);
        let swept = swept_aabb_linear(&start, &end);
        assert_eq!(swept.min, SVector::from([0.0, -1.0]));
        assert_eq!(swept.max, SVector::from([10.0, 1.0]));
    }
}
