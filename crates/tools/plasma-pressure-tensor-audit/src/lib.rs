use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const SCHEMA_VERSION: &str = "luminous.plasma-pressure-tensor-audit.v0.1";

pub type Matrix3 = [[f64; 3]; 3];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditTolerances {
    /// Relative max-entry tolerance for P_ij - P_ji, scaled by max(|P_ij|).
    pub symmetry_relative: f64,
    /// Dimensionless tolerance for scale-normalized principal minors.
    pub psd_relative: f64,
    /// Relative tolerance for the pressure-work decomposition identity.
    pub energy_identity_relative: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditRequest {
    pub schema_version: String,
    pub request_id: String,
    pub source_id: String,
    pub coordinate_frame_id: String,
    pub pressure_units: String,
    pub gradient_units: String,
    /// P_ij = m ∫ (v_i - V_i)(v_j - V_j) f d^3v.
    pub pressure_tensor: Matrix3,
    /// G_ij = ∂_j V_i. Rows are velocity components; columns are derivative axes.
    pub velocity_gradient: Matrix3,
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
pub struct TensorMetrics {
    pub symmetry_relative_residual: f64,
    pub trace: f64,
    pub determinant: f64,
    pub frobenius_norm: f64,
    pub tensor_scale: f64,
    pub normalized_min_diagonal_minor: f64,
    pub normalized_min_two_by_two_principal_minor: f64,
    pub normalized_determinant: f64,
    pub positive_semidefinite: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PressureStrainMetrics {
    pub pressure_scalar: f64,
    pub velocity_divergence: f64,
    /// D_ij = (G_ij + G_ji)/2 - theta delta_ij / 3.
    pub deviatoric_strain: Matrix3,
    /// Π_ij = P_ij - p delta_ij.
    pub deviatoric_pressure: Matrix3,
    /// -P_ij G_ij.
    pub total_pressure_work: f64,
    /// -p theta.
    pub pressure_dilation_work: f64,
    /// -Π_ij D_ij.
    pub deviatoric_pressure_strain_work: f64,
    pub energy_identity_absolute_residual: f64,
    pub energy_identity_relative_residual: f64,
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
    pub coordinate_frame_id: String,
    pub pressure_units: String,
    pub gradient_units: String,
    pub request_sha256: String,
    pub disposition: AuditDisposition,
    pub tensor: TensorMetrics,
    pub pressure_strain: PressureStrainMetrics,
    pub findings: Vec<AuditFinding>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditReport {
    pub body: AuditReportBody,
    pub receipt_sha256: String,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

fn non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn finite_matrix(matrix: &Matrix3) -> bool {
    matrix.iter().flatten().all(|value| value.is_finite())
}

fn determinant(matrix: &Matrix3) -> f64 {
    matrix[0][0] * (matrix[1][1] * matrix[2][2] - matrix[1][2] * matrix[2][1])
        - matrix[0][1] * (matrix[1][0] * matrix[2][2] - matrix[1][2] * matrix[2][0])
        + matrix[0][2] * (matrix[1][0] * matrix[2][1] - matrix[1][1] * matrix[2][0])
}

fn contract_product(a: &Matrix3, b: &Matrix3) -> f64 {
    a.iter()
        .zip(b.iter())
        .flat_map(|(a_row, b_row)| a_row.iter().zip(b_row.iter()))
        .map(|(a_value, b_value)| a_value * b_value)
        .sum()
}

fn symmetrize(matrix: &Matrix3) -> Matrix3 {
    let mut output = [[0.0; 3]; 3];
    for (i, row) in output.iter_mut().enumerate() {
        for (j, value) in row.iter_mut().enumerate() {
            *value = 0.5 * (matrix[i][j] + matrix[j][i]);
        }
    }
    output
}

fn validate_tolerances(tolerances: &AuditTolerances) -> Result<(), String> {
    for (name, value) in [
        ("symmetry_relative", tolerances.symmetry_relative),
        ("psd_relative", tolerances.psd_relative),
        ("energy_identity_relative", tolerances.energy_identity_relative),
    ] {
        if !value.is_finite() || value < 0.0 || value >= 1.0 {
            return Err(format!("{name} must be finite and in [0, 1)"));
        }
    }
    Ok(())
}

fn tensor_metrics(
    input: &Matrix3,
    symmetry_tolerance: f64,
    psd_tolerance: f64,
) -> Result<(Matrix3, TensorMetrics), String> {
    let mut scale = input.iter().flatten().fold(0.0_f64, |acc, value| acc.max(value.abs()));
    if scale == 0.0 {
        scale = 1.0;
    }

    let mut max_asymmetry = 0.0_f64;
    for (i, row) in input.iter().enumerate() {
        for (j, value) in row.iter().enumerate() {
            max_asymmetry = max_asymmetry.max((value - input[j][i]).abs());
        }
    }
    let symmetry_relative_residual = max_asymmetry / scale;
    let symmetric = symmetrize(input);
    let mut normalized = [[0.0; 3]; 3];
    for (i, row) in normalized.iter_mut().enumerate() {
        for (j, value) in row.iter_mut().enumerate() {
            *value = symmetric[i][j] / scale;
        }
    }

    // For a symmetric 3x3 matrix, nonnegative principal minors are
    // necessary and sufficient for positive semidefiniteness.
    let diagonal_minors = [
        normalized[0][0],
        normalized[1][1],
        normalized[2][2],
    ];
    let two_by_two_minors = [
        normalized[0][0] * normalized[1][1] - normalized[0][1] * normalized[0][1],
        normalized[0][0] * normalized[2][2] - normalized[0][2] * normalized[0][2],
        normalized[1][1] * normalized[2][2] - normalized[1][2] * normalized[1][2],
    ];
    let normalized_determinant = determinant(&normalized);
    let normalized_min_diagonal_minor = diagonal_minors.into_iter().fold(f64::INFINITY, f64::min);
    let normalized_min_two_by_two_principal_minor =
        two_by_two_minors.into_iter().fold(f64::INFINITY, f64::min);
    let positive_semidefinite = normalized_min_diagonal_minor >= -psd_tolerance
        && normalized_min_two_by_two_principal_minor >= -psd_tolerance
        && normalized_determinant >= -psd_tolerance;

    let trace = symmetric[0][0] + symmetric[1][1] + symmetric[2][2];
    let determinant_value = normalized_determinant * scale * scale * scale;
    let normalized_frobenius_squared = normalized.iter().flatten().map(|value| value * value).sum::<f64>();
    let frobenius_norm = scale * normalized_frobenius_squared.sqrt();

    if !symmetry_relative_residual.is_finite()
        || !trace.is_finite()
        || !determinant_value.is_finite()
        || !frobenius_norm.is_finite()
        || !normalized_determinant.is_finite()
    {
        return Err("derived tensor metric is non-finite; check input scale".to_owned());
    }

    Ok((
        symmetric,
        TensorMetrics {
            symmetry_relative_residual,
            trace,
            determinant: determinant_value,
            frobenius_norm,
            tensor_scale: scale,
            normalized_min_diagonal_minor,
            normalized_min_two_by_two_principal_minor,
            normalized_determinant,
            positive_semidefinite,
        },
    ))
}

fn pressure_strain_metrics(
    pressure: &Matrix3,
    gradient: &Matrix3,
    identity_tolerance: f64,
) -> Result<PressureStrainMetrics, String> {
    let pressure_scalar = (pressure[0][0] + pressure[1][1] + pressure[2][2]) / 3.0;
    let velocity_divergence = gradient[0][0] + gradient[1][1] + gradient[2][2];

    let mut deviatoric_pressure = *pressure;
    let mut deviatoric_strain = [[0.0; 3]; 3];
    for (i, pressure_row) in deviatoric_pressure.iter_mut().enumerate() {
        pressure_row[i] -= pressure_scalar;
        for (j, strain_value) in deviatoric_strain[i].iter_mut().enumerate() {
            *strain_value = 0.5 * (gradient[i][j] + gradient[j][i]);
            if i == j {
                *strain_value -= velocity_divergence / 3.0;
            }
        }
    }

    let total_pressure_work = -contract_product(pressure, gradient);
    let pressure_dilation_work = -pressure_scalar * velocity_divergence;
    let deviatoric_pressure_strain_work =
        -contract_product(&deviatoric_pressure, &deviatoric_strain);
    let decomposed = pressure_dilation_work + deviatoric_pressure_strain_work;
    let energy_identity_absolute_residual = (total_pressure_work - decomposed).abs();
    let denominator = total_pressure_work
        .abs()
        .max(pressure_dilation_work.abs() + deviatoric_pressure_strain_work.abs())
        .max(f64::MIN_POSITIVE);
    let energy_identity_relative_residual = energy_identity_absolute_residual / denominator;

    let values = [
        pressure_scalar,
        velocity_divergence,
        total_pressure_work,
        pressure_dilation_work,
        deviatoric_pressure_strain_work,
        energy_identity_absolute_residual,
        energy_identity_relative_residual,
    ];
    if values.iter().any(|value| !value.is_finite())
        || deviatoric_pressure.iter().flatten().any(|value| !value.is_finite())
        || deviatoric_strain.iter().flatten().any(|value| !value.is_finite())
    {
        return Err("derived pressure-strain metric is non-finite; check input scale".to_owned());
    }

    if energy_identity_relative_residual > identity_tolerance {
        return Err("pressure-work decomposition identity exceeds the declared tolerance".to_owned());
    }

    Ok(PressureStrainMetrics {
        pressure_scalar,
        velocity_divergence,
        deviatoric_strain,
        deviatoric_pressure,
        total_pressure_work,
        pressure_dilation_work,
        deviatoric_pressure_strain_work,
        energy_identity_absolute_residual,
        energy_identity_relative_residual,
    })
}

/// Audit a supplied pressure tensor and velocity gradient.
///
/// The tensor must be symmetric and positive semidefinite within explicit,
/// scale-independent tolerances. Inputs are never modified. A tensor whose
/// asymmetry is below tolerance is averaged with its transpose only for
/// diagnostic contractions; the original asymmetry is reported.
pub fn audit_pressure_tensor(request: &AuditRequest) -> Result<AuditReport, String> {
    if request.schema_version != SCHEMA_VERSION {
        return Err("unsupported schema version".to_owned());
    }
    for (name, value) in [
        ("request_id", request.request_id.as_str()),
        ("source_id", request.source_id.as_str()),
        ("coordinate_frame_id", request.coordinate_frame_id.as_str()),
        ("pressure_units", request.pressure_units.as_str()),
        ("gradient_units", request.gradient_units.as_str()),
    ] {
        if !non_empty(value) {
            return Err(format!("{name} must be non-empty"));
        }
    }
    validate_tolerances(&request.tolerances)?;

    if !finite_matrix(&request.pressure_tensor) || !finite_matrix(&request.velocity_gradient) {
        return Err("pressure tensor and velocity gradient must contain only finite values".to_owned());
    }

    let (symmetric_pressure, tensor) = tensor_metrics(
        &request.pressure_tensor,
        request.tolerances.symmetry_relative,
        request.tolerances.psd_relative,
    )?;
    let pressure_strain = pressure_strain_metrics(
        &symmetric_pressure,
        &request.velocity_gradient,
        request.tolerances.energy_identity_relative,
    )?;

    let mut findings = Vec::new();
    let mut rejected = false;
    if tensor.symmetry_relative_residual > request.tolerances.symmetry_relative {
        findings.push(AuditFinding {
            code: "pressure_tensor_not_symmetric".to_owned(),
            detail: "The relative P-P^T residual exceeds the declared tolerance.".to_owned(),
        });
        rejected = true;
    }
    if !tensor.positive_semidefinite {
        findings.push(AuditFinding {
            code: "pressure_tensor_not_positive_semidefinite".to_owned(),
            detail: "At least one scale-normalized principal minor is below the declared PSD tolerance.".to_owned(),
        });
        rejected = true;
    }

    let request_bytes = serde_json::to_vec(request)
        .map_err(|error| format!("request serialization failed: {error}"))?;
    let request_sha256 = sha256_hex(&request_bytes);
    let body = AuditReportBody {
        schema_version: SCHEMA_VERSION.to_owned(),
        request_id: request.request_id.clone(),
        source_id: request.source_id.clone(),
        coordinate_frame_id: request.coordinate_frame_id.clone(),
        pressure_units: request.pressure_units.clone(),
        gradient_units: request.gradient_units.clone(),
        request_sha256,
        disposition: if rejected {
            AuditDisposition::Rejected
        } else {
            AuditDisposition::Admissible
        },
        tensor,
        pressure_strain,
        findings,
    };
    let receipt_bytes = serde_json::to_vec(&body)
        .map_err(|error| format!("report serialization failed: {error}"))?;

    Ok(AuditReport {
        body,
        receipt_sha256: sha256_hex(&receipt_bytes),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tolerances() -> AuditTolerances {
        AuditTolerances {
            symmetry_relative: 1.0e-12,
            psd_relative: 1.0e-12,
            energy_identity_relative: 1.0e-12,
        }
    }

    fn request(pressure_tensor: Matrix3, velocity_gradient: Matrix3) -> AuditRequest {
        AuditRequest {
            schema_version: SCHEMA_VERSION.to_owned(),
            request_id: "known-answer-1".to_owned(),
            source_id: "synthetic-fixture".to_owned(),
            coordinate_frame_id: "lab-cartesian-xyz".to_owned(),
            pressure_units: "normalized_pressure".to_owned(),
            gradient_units: "inverse_normalized_time".to_owned(),
            pressure_tensor,
            velocity_gradient,
            tolerances: tolerances(),
        }
    }

    fn diagonal(values: [f64; 3]) -> Matrix3 {
        [
            [values[0], 0.0, 0.0],
            [0.0, values[1], 0.0],
            [0.0, 0.0, values[2]],
        ]
    }

    #[test]
    fn isotropic_pressure_and_expansion_have_only_dilation_work() {
        let result = audit_pressure_tensor(&request(
            diagonal([2.0, 2.0, 2.0]),
            diagonal([1.0, 1.0, 1.0]),
        ))
        .expect("audit succeeds");

        assert_eq!(result.body.disposition, AuditDisposition::Admissible);
        assert!((result.body.tensor.trace - 6.0).abs() < 1.0e-12);
        assert!((result.body.pressure_strain.pressure_scalar - 2.0).abs() < 1.0e-12);
        assert!((result.body.pressure_strain.total_pressure_work + 6.0).abs() < 1.0e-12);
        assert!((result.body.pressure_strain.pressure_dilation_work + 6.0).abs() < 1.0e-12);
        assert!(result.body.pressure_strain.deviatoric_pressure_strain_work.abs() < 1.0e-12);
    }

    #[test]
    fn anisotropic_pressure_and_trace_free_shear_match_pressure_strain_identity() {
        let result = audit_pressure_tensor(&request(
            diagonal([2.0, 1.0, 1.0]),
            diagonal([1.0, -1.0, 0.0]),
        ))
        .expect("audit succeeds");

        assert_eq!(result.body.disposition, AuditDisposition::Admissible);
        assert!(result.body.pressure_strain.velocity_divergence.abs() < 1.0e-12);
        assert!((result.body.pressure_strain.total_pressure_work + 1.0).abs() < 1.0e-12);
        assert!((result.body.pressure_strain.deviatoric_pressure_strain_work + 1.0).abs() < 1.0e-12);
        assert!(result.body.pressure_strain.energy_identity_relative_residual < 1.0e-12);
    }

    #[test]
    fn rank_deficient_positive_semidefinite_tensor_is_admissible() {
        let result = audit_pressure_tensor(&request(
            diagonal([1.0, 1.0, 0.0]),
            [[0.0; 3]; 3],
        ))
        .expect("audit succeeds");
        assert_eq!(result.body.disposition, AuditDisposition::Admissible);
        assert!(result.body.tensor.normalized_determinant.abs() < 1.0e-12);
    }

    #[test]
    fn zero_pressure_tensor_is_admissible() {
        let result = audit_pressure_tensor(&request(
            [[0.0; 3]; 3],
            [[0.0; 3]; 3],
        ))
        .expect("audit succeeds");
        assert_eq!(result.body.disposition, AuditDisposition::Admissible);
    }

    #[test]
    fn indefinite_pressure_tensor_is_rejected() {
        let result = audit_pressure_tensor(&request(
            diagonal([1.0, 1.0, -0.1]),
            [[0.0; 3]; 3],
        ))
        .expect("audit produces a disposition");
        assert_eq!(result.body.disposition, AuditDisposition::Rejected);
        assert!(result.body.findings.iter().any(|finding| {
            finding.code == "pressure_tensor_not_positive_semidefinite"
        }));
    }

    #[test]
    fn materially_asymmetric_pressure_tensor_is_rejected() {
        let result = audit_pressure_tensor(&request(
            [[1.0, 0.1, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            [[0.0; 3]; 3],
        ))
        .expect("audit produces a disposition");
        assert_eq!(result.body.disposition, AuditDisposition::Rejected);
        assert!(result.body.tensor.symmetry_relative_residual > 0.0);
    }

    #[test]
    fn near_psd_negative_minor_is_allowed_only_within_declared_tolerance() {
        let mut req = request(diagonal([1.0, 1.0, -1.0e-13]), [[0.0; 3]; 3]);
        req.tolerances.psd_relative = 1.0e-12;
        let allowed = audit_pressure_tensor(&req).expect("audit succeeds");
        assert_eq!(allowed.body.disposition, AuditDisposition::Admissible);

        req.tolerances.psd_relative = 1.0e-14;
        let rejected = audit_pressure_tensor(&req).expect("audit produces a disposition");
        assert_eq!(rejected.body.disposition, AuditDisposition::Rejected);
    }

    #[test]
    fn rotation_preserves_tensor_invariants_and_psd_decision() {
        let angle = std::f64::consts::FRAC_PI_2;
        let rotation = [
            [angle.cos(), -angle.sin(), 0.0],
            [angle.sin(), angle.cos(), 0.0],
            [0.0, 0.0, 1.0],
        ];
        let pressure = [[2.0, 0.2, 0.1], [0.2, 1.5, 0.05], [0.1, 0.05, 1.0]];
        let mut rotated = [[0.0; 3]; 3];
        for (i, row) in rotated.iter_mut().enumerate() {
            for (j, value) in row.iter_mut().enumerate() {
                for (k, left) in rotation[i].iter().enumerate() {
                    for (l, pressure_row) in pressure.iter().enumerate() {
                        *value += left * pressure_row[l] * rotation[j][l];
                    }
                }
            }
        }

        let original = audit_pressure_tensor(&request(pressure, [[0.0; 3]; 3])).expect("original");
        let transformed = audit_pressure_tensor(&request(rotated, [[0.0; 3]; 3])).expect("rotated");
        assert_eq!(original.body.disposition, AuditDisposition::Admissible);
        assert_eq!(transformed.body.disposition, AuditDisposition::Admissible);
        assert!((original.body.tensor.trace - transformed.body.tensor.trace).abs() < 1.0e-12);
        assert!((original.body.tensor.determinant - transformed.body.tensor.determinant).abs() < 1.0e-12);
        assert!((original.body.tensor.frobenius_norm - transformed.body.tensor.frobenius_norm).abs() < 1.0e-12);
    }

    #[test]
    fn pressure_work_decomposition_is_rotation_invariant() {
        let pressure = [
            [2.0, 0.4, -0.1],
            [0.4, 1.5, 0.2],
            [-0.1, 0.2, 0.8],
        ];
        let gradient = [
            [0.3, 0.7, -0.2],
            [-0.1, -0.4, 0.5],
            [0.6, 0.2, 0.1],
        ];
        let angle = 0.37_f64;
        let rotation = [
            [angle.cos(), -angle.sin(), 0.0],
            [angle.sin(), angle.cos(), 0.0],
            [0.0, 0.0, 1.0],
        ];
        let transform = |matrix: Matrix3| {
            let mut left = [[0.0; 3]; 3];
            for (i, row) in left.iter_mut().enumerate() {
                for (j, value) in row.iter_mut().enumerate() {
                    for (k, r_ik) in rotation[i].iter().enumerate() {
                        *value += r_ik * matrix[k][j];
                    }
                }
            }
            let mut output = [[0.0; 3]; 3];
            for (i, row) in output.iter_mut().enumerate() {
                for (j, value) in row.iter_mut().enumerate() {
                    for (k, left_ik) in left[i].iter().enumerate() {
                        *value += left_ik * rotation[j][k];
                    }
                }
            }
            output
        };

        let original = audit_pressure_tensor(&request(pressure, gradient)).expect("original");
        let transformed = audit_pressure_tensor(&request(
            transform(pressure),
            transform(gradient),
        )).expect("rotated");

        let a = &original.body.pressure_strain;
        let b = &transformed.body.pressure_strain;
        assert!((a.total_pressure_work - b.total_pressure_work).abs() < 1.0e-11);
        assert!((a.pressure_dilation_work - b.pressure_dilation_work).abs() < 1.0e-11);
        assert!((a.deviatoric_pressure_strain_work - b.deviatoric_pressure_strain_work).abs() < 1.0e-11);
        assert!((a.energy_identity_relative_residual - b.energy_identity_relative_residual).abs() < 1.0e-11);
    }

    #[test]
    fn antisymmetric_velocity_gradient_does_no_symmetric_pressure_work() {
        let pressure = [
            [2.0, 0.5, -0.2],
            [0.5, 1.0, 0.3],
            [-0.2, 0.3, 0.8],
        ];
        let antisymmetric_gradient = [
            [0.0, -2.0, 0.4],
            [2.0, 0.0, -0.7],
            [-0.4, 0.7, 0.0],
        ];
        let result = audit_pressure_tensor(&request(pressure, antisymmetric_gradient))
            .expect("audit succeeds");
        assert_eq!(result.body.disposition, AuditDisposition::Admissible);
        assert!(result.body.pressure_strain.velocity_divergence.abs() < 1.0e-12);
        assert!(result.body.pressure_strain.total_pressure_work.abs() < 1.0e-12);
        assert!(result.body.pressure_strain.pressure_dilation_work.abs() < 1.0e-12);
        assert!(result.body.pressure_strain.deviatoric_pressure_strain_work.abs() < 1.0e-12);
    }

    #[test]
    fn non_finite_velocity_gradient_is_rejected() {
        let result = audit_pressure_tensor(&request(
            diagonal([1.0, 1.0, 1.0]),
            [[f64::INFINITY, 0.0, 0.0], [0.0; 3], [0.0; 3]],
        ));
        assert!(result.is_err());
    }

    #[test]
    fn negative_tolerances_are_rejected() {
        let mut req = request(diagonal([1.0, 1.0, 1.0]), [[0.0; 3]; 3]);
        req.tolerances.psd_relative = -1.0e-6;
        assert!(audit_pressure_tensor(&req).is_err());
    }

    #[test]
    fn non_finite_input_is_rejected() {
        let result = audit_pressure_tensor(&request(
            [[f64::NAN, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            [[0.0; 3]; 3],
        ));
        assert!(result.is_err());
    }

    #[test]
    fn off_diagonal_pressure_and_strain_terms_are_contracted_without_losing_tensor_structure() {
        let pressure = [
            [2.0, 0.5, 0.0],
            [0.5, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ];
        let gradient = [
            [1.0, 2.0, 0.0],
            [0.0, -1.0, 0.0],
            [0.0, 0.0, 0.0],
        ];
        let result = audit_pressure_tensor(&request(pressure, gradient)).expect("audit succeeds");
        assert_eq!(result.body.disposition, AuditDisposition::Admissible);
        assert!((result.body.pressure_strain.total_pressure_work + 2.0).abs() < 1.0e-12);
        assert!((result.body.pressure_strain.deviatoric_pressure_strain_work + 2.0).abs() < 1.0e-12);
    }

    #[test]
    fn psd_decision_is_invariant_to_pressure_unit_scale() {
        let pressure = diagonal([2.0, 1.0, 0.5]);
        let unit_scale = 1.0e18;
        let scaled = pressure.map(|row| row.map(|value| value * unit_scale));
        let base = audit_pressure_tensor(&request(pressure, [[0.0; 3]; 3])).expect("base");
        let high = audit_pressure_tensor(&request(scaled, [[0.0; 3]; 3])).expect("scaled");
        assert_eq!(base.body.disposition, AuditDisposition::Admissible);
        assert_eq!(high.body.disposition, AuditDisposition::Admissible);
        assert_eq!(
            base.body.tensor.normalized_min_two_by_two_principal_minor,
            high.body.tensor.normalized_min_two_by_two_principal_minor
        );
    }

    #[test]
    fn deterministic_receipt_for_identical_request() {
        let req = request(diagonal([2.0, 1.0, 1.0]), diagonal([1.0, -1.0, 0.0]));
        let a = audit_pressure_tensor(&req).expect("first");
        let b = audit_pressure_tensor(&req).expect("second");
        assert_eq!(a, b);
        assert_eq!(a.receipt_sha256, b.receipt_sha256);
    }

    #[test]
    fn unknown_schema_version_and_empty_unit_identifiers_are_rejected() {
        let mut req = request(diagonal([1.0, 1.0, 1.0]), [[0.0; 3]; 3]);
        req.schema_version = "unknown".to_owned();
        assert!(audit_pressure_tensor(&req).is_err());

        req.schema_version = SCHEMA_VERSION.to_owned();
        req.pressure_units.clear();
        assert!(audit_pressure_tensor(&req).is_err());
    }
}
