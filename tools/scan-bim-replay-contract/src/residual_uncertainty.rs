// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Evidence-qualified uncertainty for a directed point-to-plane residual.
//!
//! This module consumes covariance values and receipt digests from the survey
//! evidence owner. It does not define survey observation, transform, calibration,
//! or adjustment contracts. Coordinates and covariance are converted to metres
//! before projection. Missing shared-error or transform evidence yields an
//! explicit unqualified result rather than an independence assumption.

use crate::metrics::Point3;

const SYMMETRY_TOLERANCE: f64 = 1.0e-12;
const PSD_TOLERANCE: f64 = 1.0e-12;
const NORMAL_TOLERANCE: f64 = 1.0e-9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResidualLinearUnit {
    Metre,
    Millimetre,
    InternationalFoot,
    UsSurveyFoot,
}

impl ResidualLinearUnit {
    fn metres_per_unit(self) -> f64 {
        match self {
            Self::Metre => 1.0,
            Self::Millimetre => 0.001,
            Self::InternationalFoot => 0.3048,
            Self::UsSurveyFoot => 1200.0 / 3937.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CorrelationEvidence {
    /// Full covariance between sample and target coordinates, with rows in
    /// sample coordinate units and columns in target coordinate units.
    CrossCovariance {
        sample_target_covariance: [[f64; 3]; 3],
        receipt_sha256: String,
    },
    /// Explicit content-addressed evidence that the two coordinate errors
    /// may be treated as independent for this exact residual population.
    IndependenceAttested { attestation_sha256: String },
    /// The common error lineage is unknown. No scalar sigma is emitted.
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransformUncertaintyEvidence {
    /// Sample and target coordinates are already in the same survey frame and
    /// no uncertain transformation was applied between them.
    NoTransformRequired,
    /// The upstream survey receipt declares that transform covariance has
    /// already been propagated into the sample covariance supplied here.
    IncludedInSampleCovariance { receipt_sha256: String },
    /// A transform was applied but its covariance is absent.
    Unavailable,
    /// Only a scalar accuracy/RMSE/U95 claim exists; no full transform
    /// covariance or compatible propagation model is available.
    ScalarAccuracyOnly { receipt_sha256: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct NormalResidualCovarianceInput {
    pub sample_frame_id: String,
    pub target_frame_id: String,
    pub sample_unit: ResidualLinearUnit,
    pub target_unit: ResidualLinearUnit,
    /// Unit normal in the common target frame.
    pub target_normal: Point3,
    /// Full covariance in sample_unit squared, from the #1607 observation or
    /// covariance artifact decoder.
    pub sample_covariance: [[f64; 3]; 3],
    /// Full covariance in target_unit squared, from the target/surface evidence.
    pub target_covariance: [[f64; 3]; 3],
    pub sample_covariance_receipt_sha256: String,
    pub target_covariance_receipt_sha256: String,
    pub correlation: CorrelationEvidence,
    pub transform_uncertainty: TransformUncertaintyEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UncertaintyUnqualification {
    UnknownSampleTargetCorrelation,
    TransformCovarianceUnavailable,
    TransformOnlyScalarAccuracy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResidualUncertaintyDisposition {
    QualifiedConditionalCovariance,
    Unqualified,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NormalResidualUncertainty {
    pub disposition: ResidualUncertaintyDisposition,
    pub reasons: Vec<UncertaintyUnqualification>,
    pub common_frame_id: String,
    pub target_normal: Point3,
    pub variance_m2: Option<f64>,
    pub standard_uncertainty_m: Option<f64>,
    pub sample_covariance_receipt_sha256: String,
    pub target_covariance_receipt_sha256: String,
    pub correlation_receipt_sha256: Option<String>,
    pub transform_receipt_sha256: Option<String>,
}

fn digest_is_lowercase_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn finite_point(point: Point3) -> bool {
    point.x.is_finite() && point.y.is_finite() && point.z.is_finite()
}

#[allow(clippy::needless_range_loop)] // Paired indices are intrinsic to fixed-size symmetric-matrix rotations.
fn covariance_is_symmetric_psd<const N: usize>(matrix: &[[f64; N]; N]) -> bool {
    if !matrix.iter().flatten().all(|value| value.is_finite()) {
        return false;
    }

    let scale = matrix
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    if scale == 0.0 {
        return true;
    }

    let mut eigen_work = [[0.0_f64; N]; N];
    for row in 0..N {
        for column in 0..N {
            if (matrix[row][column] - matrix[column][row]).abs() > scale * SYMMETRY_TOLERANCE {
                return false;
            }
            eigen_work[row][column] = matrix[row][column] / scale;
        }
    }

    // Jacobi diagonalization of a small real symmetric matrix. N is either
    // three or six here; a bounded cyclic search is deterministic and avoids
    // forming any inverse or normal equations.
    let maximum_rotations = 64 * N * N;
    for _ in 0..maximum_rotations {
        let mut largest = 0.0_f64;
        let mut pivot = None;
        for row in 0..N {
            for column in (row + 1)..N {
                let magnitude = eigen_work[row][column].abs();
                if magnitude > largest {
                    largest = magnitude;
                    pivot = Some((row, column));
                }
            }
        }

        if largest <= PSD_TOLERANCE {
            break;
        }

        let Some((p, q)) = pivot else {
            break;
        };
        let app = eigen_work[p][p];
        let aqq = eigen_work[q][q];
        let apq = eigen_work[p][q];
        if apq == 0.0 {
            continue;
        }

        let angle = 0.5 * (2.0 * apq).atan2(aqq - app);
        let cosine = angle.cos();
        let sine = angle.sin();

        for index in 0..N {
            if index == p || index == q {
                continue;
            }
            let aip = eigen_work[index][p];
            let aiq = eigen_work[index][q];
            let new_ip = cosine * aip - sine * aiq;
            let new_iq = sine * aip + cosine * aiq;
            eigen_work[index][p] = new_ip;
            eigen_work[p][index] = new_ip;
            eigen_work[index][q] = new_iq;
            eigen_work[q][index] = new_iq;
        }

        eigen_work[p][p] = cosine * cosine * app - 2.0 * sine * cosine * apq + sine * sine * aqq;
        eigen_work[q][q] = sine * sine * app + 2.0 * sine * cosine * apq + cosine * cosine * aqq;
        eigen_work[p][q] = 0.0;
        eigen_work[q][p] = 0.0;
    }

    // If a significant off-diagonal term remains after the bounded rotations,
    // refuse qualification instead of assuming the diagonal is an eigensystem.
    for row in 0..N {
        for column in (row + 1)..N {
            if eigen_work[row][column].abs() > PSD_TOLERANCE * 10.0 {
                return false;
            }
        }
    }
    (0..N).all(|index| eigen_work[index][index] >= -PSD_TOLERANCE)
}

fn scale_covariance(
    covariance: &[[f64; 3]; 3],
    row_scale: f64,
    column_scale: f64,
) -> [[f64; 3]; 3] {
    let mut output = [[0.0; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            output[row][column] = covariance[row][column] * row_scale * column_scale;
        }
    }
    output
}

fn joint_covariance_is_psd(
    sample: &[[f64; 3]; 3],
    target: &[[f64; 3]; 3],
    cross: &[[f64; 3]; 3],
) -> bool {
    let mut joint = [[0.0_f64; 6]; 6];
    for row in 0..3 {
        for column in 0..3 {
            joint[row][column] = sample[row][column];
            joint[row + 3][column + 3] = target[row][column];
            joint[row][column + 3] = cross[row][column];
            joint[column + 3][row] = cross[row][column];
        }
    }
    covariance_is_symmetric_psd(&joint)
}

fn projected_variance(
    normal: [f64; 3],
    sample: &[[f64; 3]; 3],
    target: &[[f64; 3]; 3],
    cross: &[[f64; 3]; 3],
) -> Result<f64, String> {
    let mut variance = 0.0_f64;
    for row in 0..3 {
        for column in 0..3 {
            let difference_covariance =
                sample[row][column] + target[row][column] - cross[row][column] - cross[column][row];
            variance += normal[row] * difference_covariance * normal[column];
        }
    }
    if !variance.is_finite() {
        return Err("projected residual variance is non-finite".to_owned());
    }
    let scale = sample
        .iter()
        .flatten()
        .chain(target.iter().flatten())
        .chain(cross.iter().flatten())
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max)
        * 12.0;
    let tolerance = scale.max(f64::MIN_POSITIVE) * 1.0e-10;
    if variance < -tolerance {
        return Err("projected residual variance is negative".to_owned());
    }
    Ok(variance.max(0.0))
}

/// Propagate sample/target coordinate covariance into a plane-normal residual.
///
/// All matrices are consumed in their declared native squared units and
/// converted to m² before projection. The input coordinates must already be
/// expressed in one identical frame; this function does not execute a CRS or
/// pose transformation. If the transform covariance has not already been
/// included in the sample covariance, or sample/target correlation is unknown,
/// the function returns an unqualified result with no numeric sigma.
///
/// For n a unit target normal, sample covariance Σp, target covariance Σs,
/// and sample/target cross covariance Cps:
///
/// σ²_d = nᵀ(Σp + Σs − Cps − Cpsᵀ)n
pub fn evaluate_normal_residual_uncertainty(
    input: &NormalResidualCovarianceInput,
) -> Result<NormalResidualUncertainty, String> {
    if input.sample_frame_id.trim().is_empty()
        || input.target_frame_id.trim().is_empty()
        || input.sample_frame_id != input.target_frame_id
    {
        return Err("sample and target must share one non-empty coordinate frame".to_owned());
    }
    if !finite_point(input.target_normal) {
        return Err("target normal contains a non-finite coordinate".to_owned());
    }
    let norm = input
        .target_normal
        .x
        .hypot(input.target_normal.y)
        .hypot(input.target_normal.z);
    if !norm.is_finite() || (norm - 1.0).abs() > NORMAL_TOLERANCE {
        return Err("target normal must be unit length within tolerance".to_owned());
    }
    if !digest_is_lowercase_sha256(&input.sample_covariance_receipt_sha256)
        || !digest_is_lowercase_sha256(&input.target_covariance_receipt_sha256)
    {
        return Err(
            "sample and target covariance receipt digests must be lowercase SHA-256".to_owned(),
        );
    }
    if !covariance_is_symmetric_psd(&input.sample_covariance)
        || !covariance_is_symmetric_psd(&input.target_covariance)
    {
        return Err(
            "sample and target covariance must be finite symmetric positive-semidefinite matrices"
                .to_owned(),
        );
    }

    let sample_scale = input.sample_unit.metres_per_unit();
    let target_scale = input.target_unit.metres_per_unit();
    let sample_covariance_m2 =
        scale_covariance(&input.sample_covariance, sample_scale, sample_scale);
    let target_covariance_m2 =
        scale_covariance(&input.target_covariance, target_scale, target_scale);

    let (cross_covariance_m2, correlation_receipt_sha256) = match &input.correlation {
        CorrelationEvidence::CrossCovariance {
            sample_target_covariance,
            receipt_sha256,
        } => {
            if !digest_is_lowercase_sha256(receipt_sha256) {
                return Err("cross-covariance receipt digest must be lowercase SHA-256".to_owned());
            }
            (
                scale_covariance(sample_target_covariance, sample_scale, target_scale),
                Some(receipt_sha256.clone()),
            )
        }
        CorrelationEvidence::IndependenceAttested { attestation_sha256 } => {
            if !digest_is_lowercase_sha256(attestation_sha256) {
                return Err("independence attestation digest must be lowercase SHA-256".to_owned());
            }
            ([[0.0; 3]; 3], Some(attestation_sha256.clone()))
        }
        CorrelationEvidence::Unknown => ([[0.0; 3]; 3], None),
    };

    let transform_receipt_sha256 = match &input.transform_uncertainty {
        TransformUncertaintyEvidence::NoTransformRequired => None,
        TransformUncertaintyEvidence::IncludedInSampleCovariance { receipt_sha256 } => {
            if !digest_is_lowercase_sha256(receipt_sha256) {
                return Err(
                    "transform covariance receipt digest must be lowercase SHA-256".to_owned(),
                );
            }
            Some(receipt_sha256.clone())
        }
        TransformUncertaintyEvidence::Unavailable => None,
        TransformUncertaintyEvidence::ScalarAccuracyOnly { receipt_sha256 } => {
            if !digest_is_lowercase_sha256(receipt_sha256) {
                return Err(
                    "scalar transform accuracy receipt digest must be lowercase SHA-256".to_owned(),
                );
            }
            Some(receipt_sha256.clone())
        }
    };

    let mut reasons = Vec::new();
    if matches!(
        input.transform_uncertainty,
        TransformUncertaintyEvidence::Unavailable
    ) {
        reasons.push(UncertaintyUnqualification::TransformCovarianceUnavailable);
    }
    if matches!(
        input.transform_uncertainty,
        TransformUncertaintyEvidence::ScalarAccuracyOnly { .. }
    ) {
        reasons.push(UncertaintyUnqualification::TransformOnlyScalarAccuracy);
    }
    if matches!(input.correlation, CorrelationEvidence::Unknown) {
        reasons.push(UncertaintyUnqualification::UnknownSampleTargetCorrelation);
    }

    if !matches!(input.correlation, CorrelationEvidence::Unknown)
        && !joint_covariance_is_psd(
            &sample_covariance_m2,
            &target_covariance_m2,
            &cross_covariance_m2,
        )
    {
        return Err("joint sample/target covariance block is not positive-semidefinite".to_owned());
    }

    if !reasons.is_empty() {
        return Ok(NormalResidualUncertainty {
            disposition: ResidualUncertaintyDisposition::Unqualified,
            reasons,
            common_frame_id: input.sample_frame_id.clone(),
            target_normal: input.target_normal,
            variance_m2: None,
            standard_uncertainty_m: None,
            sample_covariance_receipt_sha256: input.sample_covariance_receipt_sha256.clone(),
            target_covariance_receipt_sha256: input.target_covariance_receipt_sha256.clone(),
            correlation_receipt_sha256,
            transform_receipt_sha256,
        });
    }

    let normal = [
        input.target_normal.x,
        input.target_normal.y,
        input.target_normal.z,
    ];
    let variance_m2 = projected_variance(
        normal,
        &sample_covariance_m2,
        &target_covariance_m2,
        &cross_covariance_m2,
    )?;
    let standard_uncertainty_m = variance_m2.sqrt();

    Ok(NormalResidualUncertainty {
        disposition: ResidualUncertaintyDisposition::QualifiedConditionalCovariance,
        reasons,
        common_frame_id: input.sample_frame_id.clone(),
        target_normal: input.target_normal,
        variance_m2: Some(variance_m2),
        standard_uncertainty_m: Some(standard_uncertainty_m),
        sample_covariance_receipt_sha256: input.sample_covariance_receipt_sha256.clone(),
        target_covariance_receipt_sha256: input.target_covariance_receipt_sha256.clone(),
        correlation_receipt_sha256,
        transform_receipt_sha256,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        CorrelationEvidence, NormalResidualCovarianceInput, ResidualLinearUnit,
        ResidualUncertaintyDisposition, TransformUncertaintyEvidence, UncertaintyUnqualification,
        evaluate_normal_residual_uncertainty,
    };
    use crate::metrics::Point3;

    const SAMPLE_DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const TARGET_DIGEST: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const EVIDENCE_DIGEST: &str =
        "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

    fn input() -> NormalResidualCovarianceInput {
        NormalResidualCovarianceInput {
            sample_frame_id: "BUILDING-A-LOCAL".to_owned(),
            target_frame_id: "BUILDING-A-LOCAL".to_owned(),
            sample_unit: ResidualLinearUnit::Metre,
            target_unit: ResidualLinearUnit::Metre,
            target_normal: Point3::new(1.0, 0.0, 0.0),
            sample_covariance: [[0.04, 0.0, 0.0], [0.0, 0.01, 0.0], [0.0, 0.0, 0.01]],
            target_covariance: [[0.01, 0.0, 0.0], [0.0, 0.04, 0.0], [0.0, 0.0, 0.04]],
            sample_covariance_receipt_sha256: SAMPLE_DIGEST.to_owned(),
            target_covariance_receipt_sha256: TARGET_DIGEST.to_owned(),
            correlation: CorrelationEvidence::IndependenceAttested {
                attestation_sha256: EVIDENCE_DIGEST.to_owned(),
            },
            transform_uncertainty: TransformUncertaintyEvidence::NoTransformRequired,
        }
    }

    #[test]
    fn projects_anisotropic_covariance_onto_surface_normal() {
        let result = evaluate_normal_residual_uncertainty(&input()).expect("valid covariance");
        assert_eq!(
            result.disposition,
            ResidualUncertaintyDisposition::QualifiedConditionalCovariance
        );
        assert!((result.variance_m2.expect("variance") - 0.05).abs() < 1.0e-12);
        assert!((result.standard_uncertainty_m.expect("sigma") - 0.05_f64.sqrt()).abs() < 1.0e-12);
    }

    #[test]
    fn preserves_full_covariance_for_oblique_normal() {
        let mut value = input();
        value.target_normal = Point3::new(
            1.0 / 3.0_f64.sqrt(),
            1.0 / 3.0_f64.sqrt(),
            1.0 / 3.0_f64.sqrt(),
        );
        value.sample_covariance = [[0.04, 0.01, 0.0], [0.01, 0.02, 0.005], [0.0, 0.005, 0.03]];
        value.target_covariance = [[0.01, 0.002, 0.0], [0.002, 0.04, 0.006], [0.0, 0.006, 0.02]];
        let result = evaluate_normal_residual_uncertainty(&value).expect("valid full covariance");
        assert_eq!(
            result.disposition,
            ResidualUncertaintyDisposition::QualifiedConditionalCovariance
        );
        let variance = result.variance_m2.expect("variance");
        let expected = (0.05 + 0.06 + 0.05 + 2.0 * (0.012 + 0.0 + 0.011)) / 3.0;
        assert!((variance - expected).abs() < 1.0e-10);
    }

    #[test]
    fn cross_covariance_reduces_variance_without_assuming_independence() {
        let mut value = input();
        value.correlation = CorrelationEvidence::CrossCovariance {
            sample_target_covariance: [[0.005, 0.0, 0.0], [0.0, 0.005, 0.0], [0.0, 0.0, 0.005]],
            receipt_sha256: EVIDENCE_DIGEST.to_owned(),
        };
        let result = evaluate_normal_residual_uncertainty(&value).expect("valid joint covariance");
        assert!((result.variance_m2.expect("variance") - 0.04).abs() < 1.0e-12);
    }

    #[test]
    fn unknown_correlation_returns_unqualified_without_numeric_sigma() {
        let mut value = input();
        value.correlation = CorrelationEvidence::Unknown;
        let result =
            evaluate_normal_residual_uncertainty(&value).expect("unknown is a disposition");
        assert_eq!(
            result.disposition,
            ResidualUncertaintyDisposition::Unqualified
        );
        assert_eq!(result.variance_m2, None);
        assert_eq!(result.standard_uncertainty_m, None);
        assert_eq!(
            result.reasons,
            vec![UncertaintyUnqualification::UnknownSampleTargetCorrelation]
        );
    }

    #[test]
    fn transform_scalar_accuracy_is_not_treated_as_covariance() {
        let mut value = input();
        value.transform_uncertainty = TransformUncertaintyEvidence::ScalarAccuracyOnly {
            receipt_sha256: EVIDENCE_DIGEST.to_owned(),
        };
        let result = evaluate_normal_residual_uncertainty(&value).expect("scalar is preserved");
        assert_eq!(
            result.disposition,
            ResidualUncertaintyDisposition::Unqualified
        );
        assert_eq!(result.standard_uncertainty_m, None);
        assert_eq!(
            result.reasons,
            vec![UncertaintyUnqualification::TransformOnlyScalarAccuracy]
        );
    }

    #[test]
    fn unavailable_transform_covariance_is_not_ignored() {
        let mut value = input();
        value.transform_uncertainty = TransformUncertaintyEvidence::Unavailable;
        let result =
            evaluate_normal_residual_uncertainty(&value).expect("missing is a disposition");
        assert_eq!(
            result.disposition,
            ResidualUncertaintyDisposition::Unqualified
        );
        assert_eq!(
            result.reasons,
            vec![UncertaintyUnqualification::TransformCovarianceUnavailable]
        );
    }

    #[test]
    fn unit_conversion_produces_metres_squared() {
        let mut value = input();
        value.sample_unit = ResidualLinearUnit::Millimetre;
        value.target_unit = ResidualLinearUnit::Millimetre;
        value.sample_covariance = [
            [40_000.0, 0.0, 0.0],
            [0.0, 10_000.0, 0.0],
            [0.0, 0.0, 10_000.0],
        ];
        value.target_covariance = [
            [10_000.0, 0.0, 0.0],
            [0.0, 40_000.0, 0.0],
            [0.0, 0.0, 40_000.0],
        ];
        let result = evaluate_normal_residual_uncertainty(&value).expect("millimetre covariance");
        assert!((result.variance_m2.expect("variance") - 0.05).abs() < 1.0e-12);
    }

    #[test]
    fn scales_cross_covariance_by_both_coordinate_unit_factors() {
        let mut value = input();
        value.sample_unit = ResidualLinearUnit::Millimetre;
        value.target_unit = ResidualLinearUnit::Metre;
        value.sample_covariance = [
            [40_000.0, 0.0, 0.0],
            [0.0, 10_000.0, 0.0],
            [0.0, 0.0, 10_000.0],
        ];
        value.correlation = CorrelationEvidence::CrossCovariance {
            sample_target_covariance: [[5.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
            receipt_sha256: EVIDENCE_DIGEST.to_owned(),
        };
        let result =
            evaluate_normal_residual_uncertainty(&value).expect("mixed-unit cross covariance");
        assert!((result.variance_m2.expect("variance") - 0.04).abs() < 1.0e-12);
    }

    #[test]
    fn rejects_frame_mismatch_and_non_unit_normal() {
        let mut value = input();
        value.target_frame_id = "OTHER-FRAME".to_owned();
        let frame_error = evaluate_normal_residual_uncertainty(&value)
            .expect_err("untransformed frames cannot be mixed");
        assert!(frame_error.contains("share one non-empty coordinate frame"));

        let mut value = input();
        value.target_normal = Point3::new(2.0, 0.0, 0.0);
        let normal_error =
            evaluate_normal_residual_uncertainty(&value).expect_err("normal must be unit length");
        assert!(normal_error.contains("unit length"));
    }

    #[test]
    fn rejects_non_psd_covariances_and_incompatible_joint_covariance() {
        let mut value = input();
        value.sample_covariance = [[1.0, 2.0, 0.0], [2.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let error = evaluate_normal_residual_uncertainty(&value)
            .expect_err("individual covariance must be PSD");
        assert!(error.contains("positive-semidefinite"));

        let mut value = input();
        value.correlation = CorrelationEvidence::CrossCovariance {
            sample_target_covariance: [[1.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
            receipt_sha256: EVIDENCE_DIGEST.to_owned(),
        };
        let error = evaluate_normal_residual_uncertainty(&value)
            .expect_err("cross covariance must form a valid joint covariance block");
        assert!(error.contains("joint sample/target covariance block"));
    }

    #[test]
    fn validates_all_covariance_receipt_digests() {
        let mut value = input();
        value.sample_covariance_receipt_sha256 = "not-a-digest".to_owned();
        let error = evaluate_normal_residual_uncertainty(&value)
            .expect_err("every covariance input must be content-bound");
        assert!(error.contains("lowercase SHA-256"));
    }
}
