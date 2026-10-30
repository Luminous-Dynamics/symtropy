// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Periodic geometry for full-rank skewed (including triclinic) lattice cells.
//!
//! Unlike PeriodicBox, this type does not assume orthogonal lattice vectors.
//! The columns of basis generate periodic translations. Fractional coordinates
//! are used for canonical wrapping, but not component-wise rounding for the
//! nearest image: minimum-image displacement solves a bounded closest-lattice-
//! vector problem and returns an explicit ambiguity flag at equal-distance ties.
//!
//! This module supplies geometry only. It does not yet integrate skewed cells
//! into rigid-body broadphase, narrowphase, CCD, constraints, or fluid solvers.

use std::error::Error;
use std::fmt;

use nalgebra::{SMatrix, SVector};

const MAX_EXACT_LATTICE_INDEX: f64 = 4_503_599_627_370_496.0; // 2^52
const MAX_CONDITION_PROXY: f64 = 1.0e12;
const OUTWARD_ROUNDING_EPSILON_MULTIPLIER: f64 = 32.0;
const DISTANCE_TIE_EPSILON_MULTIPLIER: f64 = 64.0;

/// Why a lattice-domain operation could not produce a trustworthy result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LatticeDomainError {
    InvalidDimension,
    InvalidSearchBudget,
    NonFiniteOrigin,
    NonFiniteBasis,
    SingularBasis,
    IllConditionedBasis,
    NonFiniteCoordinate,
    FractionalCoordinatePrecisionLimit { axis: usize },
    InvalidImageOffset { axis: usize },
    OffsetOnNonPeriodicAxis { axis: usize },
    NumericRange,
    SearchBudgetExceeded { limit: usize },
    CandidateCountOverflow,
}

impl fmt::Display for LatticeDomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDimension => write!(f, "lattice domain requires D >= 1"),
            Self::InvalidSearchBudget => write!(f, "closest-image search budget must be positive"),
            Self::NonFiniteOrigin => write!(f, "lattice origin contains a non-finite value"),
            Self::NonFiniteBasis => write!(f, "lattice basis contains a non-finite value"),
            Self::SingularBasis => write!(f, "lattice basis is singular"),
            Self::IllConditionedBasis => write!(f, "lattice basis exceeds the supported conditioning envelope"),
            Self::NonFiniteCoordinate => write!(f, "point or displacement contains a non-finite value"),
            Self::FractionalCoordinatePrecisionLimit { axis } => {
                write!(f, "fractional lattice coordinate exceeds precision envelope on axis {axis}")
            }
            Self::InvalidImageOffset { axis } => {
                write!(f, "lattice image offset exceeds numeric envelope on axis {axis}")
            }
            Self::OffsetOnNonPeriodicAxis { axis } => {
                write!(f, "nonzero lattice offset on non-periodic basis axis {axis}")
            }
            Self::NumericRange => write!(f, "lattice geometry exceeded the finite numeric range"),
            Self::SearchBudgetExceeded { limit } => {
                write!(f, "closest-image search exceeded candidate budget {limit}")
            }
            Self::CandidateCountOverflow => write!(f, "closest-image candidate count overflow"),
        }
    }
}

impl Error for LatticeDomainError {}

/// Canonical point plus the periodic image from which it was reduced.
///
/// original = position + basis * image_offset up to floating-point error.
#[derive(Clone, Debug, PartialEq)]
pub struct LatticeWrappedPoint<const D: usize> {
    pub position: SVector<f64, D>,
    pub image_offset: [i64; D],
}

/// Closest periodic representative of a displacement.
///
/// The convention is displacement = minimum + basis * image_offset.
/// is_unique == false means multiple lattice images were tied within the
/// documented floating-point distance tolerance; callers must apply an
/// explicit policy before using the normal/direction in a solver.
#[derive(Clone, Debug, PartialEq)]
pub struct MinimumImage<const D: usize> {
    pub displacement: SVector<f64, D>,
    pub image_offset: [i64; D],
    pub candidates_examined: usize,
    pub is_unique: bool,
}

/// Full-rank lattice cell with an explicit periodic subspace.
///
/// basis is a D×D matrix whose columns are the cell basis vectors.
/// periodic_axes[i] == true enables integer translations along basis column i.
/// Non-periodic fractional coordinates are never translated or wrapped.
///
/// A Frobenius condition proxy ||B||_F ||B^-1||_F bounds numerical sensitivity.
/// This is a conservative screening policy, not a formal forward-error bound.
#[derive(Clone, Debug, PartialEq)]
pub struct LatticePeriodicDomain<const D: usize> {
    origin: SVector<f64, D>,
    basis: SMatrix<f64, D, D>,
    inverse_basis: SMatrix<f64, D, D>,
    periodic_axes: [bool; D],
    max_search_candidates: usize,
}

impl<const D: usize> LatticePeriodicDomain<D> {
    pub fn new(
        origin: SVector<f64, D>,
        basis: SMatrix<f64, D, D>,
        periodic_axes: [bool; D],
        max_search_candidates: usize,
    ) -> Result<Self, LatticeDomainError> {
        if D == 0 {
            return Err(LatticeDomainError::InvalidDimension);
        }
        if max_search_candidates == 0 {
            return Err(LatticeDomainError::InvalidSearchBudget);
        }
        if !origin.iter().all(|value| value.is_finite()) {
            return Err(LatticeDomainError::NonFiniteOrigin);
        }
        if !basis.iter().all(|value| value.is_finite()) {
            return Err(LatticeDomainError::NonFiniteBasis);
        }

        let inverse_basis = basis.try_inverse().ok_or(LatticeDomainError::SingularBasis)?;
        if !inverse_basis.iter().all(|value| value.is_finite()) {
            return Err(LatticeDomainError::IllConditionedBasis);
        }

        let condition_proxy = basis.norm() * inverse_basis.norm();
        if !condition_proxy.is_finite() || condition_proxy > MAX_CONDITION_PROXY {
            return Err(LatticeDomainError::IllConditionedBasis);
        }

        Ok(Self {
            origin,
            basis,
            inverse_basis,
            periodic_axes,
            max_search_candidates,
        })
    }

    pub fn origin(&self) -> &SVector<f64, D> {
        &self.origin
    }

    /// Matrix columns are the lattice basis vectors.
    pub fn basis(&self) -> &SMatrix<f64, D, D> {
        &self.basis
    }

    pub fn inverse_basis(&self) -> &SMatrix<f64, D, D> {
        &self.inverse_basis
    }

    pub fn periodic_axes(&self) -> &[bool; D] {
        &self.periodic_axes
    }

    pub fn max_search_candidates(&self) -> usize {
        self.max_search_candidates
    }

    /// Convert world coordinates to fractional lattice coordinates.
    pub fn fractional_coordinates(
        &self,
        point: &SVector<f64, D>,
    ) -> Result<SVector<f64, D>, LatticeDomainError> {
        if !point.iter().all(|value| value.is_finite()) {
            return Err(LatticeDomainError::NonFiniteCoordinate);
        }
        let fractional = self.inverse_basis * (point - self.origin);
        if !fractional.iter().all(|value| value.is_finite()) {
            return Err(LatticeDomainError::NumericRange);
        }
        Ok(fractional)
    }

    /// Wrap enabled fractional coordinates into the half-open interval [0, 1).
    ///
    /// This is a coordinate reduction, not a nearest-image operation. In
    /// particular, it is valid for a skewed basis without asserting that
    /// independently rounded fractional coordinates give the closest image.
    pub fn wrap_point(
        &self,
        point: &SVector<f64, D>,
    ) -> Result<LatticeWrappedPoint<D>, LatticeDomainError> {
        let fractional = self.fractional_coordinates(point)?;
        let mut canonical_fractional = fractional;
        let mut offsets = [0_i64; D];

        for axis in 0..D {
            if !self.periodic_axes[axis] {
                continue;
            }
            let coordinate = fractional[axis];
            if coordinate.abs() >= MAX_EXACT_LATTICE_INDEX {
                return Err(LatticeDomainError::FractionalCoordinatePrecisionLimit { axis });
            }

            let image = coordinate.floor();
            offsets[axis] = image as i64;
            let wrapped = coordinate.rem_euclid(1.0);
            if !wrapped.is_finite() || !(0.0..1.0).contains(&wrapped) {
                return Err(LatticeDomainError::NumericRange);
            }
            canonical_fractional[axis] = if wrapped == 0.0 { 0.0 } else { wrapped };
        }

        let position = self.origin + self.basis * canonical_fractional;
        if !position.iter().all(|value| value.is_finite()) {
            return Err(LatticeDomainError::NumericRange);
        }

        Ok(LatticeWrappedPoint {
            position,
            image_offset: offsets,
        })
    }

    /// Convert an integer lattice offset into a world-space translation.
    pub fn offset_vector(
        &self,
        offset: [i64; D],
    ) -> Result<SVector<f64, D>, LatticeDomainError> {
        let mut fractional = SVector::<f64, D>::zeros();
        for axis in 0..D {
            let count = offset[axis];
            if count == 0 {
                continue;
            }
            if !self.periodic_axes[axis] {
                return Err(LatticeDomainError::OffsetOnNonPeriodicAxis { axis });
            }
            let count_f64 = count as f64;
            if count_f64.abs() >= MAX_EXACT_LATTICE_INDEX {
                return Err(LatticeDomainError::InvalidImageOffset { axis });
            }
            fractional[axis] = count_f64;
        }

        let translation = self.basis * fractional;
        if !translation.iter().all(|value| value.is_finite()) {
            return Err(LatticeDomainError::NumericRange);
        }
        Ok(translation)
    }

    /// Apply an explicit lattice translation to a point.
    pub fn apply_offset(
        &self,
        point: &SVector<f64, D>,
        offset: [i64; D],
    ) -> Result<SVector<f64, D>, LatticeDomainError> {
        if !point.iter().all(|value| value.is_finite()) {
            return Err(LatticeDomainError::NonFiniteCoordinate);
        }
        let translated = point + self.offset_vector(offset)?;
        if !translated.iter().all(|value| value.is_finite()) {
            return Err(LatticeDomainError::NumericRange);
        }
        Ok(translated)
    }

    /// Solve the closest-lattice-vector problem for a displacement.
    ///
    /// For d = B f, find periodic integer n minimizing ||d - B n||. A
    /// component-wise fractional rounding is used only to produce an initial
    /// finite radius. The complete candidate interval is then bounded using
    /// rows of B^-1:
    ///
    /// |f_i - n_i| = |row_i(B^-1) · residual|
    ///              <= ||row_i(B^-1)|| ||residual||.
    ///
    /// Every candidate that can improve the initial radius is enumerated.
    /// If the resulting search exceeds the configured budget, this returns an
    /// error; it never returns the initial estimate as though it were exact.
    pub fn minimum_image_displacement(
        &self,
        displacement: &SVector<f64, D>,
    ) -> Result<MinimumImage<D>, LatticeDomainError> {
        if !displacement.iter().all(|value| value.is_finite()) {
            return Err(LatticeDomainError::NonFiniteCoordinate);
        }

        let fractional = self.inverse_basis * displacement;
        if !fractional.iter().all(|value| value.is_finite()) {
            return Err(LatticeDomainError::NumericRange);
        }

        let mut initial_offset = [0_i64; D];
        let mut initial_fractional = SVector::<f64, D>::zeros();
        for axis in 0..D {
            if !self.periodic_axes[axis] {
                continue;
            }
            if fractional[axis].abs() >= MAX_EXACT_LATTICE_INDEX {
                return Err(LatticeDomainError::FractionalCoordinatePrecisionLimit { axis });
            }
            let rounded = fractional[axis].round();
            initial_offset[axis] = rounded as i64;
            initial_fractional[axis] = rounded;
        }

        let initial_shift = self.basis * initial_fractional;
        let initial_residual = displacement - initial_shift;
        let initial_distance_squared = initial_residual.norm_squared();
        if !initial_distance_squared.is_finite() {
            return Err(LatticeDomainError::NumericRange);
        }

        // Any better (or tied) candidate must lie within this initial radius.
        let radius = initial_distance_squared.sqrt();
        if !radius.is_finite() {
            return Err(LatticeDomainError::NumericRange);
        }

        let mut ranges = [(0_i64, 0_i64); D];
        let mut total_candidates = 1_u128;
        for axis in 0..D {
            if !self.periodic_axes[axis] {
                continue;
            }

            let row_norm = self.inverse_basis.row(axis).norm();
            let bound = row_norm * radius;
            if !row_norm.is_finite() || !bound.is_finite() {
                return Err(LatticeDomainError::NumericRange);
            }

            let scale = fractional[axis].abs().max(bound.abs()).max(1.0);
            let padding = OUTWARD_ROUNDING_EPSILON_MULTIPLIER * f64::EPSILON * scale;
            let lower_real = fractional[axis] - bound - padding;
            let upper_real = fractional[axis] + bound + padding;
            if !lower_real.is_finite()
                || !upper_real.is_finite()
                || lower_real.abs() >= MAX_EXACT_LATTICE_INDEX
                || upper_real.abs() >= MAX_EXACT_LATTICE_INDEX
            {
                return Err(LatticeDomainError::FractionalCoordinatePrecisionLimit { axis });
            }

            let mut lower = lower_real.ceil() as i64;
            let mut upper = upper_real.floor() as i64;
            // Keep the initial candidate even if roundoff placed it one ULP
            // outside the mathematically conservative interval.
            lower = lower.min(initial_offset[axis]);
            upper = upper.max(initial_offset[axis]);
            if lower > upper {
                return Err(LatticeDomainError::NumericRange);
            }
            ranges[axis] = (lower, upper);

            let count = (upper as i128 - lower as i128 + 1) as u128;
            total_candidates = total_candidates
                .checked_mul(count)
                .ok_or(LatticeDomainError::CandidateCountOverflow)?;
            if total_candidates > self.max_search_candidates as u128 {
                return Err(LatticeDomainError::SearchBudgetExceeded {
                    limit: self.max_search_candidates,
                });
            }
        }

        let mut offsets = Vec::with_capacity(total_candidates as usize);
        offsets.push([0_i64; D]);
        for axis in 0..D {
            let (lower, upper) = ranges[axis];
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

        let mut best_offset = offsets[0];
        let mut best_residual = displacement - self.offset_vector(best_offset)?;
        let mut best_distance_squared = best_residual.norm_squared();
        if !best_distance_squared.is_finite() {
            return Err(LatticeDomainError::NumericRange);
        }
        let mut is_unique = true;

        for offset in offsets.iter().copied().skip(1) {
            let residual = displacement - self.offset_vector(offset)?;
            let distance_squared = residual.norm_squared();
            if !distance_squared.is_finite() {
                return Err(LatticeDomainError::NumericRange);
            }

            let comparison_scale = best_distance_squared
                .abs()
                .max(distance_squared.abs())
                .max(f64::MIN_POSITIVE);
            let tie_tolerance = DISTANCE_TIE_EPSILON_MULTIPLIER
                * f64::EPSILON
                * comparison_scale;

            if distance_squared < best_distance_squared - tie_tolerance {
                best_offset = offset;
                best_residual = residual;
                best_distance_squared = distance_squared;
                is_unique = true;
            } else if (distance_squared - best_distance_squared).abs() <= tie_tolerance {
                if offset != best_offset {
                    is_unique = false;
                }
                if tie_key_precedes(&residual, &best_residual, displacement) {
                    best_offset = offset;
                    best_residual = residual;
                    best_distance_squared = distance_squared;
                }
            }
        }

        Ok(MinimumImage {
            displacement: best_residual,
            image_offset: best_offset,
            candidates_examined: offsets.len(),
            is_unique,
        })
    }
}

/// Tie key is residual multiplied by the sign of the first nonzero displacement
/// component. This maps reversed displacements to reversed selected residuals,
/// preserving oddness at symmetric half-cell ties.
fn tie_key_precedes<const D: usize>(
    candidate: &SVector<f64, D>,
    current: &SVector<f64, D>,
    original: &SVector<f64, D>,
) -> bool {
    let sign = original
        .iter()
        .copied()
        .find(|value| *value != 0.0)
        .map(f64::signum)
        .unwrap_or(1.0);

    for axis in 0..D {
        let candidate_key = candidate[axis] * sign;
        let current_key = current[axis] * sign;
        match candidate_key.total_cmp(&current_key) {
            std::cmp::Ordering::Less => return true,
            std::cmp::Ordering::Greater => return false,
            std::cmp::Ordering::Equal => {}
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{LatticeDomainError, LatticePeriodicDomain};
    use nalgebra::{SMatrix, SVector};

    fn close(a: f64, b: f64, tolerance: f64) {
        assert!(
            (a - b).abs() <= tolerance,
            "expected {a:.16e} ~= {b:.16e}; tolerance {tolerance:.3e}"
        );
    }

    fn skew_2d() -> LatticePeriodicDomain<2> {
        // Columns: a=(10,0), b=(3,8).
        LatticePeriodicDomain::new(
            SVector::from([2.0, -1.0]),
            SMatrix::new(10.0, 3.0, 0.0, 8.0),
            [true, true],
            4096,
        )
        .unwrap()
    }

    #[test]
    fn skewed_wrap_reconstructs_original_with_integer_lattice_offset() {
        let domain = skew_2d();
        let original = SVector::from([13.25, -3.0]);
        let wrapped = domain.wrap_point(&original).unwrap();

        assert_eq!(wrapped.image_offset, [1, -1]);
        close(wrapped.position[0], 6.25, 1e-12);
        close(wrapped.position[1], 5.0, 1e-12);

        let restored = domain
            .apply_offset(&wrapped.position, wrapped.image_offset)
            .unwrap();
        close(restored[0], original[0], 1e-12);
        close(restored[1], original[1], 1e-12);
    }

    #[test]
    fn minimum_image_does_not_assume_componentwise_fractional_rounding_is_closest() {
        // The two basis vectors are strongly skewed but the basis remains
        // well-conditioned enough for the declared envelope.
        let domain = LatticePeriodicDomain::new(
            SVector::zeros(),
            SMatrix::new(1.0, 0.9, 0.0, 0.2),
            [true, true],
            4096,
        )
        .unwrap();
        let displacement = SVector::from([0.931, 0.098]);
        let image = domain.minimum_image_displacement(&displacement).unwrap();

        assert_eq!(image.image_offset, [1, 0]);
        assert!(image.is_unique);
        assert!(image.displacement.norm() < 0.13);
        assert!(image.candidates_examined > 1);
    }

    #[test]
    fn closest_image_matches_independent_brute_force_on_skewed_2d_corpus() {
        let domain = skew_2d();
        let mut seed = 0x7A31_9B2D_u64;

        for _case in 0..1000 {
            let mut next = || {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                let unit = (seed as f64) / (u64::MAX as f64);
                (unit * 80.0) - 40.0
            };
            let displacement = SVector::from([next(), next()]);
            let image = domain.minimum_image_displacement(&displacement).unwrap();

            let mut oracle_distance_squared = f64::INFINITY;
            let mut oracle_displacement = SVector::<f64, 2>::zeros();
            for i in -8_i64..=8 {
                for j in -8_i64..=8 {
                    let shift = domain
                        .offset_vector([i, j])
                        .expect("bounded oracle offset");
                    let candidate = displacement - shift;
                    let distance_squared = candidate.norm_squared();
                    if distance_squared < oracle_distance_squared {
                        oracle_distance_squared = distance_squared;
                        oracle_displacement = candidate;
                    }
                }
            }

            close(
                image.displacement.norm_squared(),
                oracle_distance_squared,
                2e-10 * oracle_distance_squared.max(1.0),
            );
            assert!(
                image.displacement.norm() <= oracle_displacement.norm() + 1e-10,
                "bounded exact search lost to brute-force oracle"
            );
        }
    }

    #[test]
    fn exact_half_cell_tie_is_reported_ambiguous_and_reversal_odd() {
        let domain = LatticePeriodicDomain::new(
            SVector::zeros(),
            SMatrix::new(10.0, 0.0, 0.0, 10.0),
            [true, true],
            128,
        )
        .unwrap();
        let displacement = SVector::from([5.0, 1.0]);
        let forward = domain.minimum_image_displacement(&displacement).unwrap();
        let reverse = domain
            .minimum_image_displacement(&(-displacement))
            .unwrap();

        assert!(!forward.is_unique);
        assert!(!reverse.is_unique);
        close(forward.displacement[0], -reverse.displacement[0], 1e-12);
        close(forward.displacement[1], -reverse.displacement[1], 1e-12);
    }

    #[test]
    fn basis_change_by_unimodular_matrix_preserves_physical_nearest_image() {
        let basis = SMatrix::<f64, 2, 2>::new(10.0, 3.0, 0.0, 8.0);
        // U is integer and unimodular (det=1), so B and B*U generate
        // exactly the same lattice even though their basis coordinates differ.
        let unimodular = SMatrix::<f64, 2, 2>::new(1.0, 1.0, 0.0, 1.0);
        let changed_basis = basis * unimodular;
        let original = LatticePeriodicDomain::new(
            SVector::zeros(),
            basis,
            [true, true],
            8192,
        )
        .unwrap();
        let changed = LatticePeriodicDomain::new(
            SVector::zeros(),
            changed_basis,
            [true, true],
            8192,
        )
        .unwrap();

        let displacement = SVector::from([17.3, 11.1]);
        let a = original.minimum_image_displacement(&displacement).unwrap();
        let b = changed.minimum_image_displacement(&displacement).unwrap();
        close((a.displacement - b.displacement).norm(), 0.0, 1e-10);
        close(a.displacement.norm(), b.displacement.norm(), 1e-10);
    }

    #[test]
    fn mixed_periodic_subspace_never_translates_nonperiodic_basis_axis() {
        let domain = LatticePeriodicDomain::new(
            SVector::zeros(),
            SMatrix::new(4.0, 1.0, 0.0, 3.0),
            [true, false],
            1024,
        )
        .unwrap();
        let wrapped = domain
            .wrap_point(&SVector::from([9.0, 6.0]))
            .unwrap();
        assert_eq!(wrapped.image_offset[1], 0);

        assert_eq!(
            domain.offset_vector([0, 1]),
            Err(LatticeDomainError::OffsetOnNonPeriodicAxis { axis: 1 })
        );
    }

    #[test]
    fn excessive_closest_image_search_fails_closed() {
        let domain = LatticePeriodicDomain::new(
            SVector::zeros(),
            SMatrix::new(1.0, 0.99, 0.0, 0.1),
            [true, true],
            1,
        )
        .unwrap();
        let result = domain.minimum_image_displacement(&SVector::from([0.3, 0.07]));
        assert!(matches!(
            result,
            Err(LatticeDomainError::SearchBudgetExceeded { limit: 1 })
        ));
    }

    #[test]
    fn singular_and_non_finite_bases_are_rejected() {
        assert_eq!(
            LatticePeriodicDomain::new(
                SVector::zeros(),
                SMatrix::new(1.0, 2.0, 2.0, 4.0),
                [true, true],
                128,
            ),
            Err(LatticeDomainError::SingularBasis)
        );

        assert_eq!(
            LatticePeriodicDomain::new(
                SVector::zeros(),
                SMatrix::new(f64::NAN, 0.0, 0.0, 1.0),
                [true, true],
                128,
            ),
            Err(LatticeDomainError::NonFiniteBasis)
        );
    }
}
