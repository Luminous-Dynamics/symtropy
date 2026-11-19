use chrono::{DateTime, FixedOffset};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const ENGINE_VERSION: &str = "marine-forecast-information-noise/0.1.0";
const WEIGHT_TOLERANCE: f64 = 1e-6;
const DECOMPOSITION_TOLERANCE: f64 = 1e-8;

fn string_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn number_at(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64).filter(|number| number.is_finite())
}

fn valid_digest(value: Option<&str>) -> bool {
    value.is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

fn parse_time(value: Option<&str>) -> Option<DateTime<FixedOffset>> {
    value.and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
}

#[derive(Clone)]
struct Cell {
    id: String,
    weight: f64,
    forecast: f64,
    observed: f64,
    climatology: f64,
}

#[derive(Clone)]
struct Metrics {
    case_id: String,
    pair_id: String,
    variant: String,
    rmse: f64,
    sdaf: f64,
    sdav: f64,
    acc: Option<f64>,
    fi: f64,
    ie: f64,
    ne: f64,
    decomposition_residual: f64,
}

fn compute_metrics(case: &Value, errors: &mut Vec<String>) -> Option<Metrics> {
    let case_id = string_at(case, "field_case_id").unwrap_or("unknown").to_owned();
    let pair_id = string_at(case, "pair_id").unwrap_or("").to_owned();
    let variant = string_at(case, "variant").unwrap_or("").to_owned();
    let cell_values = case.get("cells").and_then(Value::as_array)?;
    if cell_values.len() < 3 {
        errors.push(format!("field case '{case_id}' requires at least three verification cells"));
        return None;
    }
    let mut cells = Vec::<Cell>::new();
    let mut seen_cells = BTreeSet::<String>::new();
    for (index, value) in cell_values.iter().enumerate() {
        let id = string_at(value, "cell_id").unwrap_or("").to_owned();
        let weight = number_at(value, "weight");
        let forecast = number_at(value, "forecast_value");
        let observed = number_at(value, "observed_value");
        let climatology = number_at(value, "climatology_value");
        match (weight, forecast, observed, climatology) {
            (Some(weight), Some(forecast), Some(observed), Some(climatology))
                if weight > 0.0 && !id.is_empty() && seen_cells.insert(id.clone()) =>
            {
                cells.push(Cell { id, weight, forecast, observed, climatology });
            }
            _ => errors.push(format!("field case '{case_id}' cell[{index}] has missing, duplicated or invalid inputs")),
        }
    }
    if cells.len() != cell_values.len() {
        return None;
    }
    let weight_sum = cells.iter().map(|cell| cell.weight).sum::<f64>();
    if (weight_sum - 1.0).abs() > WEIGHT_TOLERANCE {
        errors.push(format!("field case '{case_id}' area weights must sum to one"));
        return None;
    }

    let forecast_anomalies = cells.iter().map(|cell| cell.forecast - cell.climatology).collect::<Vec<_>>();
    let truth_anomalies = cells.iter().map(|cell| cell.observed - cell.climatology).collect::<Vec<_>>();
    let forecast_mean = cells.iter().zip(&forecast_anomalies)
        .map(|(cell, anomaly)| cell.weight * anomaly).sum::<f64>();
    let truth_mean = cells.iter().zip(&truth_anomalies)
        .map(|(cell, anomaly)| cell.weight * anomaly).sum::<f64>();
    let forecast_debiased = forecast_anomalies.iter().map(|anomaly| anomaly - forecast_mean).collect::<Vec<_>>();
    let truth_debiased = truth_anomalies.iter().map(|anomaly| anomaly - truth_mean).collect::<Vec<_>>();

    let sdaf_squared = cells.iter().zip(&forecast_debiased)
        .map(|(cell, anomaly)| cell.weight * anomaly.powi(2)).sum::<f64>();
    let sdav_squared = cells.iter().zip(&truth_debiased)
        .map(|(cell, anomaly)| cell.weight * anomaly.powi(2)).sum::<f64>();
    if sdav_squared <= 1e-12 {
        errors.push(format!("field case '{case_id}' verifying anomaly variance is degenerate; FI is undefined"));
        return None;
    }
    let dot = cells.iter().zip(&forecast_debiased).zip(&truth_debiased)
        .map(|((cell, forecast), truth)| cell.weight * forecast * truth).sum::<f64>();
    let sdaf = sdaf_squared.sqrt();
    let sdav = sdav_squared.sqrt();
    let fi = dot / sdav_squared;
    let ie = (1.0 - fi).abs() * sdav;
    let noise_squared = cells.iter().zip(&forecast_debiased).zip(&truth_debiased)
        .map(|((cell, forecast), truth)| {
            cell.weight * (forecast - fi * truth).powi(2)
        }).sum::<f64>();
    let ne = noise_squared.max(0.0).sqrt();
    let rmse_squared = cells.iter().zip(&forecast_debiased).zip(&truth_debiased)
        .map(|((cell, forecast), truth)| cell.weight * (forecast - truth).powi(2)).sum::<f64>();
    let rmse = rmse_squared.max(0.0).sqrt();
    let decomposition_residual = (rmse_squared - ie.powi(2) - ne.powi(2)).abs();
    let tolerance = DECOMPOSITION_TOLERANCE * rmse_squared.max(1.0);
    if decomposition_residual > tolerance {
        errors.push(format!("field case '{case_id}' violates the information/noise orthogonal decomposition"));
    }
    let acc = if sdaf > 1e-12 { Some(dot / (sdaf * sdav)) } else { None };
    let _cell_identity_digest_input = cells.iter().map(|cell| cell.id.as_str()).collect::<Vec<_>>();

    Some(Metrics {
        case_id, pair_id, variant, rmse, sdaf, sdav, acc, fi, ie, ne, decomposition_residual,
    })
}

fn same_verification_support(left: &Value, right: &Value) -> bool {
    for field in [
        "fold", "episode_id", "voyage_id", "weather_event_id", "variable",
        "lead_hours", "valid_time", "verification_valid_time", "verifying_source_id",
        "verifying_source_class", "verifying_source_sha256", "grid_id", "weighting_method",
        "support_sha256", "climatology_sha256",
    ] {
        if left.get(field) != right.get(field) {
            return false;
        }
    }
    let left_cells = left.get("cells").and_then(Value::as_array);
    let right_cells = right.get("cells").and_then(Value::as_array);
    match (left_cells, right_cells) {
        (Some(left_cells), Some(right_cells)) if left_cells.len() == right_cells.len() => {
            left_cells.iter().zip(right_cells).all(|(left_cell, right_cell)| {
                left_cell.get("cell_id") == right_cell.get("cell_id")
                    && left_cell.get("weight") == right_cell.get("weight")
                    && left_cell.get("observed_value") == right_cell.get("observed_value")
                    && left_cell.get("climatology_value") == right_cell.get("climatology_value")
            })
        }
        _ => false,
    }
}

fn validate(raw: &[u8], value: &Value) -> Value {
    let mut errors = Vec::<String>::new();
    let mut blockers = Vec::<String>::new();
    if string_at(value, "schema_version") != Some("0.1.0") {
        errors.push("schema_version must be 0.1.0".to_owned());
    }
    if string_at(value, "scorecard_id").is_none_or(str::is_empty) {
        errors.push("scorecard_id is required".to_owned());
    }
    if string_at(value, "claim_ceiling") != Some("field_information_noise_diagnostics_only") {
        errors.push("claim_ceiling must be field_information_noise_diagnostics_only".to_owned());
    }
    let status = string_at(value, "record_status").unwrap_or("");
    if !["synthetic", "shadow_replay", "measured_campaign", "published_reproduction"].contains(&status) {
        errors.push("record_status is missing or unsupported".to_owned());
    }
    let dataset = value.get("dataset").unwrap_or(&Value::Null);
    for field in ["dataset_sha256", "split_manifest_sha256"] {
        if !valid_digest(string_at(dataset, field)) {
            errors.push(format!("dataset identity '{field}' must be a SHA-256 digest"));
        }
    }
    if string_at(dataset, "split_strategy") != Some("forward_chaining_joint_grouped")
        || string_at(dataset, "grouping_unit") != Some("joint_voyage_weather_event")
    {
        errors.push("field verification requires forward-chaining joint voyage/weather-event grouping".to_owned());
    }

    let fields = value.get("field_cases").and_then(Value::as_array);
    if fields.is_none_or(Vec::is_empty) {
        errors.push("field_cases must be a non-empty array".to_owned());
    }
    let mut cases = BTreeMap::<String, Value>::new();
    let mut folds = BTreeMap::<String, String>::new();
    let mut results = BTreeMap::<String, Metrics>::new();
    if let Some(fields) = fields {
        for (index, field) in fields.iter().enumerate() {
            let id = string_at(field, "field_case_id").unwrap_or("").to_owned();
            if id.is_empty() || cases.contains_key(&id) {
                errors.push(format!("field_cases[{index}] has a missing or duplicate field_case_id"));
            } else {
                cases.insert(id.clone(), field.clone());
            }
            for (name, prefix) in [
                ("episode_id", "episode"),
                ("voyage_id", "voyage"),
                ("weather_event_id", "weather-event"),
            ] {
                let identity = string_at(field, name).unwrap_or("");
                let fold = string_at(field, "fold").unwrap_or("");
                if identity.is_empty() {
                    errors.push(format!("field case '{id}' is missing {name}"));
                } else if let Some(previous) = folds.insert(format!("{prefix}:{identity}"), fold.to_owned()) {
                    if previous != fold {
                        blockers.push(format!("{name} '{identity}' crosses held-out folds"));
                    }
                }
            }
            for name in [
                "forecast_artifact_sha256", "availability_receipt_sha256",
                "verifying_source_sha256", "support_sha256", "climatology_sha256",
            ] {
                if !valid_digest(string_at(field, name)) {
                    errors.push(format!("field case '{id}' has invalid {name}"));
                }
            }
            if string_at(field, "forecast_source_id").is_none_or(str::is_empty)
                || string_at(field, "cycle_id").is_none_or(str::is_empty)
                || string_at(field, "availability_receipt_ref").is_none_or(str::is_empty)
                || string_at(field, "grid_id").is_none_or(str::is_empty)
            {
                errors.push(format!("field case '{id}' lacks source, cycle, receipt or grid identity"));
            }
            if string_at(field, "weighting_method") != Some("normalized_cell_area") {
                errors.push(format!("field case '{id}' must use normalized cell-area weights"));
            }
            if string_at(field, "forecast_source_id") == string_at(field, "verifying_source_id") {
                errors.push(format!("field case '{id}' cannot verify a source against itself"));
            }
            if !["in_situ_validation", "realized_reanalysis"].contains(&string_at(field, "verifying_source_class").unwrap_or("")) {
                errors.push(format!("field case '{id}' requires evaluator-only verification source"));
            }
            match (
                parse_time(string_at(field, "forecast_issue_time")),
                parse_time(string_at(field, "first_available_at")),
                parse_time(string_at(field, "decision_cutoff")),
                parse_time(string_at(field, "valid_time")),
                parse_time(string_at(field, "verification_valid_time")),
            ) {
                (Some(issue), Some(available), Some(cutoff), Some(valid), Some(verified)) => {
                    if issue > available {
                        errors.push(format!("field case '{id}' first availability precedes forecast issue time"));
                    }
                    if available > cutoff {
                        blockers.push(format!("field case '{id}' forecast was unavailable at decision cutoff"));
                    }
                    if valid <= cutoff || valid != verified {
                        errors.push(format!("field case '{id}' valid and verification times must agree and follow decision cutoff"));
                    }
                }
                _ => errors.push(format!("field case '{id}' contains invalid RFC3339 time fields")),
            }
            if let Some(metrics) = compute_metrics(field, &mut errors) {
                results.insert(id, metrics);
            }
        }
    }

    let mut pair_members = BTreeMap::<String, Vec<String>>::new();
    for (id, field) in &cases {
        let pair_id = string_at(field, "pair_id").unwrap_or("");
        if pair_id.is_empty() {
            errors.push(format!("field case '{id}' requires pair_id"));
        } else {
            pair_members.entry(pair_id.to_owned()).or_default().push(id.clone());
        }
    }
    let mut comparisons = Vec::<Value>::new();
    for (pair_id, ids) in pair_members {
        if ids.len() != 2 {
            errors.push(format!("smoothing pair '{pair_id}' must contain exactly two field cases"));
            continue;
        }
        let left = &cases[&ids[0]];
        let right = &cases[&ids[1]];
        let variants = [
            string_at(left, "variant").unwrap_or(""),
            string_at(right, "variant").unwrap_or(""),
        ];
        let (raw_id, damped_id) = match variants {
            ["raw", "damped"] => (&ids[0], &ids[1]),
            ["damped", "raw"] => (&ids[1], &ids[0]),
            _ => {
                errors.push(format!("smoothing pair '{pair_id}' must contain one raw and one damped field"));
                continue;
            }
        };
        if !same_verification_support(&cases[raw_id], &cases[damped_id]) {
            errors.push(format!("smoothing pair '{pair_id}' does not use exactly matched verification support, truth, climatology and folds"));
            continue;
        }
        let (Some(raw_metrics), Some(damped_metrics)) = (results.get(raw_id), results.get(damped_id)) else {
            errors.push(format!("smoothing pair '{pair_id}' is missing valid metric results"));
            continue;
        };
        comparisons.push(json!({
            "pair_id":pair_id,
            "raw_case_id":raw_id,
            "damped_case_id":damped_id,
            "raw_rmse":raw_metrics.rmse,
            "damped_rmse":damped_metrics.rmse,
            "rmse_improvement_pct":(raw_metrics.rmse-damped_metrics.rmse)/raw_metrics.rmse.max(1e-12)*100.0,
            "raw_forecast_information":raw_metrics.fi,
            "damped_forecast_information":damped_metrics.fi,
            "raw_information_error":raw_metrics.ie,
            "damped_information_error":damped_metrics.ie,
            "raw_noise_error":raw_metrics.ne,
            "damped_noise_error":damped_metrics.ne,
            "rmse_improves_while_information_degrades":damped_metrics.rmse < raw_metrics.rmse && damped_metrics.fi.abs() < raw_metrics.fi.abs() && damped_metrics.ie > raw_metrics.ie
        }));
    }

    let metric_reports = results.values().map(|metrics| json!({
        "field_case_id":metrics.case_id,
        "pair_id":metrics.pair_id,
        "variant":metrics.variant,
        "debiased_area_weighted_rmse":metrics.rmse,
        "forecast_activity_sdaf":metrics.sdaf,
        "verification_activity_sdav":metrics.sdav,
        "anomaly_correlation":metrics.acc,
        "forecast_information_fi":metrics.fi,
        "information_error_ie":metrics.ie,
        "noise_error_ne":metrics.ne,
        "orthogonal_decomposition_residual":metrics.decomposition_residual
    })).collect::<Vec<_>>();
    let smoothing_gaming_cases = comparisons.iter()
        .filter(|comparison| comparison["rmse_improves_while_information_degrades"] == true)
        .count();
    let disposition = if !errors.is_empty() {
        "INVALID"
    } else if !blockers.is_empty() {
        "BLOCKED"
    } else if status == "synthetic" {
        "SYNTHETIC_ONLY"
    } else {
        "FIELD_INFORMATION_AUDITED_READ_ONLY"
    };
    let digest = Sha256::digest(raw).iter().map(|byte| format!("{byte:02x}")).collect::<String>();
    json!({
        "engine_version":ENGINE_VERSION,
        "scorecard_id":string_at(value,"scorecard_id"),
        "disposition":disposition,
        "input_sha256":digest,
        "field_case_count":results.len(),
        "smoothing_pair_count":comparisons.len(),
        "smoothing_rmse_gaming_case_count":smoothing_gaming_cases,
        "field_metrics":metric_reports,
        "smoothing_comparisons":comparisons,
        "findings":{"errors":errors,"blockers":blockers},
        "claim_ceiling":"debiased_area_weighted_field_information_noise_diagnostics_only",
        "note":"FI, IE and NE are field/cycle diagnostics, not scalar vessel-route scores. The fixture is synthetic. Report field/cycle results before any aggregation; do not let route-selected support or smoothing conceal a loss of information."
    })
}

fn run() -> Result<(), String> {
    let arguments = env::args().collect::<Vec<_>>();
    if arguments.len() != 3 || arguments[1] != "audit-information-noise" {
        return Err("Usage: forecast-information-noise audit-information-noise <fixture.json>".to_owned());
    }
    let raw = fs::read(&arguments[2]).map_err(|error| format!("{}: {error}", arguments[2]))?;
    let value: Value = serde_json::from_slice(&raw)
        .map_err(|error| format!("{}: invalid JSON: {error}", arguments[2]))?;
    let report = validate(&raw, &value);
    println!("{}", serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?);
    match string_at(&report, "disposition") {
        Some("SYNTHETIC_ONLY" | "FIELD_INFORMATION_AUDITED_READ_ONLY") => Ok(()),
        Some(disposition) => Err(format!("information/noise audit disposition: {disposition}")),
        None => Err("information/noise audit report has no disposition".to_owned()),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        serde_json::from_str(include_str!("../../fixtures/synthetic-information-noise-0001.json"))
            .expect("synthetic information/noise fixture parses")
    }

    fn audit(value: &Value) -> Value {
        let raw = serde_json::to_vec(value).expect("fixture serializes");
        validate(&raw, value)
    }

    #[test]
    fn synthetic_fixture_computes_information_noise_decomposition() {
        let report = audit(&fixture());
        assert_eq!(report["disposition"], "SYNTHETIC_ONLY");
        assert_eq!(report["field_case_count"], 2);
        assert_eq!(report["smoothing_pair_count"], 1);
        for field in report["field_metrics"].as_array().unwrap() {
            assert!(field["orthogonal_decomposition_residual"].as_f64().unwrap() < 1e-8);
        }
    }

    #[test]
    fn smoothing_can_improve_rmse_while_worsening_information() {
        let report = audit(&fixture());
        let comparison = &report["smoothing_comparisons"][0];
        assert_eq!(comparison["rmse_improves_while_information_degrades"], true);
        assert!(comparison["damped_rmse"].as_f64().unwrap() < comparison["raw_rmse"].as_f64().unwrap());
        assert!(comparison["damped_forecast_information"].as_f64().unwrap()
            < comparison["raw_forecast_information"].as_f64().unwrap());
        assert!(comparison["damped_information_error"].as_f64().unwrap()
            > comparison["raw_information_error"].as_f64().unwrap());
    }

    #[test]
    fn first_availability_after_decision_cutoff_is_blocked() {
        let mut value = fixture();
        value["field_cases"][0]["first_available_at"] = json!("2025-01-01T00:40:00Z");
        assert_eq!(audit(&value)["disposition"], "BLOCKED");
    }

    #[test]
    fn field_pair_must_share_exact_truth_and_area_support() {
        let mut value = fixture();
        value["field_cases"][1]["cells"][1]["observed_value"] = json!(0.2);
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn non_normalized_area_weights_are_rejected() {
        let mut value = fixture();
        value["field_cases"][0]["cells"][0]["weight"] = json!(0.5);
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn degenerate_truth_anomaly_variance_is_invalid() {
        let mut value = fixture();
        for cell in value["field_cases"][0]["cells"].as_array_mut().unwrap() {
            cell["observed_value"] = cell["climatology_value"].clone();
        }
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn verification_time_must_match_forecast_valid_time() {
        let mut value = fixture();
        value["field_cases"][0]["verification_valid_time"] = json!("2025-01-01T07:00:00Z");
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }
}
