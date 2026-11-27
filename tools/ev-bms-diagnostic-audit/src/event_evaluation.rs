use jsonschema::draft202012;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const MANIFEST_VERSION: &str = "ev-bms-event-evaluation/v0.1";
const SCHEMA_ID: &str =
    "https://luminousdynamics.io/schemas/transport/ev-bms-event-evaluation/0.1";
const HOUR_MS: f64 = 3_600_000.0;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvaluationManifest {
    manifest_version: String,
    evaluation_id: String,
    dataset_id: String,
    dataset_domain: String,
    dataset_admission_state: String,
    dataset_manifest_sha256: String,
    dataset_snapshot_sha256: Option<String>,
    split_manifest_sha256: String,
    split_partition: String,
    evidence_class: String,
    event_lookback_ms: u64,
    threshold_policy_id: String,
    exposures: Vec<Exposure>,
    events: Vec<FaultEvent>,
    alarms: Vec<AlarmEpisode>,
    claim_ceiling: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Exposure {
    exposure_id: String,
    asset_id: String,
    role: String,
    window_start_unix_ms: u64,
    window_end_unix_ms: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FaultEvent {
    event_id: String,
    asset_id: String,
    event_time_unix_ms: u64,
    event_time_semantics: String,
    label_source: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AlarmEpisode {
    alarm_id: String,
    asset_id: String,
    alarm_time_unix_ms: u64,
    score: f64,
    threshold: f64,
}

#[derive(Debug, Serialize)]
pub struct EventEvaluationReport {
    pub report_version: String,
    pub evaluation_id: String,
    pub dataset_id: String,
    pub dataset_domain: String,
    pub dataset_admission_state: String,
    pub dataset_manifest_sha256: String,
    pub dataset_snapshot_sha256: Option<String>,
    pub split_manifest_sha256: String,
    pub split_partition: String,
    pub evidence_class: String,
    pub event_lookback_ms: u64,
    pub threshold_policy_id: String,
    pub independent_asset_count: usize,
    pub positive_event_count: usize,
    pub positive_asset_count: usize,
    pub negative_control_asset_count: usize,
    pub event_warning_count: usize,
    pub event_miss_count: usize,
    pub event_warning_coverage_fraction: f64,
    pub false_alarm_episode_count: usize,
    pub negative_control_exposure_hours: f64,
    pub false_alarms_per_1000_negative_hours: Option<f64>,
    pub primary_event_alarm_count: usize,
    pub repeated_event_alarm_count: usize,
    pub unmatched_event_exposure_alarm_count: usize,
    pub total_alarm_episode_count: usize,
    pub lead_time_hours_min: Option<f64>,
    pub lead_time_hours_p05: Option<f64>,
    pub lead_time_hours_p50: Option<f64>,
    pub lead_time_hours_p95: Option<f64>,
    pub lead_time_hours_max: Option<f64>,
    pub schema_sha256: String,
    pub canonical_manifest_sha256: String,
    pub claim_ceiling: String,
    pub report_sha256: String,
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn percentile(values: &[f64], fraction: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let index = ((fraction * sorted.len() as f64).ceil() as usize)
        .saturating_sub(1)
        .min(sorted.len() - 1);
    Some(sorted[index])
}

fn canonicalize(value: &mut Value) {
    if let Some(exposures) = value.get_mut("exposures").and_then(Value::as_array_mut) {
        exposures.sort_by(|left, right| {
            (
                left.get("asset_id").and_then(Value::as_str),
                left.get("window_start_unix_ms").and_then(Value::as_u64),
                left.get("exposure_id").and_then(Value::as_str),
            )
                .cmp(&(
                    right.get("asset_id").and_then(Value::as_str),
                    right.get("window_start_unix_ms").and_then(Value::as_u64),
                    right.get("exposure_id").and_then(Value::as_str),
                ))
        });
    }
    if let Some(events) = value.get_mut("events").and_then(Value::as_array_mut) {
        events.sort_by(|left, right| {
            (
                left.get("asset_id").and_then(Value::as_str),
                left.get("event_time_unix_ms").and_then(Value::as_u64),
                left.get("event_id").and_then(Value::as_str),
            )
                .cmp(&(
                    right.get("asset_id").and_then(Value::as_str),
                    right.get("event_time_unix_ms").and_then(Value::as_u64),
                    right.get("event_id").and_then(Value::as_str),
                ))
        });
    }
    if let Some(alarms) = value.get_mut("alarms").and_then(Value::as_array_mut) {
        alarms.sort_by(|left, right| {
            (
                left.get("asset_id").and_then(Value::as_str),
                left.get("alarm_time_unix_ms").and_then(Value::as_u64),
                left.get("alarm_id").and_then(Value::as_str),
            )
                .cmp(&(
                    right.get("asset_id").and_then(Value::as_str),
                    right.get("alarm_time_unix_ms").and_then(Value::as_u64),
                    right.get("alarm_id").and_then(Value::as_str),
                ))
        });
    }
}

fn validate_and_evaluate(manifest: &EvaluationManifest) -> Result<EventEvaluationReport, String> {
    if manifest.manifest_version != MANIFEST_VERSION
        || !nonempty(&manifest.evaluation_id)
        || !nonempty(&manifest.dataset_id)
        || !nonempty(&manifest.threshold_policy_id)
        || !nonempty(&manifest.claim_ceiling)
    {
        return Err("evaluation manifest version or identity is invalid".to_owned());
    }
    if manifest.split_partition != "test" {
        return Err("event-level promotion metrics must be evaluated on the frozen test partition".to_owned());
    }
    if !["onboard_bms", "charger_side"].contains(&manifest.dataset_domain.as_str()) {
        return Err("unsupported event evaluation dataset domain".to_owned());
    }
    if !is_sha256(&manifest.dataset_manifest_sha256)
        || !is_sha256(&manifest.split_manifest_sha256)
    {
        return Err("dataset and split manifest digests must be lowercase SHA-256".to_owned());
    }
    if manifest.event_lookback_ms == 0 {
        return Err("event lookback horizon must be positive".to_owned());
    }
    match manifest.dataset_admission_state.as_str() {
        "metadata_only" => {
            if manifest.evidence_class != "synthetic_fixture"
                || manifest.dataset_snapshot_sha256.is_some()
            {
                return Err("metadata-only data may only use synthetic event fixtures".to_owned());
            }
        }
        "snapshot_admitted" => {
            if manifest.evidence_class != "operator_collected_field_observation"
                || manifest
                    .dataset_snapshot_sha256
                    .as_deref()
                    .is_none_or(|digest| !is_sha256(digest))
            {
                return Err("field event evaluation requires an admitted snapshot digest".to_owned());
            }
        }
        _ => return Err("unknown dataset admission state".to_owned()),
    }
    if manifest.exposures.is_empty() {
        return Err("monitoring exposure inventory must not be empty".to_owned());
    }
    if manifest.events.is_empty() {
        return Err("at least one independently labelled fault event is required".to_owned());
    }

    let mut exposure_ids = BTreeSet::new();
    let mut event_ids = BTreeSet::new();
    let mut alarm_ids = BTreeSet::new();
    let mut asset_roles: BTreeMap<String, String> = BTreeMap::new();
    let mut exposure_by_asset: BTreeMap<String, Vec<&Exposure>> = BTreeMap::new();
    let mut total_assets = BTreeSet::new();
    let mut negative_assets = BTreeSet::new();
    let mut positive_assets = BTreeSet::new();
    let mut negative_control_exposure_ms = 0_u128;

    for exposure in &manifest.exposures {
        if !nonempty(&exposure.exposure_id)
            || !exposure_ids.insert(exposure.exposure_id.clone())
            || !nonempty(&exposure.asset_id)
            || exposure.window_start_unix_ms >= exposure.window_end_unix_ms
            || !["negative_control", "event_monitoring"].contains(&exposure.role.as_str())
        {
            return Err("exposure identity, role or half-open time window is invalid".to_owned());
        }
        if let Some(role) = asset_roles.get(&exposure.asset_id) {
            if role != &exposure.role {
                return Err(format!(
                    "asset {} mixes negative-control and event-monitoring exposure",
                    exposure.asset_id
                ));
            }
        } else {
            asset_roles.insert(exposure.asset_id.clone(), exposure.role.clone());
        }
        if exposure.role == "negative_control" {
            negative_assets.insert(exposure.asset_id.clone());
            negative_control_exposure_ms +=
                (exposure.window_end_unix_ms - exposure.window_start_unix_ms) as u128;
        }
        total_assets.insert(exposure.asset_id.clone());
        exposure_by_asset
            .entry(exposure.asset_id.clone())
            .or_default()
            .push(exposure);
    }

    for (asset_id, exposures) in &mut exposure_by_asset {
        exposures.sort_by_key(|exposure| exposure.window_start_unix_ms);
        for pair in exposures.windows(2) {
            if pair[1].window_start_unix_ms < pair[0].window_end_unix_ms {
                return Err(format!("monitoring exposure intervals overlap for asset {asset_id}"));
            }
        }
    }

    let mut event_windows_by_asset: BTreeMap<String, Vec<(u64, u64, String)>> = BTreeMap::new();
    for event in &manifest.events {
        if !nonempty(&event.event_id)
            || !event_ids.insert(event.event_id.clone())
            || !nonempty(&event.asset_id)
            || !nonempty(&event.label_source)
            || ![
                "recorded_bms_alarm",
                "source_fault_time_file",
                "maintenance_confirmation",
                "adjudicated_fault_onset",
                "other_recorded_event_time",
            ]
            .contains(&event.event_time_semantics.as_str())
        {
            return Err("fault event identity, label source or time semantics are invalid".to_owned());
        }
        if asset_roles.get(&event.asset_id).map(String::as_str) != Some("event_monitoring") {
            return Err(format!("event {} is not bound to event-monitoring exposure", event.event_id));
        }
        let window_start = event
            .event_time_unix_ms
            .checked_sub(manifest.event_lookback_ms)
            .ok_or_else(|| format!("event {} has insufficient timestamp range for lookback", event.event_id))?;
        let covers_horizon = exposure_by_asset
            .get(&event.asset_id)
            .is_some_and(|intervals| {
                intervals.iter().any(|exposure| {
                    exposure.role == "event_monitoring"
                        && exposure.window_start_unix_ms <= window_start
                        && exposure.window_end_unix_ms >= event.event_time_unix_ms
                })
            });
        if !covers_horizon {
            return Err(format!(
                "event {} lacks continuous monitoring over its full pre-event horizon",
                event.event_id
            ));
        }
        event_windows_by_asset
            .entry(event.asset_id.clone())
            .or_default()
            .push((window_start, event.event_time_unix_ms, event.event_id.clone()));
        positive_assets.insert(event.asset_id.clone());
    }

    for (asset_id, windows) in &mut event_windows_by_asset {
        windows.sort_by_key(|window| window.0);
        for pair in windows.windows(2) {
            if pair[1].0 < pair[0].1 {
                return Err(format!(
                    "pre-event warning horizons overlap for asset {asset_id}; event matching would be ambiguous"
                ));
            }
        }
    }

    let mut alarm_refs = Vec::new();
    for alarm in &manifest.alarms {
        if !nonempty(&alarm.alarm_id)
            || !alarm_ids.insert(alarm.alarm_id.clone())
            || !nonempty(&alarm.asset_id)
            || !alarm.score.is_finite()
            || !alarm.threshold.is_finite()
            || alarm.score <= alarm.threshold
        {
            return Err("alarm episode identity or emitted-score/threshold contract is invalid".to_owned());
        }
        let covered = exposure_by_asset
            .get(&alarm.asset_id)
            .is_some_and(|intervals| {
                intervals.iter().any(|exposure| {
                    exposure.window_start_unix_ms <= alarm.alarm_time_unix_ms
                        && alarm.alarm_time_unix_ms < exposure.window_end_unix_ms
                })
            });
        if !covered {
            return Err(format!("alarm {} falls outside declared monitoring exposure", alarm.alarm_id));
        }
        alarm_refs.push(alarm);
    }
    alarm_refs.sort_by_key(|alarm| (alarm.asset_id.as_str(), alarm.alarm_time_unix_ms));

    let mut event_warning_count = 0usize;
    let mut event_miss_count = 0usize;
    let mut primary_event_alarm_count = 0usize;
    let mut repeated_event_alarm_count = 0usize;
    let mut unmatched_event_exposure_alarm_count = 0usize;
    let mut lead_times_hours = Vec::new();
    let mut warning_window_by_alarm = BTreeSet::new();

    for event in &manifest.events {
        let window_start = event.event_time_unix_ms - manifest.event_lookback_ms;
        let matching = alarm_refs
            .iter()
            .filter(|alarm| {
                alarm.asset_id == event.asset_id
                    && alarm.alarm_time_unix_ms >= window_start
                    && alarm.alarm_time_unix_ms < event.event_time_unix_ms
            })
            .collect::<Vec<_>>();
        if let Some(first) = matching.first() {
            event_warning_count += 1;
            primary_event_alarm_count += 1;
            lead_times_hours.push(
                (event.event_time_unix_ms - first.alarm_time_unix_ms) as f64 / HOUR_MS,
            );
            for alarm in &matching {
                warning_window_by_alarm.insert(alarm.alarm_id.clone());
            }
            repeated_event_alarm_count += matching.len().saturating_sub(1);
        } else {
            event_miss_count += 1;
        }
    }

    let mut false_alarm_episode_count = 0usize;
    for alarm in &alarm_refs {
        if warning_window_by_alarm.contains(&alarm.alarm_id) {
            continue;
        }
        match asset_roles.get(&alarm.asset_id).map(String::as_str) {
            Some("negative_control") => false_alarm_episode_count += 1,
            Some("event_monitoring") => unmatched_event_exposure_alarm_count += 1,
            _ => return Err(format!("alarm {} has no declared asset exposure role", alarm.alarm_id)),
        }
    }

    let negative_control_exposure_hours = negative_control_exposure_ms as f64 / HOUR_MS;
    let false_alarms_per_1000_negative_hours = if negative_control_exposure_hours > 0.0 {
        Some(false_alarm_episode_count as f64 / negative_control_exposure_hours * 1000.0)
    } else {
        None
    };
    let event_warning_coverage_fraction =
        event_warning_count as f64 / manifest.events.len() as f64;
    let total_alarms = alarm_refs.len();

    let mut report = EventEvaluationReport {
        report_version: "ev-bms-event-evaluation-report/v0.1".to_owned(),
        evaluation_id: manifest.evaluation_id.clone(),
        dataset_id: manifest.dataset_id.clone(),
        dataset_domain: manifest.dataset_domain.clone(),
        dataset_admission_state: manifest.dataset_admission_state.clone(),
        dataset_manifest_sha256: manifest.dataset_manifest_sha256.clone(),
        dataset_snapshot_sha256: manifest.dataset_snapshot_sha256.clone(),
        split_manifest_sha256: manifest.split_manifest_sha256.clone(),
        split_partition: manifest.split_partition.clone(),
        evidence_class: manifest.evidence_class.clone(),
        event_lookback_ms: manifest.event_lookback_ms,
        threshold_policy_id: manifest.threshold_policy_id.clone(),
        independent_asset_count: total_assets.len(),
        positive_event_count: manifest.events.len(),
        positive_asset_count: positive_assets.len(),
        negative_control_asset_count: negative_assets.len(),
        event_warning_count,
        event_miss_count,
        event_warning_coverage_fraction,
        false_alarm_episode_count,
        negative_control_exposure_hours,
        false_alarms_per_1000_negative_hours,
        primary_event_alarm_count,
        repeated_event_alarm_count,
        unmatched_event_exposure_alarm_count,
        total_alarm_episode_count: total_alarms,
        lead_time_hours_min: lead_times_hours.iter().copied().reduce(f64::min),
        lead_time_hours_p05: percentile(&lead_times_hours, 0.05),
        lead_time_hours_p50: percentile(&lead_times_hours, 0.50),
        lead_time_hours_p95: percentile(&lead_times_hours, 0.95),
        lead_time_hours_max: lead_times_hours.iter().copied().reduce(f64::max),
        schema_sha256: String::new(),
        canonical_manifest_sha256: String::new(),
        claim_ceiling: manifest.claim_ceiling.clone(),
        report_sha256: String::new(),
    };
    Ok(report)
}

pub fn audit_event_evaluation(
    schema_bytes: &[u8],
    manifest_bytes: &[u8],
) -> Result<EventEvaluationReport, String> {
    let schema: Value =
        serde_json::from_slice(schema_bytes).map_err(|error| format!("schema JSON: {error}"))?;
    if schema.get("$schema").and_then(Value::as_str)
        != Some("https://json-schema.org/draft/2020-12/schema")
    {
        return Err("event evaluation schema must declare JSON Schema Draft 2020-12".to_owned());
    }
    if schema.get("$id").and_then(Value::as_str) != Some(SCHEMA_ID) {
        return Err("event evaluation schema identity does not match v0.1".to_owned());
    }
    if !draft202012::meta::is_valid(&schema) {
        return Err("event evaluation schema fails Draft 2020-12 meta-schema validation".to_owned());
    }
    let validator = draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
        .map_err(|error| format!("event evaluation schema compilation: {error}"))?;

    let mut value: Value =
        serde_json::from_slice(manifest_bytes).map_err(|error| format!("evaluation JSON: {error}"))?;
    canonicalize(&mut value);
    if let Err(error) = validator.validate(&value) {
        return Err(format!("event evaluation schema validation: {error}"));
    }
    let canonical_bytes =
        serde_json::to_vec(&value).map_err(|error| format!("canonical evaluation: {error}"))?;
    let manifest: EvaluationManifest = serde_json::from_slice(&canonical_bytes)
        .map_err(|error| format!("typed evaluation manifest: {error}"))?;
    let mut report = validate_and_evaluate(&manifest)?;
    report.schema_sha256 = sha256(schema_bytes);
    report.canonical_manifest_sha256 = sha256(&canonical_bytes);
    let report_bytes =
        serde_json::to_vec(&report).map_err(|error| format!("event report serialization: {error}"))?;
    report.report_sha256 = sha256(&report_bytes);
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const SCHEMA: &[u8] =
        include_bytes!("../../../schemas/transport/ev_bms_event_evaluation_v0_1.schema.json");
    const FIXTURE: &[u8] =
        include_bytes!("../../../fixtures/ev_battery/event_evaluation_synthetic_v0_1.json");

    #[test]
    fn known_answer_reports_event_coverage_lead_time_and_exposure_adjusted_false_alarms() {
        let report = audit_event_evaluation(SCHEMA, FIXTURE).expect("synthetic event evaluation");
        assert_eq!(report.independent_asset_count, 2);
        assert_eq!(report.positive_event_count, 2);
        assert_eq!(report.positive_asset_count, 1);
        assert_eq!(report.negative_control_asset_count, 1);
        assert_eq!(report.event_warning_count, 1);
        assert_eq!(report.event_miss_count, 1);
        assert!((report.event_warning_coverage_fraction - 0.5).abs() < 1e-12);
        assert_eq!(report.false_alarm_episode_count, 2);
        assert!((report.negative_control_exposure_hours - 10.0).abs() < 1e-12);
        assert!(
            (report.false_alarms_per_1000_negative_hours.unwrap() - 200.0).abs() < 1e-12
        );
        assert_eq!(report.primary_event_alarm_count, 1);
        assert_eq!(report.repeated_event_alarm_count, 1);
        assert_eq!(report.unmatched_event_exposure_alarm_count, 1);
        assert_eq!(report.total_alarm_episode_count, 5);
        assert!((report.lead_time_hours_min.unwrap() - 3.0).abs() < 1e-12);
        assert!((report.lead_time_hours_p50.unwrap() - 3.0).abs() < 1e-12);
    }

    #[test]
    fn rejects_event_without_full_pre_event_monitoring_horizon() {
        let mut value: Value = serde_json::from_slice(FIXTURE).expect("fixture JSON");
        value["exposures"][1]["window_start_unix_ms"] = Value::from(7_u64 * 3_600_000);
        let bytes = serde_json::to_vec(&value).expect("mutated event manifest");
        assert!(audit_event_evaluation(SCHEMA, &bytes).is_err());
    }

    #[test]
    fn rejects_alarm_outside_monitoring_exposure() {
        let mut value: Value = serde_json::from_slice(FIXTURE).expect("fixture JSON");
        value["alarms"][0]["alarm_time_unix_ms"] = Value::from(99_u64 * 3_600_000);
        let bytes = serde_json::to_vec(&value).expect("mutated event manifest");
        assert!(audit_event_evaluation(SCHEMA, &bytes).is_err());
    }

    #[test]
    fn rejects_overlapping_event_warning_horizons_for_same_asset() {
        let mut value: Value = serde_json::from_slice(FIXTURE).expect("fixture JSON");
        value["events"][1]["event_time_unix_ms"] = Value::from(12_u64 * 3_600_000);
        let bytes = serde_json::to_vec(&value).expect("mutated event manifest");
        assert!(audit_event_evaluation(SCHEMA, &bytes).is_err());
    }

    #[test]
    fn rejects_emitted_alarm_not_above_the_frozen_threshold() {
        let mut value: Value = serde_json::from_slice(FIXTURE).expect("fixture JSON");
        value["alarms"][0]["score"] = Value::from(0.5);
        value["alarms"][0]["threshold"] = Value::from(0.5);
        let bytes = serde_json::to_vec(&value).expect("mutated event manifest");
        assert!(audit_event_evaluation(SCHEMA, &bytes).is_err());
    }

    #[test]
    fn canonical_digest_is_invariant_to_exposure_event_and_alarm_order() {
        let first = audit_event_evaluation(SCHEMA, FIXTURE).expect("first evaluation");
        let mut value: Value = serde_json::from_slice(FIXTURE).expect("fixture JSON");
        value["exposures"].as_array_mut().unwrap().reverse();
        value["events"].as_array_mut().unwrap().reverse();
        value["alarms"].as_array_mut().unwrap().reverse();
        let bytes = serde_json::to_vec(&value).expect("permuted evaluation");
        let second = audit_event_evaluation(SCHEMA, &bytes).expect("permuted evaluation");
        assert_eq!(first.canonical_manifest_sha256, second.canonical_manifest_sha256);
        assert_eq!(first.report_sha256, second.report_sha256);
    }
}
