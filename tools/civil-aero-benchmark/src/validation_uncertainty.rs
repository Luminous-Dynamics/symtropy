//! Validation uncertainty budget separating numerical epistemic error from aleatory terms.
//!
//! This module binds validation comparison to the finest-grid value in a numerically admitted
//! CFD study. GCI is converted to a standard uncertainty only under an explicit distribution
//! assumption. Numerical uncertainty is conservatively summed; correlated input uncertainties
//! use a covariance matrix. Input and experimental uncertainties are combined only when their
//! correlation is supplied. The final linear envelope carries no joint coverage claim.

use crate::cfd_admission::CfdEvidenceAdmissionReport;
use std::f64;

const MATRIX_SYMMETRY_TOLERANCE: f64 = 1.0e-10;
const MATRIX_PSD_TOLERANCE: f64 = 1.0e-10;
const JACOBI_TOLERANCE: f64 = 1.0e-13;
const MAX_JACOBI_SWEEPS_FACTOR: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GciCoverageModel {
    Gaussian95,
    ShiftedGaussian95,
    Custom,
}

impl GciCoverageModel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Gaussian95 => "gaussian_95_k_2",
            Self::ShiftedGaussian95 => "shifted_gaussian_95_k_1_1_to_1_15",
            Self::Custom => "caller_supplied_custom_coverage_model",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct InputUncertaintyContribution {
    pub input_id: String,
    pub input_unit: String,
    pub source_reference: String,
    /// Local derivative d(quantity)/d(input), with output/input units.
    pub sensitivity_coefficient: f64,
    /// Standard uncertainty of the input parameter, in input_unit.
    pub standard_uncertainty: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidationUncertaintyStudy {
    pub case_id: String,
    pub quantity: String,
    pub output_unit: String,
    pub predicted_value: f64,
    pub measured_value: f64,
    /// Explicitly selected conversion of the finest-grid GCI to standard uncertainty.
    pub gci_coverage_model: GciCoverageModel,
    pub gci_coverage_factor: f64,
    /// Rationale is mandatory even when a standard convention is selected.
    pub gci_coverage_rationale: String,
    /// Numerical iterative and round-off standard uncertainties in output units.
    pub iterative_standard_uncertainty: f64,
    pub roundoff_standard_uncertainty: f64,
    /// Caller-supplied expansion factor for the conservative numerical epistemic envelope.
    pub numerical_expansion_factor: f64,
    pub input_contributions: Vec<InputUncertaintyContribution>,
    /// Ordered exactly like input_contributions; must be symmetric positive semidefinite.
    pub input_correlation_matrix: Vec<Vec<f64>>,
    /// Aggregate experimental standard uncertainty in output units.
    pub experimental_standard_uncertainty: f64,
    /// Immutable reference to the experimental uncertainty budget, calibration certificate, or report.
    pub experimental_uncertainty_source_reference: String,
    /// Method used to estimate this aggregate, including the included random/systematic terms.
    pub experimental_uncertainty_method: String,
    /// Explicitly declares that input and experimental uncertainty sources are aleatory.
    /// A correlation coefficient alone is not evidence of aleatory behaviour.
    pub input_and_experiment_are_aleatory: bool,
    pub aleatory_basis_rationale: String,
    /// Correlation between input-induced prediction error and experimental error in E = S - D.
    /// None means no combined aleatory uncertainty or discrepancy interval is reported.
    pub input_experiment_error_correlation: Option<f64>,
    /// Caller-selected expansion factor for the combined aleatory comparison component.
    pub aleatory_expansion_factor: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidationUncertaintyReport {
    pub case_id: String,
    pub quantity: String,
    pub output_unit: String,
    pub admission_status: &'static str,
    pub source_reference: String,
    pub gci_coarse_grid_id: String,
    pub gci_medium_grid_id: String,
    pub gci_fine_grid_id: String,
    pub gci_fine_absolute: f64,
    pub gci_coverage_model: GciCoverageModel,
    pub gci_coverage_factor: f64,
    pub gci_standard_uncertainty: f64,
    pub iterative_standard_uncertainty: f64,
    pub roundoff_standard_uncertainty: f64,
    /// Epistemic numerical contributions are added conservatively, not RSS-combined.
    pub numerical_standard_uncertainty: f64,
    pub numerical_expansion_factor: f64,
    pub numerical_expanded_bound: f64,
    pub input_standard_uncertainty: f64,
    pub experimental_standard_uncertainty: f64,
    pub experimental_uncertainty_source_reference: String,
    pub experimental_uncertainty_method: String,
    pub input_and_experiment_are_aleatory: bool,
    pub aleatory_basis_rationale: String,
    pub input_experiment_error_correlation: Option<f64>,
    pub aleatory_standard_uncertainty: Option<f64>,
    pub aleatory_expansion_factor: f64,
    pub aleatory_expanded_bound: Option<f64>,
    /// Linear sum of the expanded numerical and aleatory bounds. This is not a joint-coverage interval.
    pub conservative_total_bound: Option<f64>,
    pub comparison_error: f64,
    pub model_discrepancy_lower: Option<f64>,
    pub model_discrepancy_upper: Option<f64>,
    pub input_contributions: Vec<InputUncertaintyContribution>,
    pub input_correlation_matrix: Vec<Vec<f64>>,
    pub input_correlation_min_eigenvalue: f64,
    pub evidence_level: &'static str,
}

pub fn evaluate_validation_uncertainty(
    admission: &CfdEvidenceAdmissionReport,
    study: &ValidationUncertaintyStudy,
) -> Result<ValidationUncertaintyReport, String> {
    const ADMITTED_STATUS: &str = "numerically_admitted_under_declared_criteria_caller_hashes_unverified_not_experimentally_validated";

    if admission.status != ADMITTED_STATUS {
        return Err("validation uncertainty requires an admitted numerical CFD study".to_owned());
    }
    if study.case_id.trim().is_empty()
        || study.quantity.trim().is_empty()
        || study.output_unit.trim().is_empty()
        || study.gci_coverage_rationale.trim().is_empty()
        || study
            .experimental_uncertainty_source_reference
            .trim()
            .is_empty()
        || study.experimental_uncertainty_method.trim().is_empty()
        || study
            .experimental_uncertainty_source_reference
            .contains('|')
        || study.experimental_uncertainty_method.contains('|')
        || (study.input_and_experiment_are_aleatory
            && study.aleatory_basis_rationale.trim().is_empty())
    {
        return Err(
            "case_id, quantity, output_unit, GCI coverage rationale, and experimental uncertainty source and method must be non-empty"
                .to_owned(),
        );
    }
    if study.case_id != admission.case_id || study.quantity != admission.quantity {
        return Err("validation case or quantity differs from numerical admission".to_owned());
    }
    if study.quantity != admission.gci_report.quantity
        || admission.gci_report.source_reference != admission.source_reference
    {
        return Err("validation quantity/source differs from the admitted GCI report".to_owned());
    }
    if !study.predicted_value.is_finite() || !study.measured_value.is_finite() {
        return Err("predicted and measured values must be finite".to_owned());
    }
    for (name, value) in [
        (
            "iterative_standard_uncertainty",
            study.iterative_standard_uncertainty,
        ),
        (
            "roundoff_standard_uncertainty",
            study.roundoff_standard_uncertainty,
        ),
        (
            "experimental_standard_uncertainty",
            study.experimental_standard_uncertainty,
        ),
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(format!("{name} must be finite and non-negative"));
        }
    }
    for (name, value) in [
        (
            "numerical_expansion_factor",
            study.numerical_expansion_factor,
        ),
        ("aleatory_expansion_factor", study.aleatory_expansion_factor),
        ("gci_coverage_factor", study.gci_coverage_factor),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(format!("{name} must be finite and positive"));
        }
    }
    match study.gci_coverage_model {
        GciCoverageModel::Gaussian95 => {
            if (study.gci_coverage_factor - 2.0).abs() > 1.0e-12 {
                return Err("Gaussian95 GCI conversion requires coverage factor 2.0".to_owned());
            }
        }
        GciCoverageModel::ShiftedGaussian95 => {
            if !(1.1..=1.15).contains(&study.gci_coverage_factor) {
                return Err(
                    "ShiftedGaussian95 GCI conversion requires coverage factor in [1.10, 1.15]"
                        .to_owned(),
                );
            }
        }
        GciCoverageModel::Custom => {}
    }

    let triplet = admission
        .gci_report
        .triplets
        .last()
        .ok_or_else(|| "admitted GCI report contains no grid triplets".to_owned())?;
    if study.predicted_value.to_bits() != triplet.phi_fine.to_bits() {
        return Err(
            "predicted validation value must exactly match the finest GCI grid value".to_owned(),
        );
    }
    if !triplet.fine_gci_absolute.is_finite() || triplet.fine_gci_absolute < 0.0 {
        return Err("finest-grid GCI must be finite and non-negative".to_owned());
    }

    let input_standard_uncertainty =
        propagate_input_uncertainty(&study.input_contributions, &study.input_correlation_matrix)?;
    let input_correlation_min_eigenvalue =
        minimum_eigenvalue_symmetric(&study.input_correlation_matrix)?;

    let gci_standard_uncertainty = triplet.fine_gci_absolute / study.gci_coverage_factor;
    let numerical_standard_uncertainty = gci_standard_uncertainty
        + study.iterative_standard_uncertainty
        + study.roundoff_standard_uncertainty;
    let numerical_expanded_bound =
        study.numerical_expansion_factor * numerical_standard_uncertainty;
    let comparison_error = study.predicted_value - study.measured_value;

    for (name, value) in [
        ("GCI standard uncertainty", gci_standard_uncertainty),
        (
            "numerical standard uncertainty",
            numerical_standard_uncertainty,
        ),
        ("numerical expanded bound", numerical_expanded_bound),
        ("comparison error", comparison_error),
        ("input standard uncertainty", input_standard_uncertainty),
    ] {
        if !value.is_finite() {
            return Err(format!("{name} overflowed to a non-finite value"));
        }
    }

    let (aleatory_standard_uncertainty, aleatory_expanded_bound, conservative_total_bound) =
        if study.input_and_experiment_are_aleatory {
            let correlation = study.input_experiment_error_correlation.ok_or_else(|| {
                "aleatory input/experiment combination requires an explicit correlation".to_owned()
            })?;
            if !correlation.is_finite() || !(-1.0..=1.0).contains(&correlation) {
                return Err(
                    "input/experiment error correlation must be finite and in [-1, 1]".to_owned(),
                );
            }
            let variance = input_standard_uncertainty.powi(2)
                + study.experimental_standard_uncertainty.powi(2)
                - 2.0
                    * correlation
                    * input_standard_uncertainty
                    * study.experimental_standard_uncertainty;
            if !variance.is_finite() || variance < -MATRIX_PSD_TOLERANCE {
                return Err("input/experiment covariance produced a negative variance".to_owned());
            }
            let aleatory = variance.max(0.0).sqrt();
            let expanded = study.aleatory_expansion_factor * aleatory;
            let total = numerical_expanded_bound + expanded;
            if !aleatory.is_finite() || !expanded.is_finite() || !total.is_finite() {
                return Err("combined uncertainty envelope overflowed".to_owned());
            }
            (Some(aleatory), Some(expanded), Some(total))
        } else {
            (None, None, None)
        };

    let (model_discrepancy_lower, model_discrepancy_upper) =
        if let Some(bound) = conservative_total_bound {
            let lower = comparison_error - bound;
            let upper = comparison_error + bound;
            if !lower.is_finite() || !upper.is_finite() {
                return Err("model-discrepancy envelope overflowed".to_owned());
            }
            (Some(lower), Some(upper))
        } else {
            (None, None)
        };

    Ok(ValidationUncertaintyReport {
        case_id: study.case_id.clone(),
        quantity: study.quantity.clone(),
        output_unit: study.output_unit.clone(),
        admission_status: admission.status,
        source_reference: admission.source_reference.clone(),
        gci_coarse_grid_id: triplet.coarse_grid_id.clone(),
        gci_medium_grid_id: triplet.medium_grid_id.clone(),
        gci_fine_grid_id: triplet.fine_grid_id.clone(),
        gci_fine_absolute: triplet.fine_gci_absolute,
        gci_coverage_model: study.gci_coverage_model,
        gci_coverage_factor: study.gci_coverage_factor,
        gci_standard_uncertainty,
        iterative_standard_uncertainty: study.iterative_standard_uncertainty,
        roundoff_standard_uncertainty: study.roundoff_standard_uncertainty,
        numerical_standard_uncertainty,
        numerical_expansion_factor: study.numerical_expansion_factor,
        numerical_expanded_bound,
        input_standard_uncertainty,
        experimental_standard_uncertainty: study.experimental_standard_uncertainty,
        experimental_uncertainty_source_reference: study
            .experimental_uncertainty_source_reference
            .clone(),
        experimental_uncertainty_method: study.experimental_uncertainty_method.clone(),
        input_and_experiment_are_aleatory: study.input_and_experiment_are_aleatory,
        aleatory_basis_rationale: study.aleatory_basis_rationale.clone(),
        input_experiment_error_correlation: study.input_experiment_error_correlation,
        aleatory_standard_uncertainty,
        aleatory_expansion_factor: study.aleatory_expansion_factor,
        aleatory_expanded_bound,
        conservative_total_bound,
        comparison_error,
        model_discrepancy_lower,
        model_discrepancy_upper,
        input_contributions: study.input_contributions.clone(),
        input_correlation_matrix: study.input_correlation_matrix.clone(),
        input_correlation_min_eigenvalue,
        evidence_level: "caller_supplied_uq_unsigned_conservative_linear_envelope_no_joint_coverage_claim",
    })
}

fn propagate_input_uncertainty(
    contributions: &[InputUncertaintyContribution],
    correlation: &[Vec<f64>],
) -> Result<f64, String> {
    if contributions.is_empty() {
        return Err("at least one explicit input uncertainty contribution is required".to_owned());
    }
    let count = contributions.len();
    if correlation.len() != count || correlation.iter().any(|row| row.len() != count) {
        return Err(
            "input correlation matrix dimensions must match input contributions".to_owned(),
        );
    }
    let mut propagated = Vec::with_capacity(count);
    let mut seen = std::collections::HashSet::new();
    for contribution in contributions {
        if contribution.input_id.trim().is_empty()
            || contribution.input_id.contains('|')
            || contribution.input_unit.trim().is_empty()
            || contribution.input_unit.contains('|')
            || contribution.source_reference.trim().is_empty()
            || contribution.source_reference.contains('|')
        {
            return Err(
                "input IDs, units, and source references must be non-empty and CSV-safe".to_owned(),
            );
        }
        if !seen.insert(contribution.input_id.clone()) {
            return Err(format!(
                "duplicate input uncertainty ID {}",
                contribution.input_id
            ));
        }
        if !contribution.sensitivity_coefficient.is_finite()
            || !contribution.standard_uncertainty.is_finite()
            || contribution.standard_uncertainty < 0.0
        {
            return Err(format!(
                "input {} has non-finite sensitivity or invalid standard uncertainty",
                contribution.input_id
            ));
        }
        let value = contribution.sensitivity_coefficient * contribution.standard_uncertainty;
        if !value.is_finite() {
            return Err(format!(
                "input {} sensitivity contribution overflowed",
                contribution.input_id
            ));
        }
        propagated.push(value);
    }

    minimum_eigenvalue_symmetric(correlation)?;
    let mut variance = 0.0;
    for row in 0..count {
        for column in 0..count {
            variance += propagated[row] * correlation[row][column] * propagated[column];
        }
    }
    if !variance.is_finite() || variance < -MATRIX_PSD_TOLERANCE {
        return Err(
            "input covariance propagation produced a negative or non-finite variance".to_owned(),
        );
    }
    Ok(variance.max(0.0).sqrt())
}

/// Return the minimum eigenvalue after a symmetric Jacobi diagonalization.
fn minimum_eigenvalue_symmetric(matrix: &[Vec<f64>]) -> Result<f64, String> {
    let count = matrix.len();
    if count == 0 || matrix.iter().any(|row| row.len() != count) {
        return Err("correlation matrix must be non-empty and square".to_owned());
    }
    let mut values = matrix.to_vec();
    for row in 0..count {
        for column in 0..count {
            let value = values[row][column];
            if !value.is_finite() || !(-1.0..=1.0).contains(&value) {
                return Err("correlation matrix entries must be finite and in [-1, 1]".to_owned());
            }
            if (values[row][column] - values[column][row]).abs() > MATRIX_SYMMETRY_TOLERANCE {
                return Err("correlation matrix must be symmetric".to_owned());
            }
        }
        if (values[row][row] - 1.0).abs() > MATRIX_SYMMETRY_TOLERANCE {
            return Err("correlation matrix diagonal entries must equal 1".to_owned());
        }
    }

    let max_iterations = MAX_JACOBI_SWEEPS_FACTOR * count * count;
    for _ in 0..max_iterations {
        let mut max_off_diagonal = 0.0;
        let mut pivot = None;
        for row in 0..count {
            for column in (row + 1)..count {
                let magnitude = values[row][column].abs();
                if magnitude > max_off_diagonal {
                    max_off_diagonal = magnitude;
                    pivot = Some((row, column));
                }
            }
        }
        if max_off_diagonal <= JACOBI_TOLERANCE {
            break;
        }
        let (p, q) = pivot.expect("non-zero off-diagonal entry has a pivot");
        let app = values[p][p];
        let aqq = values[q][q];
        let apq = values[p][q];
        let theta = 0.5 * (2.0 * apq).atan2(aqq - app);
        let cosine = theta.cos();
        let sine = theta.sin();

        for index in 0..count {
            if index == p || index == q {
                continue;
            }
            let aip = values[index][p];
            let aiq = values[index][q];
            let new_aip = cosine * aip - sine * aiq;
            let new_aiq = sine * aip + cosine * aiq;
            values[index][p] = new_aip;
            values[p][index] = new_aip;
            values[index][q] = new_aiq;
            values[q][index] = new_aiq;
        }
        values[p][p] = cosine * cosine * app - 2.0 * sine * cosine * apq + sine * sine * aqq;
        values[q][q] = sine * sine * app + 2.0 * sine * cosine * apq + cosine * cosine * aqq;
        values[p][q] = 0.0;
        values[q][p] = 0.0;
    }

    let min_eigenvalue = (0..count)
        .map(|index| values[index][index])
        .fold(f64::INFINITY, f64::min);
    if !min_eigenvalue.is_finite() || min_eigenvalue < -MATRIX_PSD_TOLERANCE {
        return Err(format!(
            "input correlation matrix is not positive semidefinite: minimum eigenvalue {min_eigenvalue}"
        ));
    }
    Ok(min_eigenvalue)
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn number_list(values: impl IntoIterator<Item = f64>) -> String {
    values
        .into_iter()
        .map(|value| format!("{value:.12}"))
        .collect::<Vec<_>>()
        .join("|")
}

fn matrix_string(matrix: &[Vec<f64>]) -> String {
    matrix
        .iter()
        .map(|row| number_list(row.iter().copied()))
        .collect::<Vec<_>>()
        .join(";")
}

pub const VALIDATION_UNCERTAINTY_CSV_HEADER: &str = "case_id,quantity,output_unit,admission_status,source_reference,coarse_grid_id,medium_grid_id,fine_grid_id,gci_fine_absolute,gci_coverage_model,gci_coverage_factor,gci_standard_uncertainty,iterative_standard_uncertainty,roundoff_standard_uncertainty,numerical_standard_uncertainty,numerical_expansion_factor,numerical_expanded_bound,input_standard_uncertainty,experimental_standard_uncertainty,input_experiment_error_correlation,aleatory_standard_uncertainty,aleatory_expansion_factor,aleatory_expanded_bound,conservative_total_bound,comparison_error,model_discrepancy_lower,model_discrepancy_upper,input_correlation_matrix,input_correlation_min_eigenvalue,input_component_ids,input_component_units,input_component_source_refs,input_sensitivity_coefficients,input_standard_uncertainties,experimental_uncertainty_source_reference,experimental_uncertainty_method,input_and_experiment_are_aleatory,aleatory_basis_rationale,evidence_level";

pub fn report_to_csv(report: &ValidationUncertaintyReport) -> String {
    let optional = |value: Option<f64>| {
        value
            .map(|number| format!("{number:.12}"))
            .unwrap_or_default()
    };
    let fields = vec![
        csv_escape(&report.case_id),
        csv_escape(&report.quantity),
        csv_escape(&report.output_unit),
        csv_escape(report.admission_status),
        csv_escape(&report.source_reference),
        csv_escape(&report.gci_coarse_grid_id),
        csv_escape(&report.gci_medium_grid_id),
        csv_escape(&report.gci_fine_grid_id),
        format!("{:.12}", report.gci_fine_absolute),
        report.gci_coverage_model.as_str().to_owned(),
        format!("{:.12}", report.gci_coverage_factor),
        format!("{:.12}", report.gci_standard_uncertainty),
        format!("{:.12}", report.iterative_standard_uncertainty),
        format!("{:.12}", report.roundoff_standard_uncertainty),
        format!("{:.12}", report.numerical_standard_uncertainty),
        format!("{:.12}", report.numerical_expansion_factor),
        format!("{:.12}", report.numerical_expanded_bound),
        format!("{:.12}", report.input_standard_uncertainty),
        format!("{:.12}", report.experimental_standard_uncertainty),
        optional(report.input_experiment_error_correlation),
        optional(report.aleatory_standard_uncertainty),
        format!("{:.12}", report.aleatory_expansion_factor),
        optional(report.aleatory_expanded_bound),
        optional(report.conservative_total_bound),
        format!("{:.12}", report.comparison_error),
        optional(report.model_discrepancy_lower),
        optional(report.model_discrepancy_upper),
        csv_escape(&matrix_string(&report.input_correlation_matrix)),
        format!("{:.12}", report.input_correlation_min_eigenvalue),
        csv_escape(
            &report
                .input_contributions
                .iter()
                .map(|item| item.input_id.as_str())
                .collect::<Vec<_>>()
                .join("|"),
        ),
        csv_escape(
            &report
                .input_contributions
                .iter()
                .map(|item| item.input_unit.as_str())
                .collect::<Vec<_>>()
                .join("|"),
        ),
        csv_escape(
            &report
                .input_contributions
                .iter()
                .map(|item| item.source_reference.as_str())
                .collect::<Vec<_>>()
                .join("|"),
        ),
        csv_escape(number_list(
            report
                .input_contributions
                .iter()
                .map(|item| item.sensitivity_coefficient),
        )),
        csv_escape(number_list(
            report
                .input_contributions
                .iter()
                .map(|item| item.standard_uncertainty),
        )),
        csv_escape(&report.experimental_uncertainty_source_reference),
        csv_escape(&report.experimental_uncertainty_method),
        report.input_and_experiment_are_aleatory.to_string(),
        csv_escape(&report.aleatory_basis_rationale),
        report.evidence_level.to_owned(),
    ];
    format!(
        "{VALIDATION_UNCERTAINTY_CSV_HEADER}\n{}\n",
        fields.join(",")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfd_admission::CfdEvidenceAdmissionReport;
    use crate::grid_convergence::{
        GciReport, GciStudy, GridFamilyKind, GridObservation, evaluate_grid_convergence,
    };

    fn admission() -> CfdEvidenceAdmissionReport {
        let gci = evaluate_grid_convergence(&GciStudy {
            quantity: "CD".to_owned(),
            source_reference: "synthetic-oat15a-known-answer-v0.1".to_owned(),
            spacing_definition: "synthetic h values supplied explicitly".to_owned(),
            grid_family_kind: GridFamilyKind::Structured,
            safety_factor: 1.25,
            observations: vec![
                GridObservation {
                    grid_id: "L1".to_owned(),
                    characteristic_spacing: 4.0,
                    value: 14.8,
                },
                GridObservation {
                    grid_id: "L2".to_owned(),
                    characteristic_spacing: 2.0,
                    value: 5.2,
                },
                GridObservation {
                    grid_id: "L3".to_owned(),
                    characteristic_spacing: 1.0,
                    value: 2.8,
                },
            ],
        })
        .expect("synthetic GCI");
        CfdEvidenceAdmissionReport {
            case_id: "synthetic-oat15a".to_owned(),
            quantity: "CD".to_owned(),
            source_reference: "synthetic-oat15a-known-answer-v0.1".to_owned(),
            spacing_definition: "synthetic h values supplied explicitly".to_owned(),
            solver_build_sha256: "a".repeat(64),
            physics_config_sha256: "b".repeat(64),
            geometry_sha256: "c".repeat(64),
            grid_family_kind: GridFamilyKind::Structured,
            safety_factor: 1.25,
            criteria: crate::cfd_admission::AdmissionCriteria {
                residual_final_max: 1.0e-6,
                residual_reduction_ratio_max: 1.0e-6,
                quantity_history_drift_absolute_max: 1.0e-5,
                mass_imbalance_fraction_max: 1.0e-7,
                energy_imbalance_fraction_max: 1.0e-7,
                observed_order_min: 1.0,
                observed_order_max: 4.0,
                asymptotic_ratio_min: 0.8,
                asymptotic_ratio_max: 1.2,
            },
            status: "numerically_admitted_under_declared_criteria_caller_hashes_unverified_not_experimentally_validated",
            grid_run_audits: Vec::new(),
            total_cpu_core_hours: 3.0,
            total_gpu_hours_measured: None,
            gpu_measured_run_count: 0,
            total_wall_clock_seconds: 10800.0,
            peak_memory_bytes: 1_000_000,
            total_energy_joules_measured: None,
            energy_measured_run_count: 0,
            gci_report: gci,
        }
    }

    fn base_study() -> ValidationUncertaintyStudy {
        ValidationUncertaintyStudy {
            case_id: "synthetic-oat15a".to_owned(),
            quantity: "CD".to_owned(),
            output_unit: "dimensionless".to_owned(),
            predicted_value: 2.8,
            measured_value: 2.75,
            gci_coverage_model: GciCoverageModel::Gaussian95,
            gci_coverage_factor: 2.0,
            gci_coverage_rationale: "NIST Gaussian-about-fine-grid convention".to_owned(),
            iterative_standard_uncertainty: 0.02,
            roundoff_standard_uncertainty: 0.005,
            numerical_expansion_factor: 2.0,
            input_contributions: vec![
                InputUncertaintyContribution {
                    input_id: "Mach".to_owned(),
                    input_unit: "1".to_owned(),
                    source_reference: "synthetic-input-uncertainty".to_owned(),
                    sensitivity_coefficient: 2.0,
                    standard_uncertainty: 0.1,
                },
                InputUncertaintyContribution {
                    input_id: "Re".to_owned(),
                    input_unit: "1".to_owned(),
                    source_reference: "synthetic-input-uncertainty".to_owned(),
                    sensitivity_coefficient: -1.5,
                    standard_uncertainty: 0.2,
                },
            ],
            input_correlation_matrix: vec![vec![1.0, 0.4], vec![0.4, 1.0]],
            experimental_standard_uncertainty: 0.15,
            experimental_uncertainty_source_reference: "synthetic-experiment-budget-v0.1"
                .to_owned(),
            experimental_uncertainty_method: "synthetic repeatability and calibration aggregate"
                .to_owned(),
            input_and_experiment_are_aleatory: true,
            aleatory_basis_rationale: "synthetic declared aleatory sources".to_owned(),
            input_experiment_error_correlation: Some(0.25),
            aleatory_expansion_factor: 2.0,
        }
    }

    #[test]
    fn separates_epistemic_numeric_sum_from_covariance_propagated_aleatory_terms() {
        let report = evaluate_validation_uncertainty(&admission(), &base_study())
            .expect("complete uncertainty budget");
        assert!((report.gci_fine_absolute - 1.0).abs() < 1.0e-10);
        assert!((report.gci_standard_uncertainty - 0.5).abs() < 1.0e-10);
        assert!((report.numerical_standard_uncertainty - 0.525).abs() < 1.0e-10);
        assert!((report.input_standard_uncertainty - 0.082_f64.sqrt()).abs() < 1.0e-10);
        let input = report.input_standard_uncertainty;
        let expected_aleatory =
            (input * input + 0.15_f64.powi(2) - 2.0 * 0.25 * input * 0.15).sqrt();
        assert!(
            (report.aleatory_standard_uncertainty.unwrap() - expected_aleatory).abs() < 1.0e-10
        );
        assert!((report.numerical_expanded_bound - 1.05).abs() < 1.0e-10);
        assert!(
            (report.conservative_total_bound.unwrap() - (1.05 + 2.0 * expected_aleatory)).abs()
                < 1.0e-10
        );
        assert_eq!(
            report.evidence_level,
            "caller_supplied_uq_unsigned_conservative_linear_envelope_no_joint_coverage_claim"
        );
    }

    #[test]
    fn absent_input_experiment_correlation_does_not_become_independence_or_zero() {
        let mut study = base_study();
        study.input_and_experiment_are_aleatory = false;
        study.aleatory_basis_rationale.clear();
        study.input_experiment_error_correlation = None;
        let report = evaluate_validation_uncertainty(&admission(), &study)
            .expect("unclassified uncertainty components remain separate");
        assert!(report.aleatory_standard_uncertainty.is_none());
        assert!(report.aleatory_expanded_bound.is_none());
        assert!(report.conservative_total_bound.is_none());
        assert!(report.model_discrepancy_lower.is_none());
        assert!(report.model_discrepancy_upper.is_none());
    }

    #[test]
    fn correlation_alone_does_not_authorize_aleatory_combination() {
        let mut study = base_study();
        study.input_and_experiment_are_aleatory = false;
        study.aleatory_basis_rationale.clear();
        let report = evaluate_validation_uncertainty(&admission(), &study)
            .expect("unclassified components remain separate");
        assert!(report.input_experiment_error_correlation.is_some());
        assert!(report.aleatory_standard_uncertainty.is_none());
        assert!(report.conservative_total_bound.is_none());
    }

    #[test]
    fn aleatory_combination_without_explicit_correlation_is_rejected() {
        let mut study = base_study();
        study.input_experiment_error_correlation = None;
        let error = evaluate_validation_uncertainty(&admission(), &study)
            .expect_err("declared aleatory combination requires rho");
        assert!(error.contains("requires an explicit correlation"));
    }

    #[test]
    fn input_correlation_matrix_must_be_positive_semidefinite() {
        let mut study = base_study();
        study.input_correlation_matrix = vec![
            vec![1.0, 1.0, 1.0],
            vec![1.0, 1.0, -1.0],
            vec![1.0, -1.0, 1.0],
        ];
        study
            .input_contributions
            .push(InputUncertaintyContribution {
                input_id: "temperature".to_owned(),
                input_unit: "K".to_owned(),
                source_reference: "synthetic-input-uncertainty".to_owned(),
                sensitivity_coefficient: 0.1,
                standard_uncertainty: 2.0,
            });
        let error = evaluate_validation_uncertainty(&admission(), &study)
            .expect_err("non-PSD covariance must fail");
        assert!(error.contains("not positive semidefinite"));
    }

    #[test]
    fn asymmetric_matrix_and_invalid_gci_conversion_are_rejected() {
        let mut study = base_study();
        study.input_correlation_matrix[0][1] = 0.7;
        assert!(
            evaluate_validation_uncertainty(&admission(), &study)
                .expect_err("asymmetric matrix rejected")
                .contains("symmetric")
        );

        let mut study = base_study();
        study.gci_coverage_factor = 1.8;
        assert!(
            evaluate_validation_uncertainty(&admission(), &study)
                .expect_err("wrong Gaussian factor rejected")
                .contains("Gaussian95")
        );
    }

    #[test]
    fn predicted_value_must_bind_to_finest_admitted_gci_grid() {
        let mut study = base_study();
        study.predicted_value += 1.0e-10;
        assert!(
            evaluate_validation_uncertainty(&admission(), &study)
                .expect_err("prediction not matching finest GCI grid rejected")
                .contains("finest GCI grid value")
        );
    }

    #[test]
    fn experimental_uncertainty_without_source_or_method_is_rejected() {
        let mut study = base_study();
        study.experimental_uncertainty_source_reference.clear();
        assert!(
            evaluate_validation_uncertainty(&admission(), &study)
                .expect_err("missing experimental source must fail")
                .contains("experimental uncertainty source and method")
        );
        let mut study = base_study();
        study.experimental_uncertainty_method.clear();
        assert!(
            evaluate_validation_uncertainty(&admission(), &study)
                .expect_err("missing experimental method must fail")
                .contains("experimental uncertainty source and method")
        );
    }

    #[test]
    fn shifted_gaussian_conversion_requires_documented_factor_interval() {
        let mut study = base_study();
        study.gci_coverage_model = GciCoverageModel::ShiftedGaussian95;
        study.gci_coverage_factor = 1.125;
        assert!(evaluate_validation_uncertainty(&admission(), &study).is_ok());
        study.gci_coverage_factor = 1.2;
        assert!(
            evaluate_validation_uncertainty(&admission(), &study)
                .expect_err("shifted Gaussian factor outside range rejected")
                .contains("[1.10, 1.15]")
        );
    }

    #[test]
    fn rejects_non_admitted_case_and_quantity_mismatch() {
        let mut report = admission();
        report.status = "screening_only";
        assert!(evaluate_validation_uncertainty(&report, &base_study()).is_err());

        let mut study = base_study();
        study.quantity = "CL".to_owned();
        assert!(evaluate_validation_uncertainty(&admission(), &study).is_err());
    }

    #[test]
    fn report_csv_is_deterministic_and_retains_input_covariance_lineage() {
        let report = evaluate_validation_uncertainty(&admission(), &base_study())
            .expect("complete uncertainty budget");
        let first = report_to_csv(&report);
        assert_eq!(first, report_to_csv(&report));
        assert!(first.contains("Mach|Re"));
        assert!(first.contains("1.000000000000|0.400000000000;0.400000000000|1.000000000000"));
        assert!(first.contains("true,\"synthetic declared aleatory sources\""));
        assert_eq!(first.lines().count(), 2);
    }
}
