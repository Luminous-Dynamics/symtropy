// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Covariance-aware response comparison. This is a diagnostic metric layer,
// not a physical solver and not a claim of compliance with ASME VVUQ 20.1.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const SYMMETRY_TOL: f64 = 1.0e-12;
const PSD_TOL: f64 = 1.0e-12;
const MIN_RELATIVE_PIVOT: f64 = 1.0e-12;
const MAX_CONDITION_INFINITY: f64 = 1.0e12;

type Matrix = Vec<Vec<f64>>;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StateIdentity {
    pub configuration_sha256: String,
    pub geometry_sha256: String,
    pub mesh_sha256: String,
    pub loading_state_sha256: String,
    pub environment_sha256: String,
    pub solver_build_sha256: String,
    pub claim_scope_sha256: String,
    pub mission_phase_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResponseChannel {
    pub id: String,
    pub quantity: String,
    pub units: String,
    pub coordinate_frame: String,
    pub sample_id: String,
    pub normalization_scale: f64,
    pub reference_value: f64,
    pub simulation_value: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CovarianceBlock {
    pub channel_ids: Vec<String>,
    pub matrix: Matrix,
    pub source_id: String,
    pub artifact_sha256: String,
    pub estimator_build_sha256: String,
    pub calibration_dataset_sha256: String,
    pub calibration_envelope_sha256: String,
    pub state_identity_sha256: String,
    pub normalization_basis_sha256: String,
    pub max_diagonal_variance: Option<Vec<f64>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UncertaintyContract {
    pub independent_components: bool,
    pub experimental: CovarianceBlock,
    pub numerical: CovarianceBlock,
    pub input: CovarianceBlock,
    pub cross_source: Option<CovarianceBlock>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionLimit {
    pub max_d2: f64,
    pub max_abs_marginal_z: f64,
    pub protocol_sha256: String,
    pub claim_scope_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResponseVectorRequest {
    pub schema_version: String,
    pub response_family_id: String,
    pub identity: StateIdentity,
    pub held_out_validation_dataset_sha256: String,
    pub channels: Vec<ResponseChannel>,
    pub uncertainty: UncertaintyContract,
    pub decision_limit: Option<DecisionLimit>,
    pub model_discrepancy_evidence_sha256: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ChannelDiagnostic {
    pub id: String,
    pub quantity: String,
    pub units: String,
    pub coordinate_frame: String,
    pub sample_id: String,
    pub reference_value: f64,
    pub simulation_value: f64,
    pub physical_residual: f64,
    pub normalized_residual: f64,
    pub combined_standard_deviation: f64,
    pub marginal_standardized_residual: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct UncertaintySourceReceipt {
    pub source_id: String,
    pub artifact_sha256: String,
    pub estimator_build_sha256: String,
    pub calibration_dataset_sha256: String,
    pub calibration_envelope_sha256: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ResponseVectorResult {
    pub schema_version: String,
    pub response_family_id: String,
    pub state_identity_sha256: String,
    pub normalization_basis_sha256: String,
    pub held_out_validation_dataset_sha256: String,
    pub channel_diagnostics: Vec<ChannelDiagnostic>,
    pub covariance_weighted_d2: f64,
    pub covariance_condition_infinity_estimate: f64,
    pub minimum_cholesky_pivot: f64,
    pub maximum_cholesky_pivot: f64,
    pub maximum_absolute_marginal_z: f64,
    pub uncertainty_sources: Vec<UncertaintySourceReceipt>,
    pub cross_source_covariance_source_id: Option<String>,
    pub decision_status: String,
    pub decision_limit_d2: Option<f64>,
    pub decision_limit_max_abs_marginal_z: Option<f64>,
    pub decision_protocol_sha256: Option<String>,
    pub model_discrepancy_evidence_sha256: Option<String>,
    pub claim_ceiling: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VvuqError(pub String);

impl std::fmt::Display for VvuqError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for VvuqError {}

type Result<T> = std::result::Result<T, VvuqError>;

#[derive(Serialize)]
struct ChannelBasis<'a> {
    id: &'a str,
    quantity: &'a str,
    units: &'a str,
    coordinate_frame: &'a str,
    sample_id: &'a str,
    normalization_scale: f64,
}

fn error<T>(message: impl Into<String>) -> Result<T> {
    Err(VvuqError(message.into()))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn digest_json<T: Serialize>(value: &T) -> Result<String> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| VvuqError(format!("canonical serialization failed: {error}")))?;
    Ok(sha256_hex(&bytes))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn require_sha256(value: &str, label: &str) -> Result<()> {
    if !is_sha256(value) {
        return error(format!("{label}: expected lowercase 64-character SHA-256"));
    }
    Ok(())
}

fn require_nonempty(value: &str, label: &str) -> Result<()> {
    if value.trim().is_empty() {
        return error(format!("{label}: must be non-empty"));
    }
    Ok(())
}

fn finite(value: f64, label: &str) -> Result<()> {
    if !value.is_finite() {
        return error(format!("{label}: non-finite value"));
    }
    Ok(())
}

fn validate_identity(identity: &StateIdentity) -> Result<String> {
    for (value, label) in [
        (&identity.configuration_sha256, "configuration digest"),
        (&identity.geometry_sha256, "geometry digest"),
        (&identity.mesh_sha256, "mesh digest"),
        (&identity.loading_state_sha256, "loading-state digest"),
        (&identity.environment_sha256, "environment digest"),
        (&identity.solver_build_sha256, "solver-build digest"),
        (&identity.claim_scope_sha256, "claim-scope digest"),
    ] {
        require_sha256(value, label)?;
    }
    require_nonempty(&identity.mission_phase_id, "mission phase ID")?;
    digest_json(identity)
}

fn canonical_channels(request: &ResponseVectorRequest) -> Result<Vec<ResponseChannel>> {
    if request.channels.is_empty() {
        return error("response vector has no channels");
    }
    let mut channels = request.channels.clone();
    channels.sort_by(|left, right| left.id.cmp(&right.id));

    let mut seen = BTreeSet::new();
    for channel in &channels {
        require_nonempty(&channel.id, "channel ID")?;
        require_nonempty(&channel.quantity, "channel quantity")?;
        require_nonempty(&channel.units, "channel units")?;
        require_nonempty(&channel.coordinate_frame, "channel coordinate frame")?;
        require_nonempty(&channel.sample_id, "channel sample ID")?;
        if !seen.insert(channel.id.clone()) {
            return error(format!("duplicate response channel ID {}", channel.id));
        }
        finite(
            channel.normalization_scale,
            &format!("{} normalization scale", channel.id),
        )?;
        if channel.normalization_scale <= 0.0 {
            return error(format!(
                "{} normalization scale must be positive",
                channel.id
            ));
        }
        finite(
            channel.reference_value,
            &format!("{} reference value", channel.id),
        )?;
        finite(
            channel.simulation_value,
            &format!("{} simulation value", channel.id),
        )?;
    }
    Ok(channels)
}

fn normalization_basis_hash(channels: &[ResponseChannel]) -> Result<String> {
    let basis = channels
        .iter()
        .map(|channel| ChannelBasis {
            id: &channel.id,
            quantity: &channel.quantity,
            units: &channel.units,
            coordinate_frame: &channel.coordinate_frame,
            sample_id: &channel.sample_id,
            normalization_scale: channel.normalization_scale,
        })
        .collect::<Vec<_>>();
    digest_json(&basis)
}

fn matrix_max_abs(matrix: &Matrix) -> f64 {
    matrix
        .iter()
        .flat_map(|row| row.iter())
        .map(|value| value.abs())
        .fold(0.0, f64::max)
}

fn validate_matrix_shape(matrix: &Matrix, n: usize, label: &str) -> Result<()> {
    if matrix.len() != n || matrix.iter().any(|row| row.len() != n) {
        return error(format!("{label}: covariance matrix must be {n}x{n}"));
    }
    for (row_index, row) in matrix.iter().enumerate() {
        for (column_index, value) in row.iter().enumerate() {
            finite(*value, &format!("{label}[{row_index}][{column_index}]"))?;
        }
    }
    Ok(())
}

fn validate_symmetric(matrix: &Matrix, label: &str) -> Result<()> {
    let scale = matrix_max_abs(matrix).max(1.0);
    for row in 0..matrix.len() {
        for column in 0..row {
            let difference = (matrix[row][column] - matrix[column][row]).abs();
            if difference > SYMMETRY_TOL * scale {
                return error(format!("{label}: asymmetric entries at ({row},{column})"));
            }
        }
    }
    Ok(())
}

fn reorder_covariance(
    block: &CovarianceBlock,
    canonical_ids: &[String],
    label: &str,
) -> Result<Matrix> {
    let n = canonical_ids.len();
    validate_matrix_shape(&block.matrix, n, label)?;
    if block.channel_ids.len() != n {
        return error(format!(
            "{label}: channel ID count does not match matrix dimension"
        ));
    }
    let mut positions = BTreeMap::new();
    for (index, channel_id) in block.channel_ids.iter().enumerate() {
        if positions.insert(channel_id.clone(), index).is_some() {
            return error(format!("{label}: duplicate matrix channel ID {channel_id}"));
        }
    }
    if positions.len() != n || canonical_ids.iter().any(|id| !positions.contains_key(id)) {
        return error(format!(
            "{label}: matrix channel set does not match response channel set"
        ));
    }

    let indices = canonical_ids
        .iter()
        .map(|id| positions[id])
        .collect::<Vec<_>>();
    let reordered = indices
        .iter()
        .map(|row| {
            indices
                .iter()
                .map(|column| block.matrix[*row][*column])
                .collect()
        })
        .collect::<Matrix>();
    validate_symmetric(&reordered, label)?;
    Ok(reordered)
}

fn psd_check(matrix: &Matrix, label: &str) -> Result<()> {
    let n = matrix.len();
    let scale = matrix_max_abs(matrix).max(1.0);
    let tolerance = PSD_TOL * scale;
    let mut lower = vec![vec![0.0; n]; n];

    for i in 0..n {
        for j in 0..=i {
            let mut residual = matrix[i][j];
            for k in 0..j {
                residual -= lower[i][k] * lower[j][k];
            }
            if i == j {
                if residual < -tolerance {
                    return error(format!(
                        "{label}: covariance component is not positive semidefinite"
                    ));
                }
                lower[i][j] = residual.max(0.0).sqrt();
            } else if lower[j][j] > tolerance.sqrt() {
                lower[i][j] = residual / lower[j][j];
            } else if residual.abs() > tolerance {
                return error(format!(
                    "{label}: covariance component has an inconsistent zero pivot"
                ));
            }
        }
    }
    Ok(())
}

fn validate_covariance_provenance(
    block: &CovarianceBlock,
    identity_sha256: &str,
    normalization_sha256: &str,
    validation_sha256: &str,
    label: &str,
    require_envelope: bool,
) -> Result<()> {
    require_nonempty(&block.source_id, &format!("{label} source ID"))?;
    for (value, suffix) in [
        (&block.artifact_sha256, "artifact"),
        (&block.estimator_build_sha256, "estimator build"),
        (&block.calibration_dataset_sha256, "calibration dataset"),
        (&block.calibration_envelope_sha256, "calibration envelope"),
        (&block.state_identity_sha256, "state identity"),
        (&block.normalization_basis_sha256, "normalization basis"),
    ] {
        require_sha256(value, &format!("{label} {suffix} digest"))?;
    }
    if block.calibration_dataset_sha256 == validation_sha256 {
        return error(format!(
            "{label}: calibration and held-out validation dataset digests overlap"
        ));
    }
    if block.state_identity_sha256 != identity_sha256 {
        return error(format!(
            "{label}: covariance source is bound to a different aircraft state"
        ));
    }
    if block.normalization_basis_sha256 != normalization_sha256 {
        return error(format!(
            "{label}: covariance source uses a different normalization basis"
        ));
    }

    if require_envelope {
        let Some(bounds) = &block.max_diagonal_variance else {
            return error(format!(
                "{label}: calibration diagonal-variance envelope is required"
            ));
        };
        if bounds.len() != block.channel_ids.len() {
            return error(format!(
                "{label}: calibration envelope dimension mismatch"
            ));
        }
        for (index, value) in bounds.iter().enumerate() {
            finite(*value, &format!("{label} variance envelope[{index}]"))?;
            if *value <= 0.0 {
                return error(format!("{label}: variance envelope must be positive"));
            }
        }
    } else if block.max_diagonal_variance.is_some() {
        return error(format!(
            "{label}: cross-source covariance must not use a marginal variance envelope"
        ));
    }
    Ok(())
}

fn validate_component_block(
    block: &CovarianceBlock,
    canonical_ids: &[String],
    identity_sha256: &str,
    normalization_sha256: &str,
    validation_sha256: &str,
    label: &str,
) -> Result<Matrix> {
    validate_covariance_provenance(
        block,
        identity_sha256,
        normalization_sha256,
        validation_sha256,
        label,
        true,
    )?;
    let matrix = reorder_covariance(block, canonical_ids, label)?;
    psd_check(&matrix, label)?;

    let bounds = block
        .max_diagonal_variance
        .as_ref()
        .expect("validated above");
    let positions = block
        .channel_ids
        .iter()
        .enumerate()
        .map(|(index, id)| (id.as_str(), index))
        .collect::<BTreeMap<_, _>>();
    for (index, channel_id) in canonical_ids.iter().enumerate() {
        let source_index = positions[channel_id.as_str()];
        let variance = matrix[index][index];
        let allowed = bounds[source_index];
        if variance > allowed * (1.0 + SYMMETRY_TOL) {
            return error(format!(
                "{label}: variance for channel {channel_id} exceeds its calibration envelope"
            ));
        }
    }
    Ok(matrix)
}

fn add_matrices(left: &Matrix, right: &Matrix) -> Matrix {
    left.iter()
        .zip(right)
        .map(|(left_row, right_row)| {
            left_row
                .iter()
                .zip(right_row)
                .map(|(left, right)| left + right)
                .collect()
        })
        .collect()
}

fn cholesky(matrix: &Matrix) -> Result<(Matrix, f64, f64)> {
    let n = matrix.len();
    validate_symmetric(matrix, "combined covariance")?;
    let max_diagonal = (0..n).map(|index| matrix[index][index]).fold(0.0, f64::max);
    if max_diagonal <= 0.0 {
        return error("combined covariance has no positive variance");
    }

    let mut lower = vec![vec![0.0; n]; n];
    let mut minimum_pivot = f64::INFINITY;
    let mut maximum_pivot = 0.0_f64;
    for i in 0..n {
        for j in 0..=i {
            let mut residual = matrix[i][j];
            for k in 0..j {
                residual -= lower[i][k] * lower[j][k];
            }
            if i == j {
                if !residual.is_finite() || residual <= MIN_RELATIVE_PIVOT * max_diagonal {
                    return error(format!(
                        "combined covariance is singular or ill-conditioned at pivot {i}"
                    ));
                }
                let pivot = residual.sqrt();
                lower[i][j] = pivot;
                minimum_pivot = minimum_pivot.min(residual);
                maximum_pivot = maximum_pivot.max(residual);
            } else {
                lower[i][j] = residual / lower[j][j];
            }
        }
    }
    Ok((lower, minimum_pivot, maximum_pivot))
}

fn forward_solve(lower: &Matrix, right: &[f64]) -> Vec<f64> {
    let n = right.len();
    let mut solution = vec![0.0; n];
    for i in 0..n {
        let mut value = right[i];
        for j in 0..i {
            value -= lower[i][j] * solution[j];
        }
        solution[i] = value / lower[i][i];
    }
    solution
}

fn backward_solve_transpose(lower: &Matrix, right: &[f64]) -> Vec<f64> {
    let n = right.len();
    let mut solution = vec![0.0; n];
    for i in (0..n).rev() {
        let mut value = right[i];
        for j in (i + 1)..n {
            value -= lower[j][i] * solution[j];
        }
        solution[i] = value / lower[i][i];
    }
    solution
}

fn infinity_norm(matrix: &Matrix) -> f64 {
    matrix
        .iter()
        .map(|row| row.iter().map(|value| value.abs()).sum::<f64>())
        .fold(0.0, f64::max)
}

fn condition_infinity_estimate(matrix: &Matrix, lower: &Matrix) -> f64 {
    let n = matrix.len();
    let mut inverse = vec![vec![0.0; n]; n];
    for column in 0..n {
        let mut unit = vec![0.0; n];
        unit[column] = 1.0;
        let intermediate = forward_solve(lower, &unit);
        let solved = backward_solve_transpose(lower, &intermediate);
        for row in 0..n {
            inverse[row][column] = solved[row];
        }
    }
    infinity_norm(matrix) * infinity_norm(&inverse)
}

fn matrix_sum(
    experimental: &Matrix,
    numerical: &Matrix,
    input: &Matrix,
    cross_source: Option<&Matrix>,
) -> Matrix {
    let first = add_matrices(experimental, numerical);
    let second = add_matrices(&first, input);
    if let Some(cross) = cross_source {
        add_matrices(&second, cross)
    } else {
        second
    }
}

pub fn evaluate(request: &ResponseVectorRequest) -> Result<ResponseVectorResult> {
    if request.schema_version != "aero-response-vvuq/0.1.0" {
        return error("unsupported response-vector schema version");
    }
    require_nonempty(&request.response_family_id, "response family ID")?;
    require_sha256(
        &request.held_out_validation_dataset_sha256,
        "held-out validation dataset digest",
    )?;
    if let Some(model_discrepancy) = &request.model_discrepancy_evidence_sha256 {
        require_sha256(model_discrepancy, "model discrepancy evidence digest")?;
    }

    let identity_hash = validate_identity(&request.identity)?;
    let channels = canonical_channels(request)?;
    let canonical_ids = channels
        .iter()
        .map(|channel| channel.id.clone())
        .collect::<Vec<_>>();
    let normalization_hash = normalization_basis_hash(&channels)?;

    let component_sources = [
        &request.uncertainty.experimental,
        &request.uncertainty.numerical,
        &request.uncertainty.input,
    ];
    let mut source_ids = BTreeSet::new();
    for block in component_sources {
        if !source_ids.insert(block.source_id.clone()) {
            return error(
                "experimental, numerical and input covariance source IDs must be distinct",
            );
        }
    }

    let experimental = validate_component_block(
        &request.uncertainty.experimental,
        &canonical_ids,
        &identity_hash,
        &normalization_hash,
        &request.held_out_validation_dataset_sha256,
        "experimental covariance",
    )?;
    let numerical = validate_component_block(
        &request.uncertainty.numerical,
        &canonical_ids,
        &identity_hash,
        &normalization_hash,
        &request.held_out_validation_dataset_sha256,
        "numerical covariance",
    )?;
    let input = validate_component_block(
        &request.uncertainty.input,
        &canonical_ids,
        &identity_hash,
        &normalization_hash,
        &request.held_out_validation_dataset_sha256,
        "input covariance",
    )?;

    let cross_source = match (
        &request.uncertainty.independent_components,
        &request.uncertainty.cross_source,
    ) {
        (true, None) => None,
        (true, Some(_)) => {
            return error(
                "cross-source covariance supplied while components are declared independent",
            );
        }
        (false, None) => {
            return error(
                "cross-source covariance is required when uncertainty components are not independent",
            );
        }
        (false, Some(block)) => {
            if !source_ids.insert(block.source_id.clone()) {
                return error("cross-source covariance source ID must be distinct");
            }
            validate_covariance_provenance(
                block,
                &identity_hash,
                &normalization_hash,
                &request.held_out_validation_dataset_sha256,
                "cross-source covariance",
                false,
            )?;
            Some(reorder_covariance(
                block,
                &canonical_ids,
                "cross-source covariance",
            )?)
        }
    };

    let total = matrix_sum(&experimental, &numerical, &input, cross_source.as_ref());
    let (lower, minimum_pivot, maximum_pivot) = cholesky(&total)?;
    let condition = condition_infinity_estimate(&total, &lower);
    if !condition.is_finite() || condition > MAX_CONDITION_INFINITY {
        return error(format!(
            "combined covariance condition estimate exceeds limit: {condition:.6e}"
        ));
    }

    let residuals = channels
        .iter()
        .map(|channel| {
            (channel.simulation_value - channel.reference_value) / channel.normalization_scale
        })
        .collect::<Vec<_>>();
    let whitened = forward_solve(&lower, &residuals);
    let d2 = whitened.iter().map(|value| value * value).sum::<f64>();
    if !d2.is_finite() || d2 < 0.0 {
        return error("covariance-weighted squared distance is invalid");
    }

    let mut diagnostics = Vec::with_capacity(channels.len());
    let mut maximum_abs_z = 0.0_f64;
    for (index, channel) in channels.iter().enumerate() {
        let variance = total[index][index];
        if !variance.is_finite() || variance <= 0.0 {
            return error(format!(
                "combined variance for {} is not positive",
                channel.id
            ));
        }
        let standard_deviation = variance.sqrt();
        let marginal_z = residuals[index] / standard_deviation;
        maximum_abs_z = maximum_abs_z.max(marginal_z.abs());
        diagnostics.push(ChannelDiagnostic {
            id: channel.id.clone(),
            quantity: channel.quantity.clone(),
            units: channel.units.clone(),
            coordinate_frame: channel.coordinate_frame.clone(),
            sample_id: channel.sample_id.clone(),
            reference_value: channel.reference_value,
            simulation_value: channel.simulation_value,
            physical_residual: channel.simulation_value - channel.reference_value,
            normalized_residual: residuals[index],
            combined_standard_deviation: standard_deviation,
            marginal_standardized_residual: marginal_z,
        });
    }

    let (decision_status, limit_d2, limit_z, protocol) = match &request.decision_limit {
        None => ("DIAGNOSTIC_ONLY".to_owned(), None, None, None),
        Some(limit) => {
            finite(limit.max_d2, "maximum D2 decision limit")?;
            finite(
                limit.max_abs_marginal_z,
                "maximum marginal z decision limit",
            )?;
            if limit.max_d2 < 0.0 || limit.max_abs_marginal_z < 0.0 {
                return error("decision limits must be non-negative");
            }
            require_sha256(&limit.protocol_sha256, "decision protocol digest")?;
            require_sha256(&limit.claim_scope_sha256, "decision claim-scope digest")?;
            if limit.claim_scope_sha256 != request.identity.claim_scope_sha256 {
                return error(
                    "decision limit claim scope does not match response-vector claim scope",
                );
            }
            let status = if d2 > limit.max_d2 || maximum_abs_z > limit.max_abs_marginal_z {
                "EXCEEDS_DECLARED_LIMIT"
            } else {
                "WITHIN_DECLARED_LIMIT"
            };
            (
                status.to_owned(),
                Some(limit.max_d2),
                Some(limit.max_abs_marginal_z),
                Some(limit.protocol_sha256.clone()),
            )
        }
    };

    let uncertainty_sources = [
        &request.uncertainty.experimental,
        &request.uncertainty.numerical,
        &request.uncertainty.input,
    ]
    .iter()
    .map(|block| UncertaintySourceReceipt {
        source_id: block.source_id.clone(),
        artifact_sha256: block.artifact_sha256.clone(),
        estimator_build_sha256: block.estimator_build_sha256.clone(),
        calibration_dataset_sha256: block.calibration_dataset_sha256.clone(),
        calibration_envelope_sha256: block.calibration_envelope_sha256.clone(),
    })
    .collect();

    Ok(ResponseVectorResult {
        schema_version: "aero-response-vvuq-result/0.1.0".to_owned(),
        response_family_id: request.response_family_id.clone(),
        state_identity_sha256: identity_hash,
        normalization_basis_sha256: normalization_hash,
        held_out_validation_dataset_sha256: request.held_out_validation_dataset_sha256.clone(),
        channel_diagnostics: diagnostics,
        covariance_weighted_d2: d2,
        covariance_condition_infinity_estimate: condition,
        minimum_cholesky_pivot: minimum_pivot,
        maximum_cholesky_pivot: maximum_pivot,
        maximum_absolute_marginal_z: maximum_abs_z,
        uncertainty_sources,
        cross_source_covariance_source_id: request
            .uncertainty
            .cross_source
            .as_ref()
            .map(|block| block.source_id.clone()),
        decision_status,
        decision_limit_d2: limit_d2,
        decision_limit_max_abs_marginal_z: limit_z,
        decision_protocol_sha256: protocol,
        model_discrepancy_evidence_sha256: request.model_discrepancy_evidence_sha256.clone(),
        claim_ceiling: "multivariate_response_discrepancy_only",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../data/known-answer-response-vector-v0.1.json");

    fn request() -> ResponseVectorRequest {
        serde_json::from_str(FIXTURE).expect("known-answer fixture must deserialize")
    }

    #[test]
    fn known_answer_covariance_aware_distance_is_reproduced() {
        let result = evaluate(&request()).expect("known-answer response should evaluate");
        assert!((result.covariance_weighted_d2 - 18.493035490698507).abs() < 1.0e-10);
        assert_eq!(result.decision_status, "WITHIN_DECLARED_LIMIT");
        assert_eq!(
            result.claim_ceiling,
            "multivariate_response_discrepancy_only"
        );
        assert_eq!(result.channel_diagnostics.len(), 3);
    }

    #[test]
    fn response_channel_reordering_does_not_change_metric() {
        let mut input = request();
        let original = evaluate(&input).unwrap();
        input.channels.reverse();
        let reordered = evaluate(&input).unwrap();
        assert!(
            (original.covariance_weighted_d2 - reordered.covariance_weighted_d2).abs() < 1.0e-12
        );
        assert_eq!(
            original
                .channel_diagnostics
                .iter()
                .map(|channel| &channel.id)
                .collect::<Vec<_>>(),
            reordered
                .channel_diagnostics
                .iter()
                .map(|channel| &channel.id)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn missing_channel_is_rejected_not_zero_filled() {
        let mut input = request();
        input.channels.pop();
        assert!(evaluate(&input).is_err());
    }

    #[test]
    fn duplicate_channel_id_is_rejected() {
        let mut input = request();
        input.channels[1].id = input.channels[0].id.clone();
        assert!(evaluate(&input).is_err());
    }

    #[test]
    fn changed_unit_invalidates_normalization_basis() {
        let mut input = request();
        input.channels[0].units = "kN".to_owned();
        assert!(evaluate(&input).is_err());
    }

    #[test]
    fn stale_state_identity_invalidates_covariance_binding() {
        let mut input = request();
        input.identity.geometry_sha256 = "f".repeat(64);
        assert!(evaluate(&input).is_err());
    }

    #[test]
    fn covariance_asymmetry_is_rejected() {
        let mut input = request();
        input.uncertainty.experimental.matrix[0][1] += 0.01;
        assert!(evaluate(&input).is_err());
    }

    #[test]
    fn indefinite_component_covariance_is_rejected() {
        let mut input = request();
        input.uncertainty.experimental.matrix = vec![
            vec![0.04, 0.20, 0.0],
            vec![0.20, 0.09, 0.0],
            vec![0.0, 0.0, 0.04],
        ];
        assert!(evaluate(&input).is_err());
    }

    #[test]
    fn singular_total_covariance_is_rejected() {
        let mut input = request();
        for block in [
            &mut input.uncertainty.experimental,
            &mut input.uncertainty.numerical,
            &mut input.uncertainty.input,
        ] {
            block.matrix = vec![vec![0.0; 3]; 3];
            block.max_diagonal_variance = Some(vec![1.0e-9; 3]);
        }
        assert!(evaluate(&input).is_err());
    }

    #[test]
    fn calibration_variance_inflation_beyond_envelope_is_rejected() {
        let mut input = request();
        input.uncertainty.experimental.matrix[1][1] = 0.50;
        assert!(evaluate(&input).is_err());
    }

    #[test]
    fn validation_dataset_cannot_be_reused_as_calibration_data() {
        let mut input = request();
        input.uncertainty.input.calibration_dataset_sha256 =
            input.held_out_validation_dataset_sha256.clone();
        assert!(evaluate(&input).is_err());
    }

    #[test]
    fn missing_decision_limit_produces_diagnostic_only() {
        let mut input = request();
        input.decision_limit = None;
        let result = evaluate(&input).unwrap();
        assert_eq!(result.decision_status, "DIAGNOSTIC_ONLY");
    }

    #[test]
    fn decision_limit_from_wrong_claim_scope_is_rejected() {
        let mut input = request();
        input.decision_limit.as_mut().unwrap().claim_scope_sha256 = "0".repeat(64);
        assert!(evaluate(&input).is_err());
    }

    #[test]
    fn component_gate_preserves_local_outlier() {
        let mut input = request();
        let limit = input.decision_limit.as_mut().unwrap();
        limit.max_d2 = 1_000_000.0;
        limit.max_abs_marginal_z = 2.0;
        let result = evaluate(&input).unwrap();
        assert_eq!(result.decision_status, "EXCEEDS_DECLARED_LIMIT");
        assert!(result.maximum_abs_marginal_z > 2.0);
    }

    #[test]
    fn dependent_uncertainty_components_require_cross_covariance() {
        let mut input = request();
        input.uncertainty.independent_components = false;
        assert!(evaluate(&input).is_err());
    }
}
