//! Interval-censored EV battery event warning evaluation.
//!
//! This module is deliberately separate from the point-time v0.1 evaluator.
//! It reports definite and possible warnings without collapsing onset
//! uncertainty into an invented point timestamp.

use jsonschema::draft202012;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const MANIFEST_VERSION: &str = "ev-bms-event-censoring-evaluation/v0.1";
const SCHEMA_ID: &str =
    "https://luminousdynamics.io/schemas/transport/ev-bms-event-censoring-evaluation/0.1";
const HOUR_MS: f64 = 3_600_000.0;

type AssetExposures<'a> = BTreeMap<String, Vec<&'a Exposure>>;
type EventWindowIndex = BTreeMap<String, Vec<(u64, u64, String)>>;

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
    time_basis_id: String,
    time_alignment_id: String,
    time_alignment_artifact_sha256: String,
    time_alignment_method: String,
    max_relative_clock_error_ms: u64,
    alignment_uncertainty_preapplied: bool,
    warning_horizon_ms: u64,
    minimum_action_lead_ms: u64,
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
    onset_type: String,
    onset_lower_unix_ms: Option<u64>,
    onset_lower_semantics: String,
    onset_upper_unix_ms: u64,
    onset_upper_semantics: String,
    event_time_semantics: String,
    onset_evidence_refs: Vec<String>,
    adjudication_revision: String,
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
pub struct EventCensoringReport {
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
    pub time_basis_id: String,
    pub time_alignment_id: String,
    pub time_alignment_artifact_sha256: String,
    pub time_alignment_method: String,
    pub max_relative_clock_error_ms: u64,
    pub alignment_uncertainty_preapplied: bool,
    pub warning_horizon_ms: u64,
    pub minimum_action_lead_ms: u64,
    pub threshold_policy_id: String,
    pub independent_asset_count: usize,
    pub event_count: usize,
    pub interval_censored_event_count: usize,
    pub exact_reported_event_count: usize,
    pub left_censored_event_count: usize,
    pub eligible_event_count: usize,
    pub unobservable_event_count: usize,
    pub unobservable_event_window_alarm_count: usize,
    pub definite_warning_count: usize,
    pub possible_only_warning_count: usize,
    pub possible_pre_onset_only_warning_count: usize,
    pub ambiguous_ordering_warning_count: usize,
    pub possible_warning_count: usize,
    pub event_miss_count: usize,
    pub definite_warning_coverage_fraction: Option<f64>,
    pub possible_warning_coverage_fraction: Option<f64>,
    pub guaranteed_actionable_warning_count: usize,
    pub primary_event_alarm_count: usize,
    pub repeated_event_alarm_count: usize,
    pub unmatched_event_monitoring_alarm_count: usize,
    pub left_censored_ambiguous_alarm_count: usize,
    pub negative_control_asset_count: usize,
    pub negative_control_exposure_hours: f64,
    pub negative_control_false_alarm_count: usize,
    pub negative_control_false_alarms_per_1000_hours: Option<f64>,
    pub right_censored_asset_count: usize,
    pub right_censored_at_risk_hours: f64,
    pub right_censored_horizon_edge_hours: f64,
    pub right_censored_eligible_forecast_hours: f64,
    pub right_censored_false_alarm_count: usize,
    pub right_censored_unresolved_alarm_count: usize,
    pub right_censored_false_alarms_per_1000_eligible_hours: Option<f64>,
    pub lead_interval_count: usize,
    pub lead_lower_hours_min: Option<f64>,
    pub lead_lower_hours_p05: Option<f64>,
    pub lead_lower_hours_p50: Option<f64>,
    pub lead_lower_hours_p95: Option<f64>,
    pub lead_lower_hours_max: Option<f64>,
    pub lead_upper_hours_min: Option<f64>,
    pub lead_upper_hours_p05: Option<f64>,
    pub lead_upper_hours_p50: Option<f64>,
    pub lead_upper_hours_p95: Option<f64>,
    pub lead_upper_hours_max: Option<f64>,
    pub total_alarm_episode_count: usize,
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
                left.get("onset_upper_unix_ms").and_then(Value::as_u64),
                left.get("event_id").and_then(Value::as_str),
            )
                .cmp(&(
                    right.get("asset_id").and_then(Value::as_str),
                    right.get("onset_upper_unix_ms").and_then(Value::as_u64),
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

fn covers_range(exposures: &[&Exposure], start: u64, end: u64) -> bool {
    if start >= end {
        return false;
    }
    let mut cursor = start;
    for exposure in exposures {
        if exposure.window_end_unix_ms <= cursor {
            continue;
        }
        if exposure.window_start_unix_ms > cursor {
            return false;
        }
        cursor = cursor.max(exposure.window_end_unix_ms);
        if cursor >= end {
            return true;
        }
    }
    false
}

fn contains_point(exposures: &[&Exposure], time: u64) -> bool {
    exposures.iter().any(|exposure| {
        exposure.window_start_unix_ms <= time && time < exposure.window_end_unix_ms
    })
}

fn contiguous_blocks(exposures: &[&Exposure]) -> Vec<(u64, u64)> {
    let mut blocks: Vec<(u64, u64)> = Vec::new();
    for exposure in exposures {
        if let Some(last) = blocks.last_mut() {
            if exposure.window_start_unix_ms == last.1 {
                last.1 = exposure.window_end_unix_ms;
                continue;
            }
        }
        blocks.push((exposure.window_start_unix_ms, exposure.window_end_unix_ms));
    }
    blocks
}

fn validate_and_evaluate(
    manifest: &EvaluationManifest,
) -> Result<EventCensoringReport, String> {
    if manifest.manifest_version != MANIFEST_VERSION
        || !nonempty(&manifest.evaluation_id)
        || !nonempty(&manifest.dataset_id)
        || !nonempty(&manifest.threshold_policy_id)
        || !nonempty(&manifest.claim_ceiling)
    {
        return Err("evaluation manifest version or identity is invalid".to_owned());
    }
    if manifest.split_partition != "test" {
        return Err("event-censoring promotion metrics require the frozen test partition".to_owned());
    }
    let expected_dataset = match manifest.dataset_domain.as_str() {
        "onboard_bms" => "cao-onboard-bms-zenodo-10656500",
        "charger_side" => "shenzhen-charger-side-mendeley-c7gg94tmvz-v3",
        _ => return Err("unsupported event-censoring dataset domain".to_owned()),
    };
    if manifest.dataset_id != expected_dataset {
        return Err("dataset ID does not match the declared measurement domain".to_owned());
    }
    if !is_sha256(&manifest.dataset_manifest_sha256) || !is_sha256(&manifest.split_manifest_sha256) {
        return Err("dataset and split manifest digests must be lowercase SHA-256".to_owned());
    }
    if manifest.warning_horizon_ms == 0 {
        return Err("warning horizon must be positive".to_owned());
    }
    if !nonempty(&manifest.time_basis_id)
        || !nonempty(&manifest.time_alignment_id)
        || !is_sha256(&manifest.time_alignment_artifact_sha256)
        || !["synthetic_shared_clock", "shared_hardware_clock", "ptp_ntp_verified", "reference_event_alignment", "operator_adjudicated_alignment"].contains(&manifest.time_alignment_method.as_str())
        || !manifest.alignment_uncertainty_preapplied
    {
        return Err(
            "time basis and alignment evidence are required; relative clock uncertainty must already be applied to event bounds and exposure guard bands".to_owned(),
        );
    }
    match manifest.dataset_admission_state.as_str() {
        "metadata_only" => {
            if manifest.evidence_class != "synthetic_fixture"
                || manifest.dataset_snapshot_sha256.is_some()
                || manifest.time_alignment_method != "synthetic_shared_clock"
            {
                return Err("metadata-only data may only use synthetic event fixtures".to_owned());
            }
        }
        "snapshot_admitted" => {
            if manifest.evidence_class != "operator_collected_field_observation"
                || manifest.time_alignment_method == "synthetic_shared_clock"
                || manifest.dataset_snapshot_sha256.as_deref().is_none_or(|d| !is_sha256(d))
            {
                return Err("field evaluation requires an admitted snapshot digest".to_owned());
            }
        }
        _ => return Err("unknown dataset admission state".to_owned()),
    }
    if manifest.exposures.is_empty() {
        return Err("monitoring exposure inventory must not be empty".to_owned());
    }

    let mut exposure_ids = BTreeSet::new();
    let mut event_ids = BTreeSet::new();
    let mut alarm_ids = BTreeSet::new();
    let mut asset_roles: BTreeMap<String, String> = BTreeMap::new();
    let mut exposures_by_asset: AssetExposures = BTreeMap::new();
    let mut all_assets = BTreeSet::new();
    let mut negative_assets = BTreeSet::new();
    let mut right_censored_assets = BTreeSet::new();
    let mut negative_exposure_ms = 0_u128;
    let mut right_at_risk_ms = 0_u128;

    for exposure in &manifest.exposures {
        if !nonempty(&exposure.exposure_id)
            || !exposure_ids.insert(exposure.exposure_id.clone())
            || !nonempty(&exposure.asset_id)
            || exposure.window_start_unix_ms >= exposure.window_end_unix_ms
            || !["negative_control", "event_monitoring", "right_censored_followup"]
                .contains(&exposure.role.as_str())
        {
            return Err("exposure identity, role or half-open time window is invalid".to_owned());
        }
        if let Some(role) = asset_roles.get(&exposure.asset_id) {
            if role != &exposure.role {
                return Err(format!(
                    "asset {} mixes exposure roles",
                    exposure.asset_id
                ));
            }
        } else {
            asset_roles.insert(exposure.asset_id.clone(), exposure.role.clone());
        }
        let duration = (exposure.window_end_unix_ms - exposure.window_start_unix_ms) as u128;
        match exposure.role.as_str() {
            "negative_control" => {
                negative_assets.insert(exposure.asset_id.clone());
                negative_exposure_ms += duration;
            }
            "right_censored_followup" => {
                right_censored_assets.insert(exposure.asset_id.clone());
                right_at_risk_ms += duration;
            }
            _ => {}
        }
        all_assets.insert(exposure.asset_id.clone());
        exposures_by_asset
            .entry(exposure.asset_id.clone())
            .or_default()
            .push(exposure);
    }

    for (asset_id, intervals) in &mut exposures_by_asset {
        intervals.sort_by_key(|exposure| exposure.window_start_unix_ms);
        for pair in intervals.windows(2) {
            if pair[1].window_start_unix_ms < pair[0].window_end_unix_ms {
                return Err(format!("monitoring intervals overlap for asset {asset_id}"));
            }
        }
    }

    let mut events_by_asset: BTreeMap<String, Vec<&FaultEvent>> = BTreeMap::new();
    let mut event_window_index: EventWindowIndex = BTreeMap::new();
    let mut exact_count = 0usize;
    let mut interval_count = 0usize;
    let mut left_count = 0usize;

    for event in &manifest.events {
        let recorded_time_semantics_valid = match event.onset_type.as_str() {
            "exact_reported" => event.event_time_semantics != "last_normal_first_abnormal_bracket",
            "interval_censored" => [
                "last_normal_first_abnormal_bracket",
                "other_recorded_time",
            ]
            .contains(&event.event_time_semantics.as_str()),
            "left_censored" => event.event_time_semantics != "last_normal_first_abnormal_bracket",
            _ => false,
        };
        let mut evidence_refs = BTreeSet::new();
        if !recorded_time_semantics_valid
            || !nonempty(&event.onset_lower_semantics)
            || !nonempty(&event.onset_upper_semantics)
            || !nonempty(&event.event_id)
            || !event_ids.insert(event.event_id.clone())
            || !nonempty(&event.asset_id)
            || !nonempty(&event.adjudication_revision)
            || event.onset_evidence_refs.is_empty()
            || event.onset_evidence_refs.iter().any(|r| !nonempty(r) || !evidence_refs.insert(r))
            || !["reported_bms_alarm", "first_positive_sample", "maintenance_confirmation",
                "teardown_adjudication", "independently_adjudicated_onset",
                "last_normal_first_abnormal_bracket", "other_recorded_time"]
                .contains(&event.event_time_semantics.as_str())
        {
            return Err("event identity, evidence references or time semantics are invalid".to_owned());
        }
        let lower = event.onset_lower_unix_ms;
        match event.onset_type.as_str() {
            "exact_reported"
                if lower == Some(event.onset_upper_unix_ms)
                    && event.onset_lower_semantics == "exact_reported_instant"
                    && event.onset_upper_semantics == "exact_reported_instant" =>
            {
                exact_count += 1;
            }
            "interval_censored"
                if lower.is_some_and(|value| value < event.onset_upper_unix_ms)
                    && [
                        "earliest_possible_instant_after_last_verified_normal",
                        "other_adjudicated_lower_bound",
                    ]
                    .contains(&event.onset_lower_semantics.as_str())
                    && [
                        "first_verified_abnormal_sample",
                        "other_adjudicated_upper_bound",
                    ]
                    .contains(&event.onset_upper_semantics.as_str()) =>
            {
                interval_count += 1;
            }
            "left_censored"
                if lower.is_none()
                    && event.onset_lower_semantics == "not_available_left_censored"
                    && [
                        "first_verified_abnormal_sample",
                        "other_adjudicated_upper_bound",
                    ]
                    .contains(&event.onset_upper_semantics.as_str()) =>
            {
                left_count += 1;
            },
            _ => {
                return Err(format!(
                    "event {} has inconsistent onset type and interval bounds",
                    event.event_id
                ));
            }
        }
        let intervals = exposures_by_asset.get(&event.asset_id).ok_or_else(|| {
            format!("event {} has no monitoring exposure", event.event_id)
        })?;
        if asset_roles.get(&event.asset_id).map(String::as_str) != Some("event_monitoring") {
            return Err(format!(
                "event {} must be bound to event-monitoring exposure",
                event.event_id
            ));
        }
        if !contains_point(intervals, event.onset_upper_unix_ms) {
            return Err(format!(
                "event {} upper-bound observation is outside monitoring exposure",
                event.event_id
            ));
        }
        if let Some(lower) = lower {
            let start = lower.checked_sub(manifest.warning_horizon_ms).ok_or_else(|| {
                format!("event {} cannot represent its warning window without underflow", event.event_id)
            })?;
            if start >= event.onset_upper_unix_ms {
                return Err(format!("event {} has an empty possible warning window", event.event_id));
            }
            event_window_index
                .entry(event.asset_id.clone())
                .or_default()
                .push((start, event.onset_upper_unix_ms, event.event_id.clone()));
        }
        events_by_asset
            .entry(event.asset_id.clone())
            .or_default()
            .push(event);
    }

    for asset_id in right_censored_assets.iter().chain(negative_assets.iter()) {
        if events_by_asset.contains_key(asset_id) {
            return Err(format!(
                "event-free exposure asset {} also contains a labelled event",
                asset_id
            ));
        }
    }

    for (asset_id, windows) in &mut event_window_index {
        windows.sort_by_key(|window| window.0);
        for pair in windows.windows(2) {
            if pair[1].0 < pair[0].1 {
                return Err(format!(
                    "possible warning windows overlap for asset {asset_id}; event attribution is ambiguous"
                ));
            }
        }
    }
    for event in manifest.events.iter().filter(|event| event.onset_type == "left_censored") {
        if event_window_index
            .get(&event.asset_id)
            .is_some_and(|windows| {
                windows.iter().any(|(start, end, _)| {
                    event.onset_upper_unix_ms >= *start
                        && event.onset_upper_unix_ms < *end
                })
            })
        {
            return Err(format!(
                "left-censored event {} overlaps another event's possible warning window",
                event.event_id
            ));
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
            return Err("alarm identity or emitted-score/threshold contract is invalid".to_owned());
        }
        let intervals = exposures_by_asset.get(&alarm.asset_id).ok_or_else(|| {
            format!("alarm {} has no declared monitoring exposure", alarm.alarm_id)
        })?;
        if !contains_point(intervals, alarm.alarm_time_unix_ms) {
            return Err(format!("alarm {} falls outside monitoring exposure", alarm.alarm_id));
        }
        alarm_refs.push(alarm);
    }
    alarm_refs.sort_by_key(|alarm| (alarm.asset_id.as_str(), alarm.alarm_time_unix_ms, alarm.alarm_id.as_str()));

    let mut eligible_events = 0usize;
    let mut unobservable_events = 0usize;
    let mut definite_warnings = 0usize;
    let mut possible_only_warnings = 0usize;
    let mut possible_pre_onset_only_warnings = 0usize;
    let mut ambiguous_ordering_warnings = 0usize;
    let mut misses = 0usize;
    let mut actionable_warnings = 0usize;
    let mut primary_alarms = 0usize;
    let mut repeated_alarms = 0usize;
    let mut unobservable_event_alarm_count = 0usize;
    let mut left_ambiguous_alarm_count = 0usize;
    let mut assigned_alarm_ids = BTreeSet::new();
    let mut lead_lower_hours = Vec::new();
    let mut lead_upper_hours = Vec::new();

    for event in &manifest.events {
        let Some(lower) = event.onset_lower_unix_ms else {
            let left_alarm_ids = alarm_refs
                .iter()
                .filter(|alarm| {
                    alarm.asset_id == event.asset_id
                        && alarm.alarm_time_unix_ms < event.onset_upper_unix_ms
                        && !assigned_alarm_ids.contains(&alarm.alarm_id)
                })
                .map(|alarm| alarm.alarm_id.clone())
                .collect::<Vec<_>>();
            for alarm_id in left_alarm_ids {
                if assigned_alarm_ids.insert(alarm_id) {
                    left_ambiguous_alarm_count += 1;
                }
            }
            continue;
        };

        let possible_start = lower.checked_sub(manifest.warning_horizon_ms).ok_or_else(|| {
            format!("event {} warning horizon underflow", event.event_id)
        })?;
        let possible_end = event.onset_upper_unix_ms;
        let intervals = exposures_by_asset.get(&event.asset_id).expect("event exposure");
        let complete = covers_range(intervals, possible_start, possible_end);

        let mut matching = alarm_refs
            .iter()
            .filter(|alarm| {
                alarm.asset_id == event.asset_id
                    && alarm.alarm_time_unix_ms >= possible_start
                    && alarm.alarm_time_unix_ms < possible_end
                    && !assigned_alarm_ids.contains(&alarm.alarm_id)
            })
            .collect::<Vec<_>>();

        let guaranteed_start = event
            .onset_upper_unix_ms
            .checked_sub(manifest.warning_horizon_ms)
            .ok_or_else(|| format!("event {} guaranteed window underflow", event.event_id))?;
        let is_guaranteed = |alarm: &&AlarmEpisode| {
            alarm.alarm_time_unix_ms >= guaranteed_start
                && alarm.alarm_time_unix_ms < lower
        };
        matching.sort_by_key(|alarm| {
            (
                if is_guaranteed(alarm) { 0_u8 } else { 1_u8 },
                alarm.alarm_time_unix_ms,
                alarm.alarm_id.as_str(),
            )
        });

        if !complete {
            unobservable_events += 1;
            unobservable_event_alarm_count += matching.len();
            for alarm in &matching {
                assigned_alarm_ids.insert(alarm.alarm_id.clone());
            }
        } else {
            eligible_events += 1;
            if let Some(primary) = matching.first() {
                primary_alarms += 1;
                assigned_alarm_ids.insert(primary.alarm_id.clone());
                for repeat in matching.iter().skip(1) {
                    assigned_alarm_ids.insert(repeat.alarm_id.clone());
                }
                repeated_alarms += matching.len().saturating_sub(1);
                if is_guaranteed(primary) {
                    definite_warnings += 1;
                    if let Some(minimum_bound) = lower.checked_sub(manifest.minimum_action_lead_ms) {
                        if primary.alarm_time_unix_ms <= minimum_bound {
                            actionable_warnings += 1;
                        }
                    }
                } else {
                    possible_only_warnings += 1;
                    if primary.alarm_time_unix_ms < lower {
                        possible_pre_onset_only_warnings += 1;
                    } else {
                        ambiguous_ordering_warnings += 1;
                    }
                }
                if primary.alarm_time_unix_ms < lower {
                    let lead_low = (lower - primary.alarm_time_unix_ms) as f64 / HOUR_MS;
                    let lead_high = (event.onset_upper_unix_ms - primary.alarm_time_unix_ms) as f64 / HOUR_MS;
                    lead_lower_hours.push(lead_low);
                    lead_upper_hours.push(lead_high);
                }
            } else {
                misses += 1;
            }
        }
    }

    let mut negative_false_alarms = 0usize;
    let mut right_false_alarms = 0usize;
    let mut right_unresolved_alarms = 0usize;
    let mut unmatched_event_alarms = 0usize;

    for alarm in &alarm_refs {
        if assigned_alarm_ids.contains(&alarm.alarm_id) {
            continue;
        }
        match asset_roles.get(&alarm.asset_id).map(String::as_str) {
            Some("negative_control") => negative_false_alarms += 1,
            Some("right_censored_followup") => {
                let end = alarm.alarm_time_unix_ms.checked_add(manifest.warning_horizon_ms);
                let intervals = exposures_by_asset.get(&alarm.asset_id).expect("right exposure");
                if end.is_some_and(|future_end| covers_range(intervals, alarm.alarm_time_unix_ms, future_end)) {
                    right_false_alarms += 1;
                } else {
                    right_unresolved_alarms += 1;
                }
            }
            Some("event_monitoring") => unmatched_event_alarms += 1,
            _ => return Err(format!("alarm {} has no exposure role", alarm.alarm_id)),
        }
    }

    let mut right_horizon_edge_ms = 0_u128;
    let mut right_eligible_ms = 0_u128;
    for asset_id in &right_censored_assets {
        let intervals = exposures_by_asset.get(asset_id).expect("right-censored exposure");
        for (start, end) in contiguous_blocks(intervals) {
            let duration = (end - start) as u128;
            let edge = duration.min(manifest.warning_horizon_ms as u128);
            right_horizon_edge_ms += edge;
            right_eligible_ms += duration - edge;
        }
    }

    let negative_hours = negative_exposure_ms as f64 / HOUR_MS;
    let right_at_risk_hours = right_at_risk_ms as f64 / HOUR_MS;
    let right_edge_hours = right_horizon_edge_ms as f64 / HOUR_MS;
    let right_eligible_hours = right_eligible_ms as f64 / HOUR_MS;
    let definite_coverage = (eligible_events > 0)
        .then_some(definite_warnings as f64 / eligible_events as f64);
    let possible_coverage = (eligible_events > 0)
        .then_some((definite_warnings + possible_only_warnings) as f64 / eligible_events as f64);
    let negative_rate = (negative_hours > 0.0)
        .then_some(negative_false_alarms as f64 / negative_hours * 1000.0);
    let right_rate = (right_eligible_hours > 0.0)
        .then_some(right_false_alarms as f64 / right_eligible_hours * 1000.0);

    let report = EventCensoringReport {
        report_version: "ev-bms-event-censoring-report/v0.1".to_owned(),
        evaluation_id: manifest.evaluation_id.clone(),
        dataset_id: manifest.dataset_id.clone(),
        dataset_domain: manifest.dataset_domain.clone(),
        dataset_admission_state: manifest.dataset_admission_state.clone(),
        dataset_manifest_sha256: manifest.dataset_manifest_sha256.clone(),
        dataset_snapshot_sha256: manifest.dataset_snapshot_sha256.clone(),
        split_manifest_sha256: manifest.split_manifest_sha256.clone(),
        split_partition: manifest.split_partition.clone(),
        evidence_class: manifest.evidence_class.clone(),
        time_basis_id: manifest.time_basis_id.clone(),
        time_alignment_id: manifest.time_alignment_id.clone(),
        time_alignment_artifact_sha256: manifest.time_alignment_artifact_sha256.clone(),
        time_alignment_method: manifest.time_alignment_method.clone(),
        max_relative_clock_error_ms: manifest.max_relative_clock_error_ms,
        alignment_uncertainty_preapplied: manifest.alignment_uncertainty_preapplied,
        warning_horizon_ms: manifest.warning_horizon_ms,
        minimum_action_lead_ms: manifest.minimum_action_lead_ms,
        threshold_policy_id: manifest.threshold_policy_id.clone(),
        independent_asset_count: all_assets.len(),
        event_count: manifest.events.len(),
        interval_censored_event_count: interval_count,
        exact_reported_event_count: exact_count,
        left_censored_event_count: left_count,
        eligible_event_count: eligible_events,
        unobservable_event_count: unobservable_events,
        unobservable_event_window_alarm_count: unobservable_event_alarm_count,
        definite_warning_count: definite_warnings,
        possible_only_warning_count: possible_only_warnings,
        possible_pre_onset_only_warning_count: possible_pre_onset_only_warnings,
        ambiguous_ordering_warning_count: ambiguous_ordering_warnings,
        possible_warning_count: definite_warnings + possible_only_warnings,
        event_miss_count: misses,
        definite_warning_coverage_fraction: definite_coverage,
        possible_warning_coverage_fraction: possible_coverage,
        guaranteed_actionable_warning_count: actionable_warnings,
        primary_event_alarm_count: primary_alarms,
        repeated_event_alarm_count: repeated_alarms,
        unmatched_event_monitoring_alarm_count: unmatched_event_alarms,
        left_censored_ambiguous_alarm_count: left_ambiguous_alarm_count,
        negative_control_asset_count: negative_assets.len(),
        negative_control_exposure_hours: negative_hours,
        negative_control_false_alarm_count: negative_false_alarms,
        negative_control_false_alarms_per_1000_hours: negative_rate,
        right_censored_asset_count: right_censored_assets.len(),
        right_censored_at_risk_hours: right_at_risk_hours,
        right_censored_horizon_edge_hours: right_edge_hours,
        right_censored_eligible_forecast_hours: right_eligible_hours,
        right_censored_false_alarm_count: right_false_alarms,
        right_censored_unresolved_alarm_count: right_unresolved_alarms,
        right_censored_false_alarms_per_1000_eligible_hours: right_rate,
        lead_interval_count: lead_lower_hours.len(),
        lead_lower_hours_min: lead_lower_hours.iter().copied().reduce(f64::min),
        lead_lower_hours_p05: percentile(&lead_lower_hours, 0.05),
        lead_lower_hours_p50: percentile(&lead_lower_hours, 0.50),
        lead_lower_hours_p95: percentile(&lead_lower_hours, 0.95),
        lead_lower_hours_max: lead_lower_hours.iter().copied().reduce(f64::max),
        lead_upper_hours_min: lead_upper_hours.iter().copied().reduce(f64::min),
        lead_upper_hours_p05: percentile(&lead_upper_hours, 0.05),
        lead_upper_hours_p50: percentile(&lead_upper_hours, 0.50),
        lead_upper_hours_p95: percentile(&lead_upper_hours, 0.95),
        lead_upper_hours_max: lead_upper_hours.iter().copied().reduce(f64::max),
        total_alarm_episode_count: alarm_refs.len(),
        schema_sha256: String::new(),
        canonical_manifest_sha256: String::new(),
        claim_ceiling: manifest.claim_ceiling.clone(),
        report_sha256: String::new(),
    };
    Ok(report)
}

pub fn audit_event_censoring_evaluation(
    schema_bytes: &[u8],
    manifest_bytes: &[u8],
) -> Result<EventCensoringReport, String> {
    let schema: Value =
        serde_json::from_slice(schema_bytes).map_err(|error| format!("schema JSON: {error}"))?;
    if schema.get("$schema").and_then(Value::as_str)
        != Some("https://json-schema.org/draft/2020-12/schema")
    {
        return Err("event-censoring schema must declare JSON Schema Draft 2020-12".to_owned());
    }
    if schema.get("$id").and_then(Value::as_str) != Some(SCHEMA_ID) {
        return Err("event-censoring schema identity does not match v0.1".to_owned());
    }
    if !draft202012::meta::is_valid(&schema) {
        return Err("event-censoring schema fails Draft 2020-12 meta-schema validation".to_owned());
    }
    let validator = draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
        .map_err(|error| format!("event-censoring schema compilation: {error}"))?;

    let mut value: Value = serde_json::from_slice(manifest_bytes)
        .map_err(|error| format!("event-censoring JSON: {error}"))?;
    canonicalize(&mut value);
    if let Err(error) = validator.validate(&value) {
        return Err(format!("event-censoring schema validation: {error}"));
    }
    let canonical_bytes =
        serde_json::to_vec(&value).map_err(|error| format!("canonical evaluation: {error}"))?;
    let manifest: EvaluationManifest = serde_json::from_slice(&canonical_bytes)
        .map_err(|error| format!("typed evaluation manifest: {error}"))?;
    let mut report = validate_and_evaluate(&manifest)?;
    report.schema_sha256 = sha256(schema_bytes);
    report.canonical_manifest_sha256 = sha256(&canonical_bytes);
    let report_bytes = serde_json::to_vec(&report)
        .map_err(|error| format!("event-censoring report serialization: {error}"))?;
    report.report_sha256 = sha256(&report_bytes);
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const SCHEMA: &[u8] = include_bytes!(
        "../../../schemas/transport/ev_bms_event_censoring_evaluation_v0_1.schema.json"
    );
    const FIXTURE: &[u8] = include_bytes!(
        "../../../fixtures/ev_battery/event_censoring_synthetic_v0_1.json"
    );

    fn report() -> EventCensoringReport {
        audit_event_censoring_evaluation(SCHEMA, FIXTURE)
            .expect("synthetic interval-censored event evaluation")
    }

    #[test]
    fn known_answer_separates_definite_possible_censored_and_false_alarm_rates() {
        let report = report();
        assert_eq!(report.independent_asset_count, 3);
        assert_eq!(report.event_count, 4);
        assert_eq!(report.exact_reported_event_count, 1);
        assert_eq!(report.interval_censored_event_count, 2);
        assert_eq!(report.left_censored_event_count, 1);
        assert_eq!(report.eligible_event_count, 3);
        assert_eq!(report.unobservable_event_count, 0);
        assert_eq!(report.definite_warning_count, 1);
        assert_eq!(report.possible_only_warning_count, 1);
        assert_eq!(report.possible_pre_onset_only_warning_count, 0);
        assert_eq!(report.ambiguous_ordering_warning_count, 1);
        assert_eq!(report.event_miss_count, 1);
        assert!((report.definite_warning_coverage_fraction.unwrap() - (1.0 / 3.0)).abs() < 1e-12);
        assert!((report.possible_warning_coverage_fraction.unwrap() - (2.0 / 3.0)).abs() < 1e-12);
        assert_eq!(report.guaranteed_actionable_warning_count, 1);
        assert_eq!(report.left_censored_ambiguous_alarm_count, 1);
        assert_eq!(report.negative_control_false_alarm_count, 2);
        assert!((report.negative_control_exposure_hours - 100.0).abs() < 1e-12);
        assert!((report.negative_control_false_alarms_per_1000_hours.unwrap() - 20.0).abs() < 1e-12);
        assert_eq!(report.right_censored_false_alarm_count, 1);
        assert_eq!(report.right_censored_unresolved_alarm_count, 1);
        assert!((report.right_censored_at_risk_hours - 50.0).abs() < 1e-12);
        assert!((report.right_censored_horizon_edge_hours - 24.0).abs() < 1e-12);
        assert!((report.right_censored_eligible_forecast_hours - 26.0).abs() < 1e-12);
        assert!((report.right_censored_false_alarms_per_1000_eligible_hours.unwrap()
            - (1000.0 / 26.0)).abs() < 1e-9);
        assert_eq!(report.lead_interval_count, 1);
        assert!((report.lead_lower_hours_min.unwrap() - 4.0).abs() < 1e-12);
        assert!((report.lead_upper_hours_min.unwrap() - 4.0).abs() < 1e-12);
    }

    #[test]
    fn interval_warning_boundaries_are_half_open_and_conservative() {
        let mut value: Value = serde_json::from_slice(FIXTURE).expect("fixture JSON");
        let alarms = value["alarms"].as_array_mut().unwrap();
        let exact_alarm = alarms
            .iter_mut()
            .find(|alarm| alarm["alarm_id"] == "alarm-event-exact")
            .expect("exact event alarm");
        exact_alarm["alarm_time_unix_ms"] = Value::from(76_u64 * 3_600_000);
        let interval_alarm = alarms
            .iter_mut()
            .find(|alarm| alarm["alarm_id"] == "alarm-event-interval")
            .expect("interval event alarm");
        interval_alarm["alarm_time_unix_ms"] = Value::from(200_u64 * 3_600_000);
        let bytes = serde_json::to_vec(&value).expect("lower-bound fixture");
        let at_lower = audit_event_censoring_evaluation(SCHEMA, &bytes).expect("inclusive lower boundary");
        assert_eq!(at_lower.definite_warning_count, 1);
        assert_eq!(at_lower.ambiguous_ordering_warning_count, 1);
        assert!((at_lower.lead_lower_hours_min.unwrap() - 24.0).abs() < 1e-12);

        let mut upper_value: Value = serde_json::from_slice(FIXTURE).expect("fixture JSON");
        let alarms = upper_value["alarms"].as_array_mut().unwrap();
        let exact_alarm = alarms
            .iter_mut()
            .find(|alarm| alarm["alarm_id"] == "alarm-event-exact")
            .expect("exact event alarm");
        exact_alarm["alarm_time_unix_ms"] = Value::from(100_u64 * 3_600_000);
        let upper_bytes = serde_json::to_vec(&upper_value).expect("exclusive upper-bound fixture");
        let at_upper = audit_event_censoring_evaluation(SCHEMA, &upper_bytes).expect("exclusive upper boundary");
        assert_eq!(at_upper.definite_warning_count, 0);
        assert_eq!(at_upper.event_miss_count, 2);
    }

    #[test]
    fn wide_onset_interval_has_possible_but_no_guaranteed_warning_window() {
        let mut value: Value = serde_json::from_slice(FIXTURE).expect("fixture JSON");
        value["alarms"].as_array_mut().unwrap().push(serde_json::json!({
            "alarm_id": "alarm-wide-interval-pre-onset",
            "asset_id": "asset-event",
            "alarm_time_unix_ms": 310_u64 * 3_600_000,
            "score": 0.9,
            "threshold": 0.8
        }));
        let bytes = serde_json::to_vec(&value).expect("wide interval alarm manifest");
        let report = audit_event_censoring_evaluation(SCHEMA, &bytes).expect("wide interval evaluation");
        assert_eq!(report.definite_warning_count, 1);
        assert_eq!(report.possible_pre_onset_only_warning_count, 1);
        assert_eq!(report.ambiguous_ordering_warning_count, 1);
        assert_eq!(report.possible_only_warning_count, 2);
        assert_eq!(report.event_miss_count, 0);
        assert_eq!(report.lead_interval_count, 2);
        assert!((report.lead_lower_hours_min.unwrap() - 4.0).abs() < 1e-12);
        assert!((report.lead_upper_hours_max.unwrap() - 30.0).abs() < 1e-12);
    }

    #[test]
    fn right_censored_horizon_addition_overflow_is_unresolved_not_false() {
        let mut value: Value = serde_json::from_slice(FIXTURE).expect("fixture JSON");
        value["warning_horizon_ms"] = Value::from(1000_u64);
        value["minimum_action_lead_ms"] = Value::from(0_u64);
        value["events"] = serde_json::json!([]);
        value["exposures"] = serde_json::json!([{
            "exposure_id": "exp-near-u64-max",
            "asset_id": "asset-right-only",
            "role": "right_censored_followup",
            "window_start_unix_ms": u64::MAX - 2000,
            "window_end_unix_ms": u64::MAX
        }]);
        value["alarms"] = serde_json::json!([{
            "alarm_id": "alarm-near-u64-max",
            "asset_id": "asset-right-only",
            "alarm_time_unix_ms": u64::MAX - 500,
            "score": 0.9,
            "threshold": 0.8
        }]);
        let bytes = serde_json::to_vec(&value).expect("overflow fixture");
        let report = audit_event_censoring_evaluation(SCHEMA, &bytes).expect("overflow evaluation");
        assert_eq!(report.right_censored_false_alarm_count, 0);
        assert_eq!(report.right_censored_unresolved_alarm_count, 1);
    }

    #[test]
    fn incomplete_possible_warning_coverage_is_unobservable_not_a_miss() {
        let mut value: Value = serde_json::from_slice(FIXTURE).expect("fixture JSON");
        let exposures = value["exposures"].as_array_mut().unwrap();
        let intervals = exposures;
        let third = intervals
            .iter_mut()
            .find(|exposure| {
                exposure["asset_id"] == "asset-event"
                    && exposure["window_start_unix_ms"] == Value::from(276_u64 * 3_600_000)
            })
            .expect("third event monitoring interval");
        third["window_end_unix_ms"] = Value::from(300_u64 * 3_600_000);
        intervals.push(serde_json::json!({
            "exposure_id": "exp-event-003b",
            "asset_id": "asset-event",
            "role": "event_monitoring",
            "window_start_unix_ms": 310_u64 * 3_600_000,
            "window_end_unix_ms": 341_u64 * 3_600_000
        }));
        let bytes = serde_json::to_vec(&value).expect("coverage gap manifest");
        let report = audit_event_censoring_evaluation(SCHEMA, &bytes).expect("gapped evaluation");
        assert_eq!(report.unobservable_event_count, 1);
        assert_eq!(report.eligible_event_count, 2);
        assert_eq!(report.event_miss_count, 0);
    }

    #[test]
    fn rejects_inconsistent_exact_and_interval_bounds() {
        let mut value: Value = serde_json::from_slice(FIXTURE).expect("fixture JSON");
        value["events"][0]["onset_upper_unix_ms"] = Value::from(99_u64 * 3_600_000);
        let bytes = serde_json::to_vec(&value).expect("invalid exact bounds");
        assert!(audit_event_censoring_evaluation(SCHEMA, &bytes).is_err());
    }

    #[test]
    fn metadata_only_fixture_cannot_claim_a_field_clock_alignment() {
        let mut value: Value = serde_json::from_slice(FIXTURE).expect("fixture JSON");
        value["time_alignment_method"] = Value::from("ptp_ntp_verified");
        let bytes = serde_json::to_vec(&value).expect("mismatched alignment method");
        assert!(audit_event_censoring_evaluation(SCHEMA, &bytes).is_err());
    }

    #[test]
    fn rejects_alarm_score_not_above_frozen_threshold() {
        let mut value: Value = serde_json::from_slice(FIXTURE).expect("fixture JSON");
        value["alarms"][0]["score"] = Value::from(0.5);
        value["alarms"][0]["threshold"] = Value::from(0.5);
        let bytes = serde_json::to_vec(&value).expect("invalid alarm");
        assert!(audit_event_censoring_evaluation(SCHEMA, &bytes).is_err());
    }

    #[test]
    fn canonical_digest_is_invariant_to_input_array_order() {
        let original = report();
        let mut value: Value = serde_json::from_slice(FIXTURE).expect("fixture JSON");
        value["exposures"].as_array_mut().unwrap().reverse();
        value["events"].as_array_mut().unwrap().reverse();
        value["alarms"].as_array_mut().unwrap().reverse();
        let bytes = serde_json::to_vec(&value).expect("permuted manifest");
        let replay = audit_event_censoring_evaluation(SCHEMA, &bytes).expect("permuted evaluation");
        assert_eq!(original.report_sha256, replay.report_sha256);
    }
}
