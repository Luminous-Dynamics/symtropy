//! Paired, fail-closed comparison of Nano-Quadrotor model metric receipts.
//!
//! This tool compares independently scored models only when every Melon run has
//! the same canonical ground-truth-window digest, row count, and orientation
//! representation. It reports each metric separately; it never invents a unit-mixed
//! composite score and does not treat timesteps as independent statistical samples.

use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::io;
use std::path::Path;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const HORIZON: usize = 50;
const EXPECTED_DT_S: f64 = 0.010;
const RUN_IDS: [&str; 3] = ["run1", "run2", "run3"];
const SUMMARY_HORIZONS: [usize; 5] = [1, 5, 10, 20, 50];

#[derive(Clone, Copy)]
struct Channel {
    key: &'static str,
    unit: &'static str,
}

const CHANNELS: [Channel; 5] = [
    Channel { key: "position_mean_euclidean_error_m", unit: "m" },
    Channel { key: "linear_velocity_mean_euclidean_error_m_s", unit: "m/s" },
    Channel { key: "orientation_geodesic_error_rad_sign_invariant", unit: "rad" },
    Channel { key: "orientation_geodesic_error_rad_reference_formula", unit: "rad_reference_formula" },
    Channel { key: "body_angular_rate_mean_euclidean_error_rad_s", unit: "rad/s" },
];

#[derive(Debug, Clone)]
struct RunReceipt {
    run_id: String,
    row_count: usize,
    orientation: String,
    truth_digest: String,
    complete_metric: Value,
}

#[derive(Debug, Clone)]
struct LoadedReceipt {
    path: String,
    sha256: String,
    orientation: String,
    runs: BTreeMap<String, RunReceipt>,
}

fn invalid_input(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

fn field_str<'a>(value: &'a Value, path: &str) -> Result<&'a str, io::Error> {
    value.as_str().ok_or_else(|| invalid_input(format!("{path} must be a string")))
}

fn field_usize(value: &Value, path: &str) -> Result<usize, io::Error> {
    value.as_u64()
        .and_then(|number| usize::try_from(number).ok())
        .ok_or_else(|| invalid_input(format!("{path} must be a non-negative integer")))
}

fn field_f64(value: &Value, path: &str) -> Result<f64, io::Error> {
    let number = value.as_f64().ok_or_else(|| invalid_input(format!("{path} must be numeric")))?;
    if !number.is_finite() {
        return Err(invalid_input(format!("{path} must be finite")));
    }
    Ok(number)
}

fn check_digest(digest: &str, label: &str) -> Result<(), io::Error> {
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()) {
        return Err(invalid_input(format!("{label} is not a lowercase SHA-256 hex digest")));
    }
    Ok(())
}

fn parse_run(value: &Value, index: usize) -> Result<RunReceipt, io::Error> {
    let prefix = format!("per_run[{index}]");
    let run_id = field_str(&value["run_id"], &format!("{prefix}.run_id"))?.to_owned();
    let row_count = field_usize(&value["row_count"], &format!("{prefix}.row_count"))?;
    if row_count < HORIZON {
        return Err(invalid_input(format!("{prefix}.row_count is less than {HORIZON}")));
    }
    let orientation = field_str(
        &value["orientation_representation"],
        &format!("{prefix}.orientation_representation"),
    )?.to_owned();
    let truth_digest = field_str(
        &value["ground_truth_window_sha256"],
        &format!("{prefix}.ground_truth_window_sha256"),
    )?.to_owned();
    check_digest(&truth_digest, &format!("{prefix}.ground_truth_window_sha256"))?;
    let complete_metric = value["complete_window_metric"].clone();
    if !complete_metric.is_object() {
        return Err(invalid_input(format!(
            "{prefix}.complete_window_metric is missing; pairwise comparison requires complete h=1..50 truth windows"
        )));
    }
    let horizons = complete_metric["per_horizon"].as_array()
        .ok_or_else(|| invalid_input(format!("{prefix}.complete_window_metric.per_horizon must be an array")))?;
    if horizons.len() != HORIZON {
        return Err(invalid_input(format!(
            "{prefix} must contain exactly {HORIZON} per-horizon metrics, found {}",
            horizons.len()
        )));
    }
    for horizon in 1..=HORIZON {
        let entry = &horizons[horizon - 1];
        let observed_horizon = field_usize(&entry["horizon_steps"], &format!("{prefix}.horizon[{horizon}].horizon_steps"))?;
        if observed_horizon != horizon {
            return Err(invalid_input(format!(
                "{prefix} horizon index {} contains horizon_steps={observed_horizon}",
                horizon - 1
            )));
        }
        let samples = field_usize(&entry["sample_count"], &format!("{prefix}.horizon[{horizon}].sample_count"))?;
        if samples != row_count {
            return Err(invalid_input(format!(
                "{prefix} horizon {horizon} scored {samples} windows but receipt declares {row_count} rows"
            )));
        }
        for channel in CHANNELS {
            let metric_value = field_f64(
                &entry["metrics"][channel.key],
                &format!("{prefix}.horizon[{horizon}].metrics.{}", channel.key),
            )?;
            if metric_value < 0.0 {
                return Err(invalid_input(format!(
                    "{prefix}.horizon[{horizon}].metrics.{} must be non-negative",
                    channel.key
                )));
            }
        }
    }
    Ok(RunReceipt {
        run_id,
        row_count,
        orientation,
        truth_digest,
        complete_metric,
    })
}

fn load_receipt(path: &Path) -> Result<LoadedReceipt, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    if bytes.is_empty() {
        return Err(invalid_input(format!("{} is empty", path.display())).into());
    }
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let value: Value = serde_json::from_slice(&bytes)?;
    if field_str(&value["benchmark_id"], "benchmark_id")? != "symtropy-nanodrone-multihorizon-metrics-v0.1" {
        return Err(invalid_input(format!(
            "{} is not a supported symtropy-nanodrone-multihorizon-metrics-v0.1 receipt",
            path.display()
        )).into());
    }
    let protocol = &value["protocol"];
    if field_str(&protocol["held_out_trajectory"], "protocol.held_out_trajectory")? != "melon" {
        return Err(invalid_input(format!("{} does not evaluate the held-out Melon trajectory", path.display())).into());
    }
    let horizon_count = field_usize(&protocol["evaluated_horizons"], "protocol.evaluated_horizons")?;
    if horizon_count != HORIZON {
        return Err(invalid_input(format!("{} evaluated {horizon_count} horizons; expected {HORIZON}", path.display())).into());
    }
    let sample_period = field_f64(&protocol["sample_period_s"], "protocol.sample_period_s")?;
    if (sample_period - EXPECTED_DT_S).abs() > 1.0e-9 {
        return Err(invalid_input(format!(
            "{} uses sample_period_s={sample_period}; expected {EXPECTED_DT_S}",
            path.display()
        )).into());
    }
    if value["macro_mean_all_available_windows"].is_null() {
        return Err(invalid_input(format!(
            "{} lacks complete-window aggregate metrics; legacy shifted-truth scores are not accepted for paired comparison",
            path.display()
        )).into());
    }
    let raw_runs = value["per_run"].as_array()
        .ok_or_else(|| invalid_input(format!("{}.per_run must be an array", path.display())))?;
    if raw_runs.len() != RUN_IDS.len() {
        return Err(invalid_input(format!(
            "{} must contain exactly {} held-out runs, found {}",
            path.display(), RUN_IDS.len(), raw_runs.len()
        )).into());
    }
    let mut runs = BTreeMap::new();
    let mut run_orientation: Option<String> = None;
    for (index, raw_run) in raw_runs.iter().enumerate() {
        let run = parse_run(raw_run, index)?;
        if !RUN_IDS.contains(&run.run_id.as_str()) {
            return Err(invalid_input(format!("unexpected held-out run identity '{}'", run.run_id)).into());
        }
        if runs.insert(run.run_id.clone(), run.clone()).is_some() {
            return Err(invalid_input(format!("duplicate held-out run '{}'", run.run_id)).into());
        }
        match &run_orientation {
            None => run_orientation = Some(run.orientation.clone()),
            Some(existing) if existing == &run.orientation => {}
            Some(_) => return Err(invalid_input(format!("{} mixes orientation representations across runs", path.display())).into()),
        }
    }
    if RUN_IDS.iter().any(|run_id| !runs.contains_key(*run_id)) {
        return Err(invalid_input(format!("{} must have run1, run2, and run3", path.display())).into());
    }
    Ok(LoadedReceipt {
        path: path.display().to_string(),
        sha256: digest,
        orientation: run_orientation.ok_or_else(|| invalid_input("empty run set"))?,
        runs,
    })
}

fn metric(run: &RunReceipt, horizon: usize, channel: Channel) -> Result<f64, io::Error> {
    field_f64(
        &run.complete_metric["per_horizon"][horizon - 1]["metrics"][channel.key],
        &format!("{}.horizon[{horizon}].{}", run.run_id, channel.key),
    )
}

fn metric_compare(reference: f64, candidate: f64) -> Value {
    let delta = candidate - reference;
    json!({
        "reference": reference,
        "candidate": candidate,
        "absolute_delta_candidate_minus_reference": delta,
        "relative_error_change_pct": if reference.abs() > f64::EPSILON {
            json!(delta / reference.abs() * 100.0)
        } else {
            Value::Null
        },
        "lower_is_better": true
    })
}

fn compare_receipts(reference: &LoadedReceipt, candidate: &LoadedReceipt) -> Result<Value, io::Error> {
    if reference.orientation != candidate.orientation {
        return Err(invalid_input(format!(
            "orientation representation mismatch: reference={} candidate={}",
            reference.orientation, candidate.orientation
        )));
    }
    for run_id in RUN_IDS {
        let left = &reference.runs[run_id];
        let right = &candidate.runs[run_id];
        if left.row_count != right.row_count {
            return Err(invalid_input(format!(
                "{run_id} row-count mismatch: reference={} candidate={}",
                left.row_count, right.row_count
            )));
        }
        if left.truth_digest != right.truth_digest {
            return Err(invalid_input(format!(
                "{run_id} ground-truth-window digest mismatch; refusing to compare models on non-identical targets"
            )));
        }
    }

    let mut per_horizon = Vec::with_capacity(HORIZON);
    let mut sum_by_channel: BTreeMap<&str, (f64, f64)> = BTreeMap::new();
    for channel in CHANNELS {
        sum_by_channel.insert(channel.key, (0.0, 0.0));
    }

    for horizon in 1..=HORIZON {
        let mut channels_json = serde_json::Map::new();
        for channel in CHANNELS {
            let mut reference_values = Vec::with_capacity(RUN_IDS.len());
            let mut candidate_values = Vec::with_capacity(RUN_IDS.len());
            let mut paired_runs = Vec::with_capacity(RUN_IDS.len());
            for run_id in RUN_IDS {
                let left = metric(&reference.runs[run_id], horizon, channel)?;
                let right = metric(&candidate.runs[run_id], horizon, channel)?;
                reference_values.push(left);
                candidate_values.push(right);
                paired_runs.push(json!({
                    "run_id": run_id,
                    "reference": left,
                    "candidate": right,
                    "absolute_delta_candidate_minus_reference": right - left,
                    "relative_error_change_pct": if left.abs() > f64::EPSILON {
                        json!((right - left) / left.abs() * 100.0)
                    } else {
                        Value::Null
                    }
                }));
            }
            let reference_macro = reference_values.iter().sum::<f64>() / RUN_IDS.len() as f64;
            let candidate_macro = candidate_values.iter().sum::<f64>() / RUN_IDS.len() as f64;
            let entry = metric_compare(reference_macro, candidate_macro);
            let mut entry_object = entry.as_object().cloned().unwrap_or_default();
            entry_object.insert("unit".to_owned(), json!(channel.unit));
            entry_object.insert("paired_run_deltas".to_owned(), json!(paired_runs));
            channels_json.insert(channel.key.to_owned(), Value::Object(entry_object));
            let sum = sum_by_channel.get_mut(channel.key).expect("all channel sums initialized");
            sum.0 += reference_macro;
            sum.1 += candidate_macro;
        }
        per_horizon.push(json!({
            "horizon_steps": horizon,
            "horizon_seconds": horizon as f64 * EXPECTED_DT_S,
            "independent_run_count": RUN_IDS.len(),
            "windows_are_nested_within_runs_not_independent_statistical_samples": true,
            "metrics": Value::Object(channels_json)
        }));
    }

    let mut summary_by_metric = serde_json::Map::new();
    for channel in CHANNELS {
        let (reference_sum, candidate_sum) = sum_by_channel[channel.key];
        let reference_mean = reference_sum / HORIZON as f64;
        let candidate_mean = candidate_sum / HORIZON as f64;
        let mut summary = metric_compare(reference_mean, candidate_mean).as_object().cloned().unwrap_or_default();
        summary.insert("unit".to_owned(), json!(channel.unit));
        summary.insert("summary_definition".to_owned(), json!(
            "unweighted average of per-horizon macro means h=1..50; descriptive only, not a cross-unit composite"
        ));
        summary_by_metric.insert(channel.key.to_owned(), Value::Object(summary));
    }

    let per_run_metadata = RUN_IDS.iter().map(|run_id| {
        let run = &reference.runs[*run_id];
        json!({
            "run_id": run_id,
            "row_count": run.row_count,
            "ground_truth_window_sha256": run.truth_digest,
            "orientation_representation": run.orientation
        })
    }).collect::<Vec<_>>();

    let summary_horizons = SUMMARY_HORIZONS.iter().map(|horizon| {
        let value = &per_horizon[*horizon - 1];
        json!({
            "horizon_steps": *horizon,
            "horizon_seconds": *horizon as f64 * EXPECTED_DT_S,
            "metrics": value["metrics"]
        })
    }).collect::<Vec<_>>();

    Ok(json!({
        "comparison_id": "symtropy-nanodrone-paired-model-comparison-v0.1",
        "claim_ceiling": "paired_prediction_error_comparison_on_identical_ground_truth_windows_only; no statistical_significance, generalization, control, or safety claim",
        "inputs": {
            "reference": { "path": reference.path, "sha256": reference.sha256 },
            "candidate": { "path": candidate.path, "sha256": candidate.sha256 }
        },
        "protocol": {
            "benchmark_id": "symtropy-nanodrone-multihorizon-metrics-v0.1",
            "held_out_trajectory": "melon",
            "runs": RUN_IDS,
            "horizons": HORIZON,
            "sample_period_s": EXPECTED_DT_S,
            "orientation_representation": reference.orientation,
            "pairing_rule": "refuse comparison unless per-run canonical ground_truth_window_sha256, row count, and orientation representation match exactly",
            "aggregation": "score each run and horizon independently, then macro-average the three per-run values with equal run weights",
            "reported_metric_channels": "keep position, velocity, orientation, and angular rate separate; no unit-mixed aggregate score",
            "uncertainty": "not estimated; three fixed held-out runs are reported as paired strata, not sufficient independent trials for a strong significance claim"
        },
        "paired_run_manifest": per_run_metadata,
        "summary_horizons": summary_horizons,
        "summary_by_metric": Value::Object(summary_by_metric),
        "per_horizon": per_horizon,
        "interpretation": {
            "relative_error_change_pct": "negative means the candidate has lower error than the reference; positive means higher error",
            "baseline_zero": "relative change is null when the reference metric is effectively zero; absolute delta remains reported",
            "decision_rule": "declare a win only after prespecified per-channel and horizon constraints are met; this receipt alone does not define a composite winner"
        }
    }))
}

fn write_output_new(path: &Path, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    if path.exists() {
        return Err(invalid_input(format!(
            "output '{}' already exists; choose a fresh path to prevent stale-result mixing",
            path.display()
        )).into());
    }
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let temp_path = path.with_extension(format!("tmp-{}-{nonce}", std::process::id()));
    let write_result = fs::write(&temp_path, bytes);
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temp_path);
        return Err(error.into());
    }
    if let Err(error) = fs::rename(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);
        return Err(error.into());
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 3 {
        return Err(invalid_input(
            "usage: nanodrone_model_compare <reference_metric_receipt.json> <candidate_metric_receipt.json> <new_output_comparison.json>",
        ).into());
    }
    let reference = load_receipt(Path::new(&args[0]))?;
    let candidate = load_receipt(Path::new(&args[1]))?;
    let comparison = compare_receipts(&reference, &candidate)?;
    let bytes = serde_json::to_vec_pretty(&comparison)?;
    write_output_new(Path::new(&args[2]), &bytes)?;
    println!("{}", String::from_utf8(bytes)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_run(run_id: &str, value: f64, truth_digest: &str) -> RunReceipt {
        let mut horizons = Vec::new();
        for horizon in 1..=HORIZON {
            let mut metrics = serde_json::Map::new();
            for channel in CHANNELS {
                metrics.insert(channel.key.to_owned(), json!(value));
            }
            horizons.push(json!({
                "horizon_steps": horizon,
                "sample_count": 60,
                "metrics": metrics
            }));
        }
        RunReceipt {
            run_id: run_id.to_owned(),
            row_count: 60,
            orientation: "so3_log_rotation_vector".to_owned(),
            truth_digest: truth_digest.to_owned(),
            complete_metric: json!({ "per_horizon": horizons }),
        }
    }

    fn fake_receipt(value: f64, truth_digest: &str) -> LoadedReceipt {
        let runs = RUN_IDS.iter().map(|run_id| {
            ((*run_id).to_owned(), fake_run(run_id, value, truth_digest))
        }).collect::<BTreeMap<_, _>>();
        LoadedReceipt {
            path: "fixture.json".to_owned(),
            sha256: "a".repeat(64),
            orientation: "so3_log_rotation_vector".to_owned(),
            runs,
        }
    }

    #[test]
    fn identical_targets_allow_paired_comparison_and_report_improvement() {
        let reference = fake_receipt(2.0, &"b".repeat(64));
        let candidate = fake_receipt(1.5, &"b".repeat(64));
        let output = compare_receipts(&reference, &candidate).unwrap();
        assert_eq!(output["per_horizon"].as_array().unwrap().len(), 50);
        assert_eq!(
            output["per_horizon"][0]["metrics"][CHANNELS[0].key]["relative_error_change_pct"]
                .as_f64().unwrap(),
            -25.0
        );
        assert!(output["protocol"]["uncertainty"].as_str().unwrap().contains("three fixed held-out runs"));
    }

    #[test]
    fn mismatched_truth_digest_fails_closed() {
        let reference = fake_receipt(2.0, &"b".repeat(64));
        let candidate = fake_receipt(1.5, &"c".repeat(64));
        let error = compare_receipts(&reference, &candidate).unwrap_err();
        assert!(error.to_string().contains("ground-truth-window digest mismatch"));
    }

    #[test]
    fn unit_mixed_composite_is_not_created() {
        let reference = fake_receipt(2.0, &"b".repeat(64));
        let candidate = fake_receipt(1.5, &"b".repeat(64));
        let output = compare_receipts(&reference, &candidate).unwrap();
        assert!(output["summary_by_metric"]["position_mean_euclidean_error_m"].is_object());
        assert!(output.get("overall_score").is_none());
    }
}
