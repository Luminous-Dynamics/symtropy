//! Receipt-checked comparison of two decision-validity evaluator reports.
//!
//! This reports metric deltas only when both methods used the same dataset,
//! paired target windows, split manifests, channel, horizon, and event rules.
//! It never creates a composite score or a significance claim.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::PathBuf;

const EPS: f64 = 1.0e-9;

#[derive(Clone, Debug, Deserialize)]
struct Metrics {
    realized_alert_rate: f64,
    window_recall: Option<f64>,
    window_precision: Option<f64>,
    auroc: Option<f64>,
    average_precision_auprc: Option<f64>,
}

#[derive(Clone, Debug, Deserialize)]
struct Point {
    nominal_calibration_alert_budget: f64,
    calibration: Metrics,
    test: Metrics,
    test_event_count: usize,
    test_detected_event_count: usize,
    test_event_recall: Option<f64>,
    median_warning_lead_s: Option<f64>,
    false_alert_episodes_per_flight_minute: Option<f64>,
}

#[derive(Clone, Debug, Deserialize)]
struct Receipt {
    schema_version: String,
    evidence_class: String,
    warning_method: String,
    source_commit: String,
    channel: String,
    unit: String,
    error_threshold: f64,
    expected_horizon_s: f64,
    event_gap_s: f64,
    alert_merge_gap_s: f64,
    dataset_receipt_sha256: String,
    paired_window_receipt_sha256: String,
    calibration_manifest_sha256: String,
    test_manifest_sha256: String,
    calibration_flight_count: usize,
    test_flight_count: usize,
    operating_points: Vec<Point>,
}

#[derive(Debug)]
struct Args {
    baseline: PathBuf,
    candidate: PathBuf,
    alert_rate_tolerance: f64,
    output: Option<PathBuf>,
}

#[derive(Serialize)]
struct Delta {
    baseline: Option<f64>,
    candidate: Option<f64>,
    candidate_minus_baseline: Option<f64>,
}

#[derive(Serialize)]
struct ComparedPoint {
    nominal_calibration_alert_budget: f64,
    test_alert_rates_within_tolerance: bool,
    calibration_auprc: Delta,
    test_auroc: Delta,
    test_auprc: Delta,
    test_window_recall: Delta,
    test_window_precision: Delta,
    test_event_recall: Delta,
    median_warning_lead_s: Delta,
    false_alert_episodes_per_flight_minute: Delta,
    baseline_test_event_count: usize,
    candidate_test_event_count: usize,
    baseline_test_detected_event_count: usize,
    candidate_test_detected_event_count: usize,
    note: String,
}

#[derive(Serialize)]
struct Comparison {
    schema_version: String,
    comparison_kind: String,
    evidence_class: String,
    baseline_method: String,
    baseline_report_sha256: String,
    baseline_source_commit: String,
    candidate_method: String,
    candidate_report_sha256: String,
    candidate_source_commit: String,
    channel: String,
    unit: String,
    error_threshold: f64,
    horizon_s: f64,
    paired_window_receipt_sha256: String,
    dataset_receipt_sha256: String,
    calibration_manifest_sha256: String,
    test_manifest_sha256: String,
    calibration_flight_count: usize,
    test_flight_count: usize,
    alert_rate_tolerance: f64,
    claim_ceiling: String,
    points: Vec<ComparedPoint>,
}

fn parse_args() -> Result<Args, String> {
    let mut baseline = None;
    let mut candidate = None;
    let mut alert_rate_tolerance = 0.01;
    let mut output = None;
    let mut args = env::args().skip(1);
    while let Some(key) = args.next() {
        let value = args.next().ok_or_else(|| format!("missing value for {key}"))?;
        match key.as_str() {
            "--baseline" if baseline.is_none() => baseline = Some(PathBuf::from(value)),
            "--candidate" if candidate.is_none() => candidate = Some(PathBuf::from(value)),
            "--alert-rate-tolerance" => {
                alert_rate_tolerance = value.parse::<f64>().map_err(|_| "alert-rate-tolerance must be numeric".to_string())?;
            }
            "--output" if output.is_none() => output = Some(PathBuf::from(value)),
            _ => return Err(format!("unknown or duplicate argument {key}")),
        }
    }
    if !alert_rate_tolerance.is_finite() || !(0.0..1.0).contains(&alert_rate_tolerance) {
        return Err("alert-rate-tolerance must be finite and in [0,1)".into());
    }
    Ok(Args {
        baseline: baseline.ok_or_else(|| "missing --baseline".to_string())?,
        candidate: candidate.ok_or_else(|| "missing --candidate".to_string())?,
        alert_rate_tolerance,
        output,
    })
}

fn read_report(path: &PathBuf) -> Result<(Receipt, String), String> {
    let bytes = fs::read(path).map_err(|e| format!("cannot read report {}: {e}", path.display()))?;
    let mut hash = Sha256::new();
    hash.update(&bytes);
    let digest = format!("{:x}", hash.finalize());
    let report = serde_json::from_slice(&bytes).map_err(|e| format!("invalid report {}: {e}", path.display()))?;
    Ok((report, digest))
}

fn validate_pair(a: &Receipt, b: &Receipt) -> Result<(), String> {
    let same = [
        ("schema_version", a.schema_version == b.schema_version),
        ("evidence_class", a.evidence_class == b.evidence_class),
        ("channel", a.channel == b.channel),
        ("unit", a.unit == b.unit),
        ("dataset receipt", a.dataset_receipt_sha256 == b.dataset_receipt_sha256),
        ("paired truth/conditioning receipt", a.paired_window_receipt_sha256 == b.paired_window_receipt_sha256),
        ("calibration manifest", a.calibration_manifest_sha256 == b.calibration_manifest_sha256),
        ("test manifest", a.test_manifest_sha256 == b.test_manifest_sha256),
        ("calibration flight count", a.calibration_flight_count == b.calibration_flight_count),
        ("test flight count", a.test_flight_count == b.test_flight_count),
        ("error threshold", (a.error_threshold - b.error_threshold).abs() <= EPS),
        ("prediction horizon", (a.expected_horizon_s - b.expected_horizon_s).abs() <= EPS),
        ("event gap", (a.event_gap_s - b.event_gap_s).abs() <= EPS),
        ("alert-merge gap", (a.alert_merge_gap_s - b.alert_merge_gap_s).abs() <= EPS),
    ];
    let mismatches: Vec<&str> = same.iter().filter(|(_, ok)| !*ok).map(|(name, _)| *name).collect();
    if !mismatches.is_empty() {
        return Err(format!("non-comparable receipts; mismatched fields: {}", mismatches.join(", ")));
    }
    if a.operating_points.len() != b.operating_points.len() {
        return Err("non-comparable reports; operating-point counts differ".into());
    }
    for (i, (x, y)) in a.operating_points.iter().zip(&b.operating_points).enumerate() {
        if (x.nominal_calibration_alert_budget - y.nominal_calibration_alert_budget).abs() > EPS {
            return Err(format!("non-comparable reports; alert budget at point {i} differs"));
        }
    }
    if a.warning_method == b.warning_method && a.source_commit == b.source_commit {
        return Err("baseline and candidate identify the same method at the same source commit".into());
    }
    Ok(())
}

fn delta(a: Option<f64>, b: Option<f64>) -> Delta {
    Delta { baseline: a, candidate: b, candidate_minus_baseline: match (a, b) {
        (Some(x), Some(y)) => Some(y - x), _ => None,
    }}
}

fn compare(a: &Receipt, b: &Receipt, tolerance: f64, a_hash: &str, b_hash: &str) -> Comparison {
    let points = a.operating_points.iter().zip(&b.operating_points).map(|(base, cand)| {
        let rate_delta = (base.test.realized_alert_rate - cand.test.realized_alert_rate).abs();
        ComparedPoint {
            nominal_calibration_alert_budget: base.nominal_calibration_alert_budget,
            test_alert_rates_within_tolerance: rate_delta <= tolerance + EPS,
            calibration_auprc: delta(base.calibration.average_precision_auprc, cand.calibration.average_precision_auprc),
            test_auroc: delta(base.test.auroc, cand.test.auroc),
            test_auprc: delta(base.test.average_precision_auprc, cand.test.average_precision_auprc),
            test_window_recall: delta(base.test.window_recall, cand.test.window_recall),
            test_window_precision: delta(base.test.window_precision, cand.test.window_precision),
            test_event_recall: delta(base.test_event_recall, cand.test_event_recall),
            median_warning_lead_s: delta(base.median_warning_lead_s, cand.median_warning_lead_s),
            false_alert_episodes_per_flight_minute: delta(base.false_alert_episodes_per_flight_minute, cand.false_alert_episodes_per_flight_minute),
            baseline_test_event_count: base.test_event_count,
            candidate_test_event_count: cand.test_event_count,
            baseline_test_detected_event_count: base.test_detected_event_count,
            candidate_test_detected_event_count: cand.test_detected_event_count,
            note: if rate_delta <= tolerance + EPS {
                "Frozen calibration thresholds; test alert rates happen to be within declared tolerance. Descriptive only; no significance claim.".into()
            } else {
                "Frozen calibration thresholds yield different realized test alert rates; do not claim equal-alert-budget superiority.".into()
            },
        }
    }).collect();
    Comparison {
        schema_version: "symtropy-decision-validity-comparison-v0.1".into(),
        comparison_kind: "receipt-checked-descriptive-delta".into(),
        evidence_class: a.evidence_class.clone(),
        baseline_method: a.warning_method.clone(),
        baseline_report_sha256: a_hash.into(),
        baseline_source_commit: a.source_commit.clone(),
        candidate_method: b.warning_method.clone(),
        candidate_report_sha256: b_hash.into(),
        candidate_source_commit: b.source_commit.clone(),
        channel: a.channel.clone(), unit: a.unit.clone(), error_threshold: a.error_threshold,
        horizon_s: a.expected_horizon_s, paired_window_receipt_sha256: a.paired_window_receipt_sha256.clone(),
        dataset_receipt_sha256: a.dataset_receipt_sha256.clone(),
        calibration_manifest_sha256: a.calibration_manifest_sha256.clone(),
        test_manifest_sha256: a.test_manifest_sha256.clone(),
        calibration_flight_count: a.calibration_flight_count, test_flight_count: a.test_flight_count,
        alert_rate_tolerance: tolerance,
        claim_ceiling: "Descriptive comparison only. Separate reports do not supply paired per-flight bootstrap deltas or prove controller benefit. Report actual alert rates; no composite score is defined.".into(),
        points,
    }
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let (baseline, baseline_hash) = read_report(&args.baseline)?;
    let (candidate, candidate_hash) = read_report(&args.candidate)?;
    validate_pair(&baseline, &candidate)?;
    let output = serde_json::to_string_pretty(&compare(&baseline, &candidate, args.alert_rate_tolerance, &baseline_hash, &candidate_hash))
        .map_err(|e| format!("cannot serialize comparison: {e}"))?;
    if let Some(path) = args.output {
        fs::write(&path, format!("{output}\n")).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    } else {
        println!("{output}");
    }
    Ok(())
}

fn main() {
    if let Err(err) = run() {
        eprintln!("nanodrone_decision_validity_compare: {err}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metrics(auc: Option<f64>, ap: Option<f64>, alert_rate: f64) -> Metrics {
        Metrics { realized_alert_rate: alert_rate, window_recall: Some(0.5), window_precision: Some(0.4),
            auroc: auc, average_precision_auprc: ap }
    }

    fn receipt(method: &str, auc: Option<f64>, ap: Option<f64>, target: &str) -> Receipt {
        let point = Point {
            nominal_calibration_alert_budget: 0.05,
            calibration: metrics(auc, ap, 0.05),
            test: metrics(auc, ap, 0.06),
            test_event_count: 2, test_detected_event_count: 1, test_event_recall: Some(0.5),
            median_warning_lead_s: Some(0.1), false_alert_episodes_per_flight_minute: Some(0.2),
        };
        Receipt {
            schema_version: "symtropy-decision-validity-v0.2".into(), evidence_class: "synthetic".into(),
            warning_method: method.into(), source_commit: if method == "base" { "a".repeat(40) } else { "b".repeat(40) },
            channel: "position".into(), unit: "m".into(), error_threshold: 0.1, expected_horizon_s: 0.5,
            event_gap_s: 0.02, alert_merge_gap_s: 0.05, dataset_receipt_sha256: "d".repeat(64),
            paired_window_receipt_sha256: target.into(), calibration_manifest_sha256: "c".repeat(64),
            test_manifest_sha256: "t".repeat(64), calibration_flight_count: 12, test_flight_count: 12,
            operating_points: vec![point],
        }
    }

    #[test]
    fn same_cohort_yields_descriptive_delta_and_alert_rate_gate() {
        let a = receipt("base", Some(0.7), Some(0.3), &"p".repeat(64));
        let b = receipt("candidate", Some(0.8), Some(0.4), &"p".repeat(64));
        validate_pair(&a, &b).unwrap();
        let c = compare(&a, &b, 0.01, &"1".repeat(64), &"2".repeat(64));
        assert!((c.points[0].test_auprc.candidate_minus_baseline.unwrap() - 0.1).abs() < 1e-12);
        assert!(c.points[0].test_alert_rates_within_tolerance);
        assert!(c.claim_ceiling.contains("Descriptive"));
    }

    #[test]
    fn mismatched_truth_receipt_is_rejected() {
        let a = receipt("base", Some(0.7), Some(0.3), &"p".repeat(64));
        let b = receipt("candidate", Some(0.8), Some(0.4), &"q".repeat(64));
        assert!(validate_pair(&a, &b).is_err());
    }

    #[test]
    fn undefined_metrics_stay_null_in_deltas() {
        let a = receipt("base", None, None, &"p".repeat(64));
        let b = receipt("candidate", Some(0.8), Some(0.4), &"p".repeat(64));
        validate_pair(&a, &b).unwrap();
        let c = compare(&a, &b, 0.01, &"1".repeat(64), &"2".repeat(64));
        assert_eq!(c.points[0].test_auprc.candidate_minus_baseline, None);
    }
}
