use chrono::{DateTime, Duration, FixedOffset};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const ENGINE_VERSION: &str = "marine-forecast-scorecard/0.1.0";

fn string_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn number_at(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64).filter(|number| number.is_finite())
}

fn count_at(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}

fn valid_digest(value: Option<&str>) -> bool {
    value.is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

fn parse_time(
    value: Option<&str>,
    label: &str,
    errors: &mut Vec<String>,
) -> Option<DateTime<FixedOffset>> {
    match value.and_then(|raw| DateTime::parse_from_rfc3339(raw).ok()) {
        Some(parsed) => Some(parsed),
        None => {
            errors.push(format!("{label} must be an RFC3339 timestamp with an explicit offset"));
            None
        }
    }
}

fn empirical_crps(members: &[f64], observation: f64) -> f64 {
    let n = members.len() as f64;
    let observation_term = members.iter().map(|member| (*member - observation).abs()).sum::<f64>() / n;
    let pair_term = members.iter().flat_map(|left| {
        members.iter().map(move |right| (*left - *right).abs())
    }).sum::<f64>() / (2.0 * n * n);
    observation_term - pair_term
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn cvar95(values: &[f64]) -> (f64, usize) {
    if values.is_empty() {
        return (0.0, 0);
    }
    let mut ordered = values.to_vec();
    ordered.sort_by(|left, right| right.total_cmp(left));
    let tail_count = ((ordered.len() as f64 * 0.05).ceil() as usize).max(1);
    (mean(&ordered[..tail_count]), tail_count)
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
    if string_at(value, "claim_ceiling") != Some("descriptive_skill_and_paired_outcome_metrics_only") {
        errors.push("claim_ceiling must preserve the descriptive-only boundary".to_owned());
    }
    let record_status = string_at(value, "record_status").unwrap_or("");
    if !["synthetic", "shadow_replay", "measured_campaign", "published_reproduction"].contains(&record_status) {
        errors.push("record_status is missing or unsupported".to_owned());
    }

    let dataset = value.get("dataset").unwrap_or(&Value::Null);
    if string_at(dataset, "dataset_id").is_none_or(str::is_empty) {
        errors.push("dataset_id is required".to_owned());
    }
    for (key, label) in [
        ("dataset_sha256", "dataset"),
        ("split_manifest_sha256", "split manifest"),
        ("transform_sha256", "transform"),
    ] {
        if !valid_digest(string_at(dataset, key)) {
            errors.push(format!("{label} identity must be a 64-character SHA-256 digest"));
        }
    }
    if string_at(dataset, "split_strategy") != Some("forward_chaining_joint_grouped")
        || string_at(dataset, "grouping_unit") != Some("joint_voyage_weather_event")
    {
        errors.push("scorecard requires forward-chaining joint voyage/weather-event grouping".to_owned());
    }
    let bin_count = value.get("calibration_bin_count").and_then(Value::as_u64)
        .filter(|count| (2..=20).contains(count)).unwrap_or(10) as usize;

    let mut case_by_id: HashMap<String, Value> = HashMap::new();
    let mut group_folds: HashMap<String, String> = HashMap::new();
    let mut test_strata = BTreeSet::<String>::new();
    let mut skill: BTreeMap<String, Vec<(f64, f64, f64)>> = BTreeMap::new();
    let mut events: BTreeMap<String, Vec<(f64, bool)>> = BTreeMap::new();

    let cases = value.get("forecast_cases").and_then(Value::as_array);
    if cases.is_none_or(Vec::is_empty) {
        errors.push("forecast_cases must be a non-empty array".to_owned());
    }
    if let Some(cases) = cases {
        for (index, case) in cases.iter().enumerate() {
            let case_id = string_at(case, "case_id").unwrap_or("").to_owned();
            let fold = string_at(case, "fold").unwrap_or("").to_owned();
            if case_id.is_empty() || !case_by_id.insert(case_id.clone(), case.clone()).is_none() {
                errors.push(format!("forecast_cases[{index}] has a missing or duplicated case_id"));
            }
            if !["development", "validation", "test"].contains(&fold.as_str()) {
                errors.push(format!("forecast case '{case_id}' has an unsupported fold"));
            }
            for (field, prefix) in [
                ("episode_id", "episode"),
                ("voyage_id", "voyage"),
                ("weather_event_id", "weather-event"),
            ] {
                let group_id = string_at(case, field).unwrap_or("");
                if group_id.is_empty() {
                    errors.push(format!("forecast case '{case_id}' requires {field}"));
                } else {
                    let key = format!("{prefix}:{group_id}");
                    if let Some(previous_fold) = group_folds.insert(key, fold.clone()) {
                        if previous_fold != fold {
                            blockers.push(format!("{field} '{group_id}' crosses held-out folds"));
                        }
                    }
                }
            }
            if fold == "test" {
                if let Some(strata) = case.get("environment_strata").and_then(Value::as_array) {
                    for stratum in strata.iter().filter_map(Value::as_str) {
                        test_strata.insert(stratum.to_owned());
                    }
                }
            }
            for field in ["forecast_sha256", "availability_receipt_sha256", "verifying_source_sha256"] {
                if !valid_digest(string_at(case, field)) {
                    errors.push(format!("forecast case '{case_id}' has invalid {field}"));
                }
            }
            if string_at(case, "availability_receipt_ref").is_none_or(str::is_empty) {
                errors.push(format!("forecast case '{case_id}' lacks first-availability receipt reference"));
            }
            let issue = parse_time(string_at(case, "forecast_issue_time"), "forecast_issue_time", &mut errors);
            let available = parse_time(string_at(case, "first_available_at"), "first_available_at", &mut errors);
            let cutoff = parse_time(string_at(case, "decision_cutoff"), "decision_cutoff", &mut errors);
            let valid = parse_time(string_at(case, "forecast_valid_time"), "forecast_valid_time", &mut errors);
            let verified = parse_time(string_at(case, "verification_valid_time"), "verification_valid_time", &mut errors);
            let observed_at = parse_time(string_at(case, "observation_time"), "observation_time", &mut errors);
            if let (Some(issue_time), Some(available_time), Some(cutoff_time)) =
                (issue.as_ref(), available.as_ref(), cutoff.as_ref())
            {
                if available_time < issue_time {
                    errors.push(format!("forecast case '{case_id}' first availability precedes model issue time"));
                }
                if available_time > cutoff_time {
                    blockers.push(format!("forecast case '{case_id}' uses information unavailable at decision cutoff"));
                }
            }
            if let (Some(issue_time), Some(valid_time)) = (issue.as_ref(), valid.as_ref()) {
                match count_at(case, "lead_hours") {
                    Some(lead) if issue_time.clone() + Duration::hours(lead as i64) == *valid_time => {}
                    _ => errors.push(format!("forecast case '{case_id}' issue time + lead_hours must equal forecast_valid_time")),
                }
            }
            if let (Some(valid_time), Some(cutoff_time)) = (valid.as_ref(), cutoff.as_ref()) {
                if valid_time <= cutoff_time {
                    errors.push(format!("forecast case '{case_id}' valid time must follow the decision cutoff"));
                }
            }
            if let (Some(valid_time), Some(verified_time), Some(observed_time)) =
                (valid.as_ref(), verified.as_ref(), observed_at.as_ref())
            {
                if valid_time != verified_time || valid_time != observed_time {
                    errors.push(format!("forecast case '{case_id}' verification timestamps do not match forecast valid time"));
                }
            }
            if string_at(case, "forecast_source_id") == string_at(case, "verifying_source_id") {
                errors.push(format!("forecast case '{case_id}' cannot verify a source against itself"));
            }
            if !["in_situ_validation", "realized_reanalysis"].contains(&string_at(case, "verifying_source_class").unwrap_or("")) {
                errors.push(format!("forecast case '{case_id}' requires an evaluator-only verifying-source class"));
            }

            let variable = string_at(case, "variable").unwrap_or("unknown");
            let expected_unit = match variable {
                "wind_speed" => "m/s",
                "significant_wave_height" => "m",
                "mean_sea_level_pressure" => "Pa",
                _ => "",
            };
            if expected_unit.is_empty() || string_at(case, "unit") != Some(expected_unit) {
                errors.push(format!("forecast case '{case_id}' has a variable/unit mismatch"));
            }
            let lead = count_at(case, "lead_hours").unwrap_or(0);
            let deterministic = number_at(case, "deterministic_value");
            let observation = number_at(case, "observed_value");
            let members = case.get("ensemble_members").and_then(Value::as_array)
                .map(|items| items.iter().filter_map(Value::as_f64).filter(|number| number.is_finite()).collect::<Vec<_>>())
                .unwrap_or_default();
            match (deterministic, observation) {
                (Some(forecast), Some(observed)) if members.len() >= 2 => {
                    let crps = empirical_crps(&members, observed);
                    skill.entry(format!("{variable}@+{lead}h")).or_default().push((forecast, observed, crps));
                }
                _ => errors.push(format!("forecast case '{case_id}' requires finite forecast/observation values and at least two ensemble members")),
            }

            let mut seen_events = HashSet::new();
            if let Some(thresholds) = case.get("threshold_events").and_then(Value::as_array) {
                if thresholds.is_empty() {
                    errors.push(format!("forecast case '{case_id}' requires at least one threshold event"));
                }
                for threshold in thresholds {
                    let event_id = string_at(threshold, "event_id").unwrap_or("");
                    let probability = number_at(threshold, "probability");
                    let boundary = number_at(threshold, "threshold");
                    let comparison = string_at(threshold, "comparison").unwrap_or("");
                    let observed_event = threshold.get("observed").and_then(Value::as_bool);
                    if event_id.is_empty() || !seen_events.insert(event_id.to_owned())
                        || probability.is_none_or(|p| !(0.0..=1.0).contains(&p))
                        || boundary.is_none()
                        || !["gt", "gte"].contains(&comparison)
                        || observed_event.is_none()
                    {
                        errors.push(format!("forecast case '{case_id}' has an invalid or duplicated threshold event"));
                        continue;
                    }
                    if let Some(observed) = observation {
                        let expected = if comparison == "gt" {
                            observed > boundary.unwrap()
                        } else {
                            observed >= boundary.unwrap()
                        };
                        if observed_event != Some(expected) {
                            errors.push(format!("forecast case '{case_id}' threshold event '{event_id}' disagrees with observed value"));
                        }
                    }
                    events.entry(event_id.to_owned()).or_default().push((probability.unwrap(), observed_event.unwrap()));
                }
            } else {
                errors.push(format!("forecast case '{case_id}' requires threshold_events"));
            }
        }
    }
    if test_strata.len() < 3 {
        errors.push("held-out test fold must include at least three declared environment strata".to_owned());
    }

    let mut seen_pairs = HashSet::<String>::new();
    let mut paired_cases = HashSet::<String>::new();
    let mut energy_improvements = Vec::<f64>::new();
    let mut regret_improvements = Vec::<f64>::new();
    let mut candidate_regrets = Vec::<f64>::new();
    let mut baseline_regrets = Vec::<f64>::new();
    let mut candidate_energy = Vec::<f64>::new();
    let mut baseline_energy = Vec::<f64>::new();
    let mut candidate_gate_violations = 0_u64;
    let mut candidate_service_breaches = 0_u64;
    let mut baseline_service_breaches = 0_u64;
    let mut candidate_comfort_breaches = 0_u64;
    let mut baseline_comfort_breaches = 0_u64;
    let mut candidate_plan_changes = 0_u64;
    let mut baseline_plan_changes = 0_u64;

    let pairs = value.get("mission_pairs").and_then(Value::as_array);
    if pairs.is_none_or(Vec::is_empty) {
        errors.push("mission_pairs must be a non-empty array".to_owned());
    }
    if let Some(pairs) = pairs {
        for (index, pair) in pairs.iter().enumerate() {
            let pair_id = string_at(pair, "pair_id").unwrap_or("").to_owned();
            let case_id = string_at(pair, "case_id").unwrap_or("").to_owned();
            if pair_id.is_empty() || !seen_pairs.insert(pair_id.clone()) {
                errors.push(format!("mission_pairs[{index}] has a missing or duplicated pair_id"));
            }
            let Some(case) = case_by_id.get(&case_id) else {
                errors.push(format!("mission pair '{pair_id}' references unknown forecast case '{case_id}'"));
                continue;
            };
            if !paired_cases.insert(case_id.clone()) {
                errors.push(format!("forecast case '{case_id}' is paired more than once"));
            }
            for field in ["fold", "episode_id", "voyage_id", "weather_event_id"] {
                if string_at(pair, field) != string_at(case, field) {
                    errors.push(format!("mission pair '{pair_id}' does not match forecast case on {field}"));
                }
            }
            for field in [
                "matching_context_sha256", "craft_configuration_sha256", "loading_sha256",
                "occupancy_sha256", "service_fingerprint_sha256", "forecast_information_set_sha256",
                "compute_budget_sha256", "hard_gate_roster_sha256",
            ] {
                if !valid_digest(string_at(pair, field)) {
                    errors.push(format!("mission pair '{pair_id}' has invalid {field}"));
                }
            }
            let candidate = pair.get("candidate").unwrap_or(&Value::Null);
            let baseline = pair.get("baseline").unwrap_or(&Value::Null);
            let candidate_energy_value = number_at(candidate, "purchased_energy_kwh_per_berth_day");
            let baseline_energy_value = number_at(baseline, "purchased_energy_kwh_per_berth_day");
            let candidate_regret_value = number_at(candidate, "mission_regret_kwh_equivalent");
            let baseline_regret_value = number_at(baseline, "mission_regret_kwh_equivalent");
            match (candidate_energy_value, baseline_energy_value, candidate_regret_value, baseline_regret_value) {
                (Some(candidate_e), Some(baseline_e), Some(candidate_r), Some(baseline_r))
                    if candidate_e > 0.0 && baseline_e > 0.0 && candidate_r >= 0.0 && baseline_r >= 0.0 =>
                {
                    energy_improvements.push((baseline_e - candidate_e) / baseline_e * 100.0);
                    let regret_improvement = if baseline_r > 0.0 {
                        (baseline_r - candidate_r) / baseline_r * 100.0
                    } else if candidate_r == 0.0 {
                        0.0
                    } else {
                        blockers.push(format!("candidate '{pair_id}' has positive regret against a zero-regret baseline"));
                        -100.0
                    };
                    regret_improvements.push(regret_improvement);
                    candidate_energy.push(candidate_e);
                    baseline_energy.push(baseline_e);
                    candidate_regrets.push(candidate_r);
                    baseline_regrets.push(baseline_r);
                }
                _ => errors.push(format!("mission pair '{pair_id}' has invalid energy or mission-regret values")),
            }
            let candidate_gates = count_at(candidate, "hard_gate_violations");
            let candidate_services = count_at(candidate, "service_floor_breaches");
            let baseline_services = count_at(baseline, "service_floor_breaches");
            let candidate_comfort = count_at(candidate, "comfort_breaches");
            let baseline_comfort = count_at(baseline, "comfort_breaches");
            let candidate_changes = count_at(candidate, "plan_changes");
            let baseline_changes = count_at(baseline, "plan_changes");
            if let (Some(gates), Some(services), Some(base_services), Some(comfort), Some(base_comfort), Some(changes), Some(base_changes)) =
                (candidate_gates, candidate_services, baseline_services, candidate_comfort, baseline_comfort, candidate_changes, baseline_changes)
            {
                candidate_gate_violations += gates;
                candidate_service_breaches += services;
                baseline_service_breaches += base_services;
                candidate_comfort_breaches += comfort;
                baseline_comfort_breaches += base_comfort;
                candidate_plan_changes += changes;
                baseline_plan_changes += base_changes;
                if gates > 0 {
                    blockers.push(format!("candidate policy '{pair_id}' violates a hard gate"));
                }
                if services > base_services {
                    blockers.push(format!("candidate policy '{pair_id}' regresses a service floor against its matched baseline"));
                }
                if comfort > base_comfort {
                    blockers.push(format!("candidate policy '{pair_id}' regresses comfort against its matched baseline"));
                }
            } else {
                errors.push(format!("mission pair '{pair_id}' requires integer non-negative breach and plan-change counts"));
            }
            if string_at(candidate, "policy_id").is_none_or(str::is_empty)
                || string_at(baseline, "policy_id").is_none_or(str::is_empty)
                || string_at(candidate, "policy_id") == string_at(baseline, "policy_id")
            {
                errors.push(format!("mission pair '{pair_id}' requires distinct candidate and baseline policy IDs"));
            }
        }
    } else {
        errors.push("mission_pairs must be a non-empty array".to_owned());
    }
    if paired_cases.len() != case_by_id.len() {
        errors.push("every forecast case must have exactly one information-matched mission pair".to_owned());
    }

    let forecast_metrics = skill.into_iter().map(|(stratum, samples)| {
        let errors_for_samples = samples.iter().map(|(forecast, observed, _)| *forecast - *observed).collect::<Vec<_>>();
        let absolute = errors_for_samples.iter().map(|error| error.abs()).collect::<Vec<_>>();
        let squared = errors_for_samples.iter().map(|error| error * error).collect::<Vec<_>>();
        let crps = samples.iter().map(|sample| sample.2).collect::<Vec<_>>();
        json!({
            "variable_and_lead": stratum,
            "sample_count": samples.len(),
            "bias": mean(&errors_for_samples),
            "mae": mean(&absolute),
            "rmse": mean(&squared).sqrt(),
            "ensemble_crps": mean(&crps)
        })
    }).collect::<Vec<_>>();

    let threshold_metrics = events.into_iter().map(|(event_id, samples)| {
        let brier = mean(&samples.iter().map(|(probability, observed)| {
            let outcome = if *observed { 1.0 } else { 0.0 };
            (*probability - outcome).powi(2)
        }).collect::<Vec<_>>());
        let mut bins = (0..bin_count).map(|_| Vec::<(f64, bool)>::new()).collect::<Vec<_>>();
        for sample in &samples {
            let bin = ((sample.0 * bin_count as f64).floor() as usize).min(bin_count - 1);
            bins[bin].push(*sample);
        }
        let reliability = bins.iter().enumerate().filter(|(_, values)| !values.is_empty()).map(|(index, values)| {
            json!({
                "bin": index,
                "count": values.len(),
                "mean_forecast_probability": mean(&values.iter().map(|value| value.0).collect::<Vec<_>>()),
                "observed_frequency": mean(&values.iter().map(|value| if value.1 {1.0} else {0.0}).collect::<Vec<_>>())
            })
        }).collect::<Vec<_>>();
        json!({"event_id":event_id,"sample_count":samples.len(),"brier_score":brier,"reliability_bins":reliability})
    }).collect::<Vec<_>>();

    let (candidate_cvar, candidate_tail_count) = cvar95(&candidate_regrets);
    let (baseline_cvar, baseline_tail_count) = cvar95(&baseline_regrets);
    let decision_metrics = json!({
        "paired_case_count": energy_improvements.len(),
        "mean_paired_energy_improvement_pct": if energy_improvements.is_empty() {Value::Null} else {json!(mean(&energy_improvements))},
        "mean_paired_regret_improvement_pct": if regret_improvements.is_empty() {Value::Null} else {json!(mean(&regret_improvements))},
        "candidate_cvar95_regret_kwh_equivalent": candidate_cvar,
        "candidate_cvar95_tail_sample_count": candidate_tail_count,
        "baseline_cvar95_regret_kwh_equivalent": baseline_cvar,
        "baseline_cvar95_tail_sample_count": baseline_tail_count,
        "candidate_mean_purchased_energy_kwh_per_berth_day": if candidate_energy.is_empty() {Value::Null} else {json!(mean(&candidate_energy))},
        "baseline_mean_purchased_energy_kwh_per_berth_day": if baseline_energy.is_empty() {Value::Null} else {json!(mean(&baseline_energy))},
        "candidate_hard_gate_violations": candidate_gate_violations,
        "candidate_service_floor_breaches": candidate_service_breaches,
        "baseline_service_floor_breaches": baseline_service_breaches,
        "candidate_comfort_breaches": candidate_comfort_breaches,
        "baseline_comfort_breaches": baseline_comfort_breaches,
        "candidate_plan_changes": candidate_plan_changes,
        "baseline_plan_changes": baseline_plan_changes,
        "statistical_inference": "NOT_PERFORMED",
        "paired_uncertainty_intervals": "NOT_COMPUTED",
        "claim_ceiling": "descriptive_paired_outcomes_only"
    });

    let disposition = if !errors.is_empty() {
        "INVALID"
    } else if !blockers.is_empty() {
        "BLOCKED"
    } else if record_status == "synthetic" {
        "SYNTHETIC_ONLY"
    } else {
        "SCORECARD_AUDITED_READ_ONLY"
    };
    let digest = Sha256::digest(raw);
    let digest_hex = digest.iter().map(|byte| format!("{byte:02x}")).collect::<String>();
    json!({
        "engine_version": ENGINE_VERSION,
        "scorecard_id": string_at(value, "scorecard_id"),
        "disposition": disposition,
        "input_scorecard_sha256": digest_hex,
        "forecast_case_count": case_by_id.len(),
        "mission_pair_count": seen_pairs.len(),
        "held_out_test_strata": test_strata,
        "forecast_metrics": forecast_metrics,
        "threshold_metrics": threshold_metrics,
        "decision_metrics": decision_metrics,
        "findings": {
            "errors": errors,
            "blockers": blockers
        },
        "claim_ceiling": "descriptive_metrics_only_no_statistical_superiority_or_operational_authorization",
        "note": "CRPS is an empirical ensemble score; Brier scores and reliability bins are threshold-specific. CVaR95 is descriptive and its tail count is reported. No inferential claim is made; use whole-voyage/weather-event block resampling and prospective power analysis before superiority claims."
    })
}

fn run() -> Result<(), String> {
    let arguments = env::args().collect::<Vec<_>>();
    if arguments.len() != 3 || arguments[1] != "audit-scorecard" {
        return Err("Usage: marine-forecast-scorecard audit-scorecard <scorecard.json>".to_owned());
    }
    let raw = fs::read(&arguments[2]).map_err(|error| format!("{}: {error}", arguments[2]))?;
    let value: Value = serde_json::from_slice(&raw)
        .map_err(|error| format!("{}: invalid JSON: {error}", arguments[2]))?;
    let report = validate(&raw, &value);
    println!("{}", serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?);
    match string_at(&report, "disposition") {
        Some("SYNTHETIC_ONLY" | "SCORECARD_AUDITED_READ_ONLY") => Ok(()),
        Some(disposition) => Err(format!("scorecard disposition: {disposition}")),
        None => Err("scorecard report has no disposition".to_owned()),
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
        serde_json::from_str(include_str!("../fixtures/synthetic-scorecard-0001.json"))
            .expect("synthetic scorecard fixture parses")
    }

    fn audit(value: &Value) -> Value {
        let raw = serde_json::to_vec(value).expect("scorecard serializes");
        validate(&raw, value)
    }

    #[test]
    fn synthetic_scorecard_computes_skill_and_paired_decision_metrics() {
        let report = audit(&fixture());
        assert_eq!(report["disposition"], "SYNTHETIC_ONLY");
        assert_eq!(report["forecast_case_count"], 9);
        assert_eq!(report["mission_pair_count"], 9);
        assert_eq!(report["forecast_metrics"][0]["sample_count"], 9);
        assert!(report["forecast_metrics"][0]["ensemble_crps"].as_f64().unwrap() >= 0.0);
        assert!(report["threshold_metrics"][0]["brier_score"].as_f64().unwrap() >= 0.0);
        assert_eq!(report["decision_metrics"]["statistical_inference"], "NOT_PERFORMED");
        assert!(report["decision_metrics"]["mean_paired_energy_improvement_pct"].as_f64().unwrap() > 0.0);
    }

    #[test]
    fn forecast_first_available_after_cutoff_is_blocked() {
        let mut value = fixture();
        value["forecast_cases"][6]["first_available_at"] = json!("2025-03-01T00:40:00Z");
        assert_eq!(audit(&value)["disposition"], "BLOCKED");
    }

    #[test]
    fn weather_event_cannot_cross_folds() {
        let mut value = fixture();
        value["forecast_cases"][6]["weather_event_id"] =
            value["forecast_cases"][0]["weather_event_id"].clone();
        assert_eq!(audit(&value)["disposition"], "BLOCKED");
    }

    #[test]
    fn candidate_hard_gate_violation_blocks_scorecard() {
        let mut value = fixture();
        value["mission_pairs"][0]["candidate"]["hard_gate_violations"] = json!(1);
        assert_eq!(audit(&value)["disposition"], "BLOCKED");
    }

    #[test]
    fn paired_context_must_match_the_forecast_case_identity() {
        let mut value = fixture();
        value["mission_pairs"][0]["voyage_id"] = json!("different-voyage");
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn malformed_forecast_digest_is_invalid() {
        let mut value = fixture();
        value["forecast_cases"][0]["forecast_sha256"] = json!("not-a-digest");
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn verification_must_match_forecast_valid_time() {
        let mut value = fixture();
        value["forecast_cases"][0]["observation_time"] = json!("2025-01-01T07:00:00Z");
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn threshold_outcome_must_agree_with_observation() {
        let mut value = fixture();
        value["forecast_cases"][0]["threshold_events"][0]["observed"] = json!(true);
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }
}
