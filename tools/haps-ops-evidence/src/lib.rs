//! Validate HAPS operations and maintenance evidence ledgers.
//!
//! This crate checks sequence, state transitions, timestamp semantics,
//! configuration references, censoring and release-authority boundaries.
//! It does not validate physical maintenance, cryptographic signatures,
//! airworthiness, or the truth of supplied source evidence.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

const TIME_TOLERANCE_S: i64 = 0;
const SECONDS_PER_DAY: i64 = 86_400;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerError {
    Json(String),
    InvalidLedger(String),
    InvalidEvent(String),
    InvalidExposure(String),
    AuthorizationBoundary(String),
}

impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(message)
            | Self::InvalidLedger(message)
            | Self::InvalidEvent(message)
            | Self::InvalidExposure(message)
            | Self::AuthorizationBoundary(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for LedgerError {}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpsEvidenceLedger {
    pub schema_version: String,
    pub ledger_id: String,
    pub revision: u64,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub observation_window: ObservationWindow,
    pub assets: Vec<Asset>,
    pub events: Vec<OpsEvent>,
    pub exposure_records: Vec<ExposureRecord>,
    pub authorization_boundary: AuthorizationBoundary,
    #[serde(default)]
    pub source_evidence_refs: Vec<String>,
    pub assumptions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationWindow {
    pub start_utc: String,
    pub end_utc: String,
    pub time_basis: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    pub asset_ref: String,
    pub lift_class: String,
    pub configuration_ref: String,
    pub configuration_revision: String,
    pub initial_state: String,
    #[serde(default)]
    pub component_refs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpsEvent {
    pub event_id: String,
    pub event_sequence: u64,
    pub previous_event_id: Option<String>,
    pub asset_ref: String,
    pub configuration_ref: String,
    pub event_type: String,
    pub occurred_at_utc: Option<String>,
    pub observed_at_utc: String,
    pub recorded_at_utc: String,
    pub clock_uncertainty_s: f64,
    pub from_state: String,
    pub to_state: String,
    pub onset_knowledge: String,
    pub onset_interval_earliest_utc: Option<String>,
    pub onset_interval_latest_utc: Option<String>,
    #[serde(default)]
    pub component_refs: Vec<String>,
    pub producer: Producer,
    pub source_evidence_refs: Vec<EventEvidenceRef>,
    pub event_digest: Option<String>,
    pub release_authority_ref: Option<String>,
    pub payload: Value,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Producer {
    pub organization_ref: String,
    pub role: String,
    pub source_system_ref: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventEvidenceRef {
    pub evidence_ref: String,
    pub evidence_kind: String,
    pub content_digest: Option<String>,
    pub source_uri: Option<String>,
    pub rights_ref: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExposureRecord {
    pub exposure_id: String,
    pub asset_ref: String,
    pub configuration_ref: String,
    pub exposure_type: String,
    pub start_utc: String,
    pub end_utc: String,
    pub exposure_seconds: f64,
    pub failure_observation: String,
    pub censoring_status: String,
    pub event_time_lower_bound_utc: Option<String>,
    pub event_time_upper_bound_utc: Option<String>,
    pub source_evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorizationBoundary {
    pub release_status: String,
    pub return_to_service_authorized: bool,
    pub release_authority_ref: Option<String>,
    pub release_scope: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct OpsEvidenceReport {
    pub ledger_id: String,
    pub evidence_class: String,
    pub event_count: usize,
    pub exposure_record_count: usize,
    pub final_asset_states: BTreeMap<String, String>,
    pub right_censored_exposure_seconds: f64,
    pub interval_censored_record_count: usize,
    pub return_to_service_authorized: bool,
    pub release_status: String,
    pub claim_ceiling: String,
}

pub fn parse_ledger_json(bytes: &[u8]) -> Result<OpsEvidenceLedger, LedgerError> {
    let value: Value =
        serde_json::from_slice(bytes).map_err(|error| LedgerError::Json(error.to_string()))?;
    require_keys(
        &value,
        "operations evidence ledger",
        &[
            "schema_version",
            "ledger_id",
            "revision",
            "evidence_class",
            "claim_ceiling",
            "observation_window",
            "assets",
            "events",
            "exposure_records",
            "authorization_boundary",
            "assumptions",
        ],
    )?;
    let ledger: OpsEvidenceLedger =
        serde_json::from_value(value).map_err(|error| LedgerError::Json(error.to_string()))?;
    validate_ledger(&ledger)?;
    Ok(ledger)
}

fn require_keys(value: &Value, context: &str, keys: &[&str]) -> Result<(), LedgerError> {
    let object = value.as_object().ok_or_else(|| {
        LedgerError::InvalidLedger(format!("{context} must be a JSON object"))
    })?;
    for key in keys {
        if !object.contains_key(*key) {
            return Err(LedgerError::InvalidLedger(format!(
                "{context} is missing required field {key}"
            )));
        }
    }
    Ok(())
}

fn parse_utc_seconds(value: &str, label: &str) -> Result<i64, LedgerError> {
    // v1 uses canonical UTC timestamps with whole-second precision: YYYY-MM-DDTHH:MM:SSZ.
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return Err(LedgerError::InvalidLedger(format!(
            "{label} must use canonical UTC format YYYY-MM-DDTHH:MM:SSZ"
        )));
    }
    let parse = |range: std::ops::Range<usize>| -> Result<i64, LedgerError> {
        value[range]
            .parse::<i64>()
            .map_err(|_| LedgerError::InvalidLedger(format!("invalid {label}")))
    };
    let year = parse(0..4)?;
    let month = parse(5..7)?;
    let day = parse(8..10)?;
    let hour = parse(11..13)?;
    let minute = parse(14..16)?;
    let second = parse(17..19)?;
    if !(1..=12).contains(&month)
        || !(0..=23).contains(&hour)
        || !(0..=59).contains(&minute)
        || !(0..=59).contains(&second)
    {
        return Err(LedgerError::InvalidLedger(format!(
            "{label} has an out-of-range clock field"
        )));
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => unreachable!(),
    };
    if day < 1 || day > days_in_month {
        return Err(LedgerError::InvalidLedger(format!(
            "{label} has an invalid day of month"
        )));
    }

    // Howard Hinnant's civil-date conversion to days since 1970-01-01.
    let y = year - i64::from(month <= 2);
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let year_of_era = y - era * 400;
    let adjusted_month = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * adjusted_month + 2) / 5 + day - 1;
    let day_of_era =
        year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;
    Ok(days * SECONDS_PER_DAY + hour * 3_600 + minute * 60 + second)
}

fn valid_state(state: &str) -> bool {
    matches!(
        state,
        "ready_for_dispatch"
            | "launch_recovery"
            | "in_service"
            | "postflight_inspection"
            | "grounded"
            | "maintenance_queued"
            | "maintenance_in_progress"
            | "verification_hold"
            | "awaiting_authorized_release"
            | "ready_for_dispatch_pending_authorized_release"
            | "retired"
    )
}

fn valid_transition(event: &OpsEvent) -> bool {
    if event.event_type == "return_to_service_authorized" {
        return event.from_state == "awaiting_authorized_release"
            && event.to_state == "ready_for_dispatch"
            && event.release_authority_ref.as_deref().is_some_and(|value| !value.trim().is_empty());
    }
    match event.event_type.as_str() {
        "sortie_launched" => event.from_state == "ready_for_dispatch" && event.to_state == "launch_recovery",
        "service_started" => event.from_state == "launch_recovery" && event.to_state == "in_service",
        "service_ended" => event.from_state == "in_service" && event.to_state == "postflight_inspection",
        "recovered" => event.from_state == "in_service" && event.to_state == "postflight_inspection",
        "postflight_inspection_started" => event.from_state == "postflight_inspection" && event.to_state == "postflight_inspection",
        "anomaly_detected" | "discrepancy_recorded" | "aircraft_grounded" => {
            event.from_state == "postflight_inspection" && event.to_state == "grounded"
        }
        "work_order_opened" => event.from_state == "grounded" && event.to_state == "maintenance_queued",
        "diagnosis_completed" | "parts_requested" | "parts_received" => {
            event.from_state == "maintenance_queued" && event.to_state == "maintenance_queued"
        }
        "repair_started" => event.from_state == "maintenance_queued" && event.to_state == "maintenance_in_progress",
        "repair_completed" => event.from_state == "maintenance_in_progress" && event.to_state == "verification_hold",
        "inspection_completed" | "functional_test_completed" | "verification_passed" => {
            event.from_state == "verification_hold"
                && (event.to_state == "verification_hold" || event.to_state == "awaiting_authorized_release")
        }
        "verification_failed" => event.from_state == "verification_hold" && event.to_state == "maintenance_queued",
        "awaiting_authorized_release" => {
            event.from_state == "verification_hold" && event.to_state == "awaiting_authorized_release"
        }
        "maintenance_deferred" => {
            event.from_state == "grounded" && event.to_state == "maintenance_queued"
        }
        "configuration_changed" | "instrument_calibrated" => {
            event.from_state == event.to_state
        }
        _ => false,
    }
}

fn validate_ledger(ledger: &OpsEvidenceLedger) -> Result<(), LedgerError> {
    if ledger.schema_version != "haps-ops-evidence-ledger/v1" {
        return Err(LedgerError::InvalidLedger("unsupported schema version".to_owned()));
    }
    if ledger.ledger_id.trim().is_empty() || ledger.assets.is_empty() || ledger.events.is_empty() {
        return Err(LedgerError::InvalidLedger(
            "ledger ID, assets and events must be non-empty".to_owned(),
        ));
    }
    if ledger.evidence_class == "synthetic"
        && ledger.claim_ceiling != "synthetic_maintenance_lifecycle"
    {
        return Err(LedgerError::InvalidLedger(
            "synthetic ledger has an invalid claim ceiling".to_owned(),
        ));
    }
    let observation_start = parse_utc_seconds(&ledger.observation_window.start_utc, "observation start")?;
    let observation_end = parse_utc_seconds(&ledger.observation_window.end_utc, "observation end")?;
    if observation_end <= observation_start || ledger.observation_window.time_basis != "utc" {
        return Err(LedgerError::InvalidLedger(
            "observation window must be positive and use UTC".to_owned(),
        ));
    }

    let mut assets = HashMap::new();
    let mut states = HashMap::new();
    for asset in &ledger.assets {
        if asset.asset_ref.trim().is_empty()
            || asset.configuration_ref.trim().is_empty()
            || !valid_state(&asset.initial_state)
            || assets.insert(asset.asset_ref.as_str(), asset).is_some()
        {
            return Err(LedgerError::InvalidLedger(
                "asset references must be unique and initial states valid".to_owned(),
            ));
        }
        states.insert(asset.asset_ref.clone(), asset.initial_state.clone());
    }

    let mut event_ids = HashSet::new();
    let mut previous_event: Option<&str> = None;
    let mut previous_sequence = 0_u64;
    let mut last_recorded_at = i64::MIN;
    let mut authorized_release_event = false;

    for event in &ledger.events {
        if event.event_id.trim().is_empty() || !event_ids.insert(event.event_id.as_str()) {
            return Err(LedgerError::InvalidEvent(
                "event IDs must be non-empty and unique".to_owned(),
            ));
        }
        if event.event_sequence != previous_sequence + 1 {
            return Err(LedgerError::InvalidEvent(format!(
                "event {} sequence is not contiguous",
                event.event_id
            )));
        }
        if event.previous_event_id.as_deref() != previous_event {
            return Err(LedgerError::InvalidEvent(format!(
                "event {} previous_event_id does not match the prior event",
                event.event_id
            )));
        }
        let asset = assets.get(event.asset_ref.as_str()).ok_or_else(|| {
            LedgerError::InvalidEvent(format!("event {} references unknown asset", event.event_id))
        })?;
        if event.configuration_ref != asset.configuration_ref {
            return Err(LedgerError::InvalidEvent(format!(
                "event {} configuration does not match its asset",
                event.event_id
            )));
        }
        if !valid_state(&event.from_state) || !valid_state(&event.to_state) || !valid_transition(event) {
            return Err(LedgerError::InvalidEvent(format!(
                "event {} has an invalid lifecycle transition",
                event.event_id
            )));
        }
        let current_state = states.get(&event.asset_ref).expect("asset state initialized");
        if &event.from_state != current_state {
            return Err(LedgerError::InvalidEvent(format!(
                "event {} starts in {}, but current asset state is {}",
                event.event_id, event.from_state, current_state
            )));
        }
        if !event.clock_uncertainty_s.is_finite() || event.clock_uncertainty_s < 0.0 {
            return Err(LedgerError::InvalidEvent(format!(
                "event {} has invalid clock uncertainty",
                event.event_id
            )));
        }
        let observed = parse_utc_seconds(&event.observed_at_utc, "observed_at_utc")?;
        let recorded = parse_utc_seconds(&event.recorded_at_utc, "recorded_at_utc")?;
        if recorded < observed {
            return Err(LedgerError::InvalidEvent(format!(
                "event {} was recorded before it was observed",
                event.event_id
            )));
        }
        if recorded < last_recorded_at {
            return Err(LedgerError::InvalidEvent(
                "recorded_at_utc must be non-decreasing across the event chain".to_owned(),
            ));
        }
        if let Some(occurred) = event.occurred_at_utc.as_deref() {
            let occurred = parse_utc_seconds(occurred, "occurred_at_utc")?;
            if occurred > observed {
                return Err(LedgerError::InvalidEvent(format!(
                    "event {} occurred_at_utc is later than observed_at_utc",
                    event.event_id
                )));
            }
            if occurred < observation_start || occurred > observation_end {
                return Err(LedgerError::InvalidEvent(format!(
                    "event {} occurred_at_utc is outside the observation window",
                    event.event_id
                )));
            }
        }
        if observed < observation_start || observed > observation_end || recorded > observation_end {
            return Err(LedgerError::InvalidEvent(format!(
                "event {} timestamps are outside the observation window",
                event.event_id
            )));
        }
        match event.onset_knowledge.as_str() {
            "known" => {
                if event.onset_interval_earliest_utc.is_some() || event.onset_interval_latest_utc.is_some() {
                    return Err(LedgerError::InvalidEvent(
                        "known onset must not also specify censoring bounds".to_owned(),
                    ));
                }
            }
            "interval_censored" => {
                let lower = event.onset_interval_earliest_utc.as_deref().ok_or_else(|| {
                    LedgerError::InvalidEvent("interval-censored onset needs lower bound".to_owned())
                })?;
                let upper = event.onset_interval_latest_utc.as_deref().ok_or_else(|| {
                    LedgerError::InvalidEvent("interval-censored onset needs upper bound".to_owned())
                })?;
                if parse_utc_seconds(lower, "onset lower bound")? > parse_utc_seconds(upper, "onset upper bound")? {
                    return Err(LedgerError::InvalidEvent(
                        "onset interval bounds are reversed".to_owned(),
                    ));
                }
            }
            "unknown" | "not_applicable" => {
                if event.onset_interval_earliest_utc.is_some() || event.onset_interval_latest_utc.is_some() {
                    return Err(LedgerError::InvalidEvent(
                        "unknown/not-applicable onset must not carry interval bounds".to_owned(),
                    ));
                }
            }
            other => return Err(LedgerError::InvalidEvent(format!("unknown onset_knowledge value {other}"))),
        }
        if event.source_evidence_refs.is_empty() || event.producer.organization_ref.trim().is_empty()
            || event.producer.role.trim().is_empty() || event.producer.source_system_ref.trim().is_empty()
        {
            return Err(LedgerError::InvalidEvent(format!(
                "event {} lacks producer or source evidence",
                event.event_id
            )));
        }
        if event.event_type == "return_to_service_authorized" {
            authorized_release_event = true;
        }
        states.insert(event.asset_ref.clone(), event.to_state.clone());
        previous_event = Some(event.event_id.as_str());
        previous_sequence = event.event_sequence;
        last_recorded_at = recorded;
    }

    for exposure in &ledger.exposure_records {
        let asset = assets.get(exposure.asset_ref.as_str()).ok_or_else(|| {
            LedgerError::InvalidExposure(format!(
                "exposure {} references unknown asset",
                exposure.exposure_id
            ))
        })?;
        if exposure.configuration_ref != asset.configuration_ref {
            return Err(LedgerError::InvalidExposure(format!(
                "exposure {} configuration does not match asset",
                exposure.exposure_id
            )));
        }
        let start = parse_utc_seconds(&exposure.start_utc, "exposure start")?;
        let end = parse_utc_seconds(&exposure.end_utc, "exposure end")?;
        if start < observation_start || end > observation_end || end <= start {
            return Err(LedgerError::InvalidExposure(format!(
                "exposure {} has invalid window",
                exposure.exposure_id
            )));
        }
        if !exposure.exposure_seconds.is_finite()
            || (exposure.exposure_seconds - (end - start) as f64).abs() > 1.0
        {
            return Err(LedgerError::InvalidExposure(format!(
                "exposure {} duration does not match its timestamps",
                exposure.exposure_id
            )));
        }
        match exposure.censoring_status.as_str() {
            "right_censored" => {
                if exposure.failure_observation != "no_failure_observed"
                    || exposure.event_time_lower_bound_utc.is_some()
                    || exposure.event_time_upper_bound_utc.is_some()
                {
                    return Err(LedgerError::InvalidExposure(
                        "right-censored exposure must have no observed failure or event-time bounds".to_owned(),
                    ));
                }
            }
            "interval_censored" => {
                if exposure.failure_observation != "failure_or_defect_detected" {
                    return Err(LedgerError::InvalidExposure(
                        "interval-censored exposure must have a detected failure/defect".to_owned(),
                    ));
                }
                let lower = exposure.event_time_lower_bound_utc.as_deref().ok_or_else(|| {
                    LedgerError::InvalidExposure("interval-censored exposure requires a lower bound".to_owned())
                })?;
                let upper = exposure.event_time_upper_bound_utc.as_deref().ok_or_else(|| {
                    LedgerError::InvalidExposure("interval-censored exposure requires an upper bound".to_owned())
                })?;
                if parse_utc_seconds(lower, "exposure lower bound")? > parse_utc_seconds(upper, "exposure upper bound")? {
                    return Err(LedgerError::InvalidExposure("exposure censoring bounds are reversed".to_owned()));
                }
            }
            "uncensored_event" => {
                if exposure.failure_observation != "failure_or_defect_detected" {
                    return Err(LedgerError::InvalidExposure(
                        "uncensored event requires a detected failure/defect".to_owned(),
                    ));
                }
            }
            "left_truncated" | "competing_event" | "not_assessed" => {}
            other => return Err(LedgerError::InvalidExposure(format!("unknown censoring status {other}"))),
        }
        if exposure.source_evidence_refs.is_empty() {
            return Err(LedgerError::InvalidExposure(format!(
                "exposure {} lacks source evidence",
                exposure.exposure_id
            )));
        }
    }

    let boundary = &ledger.authorization_boundary;
    if boundary.return_to_service_authorized {
        if boundary.release_status != "authorized_by_external_authority"
            || boundary.release_authority_ref.as_deref().is_none_or(|value| value.trim().is_empty())
            || boundary.release_scope != "specific_configuration_and_scope"
            || !authorized_release_event
        {
            return Err(LedgerError::AuthorizationBoundary(
                "return-to-service authorization requires an external authority record, specific scope and authorized-release event"
                    .to_owned(),
            ));
        }
    } else if boundary.release_status == "authorized_by_external_authority"
        || boundary.release_authority_ref.is_some()
        || authorized_release_event
    {
        return Err(LedgerError::AuthorizationBoundary(
            "authorization evidence conflicts with return_to_service_authorized=false".to_owned(),
        ));
    }

    Ok(())
}

pub fn validate_ledger_json(bytes: &[u8]) -> Result<OpsEvidenceReport, LedgerError> {
    let ledger = parse_ledger_json(bytes)?;
    validate_ledger(&ledger)?;
    let mut final_asset_states = BTreeMap::new();
    for asset in &ledger.assets {
        final_asset_states.insert(asset.asset_ref.clone(), asset.initial_state.clone());
    }
    for event in &ledger.events {
        final_asset_states.insert(event.asset_ref.clone(), event.to_state.clone());
    }
    let right_censored_exposure_seconds = ledger.exposure_records.iter()
        .filter(|record| record.censoring_status == "right_censored")
        .map(|record| record.exposure_seconds)
        .sum();
    let interval_censored_record_count = ledger.exposure_records.iter()
        .filter(|record| record.censoring_status == "interval_censored")
        .count();
    Ok(OpsEvidenceReport {
        ledger_id: ledger.ledger_id,
        evidence_class: ledger.evidence_class,
        event_count: ledger.events.len(),
        exposure_record_count: ledger.exposure_records.len(),
        final_asset_states,
        right_censored_exposure_seconds,
        interval_censored_record_count,
        return_to_service_authorized: ledger.authorization_boundary.return_to_service_authorized,
        release_status: ledger.authorization_boundary.release_status,
        claim_ceiling: ledger.claim_ceiling,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!(
        "../../../docs/research/aviation/examples/synthetic_haps_maintenance_lifecycle_v1.json"
    );

    fn fixture_value() -> Value {
        serde_json::from_str(FIXTURE).expect("fixture JSON parses")
    }

    #[test]
    fn synthetic_lifecycle_chain_validates_without_return_to_service_release() {
        let report = validate_ledger_json(FIXTURE.as_bytes()).expect("ledger validates");
        assert_eq!(report.event_count, 12);
        assert_eq!(report.exposure_record_count, 2);
        assert_eq!(report.final_asset_states["haps-01"], "awaiting_authorized_release");
        assert!(!report.return_to_service_authorized);
        assert_eq!(report.release_status, "awaiting_authorized_release");
        assert_eq!(report.interval_censored_record_count, 1);
        assert_eq!(report.right_censored_exposure_seconds, 432_000.0);
    }

    #[test]
    fn broken_event_chain_is_rejected() {
        let mut value = fixture_value();
        value["events"][2]["previous_event_id"] = Value::Null;
        let bytes = serde_json::to_vec(&value).expect("serialize");
        let error = validate_ledger_json(&bytes).expect_err("broken chain must fail");
        assert!(error.to_string().contains("previous_event_id"));
    }

    #[test]
    fn recorded_before_observed_is_rejected() {
        let mut value = fixture_value();
        value["events"][3]["recorded_at_utc"] = Value::String("2026-05-01T12:59:00Z".to_owned());
        let bytes = serde_json::to_vec(&value).expect("serialize");
        let error = validate_ledger_json(&bytes).expect_err("clock inversion must fail");
        assert!(error.to_string().contains("recorded before it was observed"));
    }

    #[test]
    fn interval_censoring_requires_both_bounds() {
        let mut value = fixture_value();
        value["events"][3]["onset_interval_latest_utc"] = Value::Null;
        let bytes = serde_json::to_vec(&value).expect("serialize");
        let error = validate_ledger_json(&bytes).expect_err("missing onset bound must fail");
        assert!(error.to_string().contains("upper bound"));
    }

    #[test]
    fn right_censoring_cannot_claim_an_observed_failure() {
        let mut value = fixture_value();
        value["exposure_records"][1]["failure_observation"] =
            Value::String("failure_or_defect_detected".to_owned());
        let bytes = serde_json::to_vec(&value).expect("serialize");
        let error = validate_ledger_json(&bytes).expect_err("right-censor failure mismatch must fail");
        assert!(error.to_string().contains("right-censored exposure"));
    }

    #[test]
    fn return_to_service_requires_external_authority_receipt() {
        let mut value = fixture_value();
        value["authorization_boundary"]["return_to_service_authorized"] = Value::Bool(true);
        value["authorization_boundary"]["release_status"] =
            Value::String("authorized_by_external_authority".to_owned());
        value["authorization_boundary"]["release_scope"] =
            Value::String("specific_configuration_and_scope".to_owned());
        let bytes = serde_json::to_vec(&value).expect("serialize");
        let error = validate_ledger_json(&bytes).expect_err("authorization without authority receipt must fail");
        assert!(error.to_string().contains("external authority record"));
    }

    #[test]
    fn state_transition_must_match_current_asset_state() {
        let mut value = fixture_value();
        value["events"][5]["from_state"] = Value::String("grounded".to_owned());
        let bytes = serde_json::to_vec(&value).expect("serialize");
        let error = validate_ledger_json(&bytes).expect_err("state mismatch must fail");
        assert!(error.to_string().contains("from_state"));
    }

    #[test]
    fn right_censored_exposure_duration_is_checked() {
        let mut value = fixture_value();
        value["exposure_records"][1]["exposure_seconds"] = Value::from(430_000.0);
        let bytes = serde_json::to_vec(&value).expect("serialize");
        let error = validate_ledger_json(&bytes).expect_err("bad exposure duration must fail");
        assert!(error.to_string().contains("duration does not match"));
    }
}
