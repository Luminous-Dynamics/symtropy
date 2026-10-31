//! Blind-set selective prediction risk/coverage and false-feasible audit.
//!
//! The auditor ranks samples by caller-supplied uncertainty and simulates abstention thresholds.
//! It requires blind_confirmation data, identical sample/constraint sets across compared models,
//! and identical observed truth across models. This evaluates uncertainty ranking and selective
//! decision risk; it does not establish future coverage or authenticate the observations.

use std::collections::{BTreeMap, BTreeSet};

use crate::interval_calibration::DomainStatus;

#[derive(Debug, Clone, PartialEq)]
pub struct SelectivePrediction {
    pub task_id: String,
    pub sample_id: String,
    pub model_id: String,
    pub geometry_family: String,
    pub flow_regime: String,
    pub constraint_id: String,
    pub dataset_split: String,
    pub domain_status: DomainStatus,
    pub uncertainty_score: f64,
    pub point_prediction: f64,
    pub observed_value: f64,
    pub predicted_feasible: bool,
    pub observed_feasible: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RiskCoveragePoint {
    pub accepted_count: usize,
    pub sample_count: usize,
    pub uncertainty_threshold: f64,
    pub coverage_fraction: f64,
    pub selective_mae: f64,
    pub selective_rmse: f64,
    pub false_feasible_count: usize,
    /// False-feasible decisions divided by all accepted samples.
    pub false_feasible_rate: f64,
    pub predicted_feasible_count: usize,
    /// False-feasible decisions divided by accepted predictions labelled feasible.
    pub false_feasible_rate_per_predicted_feasible: Option<f64>,
    pub missed_feasible_count: usize,
    /// Running area under the MAE risk-coverage curve from zero coverage to this point.
    pub partial_area_under_mae_risk_coverage: f64,
    /// Running area under the false-feasible risk-coverage curve from zero to this point.
    pub partial_area_under_false_feasible_risk_coverage: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectiveRiskSummary {
    pub task_id: String,
    pub model_id: String,
    pub geometry_family: String,
    pub flow_regime: String,
    pub constraint_id: String,
    pub domain_status: DomainStatus,
    pub sample_count: usize,
    pub fully_accepted_mae: f64,
    pub fully_accepted_rmse: f64,
    pub fully_accepted_false_feasible_count: usize,
    pub fully_accepted_false_feasible_rate: f64,
    pub area_under_mae_risk_coverage: f64,
    pub area_under_false_feasible_risk_coverage: f64,
    pub curve: Vec<RiskCoveragePoint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectiveRiskReport {
    pub dataset_split: &'static str,
    pub evidence_level: &'static str,
    pub summaries: Vec<SelectiveRiskSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct GroupKey {
    task_id: String,
    model_id: String,
    geometry_family: String,
    flow_regime: String,
    constraint_id: String,
    domain_status: DomainStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ComparisonKey {
    task_id: String,
    geometry_family: String,
    flow_regime: String,
    constraint_id: String,
    domain_status: DomainStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct TruthKey {
    task_id: String,
    sample_id: String,
    geometry_family: String,
    flow_regime: String,
    constraint_id: String,
    domain_status: DomainStatus,
}

#[derive(Debug, Clone)]
struct Accumulator {
    sample_id: String,
    uncertainty_score: f64,
    absolute_error: f64,
    squared_error: f64,
    false_feasible: bool,
    predicted_feasible: bool,
    missed_feasible: bool,
}

pub fn audit_blind_selective_risk(
    predictions: &[SelectivePrediction],
) -> Result<SelectiveRiskReport, String> {
    if predictions.is_empty() {
        return Err("selective risk audit requires at least one blind sample".to_owned());
    }

    let mut groups: BTreeMap<GroupKey, Vec<Accumulator>> = BTreeMap::new();
    let mut truth_by_sample: BTreeMap<TruthKey, (u64, bool)> = BTreeMap::new();
    let mut comparison_sets: BTreeMap<ComparisonKey, BTreeMap<String, BTreeSet<String>>> =
        BTreeMap::new();
    let mut seen_model_samples = BTreeSet::new();

    for (index, prediction) in predictions.iter().enumerate() {
        validate_prediction(prediction, index)?;
        let truth_key = TruthKey {
            task_id: prediction.task_id.clone(),
            sample_id: prediction.sample_id.clone(),
            geometry_family: prediction.geometry_family.clone(),
            flow_regime: prediction.flow_regime.clone(),
            constraint_id: prediction.constraint_id.clone(),
            domain_status: prediction.domain_status,
        };
        if let Some(previous) = truth_by_sample.insert(
            truth_key,
            (prediction.observed_value.to_bits(), prediction.observed_feasible),
        ) {
            if previous != (prediction.observed_value.to_bits(), prediction.observed_feasible) {
                return Err(format!(
                    "sample {} changes observed value or feasibility truth across models",
                    prediction.sample_id
                ));
            }
        }

        let group_key = GroupKey {
            task_id: prediction.task_id.clone(),
            model_id: prediction.model_id.clone(),
            geometry_family: prediction.geometry_family.clone(),
            flow_regime: prediction.flow_regime.clone(),
            constraint_id: prediction.constraint_id.clone(),
            domain_status: prediction.domain_status,
        };
        if !seen_model_samples.insert((group_key.clone(), prediction.sample_id.clone())) {
            return Err(format!(
                "model {} has duplicate blind sample {}",
                prediction.model_id, prediction.sample_id
            ));
        }
        let comparison_key = ComparisonKey {
            task_id: prediction.task_id.clone(),
            geometry_family: prediction.geometry_family.clone(),
            flow_regime: prediction.flow_regime.clone(),
            constraint_id: prediction.constraint_id.clone(),
            domain_status: prediction.domain_status,
        };
        comparison_sets
            .entry(comparison_key)
            .or_default()
            .entry(prediction.model_id.clone())
            .or_default()
            .insert(prediction.sample_id.clone());

        let error = prediction.point_prediction - prediction.observed_value;
        groups.entry(group_key).or_default().push(Accumulator {
            sample_id: prediction.sample_id.clone(),
            uncertainty_score: prediction.uncertainty_score,
            absolute_error: error.abs(),
            squared_error: error * error,
            false_feasible: prediction.predicted_feasible && !prediction.observed_feasible,
            predicted_feasible: prediction.predicted_feasible,
            missed_feasible: !prediction.predicted_feasible && prediction.observed_feasible,
        });
    }

    for (group, model_sets) in &comparison_sets {
        let mut sample_sets = model_sets.values();
        if let Some(reference_samples) = sample_sets.next() {
            if sample_sets.any(|samples| samples != reference_samples) {
                return Err(format!(
                    "models have different blind sample sets for task={} geometry={} regime={} constraint={} domain={}",
                    group.task_id,
                    group.geometry_family,
                    group.flow_regime,
                    group.constraint_id,
                    group.domain_status.as_str()
                ));
            }
        }
    }

    let mut summaries = Vec::with_capacity(groups.len());
    for (group, mut samples) in groups {
        samples.sort_by(|left, right| {
            left.uncertainty_score
                .total_cmp(&right.uncertainty_score)
                .then_with(|| left.sample_id.cmp(&right.sample_id))
        });
        let sample_count = samples.len();
        let fully_accepted_absolute_error =
            samples.iter().map(|sample| sample.absolute_error).sum::<f64>();
        let fully_accepted_squared_error =
            samples.iter().map(|sample| sample.squared_error).sum::<f64>();
        let fully_accepted_false_feasible_count =
            samples.iter().filter(|sample| sample.false_feasible).count();
        let fully_accepted_mae = fully_accepted_absolute_error / sample_count as f64;
        let fully_accepted_rmse =
            (fully_accepted_squared_error / sample_count as f64).sqrt();
        let fully_accepted_false_feasible_rate =
            fully_accepted_false_feasible_count as f64 / sample_count as f64;

        let mut curve = Vec::new();
        let mut accepted_count = 0usize;
        let mut absolute_error_sum = 0.0;
        let mut squared_error_sum = 0.0;
        let mut false_feasible_count = 0usize;
        let mut predicted_feasible_count = 0usize;
        let mut missed_feasible_count = 0usize;
        let mut previous_coverage = 0.0;
        let mut previous_mae_risk = 0.0;
        let mut previous_false_feasible_risk = 0.0;
        let mut area_under_mae_risk_coverage = 0.0;
        let mut area_under_false_feasible_risk_coverage = 0.0;
        let mut cursor = 0usize;

        while cursor < samples.len() {
            let threshold = samples[cursor].uncertainty_score;
            let mut end = cursor;
            while end < samples.len() && samples[end].uncertainty_score == threshold {
                let sample = &samples[end];
                accepted_count += 1;
                absolute_error_sum += sample.absolute_error;
                squared_error_sum += sample.squared_error;
                false_feasible_count += if sample.false_feasible { 1 } else { 0 };
                predicted_feasible_count += if sample.predicted_feasible { 1 } else { 0 };
                missed_feasible_count += if sample.missed_feasible { 1 } else { 0 };
                end += 1;
            }

            let coverage_fraction = accepted_count as f64 / sample_count as f64;
            let selective_mae = absolute_error_sum / accepted_count as f64;
            let selective_rmse = (squared_error_sum / accepted_count as f64).sqrt();
            let false_feasible_rate = false_feasible_count as f64 / accepted_count as f64;
            let false_feasible_rate_per_predicted_feasible = if predicted_feasible_count == 0 {
                None
            } else {
                Some(false_feasible_count as f64 / predicted_feasible_count as f64)
            };
            area_under_mae_risk_coverage += (coverage_fraction - previous_coverage)
                * (previous_mae_risk + selective_mae)
                * 0.5;
            area_under_false_feasible_risk_coverage += (coverage_fraction - previous_coverage)
                * (previous_false_feasible_risk + false_feasible_rate)
                * 0.5;
            curve.push(RiskCoveragePoint {
                accepted_count,
                sample_count,
                uncertainty_threshold: threshold,
                coverage_fraction,
                selective_mae,
                selective_rmse,
                false_feasible_count,
                false_feasible_rate,
                predicted_feasible_count,
                false_feasible_rate_per_predicted_feasible,
                missed_feasible_count,
                partial_area_under_mae_risk_coverage: area_under_mae_risk_coverage,
                partial_area_under_false_feasible_risk_coverage:
                    area_under_false_feasible_risk_coverage,
            });
            previous_coverage = coverage_fraction;
            previous_mae_risk = selective_mae;
            previous_false_feasible_risk = false_feasible_rate;
            cursor = end;
        }

        let finite_values = [
            fully_accepted_mae,
            fully_accepted_rmse,
            fully_accepted_false_feasible_rate,
            area_under_mae_risk_coverage,
            area_under_false_feasible_risk_coverage,
        ];
        if finite_values.iter().any(|value| !value.is_finite())
            || curve.iter().any(|point| {
                !point.coverage_fraction.is_finite()
                    || !point.selective_mae.is_finite()
                    || !point.selective_rmse.is_finite()
                    || !point.false_feasible_rate.is_finite()
                    || !point.partial_area_under_mae_risk_coverage.is_finite()
                    || !point
                        .partial_area_under_false_feasible_risk_coverage
                        .is_finite()
            })
        {
            return Err(format!(
                "risk/coverage metrics overflowed for model {}",
                group.model_id
            ));
        }

        summaries.push(SelectiveRiskSummary {
            task_id: group.task_id,
            model_id: group.model_id,
            geometry_family: group.geometry_family,
            flow_regime: group.flow_regime,
            constraint_id: group.constraint_id,
            domain_status: group.domain_status,
            sample_count,
            fully_accepted_mae,
            fully_accepted_rmse,
            fully_accepted_false_feasible_count,
            fully_accepted_false_feasible_rate,
            area_under_mae_risk_coverage,
            area_under_false_feasible_risk_coverage,
            curve,
        });
    }

    Ok(SelectiveRiskReport {
        dataset_split: "blind_confirmation_only",
        evidence_level: "empirical_selective_risk_coverage_and_false_feasible_audit_not_future_guarantee",
        summaries,
    })
}

fn validate_prediction(prediction: &SelectivePrediction, index: usize) -> Result<(), String> {
    for (name, value) in [
        ("task_id", prediction.task_id.as_str()),
        ("sample_id", prediction.sample_id.as_str()),
        ("model_id", prediction.model_id.as_str()),
        ("geometry_family", prediction.geometry_family.as_str()),
        ("flow_regime", prediction.flow_regime.as_str()),
        ("constraint_id", prediction.constraint_id.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(format!("prediction row {index}: {name} must not be empty"));
        }
    }
    if prediction.dataset_split != "blind_confirmation" {
        return Err(format!(
            "prediction row {index}: only blind_confirmation rows may be audited"
        ));
    }
    if !prediction.uncertainty_score.is_finite() || prediction.uncertainty_score < 0.0 {
        return Err(format!(
            "prediction row {index}: uncertainty_score must be finite and non-negative"
        ));
    }
    for (name, value) in [
        ("point_prediction", prediction.point_prediction),
        ("observed_value", prediction.observed_value),
    ] {
        if !value.is_finite() {
            return Err(format!("prediction row {index}: {name} must be finite"));
        }
    }
    let error = prediction.point_prediction - prediction.observed_value;
    if !error.is_finite() || !(error * error).is_finite() {
        return Err(format!(
            "prediction row {index}: point error or squared error overflowed"
        ));
    }
    Ok(())
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

pub const SELECTIVE_RISK_COVERAGE_CSV_HEADER: &str = "task_id,model_id,geometry_family,flow_regime,constraint_id,domain_status,accepted_count,sample_count,uncertainty_threshold,coverage_fraction,selective_mae,selective_rmse,false_feasible_count,false_feasible_rate,predicted_feasible_count,false_feasible_rate_per_predicted_feasible,missed_feasible_count,area_under_mae_risk_coverage,area_under_false_feasible_risk_coverage,evidence_level";

pub fn report_to_csv(report: &SelectiveRiskReport) -> String {
    let mut output = format!("{SELECTIVE_RISK_COVERAGE_CSV_HEADER}\n");
    for summary in &report.summaries {
        for point in &summary.curve {
            output.push_str(&format!(
                "{},{},{},{},{},{},{},{},{:.12},{:.12},{:.12},{:.12},{},{:.12},{},{},{},{:.12},{:.12},{}\n",
                csv_escape(&summary.task_id),
                csv_escape(&summary.model_id),
                csv_escape(&summary.geometry_family),
                csv_escape(&summary.flow_regime),
                csv_escape(&summary.constraint_id),
                summary.domain_status.as_str(),
                point.accepted_count,
                point.sample_count,
                point.uncertainty_threshold,
                point.coverage_fraction,
                point.selective_mae,
                point.selective_rmse,
                point.false_feasible_count,
                point.false_feasible_rate,
                point.predicted_feasible_count,
                point
                    .false_feasible_rate_per_predicted_feasible
                    .map(|value| format!("{value:.12}"))
                    .unwrap_or_default(),
                point.missed_feasible_count,
                point.partial_area_under_mae_risk_coverage,
                point.partial_area_under_false_feasible_risk_coverage,
                report.evidence_level
            ));
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_predictions() -> Vec<SelectivePrediction> {
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
        predictions
    }

    #[test]
    fn risk_coverage_rewards_uncertainty_that_ranks_errors_and_false_feasible_cases() {
        let report =
            audit_blind_selective_risk(&synthetic_predictions()).expect("valid blind set");
        assert_eq!(report.summaries.len(), 4);
        for domain in [DomainStatus::InDomain, DomainStatus::OutOfDomain] {
            let ranked = report
                .summaries
                .iter()
                .find(|row| {
                    row.model_id == "error_ranked_uncertainty" && row.domain_status == domain
                })
                .expect("error-ranked uncertainty summary");
            let misranked = report
                .summaries
                .iter()
                .find(|row| {
                    row.model_id == "misranked_uncertainty" && row.domain_status == domain
                })
                .expect("misranked uncertainty summary");
            assert_eq!(ranked.sample_count, 20);
            assert_eq!(ranked.curve.len(), 20);
            assert!(ranked.area_under_mae_risk_coverage
                < misranked.area_under_mae_risk_coverage);
            assert!(ranked.area_under_false_feasible_risk_coverage
                < misranked.area_under_false_feasible_risk_coverage);
        }
    }

    #[test]
    fn blind_split_truth_and_matched_sample_sets_are_fail_closed() {
        let mut predictions = synthetic_predictions();
        predictions[0].dataset_split = "calibration".to_owned();
        assert!(audit_blind_selective_risk(&predictions)
            .expect_err("calibration not blind")
            .contains("only blind_confirmation"));

        let mut predictions = synthetic_predictions();
        predictions[0].observed_feasible = false;
        assert!(audit_blind_selective_risk(&predictions)
            .expect_err("observed feasibility must agree across models")
            .contains("changes observed value or feasibility truth"));

        let mut predictions = synthetic_predictions();
        predictions.retain(|row| {
            !(row.model_id == "misranked_uncertainty"
                && row.domain_status == DomainStatus::OutOfDomain
                && row.sample_id == "out_of_domain-19")
        });
        assert!(audit_blind_selective_risk(&predictions)
            .expect_err("model sample sets must match")
            .contains("different blind sample sets"));
    }

    #[test]
    fn duplicate_samples_negative_uncertainty_and_nonfinite_values_fail_closed() {
        let mut predictions = synthetic_predictions();
        predictions.push(predictions[0].clone());
        assert!(audit_blind_selective_risk(&predictions)
            .expect_err("duplicate sample rejected")
            .contains("duplicate blind sample"));

        let mut predictions = synthetic_predictions();
        predictions[0].uncertainty_score = -1.0;
        assert!(audit_blind_selective_risk(&predictions)
            .expect_err("negative uncertainty rejected")
            .contains("non-negative"));

        let mut predictions = synthetic_predictions();
        predictions[0].point_prediction = f64::INFINITY;
        assert!(audit_blind_selective_risk(&predictions)
            .expect_err("infinite point rejected")
            .contains("must be finite"));
    }

    #[test]
    fn csv_report_is_deterministic_and_exports_full_risk_coverage_curve() {
        let report =
            audit_blind_selective_risk(&synthetic_predictions()).expect("valid blind set");
        let first = report_to_csv(&report);
        assert_eq!(first, report_to_csv(&report));
        assert_eq!(first.lines().count(), 81);
        assert!(first.contains("false_feasible_rate"));
        assert!(first.contains("blind_confirmation_only"));
    }
}
