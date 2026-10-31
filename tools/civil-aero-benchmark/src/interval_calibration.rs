//! Blind-set prediction-interval calibration and sharpness audit.
//!
//! This module evaluates intervals already produced by a model or calibration method. It does
//! not fit intervals or guarantee future coverage. Inputs must be a blind_confirmation split;
//! calibration/training rows are rejected. Results are reported by geometry, flow regime and
//! in-domain / out-of-domain status so marginal coverage cannot hide subgroup undercoverage.

use std::collections::{BTreeMap, BTreeSet};

const WILSON_Z_95: f64 = 1.959_963_984_540_054;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DomainStatus {
    InDomain,
    OutOfDomain,
}

impl DomainStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InDomain => "in_domain",
            Self::OutOfDomain => "out_of_domain",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IntervalPrediction {
    pub task_id: String,
    pub sample_id: String,
    pub model_id: String,
    pub geometry_family: String,
    pub flow_regime: String,
    /// Must be blind_confirmation. Calibration rows must never be scored as confirmation.
    pub dataset_split: String,
    pub domain_status: DomainStatus,
    /// Central interval coverage, e.g. 0.90 for a nominal 90% interval.
    pub nominal_coverage: f64,
    pub lower: f64,
    pub upper: f64,
    pub point_prediction: f64,
    pub observed_value: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IntervalCalibrationSummary {
    pub task_id: String,
    pub model_id: String,
    pub geometry_family: String,
    pub flow_regime: String,
    pub domain_status: DomainStatus,
    pub nominal_coverage: f64,
    pub sample_count: usize,
    pub covered_count: usize,
    pub lower_miss_count: usize,
    pub upper_miss_count: usize,
    pub empirical_coverage: f64,
    pub wilson_95_lower: f64,
    pub wilson_95_upper: f64,
    /// Empirical coverage minus nominal coverage.
    pub coverage_gap: f64,
    pub mean_interval_width: f64,
    pub mean_interval_score: f64,
    pub point_mean_absolute_error: f64,
    pub point_root_mean_square_error: f64,
    /// Mean point_prediction - observed_value.
    pub point_bias: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IntervalCalibrationReport {
    pub dataset_split: &'static str,
    pub evidence_level: &'static str,
    pub summaries: Vec<IntervalCalibrationSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SampleKey {
    task_id: String,
    sample_id: String,
    model_id: String,
    geometry_family: String,
    flow_regime: String,
    domain_status: DomainStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct TruthKey {
    task_id: String,
    sample_id: String,
    geometry_family: String,
    flow_regime: String,
    domain_status: DomainStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ComparisonKey {
    task_id: String,
    geometry_family: String,
    flow_regime: String,
    domain_status: DomainStatus,
    nominal_coverage_bits: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SummaryKey {
    task_id: String,
    model_id: String,
    geometry_family: String,
    flow_regime: String,
    domain_status: DomainStatus,
    nominal_coverage_bits: u64,
}

#[derive(Debug, Clone)]
struct SampleIntervals {
    point_prediction_bits: u64,
    observed_value_bits: u64,
    intervals: BTreeMap<u64, (f64, f64)>,
}

#[derive(Debug, Clone, Default)]
struct Accumulator {
    sample_count: usize,
    covered_count: usize,
    lower_miss_count: usize,
    upper_miss_count: usize,
    interval_width_sum: f64,
    interval_score_sum: f64,
    absolute_error_sum: f64,
    squared_error_sum: f64,
    signed_error_sum: f64,
}

pub fn audit_blind_prediction_intervals(
    predictions: &[IntervalPrediction],
) -> Result<IntervalCalibrationReport, String> {
    if predictions.is_empty() {
        return Err("blind prediction-interval audit requires at least one sample".to_owned());
    }

    let mut sample_intervals: BTreeMap<SampleKey, SampleIntervals> = BTreeMap::new();
    let mut truth_by_sample: BTreeMap<TruthKey, u64> = BTreeMap::new();
    let mut comparison_sets: BTreeMap<ComparisonKey, BTreeMap<String, BTreeSet<String>>> =
        BTreeMap::new();
    let mut summaries: BTreeMap<SummaryKey, Accumulator> = BTreeMap::new();

    for (index, prediction) in predictions.iter().enumerate() {
        validate_prediction(prediction, index)?;
        let truth_key = TruthKey {
            task_id: prediction.task_id.clone(),
            sample_id: prediction.sample_id.clone(),
            geometry_family: prediction.geometry_family.clone(),
            flow_regime: prediction.flow_regime.clone(),
            domain_status: prediction.domain_status,
        };
        if let Some(previous_observation) =
            truth_by_sample.insert(truth_key, prediction.observed_value.to_bits())
        {
            if previous_observation != prediction.observed_value.to_bits() {
                return Err(format!(
                    "sample {} changes observed truth across model predictions",
                    prediction.sample_id
                ));
            }
        }

        let comparison_key = ComparisonKey {
            task_id: prediction.task_id.clone(),
            geometry_family: prediction.geometry_family.clone(),
            flow_regime: prediction.flow_regime.clone(),
            domain_status: prediction.domain_status,
            nominal_coverage_bits: prediction.nominal_coverage.to_bits(),
        };
        comparison_sets
            .entry(comparison_key)
            .or_default()
            .entry(prediction.model_id.clone())
            .or_default()
            .insert(prediction.sample_id.clone());

        let sample_key = SampleKey {
            task_id: prediction.task_id.clone(),
            sample_id: prediction.sample_id.clone(),
            model_id: prediction.model_id.clone(),
            geometry_family: prediction.geometry_family.clone(),
            flow_regime: prediction.flow_regime.clone(),
            domain_status: prediction.domain_status,
        };
        let sample = sample_intervals.entry(sample_key).or_insert_with(|| SampleIntervals {
            point_prediction_bits: prediction.point_prediction.to_bits(),
            observed_value_bits: prediction.observed_value.to_bits(),
            intervals: BTreeMap::new(),
        });
        if sample.point_prediction_bits != prediction.point_prediction.to_bits()
            || sample.observed_value_bits != prediction.observed_value.to_bits()
        {
            return Err(format!(
                "sample {} changes point prediction or observed value across interval levels",
                prediction.sample_id
            ));
        }
        if sample
            .intervals
            .insert(
                prediction.nominal_coverage.to_bits(),
                (prediction.lower, prediction.upper),
            )
            .is_some()
        {
            return Err(format!(
                "sample {} has a duplicate nominal coverage level {}",
                prediction.sample_id, prediction.nominal_coverage
            ));
        }

        let width = prediction.upper - prediction.lower;
        let alpha = 1.0 - prediction.nominal_coverage;
        let lower_miss = prediction.observed_value < prediction.lower;
        let upper_miss = prediction.observed_value > prediction.upper;
        let covered = !lower_miss && !upper_miss;
        let mut interval_score = width;
        if lower_miss {
            interval_score += (2.0 / alpha) * (prediction.lower - prediction.observed_value);
        } else if upper_miss {
            interval_score += (2.0 / alpha) * (prediction.observed_value - prediction.upper);
        }

        let point_error = prediction.point_prediction - prediction.observed_value;
        let key = SummaryKey {
            task_id: prediction.task_id.clone(),
            model_id: prediction.model_id.clone(),
            geometry_family: prediction.geometry_family.clone(),
            flow_regime: prediction.flow_regime.clone(),
            domain_status: prediction.domain_status,
            nominal_coverage_bits: prediction.nominal_coverage.to_bits(),
        };
        let summary = summaries.entry(key).or_default();
        summary.sample_count += 1;
        summary.covered_count += usize::from(covered);
        summary.lower_miss_count += usize::from(lower_miss);
        summary.upper_miss_count += usize::from(upper_miss);
        summary.interval_width_sum += width;
        summary.interval_score_sum += interval_score;
        summary.absolute_error_sum += point_error.abs();
        summary.squared_error_sum += point_error * point_error;
        summary.signed_error_sum += point_error;
    }

    // Comparative model rows must cover exactly the same blind samples. Otherwise a
    // model could appear calibrated by silently omitting difficult cases.
    for (group, model_sets) in &comparison_sets {
        let mut sample_sets = model_sets.values();
        if let Some(reference_samples) = sample_sets.next() {
            if sample_sets.any(|samples| samples != reference_samples) {
                return Err(format!(
                    "models have different blind sample sets for task={} geometry={} regime={} domain={} coverage={}",
                    group.task_id,
                    group.geometry_family,
                    group.flow_regime,
                    group.domain_status.as_str(),
                    f64::from_bits(group.nominal_coverage_bits)
                ));
            }
        }
    }

    for (sample_key, sample) in &sample_intervals {
        let mut previous: Option<(f64, f64)> = None;
        for (coverage_bits, (lower, upper)) in &sample.intervals {
            let coverage = f64::from_bits(*coverage_bits);
            if let Some((previous_lower, previous_upper)) = previous {
                if *lower > previous_lower || *upper < previous_upper {
                    return Err(format!(
                        "sample {} has non-nested intervals as nominal coverage increases to {}",
                        sample_key.sample_id, coverage
                    ));
                }
            }
            previous = Some((*lower, *upper));
        }
    }

    let mut rows = Vec::with_capacity(summaries.len());
    for (key, accumulator) in summaries {
        let count = accumulator.sample_count as f64;
        let empirical_coverage = accumulator.covered_count as f64 / count;
        let nominal_coverage = f64::from_bits(key.nominal_coverage_bits);
        let (wilson_95_lower, wilson_95_upper) =
            wilson_interval(accumulator.covered_count, accumulator.sample_count)?;
        let values = [
            empirical_coverage,
            wilson_95_lower,
            wilson_95_upper,
            accumulator.interval_width_sum / count,
            accumulator.interval_score_sum / count,
            accumulator.absolute_error_sum / count,
            (accumulator.squared_error_sum / count).sqrt(),
            accumulator.signed_error_sum / count,
        ];
        if values.iter().any(|value| !value.is_finite()) {
            return Err(format!(
                "summary for model {} and nominal coverage {} overflowed",
                key.model_id, nominal_coverage
            ));
        }
        rows.push(IntervalCalibrationSummary {
            task_id: key.task_id,
            model_id: key.model_id,
            geometry_family: key.geometry_family,
            flow_regime: key.flow_regime,
            domain_status: key.domain_status,
            nominal_coverage,
            sample_count: accumulator.sample_count,
            covered_count: accumulator.covered_count,
            lower_miss_count: accumulator.lower_miss_count,
            upper_miss_count: accumulator.upper_miss_count,
            empirical_coverage,
            wilson_95_lower,
            wilson_95_upper,
            coverage_gap: empirical_coverage - nominal_coverage,
            mean_interval_width: values[3],
            mean_interval_score: values[4],
            point_mean_absolute_error: values[5],
            point_root_mean_square_error: values[6],
            point_bias: values[7],
        });
    }

    Ok(IntervalCalibrationReport {
        dataset_split: "blind_confirmation_only",
        evidence_level: "empirical_interval_calibration_and_sharpness_audit_not_future_coverage_guarantee",
        summaries: rows,
    })
}

fn validate_prediction(prediction: &IntervalPrediction, index: usize) -> Result<(), String> {
    for (name, value) in [
        ("task_id", prediction.task_id.as_str()),
        ("sample_id", prediction.sample_id.as_str()),
        ("model_id", prediction.model_id.as_str()),
        ("geometry_family", prediction.geometry_family.as_str()),
        ("flow_regime", prediction.flow_regime.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(format!("prediction row {index}: {name} must not be empty"));
        }
    }
    if prediction.dataset_split != "blind_confirmation" {
        return Err(format!(
            "prediction row {index}: only blind_confirmation rows may be audited; got {}",
            prediction.dataset_split
        ));
    }
    if !prediction.nominal_coverage.is_finite()
        || !(0.0..1.0).contains(&prediction.nominal_coverage)
    {
        return Err(format!(
            "prediction row {index}: nominal_coverage must be finite and in (0, 1)"
        ));
    }
    for (name, value) in [
        ("lower", prediction.lower),
        ("upper", prediction.upper),
        ("point_prediction", prediction.point_prediction),
        ("observed_value", prediction.observed_value),
    ] {
        if !value.is_finite() {
            return Err(format!("prediction row {index}: {name} must be finite"));
        }
    }
    if prediction.lower > prediction.upper {
        return Err(format!(
            "prediction row {index}: lower interval endpoint exceeds upper endpoint"
        ));
    }
    let width = prediction.upper - prediction.lower;
    let alpha = 1.0 - prediction.nominal_coverage;
    if !width.is_finite() || !(2.0 / alpha).is_finite() {
        return Err(format!(
            "prediction row {index}: interval width or interval-score factor overflowed"
        ));
    }
    Ok(())
}

fn wilson_interval(successes: usize, trials: usize) -> Result<(f64, f64), String> {
    if trials == 0 || successes > trials {
        return Err("Wilson interval requires 0 <= successes <= positive trials".to_owned());
    }
    let n = trials as f64;
    let p = successes as f64 / n;
    let z_squared = WILSON_Z_95 * WILSON_Z_95;
    let denominator = 1.0 + z_squared / n;
    let center = (p + z_squared / (2.0 * n)) / denominator;
    let half_width = WILSON_Z_95
        * ((p * (1.0 - p) / n + z_squared / (4.0 * n * n)).sqrt())
        / denominator;
    let lower = (center - half_width).max(0.0);
    let upper = (center + half_width).min(1.0);
    if !lower.is_finite() || !upper.is_finite() || lower > upper {
        return Err("Wilson interval calculation produced invalid bounds".to_owned());
    }
    Ok((lower, upper))
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

pub const INTERVAL_CALIBRATION_CSV_HEADER: &str = "task_id,model_id,geometry_family,flow_regime,domain_status,nominal_coverage,sample_count,covered_count,lower_miss_count,upper_miss_count,empirical_coverage,wilson_95_lower,wilson_95_upper,coverage_gap,mean_interval_width,mean_interval_score,point_mae,point_rmse,point_bias,evidence_level";

pub fn report_to_csv(report: &IntervalCalibrationReport) -> String {
    let mut output = format!("{INTERVAL_CALIBRATION_CSV_HEADER}\n");
    for row in &report.summaries {
        output.push_str(&format!(
            "{},{},{},{},{},{:.12},{},{},{},{},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{}\n",
            csv_escape(&row.task_id),
            csv_escape(&row.model_id),
            csv_escape(&row.geometry_family),
            csv_escape(&row.flow_regime),
            row.domain_status.as_str(),
            row.nominal_coverage,
            row.sample_count,
            row.covered_count,
            row.lower_miss_count,
            row.upper_miss_count,
            row.empirical_coverage,
            row.wilson_95_lower,
            row.wilson_95_upper,
            row.coverage_gap,
            row.mean_interval_width,
            row.mean_interval_score,
            row.point_mean_absolute_error,
            row.point_root_mean_square_error,
            row.point_bias,
            report.evidence_level,
        ));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_predictions() -> Vec<IntervalPrediction> {
        let levels = [0.80_f64, 0.90, 0.95];
        let widths = [0.80_f64, 0.90, 0.95];
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
                        let half_width = widths[level_index] * width_scale;
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
        predictions
    }

    #[test]
    fn blind_audit_reports_nominal_coverage_and_out_of_domain_degradation() {
        let report =
            audit_blind_prediction_intervals(&synthetic_predictions()).expect("valid blind set");
        assert_eq!(report.summaries.len(), 12);
        let nominal_ood = report
            .summaries
            .iter()
            .find(|row| {
                row.model_id == "nominal_model"
                    && row.domain_status == DomainStatus::OutOfDomain
                    && (row.nominal_coverage - 0.90).abs() < 1.0e-12
            })
            .expect("nominal model OOD row");
        assert_eq!(nominal_ood.sample_count, 60);
        assert_eq!(nominal_ood.covered_count, 27);
        assert!((nominal_ood.empirical_coverage - 0.45).abs() < 1.0e-12);

        let robust_ood = report
            .summaries
            .iter()
            .find(|row| {
                row.model_id == "robust_model"
                    && row.domain_status == DomainStatus::OutOfDomain
                    && (row.nominal_coverage - 0.90).abs() < 1.0e-12
            })
            .expect("robust model OOD row");
        assert_eq!(robust_ood.covered_count, 54);
        assert!((robust_ood.empirical_coverage - 0.90).abs() < 1.0e-12);
        assert!(robust_ood.mean_interval_width > nominal_ood.mean_interval_width);
    }

    #[test]
    fn wilson_interval_contains_nominal_rate_for_known_coverage_fixture() {
        let report =
            audit_blind_prediction_intervals(&synthetic_predictions()).expect("valid blind set");
        let row = report
            .summaries
            .iter()
            .find(|row| {
                row.model_id == "nominal_model"
                    && row.domain_status == DomainStatus::InDomain
                    && (row.nominal_coverage - 0.95).abs() < 1.0e-12
            })
            .expect("95 percent row");
        assert!(row.wilson_95_lower <= 0.95);
        assert!(row.wilson_95_upper >= 0.95);
    }

    #[test]
    fn calibration_split_is_rejected_from_blind_confirmation_audit() {
        let mut predictions = synthetic_predictions();
        predictions[0].dataset_split = "calibration".to_owned();
        let error = audit_blind_prediction_intervals(&predictions)
            .expect_err("calibration data cannot be evaluated as blind");
        assert!(error.contains("only blind_confirmation"));
    }

    #[test]
    fn invalid_or_non_nested_intervals_are_rejected() {
        let mut predictions = synthetic_predictions();
        predictions[0].lower = 1.0;
        predictions[0].upper = -1.0;
        assert!(audit_blind_prediction_intervals(&predictions)
            .expect_err("inverted interval rejected")
            .contains("lower interval endpoint"));

        let mut predictions = synthetic_predictions();
        let sample_id = predictions[0].sample_id.clone();
        let level_90 = predictions
            .iter_mut()
            .find(|row| row.sample_id == sample_id && (row.nominal_coverage - 0.90).abs() < 1e-12)
            .expect("90 percent interval");
        level_90.lower = -0.5;
        level_90.upper = 0.5;
        assert!(audit_blind_prediction_intervals(&predictions)
            .expect_err("non-nested intervals rejected")
            .contains("non-nested"));
    }

    #[test]
    fn duplicate_levels_and_inconsistent_observations_are_rejected() {
        let mut predictions = synthetic_predictions();
        predictions.push(predictions[0].clone());
        assert!(audit_blind_prediction_intervals(&predictions)
            .expect_err("duplicate level rejected")
            .contains("duplicate nominal coverage"));

        let mut predictions = synthetic_predictions();
        predictions[1].observed_value += 0.01;
        assert!(audit_blind_prediction_intervals(&predictions)
            .expect_err("sample observation must not change by level")
            .contains("changes point prediction or observed value"));
    }

    #[test]
    fn comparative_models_must_cover_the_same_blind_samples() {
        let mut predictions = synthetic_predictions();
        predictions.retain(|row| {
            !(row.model_id == "robust_model" && row.sample_id == "in_domain-0")
        });
        assert!(audit_blind_prediction_intervals(&predictions)
            .expect_err("omitted hard samples must be detected")
            .contains("different blind sample sets"));
    }

    #[test]
    fn observed_truth_must_match_across_compared_models() {
        let mut predictions = synthetic_predictions();
        for row in predictions
            .iter_mut()
            .filter(|row| row.model_id == "robust_model" && row.sample_id == "in_domain-0")
        {
            row.observed_value += 0.01;
        }
        assert!(audit_blind_prediction_intervals(&predictions)
            .expect_err("models cannot be scored against different observations")
            .contains("changes observed truth across model predictions"));
    }

    #[test]
    fn csv_is_deterministic_and_preserves_domain_strata() {
        let report =
            audit_blind_prediction_intervals(&synthetic_predictions()).expect("valid blind set");
        let first = report_to_csv(&report);
        assert_eq!(first, report_to_csv(&report));
        assert_eq!(first.lines().count(), 13);
        assert!(first.contains("in_domain"));
        assert!(first.contains("out_of_domain"));
        assert!(first.contains("blind_confirmation_only"));
    }
}
