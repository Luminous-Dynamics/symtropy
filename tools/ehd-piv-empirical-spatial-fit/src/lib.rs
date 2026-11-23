// SPDX-License-Identifier: AGPL-3.0-or-later

//! Empirical, acquisition-cluster-weighted fitting of spatial PIV error covariance.
//!
//! Residuals are measured minus an independently established reference field.
//! The fitter separates the mean spatial bias profile, a stationary correlated
//! Wendland-C2 component, and a per-vector local nugget. The fit cohort and
//! validation cohort are disjoint complete acquisitions under one frozen
//! processing protocol. This module does not ingest images or prove that a
//! supplied reference is independent; those are evidence/attestation duties.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;

use ehd_piv_conformal_functional_uq::spatial_correlation::{
    SpatialErrorKernel, SpatialKernelCalibrationReceipt,
};

const MIN_INDEPENDENT_ACQUISITIONS: usize = 20;
const MATRIX_EPSILON: f64 = 1.0e-12;

pub type Matrix3 = [[f64; 3]; 3];

const SUPPORTED_MODEL_ID: &str = "wendland_c2_anisotropic_3d";

#[derive(Clone, Debug, PartialEq)]
pub struct ResidualField {
    /// One complete acquisition/dependence cluster, never an individual vector.
    pub acquisition_cluster_id: String,
    /// Unique immutable identity for the independent reference evidence used
    /// to form this residual field (synthetic truth, LPT, a calibrated probe,
    /// or another declared reference).
    pub reference_evidence_id: String,
    pub protocol_fingerprint: String,
    /// Coordinates and residuals must have identical order in every field.
    pub coordinates_m: Vec<[f64; 3]>,
    /// measured PIV velocity minus independent reference velocity, in m/s.
    pub residuals_m_s: Vec<[f64; 3]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FitConfig {
    pub fit_config_id: String,
    pub protocol_fingerprint: String,
    pub reference_contract_id: String,
    /// External evidence identity for the reference instrument/model's own
    /// uncertainty assessment. The fitter does not infer or remove reference
    /// uncertainty from the measured-minus-reference residuals.
    pub reference_uncertainty_assessment_id: String,
    /// Predeclared isotropic range candidates in metres, strictly increasing.
    /// This list must be frozen before inspecting the validation cohort.
    pub length_scale_candidates_m: Vec<f64>,
    /// Maximum held-out absolute correlation error. 0.05 is an initial
    /// prospective target, not a universal PIV law.
    pub maximum_validation_correlation_error: f64,
    /// Separate maximum relative held-out component-variance shift. The
    /// initial 0.10 target is prospective and must be frozen pre-evaluation.
    pub maximum_validation_relative_variance_error: f64,
    /// Cluster-level significance threshold for a negative same-component
    /// spatial-correlation lobe.
    pub negative_lobe_z_threshold: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EmpiricalSpatialFit {
    pub model_id: String,
    pub fit_config_id: String,
    pub protocol_fingerprint: String,
    pub reference_contract_id: String,
    pub reference_uncertainty_assessment_id: String,
    pub coordinates_m: Vec<[f64; 3]>,
    pub length_scale_candidates_m: Vec<f64>,
    pub maximum_validation_correlation_error: f64,
    pub maximum_validation_relative_variance_error: f64,
    pub negative_lobe_z_threshold: f64,
    pub length_scale_m: f64,
    /// Correlated stationary component S, in (m/s)^2.
    pub spatial_component_covariance_m2_s2: Matrix3,
    /// Independent per-vector residual component N = C(0) - S, in (m/s)^2.
    /// Supply this to the existing functional reducer as local covariance;
    /// do not also supply S there.
    pub local_nugget_covariance_m2_s2: Matrix3,
    /// Mean residual profile learned only from the calibration acquisitions.
    /// It is a bias diagnostic, not an automatic correction.
    pub calibration_mean_bias_profile_m_s: Vec<[f64; 3]>,
    pub maximum_calibration_mean_bias_m_s: f64,
    pub calibration_fit_relative_frobenius_error: f64,
    pub calibration_acquisition_count: usize,
    pub calibration_acquisition_ids: Vec<String>,
    pub calibration_reference_evidence_ids: Vec<String>,
    pub calibration_manifest_sha256: String,
}

impl EmpiricalSpatialFit {
    /// Return the independent per-vector nugget N for UQ-005 local covariance.
    /// The correlated spatial component S is exported separately by the
    /// qualified SpatialFitValidation adapter.
    pub fn local_nugget_covariance_for_uq005(
        &self,
    ) -> Result<ehd_piv_conformal_functional_uq::functional::Covariance3, SpatialFitError> {
        validate_fit_record(self)?;
        let n = self.local_nugget_covariance_m2_s2;
        Ok(ehd_piv_conformal_functional_uq::functional::Covariance3 {
            xx: n[0][0],
            xy: n[0][1],
            xz: n[0][2],
            yy: n[1][1],
            yz: n[1][2],
            zz: n[2][2],
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QualificationStatus {
    Qualified,
    RejectedNegativeCorrelationLobe,
    RejectedHeldOutVarianceScale,
    RejectedHeldOutCorrelationFit,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpatialFitValidation {
    pub fit_config_id: String,
    pub fit_protocol_fingerprint: String,
    pub reference_contract_id: String,
    pub reference_uncertainty_assessment_id: String,
    pub calibration_manifest_sha256: String,
    pub fitted_length_scale_m: f64,
    pub fitted_spatial_component_covariance_m2_s2: Matrix3,
    pub status: QualificationStatus,
    pub validation_acquisition_count: usize,
    pub validation_manifest_sha256: String,
    pub receipt_id: String,
    pub maximum_absolute_correlation_fit_error: f64,
    pub maximum_allowed_absolute_correlation_fit_error: f64,
    /// Validation-to-calibration marginal variance ratio, one per component.
    /// None is permitted only when both calibration and validation variance
    /// are numerically zero for that component.
    pub validation_to_calibration_variance_ratio: [Option<f64>; 3],
    pub maximum_validation_relative_variance_error: f64,
    pub maximum_allowed_validation_relative_variance_error: f64,
    pub zero_to_nonzero_variance_component_detected: bool,
    pub negative_correlation_lobe_detected: bool,
    /// A separate diagnostic; it is not silently absorbed into covariance.
    pub maximum_validation_mean_bias_shift_m_s: f64,
    pub lag_bin_width_m: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpatialFitError {
    TooFewIndependentAcquisitions { actual: usize, minimum: usize },
    EmptyIdentity { field: &'static str },
    ProtocolMismatch,
    ReferenceContractMismatch,
    InvalidManifestDigest,
    InvalidFitConfiguration,
    CoordinateCountMismatch,
    ResidualCountMismatch,
    CoordinateGridMismatch { sample_index: usize },
    DuplicateCoordinate { first: usize, second: usize },
    NonFiniteCoordinate { sample_index: usize },
    NonFiniteResidual { sample_index: usize },
    DuplicateAcquisitionCluster,
    DuplicateReferenceEvidence,
    CalibrationValidationOverlap,
    NoIdentifiableSpatialComponent,
    NoPositiveSemidefiniteCandidate,
    NonPositiveSemidefiniteMatrix,
    InvalidNumericalResult,
    NoValidationLagBins,
    NegativeCorrelationLobe,
    HeldOutFitErrorExceeded { actual: f64, maximum: f64 },
    ValidationNotQualified,
}

impl fmt::Display for SpatialFitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for SpatialFitError {}

fn zero_matrix() -> Matrix3 {
    [[0.0; 3]; 3]
}

fn add_scaled(target: &mut Matrix3, source: Matrix3, scale: f64) {
    for i in 0..3 {
        for j in 0..3 {
            target[i][j] += source[i][j] * scale;
        }
    }
}

fn symmetric_outer(a: [f64; 3], b: [f64; 3]) -> Matrix3 {
    let mut out = zero_matrix();
    for i in 0..3 {
        for j in 0..3 {
            out[i][j] = 0.5 * (a[i] * b[j] + a[j] * b[i]);
        }
    }
    out
}

fn matrix_sub(a: Matrix3, b: Matrix3) -> Matrix3 {
    let mut out = zero_matrix();
    for i in 0..3 {
        for j in 0..3 {
            out[i][j] = a[i][j] - b[i][j];
        }
    }
    out
}

fn matrix_norm_squared(a: Matrix3) -> f64 {
    let mut sum = 0.0;
    for row in a {
        for value in row {
            sum += value * value;
        }
    }
    sum
}

fn matrix_trace(a: Matrix3) -> f64 {
    a[0][0] + a[1][1] + a[2][2]
}

fn vector_norm(a: [f64; 3]) -> f64 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn wendland_c2(r: f64) -> f64 {
    if !r.is_finite() || r < 0.0 || r >= 1.0 {
        0.0
    } else {
        (1.0 - r).powi(4) * (4.0 * r + 1.0)
    }
}

fn is_psd(a: Matrix3) -> bool {
    if !a.iter().flatten().all(|v| v.is_finite()) {
        return false;
    }
    let scale = a.iter().flatten().map(|v| v.abs()).fold(0.0_f64, f64::max).max(1.0);
    let t1 = 1.0e-10 * scale;
    let t2 = 1.0e-10 * scale * scale;
    let t3 = 1.0e-10 * scale * scale * scale;
    let symmetry_tolerance = 1.0e-10 * scale;
    for i in 0..3 {
        for j in (i + 1)..3 {
            if (a[i][j] - a[j][i]).abs() > symmetry_tolerance {
                return false;
            }
        }
    }
    let minor01 = a[0][0] * a[1][1] - a[0][1] * a[0][1];
    let minor02 = a[0][0] * a[2][2] - a[0][2] * a[0][2];
    let minor12 = a[1][1] * a[2][2] - a[1][2] * a[1][2];
    let determinant = a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[1][2])
        - a[0][1] * (a[0][1] * a[2][2] - a[0][2] * a[1][2])
        + a[0][2] * (a[0][1] * a[1][2] - a[0][2] * a[1][1]);
    a[0][0] >= -t1
        && a[1][1] >= -t1
        && a[2][2] >= -t1
        && minor01 >= -t2
        && minor02 >= -t2
        && minor12 >= -t2
        && determinant.is_finite()
        && determinant >= -t3
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn validate_fit_record(fit: &EmpiricalSpatialFit) -> Result<(), SpatialFitError> {
    if fit.model_id != SUPPORTED_MODEL_ID
        || fit.fit_config_id.trim().is_empty()
        || fit.protocol_fingerprint.trim().is_empty()
        || fit.reference_contract_id.trim().is_empty()
        || fit.reference_uncertainty_assessment_id.trim().is_empty()
        || fit.calibration_acquisition_count < MIN_INDEPENDENT_ACQUISITIONS
        || fit.calibration_acquisition_ids.len() != fit.calibration_acquisition_count
        || fit.calibration_reference_evidence_ids.len() != fit.calibration_acquisition_count
        || !valid_sha256(&fit.calibration_manifest_sha256)
        || !fit.length_scale_m.is_finite()
        || fit.length_scale_m <= 0.0
        || fit.length_scale_candidates_m.len() < 3
        || !fit.length_scale_candidates_m.iter().all(|v| v.is_finite() && *v > 0.0)
        || fit.length_scale_candidates_m.windows(2).any(|w| w[0] >= w[1])
        || !fit.length_scale_candidates_m.contains(&fit.length_scale_m)
        || !fit.maximum_validation_correlation_error.is_finite()
        || fit.maximum_validation_correlation_error <= 0.0
        || !fit.maximum_validation_relative_variance_error.is_finite()
        || fit.maximum_validation_relative_variance_error <= 0.0
        || !fit.negative_lobe_z_threshold.is_finite()
        || fit.negative_lobe_z_threshold < 1.96
        || !fit.maximum_calibration_mean_bias_m_s.is_finite()
        || fit.maximum_calibration_mean_bias_m_s < 0.0
        || !fit.calibration_fit_relative_frobenius_error.is_finite()
        || fit.calibration_fit_relative_frobenius_error < 0.0
        || fit.coordinates_m.len() < 3
        || fit.calibration_mean_bias_profile_m_s.len() != fit.coordinates_m.len()
        || !fit.calibration_mean_bias_profile_m_s.iter().flatten().all(|v| v.is_finite())
        || !fit.coordinates_m.iter().flatten().all(|v| v.is_finite())
    {
        return Err(SpatialFitError::InvalidFitConfiguration);
    }
    let acquisition_ids: HashSet<&str> = fit.calibration_acquisition_ids.iter().map(String::as_str).collect();
    let reference_ids: HashSet<&str> = fit.calibration_reference_evidence_ids.iter().map(String::as_str).collect();
    if acquisition_ids.len() != fit.calibration_acquisition_count
        || reference_ids.len() != fit.calibration_acquisition_count
        || fit.calibration_acquisition_ids.iter().any(|id| id.trim().is_empty())
        || fit.calibration_reference_evidence_ids.iter().any(|id| id.trim().is_empty())
    {
        return Err(SpatialFitError::InvalidFitConfiguration);
    }
    for i in 0..fit.coordinates_m.len() {
        for j in 0..i {
            if distance(fit.coordinates_m[i], fit.coordinates_m[j]) <= 1.0e-12 {
                return Err(SpatialFitError::InvalidFitConfiguration);
            }
        }
    }
    if !is_psd(fit.spatial_component_covariance_m2_s2)
        || !is_psd(fit.local_nugget_covariance_m2_s2)
    {
        return Err(SpatialFitError::NonPositiveSemidefiniteMatrix);
    }
    Ok(())
}

fn close_coordinate(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1.0e-12 * a.abs().max(b.abs()).max(1.0)
}

fn validate_fields(
    fields: &[ResidualField],
    protocol_fingerprint: &str,
    expected_coordinates: Option<&[[f64; 3]]>,
) -> Result<Vec<[f64; 3]>, SpatialFitError> {
    if fields.len() < MIN_INDEPENDENT_ACQUISITIONS {
        return Err(SpatialFitError::TooFewIndependentAcquisitions {
            actual: fields.len(),
            minimum: MIN_INDEPENDENT_ACQUISITIONS,
        });
    }
    if protocol_fingerprint.trim().is_empty() {
        return Err(SpatialFitError::EmptyIdentity { field: "protocol_fingerprint" });
    }

    let mut acquisition_ids = HashSet::new();
    let mut reference_ids = HashSet::new();
    let first = &fields[0];
    if first.coordinates_m.len() < 3 {
        return Err(SpatialFitError::CoordinateCountMismatch);
    }
    let coordinates = first.coordinates_m.clone();
    if let Some(expected) = expected_coordinates {
        if expected.len() != coordinates.len() {
            return Err(SpatialFitError::CoordinateCountMismatch);
        }
        for (index, (actual, wanted)) in coordinates.iter().zip(expected.iter()).enumerate() {
            if (0..3).any(|axis| !close_coordinate(actual[axis], wanted[axis])) {
                return Err(SpatialFitError::CoordinateGridMismatch { sample_index: index });
            }
        }
    }

    for i in 0..coordinates.len() {
        if !coordinates[i].iter().all(|v| v.is_finite()) {
            return Err(SpatialFitError::NonFiniteCoordinate { sample_index: i });
        }
        for j in 0..i {
            if distance(coordinates[i], coordinates[j]) <= 1.0e-12 {
                return Err(SpatialFitError::DuplicateCoordinate { first: j, second: i });
            }
        }
    }

    for field in fields {
        if field.acquisition_cluster_id.trim().is_empty() {
            return Err(SpatialFitError::EmptyIdentity { field: "acquisition_cluster_id" });
        }
        if field.reference_evidence_id.trim().is_empty() {
            return Err(SpatialFitError::EmptyIdentity { field: "reference_evidence_id" });
        }
        if field.protocol_fingerprint != protocol_fingerprint {
            return Err(SpatialFitError::ProtocolMismatch);
        }
        if !acquisition_ids.insert(field.acquisition_cluster_id.as_str()) {
            return Err(SpatialFitError::DuplicateAcquisitionCluster);
        }
        if !reference_ids.insert(field.reference_evidence_id.as_str()) {
            return Err(SpatialFitError::DuplicateReferenceEvidence);
        }
        if field.coordinates_m.len() != coordinates.len() {
            return Err(SpatialFitError::CoordinateCountMismatch);
        }
        if field.residuals_m_s.len() != coordinates.len() {
            return Err(SpatialFitError::ResidualCountMismatch);
        }
        for (index, point) in field.coordinates_m.iter().enumerate() {
            if !point.iter().all(|v| v.is_finite()) {
                return Err(SpatialFitError::NonFiniteCoordinate { sample_index: index });
            }
            if (0..3).any(|axis| !close_coordinate(point[axis], coordinates[index][axis])) {
                return Err(SpatialFitError::CoordinateGridMismatch { sample_index: index });
            }
        }
        for (index, residual) in field.residuals_m_s.iter().enumerate() {
            if !residual.iter().all(|v| v.is_finite()) {
                return Err(SpatialFitError::NonFiniteResidual { sample_index: index });
            }
        }
    }
    Ok(coordinates)
}

fn mean_residual_profile(fields: &[ResidualField], point_count: usize) -> Vec<[f64; 3]> {
    let mut profile = vec![[0.0; 3]; point_count];
    for field in fields {
        for (index, residual) in field.residuals_m_s.iter().enumerate() {
            for axis in 0..3 {
                profile[index][axis] += residual[axis] / fields.len() as f64;
            }
        }
    }
    profile
}

fn centered_residuals(field: &ResidualField, mean_profile: &[[f64; 3]]) -> Vec<[f64; 3]> {
    field.residuals_m_s.iter().zip(mean_profile.iter()).map(|(r, m)| {
        [r[0] - m[0], r[1] - m[1], r[2] - m[2]]
    }).collect()
}

fn matrix_sum(left: Matrix3, right: Matrix3) -> Matrix3 {
    let mut sum = zero_matrix();
    for row in 0..3 {
        for column in 0..3 {
            sum[row][column] = left[row][column] + right[row][column];
        }
    }
    sum
}

/// Compare held-out marginal variances against the frozen calibration C(0).
/// Components with zero calibration variance are not assigned an infinite
/// ratio; a newly non-zero held-out variance is reported as a separate hard
/// mismatch. The maximum relative error covers components with positive
/// calibration variance.
fn held_out_variance_scale_metrics(
    calibration: Matrix3,
    validation: Matrix3,
) -> ([Option<f64>; 3], f64, bool) {
    let mut ratios = [None; 3];
    let mut maximum_relative_error = 0.0_f64;
    let mut zero_to_nonzero = false;

    for component in 0..3 {
        let calibration_variance = calibration[component][component].max(0.0);
        let validation_variance = validation[component][component].max(0.0);
        if calibration_variance <= MATRIX_EPSILON {
            if validation_variance > MATRIX_EPSILON {
                zero_to_nonzero = true;
            }
        } else {
            let ratio = validation_variance / calibration_variance;
            ratios[component] = Some(ratio);
            maximum_relative_error =
                maximum_relative_error.max((ratio - 1.0).abs());
        }
    }
    (ratios, maximum_relative_error, zero_to_nonzero)
}

fn zero_lag_covariance(fields: &[ResidualField], mean_profile: &[[f64; 3]]) -> Matrix3 {
    let mut covariance = zero_matrix();
    let denominator = (fields.len() * mean_profile.len()) as f64;
    for field in fields {
        for residual in centered_residuals(field, mean_profile) {
            add_scaled(&mut covariance, symmetric_outer(residual, residual), 1.0 / denominator);
        }
    }
    covariance
}

fn median_nearest_neighbour_spacing(coordinates: &[[f64; 3]]) -> Result<f64, SpatialFitError> {
    let mut nearest = Vec::with_capacity(coordinates.len());
    for i in 0..coordinates.len() {
        let mut best = f64::INFINITY;
        for j in 0..coordinates.len() {
            if i != j {
                best = best.min(distance(coordinates[i], coordinates[j]));
            }
        }
        if !best.is_finite() || best <= 0.0 {
            return Err(SpatialFitError::InvalidNumericalResult);
        }
        nearest.push(best);
    }
    nearest.sort_by(|a, b| a.total_cmp(b));
    let middle = nearest.len() / 2;
    let median = if nearest.len() % 2 == 0 {
        0.5 * (nearest[middle - 1] + nearest[middle])
    } else {
        nearest[middle]
    };
    if median.is_finite() && median > 0.0 {
        Ok(median)
    } else {
        Err(SpatialFitError::InvalidNumericalResult)
    }
}

fn validate_config(config: &FitConfig) -> Result<(), SpatialFitError> {
    if config.fit_config_id.trim().is_empty() {
        return Err(SpatialFitError::EmptyIdentity { field: "fit_config_id" });
    }
    if config.protocol_fingerprint.trim().is_empty() {
        return Err(SpatialFitError::EmptyIdentity { field: "protocol_fingerprint" });
    }
    if config.reference_contract_id.trim().is_empty() {
        return Err(SpatialFitError::EmptyIdentity { field: "reference_contract_id" });
    }
    if config.reference_uncertainty_assessment_id.trim().is_empty() {
        return Err(SpatialFitError::EmptyIdentity { field: "reference_uncertainty_assessment_id" });
    }
    let candidates = &config.length_scale_candidates_m;
    if candidates.len() < 3
        || !candidates.iter().all(|v| v.is_finite() && *v > 0.0)
        || candidates.windows(2).any(|w| w[0] >= w[1])
        || !config.maximum_validation_correlation_error.is_finite()
        || config.maximum_validation_correlation_error <= 0.0
        || !config.maximum_validation_relative_variance_error.is_finite()
        || config.maximum_validation_relative_variance_error <= 0.0
        || !config.negative_lobe_z_threshold.is_finite()
        || config.negative_lobe_z_threshold < 1.96
    {
        return Err(SpatialFitError::InvalidFitConfiguration);
    }
    Ok(())
}

struct CandidateFit {
    length_scale_m: f64,
    spatial_covariance: Matrix3,
    nugget_covariance: Matrix3,
    relative_error: f64,
}

fn fit_candidate(
    fields: &[ResidualField],
    mean_profile: &[[f64; 3]],
    c0: Matrix3,
    coordinates: &[[f64; 3]],
    length_scale_m: f64,
) -> Option<CandidateFit> {
    let mut spatial = zero_matrix();
    for field in fields {
        let residuals = centered_residuals(field, mean_profile);
        let mut numerator = zero_matrix();
        let mut denominator = 0.0;
        for i in 0..coordinates.len() {
            for j in (i + 1)..coordinates.len() {
                let k = wendland_c2(distance(coordinates[i], coordinates[j]) / length_scale_m);
                if k > 0.0 {
                    add_scaled(&mut numerator, symmetric_outer(residuals[i], residuals[j]), k);
                    denominator += k * k;
                }
            }
        }
        if denominator <= MATRIX_EPSILON {
            return None;
        }
        add_scaled(&mut spatial, numerator, 1.0 / (denominator * fields.len() as f64));
    }
    let nugget = matrix_sub(c0, spatial);
    if !is_psd(spatial) || !is_psd(nugget) || matrix_trace(spatial) <= MATRIX_EPSILON {
        return None;
    }

    let mut residual_energy = 0.0;
    let mut empirical_energy = 0.0;
    for i in 0..coordinates.len() {
        for j in (i + 1)..coordinates.len() {
            let k = wendland_c2(distance(coordinates[i], coordinates[j]) / length_scale_m);
            let mut empirical = zero_matrix();
            for field in fields {
                let residuals = centered_residuals(field, mean_profile);
                add_scaled(&mut empirical, symmetric_outer(residuals[i], residuals[j]), 1.0 / fields.len() as f64);
            }
            let predicted = {
                let mut p = zero_matrix();
                add_scaled(&mut p, spatial, k);
                p
            };
            residual_energy += matrix_norm_squared(matrix_sub(empirical, predicted));
            empirical_energy += matrix_norm_squared(empirical);
        }
    }
    if empirical_energy <= MATRIX_EPSILON {
        return None;
    }
    let relative_error = (residual_energy / empirical_energy).sqrt();
    if !relative_error.is_finite() {
        return None;
    }
    Some(CandidateFit {
        length_scale_m,
        spatial_covariance: spatial,
        nugget_covariance: nugget,
        relative_error,
    })
}

/// Fit an isotropic Wendland-C2 spatial component on complete independent
/// acquisitions. Candidate ranges must be predeclared in FitConfig. The
/// returned local nugget is C(0)-S and must be used as per-vector local
/// covariance when the resulting spatial kernel is passed to the UQ-005
/// functional propagator.
pub fn fit_spatial_error_kernel(
    calibration_fields: &[ResidualField],
    config: &FitConfig,
    calibration_manifest_sha256: &str,
) -> Result<EmpiricalSpatialFit, SpatialFitError> {
    validate_config(config)?;
    if !valid_sha256(calibration_manifest_sha256) {
        return Err(SpatialFitError::InvalidManifestDigest);
    }
    let coordinates = validate_fields(calibration_fields, &config.protocol_fingerprint, None)?;
    let mean_profile = mean_residual_profile(calibration_fields, coordinates.len());
    let c0 = zero_lag_covariance(calibration_fields, &mean_profile);

    let mut best: Option<CandidateFit> = None;
    for candidate in &config.length_scale_candidates_m {
        if let Some(fitted) = fit_candidate(
            calibration_fields,
            &mean_profile,
            c0,
            &coordinates,
            *candidate,
        ) {
            if best.as_ref().map_or(true, |current| fitted.relative_error < current.relative_error) {
                best = Some(fitted);
            }
        }
    }
    let best = best.ok_or(SpatialFitError::NoPositiveSemidefiniteCandidate)?;

    let maximum_bias = mean_profile.iter().map(|v| vector_norm(*v)).fold(0.0_f64, f64::max);
    Ok(EmpiricalSpatialFit {
        // The isotropic case is represented by equal x/y/z ranges in the
        // existing UQ-005 anisotropic Wendland-C2 contract.
        model_id: SUPPORTED_MODEL_ID.into(),
        fit_config_id: config.fit_config_id.clone(),
        protocol_fingerprint: config.protocol_fingerprint.clone(),
        reference_contract_id: config.reference_contract_id.clone(),
        reference_uncertainty_assessment_id: config.reference_uncertainty_assessment_id.clone(),
        coordinates_m: coordinates.clone(),
        length_scale_candidates_m: config.length_scale_candidates_m.clone(),
        maximum_validation_correlation_error: config.maximum_validation_correlation_error,
        maximum_validation_relative_variance_error: config.maximum_validation_relative_variance_error,
        negative_lobe_z_threshold: config.negative_lobe_z_threshold,
        length_scale_m: best.length_scale_m,
        spatial_component_covariance_m2_s2: best.spatial_covariance,
        local_nugget_covariance_m2_s2: best.nugget_covariance,
        calibration_mean_bias_profile_m_s: mean_profile,
        maximum_calibration_mean_bias_m_s: maximum_bias,
        calibration_fit_relative_frobenius_error: best.relative_error,
        calibration_acquisition_count: calibration_fields.len(),
        calibration_acquisition_ids: calibration_fields.iter().map(|f| f.acquisition_cluster_id.clone()).collect(),
        calibration_reference_evidence_ids: calibration_fields.iter().map(|f| f.reference_evidence_id.clone()).collect(),
        calibration_manifest_sha256: calibration_manifest_sha256.to_ascii_lowercase(),
    })
}

#[derive(Clone, Copy)]
struct Pair {
    i: usize,
    j: usize,
    bin: usize,
    distance_m: f64,
}

#[derive(Clone, Copy)]
struct BinAccumulator {
    count: usize,
    kernel_sum: f64,
    cross_sum: Matrix3,
}

impl BinAccumulator {
    fn empty() -> Self {
        Self { count: 0, kernel_sum: 0.0, cross_sum: zero_matrix() }
    }
}

fn sample_mean_and_se(values: &[f64]) -> Option<(f64, f64)> {
    if values.len() < 2 {
        return None;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>()
        / (values.len() - 1) as f64;
    Some((mean, (variance / values.len() as f64).sqrt()))
}

/// Evaluate the fitted model only on untouched acquisition clusters. The
/// maximum correlation error is computed from equal-weight cluster averages
/// within fixed physical lag bins; individual vectors never become
/// independent validation units.
pub fn validate_spatial_error_kernel(
    fit: &EmpiricalSpatialFit,
    validation_fields: &[ResidualField],
    validation_manifest_sha256: &str,
    receipt_id: &str,
) -> Result<SpatialFitValidation, SpatialFitError> {
    if receipt_id.trim().is_empty() {
        return Err(SpatialFitError::EmptyIdentity { field: "receipt_id" });
    }
    if !valid_sha256(validation_manifest_sha256) {
        return Err(SpatialFitError::InvalidManifestDigest);
    }
    validate_fit_record(fit)?;
    if !fit.maximum_validation_correlation_error.is_finite()
        || fit.maximum_validation_correlation_error <= 0.0
        || !fit.negative_lobe_z_threshold.is_finite()
        || fit.negative_lobe_z_threshold < 1.96
        || fit.length_scale_candidates_m.len() < 3
    {
        return Err(SpatialFitError::InvalidFitConfiguration);
    }

    let coordinates = validate_fields(
        validation_fields,
        &fit.protocol_fingerprint,
        Some(&fit.coordinates_m),
    )?;
    if fit.calibration_mean_bias_profile_m_s.len() != coordinates.len() {
        return Err(SpatialFitError::CoordinateCountMismatch);
    }

    let calibration_ids: HashSet<&str> = fit.calibration_acquisition_ids.iter().map(String::as_str).collect();
    let calibration_refs: HashSet<&str> = fit.calibration_reference_evidence_ids.iter().map(String::as_str).collect();
    if validation_fields.iter().any(|f| calibration_ids.contains(f.acquisition_cluster_id.as_str())
        || calibration_refs.contains(f.reference_evidence_id.as_str()))
    {
        return Err(SpatialFitError::CalibrationValidationOverlap);
    }

    let spacing = median_nearest_neighbour_spacing(&coordinates)?;
    let mut pairs = Vec::new();
    let mut max_bin = 0usize;
    for i in 0..coordinates.len() {
        for j in (i + 1)..coordinates.len() {
            let d = distance(coordinates[i], coordinates[j]);
            let bin = ((d / spacing).round() as usize).max(1);
            max_bin = max_bin.max(bin);
            pairs.push(Pair { i, j, bin, distance_m: d });
        }
    }
    if pairs.is_empty() || max_bin == 0 {
        return Err(SpatialFitError::NoValidationLagBins);
    }

    let mut observed_by_bin: Vec<Vec<Matrix3>> = vec![Vec::new(); max_bin + 1];
    let mut predicted_by_bin: Vec<Vec<Matrix3>> = vec![Vec::new(); max_bin + 1];
    let mut diagonal_correlations: Vec<[Vec<f64>; 3]> = (0..=max_bin)
        .map(|_| std::array::from_fn(|_| Vec::new()))
        .collect();
    let validation_mean_profile = mean_residual_profile(validation_fields, coordinates.len());
    // Keep the calibration zero-lag variance fixed. Normalizing by validation
    // C(0) would hide a held-out amplitude shift while appearing to preserve
    // correlation shape. The held-out C(0) is scored separately below.
    let calibration_c0 = matrix_sum(
        fit.spatial_component_covariance_m2_s2,
        fit.local_nugget_covariance_m2_s2,
    );
    let validation_c0 = zero_lag_covariance(validation_fields, &validation_mean_profile);
    let (variance_ratios, maximum_variance_error, zero_to_nonzero_variance) =
        held_out_variance_scale_metrics(calibration_c0, validation_c0);

    for field in validation_fields {
        // Estimate the held-out mean profile only to separate its covariance
        // from a systematic shift. The shift itself is retained below as a
        // separate validation diagnostic and never applied as a correction.
        let residuals = centered_residuals(field, &validation_mean_profile);
        let mut bins = vec![BinAccumulator::empty(); max_bin + 1];
        for pair in &pairs {
            let bin = &mut bins[pair.bin];
            bin.count += 1;
            bin.kernel_sum += wendland_c2(pair.distance_m / fit.length_scale_m);
            add_scaled(&mut bin.cross_sum, symmetric_outer(residuals[pair.i], residuals[pair.j]), 1.0);
        }

        for bin_index in 1..=max_bin {
            let bin = bins[bin_index];
            if bin.count == 0 {
                continue;
            }
            let mut observed = zero_matrix();
            let mut predicted = zero_matrix();
            add_scaled(&mut observed, bin.cross_sum, 1.0 / bin.count as f64);
            add_scaled(&mut predicted, fit.spatial_component_covariance_m2_s2, bin.kernel_sum / bin.count as f64);

            let mut observed_correlation = zero_matrix();
            let mut predicted_correlation = zero_matrix();
            for a in 0..3 {
                for b in 0..3 {
                    let denominator = (calibration_c0[a][a].max(0.0) * calibration_c0[b][b].max(0.0)).sqrt();
                    if denominator > MATRIX_EPSILON {
                        observed_correlation[a][b] = observed[a][b] / denominator;
                        predicted_correlation[a][b] = predicted[a][b] / denominator;
                    }
                }
            }
            // One observation per independent acquisition and lag bin. Do
            // not count the many vector pairs inside a field as independent.
            for axis in 0..3 {
                diagonal_correlations[bin_index][axis].push(observed_correlation[axis][axis]);
            }
            observed_by_bin[bin_index].push(observed_correlation);
            predicted_by_bin[bin_index].push(predicted_correlation);
        }
    }

    let mut maximum_error = 0.0_f64;
    let mut negative_lobe = false;
    let mut bins_with_evidence = 0usize;
    for bin_index in 1..=max_bin {
        let observations = &observed_by_bin[bin_index];
        let predictions = &predicted_by_bin[bin_index];
        if observations.len() < MIN_INDEPENDENT_ACQUISITIONS || predictions.len() != observations.len() {
            continue;
        }
        bins_with_evidence += 1;
        for a in 0..3 {
            for b in 0..3 {
                let observed_mean = observations.iter().map(|m| m[a][b]).sum::<f64>() / observations.len() as f64;
                let predicted_mean = predictions.iter().map(|m| m[a][b]).sum::<f64>() / predictions.len() as f64;
                maximum_error = maximum_error.max((observed_mean - predicted_mean).abs());
            }
            if let Some((mean, standard_error)) = sample_mean_and_se(&diagonal_correlations[bin_index][a]) {
                if mean < -fit.negative_lobe_z_threshold * standard_error {
                    negative_lobe = true;
                }
            }
        }
    }
    if bins_with_evidence == 0 {
        return Err(SpatialFitError::NoValidationLagBins);
    }

    let maximum_bias_shift = validation_mean_profile.iter().zip(fit.calibration_mean_bias_profile_m_s.iter())
        .map(|(validation, calibration)| vector_norm([
            validation[0] - calibration[0],
            validation[1] - calibration[1],
            validation[2] - calibration[2],
        ]))
        .fold(0.0_f64, f64::max);

    let variance_scale_mismatch = zero_to_nonzero_variance
        || maximum_variance_error > fit.maximum_validation_relative_variance_error;
    let status = if negative_lobe {
        QualificationStatus::RejectedNegativeCorrelationLobe
    } else if variance_scale_mismatch {
        QualificationStatus::RejectedHeldOutVarianceScale
    } else if maximum_error > fit.maximum_validation_correlation_error {
        QualificationStatus::RejectedHeldOutCorrelationFit
    } else {
        QualificationStatus::Qualified
    };

    Ok(SpatialFitValidation {
        fit_config_id: fit.fit_config_id.clone(),
        fit_protocol_fingerprint: fit.protocol_fingerprint.clone(),
        reference_contract_id: fit.reference_contract_id.clone(),
        reference_uncertainty_assessment_id: fit.reference_uncertainty_assessment_id.clone(),
        calibration_manifest_sha256: fit.calibration_manifest_sha256.clone(),
        fitted_length_scale_m: fit.length_scale_m,
        fitted_spatial_component_covariance_m2_s2: fit.spatial_component_covariance_m2_s2,
        status,
        validation_acquisition_count: validation_fields.len(),
        validation_manifest_sha256: validation_manifest_sha256.to_ascii_lowercase(),
        receipt_id: receipt_id.to_owned(),
        maximum_absolute_correlation_fit_error: maximum_error,
        maximum_allowed_absolute_correlation_fit_error: fit.maximum_validation_correlation_error,
        validation_to_calibration_variance_ratio: variance_ratios,
        maximum_validation_relative_variance_error: maximum_variance_error,
        maximum_allowed_validation_relative_variance_error: fit.maximum_validation_relative_variance_error,
        zero_to_nonzero_variance_component_detected: zero_to_nonzero_variance,
        negative_correlation_lobe_detected: negative_lobe,
        maximum_validation_mean_bias_shift_m_s: maximum_bias_shift,
        lag_bin_width_m: spacing,
    })
}

impl SpatialFitValidation {
    /// Convert only a held-out-qualified empirical fit into the existing UQ-005
    /// kernel type. The returned fit's local nugget must be copied into every
    /// FaceVelocitySample local covariance; the correlated component is added
    /// by the spatial-kernel reducer. This keeps C(0)=S+N without double count.
    pub fn to_spatial_error_kernel(
        &self,
        fit: &EmpiricalSpatialFit,
    ) -> Result<SpatialErrorKernel, SpatialFitError> {
        validate_fit_record(fit)?;
        if self.status != QualificationStatus::Qualified
            || self.negative_correlation_lobe_detected
            || self.maximum_absolute_correlation_fit_error > self.maximum_allowed_absolute_correlation_fit_error
            || !self.maximum_absolute_correlation_fit_error.is_finite()
            || !self.maximum_allowed_absolute_correlation_fit_error.is_finite()
            || self.maximum_absolute_correlation_fit_error < 0.0
            || self.maximum_allowed_absolute_correlation_fit_error <= 0.0
            || self.maximum_allowed_absolute_correlation_fit_error != fit.maximum_validation_correlation_error
            || !self.maximum_validation_relative_variance_error.is_finite()
            || self.maximum_validation_relative_variance_error < 0.0
            || self.maximum_validation_relative_variance_error > fit.maximum_validation_relative_variance_error
            || self.validation_to_calibration_variance_ratio.iter().flatten().any(|ratio| {
                !ratio.is_finite() || *ratio < 0.0
            })
            || !self.maximum_allowed_validation_relative_variance_error.is_finite()
            || self.maximum_allowed_validation_relative_variance_error != fit.maximum_validation_relative_variance_error
            || self.zero_to_nonzero_variance_component_detected
            || self.validation_acquisition_count < MIN_INDEPENDENT_ACQUISITIONS
            || !valid_sha256(&self.validation_manifest_sha256)
            || self.receipt_id.trim().is_empty()
            || self.fit_config_id != fit.fit_config_id
            || self.fit_protocol_fingerprint != fit.protocol_fingerprint
            || self.reference_contract_id != fit.reference_contract_id
            || self.reference_uncertainty_assessment_id != fit.reference_uncertainty_assessment_id
            || self.calibration_manifest_sha256 != fit.calibration_manifest_sha256
            || self.fitted_length_scale_m != fit.length_scale_m
            || self.fitted_spatial_component_covariance_m2_s2 != fit.spatial_component_covariance_m2_s2
        {
            return Err(SpatialFitError::ValidationNotQualified);
        }
        let s = fit.spatial_component_covariance_m2_s2;
        Ok(SpatialErrorKernel {
            model_id: fit.model_id.clone(),
            protocol_fingerprint: fit.protocol_fingerprint.clone(),
            calibration_receipt: SpatialKernelCalibrationReceipt {
                receipt_id: self.receipt_id.clone(),
                protocol_fingerprint: fit.protocol_fingerprint.clone(),
                independent_acquisition_count: fit.calibration_acquisition_count,
                maximum_absolute_correlation_fit_error: self.maximum_absolute_correlation_fit_error,
                maximum_allowed_absolute_correlation_fit_error: self.maximum_allowed_absolute_correlation_fit_error,
                negative_correlation_lobe_detected: self.negative_correlation_lobe_detected,
            },
            length_scales_m: [fit.length_scale_m; 3],
            component_covariance_m2_s2: ehd_piv_conformal_functional_uq::functional::Covariance3 {
                xx: s[0][0],
                xy: s[0][1],
                xz: s[0][2],
                yy: s[1][1],
                yz: s[1][2],
                zz: s[2][2],
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cholesky(a: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
        let mut l = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..=i {
                let mut value = a[i][j];
                for k in 0..j {
                    value -= l[i][k] * l[j][k];
                }
                if i == j {
                    l[i][j] = value.max(0.0).sqrt();
                } else {
                    l[i][j] = value / l[j][j];
                }
            }
        }
        l
    }

    fn synthetic_fields(prefix: &str, count: usize, seed: u64, ell: f64) -> Vec<ResidualField> {
        let coordinates = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]];
        let mut covariance = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                covariance[i][j] = 0.8 * wendland_c2((i.abs_diff(j) as f64) / ell)
                    + if i == j { 0.2 } else { 0.0 };
            }
        }
        let l = cholesky(covariance);
        (0..count).map(|index| {
            // Repeated complete 2^3 Hadamard sign blocks give exact zero mean
            // and identity second moments. This is a manufactured deterministic
            // covariance fixture, not a pseudo-random physical experiment.
            let code = (index as u64).wrapping_add(seed);
            let z = [
                if code & 1 == 0 { -1.0 } else { 1.0 },
                if code & 2 == 0 { -1.0 } else { 1.0 },
                if code & 4 == 0 { -1.0 } else { 1.0 },
            ];
            let mut residuals = vec![[0.0; 3]; 3];
            for i in 0..3 {
                let x = (0..=i).map(|j| l[i][j] * z[j]).sum::<f64>();
                residuals[i] = [x, 0.0, 0.0];
            }
            ResidualField {
                acquisition_cluster_id: format!("{prefix}-acq-{index}"),
                reference_evidence_id: format!("{prefix}-ref-{index}"),
                protocol_fingerprint: "piv-grid-32-overlap-75-v1".into(),
                coordinates_m: coordinates.clone(),
                residuals_m_s: residuals,
            }
        }).collect()
    }

    fn config() -> FitConfig {
        FitConfig {
            fit_config_id: "frozen-fit-config-v1".into(),
            protocol_fingerprint: "piv-grid-32-overlap-75-v1".into(),
            reference_contract_id: "independent-reference-contract-v1".into(),
            reference_uncertainty_assessment_id: "reference-uncertainty-assessment-001".into(),
            length_scale_candidates_m: vec![1.0, 1.5, 2.0, 2.5, 3.0, 4.0],
            maximum_validation_correlation_error: 0.05,
            maximum_validation_relative_variance_error: 0.10,
            negative_lobe_z_threshold: 1.96,
        }
    }

    #[test]
    fn empirical_fit_and_held_out_validation_produce_uq005_kernel() {
        let calibration = synthetic_fields("cal", 256, 0x1234, 2.0);
        let validation = synthetic_fields("val", 256, 0x9876, 2.0);
        let fit = fit_spatial_error_kernel(&calibration, &config(), &"a".repeat(64)).unwrap();
        assert!(fit.spatial_component_covariance_m2_s2[0][0] > 0.0);
        assert!(fit.local_nugget_covariance_m2_s2[0][0] >= 0.0);
        let report = validate_spatial_error_kernel(
            &fit, &validation, &"b".repeat(64), "heldout-receipt-001",
        ).unwrap();
        assert_eq!(report.status, QualificationStatus::Qualified);
        let kernel = report.to_spatial_error_kernel(&fit).unwrap();
        assert_eq!(kernel.calibration_receipt.independent_acquisition_count, 256);
        assert_eq!(kernel.model_id, SUPPORTED_MODEL_ID);
    }

    #[test]
    fn local_nugget_adapter_exports_independent_component_not_total_variance() {
        let calibration = synthetic_fields("cal-nugget", 256, 0x1234, 2.0);
        let fit = fit_spatial_error_kernel(&calibration, &config(), &"a".repeat(64)).unwrap();
        let local = fit.local_nugget_covariance_for_uq005().unwrap();
        assert!((local.xx - 0.2).abs() < 1.0e-12);
        assert!((fit.spatial_component_covariance_m2_s2[0][0] - 0.8).abs() < 1.0e-12);
        assert!((local.xx + fit.spatial_component_covariance_m2_s2[0][0] - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn qualified_receipt_is_bound_to_the_exact_calibration_fit() {
        let calibration = synthetic_fields("cal-bound", 256, 0x1234, 2.0);
        let validation = synthetic_fields("val-bound", 256, 0x9876, 2.0);
        let fit = fit_spatial_error_kernel(&calibration, &config(), &"a".repeat(64)).unwrap();
        let report = validate_spatial_error_kernel(
            &fit, &validation, &"b".repeat(64), "bound-receipt",
        ).unwrap();
        let mut substituted = fit.clone();
        substituted.length_scale_m = 2.5;
        assert!(matches!(
            report.to_spatial_error_kernel(&substituted),
            Err(SpatialFitError::ValidationNotQualified)
        ));
    }

    #[test]
    fn non_symmetric_covariance_cannot_be_exported() {
        let calibration = synthetic_fields("cal-sym", 256, 0x1234, 2.0);
        let validation = synthetic_fields("val-sym", 256, 0x9876, 2.0);
        let fit = fit_spatial_error_kernel(&calibration, &config(), &"a".repeat(64)).unwrap();
        let report = validate_spatial_error_kernel(
            &fit, &validation, &"b".repeat(64), "sym-receipt",
        ).unwrap();
        let mut invalid = fit.clone();
        invalid.spatial_component_covariance_m2_s2[0][1] = 0.01;
        assert!(matches!(
            report.to_spatial_error_kernel(&invalid),
            Err(SpatialFitError::NonPositiveSemidefiniteMatrix)
        ));
    }

    #[test]
    fn fewer_than_twenty_independent_acquisitions_are_rejected() {
        let fields = synthetic_fields("few", 19, 7, 2.0);
        assert!(matches!(
            fit_spatial_error_kernel(&fields, &config(), &"a".repeat(64)),
            Err(SpatialFitError::TooFewIndependentAcquisitions { actual: 19, .. })
        ));
    }

    #[test]
    fn calibration_and_validation_acquisition_or_reference_ids_must_be_disjoint() {
        let calibration = synthetic_fields("same", 64, 11, 2.0);
        let fit = fit_spatial_error_kernel(&calibration, &config(), &"a".repeat(64)).unwrap();
        let validation = calibration.clone();
        assert!(matches!(
            validate_spatial_error_kernel(&fit, &validation, &"b".repeat(64), "r"),
            Err(SpatialFitError::CalibrationValidationOverlap)
        ));
    }

    #[test]
    fn duplicate_reference_evidence_is_not_counted_as_independent() {
        let mut fields = synthetic_fields("dup", 32, 17, 2.0);
        fields[1].reference_evidence_id = fields[0].reference_evidence_id.clone();
        assert!(matches!(
            fit_spatial_error_kernel(&fields, &config(), &"a".repeat(64)),
            Err(SpatialFitError::DuplicateReferenceEvidence)
        ));
    }

    #[test]
    fn a_resolved_negative_lobe_blocks_kernel_export() {
        let calibration = synthetic_fields("cal-neg", 256, 0x1111, 2.0);
        let fit = fit_spatial_error_kernel(&calibration, &config(), &"a".repeat(64)).unwrap();
        let mut validation = synthetic_fields("val-neg", 128, 0x2222, 2.0);
        for (index, field) in validation.iter_mut().enumerate() {
            let sign = if index % 2 == 0 { 1.0 } else { -1.0 };
            field.residuals_m_s = vec![[sign, 0.0, 0.0], [-sign, 0.0, 0.0], [sign, 0.0, 0.0]];
        }
        let report = validate_spatial_error_kernel(
            &fit, &validation, &"b".repeat(64), "negative-lobe-receipt",
        ).unwrap();
        assert!(report.negative_correlation_lobe_detected);
        assert_eq!(report.status, QualificationStatus::RejectedNegativeCorrelationLobe);
        assert!(matches!(report.to_spatial_error_kernel(&fit), Err(SpatialFitError::ValidationNotQualified)));
    }

    #[test]
    fn invalid_manifest_digest_and_unfrozen_candidate_grid_are_rejected() {
        let fields = synthetic_fields("bad", 32, 19, 2.0);
        assert!(matches!(
            fit_spatial_error_kernel(&fields, &config(), "not-a-sha256"),
            Err(SpatialFitError::InvalidManifestDigest)
        ));
        let mut bad = config();
        bad.length_scale_candidates_m = vec![1.0, 1.0, 2.0];
        assert!(matches!(
            fit_spatial_error_kernel(&fields, &bad, &"a".repeat(64)),
            Err(SpatialFitError::InvalidFitConfiguration)
        ));
    }

    #[test]
    fn protocol_grid_and_nonfinite_values_fail_closed() {
        let mut fields = synthetic_fields("protocol", 32, 23, 2.0);
        fields[0].protocol_fingerprint = "different-protocol".into();
        assert!(matches!(
            fit_spatial_error_kernel(&fields, &config(), &"a".repeat(64)),
            Err(SpatialFitError::ProtocolMismatch)
        ));

        let mut fields = synthetic_fields("grid", 32, 29, 2.0);
        fields[1].coordinates_m[1][0] = 1.1;
        assert!(matches!(
            fit_spatial_error_kernel(&fields, &config(), &"a".repeat(64)),
            Err(SpatialFitError::CoordinateGridMismatch { .. })
        ));

        let mut fields = synthetic_fields("nan", 32, 31, 2.0);
        fields[0].residuals_m_s[0][0] = f64::NAN;
        assert!(matches!(
            fit_spatial_error_kernel(&fields, &config(), &"a".repeat(64)),
            Err(SpatialFitError::NonFiniteResidual { .. })
        ));
    }

    #[test]
    fn held_out_variance_amplitude_shift_is_rejected_even_when_mean_is_centered() {
        let calibration = synthetic_fields("cal-amplitude", 256, 0x5151, 2.0);
        let fit = fit_spatial_error_kernel(&calibration, &config(), &"a".repeat(64)).unwrap();
        let mut validation = synthetic_fields("val-amplitude", 256, 0x6161, 2.0);
        for field in &mut validation {
            for residual in &mut field.residuals_m_s {
                residual[0] *= 1.5;
            }
        }
        let report = validate_spatial_error_kernel(
            &fit,
            &validation,
            &"b".repeat(64),
            "amplitude-shift-receipt",
        )
        .unwrap();
        assert_eq!(
            report.status,
            QualificationStatus::RejectedHeldOutVarianceScale
        );
        assert!(!report.negative_correlation_lobe_detected);
        assert!(
            (report.validation_to_calibration_variance_ratio[0].unwrap() - 2.25).abs()
                < 1.0e-10
        );
        assert!(
            (report.maximum_validation_relative_variance_error - 1.25).abs() < 1.0e-10
        );
        assert!(matches!(
            report.to_spatial_error_kernel(&fit),
            Err(SpatialFitError::ValidationNotQualified)
        ));
    }

    #[test]
    fn held_out_variance_in_previously_zero_component_is_rejected() {
        let calibration = synthetic_fields("cal-zero-axis", 256, 0x7171, 2.0);
        let fit = fit_spatial_error_kernel(&calibration, &config(), &"a".repeat(64)).unwrap();
        let mut validation = synthetic_fields("val-zero-axis", 256, 0x8181, 2.0);
        for (index, field) in validation.iter_mut().enumerate() {
            let sign = if index % 2 == 0 { 1.0 } else { -1.0 };
            for residual in &mut field.residuals_m_s {
                residual[1] = 0.1 * sign;
            }
        }
        let report = validate_spatial_error_kernel(
            &fit,
            &validation,
            &"b".repeat(64),
            "new-component-variance-receipt",
        )
        .unwrap();
        assert_eq!(
            report.status,
            QualificationStatus::RejectedHeldOutVarianceScale
        );
        assert_eq!(report.validation_to_calibration_variance_ratio[1], None);
        assert!(report.zero_to_nonzero_variance_component_detected);
        assert!(matches!(
            report.to_spatial_error_kernel(&fit),
            Err(SpatialFitError::ValidationNotQualified)
        ));
    }

    #[test]
    fn held_out_mismatch_is_reported_not_silently_recalibrated() {
        let calibration = synthetic_fields("cal-shift", 256, 0x3333, 2.0);
        let fit = fit_spatial_error_kernel(&calibration, &config(), &"a".repeat(64)).unwrap();
        let mut validation = synthetic_fields("val-shift", 256, 0x4444, 2.0);
        for field in &mut validation {
            field.residuals_m_s[0][0] += 2.0;
        }
        let report = validate_spatial_error_kernel(
            &fit, &validation, &"b".repeat(64), "shifted-receipt",
        ).unwrap();
        assert!(report.maximum_validation_mean_bias_shift_m_s > 1.0);
        // Bias is reported separately; correlation covariance does not absorb it.
        assert_ne!(report.status, QualificationStatus::RejectedNegativeCorrelationLobe);
    }
}
