// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Triclinic/oblique periodic lattice primitives.
//!
//! Unlike PeriodicBox, this type represents a periodic cell using a full
//! invertible basis matrix. It supports D=1/2/3 and an arbitrary periodic-axis
//! mask in lattice coordinates. Minimum-image displacement solves the
//! closest-lattice-vector problem by bounded shell enumeration and returns
//! only after a conservative lower-bound certificate proves no unsearched
//! image can be closer. Budget exhaustion is explicit and fail-closed.

use std::error::Error;
use std::fmt;

use nalgebra::{SMatrix, SVector};

use crate::periodic::{LatticeOffset, WrappedPoint};

const MAX_EXACT_LATTICE_INDEX: f64 = 4_503_599_627_370_496.0; // 2^52
const RECONSTRUCTION_EPSILON_MULTIPLIER: f64 = 64.0;
const DISTANCE_TIE_EPSILON_MULTIPLIER: f64 = 16.0;

/// Failure reasons for triclinic lattice operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeriodicLatticeError {
    InvalidDimension { dimension: usize },
    NonFiniteOrigin,
    NonFiniteBasis,
    SingularBasis,
    UnrepresentableLatticeVector { lattice_axis: usize, cartesian_axis: usize },
    IllConditionedBasis,
    NonFiniteCoordinate,
    CoordinatePrecisionLimit { axis: usize },
    InvalidImageOffset { axis: usize },
    OffsetOnNonPeriodicAxis { axis: usize },
    InvalidCandidateBudget,
    CandidateBudgetExceeded { required_at_least: usize, maximum: usize },
    UncertifiedMinimumImage { candidates: usize, maximum: usize },
    NumericRange { field: &'static str },
}

impl fmt::Display for PeriodicLatticeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDimension { dimension } => {
                write!(f, "triclinic periodic lattice supports D=1,2,3; got D={dimension}")
            }
            Self::NonFiniteOrigin => write!(f, "triclinic lattice origin must be finite"),
            Self::NonFiniteBasis => write!(f, "triclinic lattice basis must be finite"),
            Self::SingularBasis => write!(f, "triclinic lattice basis is singular"),
            Self::UnrepresentableLatticeVector { lattice_axis, cartesian_axis } => write!(
                f,
                "basis vector {lattice_axis} is not representable on Cartesian axis {cartesian_axis} at the lattice origin"
            ),
            Self::IllConditionedBasis => {
                write!(f, "triclinic lattice inverse is outside the finite numeric envelope")
            }
            Self::NonFiniteCoordinate => write!(f, "triclinic lattice coordinate must be finite"),
            Self::CoordinatePrecisionLimit { axis } => {
                write!(f, "lattice coordinate exceeds the exact image-index envelope on axis {axis}")
            }
            Self::InvalidImageOffset { axis } => {
                write!(f, "lattice image offset exceeds the supported numeric envelope on axis {axis}")
            }
            Self::OffsetOnNonPeriodicAxis { axis } => {
                write!(f, "nonzero image offset on non-periodic lattice axis {axis}")
            }
            Self::InvalidCandidateBudget => write!(f, "minimum-image candidate budget must be positive"),
            Self::CandidateBudgetExceeded { required_at_least, maximum } => write!(
                f,
                "lattice search needs at least {required_at_least} candidates; budget is {maximum}"
            ),
            Self::UncertifiedMinimumImage { candidates, maximum } => write!(
                f,
                "minimum image remains uncertified after {candidates} candidates (budget {maximum})"
            ),
            Self::NumericRange { field } => {
                write!(f, "triclinic lattice arithmetic exceeded numeric range in {field}")
            }
        }
    }
}

impl Error for PeriodicLatticeError {}

/// Whether both directions of a displacement remain distinguishable from a
/// coordinate at the current floating-point magnitude, with at most 12.5%
/// relative representation error.
fn component_is_resolvable(coordinate: f64, displacement: f64) -> bool {
    if !coordinate.is_finite() || !displacement.is_finite() {
        return false;
    }
    if displacement == 0.0 {
        return true;
    }
    let positive = coordinate + displacement;
    let negative = coordinate - displacement;
    if !positive.is_finite()
        || !negative.is_finite()
        || positive == coordinate
        || negative == coordinate
    {
        return false;
    }
    let maximum_error = 0.125 * displacement.abs();
    let positive_error = ((positive - coordinate) - displacement).abs();
    let negative_error = ((negative - coordinate) + displacement).abs();
    positive_error <= maximum_error && negative_error <= maximum_error
}

/// A certified nearest-image displacement in the original Cartesian chart.
#[derive(Clone, Debug, PartialEq)]
pub struct MinimumImage<const D: usize> {
    pub displacement: SVector<f64, D>,
    /// The selected image translation is subtracted from the input displacement.
    /// When the nearest image is numerically ambiguous, this is a deterministic
    /// representative; inspect the ambiguous-image list before treating it as unique.
    pub image_offset: LatticeOffset<D>,
    /// Other images whose computed distance intervals overlap the best candidate
    /// upper bound. A non-empty set means the nearest image is not unique at the
    /// declared floating-point precision.
    pub ambiguous_image_offsets: Vec<LatticeOffset<D>>,
    pub candidates_evaluated: usize,
    /// Residual-based upper bound on ||I - inverse_basis * basis||_F.
    pub inverse_residual_upper_bound: f64,
    /// Conservative roundoff allowance for the selected Cartesian distance.
    pub nearest_distance_error_bound: f64,
    /// Minimum upper bound among all evaluated candidate-distance intervals.
    pub best_candidate_distance_upper_bound: f64,
    /// Conservative numerical lower bound for every image outside the searched cube.
    pub unsearched_distance_lower_bound: f64,
}

/// A full-rank lattice cell with columns given by the lattice basis vectors.
///
/// A point is represented by fractional coordinates obtained by applying
/// basis_inverse to (x - origin). Periodic axes are wrapped into [0,1) in
/// fractional coordinates. Non-periodic fractional coordinates remain
/// unchanged.
#[derive(Clone, Debug, PartialEq)]
pub struct PeriodicLattice<const D: usize> {
    origin: SVector<f64, D>,
    basis: SMatrix<f64, D, D>,
    inverse: SMatrix<f64, D, D>,
    basis_frobenius: f64,
    periodic_axes: [bool; D],
    /// Residual-inflated lower bound on the smallest singular value.
    sigma_min_lower_bound: f64,
    /// Residual-based upper bound on ||I - inverse_basis * basis||_F.
    inverse_residual_upper_bound: f64,
}

impl<const D: usize> PeriodicLattice<D> {
    pub fn new(
        origin: SVector<f64, D>,
        basis: SMatrix<f64, D, D>,
        periodic_axes: [bool; D],
    ) -> Result<Self, PeriodicLatticeError> {
        if D == 0 || D > 3 {
            return Err(PeriodicLatticeError::InvalidDimension { dimension: D });
        }
        if !origin.iter().all(|value| value.is_finite()) {
            return Err(PeriodicLatticeError::NonFiniteOrigin);
        }
        if !basis.iter().all(|value| value.is_finite()) {
            return Err(PeriodicLatticeError::NonFiniteBasis);
        }

        // Validate that each nonzero basis component can actually be
        // represented when translated from the declared origin. A finite,
        // invertible matrix can still describe a cell smaller than the ULP of
        // a huge origin (for example origin=1e16 and basis component=1).
        for lattice_axis in 0..D {
            for cartesian_axis in 0..D {
                let component = basis[(cartesian_axis, lattice_axis)];
                if component == 0.0 {
                    continue;
                }
                let coordinate = origin[cartesian_axis];
                if !component_is_resolvable(coordinate, component) {
                    return Err(PeriodicLatticeError::UnrepresentableLatticeVector {
                        lattice_axis,
                        cartesian_axis,
                    });
                }
            }
        }

        let inverse = basis
            .try_inverse()
            .ok_or(PeriodicLatticeError::SingularBasis)?;
        if !inverse.iter().all(|value| value.is_finite()) {
            return Err(PeriodicLatticeError::IllConditionedBasis);
        }
        let inverse_frobenius = inverse.norm();
        let basis_frobenius = basis.norm();
        if !inverse_frobenius.is_finite()
            || inverse_frobenius <= 0.0
            || !basis_frobenius.is_finite()
            || basis_frobenius <= 0.0
        {
            return Err(PeriodicLatticeError::IllConditionedBasis);
        }

        // Do not assume the floating-point inverse is exact. Bound the
        // residual of the computed near-inverse and inflate it for the
        // matrix-product, subtraction and Frobenius-norm roundoff. If
        // ||I - C B|| <= r < 1 for C = computed inverse, then
        // ||B^-1||_2 <= ||C||_F / (1-r), so
        // sigma_min(B) >= (1-r) / ||C||_F.
        let product = inverse * basis;
        let residual = (SMatrix::<f64, D, D>::identity() - product).norm();
        let operations = (2 * D + 3) as f64;
        let epsilon_product = operations * f64::EPSILON;
        if !residual.is_finite() || epsilon_product >= 0.5 {
            return Err(PeriodicLatticeError::IllConditionedBasis);
        }
        let gamma = epsilon_product / (1.0 - epsilon_product);
        let arithmetic_scale = inverse_frobenius * basis_frobenius + (D as f64).sqrt();
        let residual_roundoff = 8.0 * gamma * arithmetic_scale
            + 8.0 * f64::EPSILON * (residual + (D as f64).sqrt());
        let inverse_residual_upper_bound = residual + residual_roundoff;
        if !inverse_residual_upper_bound.is_finite()
            || inverse_residual_upper_bound >= 1.0
        {
            return Err(PeriodicLatticeError::IllConditionedBasis);
        }

        // Deflate once more to account for rounding in the scalar bound itself.
        let sigma_min_lower_bound = (1.0 - inverse_residual_upper_bound)
            / inverse_frobenius
            * (1.0 - 8.0 * f64::EPSILON);
        if !sigma_min_lower_bound.is_finite() || sigma_min_lower_bound <= 0.0 {
            return Err(PeriodicLatticeError::IllConditionedBasis);
        }

        Ok(Self {
            origin,
            basis,
            inverse,
            basis_frobenius,
            periodic_axes,
            sigma_min_lower_bound,
            inverse_residual_upper_bound,
        })
    }

    pub fn origin(&self) -> &SVector<f64, D> {
        &self.origin
    }

    /// Matrix whose columns are lattice basis vectors.
    pub fn basis(&self) -> &SMatrix<f64, D, D> {
        &self.basis
    }

    pub fn inverse_basis(&self) -> &SMatrix<f64, D, D> {
        &self.inverse
    }

    pub fn periodic_axes(&self) -> &[bool; D] {
        &self.periodic_axes
    }

    pub fn is_periodic(&self, axis: usize) -> Option<bool> {
        self.periodic_axes.get(axis).copied()
    }

    pub fn sigma_min_lower_bound(&self) -> f64 {
        self.sigma_min_lower_bound
    }

    pub fn inverse_residual_upper_bound(&self) -> f64 {
        self.inverse_residual_upper_bound
    }

    /// Convert a lattice image offset into its Cartesian translation.
    pub fn offset_vector(
        &self,
        offset: LatticeOffset<D>,
    ) -> Result<SVector<f64, D>, PeriodicLatticeError> {
        for axis in 0..D {
            let index = offset.periods[axis];
            if index == 0 {
                continue;
            }
            if !self.periodic_axes[axis] {
                return Err(PeriodicLatticeError::OffsetOnNonPeriodicAxis { axis });
            }
            if (index as f64).abs() >= MAX_EXACT_LATTICE_INDEX {
                return Err(PeriodicLatticeError::InvalidImageOffset { axis });
            }
        }

        let fractional_offset =
            SVector::<f64, D>::from(std::array::from_fn(|axis| offset.periods[axis] as f64));
        let translation = self.basis * fractional_offset;
        if !translation.iter().all(|value| value.is_finite()) {
            return Err(PeriodicLatticeError::NumericRange {
                field: "image offset translation",
            });
        }
        Ok(translation)
    }

    /// Wrap a point into the half-open unit fractional cell on periodic axes.
    ///
    /// The returned integer offset reconstructs the original point by
    /// adding the image translation to the canonical position. A
    /// reconstruction residual larger than the scale-aware floating-point
    /// envelope is an explicit precision failure.
    pub fn wrap_point(
        &self,
        point: &SVector<f64, D>,
    ) -> Result<WrappedPoint<D>, PeriodicLatticeError> {
        if !point.iter().all(|value| value.is_finite()) {
            return Err(PeriodicLatticeError::NonFiniteCoordinate);
        }
        for lattice_axis in 0..D {
            if !self.periodic_axes[lattice_axis] {
                continue;
            }
            for cartesian_axis in 0..D {
                let component = self.basis[(cartesian_axis, lattice_axis)];
                if !component_is_resolvable(point[cartesian_axis], component) {
                    return Err(PeriodicLatticeError::CoordinatePrecisionLimit {
                        axis: lattice_axis,
                    });
                }
            }
        }
        if !self.periodic_axes.iter().any(|active| *active) {
            return Ok(WrappedPoint {
                position: *point,
                image_offset: LatticeOffset::zero(),
            });
        }

        let relative = point - self.origin;
        if !relative.iter().all(|value| value.is_finite()) {
            return Err(PeriodicLatticeError::CoordinatePrecisionLimit { axis: 0 });
        }
        let fractional = self.inverse * relative;
        if !fractional.iter().all(|value| value.is_finite()) {
            return Err(PeriodicLatticeError::NumericRange {
                field: "fractional coordinate",
            });
        }

        let mut canonical_fractional = fractional;
        let mut periods = [0_i64; D];
        for axis in 0..D {
            if !self.periodic_axes[axis] {
                continue;
            }
            let coordinate = fractional[axis];
            if coordinate.abs() >= MAX_EXACT_LATTICE_INDEX {
                return Err(PeriodicLatticeError::CoordinatePrecisionLimit { axis });
            }

            let image = coordinate.floor();
            let mut remainder = coordinate - image;
            let mut image_index = image as i64;
            if remainder < 0.0 {
                remainder += 1.0;
                image_index = image_index.checked_sub(1).ok_or(
                    PeriodicLatticeError::CoordinatePrecisionLimit { axis },
                )?;
            }
            if remainder >= 1.0 {
                remainder = 0.0;
                image_index = image_index.checked_add(1).ok_or(
                    PeriodicLatticeError::CoordinatePrecisionLimit { axis },
                )?;
            }
            if !remainder.is_finite() || !(0.0..1.0).contains(&remainder) {
                return Err(PeriodicLatticeError::CoordinatePrecisionLimit { axis });
            }
            canonical_fractional[axis] = remainder;
            periods[axis] = image_index;
        }

        let canonical = self.origin + self.basis * canonical_fractional;
        if !canonical.iter().all(|value| value.is_finite()) {
            return Err(PeriodicLatticeError::NumericRange {
                field: "canonical point",
            });
        }

        let offset = LatticeOffset::new(periods);
        let translation = self.offset_vector(offset)?;
        let reconstructed = canonical + translation;
        let residual = (reconstructed - point).norm();
        let scale = point.norm().max(canonical.norm()).max(translation.norm()).max(1.0);
        let tolerance = RECONSTRUCTION_EPSILON_MULTIPLIER * f64::EPSILON * scale;
        if !residual.is_finite() || residual > tolerance {
            return Err(PeriodicLatticeError::CoordinatePrecisionLimit { axis: 0 });
        }

        Ok(WrappedPoint {
            position: canonical,
            image_offset: offset,
        })
    }

    /// Find the nearest periodic image of a displacement and certify it.
    ///
    /// Component-wise rounding in fractional coordinates is not a closest
    /// vector algorithm for a skewed basis. This method enumerates successive
    /// integer shells around the rounded fractional coordinate. The Frobenius
    /// inverse bound supplies a conservative lower bound for all unseen
    /// lattice vectors. An answer is returned only when that lower bound is
    /// strictly larger than the best candidate distance plus a roundoff margin.
    pub fn minimum_image_displacement(
        &self,
        displacement: &SVector<f64, D>,
        max_candidates: usize,
    ) -> Result<MinimumImage<D>, PeriodicLatticeError> {
        if max_candidates == 0 {
            return Err(PeriodicLatticeError::InvalidCandidateBudget);
        }
        if !displacement.iter().all(|value| value.is_finite()) {
            return Err(PeriodicLatticeError::NonFiniteCoordinate);
        }
        for lattice_axis in 0..D {
            if !self.periodic_axes[lattice_axis] {
                continue;
            }
            for cartesian_axis in 0..D {
                let component = self.basis[(cartesian_axis, lattice_axis)];
                if !component_is_resolvable(displacement[cartesian_axis], component) {
                    return Err(PeriodicLatticeError::CoordinatePrecisionLimit {
                        axis: lattice_axis,
                    });
                }
            }
        }
        if !self.periodic_axes.iter().any(|active| *active) {
            return Ok(MinimumImage {
                displacement: *displacement,
                image_offset: LatticeOffset::zero(),
                ambiguous_image_offsets: Vec::new(),
                candidates_evaluated: 0,
                inverse_residual_upper_bound: self.inverse_residual_upper_bound,
                nearest_distance_error_bound: 0.0,
                best_candidate_distance_upper_bound: displacement.norm(),
                unsearched_distance_lower_bound: f64::INFINITY,
            });
        }

        let fractional = self.inverse * displacement;
        if !fractional.iter().all(|value| value.is_finite()) {
            return Err(PeriodicLatticeError::NumericRange {
                field: "minimum-image fractional coordinate",
            });
        }

        let mut center = [0_i64; D];
        let mut periodic_count = 0_usize;
        for axis in 0..D {
            if !self.periodic_axes[axis] {
                continue;
            }
            periodic_count += 1;
            let rounded = fractional[axis].round();
            if !rounded.is_finite() || rounded.abs() >= MAX_EXACT_LATTICE_INDEX {
                return Err(PeriodicLatticeError::CoordinatePrecisionLimit { axis });
            }
            center[axis] = rounded as i64;
        }
        if periodic_count == 0 {
            unreachable!("periodic mask checked above");
        }

        let orientation = fractional
            .iter()
            .find(|value| value.abs() > 0.0)
            .map_or(1.0, |value| value.signum());

        let mut best_distance = f64::INFINITY;
        let mut best_distance_error_bound = f64::INFINITY;
        let mut best_candidate_distance_upper_bound = f64::INFINITY;
        let mut plausible_image_offsets = Vec::<(LatticeOffset<D>, f64)>::new();
        let mut best_alignment = f64::NEG_INFINITY;
        let mut best_displacement = *displacement;
        let mut best_offset = LatticeOffset::zero();
        let mut candidates_evaluated = 0_usize;
        let mut shell = 0_i64;

        loop {
            let side = shell
                .checked_mul(2)
                .and_then(|value| value.checked_add(1))
                .ok_or(PeriodicLatticeError::UncertifiedMinimumImage {
                    candidates: candidates_evaluated,
                    maximum: max_candidates,
                })?;
            let mut cube_count = 1_usize;
            for _ in 0..periodic_count {
                cube_count = cube_count.checked_mul(side as usize).ok_or(
                    PeriodicLatticeError::CandidateBudgetExceeded {
                        required_at_least: usize::MAX,
                        maximum: max_candidates,
                    },
                )?;
            }
            if cube_count > max_candidates {
                return Err(PeriodicLatticeError::CandidateBudgetExceeded {
                    required_at_least: cube_count,
                    maximum: max_candidates,
                });
            }

            let mut offset = [0_i64; D];
            let mut visit_error = None;
            enumerate_shell(
                0,
                shell,
                &center,
                &self.periodic_axes,
                &mut offset,
                false,
                &mut |candidate| {
                    if visit_error.is_some() {
                        return;
                    }
                    let candidate_offset = LatticeOffset::new(candidate);
                    match self.offset_vector(candidate_offset) {
                        Ok(translation) => {
                            let candidate_displacement = displacement - translation;
                            if !candidate_displacement.iter().all(|value| value.is_finite()) {
                                visit_error = Some(PeriodicLatticeError::NumericRange {
                                    field: "minimum-image candidate",
                                });
                                return;
                            }
                            let distance = candidate_displacement.norm();
                            if !distance.is_finite() {
                                visit_error = Some(PeriodicLatticeError::NumericRange {
                                    field: "minimum-image distance",
                                });
                                return;
                            }

                            let alignment = candidate_displacement.dot(displacement);
                            if !alignment.is_finite() {
                                visit_error = Some(PeriodicLatticeError::NumericRange {
                                    field: "minimum-image tie alignment",
                                });
                                return;
                            }
                            // Inflate the candidate norm for the matrix-vector
                            // product, subtraction, and norm evaluation. This
                            // is especially important when large coordinates
                            // nearly cancel to a small nearest-image residual.
                            let offset_norm = SVector::<f64, D>::from(std::array::from_fn(
                                |axis| candidate[axis] as f64,
                            ))
                            .norm();
                            let matrix_product_scale = self.basis_frobenius * offset_norm;
                            if !matrix_product_scale.is_finite() {
                                visit_error = Some(PeriodicLatticeError::NumericRange {
                                    field: "minimum-image translation error scale",
                                });
                                return;
                            }
                            let distance_error_bound = 64.0
                                * (D as f64 + 1.0)
                                * f64::EPSILON
                                * (displacement.norm()
                                    + matrix_product_scale
                                    + translation.norm()
                                    + candidate_displacement.norm()
                                    + 1.0);
                            if !distance_error_bound.is_finite() {
                                visit_error = Some(PeriodicLatticeError::NumericRange {
                                    field: "minimum-image distance error bound",
                                });
                                return;
                            }
                            let candidate_distance_upper_bound =
                                distance + distance_error_bound;
                            let candidate_distance_lower_bound =
                                (distance - distance_error_bound).max(0.0);
                            if !candidate_distance_upper_bound.is_finite()
                                || !candidate_distance_lower_bound.is_finite()
                            {
                                visit_error = Some(PeriodicLatticeError::NumericRange {
                                    field: "minimum-image distance interval",
                                });
                                return;
                            }

                            // Preserve every image whose error interval can
                            // still contain the true minimum. This exposes
                            // Voronoi-boundary ties and near-ties instead of
                            // silently pretending the representative is unique.
                            let upper_is_better =
                                candidate_distance_upper_bound < best_candidate_distance_upper_bound;
                            if upper_is_better {
                                best_candidate_distance_upper_bound =
                                    candidate_distance_upper_bound;
                                let plausible_margin = DISTANCE_TIE_EPSILON_MULTIPLIER
                                    * f64::EPSILON
                                    * best_candidate_distance_upper_bound.max(1.0);
                                plausible_image_offsets.retain(|(_, lower)| {
                                    *lower <= best_candidate_distance_upper_bound + plausible_margin
                                });
                            }

                            let alignment_tolerance = DISTANCE_TIE_EPSILON_MULTIPLIER
                                * f64::EPSILON
                                * alignment.abs().max(best_alignment.abs()).max(1.0);
                            let distance_precedes = !best_distance.is_finite()
                                || distance < best_distance
                                || (distance == best_distance
                                    && (alignment > best_alignment + alignment_tolerance
                                        || ((alignment - best_alignment).abs()
                                            <= alignment_tolerance
                                            && signed_offset_precedes(
                                                &candidate,
                                                &best_offset.periods,
                                                orientation,
                                            ))));
                            if upper_is_better || (candidate_distance_upper_bound == best_candidate_distance_upper_bound
                                && distance_precedes)
                            {
                                best_distance = distance;
                                best_distance_error_bound = distance_error_bound;
                                best_alignment = alignment;
                                best_displacement = candidate_displacement;
                                best_offset = candidate_offset;
                            }

                            let plausible_margin = DISTANCE_TIE_EPSILON_MULTIPLIER
                                * f64::EPSILON
                                * best_candidate_distance_upper_bound.max(1.0);
                            if candidate_distance_lower_bound
                                <= best_candidate_distance_upper_bound + plausible_margin
                            {
                                plausible_image_offsets
                                    .push((candidate_offset, candidate_distance_lower_bound));
                            }
                        }
                        Err(error) => visit_error = Some(error),
                    }
                },
            );
            if let Some(error) = visit_error {
                return Err(error);
            }
            candidates_evaluated = cube_count;

            let lower_bound =
                self.sigma_min_lower_bound * (shell as f64 + 0.5) * (1.0 - 8.0 * f64::EPSILON);
            if !lower_bound.is_finite() {
                return Err(PeriodicLatticeError::NumericRange {
                    field: "unsearched-image lower bound",
                });
            }
            let margin = DISTANCE_TIE_EPSILON_MULTIPLIER
                * f64::EPSILON
                * lower_bound
                    .max(best_candidate_distance_upper_bound)
                    .max(1.0);
            if lower_bound > best_candidate_distance_upper_bound + margin {
                let mut ambiguous_image_offsets: Vec<_> = plausible_image_offsets
                    .into_iter()
                    .map(|(offset, _)| offset)
                    .filter(|offset| *offset != best_offset)
                    .collect();
                ambiguous_image_offsets.sort_by(|left, right| {
                    left.periods.cmp(&right.periods)
                });
                ambiguous_image_offsets.dedup();
                return Ok(MinimumImage {
                    displacement: best_displacement,
                    image_offset: best_offset,
                    ambiguous_image_offsets,
                    candidates_evaluated,
                    inverse_residual_upper_bound: self.inverse_residual_upper_bound,
                    nearest_distance_error_bound: best_distance_error_bound,
                    best_candidate_distance_upper_bound,
                    unsearched_distance_lower_bound: lower_bound,
                });
            }

            if candidates_evaluated >= max_candidates {
                return Err(PeriodicLatticeError::UncertifiedMinimumImage {
                    candidates: candidates_evaluated,
                    maximum: max_candidates,
                });
            }
            shell = shell.checked_add(1).ok_or(
                PeriodicLatticeError::UncertifiedMinimumImage {
                    candidates: candidates_evaluated,
                    maximum: max_candidates,
                },
            )?;
        }
    }
}

/// Enumerate only the outer boundary of the current integer cube. Therefore
/// each offset is visited exactly once over shells 0, 1, 2, ... .
fn enumerate_shell<const D: usize, F>(
    axis: usize,
    shell: i64,
    center: &[i64; D],
    periodic_axes: &[bool; D],
    current: &mut [i64; D],
    already_on_boundary: bool,
    visitor: &mut F,
) where
    F: FnMut([i64; D]),
{
    if axis == D {
        if already_on_boundary {
            visitor(*current);
        }
        return;
    }

    if !periodic_axes[axis] {
        current[axis] = 0;
        enumerate_shell(
            axis + 1,
            shell,
            center,
            periodic_axes,
            current,
            already_on_boundary,
            visitor,
        );
        return;
    }

    let low = center[axis] - shell;
    let high = center[axis] + shell;
    for value in low..=high {
        current[axis] = value;
        let on_boundary = already_on_boundary || (value - center[axis]).abs() == shell;
        enumerate_shell(
            axis + 1,
            shell,
            center,
            periodic_axes,
            current,
            on_boundary,
            visitor,
        );
    }
}

fn signed_offset_precedes<const D: usize>(
    candidate: &[i64; D],
    current: &[i64; D],
    orientation: f64,
) -> bool {
    for axis in 0..D {
        let left = candidate[axis] as f64 * orientation;
        let right = current[axis] as f64 * orientation;
        if left < right {
            return true;
        }
        if left > right {
            return false;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{PeriodicLattice, PeriodicLatticeError};
    use crate::periodic::{LatticeOffset, PeriodicBox};
    use nalgebra::{SMatrix, SVector};

    fn orthogonal_lattice() -> PeriodicLattice<2> {
        PeriodicLattice::new(
            SVector::from([2.0, -3.0]),
            SMatrix::from_diagonal(&SVector::from([10.0, 8.0])),
            [true, true],
        )
        .unwrap()
    }

    #[derive(Clone, Copy, Debug)]
    struct DoubleDouble {
        hi: f64,
        lo: f64,
    }

    impl DoubleDouble {
        fn from_f64(value: f64) -> Self {
            Self { hi: value, lo: 0.0 }
        }

        fn two_sum(a: f64, b: f64) -> (f64, f64) {
            let sum = a + b;
            let b_virtual = sum - a;
            let error = (a - (sum - b_virtual)) + (b - b_virtual);
            (sum, error)
        }

        fn add(self, other: Self) -> Self {
            let (sum, error) = Self::two_sum(self.hi, other.hi);
            let low = error + self.lo + other.lo;
            let (hi, lo) = Self::two_sum(sum, low);
            Self { hi, lo }
        }

        fn neg(self) -> Self {
            Self {
                hi: -self.hi,
                lo: -self.lo,
            }
        }

        fn sub(self, other: Self) -> Self {
            self.add(other.neg())
        }

        fn mul(self, other: Self) -> Self {
            let product = self.hi * other.hi;
            let product_error = self.hi.mul_add(other.hi, -product);
            let cross = self.hi * other.lo + self.lo * other.hi + self.lo * other.lo;
            let (hi, lo) = Self::two_sum(product, product_error + cross);
            Self { hi, lo }
        }

        fn cmp(self, other: Self) -> std::cmp::Ordering {
            match self.hi.total_cmp(&other.hi) {
                std::cmp::Ordering::Equal => self.lo.total_cmp(&other.lo),
                ordering => ordering,
            }
        }
    }

    /// Independent double-double evaluation of ||d - B n||². This oracle
    /// deliberately avoids nalgebra's f64 matrix-vector multiply and norm
    /// used by the production candidate evaluator.
    fn dd_squared_distance<const D: usize>(
        basis: &SMatrix<f64, D, D>,
        displacement: &SVector<f64, D>,
        offset: [i64; D],
    ) -> DoubleDouble {
        let mut squared = DoubleDouble::from_f64(0.0);
        for cartesian_axis in 0..D {
            let mut translated = DoubleDouble::from_f64(0.0);
            for lattice_axis in 0..D {
                let term = DoubleDouble::from_f64(basis[(cartesian_axis, lattice_axis)])
                    .mul(DoubleDouble::from_f64(offset[lattice_axis] as f64));
                translated = translated.add(term);
            }
            let residual = DoubleDouble::from_f64(displacement[cartesian_axis])
                .sub(translated);
            squared = squared.add(residual.mul(residual));
        }
        squared
    }

    fn dd_closest_image<const D: usize>(
        basis: &SMatrix<f64, D, D>,
        displacement: &SVector<f64, D>,
        center: [i64; D],
        periodic_axes: [bool; D],
        radius: i64,
    ) -> ([i64; D], DoubleDouble) {
        fn visit<const D: usize>(
            axis: usize,
            basis: &SMatrix<f64, D, D>,
            displacement: &SVector<f64, D>,
            center: &[i64; D],
            periodic_axes: &[bool; D],
            radius: i64,
            current: &mut [i64; D],
            best: &mut Option<([i64; D], DoubleDouble)>,
        ) {
            if axis == D {
                let distance = dd_squared_distance(basis, displacement, *current);
                let replace = best
                    .as_ref()
                    .is_none_or(|(_, best_distance)| {
                        distance.cmp(*best_distance) == std::cmp::Ordering::Less
                    });
                if replace {
                    *best = Some((*current, distance));
                }
                return;
            }
            if !periodic_axes[axis] {
                current[axis] = 0;
                visit(
                    axis + 1,
                    basis,
                    displacement,
                    center,
                    periodic_axes,
                    radius,
                    current,
                    best,
                );
                return;
            }
            for value in (center[axis] - radius)..=(center[axis] + radius) {
                current[axis] = value;
                visit(
                    axis + 1,
                    basis,
                    displacement,
                    center,
                    periodic_axes,
                    radius,
                    current,
                    best,
                );
            }
            current[axis] = 0;
        }

        let mut current = [0_i64; D];
        let mut best = None;
        visit(
            0,
            basis,
            displacement,
            &center,
            &periodic_axes,
            radius,
            &mut current,
            &mut best,
        );
        best.expect("the independent integer-image oracle visits at least one image")
    }

    #[test]
    fn orthogonal_lattice_matches_periodic_box_wrapping_and_minimum_image() {
        let lattice = orthogonal_lattice();
        let domain = PeriodicBox::new(
            SVector::from([2.0, -3.0]),
            SVector::from([10.0, 8.0]),
            [true, true],
        )
        .unwrap();
        for point in [
            SVector::from([-28.0, 13.0]),
            SVector::from([2.0, -3.0]),
            SVector::from([12.0, 5.0]),
            SVector::from([17.25, -19.5]),
        ] {
            let lattice_wrapped = lattice.wrap_point(&point).unwrap();
            let box_wrapped = domain.wrap_point(&point).unwrap();
            assert_eq!(lattice_wrapped.image_offset, box_wrapped.image_offset);
            assert!((lattice_wrapped.position - box_wrapped.position).norm() < 1e-11);
        }

        for delta in [
            SVector::from([9.0, -7.0]),
            SVector::from([-21.0, 18.0]),
            SVector::from([5.0, -4.0]),
        ] {
            let lattice_minimum = lattice.minimum_image_displacement(&delta, 81).unwrap();
            let box_minimum = domain.minimum_image_displacement(&delta).unwrap();
            assert!((lattice_minimum.displacement - box_minimum).norm() < 1e-10);
        }
    }

    #[test]
    fn exact_half_cell_ties_preserve_reversal_oddness() {
        let lattice = orthogonal_lattice();
        for delta in [
            SVector::from([5.0, -4.0]),
            SVector::from([-5.0, 4.0]),
            SVector::from([15.0, 12.0]),
        ] {
            let forward = lattice.minimum_image_displacement(&delta, 81).unwrap();
            let reverse = lattice.minimum_image_displacement(&(-delta), 81).unwrap();
            assert!((forward.displacement + reverse.displacement).norm() < 1e-10);
            assert_eq!(
                forward.image_offset.periods,
                reverse.image_offset.periods.map(|period| -period)
            );
            assert!(
                !forward.ambiguous_image_offsets.is_empty(),
                "exact half-cell geometry should expose its other co-nearest images"
            );
            let mut forward_images = vec![forward.image_offset.periods];
            forward_images.extend(
                forward
                    .ambiguous_image_offsets
                    .iter()
                    .map(|offset| offset.periods),
            );
            let mut reverse_images = vec![reverse.image_offset.periods];
            reverse_images.extend(
                reverse
                    .ambiguous_image_offsets
                    .iter()
                    .map(|offset| offset.periods),
            );
            forward_images.sort();
            reverse_images.sort();
            assert_eq!(
                forward_images
                    .iter()
                    .map(|offset| offset.map(|period| -period))
                    .collect::<Vec<_>>(),
                reverse_images
            );
        }
    }

    #[test]
    fn skewed_cell_wraps_fractional_coordinates_and_reconstructs() {
        let basis = SMatrix::<f64, 2, 2>::new(10.0, 4.0, 0.0, 8.0);
        let lattice = PeriodicLattice::new(
            SVector::from([1.0, -2.0]),
            basis,
            [true, true],
        )
        .unwrap();

        let fractional = SVector::from([2.25, -1.75]);
        let point = lattice.origin() + lattice.basis() * fractional;
        let wrapped = lattice.wrap_point(&point).unwrap();
        assert_eq!(wrapped.image_offset, LatticeOffset::new([2, -2]));

        let canonical_fractional = lattice.inverse_basis() * (wrapped.position - lattice.origin());
        assert!((canonical_fractional - SVector::from([0.25, 0.25])).norm() < 1e-11);
        let restored = wrapped.position + lattice.offset_vector(wrapped.image_offset).unwrap();
        assert!((restored - point).norm() < 1e-10);
    }

    #[test]
    fn skewed_minimum_image_matches_independent_integer_search() {
        let basis = SMatrix::<f64, 2, 2>::new(10.0, 8.5, 0.0, 3.0);
        let lattice = PeriodicLattice::new(
            SVector::zeros(),
            basis,
            [true, true],
        )
        .unwrap();

        let mut seed = 0x5452_4943_4c49_4e49_u64;
        for _case in 0..300 {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let x = ((seed >> 11) as f64 / ((1_u64 << 53) as f64) - 0.5) * 100.0;
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let y = ((seed >> 11) as f64 / ((1_u64 << 53) as f64) - 0.5) * 100.0;
            let delta = SVector::from([x, y]);

            let result = lattice.minimum_image_displacement(&delta, 4_000).unwrap();
            let fractional = lattice.inverse_basis() * delta;
            let center = [fractional[0].round() as i64, fractional[1].round() as i64];

            let mut best = f64::INFINITY;
            for i in (center[0] - 48)..=(center[0] + 48) {
                for j in (center[1] - 48)..=(center[1] + 48) {
                    let shifted = delta - lattice.basis() * SVector::from([i as f64, j as f64]);
                    best = best.min(shifted.norm());
                }
            }
            assert!(
                (result.displacement.norm() - best).abs() < 1e-9,
                "closest-image mismatch for delta={delta:?}, result={:?}, oracle={best}",
                result
            );
            let (_, high_precision_oracle) =
                dd_closest_image(&basis, &delta, center, [true, true], 48);
            let selected_high_precision =
                dd_squared_distance(&basis, &delta, result.image_offset.periods);
            let selected_order = selected_high_precision.cmp(high_precision_oracle);
            assert!(
                selected_order != std::cmp::Ordering::Less,
                "bounded double-double oracle missed a closer image for delta={delta:?}"
            );
            let mut ambiguity_contains_oracle = false;
            for alternative in &result.ambiguous_image_offsets {
                let alternative_distance =
                    dd_squared_distance(&basis, &delta, alternative.periods);
                let ordering = alternative_distance.cmp(high_precision_oracle);
                assert!(
                    ordering != std::cmp::Ordering::Less,
                    "reported ambiguity contains an image closer than the oracle minimum"
                );
                ambiguity_contains_oracle |= ordering == std::cmp::Ordering::Equal;
            }
            if selected_order == std::cmp::Ordering::Greater {
                assert!(
                    ambiguity_contains_oracle,
                    "the double-double minimum is not the representative or an ambiguity"
                );
            }
            assert!(result.unsearched_distance_lower_bound
                > result.displacement.norm() + result.nearest_distance_error_bound);
        }
    }

    #[test]
    fn one_periodic_axis_keeps_nonperiodic_fractional_component_fixed() {
        let basis = SMatrix::<f64, 2, 2>::new(10.0, 4.0, 0.0, 8.0);
        let lattice = PeriodicLattice::new(SVector::zeros(), basis, [true, false]).unwrap();
        let delta = SVector::from([9.0, 7.0]);
        let result = lattice.minimum_image_displacement(&delta, 81).unwrap();

        assert_eq!(result.image_offset.periods[1], 0);
        let expected = delta - lattice.offset_vector(result.image_offset).unwrap();
        assert!((result.displacement - expected).norm() < 1e-10);
    }

    #[test]
    fn unimodular_basis_change_preserves_physical_wrapping_and_minimum_image() {
        let basis = SMatrix::<f64, 2, 2>::new(10.0, 3.0, 0.0, 8.0);
        let shear = SMatrix::<f64, 2, 2>::new(1.0, 1.0, 0.0, 1.0);
        let changed_basis = basis * shear;
        let original = PeriodicLattice::new(SVector::zeros(), basis, [true, true]).unwrap();
        let changed =
            PeriodicLattice::new(SVector::zeros(), changed_basis, [true, true]).unwrap();

        let point = SVector::from([57.25, -31.75]);
        let original_wrapped = original.wrap_point(&point).unwrap();
        let changed_wrapped = changed.wrap_point(&point).unwrap();
        let changed_representative_in_original_cell =
            original.wrap_point(&changed_wrapped.position).unwrap();
        assert!(
            (original_wrapped.position - changed_representative_in_original_cell.position).norm()
                < 1e-10
        );

        for delta in [
            SVector::from([31.0, -17.0]),
            SVector::from([-44.5, 29.0]),
            SVector::from([5.0, 4.0]),
        ] {
            let left = original.minimum_image_displacement(&delta, 729).unwrap();
            let right = changed.minimum_image_displacement(&delta, 729).unwrap();
            assert!((left.displacement - right.displacement).norm() < 1e-9);
        }
    }

    #[test]
    fn three_dimensional_skew_lattice_wraps_and_matches_brute_force() {
        let basis = SMatrix::<f64, 3, 3>::new(
            10.0, 2.0, -1.0,
            0.0, 9.0, 1.0,
            0.0, 0.0, 8.0,
        );
        let lattice = PeriodicLattice::new(
            SVector::from([1.0, -2.0, 3.0]),
            basis,
            [true, true, true],
        )
        .unwrap();

        let fractional = SVector::from([2.25, -1.75, 3.5]);
        let point = lattice.origin() + lattice.basis() * fractional;
        let wrapped = lattice.wrap_point(&point).unwrap();
        assert_eq!(wrapped.image_offset, LatticeOffset::new([2, -2, 3]));
        let restored = wrapped.position + lattice.offset_vector(wrapped.image_offset).unwrap();
        assert!((restored - point).norm() < 1e-9);

        let mut seed = 0x3344_4455_6666_7777_u64;
        for case in 0..100 {
            let mut delta = SVector::<f64, 3>::zeros();
            for axis in 0..3 {
                seed = seed
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                delta[axis] =
                    ((seed >> 11) as f64 / ((1_u64 << 53) as f64) - 0.5) * 100.0;
            }

            let result = lattice.minimum_image_displacement(&delta, 729).unwrap();
            let fractional = lattice.inverse_basis() * delta;
            let center = [
                fractional[0].round() as i64,
                fractional[1].round() as i64,
                fractional[2].round() as i64,
            ];
            let mut best = f64::INFINITY;
            for i in center[0] - 10..=center[0] + 10 {
                for j in center[1] - 10..=center[1] + 10 {
                    for k in center[2] - 10..=center[2] + 10 {
                        let shifted = delta
                            - lattice.basis()
                                * SVector::from([i as f64, j as f64, k as f64]);
                        best = best.min(shifted.norm());
                    }
                }
            }
            assert!(
                (result.displacement.norm() - best).abs() < 1e-9,
                "case={case} delta={delta:?} result={result:?} oracle={best}"
            );
            let (_, high_precision_oracle) =
                dd_closest_image(&basis, &delta, center, [true, true, true], 10);
            let selected_high_precision =
                dd_squared_distance(&basis, &delta, result.image_offset.periods);
            let selected_order = selected_high_precision.cmp(high_precision_oracle);
            assert!(
                selected_order != std::cmp::Ordering::Less,
                "bounded 3D double-double oracle missed a closer image in case {case}"
            );
            let mut ambiguity_contains_oracle = false;
            for alternative in &result.ambiguous_image_offsets {
                let alternative_distance =
                    dd_squared_distance(&basis, &delta, alternative.periods);
                let ordering = alternative_distance.cmp(high_precision_oracle);
                assert!(
                    ordering != std::cmp::Ordering::Less,
                    "reported 3D ambiguity contains an image closer than the oracle minimum"
                );
                ambiguity_contains_oracle |= ordering == std::cmp::Ordering::Equal;
            }
            if selected_order == std::cmp::Ordering::Greater {
                assert!(
                    ambiguity_contains_oracle,
                    "the double-double 3D minimum is not the representative or an ambiguity"
                );
            }
            assert!(result.unsearched_distance_lower_bound
                > result.displacement.norm() + result.nearest_distance_error_bound);
        }
    }

    #[test]
    fn nearest_image_certificate_exposes_inverse_residual_bound() {
        let lattice = orthogonal_lattice();
        assert!(lattice.inverse_residual_upper_bound().is_finite());
        assert!(lattice.inverse_residual_upper_bound() < 1e-12);
        assert!(
            lattice.sigma_min_lower_bound()
                <= 1.0 / lattice.inverse_basis().norm()
        );

        let result = lattice
            .minimum_image_displacement(&SVector::from([9.0, -7.0]), 81)
            .unwrap();
        assert_eq!(
            result.inverse_residual_upper_bound,
            lattice.inverse_residual_upper_bound()
        );
        assert!(result.unsearched_distance_lower_bound
            > result.best_candidate_distance_upper_bound);
        assert!(result.best_candidate_distance_upper_bound
            >= result.displacement.norm() - result.nearest_distance_error_bound);
    }

    #[test]
    fn skewed_minimum_image_fails_closed_when_budget_cannot_certify() {
        let basis = SMatrix::<f64, 2, 2>::new(10.0, 9.9, 0.0, 0.2);
        let lattice = PeriodicLattice::new(SVector::zeros(), basis, [true, true]).unwrap();
        let result = lattice.minimum_image_displacement(&SVector::from([0.0, 1.0]), 9);
        assert!(matches!(
            result,
            Err(PeriodicLatticeError::CandidateBudgetExceeded { .. })
                | Err(PeriodicLatticeError::UncertifiedMinimumImage { .. })
        ));
    }

    #[test]
    fn orthogonal_coordinate_rotation_preserves_offsets_and_rotates_results() {
        let basis = SMatrix::<f64, 2, 2>::new(10.0, 4.0, 0.0, 8.0);
        let origin = SVector::from([1.5, -2.0]);
        let lattice = PeriodicLattice::new(origin, basis, [true, true]).unwrap();

        let angle: f64 = 0.73;
        let rotation = SMatrix::<f64, 2, 2>::new(
            angle.cos(),
            -angle.sin(),
            angle.sin(),
            angle.cos(),
        );
        let rotated_lattice = PeriodicLattice::new(
            rotation * origin,
            rotation * basis,
            [true, true],
        )
        .unwrap();

        let point = origin + basis * SVector::from([3.25, -4.75]);
        let rotated_point = rotation * point;
        let wrapped = lattice.wrap_point(&point).unwrap();
        let rotated_wrapped = rotated_lattice.wrap_point(&rotated_point).unwrap();
        assert_eq!(wrapped.image_offset, rotated_wrapped.image_offset);
        assert!(
            (rotation * wrapped.position - rotated_wrapped.position).norm() < 1e-10
        );

        let displacement = basis * SVector::from([5.25, -3.75]);
        let result = lattice.minimum_image_displacement(&displacement, 729).unwrap();
        let rotated_result = rotated_lattice
            .minimum_image_displacement(&(rotation * displacement), 729)
            .unwrap();
        assert_eq!(result.image_offset, rotated_result.image_offset);
        assert!((rotation * result.displacement - rotated_result.displacement).norm() < 1e-9);
    }

    #[test]
    fn one_dimensional_and_nonperiodic_lattice_edges_are_explicit() {
        let one_d = PeriodicLattice::new(
            SVector::from([3.0]),
            SMatrix::<f64, 1, 1>::new(5.0),
            [true],
        )
        .unwrap();
        let point = SVector::from([19.5]);
        let wrapped = one_d.wrap_point(&point).unwrap();
        assert_eq!(wrapped.image_offset, LatticeOffset::new([3]));
        assert!((wrapped.position[0] - 4.5).abs() < 1e-12);
        let minimum = one_d
            .minimum_image_displacement(&SVector::from([12.5]), 81)
            .unwrap();
        assert!((minimum.displacement[0] - 2.5).abs() < 1e-12);

        let nonperiodic = PeriodicLattice::new(
            SVector::from([2.0, -1.0]),
            SMatrix::<f64, 2, 2>::new(3.0, 0.5, 0.0, 4.0),
            [false, false],
        )
        .unwrap();
        let unchanged_point = SVector::from([8.0, 7.0]);
        let unchanged = nonperiodic.wrap_point(&unchanged_point).unwrap();
        assert_eq!(unchanged.position, unchanged_point);
        assert_eq!(unchanged.image_offset, LatticeOffset::zero());

        let displacement = SVector::from([12.0, -9.0]);
        let unchanged_minimum = nonperiodic
            .minimum_image_displacement(&displacement, 1)
            .unwrap();
        assert_eq!(unchanged_minimum.displacement, displacement);
        assert_eq!(unchanged_minimum.candidates_evaluated, 0);
    }

    #[test]
    fn lattice_offset_translation_is_additive_and_odd() {
        let lattice = PeriodicLattice::new(
            SVector::zeros(),
            SMatrix::<f64, 2, 2>::new(10.0, 4.0, 0.0, 8.0),
            [true, true],
        )
        .unwrap();
        let offset_a = LatticeOffset::new([3, -2]);
        let offset_b = LatticeOffset::new([-1, 4]);
        let combined = LatticeOffset::new([2, 2]);

        let a = lattice.offset_vector(offset_a).unwrap();
        let b = lattice.offset_vector(offset_b).unwrap();
        let sum = lattice.offset_vector(combined).unwrap();
        let negative = lattice.offset_vector(LatticeOffset::new([-3, 2])).unwrap();

        assert!((a + b - sum).norm() < 1e-12);
        assert!((a + negative).norm() < 1e-12);
    }

    #[test]
    fn singular_basis_and_nonfinite_inputs_are_rejected() {
        let singular = SMatrix::<f64, 2, 2>::new(1.0, 2.0, 2.0, 4.0);
        assert!(matches!(
            PeriodicLattice::new(SVector::zeros(), singular, [true, true]),
            Err(PeriodicLatticeError::SingularBasis)
        ));

        let lattice = orthogonal_lattice();
        assert_eq!(
            lattice.wrap_point(&SVector::from([f64::NAN, 0.0])),
            Err(PeriodicLatticeError::NonFiniteCoordinate)
        );
    }

    #[test]
    fn rejects_point_and_displacement_beyond_local_cell_resolution() {
        let lattice = PeriodicLattice::new(
            SVector::zeros(),
            SMatrix::<f64, 2, 2>::from_diagonal(&SVector::from([1.0, 4.0])),
            [true, true],
        )
        .unwrap();

        assert!(matches!(
            lattice.wrap_point(&SVector::from([1.0e16, 0.0])),
            Err(PeriodicLatticeError::CoordinatePrecisionLimit { axis: 0 })
        ));
        assert!(matches!(
            lattice.minimum_image_displacement(&SVector::from([1.0e16, 0.0]), 81),
            Err(PeriodicLatticeError::CoordinatePrecisionLimit { axis: 0 })
        ));
        assert!(lattice
            .minimum_image_displacement(&SVector::from([1.0e9, 0.0]), 81)
            .is_ok());
    }

    #[test]
    fn rejects_cell_vectors_smaller_than_origin_resolution() {
        let basis = SMatrix::<f64, 2, 2>::new(1.0, 0.0, 0.0, 4.0);
        assert!(matches!(
            PeriodicLattice::new(
                SVector::from([1.0e16, 0.0]),
                basis,
                [true, true],
            ),
            Err(PeriodicLatticeError::UnrepresentableLatticeVector {
                lattice_axis: 0,
                cartesian_axis: 0,
            })
        ));

        let representable = SMatrix::<f64, 2, 2>::new(2.0, 0.0, 0.0, 4.0);
        assert!(PeriodicLattice::new(
            SVector::from([1.0e16, 0.0]),
            representable,
            [true, true],
        ).is_ok());
    }

    #[test]
    fn invalid_offsets_on_nonperiodic_axes_are_rejected() {
        let lattice = PeriodicLattice::new(
            SVector::zeros(),
            SMatrix::<f64, 2, 2>::identity(),
            [true, false],
        )
        .unwrap();
        assert!(matches!(
            lattice.offset_vector(LatticeOffset::new([0, 1])),
            Err(PeriodicLatticeError::OffsetOnNonPeriodicAxis { axis: 1 })
        ));
    }
}
