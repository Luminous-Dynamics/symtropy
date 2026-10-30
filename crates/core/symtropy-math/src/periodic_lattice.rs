// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Triclinic periodic cells and bounded closest-lattice-image queries.
//!
//! The lattice basis stores cell vectors as matrix columns. Fractional
//! component rounding is sufficient to wrap a point into a fundamental
//! parallelepiped, but it is not generally sufficient to find the nearest
//! Cartesian image for a skew basis. Minimum-image queries therefore enumerate
//! a conservative bounded set of lattice offsets around a Babai incumbent.
//!
//! This module is geometry-only. It is not wired into broadphase, GJK, CCD,
//! molecular dynamics, or PhysicsWorld.

use std::error::Error;
use std::fmt;

use nalgebra::{SMatrix, SVector};

use crate::periodic::LatticeOffset;

const MAX_EXACT_LATTICE_INDEX: f64 = 4_503_599_627_370_496.0; // 2^52
const OUTWARD_ROUNDING_ULPS: f64 = 8.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriclinicCellError {
    InvalidDimension,
    NonFiniteOrigin { axis: usize },
    NonFiniteBasis { row: usize, column: usize },
    SingularBasis,
    NonFiniteCoordinate { axis: usize },
    CoordinatePrecisionLimit { axis: usize },
    InvalidImageOffset { axis: usize },
    OffsetOnNonPeriodicAxis { axis: usize },
    InvalidCandidateBudget,
    CandidateBudgetExceeded { candidates: usize, limit: usize },
    NumericRange,
}

impl fmt::Display for TriclinicCellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDimension => write!(f, "triclinic periodic cell requires D >= 1"),
            Self::NonFiniteOrigin { axis } => {
                write!(f, "non-finite triclinic cell origin on axis {axis}")
            }
            Self::NonFiniteBasis { row, column } => {
                write!(f, "non-finite lattice basis entry ({row}, {column})")
            }
            Self::SingularBasis => write!(f, "triclinic lattice basis is singular or non-invertible"),
            Self::NonFiniteCoordinate { axis } => {
                write!(f, "non-finite triclinic coordinate on axis {axis}")
            }
            Self::CoordinatePrecisionLimit { axis } => {
                write!(f, "triclinic fractional coordinate exceeds precision envelope on axis {axis}")
            }
            Self::InvalidImageOffset { axis } => {
                write!(f, "triclinic image offset exceeds numeric envelope on axis {axis}")
            }
            Self::OffsetOnNonPeriodicAxis { axis } => {
                write!(f, "nonzero triclinic image offset on non-periodic axis {axis}")
            }
            Self::InvalidCandidateBudget => write!(f, "closest-lattice candidate budget must be positive"),
            Self::CandidateBudgetExceeded { candidates, limit } => write!(
                f,
                "closest-lattice query requires {candidates} candidates, exceeding budget {limit}"
            ),
            Self::NumericRange => write!(f, "triclinic periodic arithmetic exceeded finite numeric range"),
        }
    }
}

impl Error for TriclinicCellError {}

#[derive(Clone, Debug, PartialEq)]
pub struct TriclinicWrappedPoint<const D: usize> {
    pub position: SVector<f64, D>,
    /// Original point = canonical point + basis * image_offset.
    pub image_offset: LatticeOffset<D>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MinimumImage<const D: usize> {
    /// The selected Cartesian displacement d + basis * image_offset.
    pub displacement: SVector<f64, D>,
    /// Integer lattice translation added to the original displacement.
    pub image_offset: LatticeOffset<D>,
    /// Number of lattice images examined by the bounded closest-vector search.
    pub candidates_examined: usize,
}

/// A full-rank lattice cell with independently selectable periodic basis axes.
///
/// Matrix columns are lattice vectors. The periodic mask selects the columns
/// that generate the quotient lattice; non-periodic fractional coordinates
/// are never translated or wrapped.
#[derive(Clone, Debug, PartialEq)]
pub struct TriclinicPeriodicCell<const D: usize> {
    origin: SVector<f64, D>,
    basis: SMatrix<f64, D, D>,
    inverse_basis: SMatrix<f64, D, D>,
    periodic_axes: [bool; D],
}

impl<const D: usize> TriclinicPeriodicCell<D> {
    pub fn new(
        origin: SVector<f64, D>,
        basis: SMatrix<f64, D, D>,
        periodic_axes: [bool; D],
    ) -> Result<Self, TriclinicCellError> {
        if D == 0 {
            return Err(TriclinicCellError::InvalidDimension);
        }
        for axis in 0..D {
            if !origin[axis].is_finite() {
                return Err(TriclinicCellError::NonFiniteOrigin { axis });
            }
        }
        for row in 0..D {
            for column in 0..D {
                if !basis[(row, column)].is_finite() {
                    return Err(TriclinicCellError::NonFiniteBasis { row, column });
                }
            }
        }

        let inverse_basis = basis
            .try_inverse()
            .ok_or(TriclinicCellError::SingularBasis)?;
        if !inverse_basis.iter().all(|value| value.is_finite()) {
            return Err(TriclinicCellError::SingularBasis);
        }

        for axis in 0..D {
            let column_norm = basis.column(axis).norm();
            if !column_norm.is_finite() || column_norm <= 0.0 {
                return Err(TriclinicCellError::SingularBasis);
            }
        }

        Ok(Self {
            origin,
            basis,
            inverse_basis,
            periodic_axes,
        })
    }

    pub fn origin(&self) -> &SVector<f64, D> {
        &self.origin
    }

    /// Lattice basis, with lattice vectors stored as columns.
    pub fn basis(&self) -> &SMatrix<f64, D, D> {
        &self.basis
    }

    pub fn periodic_axes(&self) -> &[bool; D] {
        &self.periodic_axes
    }

    fn fractional_coordinates(
        &self,
        displacement: &SVector<f64, D>,
    ) -> Result<SVector<f64, D>, TriclinicCellError> {
        for axis in 0..D {
            if !displacement[axis].is_finite() {
                return Err(TriclinicCellError::NonFiniteCoordinate { axis });
            }
        }
        let fractional = self.inverse_basis * *displacement;
        for axis in 0..D {
            if !fractional[axis].is_finite() {
                return Err(TriclinicCellError::CoordinatePrecisionLimit { axis });
            }
        }
        Ok(fractional)
    }

    /// Wrap a Cartesian point into the fundamental parallelepiped.
    ///
    /// Only active periodic fractional coordinates are reduced into [0, 1).
    /// The returned integer image offset reconstructs the original point.
    pub fn wrap_point(
        &self,
        point: &SVector<f64, D>,
    ) -> Result<TriclinicWrappedPoint<D>, TriclinicCellError> {
        for axis in 0..D {
            if !point[axis].is_finite() {
                return Err(TriclinicCellError::NonFiniteCoordinate { axis });
            }
        }

        let relative = *point - self.origin;
        let mut fractional = self.fractional_coordinates(&relative)?;
        let mut image = [0_i64; D];

        for axis in 0..D {
            if !self.periodic_axes[axis] {
                continue;
            }
            let quotient = fractional[axis];
            if quotient.abs() >= MAX_EXACT_LATTICE_INDEX {
                return Err(TriclinicCellError::CoordinatePrecisionLimit { axis });
            }
            let integer = quotient.floor();
            if integer < i64::MIN as f64 || integer > i64::MAX as f64 {
                return Err(TriclinicCellError::CoordinatePrecisionLimit { axis });
            }
            image[axis] = integer as i64;
            fractional[axis] = quotient - integer;

            if !fractional[axis].is_finite()
                || fractional[axis] < 0.0
                || fractional[axis] >= 1.0
            {
                return Err(TriclinicCellError::CoordinatePrecisionLimit { axis });
            }
        }

        let canonical = self.origin + self.basis * fractional;
        if !canonical.iter().all(|value| value.is_finite()) {
            return Err(TriclinicCellError::NumericRange);
        }

        Ok(TriclinicWrappedPoint {
            position: canonical,
            image_offset: LatticeOffset::new(image),
        })
    }

    /// Convert a lattice offset into its Cartesian translation.
    pub fn offset_vector(
        &self,
        offset: LatticeOffset<D>,
    ) -> Result<SVector<f64, D>, TriclinicCellError> {
        let mut coefficients = SVector::<f64, D>::zeros();
        for axis in 0..D {
            let integer = offset.periods[axis];
            if integer == 0 {
                continue;
            }
            if !self.periodic_axes[axis] {
                return Err(TriclinicCellError::OffsetOnNonPeriodicAxis { axis });
            }
            let value = integer as f64;
            if value.abs() >= MAX_EXACT_LATTICE_INDEX {
                return Err(TriclinicCellError::InvalidImageOffset { axis });
            }
            coefficients[axis] = value;
        }

        let translation = self.basis * coefficients;
        if !translation.iter().all(|value| value.is_finite()) {
            return Err(TriclinicCellError::InvalidImageOffset { axis: 0 });
        }
        Ok(translation)
    }

    pub fn apply_offset(
        &self,
        point: &SVector<f64, D>,
        offset: LatticeOffset<D>,
    ) -> Result<SVector<f64, D>, TriclinicCellError> {
        if !point.iter().all(|value| value.is_finite()) {
            let axis = point.iter().position(|value| !value.is_finite()).unwrap_or(0);
            return Err(TriclinicCellError::NonFiniteCoordinate { axis });
        }
        let translated = *point + self.offset_vector(offset)?;
        if !translated.iter().all(|value| value.is_finite()) {
            return Err(TriclinicCellError::NumericRange);
        }
        Ok(translated)
    }

    /// Find the nearest Cartesian lattice image by a bounded closest-vector search.
    ///
    /// The Babai-rounded fractional offset supplies an initial upper bound.
    /// For any candidate y = d + B*n with ||y|| <= R, each fractional
    /// component satisfies |(B^-1 y)_i| <= ||row_i(B^-1)|| * R. This yields a
    /// finite conservative integer interval for every periodic offset.
    /// Outward-rounded intervals are exhaustively enumerated. If their
    /// Cartesian product exceeds max_candidates, the query returns a typed
    /// error rather than silently using the Babai approximation.
    pub fn minimum_image(
        &self,
        displacement: &SVector<f64, D>,
        max_candidates: usize,
    ) -> Result<MinimumImage<D>, TriclinicCellError> {
        if max_candidates == 0 {
            return Err(TriclinicCellError::InvalidCandidateBudget);
        }

        let fractional = self.fractional_coordinates(displacement)?;
        let mut babai = [0_i64; D];
        for axis in 0..D {
            if !self.periodic_axes[axis] {
                continue;
            }
            if fractional[axis].abs() >= MAX_EXACT_LATTICE_INDEX {
                return Err(TriclinicCellError::CoordinatePrecisionLimit { axis });
            }
            let rounded = -fractional[axis].round();
            if rounded.abs() >= MAX_EXACT_LATTICE_INDEX
                || rounded < i64::MIN as f64
                || rounded > i64::MAX as f64
            {
                return Err(TriclinicCellError::CoordinatePrecisionLimit { axis });
            }
            babai[axis] = rounded as i64;
        }

        let babai_offset = LatticeOffset::new(babai);
        let babai_translation = self.offset_vector(babai_offset)?;
        let babai_displacement = *displacement + babai_translation;
        let mut best_score = babai_displacement.norm_squared();
        if !best_score.is_finite() {
            return Err(TriclinicCellError::NumericRange);
        }
        let mut best_offset = babai;
        let best_radius = best_score.sqrt();

        let mut lower = [0_i64; D];
        let mut upper = [0_i64; D];
        let mut candidate_count = 1_usize;

        for axis in 0..D {
            if !self.periodic_axes[axis] {
                lower[axis] = 0;
                upper[axis] = 0;
                continue;
            }

            let row_norm = self.inverse_basis.row(axis).norm();
            let radius_bound = row_norm * best_radius;
            if !row_norm.is_finite() || !radius_bound.is_finite() {
                return Err(TriclinicCellError::NumericRange);
            }
            let magnitude = fractional[axis]
                .abs()
                .max(radius_bound.abs())
                .max(1.0);
            let padding = OUTWARD_ROUNDING_ULPS * f64::EPSILON * magnitude;
            let low_real = -fractional[axis] - radius_bound - padding;
            let high_real = -fractional[axis] + radius_bound + padding;
            if !low_real.is_finite() || !high_real.is_finite() {
                return Err(TriclinicCellError::NumericRange);
            }
            let low = low_real.ceil();
            let high = high_real.floor();
            if low < i64::MIN as f64
                || high > i64::MAX as f64
                || low.abs() >= MAX_EXACT_LATTICE_INDEX
                || high.abs() >= MAX_EXACT_LATTICE_INDEX
            {
                return Err(TriclinicCellError::CoordinatePrecisionLimit { axis });
            }

            lower[axis] = (low as i64).min(babai[axis]);
            upper[axis] = (high as i64).max(babai[axis]);
            let width_i128 = upper[axis] as i128 - lower[axis] as i128 + 1;
            if width_i128 <= 0 || width_i128 > usize::MAX as i128 {
                return Err(TriclinicCellError::CandidateBudgetExceeded {
                    candidates: usize::MAX,
                    limit: max_candidates,
                });
            }
            let width = width_i128 as usize;
            let Some(next_count) = candidate_count.checked_mul(width) else {
                return Err(TriclinicCellError::CandidateBudgetExceeded {
                    candidates: usize::MAX,
                    limit: max_candidates,
                });
            };
            candidate_count = next_count;
            if candidate_count > max_candidates {
                return Err(TriclinicCellError::CandidateBudgetExceeded {
                    candidates: candidate_count,
                    limit: max_candidates,
                });
            }
        }

        let mut current = lower;
        let mut examined = 0_usize;
        loop {
            let offset = LatticeOffset::new(current);
            let translation = self.offset_vector(offset)?;
            let candidate = *displacement + translation;
            let score = candidate.norm_squared();
            if !score.is_finite() {
                return Err(TriclinicCellError::NumericRange);
            }
            examined += 1;

            if score < best_score || (score == best_score && current < best_offset) {
                best_score = score;
                best_offset = current;
            }

            if !increment_lattice_offset(&mut current, &lower, &upper) {
                break;
            }
        }

        let best_translation = self.offset_vector(LatticeOffset::new(best_offset))?;
        let nearest = *displacement + best_translation;
        if !nearest.iter().all(|value| value.is_finite()) {
            return Err(TriclinicCellError::NumericRange);
        }

        Ok(MinimumImage {
            displacement: nearest,
            image_offset: LatticeOffset::new(best_offset),
            candidates_examined: examined,
        })
    }

    pub fn minimum_image_displacement(
        &self,
        displacement: &SVector<f64, D>,
        max_candidates: usize,
    ) -> Result<SVector<f64, D>, TriclinicCellError> {
        Ok(self.minimum_image(displacement, max_candidates)?.displacement)
    }
}

fn increment_lattice_offset<const D: usize>(
    current: &mut [i64; D],
    lower: &[i64; D],
    upper: &[i64; D],
) -> bool {
    for axis in (0..D).rev() {
        if current[axis] < upper[axis] {
            current[axis] += 1;
            for reset_axis in (axis + 1)..D {
                current[reset_axis] = lower[reset_axis];
            }
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{TriclinicCellError, TriclinicPeriodicCell};
    use crate::periodic::{LatticeOffset, PeriodicBox};
    use nalgebra::{SMatrix, SVector};

    fn skew_cell() -> TriclinicPeriodicCell<2> {
        let basis = SMatrix::<f64, 2, 2>::new(2.0, 1.8, 0.0, 0.5);
        TriclinicPeriodicCell::new(SVector::from([3.0, -2.0]), basis, [true, true])
            .unwrap()
    }

    fn close(a: f64, b: f64, tolerance: f64) {
        assert!(
            (a - b).abs() <= tolerance,
            "expected {a:.16e} ~= {b:.16e} within {tolerance:.3e}"
        );
    }

    fn lcg_next(state: &mut u64) -> u64 {
        *state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        *state
    }

    fn unit(state: &mut u64) -> f64 {
        (lcg_next(state) >> 11) as f64 / ((1_u64 << 53) as f64)
    }

    fn range(state: &mut u64, low: f64, high: f64) -> f64 {
        low + (high - low) * unit(state)
    }

    #[test]
    fn skew_basis_requires_cartesian_closest_image_not_component_rounding() {
        let cell = skew_cell();
        let fractional = SVector::from([0.49, 0.49]);
        let displacement = cell.basis() * fractional;

        let result = cell.minimum_image(&displacement, 128).unwrap();
        assert_eq!(result.image_offset, LatticeOffset::new([0, -1]));
        assert!(result.displacement.norm() < 0.27);

        // Independent comparison with the component-wise Babai choice n=(0,0).
        assert!(result.displacement.norm() < displacement.norm());
    }

    #[test]
    fn wrapping_and_explicit_offset_reconstruct_cartesian_point() {
        let cell = skew_cell();
        let fractional = SVector::from([2.25, -1.75]);
        let point = *cell.origin() + *cell.basis() * fractional;
        let wrapped = cell.wrap_point(&point).unwrap();

        assert_eq!(wrapped.image_offset, LatticeOffset::new([2, -2]));
        let restored = cell
            .apply_offset(&wrapped.position, wrapped.image_offset)
            .unwrap();
        for axis in 0..2 {
            close(restored[axis], point[axis], 1e-12);
        }

        let twice = cell.wrap_point(&wrapped.position).unwrap();
        assert_eq!(twice.image_offset, LatticeOffset::zero());
        assert!((twice.position - wrapped.position).norm() < 1e-12);
    }

    #[test]
    fn closest_image_matches_independent_integer_lattice_search() {
        let cell = skew_cell();
        let mut seed = 0x5452_4943_4c49_4e49_u64;

        for case in 0..500 {
            let fractional = SVector::from([
                range(&mut seed, -1.5, 1.5),
                range(&mut seed, -1.5, 1.5),
            ]);
            let displacement = *cell.basis() * fractional;
            let actual = cell.minimum_image(&displacement, 256).unwrap();

            let mut best: Option<(f64, [i64; 2])> = None;
            for n0 in -8_i64..=8 {
                for n1 in -8_i64..=8 {
                    let candidate =
                        displacement + *cell.basis() * SVector::from([n0 as f64, n1 as f64]);
                    let score = candidate.norm_squared();
                    let offset = [n0, n1];
                    if best.is_none_or(|(best_score, best_offset)| {
                        score < best_score || (score == best_score && offset < best_offset)
                    }) {
                        best = Some((score, offset));
                    }
                }
            }
            let (oracle_score, oracle_offset) = best.expect("finite oracle search has candidates");
            close(actual.displacement.norm_squared(), oracle_score, 1e-11);
            assert_eq!(actual.image_offset.periods, oracle_offset, "case={case}");
        }
    }

    #[test]
    fn orthogonal_cell_matches_periodic_box_away_from_ties() {
        let origin = SVector::from([2.0, -3.0]);
        let periods = SVector::from([7.0, 4.0]);
        let orthogonal_basis = SMatrix::<f64, 2, 2>::new(7.0, 0.0, 0.0, 4.0);
        let triclinic =
            TriclinicPeriodicCell::new(origin, orthogonal_basis, [true, true]).unwrap();
        let orthogonal = PeriodicBox::new(origin, periods, [true, true]).unwrap();

        for displacement in [
            SVector::from([6.8, -3.8]),
            SVector::from([-5.2, 2.9]),
            SVector::from([14.1, 7.3]),
            SVector::from([-12.4, -8.2]),
        ] {
            let expected = orthogonal.minimum_image_displacement(&displacement).unwrap();
            let actual = triclinic.minimum_image_displacement(&displacement, 64).unwrap();
            assert!((actual - expected).norm() < 1e-11);
        }
    }

    #[test]
    fn rotation_covariance_preserves_lattice_offset_and_rotates_result() {
        let cell = skew_cell();
        let angle = 0.37_f64;
        let rotation = SMatrix::<f64, 2, 2>::new(
            angle.cos(),
            -angle.sin(),
            angle.sin(),
            angle.cos(),
        );
        let rotated_cell = TriclinicPeriodicCell::new(
            rotation * *cell.origin(),
            rotation * *cell.basis(),
            [true, true],
        )
        .unwrap();
        let displacement = *cell.basis() * SVector::from([0.31, -0.42]);

        let original = cell.minimum_image(&displacement, 256).unwrap();
        let rotated = rotated_cell
            .minimum_image(&(rotation * displacement), 256)
            .unwrap();

        assert_eq!(original.image_offset, rotated.image_offset);
        assert!((rotated.displacement - rotation * original.displacement).norm() < 1e-11);
    }

    #[test]
    fn partial_periodic_mask_only_translates_active_lattice_axes() {
        let cell = TriclinicPeriodicCell::new(
            SVector::zeros(),
            SMatrix::<f64, 2, 2>::new(2.0, 0.4, 0.0, 1.0),
            [true, false],
        )
        .unwrap();
        let offset = cell
            .offset_vector(LatticeOffset::new([2, 0]))
            .unwrap();
        assert_eq!(offset, SVector::from([4.0, 0.0]));
        assert!(matches!(
            cell.offset_vector(LatticeOffset::new([0, 1])),
            Err(TriclinicCellError::OffsetOnNonPeriodicAxis { axis: 1 })
        ));
    }

    #[test]
    fn candidate_budget_exhaustion_is_an_explicit_failure() {
        let cell = TriclinicPeriodicCell::new(
            SVector::zeros(),
            SMatrix::<f64, 2, 2>::identity(),
            [true, true],
        )
        .unwrap();
        assert!(matches!(
            cell.minimum_image(&SVector::from([0.5, 0.0]), 1),
            Err(TriclinicCellError::CandidateBudgetExceeded { limit: 1, .. })
        ));
    }

    #[test]
    fn equal_distance_ties_use_lexicographic_lattice_offset() {
        let cell = TriclinicPeriodicCell::new(
            SVector::zeros(),
            SMatrix::<f64, 2, 2>::identity(),
            [true, true],
        )
        .unwrap();
        let result = cell.minimum_image(&SVector::from([0.5, 0.0]), 16).unwrap();
        assert_eq!(result.image_offset, LatticeOffset::new([-1, 0]));
        close(result.displacement[0], -0.5, 1e-12);
    }

    #[test]
    fn rejects_singular_and_non_finite_cells() {
        assert_eq!(
            TriclinicPeriodicCell::new(
                SVector::zeros(),
                SMatrix::<f64, 2, 2>::new(1.0, 2.0, 2.0, 4.0),
                [true, true],
            ),
            Err(TriclinicCellError::SingularBasis)
        );
        assert!(matches!(
            TriclinicPeriodicCell::new(
                SVector::zeros(),
                SMatrix::<f64, 2, 2>::new(1.0, f64::NAN, 0.0, 1.0),
                [true, true],
            ),
            Err(TriclinicCellError::NonFiniteBasis { row: 0, column: 1 })
        ));
    }
}
