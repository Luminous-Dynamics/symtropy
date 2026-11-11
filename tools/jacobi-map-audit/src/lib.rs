//! Condition-aware numerical audits for screen-projected Jacobi maps.
//!
//! This crate does not integrate geodesics or prove a caustic exists. It audits
//! 2x2 maps emitted by independent variational and finite-difference solvers,
//! propagates a declared entrywise uncertainty bound, tests screen-basis
//! covariance, and identifies determinant-sign brackets along a tracked branch.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const DEFAULT_MAX_CONDITION_NUMBER: f64 = 1.0e10;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Matrix2 {
    pub entries: [[f64; 2]; 2],
}

impl Matrix2 {
    pub const IDENTITY: Self = Self { entries: [[1.0, 0.0], [0.0, 1.0]] };

    pub const fn new(a11: f64, a12: f64, a21: f64, a22: f64) -> Self {
        Self { entries: [[a11, a12], [a21, a22]] }
    }

    pub fn rotation(angle_radians: f64) -> Result<Self, AuditError> {
        if !angle_radians.is_finite() {
            return Err(AuditError::NonFinite("screen rotation angle"));
        }
        let (s, c) = angle_radians.sin_cos();
        Ok(Self::new(c, -s, s, c))
    }

    pub fn transpose(self) -> Self {
        Self::new(
            self.entries[0][0],
            self.entries[1][0],
            self.entries[0][1],
            self.entries[1][1],
        )
    }

    pub fn multiply(self, rhs: Self) -> Self {
        let a = self.entries;
        let b = rhs.entries;
        Self::new(
            a[0][0] * b[0][0] + a[0][1] * b[1][0],
            a[0][0] * b[0][1] + a[0][1] * b[1][1],
            a[1][0] * b[0][0] + a[1][1] * b[1][0],
            a[1][0] * b[0][1] + a[1][1] * b[1][1],
        )
    }

    /// Change observer and source screen bases independently.
    ///
    /// If R_o and R_s are proper rotations of the observer and source
    /// orthonormal screens, map components transform as R_s^T D R_o.
    pub fn rotate_screens(
        self,
        observer_angle_radians: f64,
        source_angle_radians: f64,
    ) -> Result<Self, AuditError> {
        let observer = Self::rotation(observer_angle_radians)?;
        let source = Self::rotation(source_angle_radians)?;
        Ok(source.transpose().multiply(self).multiply(observer))
    }

    pub fn is_finite(self) -> bool {
        self.entries.iter().flatten().all(|value| value.is_finite())
    }

    pub fn frobenius_norm(self) -> f64 {
        self.entries
            .iter()
            .flatten()
            .fold(0.0_f64, |norm, value| norm.hypot(*value))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JacobiMapEvidence {
    pub sample_id: String,
    /// Canonical image/ray branch identity shared across implementations and samples.
    pub branch_id: String,
    /// Monotone parameter along one continuously tracked ray/image branch.
    pub branch_parameter: f64,
    pub normalization_id: String,
    pub map: Matrix2,
    /// Absolute error bound for each of the four map entries.
    /// The induced spectral-norm uncertainty is bounded by 2 * this value.
    pub componentwise_absolute_error_bound: f64,
}

impl JacobiMapEvidence {
    pub fn validate(&self) -> Result<(), AuditError> {
        if self.sample_id.trim().is_empty() {
            return Err(AuditError::EmptyField("sample_id"));
        }
        if self.branch_id.trim().is_empty() {
            return Err(AuditError::EmptyField("branch_id"));
        }
        if self.normalization_id.trim().is_empty() {
            return Err(AuditError::EmptyField("normalization_id"));
        }
        if !self.branch_parameter.is_finite() {
            return Err(AuditError::NonFinite("branch_parameter"));
        }
        if !self.map.is_finite() {
            return Err(AuditError::NonFinite("Jacobi map entry"));
        }
        let error = self.componentwise_absolute_error_bound;
        if !error.is_finite() || error < 0.0 {
            return Err(AuditError::InvalidUncertainty(error));
        }
        if !(2.0 * error).is_finite() {
            return Err(AuditError::NumericRange("operator uncertainty bound"));
        }
        Ok(())
    }

    pub fn analyze(&self, max_condition_number: f64) -> Result<JacobiMapAnalysis, AuditError> {
        self.validate()?;
        if !max_condition_number.is_finite() || max_condition_number < 1.0 {
            return Err(AuditError::InvalidConditionLimit(max_condition_number));
        }

        let [[a, b], [c, d]] = self.map.entries;
        let scale = a.abs().max(b.abs()).max(c.abs()).max(d.abs());
        let operator_error_bound = 2.0 * self.componentwise_absolute_error_bound;

        if scale == 0.0 {
            let status = if operator_error_bound == 0.0 {
                MapStatus::Singular
            } else {
                MapStatus::Unresolved
            };
            return Ok(JacobiMapAnalysis {
                sample_id: self.sample_id.clone(),
                branch_id: self.branch_id.clone(),
                branch_parameter: self.branch_parameter,
                normalization_id: self.normalization_id.clone(),
                determinant_sign: 0,
                log_absolute_determinant: None,
                singular_value_max: 0.0,
                singular_value_min: 0.0,
                log_singular_value_min: None,
                condition_number: None,
                log_condition_number: None,
                operator_error_bound,
                status,
                log_magnification: None,
            });
        }

        // Scale first to avoid overflow in A^T A and preserve a useful
        // determinant for matrices whose physical units are very large/small.
        let an = a / scale;
        let bn = b / scale;
        let cn = c / scale;
        let dn = d / scale;

        // For a real 2x2 matrix, the sum/difference of these two hypot values
        // gives sigma_max without squaring large entries. Recover sigma_min
        // from |det|/sigma_max to avoid subtractive cancellation near caustics.
        let p = (an + dn).hypot(cn - bn);
        let q = (an - dn).hypot(cn + bn);
        let sigma_max_normalized = 0.5 * (p + q);
        if !sigma_max_normalized.is_finite() || sigma_max_normalized <= 0.0 {
            return Err(AuditError::NumericRange("largest singular value"));
        }

        // Fused multiply-add improves determinant sign/resolution near a
        // caustic compared with two separately rounded products.
        let determinant_normalized = an.mul_add(dn, -(bn * cn));
        let determinant_sign = if determinant_normalized > 0.0 {
            1
        } else if determinant_normalized < 0.0 {
            -1
        } else {
            0
        };

        let log_scale = scale.ln();
        let log_sigma_max = log_scale + sigma_max_normalized.ln();
        let (log_absolute_determinant, log_sigma_min, sigma_min_normalized) =
            if determinant_sign == 0 {
                (None, None, 0.0)
            } else {
                let log_det = 2.0 * log_scale + determinant_normalized.abs().ln();
                let log_min = log_scale + determinant_normalized.abs().ln()
                    - sigma_max_normalized.ln();
                let min_normalized = determinant_normalized.abs() / sigma_max_normalized;
                (Some(log_det), Some(log_min), min_normalized)
            };

        let singular_value_max = scale * sigma_max_normalized;
        let singular_value_min = scale * sigma_min_normalized;
        if !singular_value_max.is_finite() {
            return Err(AuditError::NumericRange("largest singular value"));
        }

        let log_condition_number = log_sigma_min.map(|log_min| log_sigma_max - log_min);
        let condition_number = log_condition_number.and_then(|log_cond| {
            if log_cond <= f64::MAX.ln() {
                Some(log_cond.exp())
            } else {
                None
            }
        });

        let status = if determinant_sign == 0 {
            if operator_error_bound == 0.0 {
                MapStatus::Singular
            } else {
                MapStatus::Unresolved
            }
        } else if operator_error_bound > 0.0
            && log_sigma_min.is_some_and(|log_min| log_min <= operator_error_bound.ln())
        {
            MapStatus::Unresolved
        } else if log_condition_number.is_some_and(|log_cond| log_cond > max_condition_number.ln()) {
            MapStatus::IllConditioned
        } else {
            MapStatus::Resolved
        };

        let log_magnification = if status == MapStatus::Resolved {
            log_absolute_determinant.map(|log_det| -log_det)
        } else {
            None
        };

        Ok(JacobiMapAnalysis {
            sample_id: self.sample_id.clone(),
            branch_id: self.branch_id.clone(),
            branch_parameter: self.branch_parameter,
            normalization_id: self.normalization_id.clone(),
            determinant_sign,
            log_absolute_determinant,
            singular_value_max,
            singular_value_min,
            log_singular_value_min: log_sigma_min,
            condition_number,
            log_condition_number,
            operator_error_bound,
            status,
            log_magnification,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MapStatus {
    Resolved,
    IllConditioned,
    Singular,
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JacobiMapAnalysis {
    pub sample_id: String,
    pub branch_id: String,
    pub branch_parameter: f64,
    pub normalization_id: String,
    pub determinant_sign: i8,
    /// Natural logarithm of |det(D)|; absent for a numerically singular map.
    pub log_absolute_determinant: Option<f64>,
    pub singular_value_max: f64,
    /// May underflow to zero; use log_singular_value_min for classification.
    pub singular_value_min: f64,
    pub log_singular_value_min: Option<f64>,
    pub condition_number: Option<f64>,
    pub log_condition_number: Option<f64>,
    /// Frobenius-derived upper bound on spectral-norm map uncertainty.
    pub operator_error_bound: f64,
    pub status: MapStatus,
    /// Present only for a resolved map with a pinned area normalization.
    /// This is a logarithm to avoid overflow near strong focusing.
    pub log_magnification: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CausticBracketStatus {
    NoSignChange,
    Bracketed,
    EndpointSingularity,
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CausticBracket {
    pub status: CausticBracketStatus,
    pub branch_id: String,
    pub normalization_id: String,
    pub lower_parameter: f64,
    pub upper_parameter: f64,
    /// Linear-in-determinant estimate only; the full bracket remains authoritative.
    pub interpolated_parameter_estimate: Option<f64>,
    pub left_determinant_sign: i8,
    pub right_determinant_sign: i8,
    pub reason: String,
}

/// Audit two consecutive samples on the same, continuously tracked image branch.
///
/// A sign-changing determinant brackets a parity change/caustic only when both
/// endpoint determinant signs are resolved against their declared map errors.
/// The interpolation estimate is diagnostic; the endpoint interval is the
/// reported bracket. It is not an exact-geodesic root error bound.
pub fn bracket_caustic(
    left: &JacobiMapEvidence,
    right: &JacobiMapEvidence,
    max_condition_number: f64,
) -> Result<CausticBracket, AuditError> {
    left.validate()?;
    right.validate()?;
    if left.normalization_id != right.normalization_id {
        return Err(AuditError::NormalizationMismatch);
    }
    if left.branch_id != right.branch_id {
        return Err(AuditError::BranchMismatch);
    }
    if right.branch_parameter <= left.branch_parameter {
        return Err(AuditError::NonIncreasingBranchParameter);
    }

    let left_analysis = left.analyze(max_condition_number)?;
    let right_analysis = right.analyze(max_condition_number)?;
    let (left_sign, right_sign) = (
        left_analysis.determinant_sign,
        right_analysis.determinant_sign,
    );

    let base = |status, estimate, reason: &str| CausticBracket {
        status,
        branch_id: left.branch_id.clone(),
        normalization_id: left.normalization_id.clone(),
        lower_parameter: left.branch_parameter,
        upper_parameter: right.branch_parameter,
        interpolated_parameter_estimate: estimate,
        left_determinant_sign: left_sign,
        right_determinant_sign: right_sign,
        reason: reason.to_owned(),
    };

    if left_analysis.status == MapStatus::Unresolved
        || right_analysis.status == MapStatus::Unresolved
    {
        return Ok(base(
            CausticBracketStatus::Unresolved,
            None,
            "At least one endpoint's determinant sign is not resolved against its map uncertainty",
        ));
    }

    if left_sign == 0 || right_sign == 0 {
        return Ok(base(
            CausticBracketStatus::EndpointSingularity,
            Some(if left_sign == 0 {
                left.branch_parameter
            } else {
                right.branch_parameter
            }),
            "An endpoint is exactly singular under a zero declared map-error bound",
        ));
    }

    if left_sign == right_sign {
        return Ok(base(
            CausticBracketStatus::NoSignChange,
            None,
            "The resolved endpoint determinants have the same sign",
        ));
    }

    let left_log = left_analysis
        .log_absolute_determinant
        .ok_or(AuditError::NumericRange("left log determinant"))?;
    let right_log = right_analysis
        .log_absolute_determinant
        .ok_or(AuditError::NumericRange("right log determinant"))?;
    let difference = left_log - right_log;
    let fraction = if difference >= 0.0 {
        1.0 / (1.0 + (-difference).exp())
    } else {
        let exp_difference = difference.exp();
        exp_difference / (1.0 + exp_difference)
    };
    let estimate = left.branch_parameter
        + fraction * (right.branch_parameter - left.branch_parameter);

    Ok(base(
        CausticBracketStatus::Bracketed,
        Some(estimate),
        "Resolved determinant signs differ; endpoint interval brackets a parity change",
    ))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapAgreement {
    pub branch_id: String,
    pub normalization_id: String,
    pub absolute_frobenius_difference: f64,
    pub relative_frobenius_difference: f64,
}

/// Compare two independently generated maps under the same source/observer
/// screen-area normalization. This is a map discrepancy, not a physical error
/// bound unless the two methods' own error budgets are also supplied.
pub fn compare_maps(
    left: &JacobiMapEvidence,
    right: &JacobiMapEvidence,
) -> Result<MapAgreement, AuditError> {
    left.validate()?;
    right.validate()?;
    if left.normalization_id != right.normalization_id {
        return Err(AuditError::NormalizationMismatch);
    }
    if left.branch_id != right.branch_id {
        return Err(AuditError::BranchMismatch);
    }
    if left.branch_parameter != right.branch_parameter {
        return Err(AuditError::ParameterMismatch);
    }
    let l = left.map.entries;
    let r = right.map.entries;
    let difference = Matrix2::new(
        l[0][0] - r[0][0],
        l[0][1] - r[0][1],
        l[1][0] - r[1][0],
        l[1][1] - r[1][1],
    );
    if !difference.is_finite() {
        return Err(AuditError::NumericRange("map difference"));
    }
    let absolute = difference.frobenius_norm();
    let denominator = left.map.frobenius_norm().max(right.map.frobenius_norm());
    let relative = if denominator == 0.0 {
        if absolute == 0.0 { 0.0 } else { return Err(AuditError::NumericRange("relative map difference")); }
    } else {
        absolute / denominator
    };
    if !absolute.is_finite() || !relative.is_finite() {
        return Err(AuditError::NumericRange("map agreement"));
    }
    Ok(MapAgreement {
        branch_id: left.branch_id.clone(),
        normalization_id: left.normalization_id.clone(),
        absolute_frobenius_difference: absolute,
        relative_frobenius_difference: relative,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub enum AuditError {
    EmptyField(&'static str),
    NonFinite(&'static str),
    InvalidUncertainty(f64),
    InvalidConditionLimit(f64),
    NumericRange(&'static str),
    NormalizationMismatch,
    BranchMismatch,
    ParameterMismatch,
    NonIncreasingBranchParameter,
}

impl std::fmt::Display for AuditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyField(field) => write!(f, "{field} must be non-empty"),
            Self::NonFinite(field) => write!(f, "{field} must be finite"),
            Self::InvalidUncertainty(value) => write!(f, "map uncertainty must be finite and non-negative, got {value}"),
            Self::InvalidConditionLimit(value) => write!(f, "maximum condition number must be finite and >= 1, got {value}"),
            Self::NumericRange(field) => write!(f, "numeric range exhausted while evaluating {field}"),
            Self::NormalizationMismatch => write!(f, "Jacobi maps use different source/observer area normalizations"),
            Self::BranchMismatch => write!(f, "Jacobi maps refer to different tracked image branches"),
            Self::ParameterMismatch => write!(f, "Jacobi maps refer to different points along the tracked branch"),
            Self::NonIncreasingBranchParameter => write!(f, "caustic samples must be strictly increasing along one tracked branch"),
        }
    }
}
impl std::error::Error for AuditError {}

pub fn parse_and_analyze_json(
    input: &str,
    max_condition_number: f64,
) -> Result<Vec<JacobiMapAnalysis>, String> {
    let samples: Vec<JacobiMapEvidence> =
        serde_json::from_str(input).map_err(|error| format!("invalid Jacobi map JSON: {error}"))?;
    let mut sample_ids = BTreeSet::new();
    let mut branch_parameters = BTreeSet::new();
    let mut branch_normalizations = BTreeMap::new();
    for sample in &samples {
        sample.validate().map_err(|error| error.to_string())?;
        if !sample_ids.insert(sample.sample_id.as_str()) {
            return Err(format!("duplicate Jacobi map sample_id: {}", sample.sample_id));
        }
        if !branch_parameters.insert((sample.branch_id.as_str(), sample.branch_parameter.to_bits())) {
            return Err(format!(
                "duplicate branch parameter {} for branch {}",
                sample.branch_parameter, sample.branch_id
            ));
        }
        if let Some(existing) = branch_normalizations.insert(
            sample.branch_id.as_str(),
            sample.normalization_id.as_str(),
        ) {
            if existing != sample.normalization_id {
                return Err(format!(
                    "branch {} mixes source/observer normalizations",
                    sample.branch_id
                ));
            }
        }
    }
    samples
        .iter()
        .map(|sample| {
            sample
                .analyze(max_condition_number)
                .map_err(|error| error.to_string())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(id: &str, parameter: f64, entries: [[f64; 2]; 2], error: f64) -> JacobiMapEvidence {
        JacobiMapEvidence {
            sample_id: id.into(),
            branch_id: "tracked-image-branch-0".into(),
            branch_parameter: parameter,
            normalization_id: "source-orthonormal-area-v1/observer-unit-angle-v1".into(),
            map: Matrix2 { entries },
            componentwise_absolute_error_bound: error,
        }
    }

    #[test]
    fn flat_space_parallel_screen_known_answer_is_lambda_identity() {
        let lambda = 7.5;
        let map = sample("flat", 0.0, [[lambda, 0.0], [0.0, lambda]], 0.0);
        let result = map.analyze(DEFAULT_MAX_CONDITION_NUMBER).unwrap();
        assert_eq!(result.status, MapStatus::Resolved);
        assert!((result.singular_value_max - lambda).abs() < 1e-14);
        assert!((result.singular_value_min - lambda).abs() < 1e-14);
        assert_eq!(result.determinant_sign, 1);
        assert!((result.log_absolute_determinant.unwrap() - 2.0 * lambda.ln()).abs() < 1e-14);
        assert!((result.log_magnification.unwrap() + 2.0 * lambda.ln()).abs() < 1e-14);
    }

    #[test]
    fn screen_basis_rotations_preserve_singular_values_and_determinant_sign() {
        let original = sample("central", 1.0, [[2.0, 0.25], [-0.5, 1.25]], 0.0);
        let transformed = JacobiMapEvidence {
            map: original.map.rotate_screens(0.713, -1.127).unwrap(),
            ..original.clone()
        };
        let a = original.analyze(DEFAULT_MAX_CONDITION_NUMBER).unwrap();
        let b = transformed.analyze(DEFAULT_MAX_CONDITION_NUMBER).unwrap();
        assert_eq!(a.determinant_sign, b.determinant_sign);
        assert!((a.singular_value_max - b.singular_value_max).abs() < 2e-15);
        assert!((a.singular_value_min - b.singular_value_min).abs() < 2e-15);
        assert!((a.log_absolute_determinant.unwrap() - b.log_absolute_determinant.unwrap()).abs() < 2e-15);
    }

    #[test]
    fn ill_conditioned_maps_do_not_emit_magnification() {
        let map = sample("near-caustic", 2.0, [[1.0, 0.0], [0.0, 1.0e-12]], 0.0);
        let result = map.analyze(1.0e8).unwrap();
        assert_eq!(result.status, MapStatus::IllConditioned);
        assert!(result.log_magnification.is_none());
        assert!(result.condition_number.unwrap() > 1.0e8);
    }

    #[test]
    fn uncertainty_larger_than_smallest_singular_value_forces_abstention() {
        let map = sample("unresolved", 2.0, [[1.0, 0.0], [0.0, 1.0e-13]], 1.0e-13);
        let result = map.analyze(DEFAULT_MAX_CONDITION_NUMBER).unwrap();
        assert_eq!(result.status, MapStatus::Unresolved);
        assert!(result.log_magnification.is_none());
    }

    #[test]
    fn exactly_singular_map_is_never_clamped_to_finite_magnification() {
        let map = sample("singular", 2.0, [[1.0, 0.0], [0.0, 0.0]], 0.0);
        let result = map.analyze(DEFAULT_MAX_CONDITION_NUMBER).unwrap();
        assert_eq!(result.status, MapStatus::Singular);
        assert_eq!(result.determinant_sign, 0);
        assert!(result.log_magnification.is_none());
    }

    #[test]
    fn signed_determinant_change_brackets_caustic_on_tracked_branch() {
        let left = sample("left", 0.0, [[1.0, 0.0], [0.0, -0.2]], 0.0);
        let right = sample("right", 1.0, [[1.0, 0.0], [0.0, 0.3]], 0.0);
        let bracket = bracket_caustic(&left, &right, DEFAULT_MAX_CONDITION_NUMBER).unwrap();
        assert_eq!(bracket.status, CausticBracketStatus::Bracketed);
        assert_eq!((bracket.left_determinant_sign, bracket.right_determinant_sign), (-1, 1));
        assert!((bracket.interpolated_parameter_estimate.unwrap() - 0.4).abs() < 1e-14);
        assert_eq!((bracket.lower_parameter, bracket.upper_parameter), (0.0, 1.0));
    }

    #[test]
    fn uncertain_endpoint_abstains_instead_of_claiming_caustic() {
        let left = sample("left", 0.0, [[1.0, 0.0], [0.0, -1.0e-13]], 1.0e-13);
        let right = sample("right", 1.0, [[1.0, 0.0], [0.0, 0.3]], 0.0);
        let bracket = bracket_caustic(&left, &right, DEFAULT_MAX_CONDITION_NUMBER).unwrap();
        assert_eq!(bracket.status, CausticBracketStatus::Unresolved);
        assert!(bracket.interpolated_parameter_estimate.is_none());
    }

    #[test]
    fn same_parity_endpoints_do_not_claim_caustic() {
        let left = sample("left", 0.0, [[1.0, 0.0], [0.0, 0.2]], 0.0);
        let right = sample("right", 1.0, [[1.0, 0.0], [0.0, 0.3]], 0.0);
        let bracket = bracket_caustic(&left, &right, DEFAULT_MAX_CONDITION_NUMBER).unwrap();
        assert_eq!(bracket.status, CausticBracketStatus::NoSignChange);
    }

    #[test]
    fn endpoint_singularity_is_distinguished_from_a_sign_change() {
        let left = sample("left", 0.0, [[1.0, 0.0], [0.0, 0.0]], 0.0);
        let right = sample("right", 1.0, [[1.0, 0.0], [0.0, 0.2]], 0.0);
        let bracket = bracket_caustic(&left, &right, DEFAULT_MAX_CONDITION_NUMBER).unwrap();
        assert_eq!(bracket.status, CausticBracketStatus::EndpointSingularity);
        assert_eq!(bracket.interpolated_parameter_estimate, Some(0.0));
    }

    #[test]
    fn independent_maps_compare_only_under_matching_normalization() {
        let left = sample("variational", 0.0, [[2.0, 0.0], [0.0, 1.0]], 1.0e-14);
        let right = sample("finite-difference", 0.0, [[2.0, 1.0e-5], [0.0, 1.0]], 1.0e-14);
        let agreement = compare_maps(&left, &right).unwrap();
        assert!(agreement.relative_frobenius_difference > 0.0);
        assert!(agreement.relative_frobenius_difference < 1.0e-4);

        let mismatched = JacobiMapEvidence {
            normalization_id: "different-area-convention".into(),
            ..right.clone()
        };
        assert_eq!(compare_maps(&left, &mismatched), Err(AuditError::NormalizationMismatch));
        let wrong_branch = JacobiMapEvidence { branch_id: "other-branch".into(), ..mismatched.clone() };
        assert_eq!(compare_maps(&left, &wrong_branch), Err(AuditError::BranchMismatch));
        let wrong_point = JacobiMapEvidence { branch_parameter: 1.0, ..right.clone() };
        assert_eq!(compare_maps(&left, &wrong_point), Err(AuditError::ParameterMismatch));
    }

    #[test]
    fn malformed_uncertainty_and_nonfinite_maps_fail_closed() {
        let mut map = sample("bad", 0.0, [[1.0, 0.0], [0.0, 1.0]], -1.0);
        assert!(matches!(map.analyze(DEFAULT_MAX_CONDITION_NUMBER), Err(AuditError::InvalidUncertainty(_))));
        map.componentwise_absolute_error_bound = 0.0;
        map.map.entries[0][0] = f64::NAN;
        assert_eq!(map.analyze(DEFAULT_MAX_CONDITION_NUMBER), Err(AuditError::NonFinite("Jacobi map entry")));
    }

    #[test]
    fn branch_parameter_and_normalization_are_part_of_caustic_identity() {
        let left = sample("left", 1.0, [[1.0, 0.0], [0.0, -1.0]], 0.0);
        let mut right = sample("right", 0.5, [[1.0, 0.0], [0.0, 1.0]], 0.0);
        assert_eq!(bracket_caustic(&left, &right, DEFAULT_MAX_CONDITION_NUMBER), Err(AuditError::NonIncreasingBranchParameter));
        right.branch_parameter = 2.0;
        right.normalization_id = "another-normalization".into();
        assert_eq!(bracket_caustic(&left, &right, DEFAULT_MAX_CONDITION_NUMBER), Err(AuditError::NormalizationMismatch));
        right.normalization_id = left.normalization_id.clone();
        right.branch_id = "another-branch".into();
        assert_eq!(bracket_caustic(&left, &right, DEFAULT_MAX_CONDITION_NUMBER), Err(AuditError::BranchMismatch));
    }

    #[test]
    fn parses_the_schema_backed_synthetic_atlas_fixture() {
        let fixture = include_str!("../examples/synthetic-screen-map-atlas-v0.1.json");
        let analyses = parse_and_analyze_json(fixture, DEFAULT_MAX_CONDITION_NUMBER).unwrap();
        assert_eq!(analyses.len(), 4);
        assert_eq!(analyses[0].status, MapStatus::Resolved);
        let near_caustic = analyses.iter().find(|analysis| analysis.sample_id == "near-caustic-unresolved").unwrap();
        assert_eq!(near_caustic.status, MapStatus::Unresolved);
        assert!(near_caustic.log_magnification.is_none());
    }

    #[test]
    fn atlas_parser_rejects_duplicate_ids_parameters_and_mixed_branch_normalization() {
        let first = sample("same", 0.0, [[1.0, 0.0], [0.0, 1.0]], 0.0);
        let duplicate_id = sample("same", 1.0, [[1.0, 0.0], [0.0, 1.0]], 0.0);
        let duplicate_parameter = sample("other", 0.0, [[1.0, 0.0], [0.0, 1.0]], 0.0);
        let mixed_norm = JacobiMapEvidence {
            sample_id: "mixed".into(),
            branch_parameter: 2.0,
            normalization_id: "different".into(),
            ..first.clone()
        };
        let serialize = |samples: &[JacobiMapEvidence]| serde_json::to_string(samples).unwrap();
        assert!(parse_and_analyze_json(&serialize(&[first.clone(), duplicate_id]), DEFAULT_MAX_CONDITION_NUMBER)
            .unwrap_err().contains("duplicate Jacobi map sample_id"));
        assert!(parse_and_analyze_json(&serialize(&[first.clone(), duplicate_parameter]), DEFAULT_MAX_CONDITION_NUMBER)
            .unwrap_err().contains("duplicate branch parameter"));
        assert!(parse_and_analyze_json(&serialize(&[first, mixed_norm]), DEFAULT_MAX_CONDITION_NUMBER)
            .unwrap_err().contains("mixes source/observer normalizations"));
    }

    #[test]
    fn scale_normalization_avoids_determinant_overflow() {
        let map = sample("large", 0.0, [[1.0e200, 0.0], [0.0, 1.0e200]], 0.0);
        let result = map.analyze(DEFAULT_MAX_CONDITION_NUMBER).unwrap();
        assert_eq!(result.status, MapStatus::Resolved);
        assert!((result.log_absolute_determinant.unwrap() - 2.0 * 1.0e200_f64.ln()).abs() < 1e-12);
        assert!(result.log_magnification.is_some());
    }
}
