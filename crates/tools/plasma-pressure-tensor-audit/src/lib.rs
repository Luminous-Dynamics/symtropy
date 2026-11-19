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


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PressureGridRequest {
    pub schema_version: String,
    pub grid_id: String,
    pub coordinate_frame_id: String,
    pub pressure_units: String,
    pub nx: usize,
    pub ny: usize,
    pub dx: f64,
    pub dy: f64,
    /// Cell-major row order: index = y * nx + x.
    pub pressure_tensors: Vec<Matrix3>,
    pub tolerances: AuditTolerances,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PressureGridDisposition {
    Admissible,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PressureGridFinding {
    pub cell_index: usize,
    pub code: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PressureGridReportBody {
    pub schema_version: String,
    pub grid_id: String,
    pub coordinate_frame_id: String,
    pub pressure_units: String,
    pub nx: usize,
    pub ny: usize,
    pub dx: f64,
    pub dy: f64,
    pub request_sha256: String,
    pub disposition: PressureGridDisposition,
    pub maximum_symmetry_relative_residual: f64,
    pub minimum_normalized_principal_minor: f64,
    pub all_cells_positive_semidefinite: bool,
    pub divergence: Vec<[f64; 3]>,
    pub global_integrated_divergence: [f64; 3],
    pub maximum_absolute_divergence: f64,
    pub global_conservation_residual_l2: f64,
    pub findings: Vec<PressureGridFinding>,
    pub operator_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PressureGridReport {
    pub body: PressureGridReportBody,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PressureDivergenceComponentError {
    pub mae: f64,
    pub rmse: f64,
    pub relative_l2: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PressureDivergenceComparisonBody {
    pub schema_version: String,
    pub grid_id: String,
    pub reference_request_sha256: String,
    pub predicted_request_sha256: String,
    pub reference_admissible: bool,
    pub predicted_admissible: bool,
    pub component_errors: [PressureDivergenceComponentError; 3],
    pub both_fields_admissible: bool,
    pub operator_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PressureDivergenceComparisonReport {
    pub body: PressureDivergenceComparisonBody,
    pub receipt_sha256: String,
}

const PERIODIC_FACE_FLUX_OPERATOR: &str =
    "periodic_uniform_cell_centered_shared_arithmetic_face_flux_v0.1";

fn pressure_grid_request_sha256(request: &PressureGridRequest) -> Result<String, String> {
    let bytes = serde_json::to_vec(request)
        .map_err(|error| format!("pressure grid request serialization failed: {error}"))?;
    Ok(sha256_hex(&bytes))
}

/// Compute div(P)_i = d_x P_ix + d_y P_iy on a uniform periodic grid.
///
/// Every shared face uses one arithmetic-average flux. The global integral
/// therefore telescopes up to floating-point summation error. This is a
/// synthetic reference; solver adapters must bind their actual stencil.
pub fn audit_periodic_pressure_grid(
    request: &PressureGridRequest,
) -> Result<PressureGridReport, String> {
    if request.schema_version != SCHEMA_VERSION {
        return Err("unsupported pressure-grid schema version".to_owned());
    }
    for (name, value) in [
        ("grid_id", request.grid_id.as_str()),
        ("coordinate_frame_id", request.coordinate_frame_id.as_str()),
        ("pressure_units", request.pressure_units.as_str()),
    ] {
        if !non_empty(value) {
            return Err(format!("{name} must be non-empty"));
        }
    }
    validate_tolerances(&request.tolerances)?;
    if request.nx < 2 || request.ny < 2 {
        return Err("periodic pressure grid requires nx >= 2 and ny >= 2".to_owned());
    }
    if !request.dx.is_finite()
        || !request.dy.is_finite()
        || request.dx <= 0.0
        || request.dy <= 0.0
    {
        return Err("grid spacing must be finite and strictly positive".to_owned());
    }
    let expected_cells = request
        .nx
        .checked_mul(request.ny)
        .ok_or_else(|| "pressure-grid cell count overflow".to_owned())?;
    if request.pressure_tensors.len() != expected_cells {
        return Err(format!(
            "pressure tensor count {} does not equal nx*ny ({expected_cells})",
            request.pressure_tensors.len()
        ));
    }

    let request_sha256 = pressure_grid_request_sha256(request)?;
    let mut symmetric_pressure = Vec::with_capacity(expected_cells);
    let mut findings = Vec::new();
    let mut maximum_symmetry_relative_residual = 0.0_f64;
    let mut minimum_normalized_principal_minor = f64::INFINITY;
    let mut all_cells_positive_semidefinite = true;

    for (cell_index, pressure) in request.pressure_tensors.iter().enumerate() {
        if !finite_matrix(pressure) {
            return Err(format!("pressure tensor at cell {cell_index} is non-finite"));
        }
        let (symmetric, metrics) = tensor_metrics(
            pressure,
            request.tolerances.symmetry_relative,
            request.tolerances.psd_relative,
        )?;
        maximum_symmetry_relative_residual =
            maximum_symmetry_relative_residual.max(metrics.symmetry_relative_residual);
        minimum_normalized_principal_minor = minimum_normalized_principal_minor
            .min(metrics.normalized_min_diagonal_minor)
            .min(metrics.normalized_min_two_by_two_principal_minor)
            .min(metrics.normalized_determinant);

        if metrics.symmetry_relative_residual > request.tolerances.symmetry_relative {
            findings.push(PressureGridFinding {
                cell_index,
                code: "pressure_tensor_not_symmetric".to_owned(),
                detail: "Cell pressure tensor exceeds the declared relative symmetry tolerance."
                    .to_owned(),
            });
        }
        if !metrics.positive_semidefinite {
            all_cells_positive_semidefinite = false;
            findings.push(PressureGridFinding {
                cell_index,
                code: "pressure_tensor_not_positive_semidefinite".to_owned(),
                detail: "Cell pressure tensor has a normalized principal minor below the PSD tolerance."
                    .to_owned(),
            });
        }
        symmetric_pressure.push(symmetric);
    }

    let mut divergence = vec![[0.0; 3]; expected_cells];
    let mut global_integrated_divergence = [0.0; 3];
    let cell_area = request.dx * request.dy;
    if !cell_area.is_finite() || cell_area <= 0.0 {
        return Err("cell area is non-finite or non-positive".to_owned());
    }
    let mut maximum_absolute_divergence = 0.0_f64;

    for y in 0..request.ny {
        let y_up = (y + 1) % request.ny;
        let y_down = (y + request.ny - 1) % request.ny;
        for x in 0..request.nx {
            let x_right = (x + 1) % request.nx;
            let x_left = (x + request.nx - 1) % request.nx;
            let index = y * request.nx + x;
            let right_index = y * request.nx + x_right;
            let left_index = y * request.nx + x_left;
            let up_index = y_up * request.nx + x;
            let down_index = y_down * request.nx + x;

            for component in 0..3 {
                let flux_x_right = 0.5
                    * (symmetric_pressure[index][component][0]
                        + symmetric_pressure[right_index][component][0]);
                let flux_x_left = 0.5
                    * (symmetric_pressure[left_index][component][0]
                        + symmetric_pressure[index][component][0]);
                let flux_y_up = 0.5
                    * (symmetric_pressure[index][component][1]
                        + symmetric_pressure[up_index][component][1]);
                let flux_y_down = 0.5
                    * (symmetric_pressure[down_index][component][1]
                        + symmetric_pressure[index][component][1]);
                let value = (flux_x_right - flux_x_left) / request.dx
                    + (flux_y_up - flux_y_down) / request.dy;
                if !value.is_finite() {
                    return Err(format!("non-finite pressure divergence at cell {index}"));
                }
                divergence[index][component] = value;
                maximum_absolute_divergence = maximum_absolute_divergence.max(value.abs());
                global_integrated_divergence[component] += value * cell_area;
            }
        }
    }

    let global_conservation_residual_l2 = global_integrated_divergence
        .iter()
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt();
    if !global_conservation_residual_l2.is_finite() {
        return Err("global pressure-divergence conservation residual is non-finite".to_owned());
    }

    let rejected = findings.iter().any(|finding| {
        finding.code == "pressure_tensor_not_symmetric"
            || finding.code == "pressure_tensor_not_positive_semidefinite"
    });
    let body = PressureGridReportBody {
        schema_version: SCHEMA_VERSION.to_owned(),
        grid_id: request.grid_id.clone(),
        coordinate_frame_id: request.coordinate_frame_id.clone(),
        pressure_units: request.pressure_units.clone(),
        nx: request.nx,
        ny: request.ny,
        dx: request.dx,
        dy: request.dy,
        request_sha256,
        disposition: if rejected {
            PressureGridDisposition::Rejected
        } else {
            PressureGridDisposition::Admissible
        },
        maximum_symmetry_relative_residual,
        minimum_normalized_principal_minor,
        all_cells_positive_semidefinite,
        divergence,
        global_integrated_divergence,
        maximum_absolute_divergence,
        global_conservation_residual_l2,
        findings,
        operator_id: PERIODIC_FACE_FLUX_OPERATOR.to_owned(),
    };
    let receipt_bytes = serde_json::to_vec(&body)
        .map_err(|error| format!("pressure grid report serialization failed: {error}"))?;
    Ok(PressureGridReport {
        body,
        receipt_sha256: sha256_hex(&receipt_bytes),
    })
}

pub fn compare_periodic_pressure_divergence(
    reference: &PressureGridRequest,
    predicted: &PressureGridRequest,
) -> Result<PressureDivergenceComparisonReport, String> {
    if reference.grid_id != predicted.grid_id
        || reference.coordinate_frame_id != predicted.coordinate_frame_id
        || reference.pressure_units != predicted.pressure_units
        || reference.nx != predicted.nx
        || reference.ny != predicted.ny
        || reference.dx.to_bits() != predicted.dx.to_bits()
        || reference.dy.to_bits() != predicted.dy.to_bits()
    {
        return Err("reference and predicted fields do not share the exact grid/frame/unit contract"
            .to_owned());
    }

    let reference_report = audit_periodic_pressure_grid(reference)?;
    let predicted_report = audit_periodic_pressure_grid(predicted)?;
    let mut component_errors = Vec::with_capacity(3);

    for component in 0..3 {
        let mut absolute_error_sum = 0.0;
        let mut squared_error_sum = 0.0;
        let mut reference_squared_sum = 0.0;
        for cell in 0..reference_report.body.divergence.len() {
            let expected = reference_report.body.divergence[cell][component];
            let actual = predicted_report.body.divergence[cell][component];
            let error = actual - expected;
            absolute_error_sum += error.abs();
            squared_error_sum += error * error;
            reference_squared_sum += expected * expected;
        }
        let cell_count = reference_report.body.divergence.len() as f64;
        let mae = absolute_error_sum / cell_count;
        let rmse = (squared_error_sum / cell_count).sqrt();
        let relative_l2 = if reference_squared_sum > f64::MIN_POSITIVE {
            Some((squared_error_sum / reference_squared_sum).sqrt())
        } else {
            None
        };
        if !mae.is_finite()
            || !rmse.is_finite()
            || relative_l2.is_some_and(|value| !value.is_finite())
        {
            return Err("non-finite pressure-divergence comparison metric".to_owned());
        }
        component_errors.push(PressureDivergenceComponentError {
            mae,
            rmse,
            relative_l2,
        });
    }

    let component_errors: [PressureDivergenceComponentError; 3] = component_errors
        .try_into()
        .map_err(|_| "internal pressure-divergence component count mismatch".to_owned())?;
    let reference_admissible =
        reference_report.body.disposition == PressureGridDisposition::Admissible;
    let predicted_admissible =
        predicted_report.body.disposition == PressureGridDisposition::Admissible;
    let body = PressureDivergenceComparisonBody {
        schema_version: SCHEMA_VERSION.to_owned(),
        grid_id: reference.grid_id.clone(),
        reference_request_sha256: reference_report.body.request_sha256.clone(),
        predicted_request_sha256: predicted_report.body.request_sha256.clone(),
        reference_admissible,
        predicted_admissible,
        component_errors,
        both_fields_admissible: reference_admissible && predicted_admissible,
        operator_id: PERIODIC_FACE_FLUX_OPERATOR.to_owned(),
    };
    let receipt_bytes = serde_json::to_vec(&body)
        .map_err(|error| format!("pressure divergence comparison serialization failed: {error}"))?;
    Ok(PressureDivergenceComparisonReport {
        body,
        receipt_sha256: sha256_hex(&receipt_bytes),
    })
}

pub fn verify_pressure_grid_report(
    request: &PressureGridRequest,
    report: &PressureGridReport,
) -> Result<(), String> {
    let expected = audit_periodic_pressure_grid(request)?;
    if expected != *report {
        return Err("replayed pressure-grid report differs from the supplied report".to_owned());
    }
    Ok(())
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
    fn grid_request(values: &[f64]) -> PressureGridRequest {
        let pressure_tensors = values
            .iter()
            .map(|pxx| diagonal([*pxx, 1.0, 1.0]))
            .collect::<Vec<_>>();
        PressureGridRequest {
            schema_version: SCHEMA_VERSION.to_owned(),
            grid_id: "periodic-grid-4x2".to_owned(),
            coordinate_frame_id: "cartesian-xyz".to_owned(),
            pressure_units: "normalized_pressure".to_owned(),
            nx: 4,
            ny: 2,
            dx: 1.0,
            dy: 2.0,
            pressure_tensors,
            tolerances: tolerances(),
        }
    }

    #[test]
    fn constant_periodic_pressure_has_zero_divergence() {
        let request = grid_request(&[2.0; 8]);
        let report = audit_periodic_pressure_grid(&request).expect("grid audit");
        assert_eq!(report.body.disposition, PressureGridDisposition::Admissible);
        assert!(report.body.divergence.iter().flatten().all(|value| value.abs() < 1.0e-12));
        assert!(report.body.global_conservation_residual_l2 < 1.0e-12);
    }

    #[test]
    fn periodic_face_flux_operator_matches_known_pxx_divergence() {
        let request = grid_request(&[
            0.0, 2.0, 4.0, 2.0,
            0.0, 2.0, 4.0, 2.0,
        ]);
        let report = audit_periodic_pressure_grid(&request).expect("grid audit");
        assert_eq!(report.body.disposition, PressureGridDisposition::Admissible);
        for y in 0..2 {
            let base = y * 4;
            assert!(report.body.divergence[base][0].abs() < 1.0e-12);
            assert!((report.body.divergence[base + 1][0] - 2.0).abs() < 1.0e-12);
            assert!(report.body.divergence[base + 2][0].abs() < 1.0e-12);
            assert!((report.body.divergence[base + 3][0] + 2.0).abs() < 1.0e-12);
        }
        assert!(report.body.global_conservation_residual_l2 < 1.0e-12);
    }

    #[test]
    fn arbitrary_periodic_pressure_field_has_telescoping_global_divergence() {
        let values = [
            1.0, 1.2, 0.8, 1.1,
            0.9, 1.4, 1.3, 0.7,
        ];
        let mut request = grid_request(&values);
        for (index, pressure) in request.pressure_tensors.iter_mut().enumerate() {
            pressure[0][1] = 0.03 * index as f64;
            pressure[1][0] = 0.03 * index as f64;
            pressure[1][1] = 1.5 + 0.1 * index as f64;
        }
        let report = audit_periodic_pressure_grid(&request).expect("grid audit");
        assert!(report.body.global_conservation_residual_l2 < 1.0e-11);
    }

    #[test]
    fn cellwise_indefinite_tensor_rejects_grid_admissibility() {
        let mut request = grid_request(&[1.0; 8]);
        request.pressure_tensors[3] = diagonal([1.0, 1.0, -0.5]);
        let report = audit_periodic_pressure_grid(&request).expect("grid audit");
        assert_eq!(report.body.disposition, PressureGridDisposition::Rejected);
        assert!(!report.body.all_cells_positive_semidefinite);
        assert!(report.body.findings.iter().any(|finding| {
            finding.cell_index == 3
                && finding.code == "pressure_tensor_not_positive_semidefinite"
        }));
    }

    #[test]
    fn invalid_grid_shape_spacing_and_non_finite_cell_are_refused() {
        let mut request = grid_request(&[1.0; 8]);
        request.nx = 3;
        assert!(audit_periodic_pressure_grid(&request).is_err());

        let mut request = grid_request(&[1.0; 8]);
        request.dx = 0.0;
        assert!(audit_periodic_pressure_grid(&request).is_err());

        let mut request = grid_request(&[1.0; 8]);
        request.pressure_tensors[0][0][0] = f64::NAN;
        assert!(audit_periodic_pressure_grid(&request).is_err());
    }

    #[test]
    fn pressure_divergence_comparison_is_operator_level() {
        let reference = grid_request(&[0.0, 2.0, 4.0, 2.0, 0.0, 2.0, 4.0, 2.0]);
        let mut predicted = reference.clone();
        for pressure in &mut predicted.pressure_tensors {
            pressure[0][0] += 0.25;
        }
        let report =
            compare_periodic_pressure_divergence(&reference, &predicted).expect("compare");
        assert!(report.body.both_fields_admissible);
        assert!(report.body.component_errors.iter().all(|error| error.rmse < 1.0e-12));
    }

    #[test]
    fn pressure_divergence_comparison_refuses_grid_contract_mismatch() {
        let reference = grid_request(&[1.0; 8]);
        let mut predicted = reference.clone();
        predicted.dy = 3.0;
        assert!(compare_periodic_pressure_divergence(&reference, &predicted).is_err());
    }

    #[test]
    fn pressure_divergence_comparison_detects_off_diagonal_flux_error() {
        let reference = grid_request(&[1.0; 8]);
        let mut predicted = reference.clone();
        for y in 0..2 {
            for x in 0..4 {
                let value = if (x + y) % 2 == 0 { 0.0 } else { 0.2 };
                let cell = y * 4 + x;
                predicted.pressure_tensors[cell][0][1] = value;
                predicted.pressure_tensors[cell][1][0] = value;
            }
        }
        let report =
            compare_periodic_pressure_divergence(&reference, &predicted).expect("compare");
        assert!(report.body.component_errors[0].rmse > 0.0);
        assert!(report.body.component_errors[1].rmse > 0.0);
    }

    #[test]
    fn pressure_grid_receipt_replays_and_rejects_rehashed_tampering() {
        let request = grid_request(&[0.0, 2.0, 4.0, 2.0, 0.0, 2.0, 4.0, 2.0]);
        let mut report = audit_periodic_pressure_grid(&request).expect("grid audit");
        verify_pressure_grid_report(&request, &report).expect("valid receipt");

        report.body.maximum_absolute_divergence = 999.0;
        let bytes = serde_json::to_vec(&report.body).expect("serialize report body");
        report.receipt_sha256 = sha256_hex(&bytes);
        assert!(verify_pressure_grid_report(&request, &report).is_err());
    }

}
