use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const SCHEMA_VERSION: &str = "luminous.plasma-metriplectic-audit.v0.1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditTolerances {
    /// Relative max-entry tolerance for L + L^T.
    pub reversible_skew_relative: f64,
    /// Relative max-entry tolerance for M - M^T.
    pub dissipative_symmetry_relative: f64,
    /// Relative tolerance for the minimum eigenvalue of normalized M.
    pub dissipative_psd_relative: f64,
    /// Relative tolerance for L * grad(S) = 0.
    pub entropy_degeneracy_relative: f64,
    /// Relative tolerance for M * grad(E) = 0.
    pub energy_degeneracy_relative: f64,
    /// Relative tolerance for grad(E) dot du/dt = 0.
    pub energy_rate_relative: f64,
    /// Relative tolerance for a negative grad(S) dot du/dt.
    pub entropy_rate_relative: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditRequest {
    pub schema_version: String,
    pub request_id: String,
    pub source_id: String,
    pub state_id: String,
    pub coordinate_frame_id: String,
    pub state_units_id: String,
    pub energy_units_id: String,
    pub entropy_units_id: String,
    pub state: Vec<f64>,
    pub energy_gradient: Vec<f64>,
    pub entropy_gradient: Vec<f64>,
    /// Row-major reversible operator L, dimension n × n.
    pub reversible_operator: Vec<f64>,
    /// Row-major dissipative operator M, dimension n × n.
    pub dissipative_operator: Vec<f64>,
    pub tolerances: AuditTolerances,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditDisposition {
    Admissible,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditFinding {
    pub code: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditReportBody {
    pub schema_version: String,
    pub request_id: String,
    pub source_id: String,
    pub state_id: String,
    pub coordinate_frame_id: String,
    pub state_dimension: usize,
    pub request_sha256: String,
    pub disposition: AuditDisposition,
    pub reversible_skew_relative_residual: f64,
    pub dissipative_symmetry_relative_residual: f64,
    pub dissipative_minimum_normalized_eigenvalue: Option<f64>,
    pub entropy_degeneracy_relative_residual: f64,
    pub energy_degeneracy_relative_residual: f64,
    pub energy_rate: f64,
    pub energy_rate_relative_residual: f64,
    pub entropy_rate: f64,
    pub entropy_rate_relative_residual: f64,
    pub time_derivative: Vec<f64>,
    pub findings: Vec<AuditFinding>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditReport {
    pub body: AuditReportBody,
    pub receipt_sha256: String,
}

fn non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn max_abs(values: &[f64]) -> f64 {
    values.iter().fold(0.0_f64, |scale, value| scale.max(value.abs()))
}

fn stable_norm(values: &[f64]) -> f64 {
    let scale = max_abs(values);
    if scale == 0.0 {
        return 0.0;
    }
    let sum = values
        .iter()
        .map(|value| {
            let normalized = value / scale;
            normalized * normalized
        })
        .sum::<f64>();
    scale * sum.sqrt()
}

fn relative_residual(residual: &[f64], left: &[f64], right: &[f64]) -> f64 {
    let denominator = stable_norm(left) * stable_norm(right);
    let numerator = stable_norm(residual);
    if denominator > f64::MIN_POSITIVE && denominator.is_finite() {
        numerator / denominator
    } else {
        numerator
    }
}

fn validate_tolerances(tolerances: &AuditTolerances) -> Result<(), String> {
    for (name, value) in [
        ("reversible_skew_relative", tolerances.reversible_skew_relative),
        ("dissipative_symmetry_relative", tolerances.dissipative_symmetry_relative),
        ("dissipative_psd_relative", tolerances.dissipative_psd_relative),
        ("entropy_degeneracy_relative", tolerances.entropy_degeneracy_relative),
        ("energy_degeneracy_relative", tolerances.energy_degeneracy_relative),
        ("energy_rate_relative", tolerances.energy_rate_relative),
        ("entropy_rate_relative", tolerances.entropy_rate_relative),
    ] {
        if !value.is_finite() || value < 0.0 || value >= 1.0 {
            return Err(format!("{name} must be finite and in [0, 1)"));
        }
    }
    Ok(())
}

fn mat_vec(matrix: &[f64], vector: &[f64], dimension: usize) -> Vec<f64> {
    let mut output = vec![0.0; dimension];
    for (row_index, output_value) in output.iter_mut().enumerate() {
        let row_start = row_index * dimension;
        let row = &matrix[row_start..row_start + dimension];
        *output_value = row
            .iter()
            .zip(vector.iter())
            .map(|(coefficient, value)| coefficient * value)
            .sum();
    }
    output
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right.iter())
        .map(|(left_value, right_value)| left_value * right_value)
        .sum()
}

fn normalized_matrix(matrix: &[f64], scale: f64) -> Vec<f64> {
    matrix.iter().map(|value| value / scale).collect()
}

fn matrix_skew_residual(matrix: &[f64], dimension: usize) -> f64 {
    let scale = max_abs(matrix);
    if scale == 0.0 {
        return 0.0;
    }
    let mut residual = 0.0_f64;
    for (row, row_values) in matrix.chunks_exact(dimension).enumerate() {
        for (column, value) in row_values.iter().enumerate() {
            residual = residual.max(
                (*value / scale + matrix[column * dimension + row] / scale).abs(),
            );
        }
    }
    residual
}

fn matrix_symmetry_residual(matrix: &[f64], dimension: usize) -> f64 {
    let scale = max_abs(matrix);
    if scale == 0.0 {
        return 0.0;
    }
    let mut residual = 0.0_f64;
    for (row, row_values) in matrix.chunks_exact(dimension).enumerate() {
        for (column, value) in row_values.iter().enumerate() {
            residual = residual.max(
                (*value / scale - matrix[column * dimension + row] / scale).abs(),
            );
        }
    }
    residual
}

fn symmetric_part(matrix: &[f64], dimension: usize, scale: f64) -> Vec<f64> {
    let mut output = vec![0.0; dimension * dimension];
    for (row, row_values) in output.chunks_exact_mut(dimension).enumerate() {
        for (column, value) in row_values.iter_mut().enumerate() {
            *value = 0.5
                * (matrix[row * dimension + column] / scale
                    + matrix[column * dimension + row] / scale);
        }
    }
    output
}

/// Return the minimum eigenvalue of a finite symmetric matrix using cyclic Jacobi rotations.
/// Input entries should be scaled to O(1); the method is intended for audit-sized discretisations.
fn minimum_symmetric_eigenvalue(
    matrix: &[f64],
    dimension: usize,
) -> Result<f64, String> {
    if dimension == 0 || matrix.len() != dimension * dimension {
        return Err("invalid symmetric matrix dimensions".to_owned());
    }
    let mut work = matrix.to_vec();
    let max_iterations = 64_usize
        .checked_mul(dimension)
        .and_then(|value| value.checked_mul(dimension))
        .ok_or_else(|| "eigenvalue iteration bound overflow".to_owned())?;

    for _ in 0..max_iterations {
        let mut p = 0_usize;
        let mut q = 0_usize;
        let mut largest_off_diagonal = 0.0_f64;
        for (row, row_values) in work.chunks_exact(dimension).enumerate() {
            for (column, value) in row_values.iter().enumerate().skip(row + 1) {
                if value.abs() > largest_off_diagonal {
                    largest_off_diagonal = value.abs();
                    p = row;
                    q = column;
                }
            }
        }
        if largest_off_diagonal <= 1.0e-14 {
            let minimum = work
                .chunks_exact(dimension)
                .enumerate()
                .map(|(index, row)| row[index])
                .fold(f64::INFINITY, f64::min);
            if minimum.is_finite() {
                return Ok(minimum);
            }
            return Err("symmetric eigenvalue iteration produced a non-finite result".to_owned());
        }

        let pp = p * dimension + p;
        let qq = q * dimension + q;
        let pq = p * dimension + q;
        let app = work[pp];
        let aqq = work[qq];
        let apq = work[pq];
        if apq == 0.0 {
            continue;
        }

        let tau = (aqq - app) / (2.0 * apq);
        let tangent = if tau >= 0.0 {
            1.0 / (tau + (1.0 + tau * tau).sqrt())
        } else {
            -1.0 / (-tau + (1.0 + tau * tau).sqrt())
        };
        let cosine = 1.0 / (1.0 + tangent * tangent).sqrt();
        let sine = tangent * cosine;

        let off_diagonal_updates = work
            .chunks_exact(dimension)
            .enumerate()
            .filter(|(index, _)| *index != p && *index != q)
            .map(|(index, row)| (index, row[p], row[q]))
            .collect::<Vec<_>>();
        for (index, aip, aiq) in off_diagonal_updates {
            let ip = index * dimension + p;
            let iq = index * dimension + q;
            let new_ip = cosine * aip - sine * aiq;
            let new_iq = sine * aip + cosine * aiq;
            work[ip] = new_ip;
            work[p * dimension + index] = new_ip;
            work[iq] = new_iq;
            work[q * dimension + index] = new_iq;
        }

        work[pp] = app - tangent * apq;
        work[qq] = aqq + tangent * apq;
        work[pq] = 0.0;
        work[q * dimension + p] = 0.0;
    }

    Err("symmetric Jacobi eigenvalue iteration did not converge".to_owned())
}

fn validate_request(request: &AuditRequest) -> Result<usize, String> {
    if request.schema_version != SCHEMA_VERSION {
        return Err("unsupported metriplectic audit schema version".to_owned());
    }
    for (name, value) in [
        ("request_id", request.request_id.as_str()),
        ("source_id", request.source_id.as_str()),
        ("state_id", request.state_id.as_str()),
        ("coordinate_frame_id", request.coordinate_frame_id.as_str()),
        ("state_units_id", request.state_units_id.as_str()),
        ("energy_units_id", request.energy_units_id.as_str()),
        ("entropy_units_id", request.entropy_units_id.as_str()),
    ] {
        if !non_empty(value) {
            return Err(format!("{name} must be non-empty"));
        }
    }
    validate_tolerances(&request.tolerances)?;
    let dimension = request.state.len();
    if dimension == 0 || dimension > 64 {
        return Err("state dimension must be in 1..=64 for v0.1 audit".to_owned());
    }
    let expected_matrix_size = dimension
        .checked_mul(dimension)
        .ok_or_else(|| "operator matrix dimension overflow".to_owned())?;
    if request.energy_gradient.len() != dimension
        || request.entropy_gradient.len() != dimension
        || request.reversible_operator.len() != expected_matrix_size
        || request.dissipative_operator.len() != expected_matrix_size
    {
        return Err("state, gradient or operator dimensions do not match".to_owned());
    }
    let all_values = request
        .state
        .iter()
        .chain(request.energy_gradient.iter())
        .chain(request.entropy_gradient.iter())
        .chain(request.reversible_operator.iter())
        .chain(request.dissipative_operator.iter());
    if all_values.clone().any(|value| !value.is_finite()) {
        return Err("state, gradients and operators must contain only finite values".to_owned());
    }
    Ok(dimension)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

pub fn audit_metriplectic_structure(request: &AuditRequest) -> Result<AuditReport, String> {
    let dimension = validate_request(request)?;
    let request_bytes =
        serde_json::to_vec(request).map_err(|error| format!("request serialization failed: {error}"))?;
    let request_sha256 = sha256_hex(&request_bytes);

    let l_scale = max_abs(&request.reversible_operator).max(f64::MIN_POSITIVE);
    let m_scale = max_abs(&request.dissipative_operator).max(f64::MIN_POSITIVE);
    let l_normalized = normalized_matrix(&request.reversible_operator, l_scale);
    let m_normalized = normalized_matrix(&request.dissipative_operator, m_scale);

    let reversible_skew_relative_residual =
        matrix_skew_residual(&request.reversible_operator, dimension);
    let dissipative_symmetry_relative_residual =
        matrix_symmetry_residual(&request.dissipative_operator, dimension);
    let dissipative_minimum_normalized_eigenvalue =
        if dissipative_symmetry_relative_residual
            <= request.tolerances.dissipative_symmetry_relative
        {
            let symmetric_dissipative =
                symmetric_part(&request.dissipative_operator, dimension, m_scale);
            Some(minimum_symmetric_eigenvalue(&symmetric_dissipative, dimension)?)
        } else {
            None
        };

    let entropy_gradient_scale = max_abs(&request.entropy_gradient).max(f64::MIN_POSITIVE);
    let energy_gradient_scale = max_abs(&request.energy_gradient).max(f64::MIN_POSITIVE);
    let entropy_gradient_normalized: Vec<f64> = request
        .entropy_gradient
        .iter()
        .map(|value| value / entropy_gradient_scale)
        .collect();
    let energy_gradient_normalized: Vec<f64> = request
        .energy_gradient
        .iter()
        .map(|value| value / energy_gradient_scale)
        .collect();

    let entropy_degeneracy_vector =
        mat_vec(&l_normalized, &entropy_gradient_normalized, dimension);
    let energy_degeneracy_vector =
        mat_vec(&m_normalized, &energy_gradient_normalized, dimension);
    let entropy_degeneracy_relative_residual = relative_residual(
        &entropy_degeneracy_vector,
        &l_normalized,
        &entropy_gradient_normalized,
    );
    let energy_degeneracy_relative_residual = relative_residual(
        &energy_degeneracy_vector,
        &m_normalized,
        &energy_gradient_normalized,
    );

    let reversible_rate =
        mat_vec(&request.reversible_operator, &request.energy_gradient, dimension);
    let dissipative_rate =
        mat_vec(&request.dissipative_operator, &request.entropy_gradient, dimension);
    let time_derivative: Vec<f64> = reversible_rate
        .iter()
        .zip(dissipative_rate.iter())
        .map(|(reversible, dissipative)| reversible + dissipative)
        .collect();
    let energy_rate = dot(&request.energy_gradient, &time_derivative);
    let entropy_rate = dot(&request.entropy_gradient, &time_derivative);
    if time_derivative.iter().any(|value| !value.is_finite())
        || !energy_rate.is_finite()
        || !entropy_rate.is_finite()
    {
        return Err("derived GENERIC time derivative or rate is non-finite".to_owned());
    }

    let time_derivative_scale = max_abs(&time_derivative).max(f64::MIN_POSITIVE);
    let normalized_time_derivative: Vec<f64> = time_derivative
        .iter()
        .map(|value| value / time_derivative_scale)
        .collect();
    let energy_rate_relative_residual = relative_residual(
        &[dot(&energy_gradient_normalized, &normalized_time_derivative)],
        &energy_gradient_normalized,
        &normalized_time_derivative,
    );
    let entropy_rate_relative_residual = relative_residual(
        &[(-dot(&entropy_gradient_normalized, &normalized_time_derivative)).max(0.0)],
        &entropy_gradient_normalized,
        &normalized_time_derivative,
    );

    let mut findings = Vec::new();
    let mut rejected = false;

    let checks = [
        (
            "reversible_operator_not_skew",
            reversible_skew_relative_residual,
            request.tolerances.reversible_skew_relative,
            "L + L^T exceeds the declared relative skew-adjointness tolerance.",
        ),
        (
            "dissipative_operator_not_symmetric",
            dissipative_symmetry_relative_residual,
            request.tolerances.dissipative_symmetry_relative,
            "M - M^T exceeds the declared relative symmetry tolerance.",
        ),
        (
            "entropy_degeneracy_violated",
            entropy_degeneracy_relative_residual,
            request.tolerances.entropy_degeneracy_relative,
            "L times the entropy gradient is nonzero beyond tolerance.",
        ),
        (
            "energy_degeneracy_violated",
            energy_degeneracy_relative_residual,
            request.tolerances.energy_degeneracy_relative,
            "M times the energy gradient is nonzero beyond tolerance.",
        ),
        (
            "energy_rate_nonzero",
            energy_rate_relative_residual,
            request.tolerances.energy_rate_relative,
            "The supplied local energy rate is nonzero beyond tolerance.",
        ),
        (
            "entropy_rate_negative",
            entropy_rate_relative_residual,
            request.tolerances.entropy_rate_relative,
            "The supplied local entropy rate is negative beyond tolerance.",
        ),
    ];
    for (code, actual, tolerance, detail) in checks {
        if actual > tolerance {
            rejected = true;
            findings.push(AuditFinding {
                code: code.to_owned(),
                detail: detail.to_owned(),
            });
        }
    }
    if dissipative_minimum_normalized_eigenvalue
        .is_some_and(|value| value < -request.tolerances.dissipative_psd_relative)
    {
        rejected = true;
        findings.push(AuditFinding {
            code: "dissipative_operator_not_positive_semidefinite".to_owned(),
            detail: concat!(
                "The minimum eigenvalue of the scale-normalized symmetric part of M ",
                "is below the declared PSD tolerance."
            )
            .to_owned(),
        });
    }

    let body = AuditReportBody {
        schema_version: SCHEMA_VERSION.to_owned(),
        request_id: request.request_id.clone(),
        source_id: request.source_id.clone(),
        state_id: request.state_id.clone(),
        coordinate_frame_id: request.coordinate_frame_id.clone(),
        state_dimension: dimension,
        request_sha256,
        disposition: if rejected {
            AuditDisposition::Rejected
        } else {
            AuditDisposition::Admissible
        },
        reversible_skew_relative_residual,
        dissipative_symmetry_relative_residual,
        dissipative_minimum_normalized_eigenvalue,
        entropy_degeneracy_relative_residual,
        energy_degeneracy_relative_residual,
        energy_rate,
        energy_rate_relative_residual,
        entropy_rate,
        entropy_rate_relative_residual,
        time_derivative,
        findings,
        claim_ceiling: concat!(
            "FINITE_DIMENSIONAL_LOCAL_GENERIC_IDENTITIES_ONLY_",
            "NOT_A_FUNCTION_SPACE_OR_PLASMA_SOLVER_PROOF"
        )
        .to_owned(),
    };
    let receipt_bytes =
        serde_json::to_vec(&body).map_err(|error| format!("report serialization failed: {error}"))?;
    Ok(AuditReport {
        body,
        receipt_sha256: sha256_hex(&receipt_bytes),
    })
}

pub fn verify_metriplectic_report(
    request: &AuditRequest,
    report: &AuditReport,
) -> Result<(), String> {
    let expected = audit_metriplectic_structure(request)?;
    if expected != *report {
        return Err("replayed metriplectic report differs from the supplied report".to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tolerances() -> AuditTolerances {
        AuditTolerances {
            reversible_skew_relative: 1.0e-12,
            dissipative_symmetry_relative: 1.0e-12,
            dissipative_psd_relative: 1.0e-12,
            entropy_degeneracy_relative: 1.0e-12,
            energy_degeneracy_relative: 1.0e-12,
            energy_rate_relative: 1.0e-12,
            entropy_rate_relative: 1.0e-12,
        }
    }

    fn base_request() -> AuditRequest {
        AuditRequest {
            schema_version: SCHEMA_VERSION.to_owned(),
            request_id: "mixed-generic-known-answer".to_owned(),
            source_id: "synthetic-structure-fixture".to_owned(),
            state_id: "state-3d-1".to_owned(),
            coordinate_frame_id: "state-space-basis-xyz".to_owned(),
            state_units_id: "normalized_state".to_owned(),
            energy_units_id: "normalized_energy".to_owned(),
            entropy_units_id: "normalized_entropy".to_owned(),
            state: vec![1.0, 0.0, 0.0],
            energy_gradient: vec![1.0, 0.0, 0.0],
            entropy_gradient: vec![0.0, 0.0, 1.0],
            reversible_operator: vec![
                0.0, -1.0, 0.0,
                1.0, 0.0, 0.0,
                0.0, 0.0, 0.0,
            ],
            dissipative_operator: vec![
                0.0, 0.0, 0.0,
                0.0, 0.0, 0.0,
                0.0, 0.0, 1.0,
            ],
            tolerances: tolerances(),
        }
    }

    #[test]
    fn mixed_reversible_dissipative_known_answer_is_admissible() {
        let report = audit_metriplectic_structure(&base_request()).expect("audit");
        assert_eq!(report.body.disposition, AuditDisposition::Admissible);
        assert_eq!(report.body.time_derivative, vec![0.0, 1.0, 1.0]);
        assert!((report.body.energy_rate).abs() < 1.0e-12);
        assert!((report.body.entropy_rate - 1.0).abs() < 1.0e-12);
        assert!(
            report
                .body
                .dissipative_minimum_normalized_eigenvalue
                .is_some_and(|value| value >= -1.0e-12)
        );
    }

    #[test]
    fn pure_reversible_case_with_zero_dissipation_is_admissible() {
        let mut request = base_request();
        request.dissipative_operator = vec![0.0; 9];
        let report = audit_metriplectic_structure(&request).expect("audit");
        assert_eq!(report.body.disposition, AuditDisposition::Admissible);
        assert!(report.body.entropy_rate.abs() < 1.0e-12);
    }

    #[test]
    fn rank_deficient_positive_semidefinite_dissipator_is_admissible() {
        let mut request = base_request();
        request.dissipative_operator = vec![
            0.0, 0.0, 0.0,
            0.0, 0.0, 0.0,
            0.0, 0.0, 1.0,
        ];
        let report = audit_metriplectic_structure(&request).expect("audit");
        assert_eq!(report.body.disposition, AuditDisposition::Admissible);
    }

    #[test]
    fn non_skew_reversible_operator_is_rejected() {
        let mut request = base_request();
        request.reversible_operator[1] = 0.5;
        let report = audit_metriplectic_structure(&request).expect("audit report");
        assert_eq!(report.body.disposition, AuditDisposition::Rejected);
        assert!(report.body.findings.iter().any(|finding| {
            finding.code == "reversible_operator_not_skew"
        }));
    }

    #[test]
    fn non_symmetric_dissipative_operator_is_rejected() {
        let mut request = base_request();
        request.dissipative_operator[2] = 0.25;
        let report = audit_metriplectic_structure(&request).expect("audit report");
        assert_eq!(report.body.disposition, AuditDisposition::Rejected);
        assert!(report.body.findings.iter().any(|finding| {
            finding.code == "dissipative_operator_not_symmetric"
        }));
    }

    #[test]
    fn materially_asymmetric_dissipator_has_no_psd_eigenvalue_claim() {
        let mut request = base_request();
        request.dissipative_operator[2] = 0.25;
        let report = audit_metriplectic_structure(&request).expect("audit report");
        assert_eq!(report.body.disposition, AuditDisposition::Rejected);
        assert_eq!(report.body.dissipative_minimum_normalized_eigenvalue, None);
    }

    #[test]
    fn symmetric_indefinite_dissipator_is_rejected() {
        let mut request = base_request();
        request.dissipative_operator = vec![
            0.0, 0.0, 0.0,
            0.0, -0.1, 0.0,
            0.0, 0.0, 1.0,
        ];
        let report = audit_metriplectic_structure(&request).expect("audit report");
        assert_eq!(report.body.disposition, AuditDisposition::Rejected);
        assert!(report.body.findings.iter().any(|finding| {
            finding.code == "dissipative_operator_not_positive_semidefinite"
        }));
    }

    #[test]
    fn entropy_degeneracy_violation_is_rejected() {
        let mut request = base_request();
        request.entropy_gradient = vec![1.0, 0.0, 0.0];
        let report = audit_metriplectic_structure(&request).expect("audit report");
        assert_eq!(report.body.disposition, AuditDisposition::Rejected);
        assert!(report.body.findings.iter().any(|finding| {
            finding.code == "entropy_degeneracy_violated"
        }));
    }

    #[test]
    fn energy_degeneracy_violation_is_rejected() {
        let mut request = base_request();
        request.energy_gradient = vec![0.0, 0.0, 1.0];
        let report = audit_metriplectic_structure(&request).expect("audit report");
        assert_eq!(report.body.disposition, AuditDisposition::Rejected);
        assert!(report.body.findings.iter().any(|finding| {
            finding.code == "energy_degeneracy_violated"
        }));
    }

    #[test]
    fn small_negative_entropy_rate_is_checked_independently_of_psd_tolerance() {
        let mut request = base_request();
        request.dissipative_operator[4] = 1.0;
        request.dissipative_operator[8] = -1.0e-13;
        request.tolerances.dissipative_psd_relative = 1.0e-12;
        request.tolerances.entropy_rate_relative = 1.0e-14;
        let report = audit_metriplectic_structure(&request).expect("audit report");
        assert_eq!(report.body.disposition, AuditDisposition::Rejected);
        assert!(report.body.findings.iter().any(|finding| {
            finding.code == "entropy_rate_negative"
        }));
    }

    #[test]
    fn non_finite_values_and_dimension_mismatch_are_refused() {
        let mut request = base_request();
        request.energy_gradient[0] = f64::NAN;
        assert!(audit_metriplectic_structure(&request).is_err());

        let mut request = base_request();
        request.entropy_gradient.pop();
        assert!(audit_metriplectic_structure(&request).is_err());
    }

    #[test]
    fn invalid_tolerance_and_excessive_dimension_are_refused() {
        let mut request = base_request();
        request.tolerances.dissipative_psd_relative = 1.0;
        assert!(audit_metriplectic_structure(&request).is_err());

        let mut request = base_request();
        request.state = vec![0.0; 65];
        assert!(audit_metriplectic_structure(&request).is_err());
    }

    #[test]
    fn operator_checks_are_scale_invariant_for_small_and_large_coefficients() {
        let mut small = base_request();
        small.reversible_operator = small.reversible_operator.iter().map(|value| value * 1.0e-100).collect();
        small.dissipative_operator = small.dissipative_operator.iter().map(|value| value * 1.0e-100).collect();
        let report_small = audit_metriplectic_structure(&small).expect("small audit");
        assert_eq!(report_small.body.disposition, AuditDisposition::Admissible);
        assert!(report_small.body.reversible_skew_relative_residual < 1.0e-12);
        assert!(report_small.body.dissipative_minimum_normalized_eigenvalue >= -1.0e-12);

        let mut large = base_request();
        large.reversible_operator = large.reversible_operator.iter().map(|value| value * 1.0e100).collect();
        large.dissipative_operator = large.dissipative_operator.iter().map(|value| value * 1.0e100).collect();
        let report_large = audit_metriplectic_structure(&large).expect("large audit");
        assert_eq!(report_large.body.disposition, AuditDisposition::Admissible);
        assert!(report_large.body.reversible_skew_relative_residual < 1.0e-12);
    }

    #[test]
    fn deterministic_receipt_for_identical_operator_contract() {
        let request = base_request();
        let first = audit_metriplectic_structure(&request).expect("first");
        let second = audit_metriplectic_structure(&request).expect("second");
        assert_eq!(first, second);
        assert_eq!(first.receipt_sha256, second.receipt_sha256);
    }

    #[test]
    fn frozen_mixed_structure_fixture_matches_known_answer() {
        let input = include_str!("../../../../benchmarks/plasma/metriplectic-audit-v0.json");
        let request: AuditRequest = serde_json::from_str(input).expect("fixture parses");
        let report = audit_metriplectic_structure(&request).expect("audit");
        assert_eq!(report.body.disposition, AuditDisposition::Admissible);
        assert_eq!(report.body.time_derivative, vec![0.0, 1.0, 1.0]);
        assert!(report.body.energy_rate.abs() < 1.0e-12);
        assert!((report.body.entropy_rate - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn frozen_indefinite_fixture_is_rejected() {
        let input =
            include_str!("../../../../benchmarks/plasma/metriplectic-audit-indefinite-v0.json");
        let request: AuditRequest = serde_json::from_str(input).expect("fixture parses");
        let report = audit_metriplectic_structure(&request).expect("audit report");
        assert_eq!(report.body.disposition, AuditDisposition::Rejected);
        assert!(report.body.findings.iter().any(|finding| {
            finding.code == "dissipative_operator_not_positive_semidefinite"
        }));
    }

    #[test]
    fn jacobi_eigensolver_recovers_off_diagonal_symmetric_eigenvalue() {
        let minimum = minimum_symmetric_eigenvalue(
            &[1.0, 0.5, 0.0, 0.5, 1.0, 0.0, 0.0, 0.0, 0.0],
            3,
        )
        .expect("eigenvalue");
        assert!((minimum - 0.5).abs() < 1.0e-12);
    }

    #[test]
    fn verifier_rejects_rehashed_report_tampering() {
        let request = base_request();
        let mut report = audit_metriplectic_structure(&request).expect("audit");
        verify_metriplectic_report(&request, &report).expect("valid report");
        report.body.entropy_rate = -10.0;
        let bytes = serde_json::to_vec(&report.body).expect("serialize body");
        report.receipt_sha256 = sha256_hex(&bytes);
        assert!(verify_metriplectic_report(&request, &report).is_err());
    }

    #[test]
    fn unknown_schema_fields_are_refused() {
        let input = r#"{
            "schema_version":"luminous.plasma-metriplectic-audit.v0.1",
            "request_id":"x",
            "unexpected":true
        }"#;
        assert!(serde_json::from_str::<AuditRequest>(input).is_err());
    }
}
