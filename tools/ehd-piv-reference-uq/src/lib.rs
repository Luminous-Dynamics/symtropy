// SPDX-License-Identifier: AGPL-3.0-or-later

//! Reference-aware decomposition of PIV-minus-reference covariance.
//!
//! For r = e_piv - e_ref:
//!
//! C_r = C_piv + C_ref - C_piv,ref - C_ref,piv.
//!
//! The evaluator only emits a PIV-only covariance when an external reference
//! assessment explicitly qualifies the required evidence and cross-covariance.
//! Otherwise it preserves the residual covariance claim and abstains from
//! separating the PIV and reference contributions.
//!
//! This is a fixed-size 3-component covariance reference, not an image
//! processor, a physical reference calibration system, or EHD validation.

#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt;

pub type Matrix3 = [[f64; 3]; 3];
pub type Matrix6 = [[f64; 6]; 6];

const SYMMETRY_RELATIVE_TOLERANCE: f64 = 1.0e-10;
const PSD_NEGATIVE_EIGENVALUE_TOLERANCE: f64 = 1.0e-10;
const JACOBI_OFF_DIAGONAL_TOLERANCE: f64 = 1.0e-14;
const RECONSTRUCTION_RELATIVE_TOLERANCE: f64 = 1.0e-10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReferenceTraceability {
    Traceable,
    Untraceable,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReferenceDependence {
    Independent,
    PartiallyShared,
    Dependent,
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReferenceAssessment {
    pub reference_id: String,
    pub assessment_id: String,
    pub modality: String,
    pub measurand: String,
    pub instrument_identity: String,
    pub protocol_fingerprint: String,
    /// Digest of the exact source manifest; this crate does not rehash assets.
    pub source_manifest_sha256: String,
    /// Digest of the reference calibration evidence bundle; it is not a
    /// certificate-authentication or traceability verifier.
    pub calibration_receipt_sha256: String,
    /// External assessment ID covering shared causes and cross-covariance.
    pub shared_cause_assessment_id: String,
    pub traceability: ReferenceTraceability,
    pub dependence: ReferenceDependence,
    pub uncertainty_budget_complete: bool,
    pub validity_interval_qualified: bool,
    pub operating_range_qualified: bool,
    pub bandwidth_response_qualified: bool,
    pub time_synchronization_qualified: bool,
    pub coordinate_registration_qualified: bool,
    pub drift_assessed: bool,
    pub cross_covariance_quantified: bool,
    pub common_causes_reviewed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceGap {
    MissingReferenceIdentity,
    MissingAssessmentIdentity,
    MissingModality,
    MissingMeasurand,
    MissingInstrumentIdentity,
    MissingProtocolFingerprint,
    InvalidSourceManifestDigest,
    InvalidCalibrationReceiptDigest,
    MissingSharedCauseAssessment,
    ReferenceNotTraceable,
    ReferenceDependenceUnknown,
    ReferenceDependenceUnqualified,
    IncompleteUncertaintyBudget,
    ValidityIntervalUnqualified,
    OperatingRangeUnqualified,
    BandwidthResponseUnqualified,
    TimeSynchronizationUnqualified,
    CoordinateRegistrationUnqualified,
    DriftNotAssessed,
    CrossCovarianceUnquantified,
    CommonCausesNotReviewed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CovarianceKind {
    Residual,
    Reference,
    DerivedPiv,
    JointPivReference,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ReferenceUqError {
    NonFiniteCovariance {
        kind: CovarianceKind,
        row: usize,
        column: usize,
    },
    NegativeVariance {
        kind: CovarianceKind,
        index: usize,
    },
    AsymmetricCovariance {
        kind: CovarianceKind,
        row: usize,
        column: usize,
    },
    ZeroVarianceWithNonzeroCovariance {
        kind: CovarianceKind,
        index: usize,
        other_index: usize,
    },
    NonPositiveSemidefinite {
        kind: CovarianceKind,
        minimum_correlation_eigenvalue: f64,
    },
    EigenSolverDidNotConverge {
        dimension: usize,
    },
    NonFiniteDerivedCovariance,
    ReconstructionClosureFailure {
        maximum_absolute_error: f64,
        allowed_absolute_error: f64,
    },
}

impl fmt::Display for ReferenceUqError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteCovariance { kind, row, column } => {
                write!(f, "{kind:?} covariance ({row}, {column}) is non-finite")
            }
            Self::NegativeVariance { kind, index } => {
                write!(f, "{kind:?} covariance variance {index} is negative")
            }
            Self::AsymmetricCovariance { kind, row, column } => write!(
                f,
                "{kind:?} covariance entries ({row}, {column}) are materially asymmetric"
            ),
            Self::ZeroVarianceWithNonzeroCovariance {
                kind,
                index,
                other_index,
            } => write!(
                f,
                "{kind:?} covariance zero-variance component {index} has nonzero covariance with {other_index}"
            ),
            Self::NonPositiveSemidefinite {
                kind,
                minimum_correlation_eigenvalue,
            } => write!(
                f,
                "{kind:?} covariance is not positive semidefinite (minimum normalized eigenvalue {minimum_correlation_eigenvalue})"
            ),
            Self::EigenSolverDidNotConverge { dimension } => write!(
                f,
                "symmetric Jacobi eigensolver did not converge for dimension {dimension}"
            ),
            Self::NonFiniteDerivedCovariance => {
                write!(f, "derived reference/PIV covariance is non-finite")
            }
            Self::ReconstructionClosureFailure {
                maximum_absolute_error,
                allowed_absolute_error,
            } => write!(
                f,
                "covariance reconstruction error {maximum_absolute_error} exceeds {allowed_absolute_error}"
            ),
        }
    }
}

impl Error for ReferenceUqError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReferenceCovarianceDisposition {
    PivOnlyDecompositionQualified,
    ResidualCovarianceOnly,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReferenceCovarianceInput {
    /// Empirical covariance of the measured-minus-reference residual field.
    pub residual_covariance_m2_s2: Matrix3,
    /// Independently assessed reference-instrument error covariance.
    pub reference_covariance_m2_s2: Matrix3,
    /// Cov(e_piv, e_ref); this matrix need not be symmetric.
    pub piv_reference_cross_covariance_m2_s2: Matrix3,
    pub reference_assessment: ReferenceAssessment,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReferenceCovarianceReport {
    pub disposition: ReferenceCovarianceDisposition,
    pub reference_id: Option<String>,
    pub assessment_id: Option<String>,
    pub evidence_gaps: Vec<EvidenceGap>,
    pub residual_covariance_m2_s2: Matrix3,
    /// Absent unless the reference and cross-covariance evidence is qualified.
    pub piv_covariance_m2_s2: Option<Matrix3>,
    pub residual_minimum_correlation_eigenvalue: f64,
    pub reference_minimum_correlation_eigenvalue: Option<f64>,
    pub piv_minimum_correlation_eigenvalue: Option<f64>,
    pub joint_minimum_correlation_eigenvalue: Option<f64>,
    pub reconstructed_residual_covariance_m2_s2: Option<Matrix3>,
    pub maximum_reconstruction_error_m2_s2: Option<f64>,
    pub reconstruction_tolerance_m2_s2: Option<f64>,
    pub claim_boundary: &'static str,
}

/// Evaluate a residual covariance and, only when the supplied reference
/// assessment qualifies, decompose it into PIV, reference and cross-covariance.
///
/// Evidence gaps result in an explicit residual-only report, not an assumed
/// zero reference error or zero PIV/reference cross-covariance.
pub fn evaluate_reference_covariance(
    input: &ReferenceCovarianceInput,
) -> Result<ReferenceCovarianceReport, ReferenceUqError> {
    let residual_minimum = validate_covariance(
        &input.residual_covariance_m2_s2,
        CovarianceKind::Residual,
    )?;
    let evidence_gaps = reference_evidence_gaps(&input.reference_assessment);

    if !evidence_gaps.is_empty() {
        return Ok(ReferenceCovarianceReport {
            disposition: ReferenceCovarianceDisposition::ResidualCovarianceOnly,
            reference_id: nonempty(&input.reference_assessment.reference_id)
                .then(|| input.reference_assessment.reference_id.clone()),
            assessment_id: nonempty(&input.reference_assessment.assessment_id)
                .then(|| input.reference_assessment.assessment_id.clone()),
            evidence_gaps,
            residual_covariance_m2_s2: input.residual_covariance_m2_s2,
            piv_covariance_m2_s2: None,
            residual_minimum_correlation_eigenvalue: residual_minimum,
            reference_minimum_correlation_eigenvalue: None,
            piv_minimum_correlation_eigenvalue: None,
            joint_minimum_correlation_eigenvalue: None,
            reconstructed_residual_covariance_m2_s2: None,
            maximum_reconstruction_error_m2_s2: None,
            reconstruction_tolerance_m2_s2: None,
            claim_boundary: "PIV-minus-reference residual covariance only; reference and/or cross-covariance evidence is insufficient for PIV-only decomposition",
        });
    }

    let reference_minimum = validate_covariance(
        &input.reference_covariance_m2_s2,
        CovarianceKind::Reference,
    )?;
    validate_finite_matrix(
        &input.piv_reference_cross_covariance_m2_s2,
        CovarianceKind::JointPivReference,
    )?;

    let mut piv_covariance = [[0.0_f64; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            piv_covariance[row][column] = input.residual_covariance_m2_s2[row][column]
                - input.reference_covariance_m2_s2[row][column]
                + input.piv_reference_cross_covariance_m2_s2[row][column]
                + input.piv_reference_cross_covariance_m2_s2[column][row];
        }
    }
    if !piv_covariance.iter().flatten().all(|value| value.is_finite()) {
        return Err(ReferenceUqError::NonFiniteDerivedCovariance);
    }

    let piv_minimum = validate_covariance(&piv_covariance, CovarianceKind::DerivedPiv)?;
    let joint = build_joint_covariance(
        &piv_covariance,
        &input.reference_covariance_m2_s2,
        &input.piv_reference_cross_covariance_m2_s2,
    );
    let joint_minimum = validate_covariance(&joint, CovarianceKind::JointPivReference)?;

    let reconstructed = reconstruct_residual_covariance(
        &piv_covariance,
        &input.reference_covariance_m2_s2,
        &input.piv_reference_cross_covariance_m2_s2,
    );
    let (maximum_error, tolerance) = reconstruction_error(
        &input.residual_covariance_m2_s2,
        &reconstructed,
    );
    if maximum_error > tolerance {
        return Err(ReferenceUqError::ReconstructionClosureFailure {
            maximum_absolute_error: maximum_error,
            allowed_absolute_error: tolerance,
        });
    }

    Ok(ReferenceCovarianceReport {
        disposition: ReferenceCovarianceDisposition::PivOnlyDecompositionQualified,
        reference_id: Some(input.reference_assessment.reference_id.clone()),
        assessment_id: Some(input.reference_assessment.assessment_id.clone()),
        evidence_gaps: Vec::new(),
        residual_covariance_m2_s2: input.residual_covariance_m2_s2,
        piv_covariance_m2_s2: Some(piv_covariance),
        residual_minimum_correlation_eigenvalue: residual_minimum,
        reference_minimum_correlation_eigenvalue: Some(reference_minimum),
        piv_minimum_correlation_eigenvalue: Some(piv_minimum),
        joint_minimum_correlation_eigenvalue: Some(joint_minimum),
        reconstructed_residual_covariance_m2_s2: Some(reconstructed),
        maximum_reconstruction_error_m2_s2: Some(maximum_error),
        reconstruction_tolerance_m2_s2: Some(tolerance),
        claim_boundary: "PIV-only local covariance decomposition qualified against the declared reference assessment and full PIV/reference cross-covariance; this is not spatial covariance calibration or physical EHD validation",
    })
}

fn reference_evidence_gaps(assessment: &ReferenceAssessment) -> Vec<EvidenceGap> {
    let mut gaps = Vec::new();
    push_gap(
        &mut gaps,
        nonempty(&assessment.reference_id),
        EvidenceGap::MissingReferenceIdentity,
    );
    push_gap(
        &mut gaps,
        nonempty(&assessment.assessment_id),
        EvidenceGap::MissingAssessmentIdentity,
    );
    push_gap(
        &mut gaps,
        nonempty(&assessment.modality),
        EvidenceGap::MissingModality,
    );
    push_gap(
        &mut gaps,
        nonempty(&assessment.measurand),
        EvidenceGap::MissingMeasurand,
    );
    push_gap(
        &mut gaps,
        nonempty(&assessment.instrument_identity),
        EvidenceGap::MissingInstrumentIdentity,
    );
    push_gap(
        &mut gaps,
        nonempty(&assessment.protocol_fingerprint),
        EvidenceGap::MissingProtocolFingerprint,
    );
    push_gap(
        &mut gaps,
        is_lowercase_sha256(&assessment.source_manifest_sha256),
        EvidenceGap::InvalidSourceManifestDigest,
    );
    push_gap(
        &mut gaps,
        is_lowercase_sha256(&assessment.calibration_receipt_sha256),
        EvidenceGap::InvalidCalibrationReceiptDigest,
    );
    push_gap(
        &mut gaps,
        nonempty(&assessment.shared_cause_assessment_id),
        EvidenceGap::MissingSharedCauseAssessment,
    );
    push_gap(
        &mut gaps,
        assessment.traceability == ReferenceTraceability::Traceable,
        EvidenceGap::ReferenceNotTraceable,
    );
    push_gap(
        &mut gaps,
        assessment.dependence != ReferenceDependence::Unknown,
        EvidenceGap::ReferenceDependenceUnknown,
    );
    push_gap(
        &mut gaps,
        assessment.dependence != ReferenceDependence::Dependent,
        EvidenceGap::ReferenceDependenceUnqualified,
    );
    push_gap(
        &mut gaps,
        assessment.uncertainty_budget_complete,
        EvidenceGap::IncompleteUncertaintyBudget,
    );
    push_gap(
        &mut gaps,
        assessment.validity_interval_qualified,
        EvidenceGap::ValidityIntervalUnqualified,
    );
    push_gap(
        &mut gaps,
        assessment.operating_range_qualified,
        EvidenceGap::OperatingRangeUnqualified,
    );
    push_gap(
        &mut gaps,
        assessment.bandwidth_response_qualified,
        EvidenceGap::BandwidthResponseUnqualified,
    );
    push_gap(
        &mut gaps,
        assessment.time_synchronization_qualified,
        EvidenceGap::TimeSynchronizationUnqualified,
    );
    push_gap(
        &mut gaps,
        assessment.coordinate_registration_qualified,
        EvidenceGap::CoordinateRegistrationUnqualified,
    );
    push_gap(
        &mut gaps,
        assessment.drift_assessed,
        EvidenceGap::DriftNotAssessed,
    );
    push_gap(
        &mut gaps,
        assessment.cross_covariance_quantified,
        EvidenceGap::CrossCovarianceUnquantified,
    );
    push_gap(
        &mut gaps,
        assessment.common_causes_reviewed,
        EvidenceGap::CommonCausesNotReviewed,
    );
    gaps
}

fn push_gap(gaps: &mut Vec<EvidenceGap>, condition: bool, gap: EvidenceGap) {
    if !condition {
        gaps.push(gap);
    }
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty() && !value.chars().any(char::is_control)
}

fn is_lowercase_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_finite_matrix<const N: usize>(
    matrix: &[[f64; N]; N],
    kind: CovarianceKind,
) -> Result<(), ReferenceUqError> {
    for row in 0..N {
        for column in 0..N {
            if !matrix[row][column].is_finite() {
                return Err(ReferenceUqError::NonFiniteCovariance {
                    kind,
                    row,
                    column,
                });
            }
        }
    }
    Ok(())
}

/// Validate a symmetric PSD matrix using its dimensionless correlation form.
/// The matrix itself is never clipped, symmetrized or projected.
fn validate_covariance<const N: usize>(
    matrix: &[[f64; N]; N],
    kind: CovarianceKind,
) -> Result<f64, ReferenceUqError> {
    validate_finite_matrix(matrix, kind)?;
    let mut standard_deviations = [0.0_f64; N];
    for index in 0..N {
        let variance = matrix[index][index];
        if variance < 0.0 {
            return Err(ReferenceUqError::NegativeVariance { kind, index });
        }
        standard_deviations[index] = variance.sqrt();
    }

    let mut correlation = [[0.0_f64; N]; N];
    for index in 0..N {
        correlation[index][index] = if standard_deviations[index] > 0.0 {
            1.0
        } else {
            0.0
        };
    }

    for row in 0..N {
        for column in (row + 1)..N {
            let upper = matrix[row][column];
            let lower = matrix[column][row];
            let left_scale = standard_deviations[row];
            let right_scale = standard_deviations[column];
            if left_scale == 0.0 || right_scale == 0.0 {
                if upper != 0.0 || lower != 0.0 {
                    return Err(ReferenceUqError::ZeroVarianceWithNonzeroCovariance {
                        kind,
                        index: if left_scale == 0.0 { row } else { column },
                        other_index: if left_scale == 0.0 { column } else { row },
                    });
                }
                continue;
            }

            // Normalize with two divisions. Multiplying standard deviations
            // first can overflow even when the final correlation is finite.
            let upper_normalized = (upper / left_scale) / right_scale;
            let lower_normalized = (lower / left_scale) / right_scale;
            if !upper_normalized.is_finite() || !lower_normalized.is_finite() {
                return Err(ReferenceUqError::NonFiniteDerivedCovariance);
            }
            let scale = upper_normalized
                .abs()
                .max(lower_normalized.abs())
                .max(1.0);
            if (upper_normalized - lower_normalized).abs()
                > SYMMETRY_RELATIVE_TOLERANCE * scale
            {
                return Err(ReferenceUqError::AsymmetricCovariance {
                    kind,
                    row,
                    column,
                });
            }
            let normalized = upper_normalized * 0.5 + lower_normalized * 0.5;
            correlation[row][column] = normalized;
            correlation[column][row] = normalized;
        }
    }

    let minimum = minimum_eigenvalue_symmetric(&correlation)?;
    if minimum < -PSD_NEGATIVE_EIGENVALUE_TOLERANCE {
        return Err(ReferenceUqError::NonPositiveSemidefinite {
            kind,
            minimum_correlation_eigenvalue: minimum,
        });
    }
    Ok(minimum)
}

fn minimum_eigenvalue_symmetric<const N: usize>(
    input: &[[f64; N]; N],
) -> Result<f64, ReferenceUqError> {
    let mut matrix = *input;
    let maximum_rotations = 64 * N * N;

    for _ in 0..maximum_rotations {
        let mut p = 0;
        let mut q = if N > 1 { 1 } else { 0 };
        let mut maximum_off_diagonal = 0.0_f64;
        for row in 0..N {
            for column in (row + 1)..N {
                let magnitude = matrix[row][column].abs();
                if magnitude > maximum_off_diagonal {
                    maximum_off_diagonal = magnitude;
                    p = row;
                    q = column;
                }
            }
        }
        if maximum_off_diagonal <= JACOBI_OFF_DIAGONAL_TOLERANCE {
            return Ok((0..N)
                .map(|index| matrix[index][index])
                .fold(f64::INFINITY, f64::min));
        }

        let app = matrix[p][p];
        let aqq = matrix[q][q];
        let apq = matrix[p][q];
        let tau = (aqq - app) / (2.0 * apq);
        let root = tau.hypot(1.0);
        let tangent = if tau >= 0.0 {
            1.0 / (tau + root)
        } else {
            -1.0 / (-tau + root)
        };
        let cosine = 1.0 / (1.0 + tangent * tangent).sqrt();
        let sine = tangent * cosine;

        for index in 0..N {
            if index != p && index != q {
                let aip = matrix[index][p];
                let aiq = matrix[index][q];
                let new_p = cosine * aip - sine * aiq;
                let new_q = sine * aip + cosine * aiq;
                matrix[index][p] = new_p;
                matrix[p][index] = new_p;
                matrix[index][q] = new_q;
                matrix[q][index] = new_q;
            }
        }
        matrix[p][p] = app - tangent * apq;
        matrix[q][q] = aqq + tangent * apq;
        matrix[p][q] = 0.0;
        matrix[q][p] = 0.0;
    }

    let residual = (0..N)
        .flat_map(|row| ((row + 1)..N).map(move |column| input[row][column].abs()))
        .fold(0.0_f64, f64::max);
    if residual > 1.0e-12 {
        return Err(ReferenceUqError::EigenSolverDidNotConverge { dimension: N });
    }
    Ok((0..N)
        .map(|index| matrix[index][index])
        .fold(f64::INFINITY, f64::min))
}

fn build_joint_covariance(
    piv: &Matrix3,
    reference: &Matrix3,
    piv_reference: &Matrix3,
) -> Matrix6 {
    let mut joint = [[0.0_f64; 6]; 6];
    for row in 0..3 {
        for column in 0..3 {
            joint[row][column] = piv[row][column];
            joint[row][column + 3] = piv_reference[row][column];
            joint[row + 3][column] = piv_reference[column][row];
            joint[row + 3][column + 3] = reference[row][column];
        }
    }
    joint
}

fn reconstruct_residual_covariance(
    piv: &Matrix3,
    reference: &Matrix3,
    piv_reference: &Matrix3,
) -> Matrix3 {
    let mut residual = [[0.0_f64; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            residual[row][column] = piv[row][column] + reference[row][column]
                - piv_reference[row][column]
                - piv_reference[column][row];
        }
    }
    residual
}

fn reconstruction_error(expected: &Matrix3, actual: &Matrix3) -> (f64, f64) {
    let mut maximum_error = 0.0_f64;
    let mut scale = f64::MIN_POSITIVE;
    for row in 0..3 {
        for column in 0..3 {
            maximum_error = maximum_error.max((expected[row][column] - actual[row][column]).abs());
            scale = scale
                .max(expected[row][column].abs())
                .max(actual[row][column].abs());
        }
    }
    (maximum_error, RECONSTRUCTION_RELATIVE_TOLERANCE * scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn qualified_evidence() -> ReferenceAssessment {
        ReferenceAssessment {
            reference_id: "independent-ldv-reference-01".to_owned(),
            assessment_id: "ref-assessment-2026-001".to_owned(),
            modality: "laser-doppler-velocimetry".to_owned(),
            measurand: "three-component velocity".to_owned(),
            instrument_identity: "ldv-serial-0042".to_owned(),
            protocol_fingerprint: "ehd-piv-protocol-0.1".to_owned(),
            source_manifest_sha256: "a".repeat(64),
            calibration_receipt_sha256: "b".repeat(64),
            shared_cause_assessment_id: "paired-reference-causes-01".to_owned(),
            traceability: ReferenceTraceability::Traceable,
            dependence: ReferenceDependence::Independent,
            uncertainty_budget_complete: true,
            validity_interval_qualified: true,
            operating_range_qualified: true,
            bandwidth_response_qualified: true,
            time_synchronization_qualified: true,
            coordinate_registration_qualified: true,
            drift_assessed: true,
            cross_covariance_quantified: true,
            common_causes_reviewed: true,
        }
    }

    fn diagonal(values: [f64; 3]) -> Matrix3 {
        [
            [values[0], 0.0, 0.0],
            [0.0, values[1], 0.0],
            [0.0, 0.0, values[2]],
        ]
    }

    fn input_from_components(
        piv: Matrix3,
        reference: Matrix3,
        cross: Matrix3,
    ) -> ReferenceCovarianceInput {
        let residual = reconstruct_residual_covariance(&piv, &reference, &cross);
        ReferenceCovarianceInput {
            residual_covariance_m2_s2: residual,
            reference_covariance_m2_s2: reference,
            piv_reference_cross_covariance_m2_s2: cross,
            reference_assessment: qualified_evidence(),
        }
    }

    fn assert_matrix_close(left: &Matrix3, right: &Matrix3, tolerance: f64) {
        for row in 0..3 {
            for column in 0..3 {
                assert!(
                    (left[row][column] - right[row][column]).abs() <= tolerance,
                    "matrix entry ({row}, {column}) differs: {} vs {}",
                    left[row][column],
                    right[row][column]
                );
            }
        }
    }

    #[test]
    fn independent_reference_decomposes_residual_covariance() {
        let piv = diagonal([0.04, 0.09, 0.16]);
        let reference = diagonal([0.01, 0.02, 0.03]);
        let input = input_from_components(piv, reference, [[0.0; 3]; 3]);

        let report = evaluate_reference_covariance(&input).unwrap();
        assert_eq!(
            report.disposition,
            ReferenceCovarianceDisposition::PivOnlyDecompositionQualified
        );
        assert_matrix_close(&report.piv_covariance_m2_s2.unwrap(), &piv, 1.0e-12);
        assert!(report.evidence_gaps.is_empty());
    }

    #[test]
    fn positive_shared_common_mode_is_removed_from_residual_error() {
        let piv = diagonal([0.04, 0.09, 0.16]);
        let reference = diagonal([0.01, 0.02, 0.03]);
        let cross = diagonal([0.005, 0.01, 0.015]);
        let mut input = input_from_components(piv, reference, cross);
        input.reference_assessment.dependence = ReferenceDependence::PartiallyShared;

        let report = evaluate_reference_covariance(&input).unwrap();
        assert_matrix_close(&report.piv_covariance_m2_s2.unwrap(), &piv, 1.0e-12);
        assert!(
            report.joint_minimum_correlation_eigenvalue.unwrap()
                >= -PSD_NEGATIVE_EIGENVALUE_TOLERANCE
        );
    }

    #[test]
    fn anticorrelated_reference_error_is_not_misread_as_independence() {
        let piv = diagonal([0.04, 0.09, 0.16]);
        let reference = diagonal([0.01, 0.02, 0.03]);
        let cross = diagonal([-0.003, -0.004, -0.005]);
        let mut input = input_from_components(piv, reference, cross);
        input.reference_assessment.dependence = ReferenceDependence::PartiallyShared;

        let report = evaluate_reference_covariance(&input).unwrap();
        assert_matrix_close(&report.piv_covariance_m2_s2.unwrap(), &piv, 1.0e-12);
    }

    #[test]
    fn unidentified_cross_covariance_preserves_residual_only_claim() {
        let piv = diagonal([0.04, 0.09, 0.16]);
        let reference = diagonal([0.01, 0.02, 0.03]);
        let mut input = input_from_components(piv, reference, [[0.0; 3]; 3]);
        input.reference_assessment.cross_covariance_quantified = false;

        let report = evaluate_reference_covariance(&input).unwrap();
        assert_eq!(
            report.disposition,
            ReferenceCovarianceDisposition::ResidualCovarianceOnly
        );
        assert!(report.piv_covariance_m2_s2.is_none());
        assert!(
            report
                .evidence_gaps
                .contains(&EvidenceGap::CrossCovarianceUnquantified)
        );
    }

    #[test]
    fn dependent_or_unknown_reference_is_not_promoted_to_piv_only() {
        let piv = diagonal([0.04, 0.09, 0.16]);
        let reference = diagonal([0.01, 0.02, 0.03]);
        for dependence in [ReferenceDependence::Dependent, ReferenceDependence::Unknown] {
            let mut input = input_from_components(piv, reference, [[0.0; 3]; 3]);
            input.reference_assessment.dependence = dependence;
            let report = evaluate_reference_covariance(&input).unwrap();
            assert_eq!(
                report.disposition,
                ReferenceCovarianceDisposition::ResidualCovarianceOnly
            );
            assert!(report.piv_covariance_m2_s2.is_none());
        }
    }

    #[test]
    fn missing_bandwidth_qualification_preserves_residual_only_claim() {
        let piv = diagonal([0.04, 0.09, 0.16]);
        let reference = diagonal([0.01, 0.02, 0.03]);
        let mut input = input_from_components(piv, reference, [[0.0; 3]; 3]);
        input.reference_assessment.bandwidth_response_qualified = false;

        let report = evaluate_reference_covariance(&input).unwrap();
        assert_eq!(
            report.disposition,
            ReferenceCovarianceDisposition::ResidualCovarianceOnly
        );
        assert!(
            report
                .evidence_gaps
                .contains(&EvidenceGap::BandwidthResponseUnqualified)
        );
    }

    #[test]
    fn joint_covariance_rejects_incoherent_marginal_and_cross_terms() {
        let piv = diagonal([0.004, 0.09, 0.16]);
        let reference = diagonal([0.01, 0.02, 0.03]);
        let cross = diagonal([0.007, 0.0, 0.0]);
        // In component 0, residual variance is exactly zero while the two
        // marginal variances and cross covariance imply a non-PSD joint block.
        let mut input = input_from_components(piv, reference, cross);
        input.residual_covariance_m2_s2[0][0] = 0.0;

        assert!(matches!(
            evaluate_reference_covariance(&input),
            Err(ReferenceUqError::NonPositiveSemidefinite {
                kind: CovarianceKind::JointPivReference,
                ..
            })
        ));
    }

    #[test]
    fn non_psd_reference_covariance_is_rejected_without_repair() {
        let piv = diagonal([0.04, 0.09, 0.16]);
        let mut reference = diagonal([0.01, 0.02, 0.03]);
        reference[0][1] = 0.02;
        reference[1][0] = 0.02;
        let mut input = input_from_components(piv, diagonal([0.01, 0.02, 0.03]), [[0.0; 3]; 3]);
        input.reference_covariance_m2_s2 = reference;

        assert!(matches!(
            evaluate_reference_covariance(&input),
            Err(ReferenceUqError::NonPositiveSemidefinite {
                kind: CovarianceKind::Reference,
                ..
            })
        ));
    }

    #[test]
    fn asymmetric_residual_covariance_is_rejected() {
        let piv = diagonal([0.04, 0.09, 0.16]);
        let reference = diagonal([0.01, 0.02, 0.03]);
        let mut input = input_from_components(piv, reference, [[0.0; 3]; 3]);
        input.residual_covariance_m2_s2[0][1] = 0.005;

        assert!(matches!(
            evaluate_reference_covariance(&input),
            Err(ReferenceUqError::AsymmetricCovariance {
                kind: CovarianceKind::Residual,
                ..
            })
        ));
    }

    #[test]
    fn cross_covariance_orientation_is_preserved() {
        let piv = [
            [0.04, 0.002, 0.0],
            [0.002, 0.09, 0.0],
            [0.0, 0.0, 0.16],
        ];
        let reference = [
            [0.01, 0.001, 0.0],
            [0.001, 0.02, 0.0],
            [0.0, 0.0, 0.03],
        ];
        let cross = [
            [0.004, 0.001, 0.0],
            [0.003, 0.005, 0.0],
            [0.0, 0.0, 0.006],
        ];
        let input = input_from_components(piv, reference, cross);
        let report = evaluate_reference_covariance(&input).unwrap();
        assert_matrix_close(&report.piv_covariance_m2_s2.unwrap(), &piv, 1.0e-12);
    }

    #[test]
    fn component_unit_rescaling_preserves_psd_disposition_and_decomposition() {
        let piv = [
            [0.04, 0.003, 0.001],
            [0.003, 0.09, 0.002],
            [0.001, 0.002, 0.16],
        ];
        let reference = [
            [0.01, 0.001, 0.0],
            [0.001, 0.02, 0.001],
            [0.0, 0.001, 0.03],
        ];
        let cross = [
            [0.002, 0.0005, 0.0],
            [0.0002, 0.004, 0.0003],
            [0.0, 0.0001, 0.006],
        ];
        let original = input_from_components(piv, reference, cross);
        let baseline = evaluate_reference_covariance(&original).unwrap();

        let scales = [1.0e6, 1.0e-4, 1.0e3];
        let mut scaled = original.clone();
        for row in 0..3 {
            for column in 0..3 {
                let factor = scales[row] * scales[column];
                scaled.residual_covariance_m2_s2[row][column] *= factor;
                scaled.reference_covariance_m2_s2[row][column] *= factor;
                scaled.piv_reference_cross_covariance_m2_s2[row][column] *= factor;
            }
        }
        let scaled_report = evaluate_reference_covariance(&scaled).unwrap();
        assert_eq!(baseline.disposition, scaled_report.disposition);

        let baseline_piv = baseline.piv_covariance_m2_s2.unwrap();
        let scaled_piv = scaled_report.piv_covariance_m2_s2.unwrap();
        for row in 0..3 {
            for column in 0..3 {
                let expected = baseline_piv[row][column] * scales[row] * scales[column];
                let relative_error = (scaled_piv[row][column] - expected).abs()
                    / expected.abs().max(f64::MIN_POSITIVE);
                assert!(relative_error < 1.0e-8);
            }
        }
    }

    #[test]
    fn malformed_source_digest_blocks_piv_only_claim() {
        let piv = diagonal([0.04, 0.09, 0.16]);
        let reference = diagonal([0.01, 0.02, 0.03]);
        let mut input = input_from_components(piv, reference, [[0.0; 3]; 3]);
        input.reference_assessment.source_manifest_sha256 = "not-a-digest".to_owned();

        let report = evaluate_reference_covariance(&input).unwrap();
        assert_eq!(
            report.disposition,
            ReferenceCovarianceDisposition::ResidualCovarianceOnly
        );
        assert!(
            report
                .evidence_gaps
                .contains(&EvidenceGap::InvalidSourceManifestDigest)
        );
    }

    #[test]
    fn nonfinite_cross_covariance_is_rejected_when_evidence_is_qualified() {
        let piv = diagonal([0.04, 0.09, 0.16]);
        let reference = diagonal([0.01, 0.02, 0.03]);
        let mut input = input_from_components(piv, reference, [[0.0; 3]; 3]);
        input.piv_reference_cross_covariance_m2_s2[0][1] = f64::NAN;

        assert!(matches!(
            evaluate_reference_covariance(&input),
            Err(ReferenceUqError::NonFiniteCovariance {
                kind: CovarianceKind::JointPivReference,
                ..
            })
        ));
    }

    #[test]
    fn near_roundoff_negative_correlation_eigenvalue_is_tolerated_without_projection() {
        let piv = [
            [1.0, 1.0 + 1.0e-12, 0.0],
            [1.0 + 1.0e-12, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ];
        let minimum = validate_covariance(&piv, CovarianceKind::DerivedPiv).unwrap();
        assert!(minimum < 0.0);
        assert!(minimum >= -PSD_NEGATIVE_EIGENVALUE_TOLERANCE);
        assert_eq!(piv[0][1], 1.0 + 1.0e-12);
    }
}
