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
    pub(crate) fn metres_per_unit(self) -> f64 {
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

pub(crate) fn digest_is_lowercase_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn finite_point(point: Point3) -> bool {
    point.x.is_finite() && point.y.is_finite() && point.z.is_finite()
}

#[allow(clippy::needless_range_loop)] // Paired indices are intrinsic to fixed-size symmetric-matrix rotations.
pub(crate) fn covariance_is_symmetric_psd<const N: usize>(matrix: &[[f64; N]; N]) -> bool {
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

pub(crate) fn scale_covariance(
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


/// Exact scope for a surface-fit normal covariance artifact.
#[derive(Debug, Clone, PartialEq)]
pub struct NormalCovarianceEvidenceScope {
    pub target_surface_id: String,
    pub surface_patch_id: String,
    pub surface_fit_revision: String,
    pub surface_fit_frame_id: String,
    pub surface_fit_output_sha256: String,
    pub surface_fit_support_population_sha256: String,
    pub estimator_profile_sha256: String,
    pub estimator_configuration_sha256: String,
}

/// Exact scope to which a relative-point/normal correlation claim applies.
#[derive(Debug, Clone, PartialEq)]
pub struct RelativePointNormalScope {
    pub sample_observation_id: String,
    pub target_surface_id: String,
    pub surface_patch_id: String,
    pub surface_fit_revision: String,
    pub surface_fit_output_sha256: String,
    pub surface_fit_support_population_sha256: String,
    pub relative_point_receipt_sha256: String,
    pub relative_point_population_sha256: String,
}

/// Evidence for correlation between relative displacement and normal-estimator error.
#[derive(Debug, Clone, PartialEq)]
pub enum RelativePointNormalCorrelationEvidence {
    CrossCovariance {
        /// Rows are displacement coordinates in metres; columns are tangent
        /// normal perturbations in radians.
        relative_point_normal_covariance_m_rad: [[f64; 3]; 3],
        scope: RelativePointNormalScope,
        receipt_sha256: String,
    },
    IndependenceAttested {
        /// The attestation is valid only for this exact point population and fit.
        scope: RelativePointNormalScope,
        attestation_sha256: String,
    },
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
fn relative_point_normal_scope_matches(
    scope: &RelativePointNormalScope,
    input: &SurfaceNormalUncertaintyInput,
) -> bool {
    scope.sample_observation_id == input.sample_observation_id
        && scope.target_surface_id == input.target_surface_id
        && scope.surface_patch_id == input.surface_patch_id
        && scope.surface_fit_revision == input.surface_fit_revision
        && scope.surface_fit_output_sha256 == input.surface_fit_output_sha256
        && scope.surface_fit_support_population_sha256
            == input.surface_fit_support_population_sha256
        && scope.relative_point_receipt_sha256 == input.relative_point_receipt_sha256
        && scope.relative_point_population_sha256 == input.relative_point_population_sha256
}

pub struct SurfaceNormalUncertaintyInput {
    /// Existing #4228 coordinate/correlation/transform evidence input.
    pub coordinate: NormalResidualCovarianceInput,
    /// Nominal sample-minus-target displacement from the surface-fit origin.
    pub relative_point: Point3,
    pub relative_point_unit: ResidualLinearUnit,
    pub sample_observation_id: String,
    pub target_surface_id: String,
    pub surface_patch_id: String,
    pub surface_fit_revision: String,
    pub surface_fit_frame_id: String,
    pub surface_fit_output_sha256: String,
    pub surface_fit_support_population_sha256: String,
    pub relative_point_receipt_sha256: String,
    pub relative_point_population_sha256: String,
    /// Covariance of the small normal perturbation in radians squared.
    /// It must be tangent to the nominal unit normal.
    pub normal_covariance_rad2: [[f64; 3]; 3],
    pub normal_covariance_receipt_sha256: String,
    pub normal_covariance_scope: NormalCovarianceEvidenceScope,
    pub estimator_profile_sha256: String,
    pub estimator_configuration_sha256: String,
    /// Maximum RMS angular standard uncertainty declared by this estimator
    /// profile. This is an applicability limit, not a universal tolerance.
    pub maximum_angular_standard_uncertainty_rad: f64,
    /// Maximum tangential displacement from the local fit origin supported by
    /// this estimator's first-order normal-uncertainty model.
    pub maximum_tangent_offset_m: f64,
    pub relative_point_normal_correlation: RelativePointNormalCorrelationEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceNormalResidualDisposition {
    QualifiedFirstOrderCovariance,
    Unqualified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceNormalUnqualification {
    CoordinateResidualUnqualified,
    UnknownRelativePointNormalCorrelation,
    AngularUncertaintyOutsideEstimatorDomain,
    TangentOffsetOutsideEstimatorDomain,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceNormalResidualUncertainty {
    pub disposition: SurfaceNormalResidualDisposition,
    pub reasons: Vec<SurfaceNormalUnqualification>,
    pub coordinate_reasons: Vec<UncertaintyUnqualification>,
    pub common_frame_id: String,
    pub target_normal: Point3,
    pub relative_point_m: Point3,
    pub sample_observation_id: String,
    pub target_surface_id: String,
    pub surface_patch_id: String,
    pub surface_fit_revision: String,
    pub surface_fit_support_population_sha256: String,
    pub relative_point_receipt_sha256: String,
    pub relative_point_population_sha256: String,
    pub maximum_angular_standard_uncertainty_rad: f64,
    pub maximum_tangent_offset_m: f64,
    pub tangent_offset_m: f64,
    pub coordinate_variance_m2: Option<f64>,
    pub normal_orientation_variance_m2: Option<f64>,
    pub cross_covariance_contribution_m2: Option<f64>,
    pub variance_m2: Option<f64>,
    pub standard_uncertainty_m: Option<f64>,
    pub surface_fit_output_sha256: String,
    pub normal_covariance_receipt_sha256: String,
    pub estimator_profile_sha256: String,
    pub estimator_configuration_sha256: String,
    pub coordinate_correlation_receipt_sha256: Option<String>,
    pub normal_correlation_receipt_sha256: Option<String>,
}

fn normal_covariance_scope_matches(
    scope: &NormalCovarianceEvidenceScope,
    input: &SurfaceNormalUncertaintyInput,
) -> bool {
    scope.target_surface_id == input.target_surface_id
        && scope.surface_patch_id == input.surface_patch_id
        && scope.surface_fit_revision == input.surface_fit_revision
        && scope.surface_fit_frame_id == input.surface_fit_frame_id
        && scope.surface_fit_output_sha256 == input.surface_fit_output_sha256
        && scope.surface_fit_support_population_sha256
            == input.surface_fit_support_population_sha256
        && scope.estimator_profile_sha256 == input.estimator_profile_sha256
        && scope.estimator_configuration_sha256 == input.estimator_configuration_sha256
}

fn cross_covariance_is_tangent(
    normal: [f64; 3],
    covariance: &[[f64; 3]; 3],
) -> bool {
    if !covariance.iter().flatten().all(|value| value.is_finite()) {
        return false;
    }
    let scale = covariance
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    if scale == 0.0 {
        return true;
    }
    for row in 0..3 {
        let projected = (0..3)
            .map(|column| covariance[row][column] * normal[column])
            .sum::<f64>();
        if projected.abs() > scale * 1.0e-9 {
            return false;
        }
    }
    true
}

fn matrix_quadratic_form(vector: [f64; 3], matrix: &[[f64; 3]; 3]) -> f64 {
    let mut value = 0.0_f64;
    for row in 0..3 {
        for column in 0..3 {
            value += vector[row] * matrix[row][column] * vector[column];
        }
    }
    value
}

fn normal_covariance_is_tangent(
    normal: [f64; 3],
    covariance: &[[f64; 3]; 3],
) -> bool {
    let scale = covariance
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    if scale == 0.0 {
        return true;
    }
    for row in 0..3 {
        let projected = (0..3)
            .map(|column| covariance[row][column] * normal[column])
            .sum::<f64>();
        if projected.abs() > scale * 1.0e-9 {
            return false;
        }
    }
    true
}

fn relative_normal_joint_covariance_is_psd(
    relative_covariance: &[[f64; 3]; 3],
    cross_covariance: &[[f64; 3]; 3],
    normal_covariance: &[[f64; 3]; 3],
) -> bool {
    let mut joint = [[0.0_f64; 6]; 6];
    for row in 0..3 {
        for column in 0..3 {
            joint[row][column] = relative_covariance[row][column];
            joint[row + 3][column + 3] = normal_covariance[row][column];
            joint[row][column + 3] = cross_covariance[row][column];
            joint[column + 3][row] = cross_covariance[row][column];
        }
    }
    covariance_is_symmetric_psd(&joint)
}

fn relative_coordinate_covariance_m2(
    input: &NormalResidualCovarianceInput,
) -> Result<([[f64; 3]; 3], Option<String>), String> {
    let sample_scale = input.sample_unit.metres_per_unit();
    let target_scale = input.target_unit.metres_per_unit();
    let sample = scale_covariance(&input.sample_covariance, sample_scale, sample_scale);
    let target = scale_covariance(&input.target_covariance, target_scale, target_scale);
    let (cross, receipt) = match &input.correlation {
        CorrelationEvidence::CrossCovariance {
            sample_target_covariance,
            receipt_sha256,
        } => {
            if !digest_is_lowercase_sha256(receipt_sha256) {
                return Err("sample/target cross-covariance digest is invalid".to_owned());
            }
            (
                scale_covariance(sample_target_covariance, sample_scale, target_scale),
                Some(receipt_sha256.clone()),
            )
        }
        CorrelationEvidence::IndependenceAttested { attestation_sha256 } => {
            if !digest_is_lowercase_sha256(attestation_sha256) {
                return Err("sample/target independence attestation digest is invalid".to_owned());
            }
            ([[0.0; 3]; 3], Some(attestation_sha256.clone()))
        }
        CorrelationEvidence::Unknown => ([[0.0; 3]; 3], None),
    };
    let mut difference = [[0.0_f64; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            difference[row][column] =
                sample[row][column] + target[row][column] - cross[row][column] - cross[column][row];
        }
    }
    Ok((difference, receipt))
}

/// Propagate coordinate and surface-normal uncertainty into a directed residual
/// using a first-order delta-method approximation.
///
/// For nominal displacement delta = p - s, nominal unit normal n and small
/// tangent-plane normal perturbation eta:
///
///     delta_d ~= n^T delta_delta + delta^T eta
///
/// Hence:
///
///     Var(d) ~= n^T Sigma_delta n + delta^T Sigma_n delta
///              + 2 n^T C_delta_n delta
///
/// This is valid only inside the exact surface-fit estimator's declared local
/// linearization envelope. It does not model surface curvature or higher-order
/// normalized-normal effects. Missing cross-covariance is not independence.
pub fn evaluate_surface_normal_residual_uncertainty(
    input: &SurfaceNormalUncertaintyInput,
) -> Result<SurfaceNormalResidualUncertainty, String> {
    let coordinate = evaluate_normal_residual_uncertainty(&input.coordinate)?;
    let normal = [
        input.coordinate.target_normal.x,
        input.coordinate.target_normal.y,
        input.coordinate.target_normal.z,
    ];
    let required_ids = [
        input.sample_observation_id.as_str(),
        input.target_surface_id.as_str(),
        input.surface_patch_id.as_str(),
        input.surface_fit_revision.as_str(),
        input.surface_fit_frame_id.as_str(),
    ];
    if required_ids.iter().any(|value| value.trim().is_empty()) {
        return Err("sample, target, patch, fit revision and fit frame identities are required".to_owned());
    }
    if input.surface_fit_frame_id != input.coordinate.sample_frame_id
        || input.surface_fit_frame_id != input.coordinate.target_frame_id
    {
        return Err("surface-fit and coordinate covariance frames must match exactly".to_owned());
    }
    if !digest_is_lowercase_sha256(&input.surface_fit_output_sha256)
        || !digest_is_lowercase_sha256(&input.surface_fit_support_population_sha256)
        || !digest_is_lowercase_sha256(&input.relative_point_receipt_sha256)
        || !digest_is_lowercase_sha256(&input.relative_point_population_sha256)
        || !digest_is_lowercase_sha256(&input.normal_covariance_receipt_sha256)
        || !digest_is_lowercase_sha256(&input.estimator_profile_sha256)
        || !digest_is_lowercase_sha256(&input.estimator_configuration_sha256)
    {
        return Err("surface-fit, relative-point, normal-covariance and estimator digests must be lowercase SHA-256".to_owned());
    }
    if !normal_covariance_scope_matches(&input.normal_covariance_scope, input) {
        return Err(
            "normal covariance receipt scope does not match exact surface, fit, frame and estimator configuration"
                .to_owned(),
        );
    }
    if !finite_point(input.relative_point) {
        return Err("relative point contains a non-finite coordinate".to_owned());
    }
    if !covariance_is_symmetric_psd(&input.normal_covariance_rad2) {
        return Err("normal covariance must be finite symmetric positive-semidefinite".to_owned());
    }
    if !normal_covariance_is_tangent(normal, &input.normal_covariance_rad2) {
        return Err("normal covariance must be tangent to the nominal unit normal".to_owned());
    }
    if !input.maximum_angular_standard_uncertainty_rad.is_finite()
        || input.maximum_angular_standard_uncertainty_rad <= 0.0
        || !input.maximum_tangent_offset_m.is_finite()
        || input.maximum_tangent_offset_m <= 0.0
    {
        return Err(
            "estimator angular and tangential-offset applicability limits must be finite and positive"
                .to_owned(),
        );
    }

    let angular_variance = input.normal_covariance_rad2[0][0]
        + input.normal_covariance_rad2[1][1]
        + input.normal_covariance_rad2[2][2];
    let angular_standard_uncertainty_rad = angular_variance.max(0.0).sqrt();
    let linear_scale = input.relative_point_unit.metres_per_unit();
    let relative_point_m = Point3::new(
        input.relative_point.x * linear_scale,
        input.relative_point.y * linear_scale,
        input.relative_point.z * linear_scale,
    );
    if !finite_point(relative_point_m) {
        return Err("relative point conversion to metres overflowed".to_owned());
    }

    let normal_offset = normal
        .iter()
        .zip([relative_point_m.x, relative_point_m.y, relative_point_m.z])
        .map(|(n, value)| n * value)
        .sum::<f64>();
    let tangent_point = [
        relative_point_m.x - normal[0] * normal_offset,
        relative_point_m.y - normal[1] * normal_offset,
        relative_point_m.z - normal[2] * normal_offset,
    ];
    let tangent_offset_m = tangent_point[0]
        .hypot(tangent_point[1])
        .hypot(tangent_point[2]);
    if !tangent_offset_m.is_finite() {
        return Err("tangential displacement is non-finite".to_owned());
    }

    let mut reasons = Vec::new();
    if coordinate.disposition != ResidualUncertaintyDisposition::QualifiedConditionalCovariance {
        reasons.push(SurfaceNormalUnqualification::CoordinateResidualUnqualified);
    }
    if angular_standard_uncertainty_rad > input.maximum_angular_standard_uncertainty_rad {
        reasons.push(SurfaceNormalUnqualification::AngularUncertaintyOutsideEstimatorDomain);
    }
    if tangent_offset_m > input.maximum_tangent_offset_m {
        reasons.push(SurfaceNormalUnqualification::TangentOffsetOutsideEstimatorDomain);
    }

    let (relative_covariance_m2, coordinate_correlation_receipt_sha256) =
        relative_coordinate_covariance_m2(&input.coordinate)?;
    let delta = [relative_point_m.x, relative_point_m.y, relative_point_m.z];
    let normal_orientation_variance_m2 =
        matrix_quadratic_form(delta, &input.normal_covariance_rad2);
    let (normal_cross_covariance, normal_correlation_receipt_sha256) =
        match &input.relative_point_normal_correlation {
            RelativePointNormalCorrelationEvidence::CrossCovariance {
                relative_point_normal_covariance_m_rad,
                scope,
                receipt_sha256,
            } => {
                if !digest_is_lowercase_sha256(receipt_sha256) {
                    return Err("relative-point/normal cross-covariance digest is invalid".to_owned());
                }
                if !relative_point_normal_scope_matches(scope, input) {
                    return Err(
                        "relative-point/normal cross-covariance scope does not match exact observation, target, patch, fit, support and point populations"
                            .to_owned(),
                    );
                }
                (
                    *relative_point_normal_covariance_m_rad,
                    Some(receipt_sha256.clone()),
                )
            }
            RelativePointNormalCorrelationEvidence::IndependenceAttested {
                scope,
                attestation_sha256,
            } => {
                if !digest_is_lowercase_sha256(attestation_sha256) {
                    return Err("relative-point/normal independence attestation digest is invalid".to_owned());
                }
                if !relative_point_normal_scope_matches(scope, input) {
                    return Err(
                        "relative-point/normal independence attestation scope does not match exact observation, target, patch, fit, support and point populations"
                            .to_owned(),
                    );
                }
                ([[0.0; 3]; 3], Some(attestation_sha256.clone()))
            }
            RelativePointNormalCorrelationEvidence::Unknown => {
                reasons.push(SurfaceNormalUnqualification::UnknownRelativePointNormalCorrelation);
                ([[0.0; 3]; 3], None)
            }
        };

    if !matches!(
        &input.relative_point_normal_correlation,
        RelativePointNormalCorrelationEvidence::Unknown
    ) && !cross_covariance_is_tangent(normal, &normal_cross_covariance)
    {
        return Err(
            "relative-point/normal cross-covariance must be finite and tangent to the nominal normal"
                .to_owned(),
        );
    }

    let cross_covariance_contribution_m2 = if matches!(
        &input.relative_point_normal_correlation,
        RelativePointNormalCorrelationEvidence::Unknown
    ) {
        None
    } else {
        let mut contribution = 0.0_f64;
        for row in 0..3 {
            for column in 0..3 {
                contribution += 2.0 * normal[row]
                    * normal_cross_covariance[row][column]
                    * delta[column];
            }
        }
        if !contribution.is_finite() {
            return Err("normal cross-covariance contribution is non-finite".to_owned());
        }
        Some(contribution)
    };

    if coordinate.disposition == ResidualUncertaintyDisposition::QualifiedConditionalCovariance
        && !matches!(
            &input.relative_point_normal_correlation,
            RelativePointNormalCorrelationEvidence::Unknown
        )
        && !relative_normal_joint_covariance_is_psd(
            &relative_covariance_m2,
            &normal_cross_covariance,
            &input.normal_covariance_rad2,
        )
    {
        return Err("joint relative-point/normal covariance block is not positive-semidefinite".to_owned());
    }

    if !normal_orientation_variance_m2.is_finite() || normal_orientation_variance_m2 < -1.0e-12 {
        return Err("normal orientation variance is invalid".to_owned());
    }
    let normal_orientation_variance_m2 = Some(normal_orientation_variance_m2.max(0.0));
    let coordinate_variance_m2 = coordinate.variance_m2;
    let variance_m2 = if reasons.is_empty() {
        let total = coordinate_variance_m2.unwrap_or(0.0)
            + normal_orientation_variance_m2.unwrap_or(0.0)
            + cross_covariance_contribution_m2.unwrap_or(0.0);
        if !total.is_finite() {
            return Err("combined first-order residual variance is non-finite".to_owned());
        }
        let scale = coordinate_variance_m2
            .unwrap_or(0.0)
            .abs()
            .max(normal_orientation_variance_m2.unwrap_or(0.0).abs())
            .max(cross_covariance_contribution_m2.unwrap_or(0.0).abs());
        let tolerance = scale.max(f64::MIN_POSITIVE) * 1.0e-10;
        if total < -tolerance {
            return Err("combined first-order residual variance is negative".to_owned());
        }
        Some(total.max(0.0))
    } else {
        None
    };
    let disposition = if variance_m2.is_some() {
        SurfaceNormalResidualDisposition::QualifiedFirstOrderCovariance
    } else {
        SurfaceNormalResidualDisposition::Unqualified
    };

    Ok(SurfaceNormalResidualUncertainty {
        disposition,
        reasons,
        coordinate_reasons: coordinate.reasons,
        common_frame_id: input.surface_fit_frame_id.clone(),
        target_normal: input.coordinate.target_normal,
        relative_point_m,
        sample_observation_id: input.sample_observation_id.clone(),
        target_surface_id: input.target_surface_id.clone(),
        surface_patch_id: input.surface_patch_id.clone(),
        surface_fit_revision: input.surface_fit_revision.clone(),
        surface_fit_support_population_sha256: input.surface_fit_support_population_sha256.clone(),
        relative_point_receipt_sha256: input.relative_point_receipt_sha256.clone(),
        relative_point_population_sha256: input.relative_point_population_sha256.clone(),
        maximum_angular_standard_uncertainty_rad: input.maximum_angular_standard_uncertainty_rad,
        maximum_tangent_offset_m: input.maximum_tangent_offset_m,
        tangent_offset_m,
        coordinate_variance_m2,
        normal_orientation_variance_m2,
        cross_covariance_contribution_m2,
        variance_m2,
        standard_uncertainty_m: variance_m2.map(f64::sqrt),
        surface_fit_output_sha256: input.surface_fit_output_sha256.clone(),
        normal_covariance_receipt_sha256: input.normal_covariance_receipt_sha256.clone(),
        estimator_profile_sha256: input.estimator_profile_sha256.clone(),
        estimator_configuration_sha256: input.estimator_configuration_sha256.clone(),
        coordinate_correlation_receipt_sha256,
        normal_correlation_receipt_sha256,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        CorrelationEvidence, NormalCovarianceEvidenceScope, NormalResidualCovarianceInput,
        RelativePointNormalCorrelationEvidence, RelativePointNormalScope, ResidualLinearUnit,
        ResidualUncertaintyDisposition,
        SurfaceNormalResidualDisposition, SurfaceNormalUncertaintyInput,
        SurfaceNormalUnqualification, TransformUncertaintyEvidence,
        UncertaintyUnqualification, evaluate_normal_residual_uncertainty,
        evaluate_surface_normal_residual_uncertainty,
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

    fn normal_covariance_scope() -> NormalCovarianceEvidenceScope {
        NormalCovarianceEvidenceScope {
            target_surface_id: "surface-wall-east".to_owned(),
            surface_patch_id: "wall-patch-12".to_owned(),
            surface_fit_revision: "surface-fit-r7".to_owned(),
            surface_fit_frame_id: "BUILDING-A-LOCAL".to_owned(),
            surface_fit_output_sha256: SAMPLE_DIGEST.to_owned(),
            surface_fit_support_population_sha256: "1111111111111111111111111111111111111111111111111111111111111111".to_owned(),
            estimator_profile_sha256: SAMPLE_DIGEST.to_owned(),
            estimator_configuration_sha256: TARGET_DIGEST.to_owned(),
        }
    }

    fn surface_scope() -> RelativePointNormalScope {
        RelativePointNormalScope {
            sample_observation_id: "sample-point-001".to_owned(),
            target_surface_id: "surface-wall-east".to_owned(),
            surface_patch_id: "wall-patch-12".to_owned(),
            surface_fit_revision: "surface-fit-r7".to_owned(),
            surface_fit_output_sha256: SAMPLE_DIGEST.to_owned(),
            surface_fit_support_population_sha256: "1111111111111111111111111111111111111111111111111111111111111111".to_owned(),
            relative_point_receipt_sha256: TARGET_DIGEST.to_owned(),
            relative_point_population_sha256: "2222222222222222222222222222222222222222222222222222222222222222".to_owned(),
        }
    }

    fn surface_normal_input() -> SurfaceNormalUncertaintyInput {
        SurfaceNormalUncertaintyInput {
            coordinate: input(),
            relative_point: Point3::new(0.0, 0.5, 0.0),
            relative_point_unit: ResidualLinearUnit::Metre,
            sample_observation_id: "sample-point-001".to_owned(),
            target_surface_id: "surface-wall-east".to_owned(),
            surface_patch_id: "wall-patch-12".to_owned(),
            surface_fit_revision: "surface-fit-r7".to_owned(),
            surface_fit_frame_id: "BUILDING-A-LOCAL".to_owned(),
            surface_fit_output_sha256: SAMPLE_DIGEST.to_owned(),
            surface_fit_support_population_sha256: "1111111111111111111111111111111111111111111111111111111111111111".to_owned(),
            relative_point_receipt_sha256: TARGET_DIGEST.to_owned(),
            relative_point_population_sha256: "2222222222222222222222222222222222222222222222222222222222222222".to_owned(),
            normal_covariance_rad2: [
                [0.0, 0.0, 0.0],
                [0.0, 4.0e-6, 0.0],
                [0.0, 0.0, 0.0],
            ],
            normal_covariance_receipt_sha256: EVIDENCE_DIGEST.to_owned(),
            normal_covariance_scope: normal_covariance_scope(),
            estimator_profile_sha256: SAMPLE_DIGEST.to_owned(),
            estimator_configuration_sha256: TARGET_DIGEST.to_owned(),
            maximum_angular_standard_uncertainty_rad: 0.05,
            maximum_tangent_offset_m: 10.0,
            relative_point_normal_correlation:
                RelativePointNormalCorrelationEvidence::IndependenceAttested {
                    scope: surface_scope(),
                    attestation_sha256: EVIDENCE_DIGEST.to_owned(),
                },
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
    fn zero_normal_covariance_reproduces_coordinate_only_result() {
        let mut value = surface_normal_input();
        value.normal_covariance_rad2 = [[0.0; 3]; 3];
        let coordinate = evaluate_normal_residual_uncertainty(&value.coordinate)
            .expect("coordinate covariance qualifies");
        let result = evaluate_surface_normal_residual_uncertainty(&value)
            .expect("zero normal covariance qualifies");
        assert_eq!(
            result.disposition,
            SurfaceNormalResidualDisposition::QualifiedFirstOrderCovariance
        );
        assert_eq!(result.coordinate_variance_m2, coordinate.variance_m2);
        assert_eq!(result.normal_orientation_variance_m2, Some(0.0));
        assert_eq!(result.cross_covariance_contribution_m2, Some(0.0));
        assert_eq!(result.variance_m2, coordinate.variance_m2);
    }

    #[test]
    fn tangent_normal_uncertainty_adds_the_hand_computed_orientation_term() {
        let result = evaluate_surface_normal_residual_uncertainty(&surface_normal_input())
            .expect("normal covariance qualifies");
        assert_eq!(
            result.disposition,
            SurfaceNormalResidualDisposition::QualifiedFirstOrderCovariance
        );
        assert!((result.coordinate_variance_m2.unwrap() - 0.05).abs() < 1.0e-12);
        assert!((result.normal_orientation_variance_m2.unwrap() - 1.0e-6).abs() < 1.0e-12);
        assert_eq!(result.cross_covariance_contribution_m2, Some(0.0));
        assert!((result.variance_m2.unwrap() - 0.050001).abs() < 1.0e-12);
    }

    #[test]
    fn tangent_normal_uncertainty_has_no_first_order_effect_on_normal_offset() {
        let mut value = surface_normal_input();
        value.relative_point = Point3::new(0.5, 0.0, 0.0);
        let result = evaluate_surface_normal_residual_uncertainty(&value)
            .expect("normal-direction offset qualifies");
        assert_eq!(result.normal_orientation_variance_m2, Some(0.0));
        assert!((result.variance_m2.unwrap() - result.coordinate_variance_m2.unwrap()).abs() < 1.0e-12);
    }

    #[test]
    fn relative_point_normal_cross_covariance_changes_variance_with_its_sign() {
        let mut positive = surface_normal_input();
        positive.relative_point_normal_correlation =
            RelativePointNormalCorrelationEvidence::CrossCovariance {
                relative_point_normal_covariance_m_rad: [
                    [0.0, 2.0e-4, 0.0],
                    [0.0, 0.0, 0.0],
                    [0.0, 0.0, 0.0],
                ],
                scope: surface_scope(),
                receipt_sha256: EVIDENCE_DIGEST.to_owned(),
            };
        let positive_result = evaluate_surface_normal_residual_uncertainty(&positive)
            .expect("positive cross-covariance block is PSD");
        assert!((positive_result.cross_covariance_contribution_m2.unwrap() - 2.0e-4).abs() < 1.0e-12);
        assert!((positive_result.variance_m2.unwrap() - 0.050201).abs() < 1.0e-12);

        let mut negative = surface_normal_input();
        negative.relative_point_normal_correlation =
            RelativePointNormalCorrelationEvidence::CrossCovariance {
                relative_point_normal_covariance_m_rad: [
                    [0.0, -2.0e-4, 0.0],
                    [0.0, 0.0, 0.0],
                    [0.0, 0.0, 0.0],
                ],
                scope: surface_scope(),
                receipt_sha256: EVIDENCE_DIGEST.to_owned(),
            };
        let negative_result = evaluate_surface_normal_residual_uncertainty(&negative)
            .expect("negative cross-covariance block is PSD");
        assert!((negative_result.cross_covariance_contribution_m2.unwrap() + 2.0e-4).abs() < 1.0e-12);
        assert!((negative_result.variance_m2.unwrap() - 0.049801).abs() < 1.0e-12);
    }

    #[test]
    fn unknown_relative_point_normal_correlation_yields_no_total_sigma() {
        let mut value = surface_normal_input();
        value.relative_point_normal_correlation = RelativePointNormalCorrelationEvidence::Unknown;
        let result = evaluate_surface_normal_residual_uncertainty(&value)
            .expect("unknown correlation is an explicit disposition");
        assert_eq!(result.disposition, SurfaceNormalResidualDisposition::Unqualified);
        assert_eq!(result.variance_m2, None);
        assert_eq!(result.standard_uncertainty_m, None);
        assert!(
            result
                .reasons
                .contains(&SurfaceNormalUnqualification::UnknownRelativePointNormalCorrelation)
        );
    }

    #[test]
    fn normal_covariance_must_live_in_the_nominal_tangent_plane() {
        let mut value = surface_normal_input();
        value.normal_covariance_rad2[0][0] = 1.0e-6;
        let error = evaluate_surface_normal_residual_uncertainty(&value)
            .expect_err("normal-axis variance is not tangent");
        assert!(error.contains("tangent to the nominal unit normal"));
    }

    #[test]
    fn rejects_invalid_joint_relative_point_normal_covariance() {
        let mut value = surface_normal_input();
        value.relative_point_normal_correlation =
            RelativePointNormalCorrelationEvidence::CrossCovariance {
                relative_point_normal_covariance_m_rad: [
                    [0.0, 1.0, 0.0],
                    [0.0, 0.0, 0.0],
                    [0.0, 0.0, 0.0],
                ],
                scope: surface_scope(),
                receipt_sha256: EVIDENCE_DIGEST.to_owned(),
            };
        let error = evaluate_surface_normal_residual_uncertainty(&value)
            .expect_err("invalid joint relative/normal covariance is rejected");
        assert!(error.contains("joint relative-point/normal covariance block"));
    }

    #[test]
    fn angular_uncertainty_outside_estimator_domain_is_unqualified() {
        let mut value = surface_normal_input();
        value.maximum_angular_standard_uncertainty_rad = 0.001;
        let result = evaluate_surface_normal_residual_uncertainty(&value)
            .expect("out-of-domain uncertainty is an explicit disposition");
        assert_eq!(result.disposition, SurfaceNormalResidualDisposition::Unqualified);
        assert_eq!(result.variance_m2, None);
        assert!(
            result
                .reasons
                .contains(&SurfaceNormalUnqualification::AngularUncertaintyOutsideEstimatorDomain)
        );
    }

    #[test]
    fn normal_covariance_receipt_requires_exact_surface_fit_scope() {
        let mut value = surface_normal_input();
        value.normal_covariance_scope.surface_fit_revision = "stale-fit".to_owned();
        let error = evaluate_surface_normal_residual_uncertainty(&value)
            .expect_err("normal covariance receipt cannot be replayed across fits");
        assert!(error.contains("normal covariance receipt scope"));
    }

    #[test]
    fn relative_point_normal_cross_covariance_must_be_tangent() {
        let mut value = surface_normal_input();
        value.relative_point_normal_correlation =
            RelativePointNormalCorrelationEvidence::CrossCovariance {
                relative_point_normal_covariance_m_rad: [
                    [1.0e-4, 0.0, 0.0],
                    [0.0, 0.0, 0.0],
                    [0.0, 0.0, 0.0],
                ],
                scope: surface_scope(),
                receipt_sha256: EVIDENCE_DIGEST.to_owned(),
            };
        let error = evaluate_surface_normal_residual_uncertainty(&value)
            .expect_err("cross-covariance cannot have a normal-axis component");
        assert!(error.contains("cross-covariance must be finite and tangent"));
    }

    #[test]
    fn relative_point_normal_correlation_requires_exact_scope_binding() {
        let mut value = surface_normal_input();
        let mut scope = surface_scope();
        scope.surface_fit_revision = "stale-fit-revision".to_owned();
        value.relative_point_normal_correlation =
            RelativePointNormalCorrelationEvidence::IndependenceAttested {
                scope,
                attestation_sha256: EVIDENCE_DIGEST.to_owned(),
            };
        let error = evaluate_surface_normal_residual_uncertainty(&value)
            .expect_err("independence attestation cannot be replayed across surface fits");
        assert!(error.contains("scope does not match exact observation"));
    }

    #[test]
    fn tangential_offset_outside_estimator_domain_is_unqualified() {
        let mut value = surface_normal_input();
        value.maximum_tangent_offset_m = 0.1;
        let result = evaluate_surface_normal_residual_uncertainty(&value)
            .expect("outside domain is an explicit disposition");
        assert_eq!(result.disposition, SurfaceNormalResidualDisposition::Unqualified);
        assert_eq!(result.variance_m2, None);
        assert!((result.tangent_offset_m - 0.5).abs() < 1.0e-12);
        assert!(
            result
                .reasons
                .contains(&SurfaceNormalUnqualification::TangentOffsetOutsideEstimatorDomain)
        );
    }

    #[test]
    fn unqualified_coordinate_covariance_does_not_build_a_fictitious_joint_block() {
        let mut value = surface_normal_input();
        value.coordinate.correlation = CorrelationEvidence::Unknown;
        value.relative_point_normal_correlation =
            RelativePointNormalCorrelationEvidence::CrossCovariance {
                relative_point_normal_covariance_m_rad: [
                    [0.0, 1.0, 0.0],
                    [0.0, 0.0, 0.0],
                    [0.0, 0.0, 0.0],
                ],
                scope: surface_scope(),
                receipt_sha256: EVIDENCE_DIGEST.to_owned(),
            };
        let result = evaluate_surface_normal_residual_uncertainty(&value)
            .expect("coordinate unknown must preserve unqualified disposition");
        assert_eq!(result.disposition, SurfaceNormalResidualDisposition::Unqualified);
        assert_eq!(result.variance_m2, None);
        assert!(
            result
                .reasons
                .contains(&SurfaceNormalUnqualification::CoordinateResidualUnqualified)
        );
    }

    #[test]
    fn relative_point_unit_conversion_preserves_first_order_variance() {
        let baseline = evaluate_surface_normal_residual_uncertainty(&surface_normal_input())
            .expect("metre displacement qualifies");
        let mut millimetres = surface_normal_input();
        millimetres.relative_point = Point3::new(0.0, 500.0, 0.0);
        millimetres.relative_point_unit = ResidualLinearUnit::Millimetre;
        let converted = evaluate_surface_normal_residual_uncertainty(&millimetres)
            .expect("millimetre displacement qualifies");
        assert!((converted.relative_point_m.y - 0.5).abs() < 1.0e-12);
        assert!((converted.variance_m2.unwrap() - baseline.variance_m2.unwrap()).abs() < 1.0e-12);
    }

    #[test]
    fn analytic_normal_jacobian_matches_small_tangent_finite_difference() {
        let normal = [1.0_f64, 0.0, 0.0];
        let delta = [0.0_f64, 0.5, 0.0];
        let tangent_perturbation = [0.0_f64, 1.0e-5, 0.0];
        let first_order = normal[0] * delta[0]
            + normal[1] * delta[1]
            + normal[2] * delta[2]
            + delta[0] * tangent_perturbation[0]
            + delta[1] * tangent_perturbation[1]
            + delta[2] * tangent_perturbation[2];
        let perturbed_normal = [
            normal[0] + tangent_perturbation[0],
            normal[1] + tangent_perturbation[1],
            normal[2] + tangent_perturbation[2],
        ];
        let norm = perturbed_normal[0]
            .hypot(perturbed_normal[1])
            .hypot(perturbed_normal[2]);
        let exact = (perturbed_normal[0] * delta[0]
            + perturbed_normal[1] * delta[1]
            + perturbed_normal[2] * delta[2])
            / norm;
        assert!((exact - first_order).abs() < 1.0e-10);
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
