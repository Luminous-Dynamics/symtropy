use std::process;

use civil_aero_benchmark::interval_calibration::{
    DomainStatus, IntervalPrediction, audit_blind_prediction_intervals, report_to_csv,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("synthetic interval-calibration audit rejected: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let levels = [0.80_f64, 0.90, 0.95];
    let half_widths = [0.80_f64, 0.90, 0.95];
    let mut predictions = Vec::new();

    for model_id in ["nominal_model", "robust_model"] {
        for domain_status in [DomainStatus::InDomain, DomainStatus::OutOfDomain] {
            for index in 0..60 {
                let scale = if domain_status == DomainStatus::InDomain {
                    1.0
                } else {
                    2.0
                };
                let magnitude = (index + 1) as f64 / 60.0 * scale;
                let observed_value = if index % 2 == 0 {
                    magnitude
                } else {
                    -magnitude
                };
                for (level_index, nominal_coverage) in levels.iter().enumerate() {
                    let width_scale = if model_id == "robust_model"
                        && domain_status == DomainStatus::OutOfDomain
                    {
                        2.0
                    } else {
                        1.0
                    };
                    let half_width = half_widths[level_index] * width_scale;
                    predictions.push(IntervalPrediction {
                        task_id: "synthetic-calibration-task".to_owned(),
                        sample_id: format!("{}-{}", domain_status.as_str(), index),
                        model_id: model_id.to_owned(),
                        geometry_family: "synthetic-OAT15A-family".to_owned(),
                        flow_regime: "synthetic-transonic".to_owned(),
                        dataset_split: "blind_confirmation".to_owned(),
                        domain_status,
                        nominal_coverage: *nominal_coverage,
                        lower: -half_width,
                        upper: half_width,
                        point_prediction: 0.0,
                        observed_value,
                    });
                }
            }
        }
    }

    let report = audit_blind_prediction_intervals(&predictions)?;
    eprintln!(
        "blind_confirmation_synthetic_only models=2 domains=2 levels=3 prediction_rows={} summaries={}",
        predictions.len(),
        report.summaries.len()
    );
    print!("{}", report_to_csv(&report));
    Ok(())
}
