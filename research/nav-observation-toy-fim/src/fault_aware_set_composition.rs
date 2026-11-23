//! Exact 2D Minkowski composition of a calibrated episode ellipse and a
//! deterministic bounded common-cause fault-offset box.
//!
//! Statistical and deterministic components remain separately inspectable.
//! A finite composition is emitted only for an explicitly complete, bounded
//! fault inventory with nonzero commitments. The commitments are preserved,
//! not authenticated, by this research kernel.
//!
//! For ellipse E = {e: e^T A^-1 e <= q^2} and box
//! B = [-dx,dx] x [-dy,dy], the composed set is E (+) B. Its support function
//! and area are exact in 2D. Membership minimizes the Mahalanobis quadratic
//! over B by checking its interior and four edges; each edge has a closed-form
//! one-dimensional convex minimizer.
//!
//! This composes assumptions. It does not prove statistical exchangeability or
//! that a supplied fault box encloses every physical fault.

use std::fmt;

use crate::anisotropic_episode_tube::{
    AnisotropicEpisodeSetError, AnisotropicEpisodeTube2d,
};

const SHA256_BYTES: usize = 32;
const DIM: usize = 2;
const PI: f64 = std::f64::consts::PI;

/// Caller-provided evidence for one position. CompleteBounded asserts that the
/// symmetric box contains every position shift in the committed inventory.
/// Digests are commitments only; this module does not authenticate their bytes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FaultEnvelopeEvidence2d {
    CompleteBounded {
        half_width_x_m: f64,
        half_width_y_m: f64,
        hypothesis_inventory_sha256: [u8; SHA256_BYTES],
        bounding_evidence_sha256: [u8; SHA256_BYTES],
        declared_hypothesis_count: u64,
        hypotheses_evaluated: u64,
    },
    Incomplete {
        hypothesis_inventory_sha256: [u8; SHA256_BYTES],
        bounding_evidence_sha256: [u8; SHA256_BYTES],
        declared_hypothesis_count: u64,
        hypotheses_evaluated: u64,
    },
    Unbounded {
        hypothesis_inventory_sha256: [u8; SHA256_BYTES],
        bounding_evidence_sha256: [u8; SHA256_BYTES],
        declared_hypothesis_count: u64,
        hypotheses_evaluated: u64,
    },
}

/// A separately inspectable deterministic outer box in the base ellipse frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FaultOffsetBox2d {
    half_width_x_m: f64,
    half_width_y_m: f64,
    hypothesis_inventory_sha256: [u8; SHA256_BYTES],
    bounding_evidence_sha256: [u8; SHA256_BYTES],
    hypotheses_evaluated: u64,
}

impl FaultOffsetBox2d {
    pub fn half_width_x_m(&self) -> f64 { self.half_width_x_m }
    pub fn half_width_y_m(&self) -> f64 { self.half_width_y_m }
    pub fn hypothesis_inventory_sha256(&self) -> [u8; SHA256_BYTES] {
        self.hypothesis_inventory_sha256
    }
    pub fn bounding_evidence_sha256(&self) -> [u8; SHA256_BYTES] {
        self.bounding_evidence_sha256
    }
    pub fn hypotheses_evaluated(&self) -> u64 { self.hypotheses_evaluated }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaultAwareSetCompositionError {
    EmptyBaseTrajectory,
    FaultEnvelopeCountMismatch { expected: usize, observed: usize },
    MissingHypothesisInventoryCommitment(usize),
    MissingBoundingEvidenceCommitment(usize),
    EmptyHypothesisInventory(usize),
    HypothesisCountMismatch { index: usize, declared: u64, evaluated: u64 },
    IncompleteHypothesisInventory(usize),
    UnboundedFaultEnvelope(usize),
    InvalidFaultHalfWidth(usize),
    InvalidTruthTrajectoryLength { expected: usize, observed: usize },
    InvalidTruthPosition(usize),
    InvalidDirection,
    PositionIndexOutOfRange(usize),
    ArithmeticOverflow,
    InvalidBaseSet(AnisotropicEpisodeSetError),
}

impl fmt::Display for FaultAwareSetCompositionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyBaseTrajectory => write!(f, "base statistical trajectory must not be empty"),
            Self::FaultEnvelopeCountMismatch { expected, observed } => write!(
                f, "fault-envelope count {observed} does not match trajectory length {expected}"
            ),
            Self::MissingHypothesisInventoryCommitment(i) => write!(f, "fault inventory digest missing at position {i}"),
            Self::MissingBoundingEvidenceCommitment(i) => write!(f, "fault-bound evidence digest missing at position {i}"),
            Self::EmptyHypothesisInventory(i) => write!(f, "fault hypothesis inventory empty at position {i}"),
            Self::HypothesisCountMismatch { index, declared, evaluated } => write!(f, "fault hypotheses evaluated {evaluated} but inventory declares {declared} at position {index}"),
            Self::IncompleteHypothesisInventory(i) => write!(f, "fault hypothesis inventory incomplete at position {i}"),
            Self::UnboundedFaultEnvelope(i) => write!(f, "fault envelope unbounded at position {i}"),
            Self::InvalidFaultHalfWidth(i) => write!(f, "fault half-width invalid at position {i}"),
            Self::InvalidTruthTrajectoryLength { expected, observed } => write!(
                f, "truth trajectory has {observed} positions but composed tube has {expected}"
            ),
            Self::InvalidTruthPosition(i) => write!(f, "truth position {i} is non-finite"),
            Self::InvalidDirection => write!(f, "support direction must be finite and nonzero"),
            Self::PositionIndexOutOfRange(i) => write!(f, "position index {i} is out of range"),
            Self::ArithmeticOverflow => write!(f, "fault-aware set arithmetic overflowed"),
            Self::InvalidBaseSet(error) => write!(f, "base statistical set rejected input: {error}"),
        }
    }
}

impl std::error::Error for FaultAwareSetCompositionError {}

/// A statistical ellipse plus its separate deterministic fault envelope at
/// every position in one trajectory episode.
#[derive(Clone, Debug, PartialEq)]
pub struct FaultAwareAnisotropicEpisodeTube2d {
    statistical_tube: AnisotropicEpisodeTube2d,
    fault_boxes: Vec<FaultOffsetBox2d>,
}

impl FaultAwareAnisotropicEpisodeTube2d {
    /// Incomplete and unbounded records never fall back to the statistical
    /// ellipse alone.
    pub fn compose(
        statistical_tube: AnisotropicEpisodeTube2d,
        evidence: &[FaultEnvelopeEvidence2d],
    ) -> Result<Self, FaultAwareSetCompositionError> {
        let expected = statistical_tube.centers_m().len();
        if expected == 0 {
            return Err(FaultAwareSetCompositionError::EmptyBaseTrajectory);
        }
        if evidence.len() != expected {
            return Err(FaultAwareSetCompositionError::FaultEnvelopeCountMismatch {
                expected,
                observed: evidence.len(),
            });
        }

        let mut fault_boxes = Vec::with_capacity(evidence.len());
        for (index, item) in evidence.iter().copied().enumerate() {
            let (inventory, bound, declared_count, evaluated_count) = match item {
                FaultEnvelopeEvidence2d::CompleteBounded {
                    hypothesis_inventory_sha256, bounding_evidence_sha256,
                    declared_hypothesis_count, hypotheses_evaluated, ..
                }
                | FaultEnvelopeEvidence2d::Incomplete {
                    hypothesis_inventory_sha256, bounding_evidence_sha256,
                    declared_hypothesis_count, hypotheses_evaluated,
                }
                | FaultEnvelopeEvidence2d::Unbounded {
                    hypothesis_inventory_sha256, bounding_evidence_sha256,
                    declared_hypothesis_count, hypotheses_evaluated,
                } => (hypothesis_inventory_sha256, bounding_evidence_sha256, declared_hypothesis_count, hypotheses_evaluated),
            };

            if inventory.iter().all(|byte| *byte == 0) {
                return Err(FaultAwareSetCompositionError::MissingHypothesisInventoryCommitment(index));
            }
            if bound.iter().all(|byte| *byte == 0) {
                return Err(FaultAwareSetCompositionError::MissingBoundingEvidenceCommitment(index));
            }
            if declared_count == 0 || evaluated_count == 0 {
                return Err(FaultAwareSetCompositionError::EmptyHypothesisInventory(index));
            }
            if matches!(item, FaultEnvelopeEvidence2d::CompleteBounded { .. })
                && evaluated_count != declared_count
            {
                return Err(FaultAwareSetCompositionError::HypothesisCountMismatch {
                    index, declared: declared_count, evaluated: evaluated_count,
                });
            }

            match item {
                FaultEnvelopeEvidence2d::CompleteBounded {
                    half_width_x_m, half_width_y_m,
                    hypothesis_inventory_sha256, bounding_evidence_sha256,
                    declared_hypothesis_count: _,
                    hypotheses_evaluated,
                } => {
                    if !half_width_x_m.is_finite() || half_width_x_m < 0.0
                        || !half_width_y_m.is_finite() || half_width_y_m < 0.0
                    {
                        return Err(FaultAwareSetCompositionError::InvalidFaultHalfWidth(index));
                    }
                    fault_boxes.push(FaultOffsetBox2d {
                        half_width_x_m, half_width_y_m,
                        hypothesis_inventory_sha256, bounding_evidence_sha256,
                        hypotheses_evaluated,
                    });
                }
                FaultEnvelopeEvidence2d::Incomplete { .. } => {
                    return Err(FaultAwareSetCompositionError::IncompleteHypothesisInventory(index));
                }
                FaultEnvelopeEvidence2d::Unbounded { .. } => {
                    return Err(FaultAwareSetCompositionError::UnboundedFaultEnvelope(index));
                }
            }
        }

        Ok(Self { statistical_tube, fault_boxes })
    }

    pub fn statistical_tube(&self) -> &AnisotropicEpisodeTube2d {
        &self.statistical_tube
    }

    pub fn fault_boxes(&self) -> &[FaultOffsetBox2d] {
        &self.fault_boxes
    }

    pub fn position_count(&self) -> usize {
        self.fault_boxes.len()
    }

    /// Exact support distance in metres. The input direction is normalized
    /// internally, so a non-unit vector yields the same distance as its ray.
    pub fn directional_support_m(
        &self,
        time_index: usize,
        direction: [f64; DIM],
    ) -> Result<f64, FaultAwareSetCompositionError> {
        let fault = self.fault_boxes.get(time_index)
            .ok_or(FaultAwareSetCompositionError::PositionIndexOutOfRange(time_index))?;
        let shape = *self.statistical_tube.shape_matrices().get(time_index)
            .ok_or(FaultAwareSetCompositionError::PositionIndexOutOfRange(time_index))?;
        if !direction.iter().all(|value| value.is_finite()) {
            return Err(FaultAwareSetCompositionError::InvalidDirection);
        }
        let norm = direction[0].hypot(direction[1]);
        if !norm.is_finite() || norm <= 0.0 {
            return Err(FaultAwareSetCompositionError::InvalidDirection);
        }
        let ux = direction[0] / norm;
        let uy = direction[1] / norm;

        let scale = shape.iter().flatten().map(|v| v.abs()).fold(0.0_f64, f64::max);
        if !scale.is_finite() || scale <= 0.0 {
            return Err(FaultAwareSetCompositionError::ArithmeticOverflow);
        }
        let a = shape[0][0] / scale;
        let b = shape[0][1] / scale / 2.0 + shape[1][0] / scale / 2.0;
        let d = shape[1][1] / scale;
        let quadratic = a * ux * ux + 2.0 * b * ux * uy + d * uy * uy;
        if !quadratic.is_finite() || quadratic <= 0.0 {
            return Err(FaultAwareSetCompositionError::ArithmeticOverflow);
        }

        let ellipse_support = self.statistical_tube.normalized_radius()
            * scale.sqrt() * quadratic.sqrt();
        let box_support = fault.half_width_x_m * ux.abs()
            + fault.half_width_y_m * uy.abs();
        let total = ellipse_support + box_support;
        if !ellipse_support.is_finite() || !box_support.is_finite() || !total.is_finite() {
            return Err(FaultAwareSetCompositionError::ArithmeticOverflow);
        }
        Ok(total)
    }

    /// Exact area of the 2D ellipse-plus-axis-aligned-rectangle Minkowski sum.
    pub fn cross_section_area_m2(
        &self,
        time_index: usize,
    ) -> Result<f64, FaultAwareSetCompositionError> {
        let fault = self.fault_boxes.get(time_index)
            .ok_or(FaultAwareSetCompositionError::PositionIndexOutOfRange(time_index))?;
        let shape = *self.statistical_tube.shape_matrices().get(time_index)
            .ok_or(FaultAwareSetCompositionError::PositionIndexOutOfRange(time_index))?;
        let scale = shape.iter().flatten().map(|v| v.abs()).fold(0.0_f64, f64::max);
        if !scale.is_finite() || scale <= 0.0 {
            return Err(FaultAwareSetCompositionError::ArithmeticOverflow);
        }
        let a = shape[0][0] / scale;
        let b = shape[0][1] / scale / 2.0 + shape[1][0] / scale / 2.0;
        let d = shape[1][1] / scale;
        let det_scaled = a * d - b * b;
        if !det_scaled.is_finite() || det_scaled <= 0.0 {
            return Err(FaultAwareSetCompositionError::ArithmeticOverflow);
        }

        let q = self.statistical_tube.normalized_radius();
        let q2 = q * q;
        let root_det = scale * det_scaled.sqrt();
        let ellipse_area = PI * q2 * root_det;
        let x_extent = q * shape[0][0].sqrt();
        let y_extent = q * shape[1][1].sqrt();
        // Build the box from horizontal and vertical segments. Each area
        // increment is segment length times orthogonal projection width.
        let horizontal = 4.0 * fault.half_width_x_m * y_extent;
        let vertical = 4.0 * fault.half_width_y_m * x_extent;
        let rectangle = 4.0 * fault.half_width_x_m * fault.half_width_y_m;
        let total = ellipse_area + horizontal + vertical + rectangle;
        if !q2.is_finite() || !root_det.is_finite() || !ellipse_area.is_finite()
            || !x_extent.is_finite() || !y_extent.is_finite()
            || !horizontal.is_finite() || !vertical.is_finite()
            || !rectangle.is_finite() || !total.is_finite() || total < 0.0
        {
            return Err(FaultAwareSetCompositionError::ArithmeticOverflow);
        }
        Ok(total)
    }

    /// Sum of per-position cross-sectional areas, not swept volume or path
    /// union area.
    pub fn summed_cross_section_area_m2(
        &self,
    ) -> Result<f64, FaultAwareSetCompositionError> {
        let mut total = 0.0_f64;
        for index in 0..self.position_count() {
            total += self.cross_section_area_m2(index)?;
            if !total.is_finite() {
                return Err(FaultAwareSetCompositionError::ArithmeticOverflow);
            }
        }
        Ok(total)
    }

    /// Membership in E (+) B. If the residual lies inside the box, membership
    /// is immediate. Otherwise the convex minimum lies on an edge; each edge
    /// is minimized analytically and clamped to its segment (including corners).
    pub fn contains_position(
        &self,
        time_index: usize,
        truth_position_m: [f64; DIM],
    ) -> Result<bool, FaultAwareSetCompositionError> {
        let fault = self.fault_boxes.get(time_index)
            .ok_or(FaultAwareSetCompositionError::PositionIndexOutOfRange(time_index))?;
        if !truth_position_m.iter().all(|v| v.is_finite()) {
            return Err(FaultAwareSetCompositionError::InvalidTruthPosition(time_index));
        }
        let center = *self.statistical_tube.centers_m().get(time_index)
            .ok_or(FaultAwareSetCompositionError::PositionIndexOutOfRange(time_index))?;
        let shape = *self.statistical_tube.shape_matrices().get(time_index)
            .ok_or(FaultAwareSetCompositionError::PositionIndexOutOfRange(time_index))?;
        let ex = truth_position_m[0] - center[0];
        let ey = truth_position_m[1] - center[1];
        if !ex.is_finite() || !ey.is_finite() {
            return Err(FaultAwareSetCompositionError::ArithmeticOverflow);
        }

        let dx = fault.half_width_x_m;
        let dy = fault.half_width_y_m;
        if ex.abs() <= dx && ey.abs() <= dy {
            return Ok(true);
        }
        let a = shape[0][0];
        let b = shape[0][1] / 2.0 + shape[1][0] / 2.0;
        let d = shape[1][1];
        if !a.is_finite() || !b.is_finite() || !d.is_finite() || a <= 0.0 || d <= 0.0 {
            return Err(FaultAwareSetCompositionError::ArithmeticOverflow);
        }

        // Fix x-box coordinate; minimize over y-box coordinate.
        for bx in [-dx, dx] {
            let rx = ex - bx;
            let by_unconstrained = ey - (b / a) * rx;
            if !rx.is_finite() || !by_unconstrained.is_finite() {
                return Err(FaultAwareSetCompositionError::ArithmeticOverflow);
            }
            let by = by_unconstrained.clamp(-dy, dy);
            if self.candidate_is_inside(time_index, truth_position_m, [bx, by])? {
                return Ok(true);
            }
        }
        // Fix y-box coordinate; minimize over x-box coordinate.
        for by in [-dy, dy] {
            let ry = ey - by;
            let bx_unconstrained = ex - (b / d) * ry;
            if !ry.is_finite() || !bx_unconstrained.is_finite() {
                return Err(FaultAwareSetCompositionError::ArithmeticOverflow);
            }
            let bx = bx_unconstrained.clamp(-dx, dx);
            if self.candidate_is_inside(time_index, truth_position_m, [bx, by])? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn candidate_is_inside(
        &self,
        time_index: usize,
        truth_position_m: [f64; DIM],
        box_offset_m: [f64; DIM],
    ) -> Result<bool, FaultAwareSetCompositionError> {
        let residual_position = [
            truth_position_m[0] - box_offset_m[0],
            truth_position_m[1] - box_offset_m[1],
        ];
        if !residual_position.iter().all(|v| v.is_finite()) {
            return Err(FaultAwareSetCompositionError::ArithmeticOverflow);
        }
        self.statistical_tube.contains_position(time_index, residual_position)
            .map_err(FaultAwareSetCompositionError::InvalidBaseSet)
    }

    /// Complete-episode hit only. Truncated or non-finite truth is an error.
    pub fn contains_complete_trajectory(
        &self,
        truth_positions_m: &[[f64; DIM]],
    ) -> Result<bool, FaultAwareSetCompositionError> {
        if truth_positions_m.len() != self.position_count() {
            return Err(FaultAwareSetCompositionError::InvalidTruthTrajectoryLength {
                expected: self.position_count(),
                observed: truth_positions_m.len(),
            });
        }
        if let Some(index) = truth_positions_m.iter()
            .position(|position| !position.iter().all(|v| v.is_finite()))
        {
            return Err(FaultAwareSetCompositionError::InvalidTruthPosition(index));
        }
        for (index, truth) in truth_positions_m.iter().enumerate() {
            if !self.contains_position(index, *truth)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anisotropic_episode_tube::{
        AnisotropicCalibrationEpisode2d, AnisotropicEpisodeFamilyConfig,
        AnisotropicEpisodeConformalFamily2d, AnisotropicEpisodeSetOutcome2d,
    };

    fn calibration_episode(
        id: u64,
        predicted: [f64; DIM],
        truth: [f64; DIM],
        shape: [[f64; DIM]; DIM],
    ) -> AnisotropicCalibrationEpisode2d {
        AnisotropicCalibrationEpisode2d {
            episode_id: id,
            independence_group_id: id + 10_000,
            predicted_positions_m: vec![predicted],
            truth_positions_m: vec![truth],
            shape_matrices: vec![shape],
            source_manifest_sha256: [((id % 255) + 1) as u8; SHA256_BYTES],
        }
    }

    fn base_tube(shape: [[f64; DIM]; DIM]) -> AnisotropicEpisodeTube2d {
        let episodes = vec![
            calibration_episode(1, [0.0, 0.0], [1.0, 0.0], shape),
            calibration_episode(2, [0.0, 0.0], [0.0, 1.0], shape),
            calibration_episode(3, [0.0, 0.0], [2.0, 0.0], shape),
            calibration_episode(4, [0.0, 0.0], [0.0, 2.0], shape),
        ];
        let family = AnisotropicEpisodeConformalFamily2d::fit(
            &episodes,
            AnisotropicEpisodeFamilyConfig {
                calibration_cohort_sha256: [0xA1; SHA256_BYTES],
                predictor_sha256: [0xB2; SHA256_BYTES],
                shape_definition_sha256: [0xC3; SHA256_BYTES],
                symmetry_tolerance: 1.0e-12,
                minimum_relative_eigenvalue: 1.0e-8,
            },
        ).unwrap();
        let outcome = family.build_episode_tube(
            500, 20_500, &[[0.0, 0.0]], &[shape], 0.2,
            [0xB2; SHA256_BYTES], [0xC3; SHA256_BYTES], 501, [0xD4; SHA256_BYTES],
        ).unwrap();
        let AnisotropicEpisodeSetOutcome2d::Ellipses(tube) = outcome else {
            panic!("expected finite calibrated ellipse");
        };
        tube
    }

    fn complete_box(dx: f64, dy: f64, domain: u8) -> FaultEnvelopeEvidence2d {
        FaultEnvelopeEvidence2d::CompleteBounded {
            half_width_x_m: dx,
            half_width_y_m: dy,
            hypothesis_inventory_sha256: [domain; SHA256_BYTES],
            bounding_evidence_sha256: [domain.wrapping_add(1); SHA256_BYTES],
            declared_hypothesis_count: 3,
            hypotheses_evaluated: 3,
        }
    }

    fn compose(
        shape: [[f64; DIM]; DIM],
        dx: f64,
        dy: f64,
    ) -> FaultAwareAnisotropicEpisodeTube2d {
        FaultAwareAnisotropicEpisodeTube2d::compose(
            base_tube(shape),
            &[complete_box(dx, dy, 0xD0)],
        ).unwrap()
    }

    #[test]
    fn zero_fault_box_recovers_statistical_ellipse_membership_and_area() {
        let statistical = base_tube([[4.0, 0.0], [0.0, 1.0]]);
        let composed = FaultAwareAnisotropicEpisodeTube2d::compose(
            statistical.clone(), &[complete_box(0.0, 0.0, 0xD0)],
        ).unwrap();
        for truth in [[0.0, 0.0], [4.0, 0.0], [0.0, 2.0], [4.1, 0.0], [0.0, 2.1]] {
            assert_eq!(
                composed.contains_position(0, truth).unwrap(),
                statistical.contains_position(0, truth).unwrap()
            );
        }
        let expected = PI * statistical.normalized_radius().powi(2) * 2.0;
        assert!((composed.cross_section_area_m2(0).unwrap() - expected).abs() < 1.0e-10);
    }

    #[test]
    fn directional_support_is_exact_sum_of_ellipse_and_box_support() {
        let composed = compose([[4.0, 0.0], [0.0, 1.0]], 0.5, 0.25);
        let q = composed.statistical_tube().normalized_radius();
        assert!((composed.directional_support_m(0, [1.0, 0.0]).unwrap() - (2.0 * q + 0.5)).abs() < 1.0e-10);
        assert!((composed.directional_support_m(0, [0.0, 1.0]).unwrap() - (q + 0.25)).abs() < 1.0e-10);
        assert_eq!(
            composed.directional_support_m(0, [10.0, 0.0]).unwrap(),
            composed.directional_support_m(0, [1.0, 0.0]).unwrap()
        );
    }

    #[test]
    fn ellipse_plus_box_area_matches_exact_minkowski_formula() {
        let composed = compose([[4.0, 0.0], [0.0, 1.0]], 0.5, 0.25);
        let q = composed.statistical_tube().normalized_radius();
        let expected = PI * q * q * 2.0
            + 4.0 * 0.5 * q
            + 4.0 * 0.25 * (2.0 * q)
            + 4.0 * 0.5 * 0.25;
        assert!((composed.cross_section_area_m2(0).unwrap() - expected).abs() < 1.0e-10);
    }

    #[test]
    fn edge_minimizers_include_exact_boundary_and_reject_outside_point() {
        let composed = compose([[4.0, 0.0], [0.0, 1.0]], 0.5, 0.25);
        let q = composed.statistical_tube().normalized_radius();
        let boundary_x = 2.0 * q + 0.5;
        assert!(composed.contains_position(0, [boundary_x, 0.25]).unwrap());
        assert!(!composed.contains_position(0, [boundary_x + 1.0e-6, 0.25]).unwrap());
        assert!(composed.contains_position(0, [0.0, q + 0.25]).unwrap());
        assert!(composed.contains_position(0, [0.5, 0.25]).unwrap());
    }

    #[test]
    fn rotated_spd_shape_is_supported() {
        let composed = compose([[4.0, 1.5], [1.5, 1.0]], 0.3, 0.2);
        assert!(composed.directional_support_m(0, [1.0, 1.0]).unwrap() > 0.0);
        assert!(composed.contains_position(0, [0.3, 0.2]).unwrap());
    }

    #[test]
    fn rotated_ellipse_edge_minimizer_includes_support_boundary_and_rejects_outside() {
        let composed = compose([[4.0, 1.5], [1.5, 1.0]], 0.3, 0.2);
        let q = composed.statistical_tube().normalized_radius();
        // The ellipse support point along +x is q*A*[1,0]/sqrt(A_xx).
        let boundary = [2.0 * q + 0.3, 0.75 * q + 0.2];
        assert!(composed.contains_position(0, boundary).unwrap());
        assert!(!composed.contains_position(0, [boundary[0] + 1.0e-5, boundary[1]]).unwrap());
    }

    #[test]
    fn complete_inventory_requires_evaluated_count_to_match_declaration() {
        let mut evidence = complete_box(0.1, 0.1, 0xD0);
        if let FaultEnvelopeEvidence2d::CompleteBounded {
            hypotheses_evaluated, ..
        } = &mut evidence {
            *hypotheses_evaluated = 2;
        }
        assert_eq!(
            FaultAwareAnisotropicEpisodeTube2d::compose(
                base_tube([[4.0, 0.0], [0.0, 1.0]]), &[evidence],
            ),
            Err(FaultAwareSetCompositionError::HypothesisCountMismatch {
                index: 0, declared: 3, evaluated: 2,
            })
        );
    }

    #[test]
    fn incomplete_or_unbounded_fault_inventory_never_falls_back_to_ellipse() {
        let base = base_tube([[4.0, 0.0], [0.0, 1.0]]);
        let incomplete = FaultEnvelopeEvidence2d::Incomplete {
            hypothesis_inventory_sha256: [0xD0; SHA256_BYTES],
            bounding_evidence_sha256: [0xD1; SHA256_BYTES],
            declared_hypothesis_count: 3,
            hypotheses_evaluated: 3,
        };
        assert_eq!(
            FaultAwareAnisotropicEpisodeTube2d::compose(base.clone(), &[incomplete]),
            Err(FaultAwareSetCompositionError::IncompleteHypothesisInventory(0))
        );
        let unbounded = FaultEnvelopeEvidence2d::Unbounded {
            hypothesis_inventory_sha256: [0xD0; SHA256_BYTES],
            bounding_evidence_sha256: [0xD1; SHA256_BYTES],
            declared_hypothesis_count: 3,
            hypotheses_evaluated: 3,
        };
        assert_eq!(
            FaultAwareAnisotropicEpisodeTube2d::compose(base, &[unbounded]),
            Err(FaultAwareSetCompositionError::UnboundedFaultEnvelope(0))
        );
    }

    #[test]
    fn missing_commitments_empty_inventory_and_invalid_widths_reject() {
        let base = base_tube([[4.0, 0.0], [0.0, 1.0]]);
        let mut missing = complete_box(0.1, 0.1, 0xD0);
        if let FaultEnvelopeEvidence2d::CompleteBounded {
            hypothesis_inventory_sha256, ..
        } = &mut missing {
            *hypothesis_inventory_sha256 = [0; SHA256_BYTES];
        }
        assert_eq!(
            FaultAwareAnisotropicEpisodeTube2d::compose(base.clone(), &[missing]),
            Err(FaultAwareSetCompositionError::MissingHypothesisInventoryCommitment(0))
        );
        let empty = FaultEnvelopeEvidence2d::CompleteBounded {
            half_width_x_m: 0.1, half_width_y_m: 0.1,
            hypothesis_inventory_sha256: [0xD0; SHA256_BYTES],
            bounding_evidence_sha256: [0xD1; SHA256_BYTES],
            declared_hypothesis_count: 3,
            hypotheses_evaluated: 0,
        };
        assert_eq!(
            FaultAwareAnisotropicEpisodeTube2d::compose(base.clone(), &[empty]),
            Err(FaultAwareSetCompositionError::EmptyHypothesisInventory(0))
        );
        assert_eq!(
            FaultAwareAnisotropicEpisodeTube2d::compose(
                base.clone(), &[complete_box(f64::NAN, 0.1, 0xD0)]
            ),
            Err(FaultAwareSetCompositionError::InvalidFaultHalfWidth(0))
        );
        assert_eq!(
            FaultAwareAnisotropicEpisodeTube2d::compose(
                base, &[complete_box(0.1, -0.1, 0xD0)]
            ),
            Err(FaultAwareSetCompositionError::InvalidFaultHalfWidth(0))
        );
    }

    #[test]
    fn envelope_count_and_complete_truth_contract_are_enforced() {
        let composed = compose([[4.0, 0.0], [0.0, 1.0]], 0.1, 0.1);
        assert_eq!(
            composed.contains_complete_trajectory(&[]),
            Err(FaultAwareSetCompositionError::InvalidTruthTrajectoryLength {
                expected: 1, observed: 0,
            })
        );
        assert_eq!(
            composed.contains_complete_trajectory(&[[f64::NAN, 0.0]]),
            Err(FaultAwareSetCompositionError::InvalidTruthPosition(0))
        );
        assert!(composed.contains_complete_trajectory(&[[0.0, 0.0]]).unwrap());
        assert_eq!(
            FaultAwareAnisotropicEpisodeTube2d::compose(
                base_tube([[4.0, 0.0], [0.0, 1.0]]), &[]
            ),
            Err(FaultAwareSetCompositionError::FaultEnvelopeCountMismatch {
                expected: 1, observed: 0,
            })
        );
    }

    #[test]
    fn widening_box_never_shrinks_area_support_or_membership() {
        let narrow = compose([[4.0, 0.0], [0.0, 1.0]], 0.1, 0.1);
        let wide = compose([[4.0, 0.0], [0.0, 1.0]], 0.5, 0.4);
        assert!(wide.cross_section_area_m2(0).unwrap() >= narrow.cross_section_area_m2(0).unwrap());
        for direction in [[1.0, 0.0], [0.0, 1.0], [1.0, 1.0], [-1.0, 2.0]] {
            assert!(wide.directional_support_m(0, direction).unwrap()
                >= narrow.directional_support_m(0, direction).unwrap());
        }
        assert!(wide.contains_position(0, [2.0, 0.0]).unwrap());
    }

    #[test]
    fn diagonal_containment_matches_independent_box_clipping_reference() {
        let composed = compose([[4.0, 0.0], [0.0, 1.0]], 0.5, 0.25);
        let q = composed.statistical_tube().normalized_radius();
        for ix in -12..=12 {
            for iy in -12..=12 {
                let truth = [ix as f64 * 0.25, iy as f64 * 0.25];
                let rx = (truth[0].abs() - 0.5).max(0.0);
                let ry = (truth[1].abs() - 0.25).max(0.0);
                let reference = (rx * rx / 4.0 + ry * ry).sqrt();
                assert_eq!(
                    composed.contains_position(0, truth).unwrap(),
                    reference <= q,
                    "truth={truth:?}, reference={reference}, q={q}"
                );
            }
        }
    }
}
