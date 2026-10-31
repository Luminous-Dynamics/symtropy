use std::process;

use civil_aero_benchmark::interval_calibration::DomainStatus;
use civil_aero_benchmark::selective_risk::{
    SelectivePrediction, audit_blind_selective_risk, report_to_csv,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("synthetic selective risk/coverage audit rejected: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut predictions = Vec::new();
    for model_id in ["error_ranked_uncertainty", "misranked_uncertainty"] {
        for domain_status in [DomainStatus::InDomain, DomainStatus::OutOfDomain] {
            let shift = if domain_status == DomainStatus::InDomain {
                1.0
            } else {
                2.0
            };
            for index in 0..20 {
                let observed_feasible = index < 15;
                let observed_value = if observed_feasible { 0.0 } else { 1.0 };
                let absolute_error = if index == 19 {
                    0.9
                } else {
                    (index + 1) as f64 * 0.01 * shift
                };
                let point_prediction = if index == 19 {
                    0.1
                } else if observed_feasible {
                    absolute_error
                } else {
                    observed_value + absolute_error
                };
                let predicted_feasible = observed_feasible || index == 19;
                let uncertainty_score = if model_id == "error_ranked_uncertainty" {
                    absolute_error
                } else {
                    1.0 - absolute_error
                };
                predictions.push(SelectivePrediction {
                    task_id: "synthetic-selective-task".to_owned(),
                    sample_id: format!("{}-{}", domain_status.as_str(), index),
                    model_id: model_id.to_owned(),
                    geometry_family: "synthetic-OAT15A-family".to_owned(),
                    flow_regime: "synthetic-transonic".to_owned(),
                    constraint_id: "energy_margin_nonnegative".to_owned(),
                    dataset_split: "blind_confirmation".to_owned(),
                    domain_status,
                    uncertainty_score,
                    point_prediction,
                    observed_value,
                    predicted_feasible,
                    observed_feasible,
                });
            }
        }
    }
    let report = audit_blind_selective_risk(&predictions)?;
    eprintln!(
        "synthetic_selective_risk_only models=2 domains=2 samples_per_group=20 curve_rows={} summaries={}",
        report.summaries.iter().map(|row| row.curve.len()).sum::<usize>(),
        report.summaries.len()
    );
    print!("{}", report_to_csv(&report));
    Ok(())
}
