use std::process;

use civil_aero_benchmark::cfd_admission::{
    AdmissionCriteria, CfdEvidenceStudy, CfdGridRun, evaluate_cfd_evidence_admission,
};
use civil_aero_benchmark::grid_convergence::GridFamilyKind;
use civil_aero_benchmark::validation_uncertainty::{
    GciCoverageModel, InputUncertaintyContribution, ValidationUncertaintyStudy,
    evaluate_validation_uncertainty, report_to_csv,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("synthetic validation uncertainty demo rejected: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let solver_sha = "a".repeat(64);
    let physics_sha = "b".repeat(64);
    let geometry_sha = "c".repeat(64);
    let values = [14.8, 5.2, 2.8];
    let spacings = [4.0, 2.0, 1.0];
    let nodes = [100_000, 200_000, 800_000];
    let runs = (0..3)
        .map(|index| CfdGridRun {
            run_id: format!("synthetic-run-{}", index + 1),
            grid_id: format!("L{}", index + 1),
            grid_level: (index + 1) as u32,
            characteristic_spacing: spacings[index],
            mesh_sha256: format!("{:064x}", index + 1),
            solver_build_sha256: solver_sha.clone(),
            physics_config_sha256: physics_sha.clone(),
            geometry_sha256: geometry_sha.clone(),
            solver_exit_code: 0,
            iterations_completed: 2_000,
            mesh_node_count: nodes[index],
            cpu_core_hours: 1.0,
            gpu_hours: None,
            wall_clock_seconds: 3_600.0,
            peak_memory_bytes: 4_000_000_000,
            energy_consumed_joules: None,
            residual_initial: 1.0,
            residual_final: 1.0e-8,
            quantity_history: vec![values[index]; 6],
            mass_imbalance_fraction: 1.0e-9,
            energy_imbalance_fraction: 1.0e-9,
            quantity_value: values[index],
        })
        .collect::<Vec<_>>();

    let admission = evaluate_cfd_evidence_admission(&CfdEvidenceStudy {
        case_id: "synthetic-oat15a-validation".to_owned(),
        quantity: "CD".to_owned(),
        source_reference: "synthetic-known-answer-validation-v0.1".to_owned(),
        spacing_definition: "synthetic h supplied explicitly".to_owned(),
        grid_family_kind: GridFamilyKind::Structured,
        safety_factor: 1.25,
        criteria: AdmissionCriteria {
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
        runs,
    })?;

    let study = ValidationUncertaintyStudy {
        case_id: admission.case_id.clone(),
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
        input_experiment_error_correlation: Some(0.25),
        aleatory_expansion_factor: 2.0,
    };
    let report = evaluate_validation_uncertainty(&admission, &study)?;
    eprintln!(
        "synthetic_validation_uncertainty_only status=numerically_admitted_not_experimentally_validated grids={} u_num={:.12} u_input={:.12} u_aleatory={} conservative_envelope={}",
        admission.grid_run_audits.len(),
        report.numerical_standard_uncertainty,
        report.input_standard_uncertainty,
        report
            .aleatory_standard_uncertainty
            .map(|value| format!("{value:.12}"))
            .unwrap_or_else(|| "not_combined_correlation_unset".to_owned()),
        report
            .conservative_total_bound
            .map(|value| format!("{value:.12}"))
            .unwrap_or_else(|| "not_available".to_owned())
    );
    print!("{}", report_to_csv(&report));
    Ok(())
}
