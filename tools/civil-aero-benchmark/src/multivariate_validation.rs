//! Joint, covariance-aware validation diagnostics for coupled engineering outputs.
//!
//! The score is a squared Mahalanobis discrepancy over standardized residuals. It preserves
//! cross-output correlation, but is diagnostic only: no chi-square tail probability, coverage,
//! validation pass, or claim-admission decision is inferred without a separately justified
//! distributional model and calibration study.

use std::collections::BTreeSet;

use crate::integrity::sha256_hex;
use crate::interval_calibration::DomainStatus;

pub const MULTIVARIATE_VALIDATION_SCHEMA: &str = "aero-multivariate-validation/0.1.0";
pub const MULTIVARIATE_VALIDATION_EVIDENCE_LEVEL: &str =
    "covariance_aware_joint_discrepancy_diagnostic_only_no_distributional_coverage_or_acceptance_claim";
const SYMMETRY_TOLERANCE: f64 = 1.0e-10;
const DIAGONAL_TOLERANCE: f64 = 1.0e-10;
const CHOLESKY_PIVOT_TOLERANCE: f64 = 1.0e-12;

#[derive(Debug, Clone, PartialEq)]
pub struct JointValidationOutput {
    pub quantity: String,
    pub unit: String,
    pub predicted_value: f64,
    pub observed_value: f64,
    /// Standard uncertainty of predicted-minus-observed discrepancy, in `unit`.
    pub standard_uncertainty: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JointValidationObservation {
    pub observation_id: String,
    pub geometry_family: String,
    pub flow_regime: String,
    /// Must be blind_confirmation; calibration/training observations are not scored here.
    pub dataset_split: String,
    pub domain_status: DomainStatus,
    pub uncertainty_source_reference: String,
    pub uncertainty_source_sha256: String,
    pub uncertainty_model_rationale: String,
    pub outputs: Vec<JointValidationOutput>,
    /// Dimensionless positive-definite correlation matrix in exactly the outputs' order.
    pub correlation_matrix: Vec<Vec<f64>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JointValidationStudy {
    pub study_id: String,
    pub model_id: String,
    pub observations: Vec<JointValidationObservation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JointValidationObservationReport {
    pub observation: JointValidationObservation,
    /// (predicted - observed) / standard_uncertainty, in declared output order.
    pub standardized_residuals: Vec<f64>,
    /// Forward solve L y = z where R = L L^T. Components retain declared output order.
    pub whitened_residuals: Vec<f64>,
    pub mahalanobis_squared: f64,
    pub mahalanobis_distance: f64,
    pub observation_sha256: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JointValidationReport {
    pub study_id: String,
    pub model_id: String,
    pub output_schema: Vec<(String, String)>,
    pub observations: Vec<JointValidationObservationReport>,
    pub in_domain_observation_count: usize,
    pub out_of_domain_observation_count: usize,
    pub mean_mahalanobis_squared: f64,
    pub maximum_mahalanobis_squared: f64,
    pub study_sha256: String,
    pub report_sha256: String,
    pub evidence_level: &'static str,
}

fn require_nonempty(value: &str, field: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{field} must not be empty"));
    }
    Ok(())
}

fn is_lower_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn append_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn append_string(bytes: &mut Vec<u8>, value: &str) {
    append_u64(bytes, value.len() as u64);
    bytes.extend_from_slice(value.as_bytes());
}

fn append_f64(bytes: &mut Vec<u8>, value: f64) {
    bytes.extend_from_slice(&value.to_bits().to_le_bytes());
}

fn cholesky_lower(matrix: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, String> {
    let dimension = matrix.len();
    if dimension == 0 || matrix.iter().any(|row| row.len() != dimension) {
        return Err("correlation matrix must be non-empty and square".to_owned());
    }
    for (row_index, row) in matrix.iter().enumerate() {
        for (column_index, value) in row.iter().enumerate() {
            if !value.is_finite() {
                return Err("correlation matrix values must be finite".to_owned());
            }
            if (*value - matrix[column_index][row_index]).abs() > SYMMETRY_TOLERANCE {
                return Err("correlation matrix must be symmetric".to_owned());
            }
            if row_index == column_index && (*value - 1.0).abs() > DIAGONAL_TOLERANCE {
                return Err("correlation matrix diagonal entries must equal one".to_owned());
            }
        }
    }

    let mut lower = vec![vec![0.0; dimension]; dimension];
    for row in 0..dimension {
        for column in 0..=row {
            let mut value = matrix[row][column];
            for inner in 0..column {
                value -= lower[row][inner] * lower[column][inner];
            }
            if row == column {
                if !value.is_finite() || value <= CHOLESKY_PIVOT_TOLERANCE {
                    return Err(
                        "correlation matrix must be strictly positive definite with stable pivots"
                            .to_owned(),
                    );
                }
                lower[row][column] = value.sqrt();
            } else {
                let pivot = lower[column][column];
                if !pivot.is_finite() || pivot <= 0.0 {
                    return Err("correlation matrix Cholesky pivot is invalid".to_owned());
                }
                lower[row][column] = value / pivot;
            }
        }
    }
    Ok(lower)
}

fn solve_lower(lower: &[Vec<f64>], right_hand_side: &[f64]) -> Result<Vec<f64>, String> {
    if lower.len() != right_hand_side.len() {
        return Err("whitening vector dimension differs from Cholesky factor".to_owned());
    }
    let mut solution = vec![0.0; lower.len()];
    for row in 0..lower.len() {
        let mut value = right_hand_side[row];
        for column in 0..row {
            value -= lower[row][column] * solution[column];
        }
        let pivot = lower[row][row];
        if !pivot.is_finite() || pivot <= 0.0 {
            return Err("whitening solve encountered invalid Cholesky pivot".to_owned());
        }
        solution[row] = value / pivot;
        if !solution[row].is_finite() {
            return Err("whitened residual is non-finite".to_owned());
        }
    }
    Ok(solution)
}

fn observation_canonical_bytes(
    study_id: &str,
    model_id: &str,
    observation: &JointValidationObservation,
) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    for value in [
        MULTIVARIATE_VALIDATION_SCHEMA,
        study_id,
        model_id,
        observation.observation_id.as_str(),
        observation.geometry_family.as_str(),
        observation.flow_regime.as_str(),
        observation.dataset_split.as_str(),
        observation.domain_status.as_str(),
        observation.uncertainty_source_reference.as_str(),
        observation.uncertainty_source_sha256.as_str(),
        observation.uncertainty_model_rationale.as_str(),
    ] {
        append_string(&mut bytes, value);
    }
    append_u64(&mut bytes, observation.outputs.len() as u64);
    for output in &observation.outputs {
        append_string(&mut bytes, &output.quantity);
        append_string(&mut bytes, &output.unit);
        append_f64(&mut bytes, output.predicted_value);
        append_f64(&mut bytes, output.observed_value);
        append_f64(&mut bytes, output.standard_uncertainty);
    }
    append_u64(&mut bytes, observation.correlation_matrix.len() as u64);
    for row in &observation.correlation_matrix {
        append_u64(&mut bytes, row.len() as u64);
        for value in row {
            append_f64(&mut bytes, *value);
        }
    }
    Ok(bytes)
}

fn study_sha256(
    study_id: &str,
    model_id: &str,
    observations: &[JointValidationObservationReport],
) -> Result<String, String> {
    let mut bytes = Vec::new();
    append_string(&mut bytes, MULTIVARIATE_VALIDATION_SCHEMA);
    append_string(&mut bytes, study_id);
    append_string(&mut bytes, model_id);
    append_u64(&mut bytes, observations.len() as u64);
    for observation in observations {
        append_string(&mut bytes, &observation.observation.observation_id);
        append_string(&mut bytes, &observation.observation_sha256);
    }
    sha256_hex(&bytes)
}

fn report_sha256(report: &JointValidationReport) -> Result<String, String> {
    let mut bytes = Vec::new();
    for value in [
        MULTIVARIATE_VALIDATION_SCHEMA,
        report.study_id.as_str(),
        report.model_id.as_str(),
        report.evidence_level,
        report.study_sha256.as_str(),
    ] {
        append_string(&mut bytes, value);
    }
    append_u64(&mut bytes, report.in_domain_observation_count as u64);
    append_u64(&mut bytes, report.out_of_domain_observation_count as u64);
    append_f64(&mut bytes, report.mean_mahalanobis_squared);
    append_f64(&mut bytes, report.maximum_mahalanobis_squared);
    append_u64(&mut bytes, report.observations.len() as u64);
    for observation in &report.observations {
        append_string(&mut bytes, &observation.observation.observation_id);
        append_string(&mut bytes, &observation.observation_sha256);
        append_f64(&mut bytes, observation.mahalanobis_squared);
        append_f64(&mut bytes, observation.mahalanobis_distance);
        append_u64(&mut bytes, observation.standardized_residuals.len() as u64);
        for value in &observation.standardized_residuals {
            append_f64(&mut bytes, *value);
        }
        append_u64(&mut bytes, observation.whitened_residuals.len() as u64);
        for value in &observation.whitened_residuals {
            append_f64(&mut bytes, *value);
        }
    }
    sha256_hex(&bytes)
}

pub fn evaluate_joint_validation(
    study: &JointValidationStudy,
) -> Result<JointValidationReport, String> {
    require_nonempty(&study.study_id, "study_id")?;
    require_nonempty(&study.model_id, "model_id")?;
    if study.observations.is_empty() {
        return Err("joint validation study must contain observations".to_owned());
    }

    let first = &study.observations[0];
    if first.outputs.is_empty() {
        return Err("joint validation must include at least one output".to_owned());
    }
    let output_schema: Vec<(String, String)> = first
        .outputs
        .iter()
        .map(|output| (output.quantity.clone(), output.unit.clone()))
        .collect();
    let mut unique_outputs = BTreeSet::new();
    for (quantity, unit) in &output_schema {
        require_nonempty(quantity, "output quantity")?;
        require_nonempty(unit, "output unit")?;
        if !unique_outputs.insert(quantity.as_str()) {
            return Err(format!("duplicate output quantity {quantity}"));
        }
    }

    let mut seen_observations = BTreeSet::new();
    let mut observation_reports = Vec::with_capacity(study.observations.len());
    for observation in &study.observations {
        require_nonempty(&observation.observation_id, "observation_id")?;
        require_nonempty(&observation.geometry_family, "geometry_family")?;
        require_nonempty(&observation.flow_regime, "flow_regime")?;
        require_nonempty(
            &observation.uncertainty_source_reference,
            "uncertainty_source_reference",
        )?;
        require_nonempty(
            &observation.uncertainty_model_rationale,
            "uncertainty_model_rationale",
        )?;
        if observation.dataset_split != "blind_confirmation" {
            return Err(format!(
                "observation {} must come from blind_confirmation, got {}",
                observation.observation_id, observation.dataset_split
            ));
        }
        if !is_lower_sha256(&observation.uncertainty_source_sha256) {
            return Err(format!(
                "observation {} uncertainty source must have a lowercase SHA-256 digest",
                observation.observation_id
            ));
        }
        if !seen_observations.insert(observation.observation_id.as_str()) {
            return Err(format!(
                "duplicate joint validation observation {}",
                observation.observation_id
            ));
        }
        if observation.outputs.len() != output_schema.len() {
            return Err(format!(
                "observation {} output count differs from the study schema",
                observation.observation_id
            ));
        }
        for (index, output) in observation.outputs.iter().enumerate() {
            if output.quantity != output_schema[index].0 || output.unit != output_schema[index].1 {
                return Err(format!(
                    "observation {} output index {index} differs from frozen quantity/unit order",
                    observation.observation_id
                ));
            }
            for (field, value) in [
                ("predicted_value", output.predicted_value),
                ("observed_value", output.observed_value),
                ("standard_uncertainty", output.standard_uncertainty),
            ] {
                if !value.is_finite() {
                    return Err(format!(
                        "observation {} output {} {field} must be finite",
                        observation.observation_id, output.quantity
                    ));
                }
            }
            if output.standard_uncertainty <= 0.0 {
                return Err(format!(
                    "observation {} output {} standard uncertainty must be positive",
                    observation.observation_id, output.quantity
                ));
            }
        }

        let lower = cholesky_lower(&observation.correlation_matrix)?;
        if lower.len() != output_schema.len() {
            return Err(format!(
                "observation {} correlation dimension differs from output schema",
                observation.observation_id
            ));
        }
        let standardized_residuals: Vec<f64> = observation
            .outputs
            .iter()
            .map(|output| {
                (output.predicted_value - output.observed_value) / output.standard_uncertainty
            })
            .collect();
        if standardized_residuals.iter().any(|value| !value.is_finite()) {
            return Err(format!(
                "observation {} standardized residual is non-finite",
                observation.observation_id
            ));
        }
        let whitened_residuals = solve_lower(&lower, &standardized_residuals)?;
        let mahalanobis_squared = whitened_residuals.iter().map(|value| value * value).sum::<f64>();
        let mahalanobis_distance = mahalanobis_squared.sqrt();
        if !mahalanobis_squared.is_finite() || !mahalanobis_distance.is_finite() {
            return Err(format!(
                "observation {} joint discrepancy is non-finite",
                observation.observation_id
            ));
        }
        let observation_sha256 = sha256_hex(&observation_canonical_bytes(
            &study.study_id,
            &study.model_id,
            observation,
        )?)?;

        observation_reports.push(JointValidationObservationReport {
            observation: observation.clone(),
            standardized_residuals,
            whitened_residuals,
            mahalanobis_squared,
            mahalanobis_distance,
            observation_sha256,
        });
    }

    observation_reports.sort_by(|left, right| {
        left.observation
            .observation_id
            .cmp(&right.observation.observation_id)
    });
    let in_domain_observation_count = observation_reports
        .iter()
        .filter(|row| row.observation.domain_status == DomainStatus::InDomain)
        .count();
    let out_of_domain_observation_count = observation_reports.len() - in_domain_observation_count;
    let mean_mahalanobis_squared = observation_reports
        .iter()
        .map(|row| row.mahalanobis_squared)
        .sum::<f64>()
        / observation_reports.len() as f64;
    let maximum_mahalanobis_squared = observation_reports
        .iter()
        .map(|row| row.mahalanobis_squared)
        .fold(0.0_f64, f64::max);
    let study_sha256 = study_sha256(&study.study_id, &study.model_id, &observation_reports)?;

    let mut report = JointValidationReport {
        study_id: study.study_id.clone(),
        model_id: study.model_id.clone(),
        output_schema,
        observations: observation_reports,
        in_domain_observation_count,
        out_of_domain_observation_count,
        mean_mahalanobis_squared,
        maximum_mahalanobis_squared,
        study_sha256,
        report_sha256: String::new(),
        evidence_level: MULTIVARIATE_VALIDATION_EVIDENCE_LEVEL,
    };
    report.report_sha256 = report_sha256(&report)?;
    Ok(report)
}

pub fn verify_joint_validation_report(report: &JointValidationReport) -> Result<(), String> {
    if report.evidence_level != MULTIVARIATE_VALIDATION_EVIDENCE_LEVEL {
        return Err("joint validation evidence level is not diagnostic-only".to_owned());
    }
    if report.output_schema.is_empty() || report.observations.is_empty() {
        return Err("joint validation report must contain an output schema and observations".to_owned());
    }
    let mut unique = BTreeSet::new();
    for row in &report.observations {
        if !unique.insert(row.observation.observation_id.as_str()) {
            return Err("joint validation report contains duplicate observations".to_owned());
        }
        let expected_observation = sha256_hex(&observation_canonical_bytes(
            &report.study_id,
            &report.model_id,
            &row.observation,
        )?)?;
        if expected_observation != row.observation_sha256 {
            return Err(format!(
                "observation {} digest mismatch",
                row.observation.observation_id
            ));
        }
        if row.standardized_residuals.len() != report.output_schema.len()
            || row.whitened_residuals.len() != report.output_schema.len()
            || row
                .observation
                .outputs
                .iter()
                .zip(&report.output_schema)
                .any(|(output, schema)| output.quantity != schema.0 || output.unit != schema.1)
        {
            return Err("joint validation report output schema mismatch".to_owned());
        }
        let lower = cholesky_lower(&row.observation.correlation_matrix)?;
        let expected_standardized: Vec<f64> = row
            .observation
            .outputs
            .iter()
            .map(|output| {
                (output.predicted_value - output.observed_value) / output.standard_uncertainty
            })
            .collect();
        let expected_whitened = solve_lower(&lower, &expected_standardized)?;
        let expected_squared = expected_whitened.iter().map(|value| value * value).sum::<f64>();
        if !vectors_close(&row.standardized_residuals, &expected_standardized)
            || !vectors_close(&row.whitened_residuals, &expected_whitened)
            || !close(row.mahalanobis_squared, expected_squared)
            || !close(row.mahalanobis_distance, expected_squared.sqrt())
        {
            return Err(format!(
                "observation {} joint discrepancy does not replay",
                row.observation.observation_id
            ));
        }
    }
    let in_domain = report
        .observations
        .iter()
        .filter(|row| row.observation.domain_status == DomainStatus::InDomain)
        .count();
    let out_domain = report.observations.len() - in_domain;
    let mean = report.observations.iter().map(|row| row.mahalanobis_squared).sum::<f64>()
        / report.observations.len() as f64;
    let maximum = report.observations.iter().map(|row| row.mahalanobis_squared).fold(0.0_f64, f64::max);
    if in_domain != report.in_domain_observation_count
        || out_domain != report.out_of_domain_observation_count
        || !close(mean, report.mean_mahalanobis_squared)
        || !close(maximum, report.maximum_mahalanobis_squared)
    {
        return Err("joint validation study summary does not replay".to_owned());
    }
    let expected_study = study_sha256(&report.study_id, &report.model_id, &report.observations)?;
    if expected_study != report.study_sha256 {
        return Err("joint validation study digest mismatch".to_owned());
    }
    if report_sha256(report)? != report.report_sha256 {
        return Err("joint validation report digest mismatch".to_owned());
    }
    Ok(())
}

fn close(left: f64, right: f64) -> bool {
    (left - right).abs() <= 1.0e-12 * left.abs().max(right.abs()).max(1.0)
}

fn vectors_close(left: &[f64], right: &[f64]) -> bool {
    left.len() == right.len() && left.iter().zip(right).all(|(a, b)| close(*a, *b))
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

fn csv_row(values: &[String]) -> String {
    values.iter().map(|value| csv_escape(value)).collect::<Vec<_>>().join(",")
}

fn number_list(values: &[f64]) -> String {
    values.iter().map(ToString::to_string).collect::<Vec<_>>().join(";")
}

/// One row per output, with repeated identities and one correlation row for independent replay.
pub fn joint_validation_report_to_csv(report: &JointValidationReport) -> Result<String, String> {
    verify_joint_validation_report(report)?;
    let header = "record_type,schema,study_id,model_id,observation_id,geometry_family,flow_regime,dataset_split,domain_status,uncertainty_source_reference,uncertainty_source_sha256,uncertainty_model_rationale,output_index,quantity,unit,predicted_value,observed_value,standard_uncertainty,correlation_row,standardized_residual,whitened_residual,mahalanobis_squared,mahalanobis_distance,in_domain_observation_count,out_of_domain_observation_count,mean_mahalanobis_squared,maximum_mahalanobis_squared,observation_sha256,study_sha256,report_sha256";
    let mut output = format!("{header}\n");
    let metadata = vec![
        "study".to_owned(),
        MULTIVARIATE_VALIDATION_SCHEMA.to_owned(),
        report.study_id.clone(),
        report.model_id.clone(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        report.in_domain_observation_count.to_string(),
        report.out_of_domain_observation_count.to_string(),
        report.mean_mahalanobis_squared.to_string(),
        report.maximum_mahalanobis_squared.to_string(),
        String::new(),
        report.study_sha256.clone(),
        report.report_sha256.clone(),
    ];
    output.push_str(&csv_row(&metadata));
    output.push('\n');

    for row in &report.observations {
        for (index, value) in row.observation.outputs.iter().enumerate() {
            let mut fields = vec![String::new(); 30];
            fields[0] = "output".to_owned();
            fields[1] = MULTIVARIATE_VALIDATION_SCHEMA.to_owned();
            fields[2] = report.study_id.clone();
            fields[3] = report.model_id.clone();
            fields[4] = row.observation.observation_id.clone();
            fields[5] = row.observation.geometry_family.clone();
            fields[6] = row.observation.flow_regime.clone();
            fields[7] = row.observation.dataset_split.clone();
            fields[8] = row.observation.domain_status.as_str().to_owned();
            fields[9] = row.observation.uncertainty_source_reference.clone();
            fields[10] = row.observation.uncertainty_source_sha256.clone();
            fields[11] = row.observation.uncertainty_model_rationale.clone();
            fields[12] = index.to_string();
            fields[13] = value.quantity.clone();
            fields[14] = value.unit.clone();
            fields[15] = value.predicted_value.to_string();
            fields[16] = value.observed_value.to_string();
            fields[17] = value.standard_uncertainty.to_string();
            fields[18] = number_list(&row.observation.correlation_matrix[index]);
            fields[19] = row.standardized_residuals[index].to_string();
            fields[20] = row.whitened_residuals[index].to_string();
            fields[21] = row.mahalanobis_squared.to_string();
            fields[22] = row.mahalanobis_distance.to_string();
            fields[23] = report.in_domain_observation_count.to_string();
            fields[24] = report.out_of_domain_observation_count.to_string();
            fields[25] = report.mean_mahalanobis_squared.to_string();
            fields[26] = report.maximum_mahalanobis_squared.to_string();
            fields[27] = row.observation_sha256.clone();
            fields[28] = report.study_sha256.clone();
            fields[29] = report.report_sha256.clone();
            output.push_str(&csv_row(&fields));
            output.push('\n');
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(id: &str, domain_status: DomainStatus, correlation: f64) -> JointValidationObservation {
        JointValidationObservation {
            observation_id: id.to_owned(),
            geometry_family: "OAT15A".to_owned(),
            flow_regime: "transonic_steady".to_owned(),
            dataset_split: "blind_confirmation".to_owned(),
            domain_status,
            uncertainty_source_reference: format!("fixture://uncertainty/{id}"),
            uncertainty_source_sha256: "a".repeat(64),
            uncertainty_model_rationale: "synthetic known-answer covariance fixture".to_owned(),
            outputs: vec![
                JointValidationOutput {
                    quantity: "CL".to_owned(),
                    unit: "1".to_owned(),
                    predicted_value: 1.1,
                    observed_value: 1.0,
                    standard_uncertainty: 0.1,
                },
                JointValidationOutput {
                    quantity: "CD".to_owned(),
                    unit: "1".to_owned(),
                    predicted_value: 0.02,
                    observed_value: 0.01,
                    standard_uncertainty: 0.01,
                },
            ],
            correlation_matrix: vec![vec![1.0, correlation], vec![correlation, 1.0]],
        }
    }

    fn study(correlation: f64) -> JointValidationStudy {
        JointValidationStudy {
            study_id: "synthetic-joint-validation-v1".to_owned(),
            model_id: "synthetic-solver-model-v1".to_owned(),
            observations: vec![
                observation("sample-in", DomainStatus::InDomain, correlation),
                observation("sample-ood", DomainStatus::OutOfDomain, correlation),
            ],
        }
    }

    #[test]
    fn identity_correlation_recovers_sum_of_squared_standardized_residuals() {
        let report = evaluate_joint_validation(&study(0.0)).unwrap();
        let row = report
            .observations
            .iter()
            .find(|row| row.observation.observation_id == "sample-in")
            .unwrap();
        assert_eq!(row.standardized_residuals, vec![1.0, 1.0]);
        assert_eq!(row.whitened_residuals, vec![1.0, 1.0]);
        assert!((row.mahalanobis_squared - 2.0).abs() < 1.0e-12);
        assert!((row.mahalanobis_distance - 2.0_f64.sqrt()).abs() < 1.0e-12);
        assert_eq!(report.in_domain_observation_count, 1);
        assert_eq!(report.out_of_domain_observation_count, 1);
    }

    #[test]
    fn cross_output_correlation_changes_joint_score_without_changing_marginals() {
        let independent = evaluate_joint_validation(&study(0.0)).unwrap();
        let correlated = evaluate_joint_validation(&study(0.5)).unwrap();
        let independent_row = &independent.observations[0];
        let correlated_row = &correlated.observations[0];
        assert_eq!(
            independent_row.standardized_residuals,
            correlated_row.standardized_residuals
        );
        assert!((independent_row.mahalanobis_squared - 2.0).abs() < 1.0e-12);
        assert!((correlated_row.mahalanobis_squared - 4.0 / 3.0).abs() < 1.0e-12);
        assert_ne!(independent_row.observation_sha256, correlated_row.observation_sha256);
    }

    #[test]
    fn blind_split_and_exact_output_order_are_required() {
        let mut not_blind = study(0.0);
        not_blind.observations[0].dataset_split = "calibration".to_owned();
        assert!(evaluate_joint_validation(&not_blind)
            .unwrap_err()
            .contains("blind_confirmation"));

        let mut reordered = study(0.0);
        reordered.observations[1].outputs.reverse();
        assert!(evaluate_joint_validation(&reordered)
            .unwrap_err()
            .contains("frozen quantity/unit order"));
    }

    #[test]
    fn singular_indefinite_and_asymmetric_correlations_are_rejected() {
        for matrix in [
            vec![vec![1.0, 1.0], vec![1.0, 1.0]],
            vec![vec![1.0, 1.2], vec![1.2, 1.0]],
            vec![vec![1.0, 0.2], vec![0.3, 1.0]],
        ] {
            let mut candidate = study(0.0);
            candidate.observations[0].correlation_matrix = matrix;
            assert!(evaluate_joint_validation(&candidate).is_err());
        }
    }

    #[test]
    fn digest_binds_uncertainty_source_and_report_replays() {
        let report = evaluate_joint_validation(&study(0.25)).unwrap();
        verify_joint_validation_report(&report).unwrap();
        let mut tampered = report.clone();
        tampered.observations[0].observation.uncertainty_source_sha256 = "b".repeat(64);
        assert!(verify_joint_validation_report(&tampered)
            .unwrap_err()
            .contains("observation"));
    }

    #[cfg(unix)]
    #[test]
    fn independent_python_oracle_replays_joint_metric_and_tamper_control() {
        let report = evaluate_joint_validation(&study(0.5)).unwrap();
        let csv = joint_validation_report_to_csv(&report).unwrap();
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "aero-multivariate-validation-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let csv_path = root.join("joint-validation.csv");
        std::fs::write(&csv_path, &csv).unwrap();
        let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("scripts/verify-joint-validation-independent.py");
        let output = std::process::Command::new("python3")
            .arg(&script)
            .arg(&csv_path)
            .output()
            .expect("execute independent joint-validation oracle");
        assert!(
            output.status.success(),
            "independent joint-validation oracle failed: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let tampered_csv = csv.replace(",1.1,1,0.1,", ",1.2,1,0.1,");
        std::fs::write(&csv_path, tampered_csv).unwrap();
        let tampered = std::process::Command::new("python3")
            .arg(&script)
            .arg(&csv_path)
            .output()
            .expect("execute oracle against tampered row");
        assert!(!tampered.status.success(), "tampered prediction must be rejected");
        std::fs::remove_dir_all(root).unwrap();
    }
}
