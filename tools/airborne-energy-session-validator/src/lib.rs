// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Independent verifier for synthetic aerial energy-session traces.
//!
//! This crate validates evidence and event semantics. It is not a flight
//! controller, transmitter interlock, physical safety case, or authorization
//! mechanism. Its synthetic permit enum must never be wired to hardware.

use serde_json::{Map, Value};
use std::collections::BTreeSet;

const TOP_LEVEL_KEYS: &[&str] = &[
    "schema_version",
    "episode_id",
    "record_state",
    "evidence_class",
    "receiver",
    "opportunity",
    "site_allocation_receipt",
    "events",
    "energy_accounting",
    "claim_ceiling",
];

const EVENT_KEYS: &[&str] = &[
    "event_id",
    "sequence",
    "at_ms",
    "from_state",
    "to_state",
    "transition_reason",
    "evidence",
];

const EVIDENCE_KEYS: &[&str] = &[
    "motion_state",
    "receiver_identity_state",
    "authorization_state",
    "allocation_state",
    "envelope_state",
    "position_state",
    "tracking_state",
    "local_protection_state",
    "deenergized_state",
];

const ENERGY_KEYS: &[&str] = &[
    "source_boundary_id",
    "channel_input_boundary_id",
    "receiver_dc_boundary_id",
    "useful_energy_boundary_id",
    "source_energy_wh",
    "channel_input_energy_wh",
    "receiver_dc_energy_wh",
    "battery_terminal_useful_energy_wh",
    "source_to_channel_loss_wh",
    "channel_to_receiver_loss_wh",
    "receiver_to_battery_loss_wh",
    "residual_wh",
    "accounting_evidence",
];

const STATES: &[&str] = &[
    "UNAVAILABLE",
    "DISCOVERED",
    "ADMISSION_PENDING",
    "ACQUISITION_PENDING",
    "TRACKED_NO_POWER",
    "POWER_ACTIVE",
    "DEPARTURE_PENDING",
    "SAFE_OFF",
    "REACQUISITION_PENDING",
    "COMPLETE",
    "ABORTED",
    "RECOVERY_REQUIRED",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationReport {
    pub episode_id: String,
    pub final_state: String,
    pub event_count: usize,
    pub power_active_intervals: usize,
    pub source_energy_wh: u64,
    pub useful_battery_energy_wh: u64,
    pub total_declared_loss_wh: u64,
}

fn fail(path: &str, reason: &str) -> String {
    format!("{path}: {reason}")
}

fn object<'a>(value: &'a Value, path: &str) -> Result<&'a Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| fail(path, "must be an object"))
}

fn array<'a>(value: &'a Value, path: &str) -> Result<&'a Vec<Value>, String> {
    value
        .as_array()
        .ok_or_else(|| fail(path, "must be an array"))
}

fn exact_keys(value: &Value, expected: &[&str], path: &str) -> Result<(), String> {
    let map = object(value, path)?;
    let expected_set: BTreeSet<&str> = expected.iter().copied().collect();
    for key in expected {
        if !map.contains_key(*key) {
            return Err(fail(path, &format!("missing property {key}")));
        }
    }
    if let Some(key) = map.keys().find(|key| !expected_set.contains(key.as_str())) {
        return Err(fail(path, &format!("unknown property {key}")));
    }
    Ok(())
}

fn field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a Value, String> {
    object(value, path)?
        .get(key)
        .ok_or_else(|| fail(path, &format!("missing property {key}")))
}

fn string<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a str, String> {
    field(value, key, path)?
        .as_str()
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| fail(path, &format!("{key} must be a non-empty string")))
}

fn unsigned(value: &Value, key: &str, path: &str) -> Result<u64, String> {
    field(value, key, path)?
        .as_u64()
        .ok_or_else(|| fail(path, &format!("{key} must be a non-negative integer")))
}

fn enum_string<'a>(
    value: &'a Value,
    key: &str,
    choices: &[&str],
    path: &str,
) -> Result<&'a str, String> {
    let actual = string(value, key, path)?;
    if choices.contains(&actual) {
        Ok(actual)
    } else {
        Err(fail(
            path,
            &format!("{key} has unsupported value {actual:?}"),
        ))
    }
}

fn allowed_transition(from: &str, to: &str) -> bool {
    match from {
        "UNAVAILABLE" => matches!(to, "DISCOVERED" | "ABORTED"),
        "DISCOVERED" => matches!(to, "ADMISSION_PENDING" | "UNAVAILABLE" | "ABORTED"),
        "ADMISSION_PENDING" => {
            matches!(to, "ACQUISITION_PENDING" | "ABORTED" | "RECOVERY_REQUIRED")
        }
        "ACQUISITION_PENDING" => {
            matches!(to, "TRACKED_NO_POWER" | "ABORTED" | "RECOVERY_REQUIRED")
        }
        "TRACKED_NO_POWER" => matches!(
            to,
            "POWER_ACTIVE" | "DEPARTURE_PENDING" | "ABORTED" | "RECOVERY_REQUIRED"
        ),
        "POWER_ACTIVE" => matches!(to, "DEPARTURE_PENDING" | "RECOVERY_REQUIRED"),
        "DEPARTURE_PENDING" => matches!(to, "SAFE_OFF" | "RECOVERY_REQUIRED"),
        "SAFE_OFF" => matches!(
            to,
            "REACQUISITION_PENDING" | "COMPLETE" | "ABORTED" | "RECOVERY_REQUIRED"
        ),
        "REACQUISITION_PENDING" => {
            matches!(to, "ADMISSION_PENDING" | "ABORTED" | "RECOVERY_REQUIRED")
        }
        "RECOVERY_REQUIRED" => matches!(to, "SAFE_OFF" | "ABORTED"),
        "COMPLETE" | "ABORTED" => false,
        _ => false,
    }
}

fn validate_receiver(value: &Value) -> Result<(), String> {
    let path = "$.receiver";
    exact_keys(
        value,
        &["receiver_id", "receiver_class", "trajectory_sha256"],
        path,
    )?;
    string(value, "receiver_id", path)?;
    enum_string(
        value,
        "receiver_class",
        &[
            "UAV_DOCKED",
            "UAV_HOVERING",
            "UAV_MOVING",
            "UAV_FIXED_WING",
            "EVTOL_DOCKED",
            "EVTOL_AIRBORNE",
        ],
        path,
    )?;
    let digest = string(value, "trajectory_sha256", path)?;
    validate_sha256(digest, &format!("{path}.trajectory_sha256"))
}

fn validate_sha256(value: &str, path: &str) -> Result<(), String> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(fail(path, "must be lowercase hexadecimal SHA-256"));
    }
    Ok(())
}

fn expected_field_regime(bearer: &str) -> &'static str {
    match bearer {
        "CONDUCTIVE_CONTACT" | "ROBOTIC_DOCK" => "CONTACT",
        "IPT" | "CPT" => "NEAR_FIELD",
        "RF_MICROWAVE" | "OPTICAL_LASER" => "FAR_FIELD",
        "TETHERED" => "GUIDED",
        "BATTERY_EXCHANGE" => "ENERGY_CARRIER",
        _ => "INVALID",
    }
}

fn validate_opportunity(value: &Value) -> Result<(), String> {
    let path = "$.opportunity";
    exact_keys(
        value,
        &[
            "opportunity_id",
            "bearer",
            "field_regime",
            "channel_id",
            "envelope_sha256",
            "envelope_state",
        ],
        path,
    )?;
    string(value, "opportunity_id", path)?;
    let bearer = enum_string(
        value,
        "bearer",
        &[
            "CONDUCTIVE_CONTACT",
            "ROBOTIC_DOCK",
            "IPT",
            "CPT",
            "RF_MICROWAVE",
            "OPTICAL_LASER",
            "TETHERED",
            "BATTERY_EXCHANGE",
        ],
        path,
    )?;
    let regime = enum_string(
        value,
        "field_regime",
        &[
            "CONTACT",
            "NEAR_FIELD",
            "FAR_FIELD",
            "GUIDED",
            "ENERGY_CARRIER",
        ],
        path,
    )?;
    if regime != expected_field_regime(bearer) {
        return Err(fail(
            path,
            "field_regime does not match the declared bearer",
        ));
    }
    string(value, "channel_id", path)?;
    validate_sha256(
        string(value, "envelope_sha256", path)?,
        &format!("{path}.envelope_sha256"),
    )?;
    enum_string(
        value,
        "envelope_state",
        &[
            "ADMITTED_SYNTHETIC",
            "NOT_ADMITTED",
            "UNKNOWN",
            "STALE",
            "OUT_OF_DOMAIN",
        ],
        path,
    )?;
    Ok(())
}

fn validate_allocation(value: &Value) -> Result<(u64, u64), String> {
    let path = "$.site_allocation_receipt";
    exact_keys(
        value,
        &[
            "site_id",
            "allocation_id",
            "allocation_sha256",
            "valid_from_ms",
            "valid_until_ms",
        ],
        path,
    )?;
    string(value, "site_id", path)?;
    string(value, "allocation_id", path)?;
    validate_sha256(
        string(value, "allocation_sha256", path)?,
        &format!("{path}.allocation_sha256"),
    )?;
    let valid_from = unsigned(value, "valid_from_ms", path)?;
    let valid_until = unsigned(value, "valid_until_ms", path)?;
    if valid_from >= valid_until {
        return Err(fail(
            path,
            "valid_from_ms must be earlier than valid_until_ms",
        ));
    }
    Ok((valid_from, valid_until))
}

fn validate_evidence(value: &Value, path: &str) -> Result<(), String> {
    exact_keys(value, EVIDENCE_KEYS, path)?;
    enum_string(
        value,
        "motion_state",
        &[
            "PARKED_LANDED_DOCKED",
            "QUASI_STATIC_HOVER",
            "MOVING_HOVER",
            "TRANSLATION",
            "CLIMB",
            "DESCENT",
            "DYNAMIC_FLIGHT",
            "DIVERSION",
        ],
        path,
    )?;
    enum_string(
        value,
        "receiver_identity_state",
        &["MATCHED_SYNTHETIC", "CONFLICT", "UNKNOWN"],
        path,
    )?;
    enum_string(
        value,
        "authorization_state",
        &["VALID_SYNTHETIC", "REVOKED", "UNKNOWN"],
        path,
    )?;
    enum_string(
        value,
        "allocation_state",
        &["VALID_SYNTHETIC", "REVOKED", "UNKNOWN", "EXPIRED"],
        path,
    )?;
    enum_string(
        value,
        "envelope_state",
        &[
            "ADMITTED_SYNTHETIC",
            "NOT_ADMITTED",
            "UNKNOWN",
            "STALE",
            "OUT_OF_DOMAIN",
        ],
        path,
    )?;
    enum_string(
        value,
        "position_state",
        &["IN_DOMAIN_SYNTHETIC", "OUT_OF_DOMAIN", "STALE", "UNKNOWN"],
        path,
    )?;
    enum_string(
        value,
        "tracking_state",
        &["TRACKED_SYNTHETIC", "LOST", "NOT_REQUIRED", "UNKNOWN"],
        path,
    )?;
    enum_string(
        value,
        "local_protection_state",
        &["PERMIT_SYNTHETIC_ONLY", "INHIBIT", "TRIPPED", "UNKNOWN"],
        path,
    )?;
    enum_string(
        value,
        "deenergized_state",
        &[
            "CONFIRMED_SYNTHETIC",
            "UNCONFIRMED",
            "NOT_REQUIRED",
            "UNKNOWN",
        ],
        path,
    )?;
    Ok(())
}

fn require_admission(value: &Value, path: &str) -> Result<(), String> {
    let required = [
        ("receiver_identity_state", "MATCHED_SYNTHETIC"),
        ("authorization_state", "VALID_SYNTHETIC"),
        ("allocation_state", "VALID_SYNTHETIC"),
        ("envelope_state", "ADMITTED_SYNTHETIC"),
        ("position_state", "IN_DOMAIN_SYNTHETIC"),
    ];
    for (key, expected) in required {
        if string(value, key, path)? != expected {
            return Err(fail(path, &format!("admission requires {key}={expected}")));
        }
    }
    Ok(())
}

fn validate_energy(value: &Value) -> Result<(u64, u64, u64), String> {
    let path = "$.energy_accounting";
    exact_keys(value, ENERGY_KEYS, path)?;
    if string(value, "source_boundary_id", path)? != "SITE_PCC_GRID_AC_INPUT" {
        return Err(fail(path, "source boundary must be SITE_PCC_GRID_AC_INPUT"));
    }
    string(value, "channel_input_boundary_id", path)?;
    string(value, "receiver_dc_boundary_id", path)?;
    if string(value, "useful_energy_boundary_id", path)? != "AIRCRAFT_BATTERY_TERMINAL_INPUT" {
        return Err(fail(
            path,
            "useful-energy boundary must be AIRCRAFT_BATTERY_TERMINAL_INPUT",
        ));
    }
    if string(value, "accounting_evidence", path)? != "SYNTHETIC_ARITHMETIC_ONLY" {
        return Err(fail(
            path,
            "v0 only admits SYNTHETIC_ARITHMETIC_ONLY energy arithmetic",
        ));
    }

    let source = unsigned(value, "source_energy_wh", path)?;
    let channel = unsigned(value, "channel_input_energy_wh", path)?;
    let receiver = unsigned(value, "receiver_dc_energy_wh", path)?;
    let useful = unsigned(value, "battery_terminal_useful_energy_wh", path)?;
    let loss_a = unsigned(value, "source_to_channel_loss_wh", path)?;
    let loss_b = unsigned(value, "channel_to_receiver_loss_wh", path)?;
    let loss_c = unsigned(value, "receiver_to_battery_loss_wh", path)?;
    let residual = unsigned(value, "residual_wh", path)?;

    if source != loss_a.saturating_add(channel) {
        return Err(fail(
            path,
            "source energy must equal source-to-channel loss plus channel input",
        ));
    }
    if channel != loss_b.saturating_add(receiver) {
        return Err(fail(
            path,
            "channel input must equal channel-to-receiver loss plus receiver DC energy",
        ));
    }
    if receiver != loss_c.saturating_add(useful) {
        return Err(fail(
            path,
            "receiver DC energy must equal receiver-to-battery loss plus battery-terminal useful energy",
        ));
    }
    if residual != 0 {
        return Err(fail(
            path,
            "v0 integer-Wh ledger requires zero unexplained residual; model uncertainty separately",
        ));
    }

    let total_loss = loss_a
        .checked_add(loss_b)
        .and_then(|sum| sum.checked_add(loss_c))
        .ok_or_else(|| fail(path, "loss total overflow"))?;
    Ok((source, useful, total_loss))
}

fn validate_not_run(value: &Value) -> Result<ValidationReport, String> {
    if string(value, "evidence_class", "$")? != "NOT_RUN" {
        return Err(fail("$", "NOT_RUN requires evidence_class=NOT_RUN"));
    }
    if string(value, "episode_id", "$")? != "NOT_RUN" {
        return Err(fail("$", "NOT_RUN template episode_id must be NOT_RUN"));
    }
    for key in [
        "receiver",
        "opportunity",
        "site_allocation_receipt",
        "energy_accounting",
    ] {
        if !field(value, key, "$")?.is_null() {
            return Err(fail("$", &format!("NOT_RUN requires {key}=null")));
        }
    }
    if !array(field(value, "events", "$")?, "$.events")?.is_empty() {
        return Err(fail("$.events", "NOT_RUN requires an empty event array"));
    }
    Ok(ValidationReport {
        episode_id: "NOT_RUN".to_string(),
        final_state: "NOT_RUN".to_string(),
        event_count: 0,
        power_active_intervals: 0,
        source_energy_wh: 0,
        useful_battery_energy_wh: 0,
        total_declared_loss_wh: 0,
    })
}

/// Validate a parsed session document. Invalid evidence is rejected as a whole.
pub fn validate_episode(value: &Value) -> Result<ValidationReport, String> {
    exact_keys(value, TOP_LEVEL_KEYS, "$")?;
    if string(value, "schema_version", "$")? != "airborne-energy-session-v0" {
        return Err(fail("$", "unsupported schema_version"));
    }
    enum_string(value, "record_state", &["NOT_RUN", "FIXTURE"], "$")?;
    string(value, "claim_ceiling", "$")?;

    if string(value, "record_state", "$")? == "NOT_RUN" {
        return validate_not_run(value);
    }
    if string(value, "evidence_class", "$")? != "SYNTHETIC_KNOWN_TRUTH" {
        return Err(fail(
            "$",
            "FIXTURE requires evidence_class=SYNTHETIC_KNOWN_TRUTH",
        ));
    }

    let episode_id = string(value, "episode_id", "$")?.to_string();
    let receiver = field(value, "receiver", "$")?;
    let opportunity = field(value, "opportunity", "$")?;
    let allocation = field(value, "site_allocation_receipt", "$")?;
    if receiver.is_null() || opportunity.is_null() || allocation.is_null() {
        return Err(fail(
            "$",
            "FIXTURE requires receiver, opportunity and site allocation evidence",
        ));
    }
    validate_receiver(receiver)?;
    validate_opportunity(opportunity)?;
    let (valid_from, valid_until) = validate_allocation(allocation)?;
    if string(opportunity, "envelope_state", "$.opportunity")? != "ADMITTED_SYNTHETIC" {
        return Err(fail(
            "$.opportunity",
            "FIXTURE opportunity must have an admitted synthetic envelope",
        ));
    }

    let events = array(field(value, "events", "$")?, "$.events")?;
    if events.is_empty() {
        return Err(fail("$.events", "FIXTURE requires at least one event"));
    }

    let mut current_state = "UNAVAILABLE";
    let mut previous_time = None;
    let mut event_ids = BTreeSet::new();
    let mut power_active_intervals = 0usize;
    let mut admission_ready = false;
    let mut tracking_ready = false;

    for (index, event) in events.iter().enumerate() {
        let path = format!("$.events[{index}]");
        exact_keys(event, EVENT_KEYS, &path)?;
        let event_id = string(event, "event_id", &path)?;
        if !event_ids.insert(event_id.to_string()) {
            return Err(fail(&path, "duplicate event_id"));
        }
        let sequence = unsigned(event, "sequence", &path)?;
        if sequence != index as u64 {
            return Err(fail(
                &path,
                "sequence must be unique, contiguous and start at zero",
            ));
        }
        let at_ms = unsigned(event, "at_ms", &path)?;
        if previous_time.is_some_and(|last| at_ms < last) {
            return Err(fail(&path, "event timestamps must be monotonic"));
        }
        previous_time = Some(at_ms);
        let from_state = enum_string(event, "from_state", STATES, &path)?;
        let to_state = enum_string(event, "to_state", STATES, &path)?;
        string(event, "transition_reason", &path)?;
        if from_state != current_state {
            return Err(fail(
                &path,
                &format!("state chain expected from_state={current_state}, got {from_state}"),
            ));
        }
        if !allowed_transition(from_state, to_state) {
            return Err(fail(
                &path,
                &format!("illegal transition {from_state}->{to_state}"),
            ));
        }

        let evidence = field(event, "evidence", &path)?;
        let evidence_path = format!("{path}.evidence");
        validate_evidence(evidence, &evidence_path)?;

        if to_state == "ACQUISITION_PENDING" && from_state == "DISCOVERED" {
            admission_ready = false;
            tracking_ready = false;
        }
        if to_state == "ACQUISITION_PENDING" && from_state == "ADMISSION_PENDING" {
            require_admission(evidence, &evidence_path)?;
            if string(opportunity, "envelope_state", "$.opportunity")? != "ADMITTED_SYNTHETIC" {
                return Err(fail(&path, "opportunity envelope is not admitted"));
            }
            if at_ms < valid_from || at_ms >= valid_until {
                return Err(fail(
                    &path,
                    "site allocation receipt is not valid at admission time",
                ));
            }
            admission_ready = true;
            tracking_ready = false;
        }
        if to_state == "TRACKED_NO_POWER" {
            if !admission_ready {
                return Err(fail(
                    &path,
                    "tracking cannot be admitted without fresh session admission",
                ));
            }
            if string(evidence, "tracking_state", &evidence_path)? != "TRACKED_SYNTHETIC" {
                return Err(fail(
                    &path,
                    "TRACKED_NO_POWER requires TRACKED_SYNTHETIC evidence",
                ));
            }
            tracking_ready = true;
        }

        if to_state == "POWER_ACTIVE" {
            require_admission(evidence, &evidence_path)?;
            if !admission_ready || !tracking_ready || from_state != "TRACKED_NO_POWER" {
                return Err(fail(
                    &path,
                    "POWER_ACTIVE requires fresh admission and a tracked no-power state",
                ));
            }
            if string(evidence, "tracking_state", &evidence_path)? != "TRACKED_SYNTHETIC" {
                return Err(fail(&path, "POWER_ACTIVE requires live synthetic tracking"));
            }
            if string(evidence, "local_protection_state", &evidence_path)?
                != "PERMIT_SYNTHETIC_ONLY"
            {
                return Err(fail(
                    &path,
                    "POWER_ACTIVE requires PERMIT_SYNTHETIC_ONLY; this is not a hardware permit",
                ));
            }
            if at_ms < valid_from || at_ms >= valid_until {
                return Err(fail(
                    &path,
                    "site allocation receipt expired before energization",
                ));
            }
            power_active_intervals += 1;
            tracking_ready = false;
        }

        if from_state == "POWER_ACTIVE"
            && matches!(to_state, "DEPARTURE_PENDING" | "RECOVERY_REQUIRED")
            && string(evidence, "local_protection_state", &evidence_path)? != "INHIBIT"
        {
            return Err(fail(
                &path,
                "leaving POWER_ACTIVE requires local protection INHIBIT",
            ));
        }
        if to_state == "SAFE_OFF" {
            if !matches!(from_state, "DEPARTURE_PENDING" | "RECOVERY_REQUIRED") {
                return Err(fail(&path, "SAFE_OFF requires departure or recovery state"));
            }
            if string(evidence, "deenergized_state", &evidence_path)? != "CONFIRMED_SYNTHETIC" {
                return Err(fail(
                    &path,
                    "SAFE_OFF requires independent CONFIRMED_SYNTHETIC de-energization evidence",
                ));
            }
            if string(evidence, "local_protection_state", &evidence_path)? != "INHIBIT" {
                return Err(fail(&path, "SAFE_OFF requires local protection INHIBIT"));
            }
            admission_ready = false;
            tracking_ready = false;
        }

        if to_state == "REACQUISITION_PENDING" {
            if !matches!(from_state, "SAFE_OFF") {
                return Err(fail(
                    &path,
                    "reacquisition requires a verified SAFE_OFF state",
                ));
            }
            admission_ready = false;
            tracking_ready = false;
        }
        if to_state == "COMPLETE" && from_state != "SAFE_OFF" {
            return Err(fail(&path, "COMPLETE requires a verified SAFE_OFF state"));
        }

        current_state = to_state;
    }

    if !matches!(current_state, "COMPLETE" | "ABORTED") {
        return Err(fail(
            "$.events",
            &format!("session trace is incomplete; terminal state is {current_state}"),
        ));
    }
    let (source_energy_wh, useful_battery_energy_wh, total_declared_loss_wh) =
        validate_energy(field(value, "energy_accounting", "$")?)?;

    Ok(ValidationReport {
        episode_id,
        final_state: current_state.to_string(),
        event_count: events.len(),
        power_active_intervals,
        source_energy_wh,
        useful_battery_energy_wh,
        total_declared_loss_wh,
    })
}

/// Parse and validate a UTF-8 JSON session document.
pub fn validate_json(input: &str) -> Result<ValidationReport, String> {
    let value: Value =
        serde_json::from_str(input).map_err(|error| format!("invalid JSON: {error}"))?;
    validate_episode(&value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const POSITIVE: &str = include_str!(
        "../../../research/airborne-energy-session-v0/session.synthetic-reacquire.example.json"
    );
    const HOSTILE: &str = include_str!(
        "../../../research/airborne-energy-session-v0/session.adversarial-revoked-authorization.example.json"
    );
    const NOT_RUN: &str =
        include_str!("../../../research/airborne-energy-session-v0/session.NOT_RUN.template.json");

    fn positive_value() -> Value {
        serde_json::from_str(POSITIVE).expect("positive fixture must be valid JSON")
    }

    #[test]
    fn positive_reacquisition_trace_closes_and_reports_two_power_intervals() {
        let report = validate_json(POSITIVE).expect("synthetic known-truth trace must pass");
        assert_eq!(report.final_state, "COMPLETE");
        assert_eq!(report.event_count, 15);
        assert_eq!(report.power_active_intervals, 2);
        assert_eq!(report.source_energy_wh, 40);
        assert_eq!(report.useful_battery_energy_wh, 24);
        assert_eq!(report.total_declared_loss_wh, 16);
    }

    #[test]
    fn not_run_template_is_not_a_pass_or_physical_result() {
        let report = validate_json(NOT_RUN).expect("NOT_RUN template is structurally valid");
        assert_eq!(report.final_state, "NOT_RUN");
        assert_eq!(report.event_count, 0);
        assert_eq!(report.source_energy_wh, 0);
        assert_eq!(report.useful_battery_energy_wh, 0);
    }

    #[test]
    fn revoked_authorization_negative_control_is_rejected() {
        let error = validate_json(HOSTILE).expect_err("revoked authorization must fail");
        assert!(error.contains("admission requires authorization_state"));
    }

    #[test]
    fn lost_tracking_cannot_enter_power_active() {
        let mut value = positive_value();
        value["events"][4]["evidence"]["tracking_state"] = json!("LOST");
        let error = validate_episode(&value).expect_err("lost tracking must fail");
        assert!(error.contains("tracking_state"));
    }

    #[test]
    fn protection_inhibit_cannot_be_promoted_to_power_permit() {
        let mut value = positive_value();
        value["events"][4]["evidence"]["local_protection_state"] = json!("INHIBIT");
        let error = validate_episode(&value).expect_err("inhibited protection must fail");
        assert!(error.contains("local protection"));
    }

    #[test]
    fn stale_opportunity_envelope_is_rejected() {
        let mut value = positive_value();
        value["opportunity"]["envelope_state"] = json!("STALE");
        let error = validate_episode(&value).expect_err("stale envelope must fail");
        assert!(error.contains("admitted synthetic envelope"));
    }

    #[test]
    fn expired_allocation_cannot_admit_or_energize() {
        let mut value = positive_value();
        value["site_allocation_receipt"]["valid_until_ms"] = json!(150);
        let error = validate_episode(&value).expect_err("expired allocation must fail");
        assert!(error.contains("not valid at admission time"));
    }

    #[test]
    fn out_of_domain_position_cannot_complete_admission() {
        let mut value = positive_value();
        value["events"][2]["evidence"]["position_state"] = json!("OUT_OF_DOMAIN");
        let error = validate_episode(&value).expect_err("out-of-domain pose must fail");
        assert!(error.contains("position_state"));
    }

    #[test]
    fn deenergization_must_be_confirmed_before_safe_off() {
        let mut value = positive_value();
        value["events"][6]["evidence"]["deenergized_state"] = json!("UNCONFIRMED");
        let error = validate_episode(&value).expect_err("unconfirmed deenergization must fail");
        assert!(error.contains("CONFIRMED_SYNTHETIC"));
    }

    #[test]
    fn departure_requires_local_inhibit() {
        let mut value = positive_value();
        value["events"][5]["evidence"]["local_protection_state"] = json!("UNKNOWN");
        let error = validate_episode(&value).expect_err("departure must inhibit local power");
        assert!(error.contains("requires local protection INHIBIT"));
    }

    #[test]
    fn duplicate_event_ids_are_rejected() {
        let mut value = positive_value();
        value["events"][1]["event_id"] = value["events"][0]["event_id"].clone();
        let error = validate_episode(&value).expect_err("duplicate IDs must fail");
        assert!(error.contains("duplicate event_id"));
    }

    #[test]
    fn nonmonotonic_event_time_is_rejected() {
        let mut value = positive_value();
        value["events"][3]["at_ms"] = json!(150);
        let error = validate_episode(&value).expect_err("reordered time must fail");
        assert!(error.contains("timestamps must be monotonic"));
    }

    #[test]
    fn noncontiguous_sequence_is_rejected() {
        let mut value = positive_value();
        value["events"][3]["sequence"] = json!(8);
        let error = validate_episode(&value).expect_err("sequence gaps must fail");
        assert!(error.contains("contiguous"));
    }

    #[test]
    fn state_chain_cannot_skip_reacquisition_admission() {
        let mut value = positive_value();
        value["events"][9]["from_state"] = json!("REACQUISITION_PENDING");
        let error = validate_episode(&value).expect_err("state-chain discontinuity must fail");
        assert!(error.contains("state chain expected"));
    }

    #[test]
    fn a_session_cannot_end_while_power_is_active() {
        let mut value = positive_value();
        value["events"]
            .as_array_mut()
            .expect("events array")
            .truncate(5);
        let error = validate_episode(&value).expect_err("active session is incomplete");
        assert!(error.contains("session trace is incomplete"));
    }

    #[test]
    fn source_to_channel_energy_mismatch_is_rejected() {
        let mut value = positive_value();
        value["energy_accounting"]["source_to_channel_loss_wh"] = json!(5);
        let error = validate_episode(&value).expect_err("source loss mismatch must fail");
        assert!(error.contains("source energy must equal"));
    }

    #[test]
    fn receiver_to_battery_mismatch_is_rejected() {
        let mut value = positive_value();
        value["energy_accounting"]["battery_terminal_useful_energy_wh"] = json!(23);
        let error = validate_episode(&value).expect_err("battery boundary mismatch must fail");
        assert!(error.contains("receiver DC energy must equal"));
    }

    #[test]
    fn unexplained_residual_is_rejected() {
        let mut value = positive_value();
        value["energy_accounting"]["residual_wh"] = json!(1);
        let error = validate_episode(&value).expect_err("unexplained energy must fail");
        assert!(error.contains("zero unexplained residual"));
    }

    #[test]
    fn bearer_and_field_regime_must_agree() {
        let mut value = positive_value();
        value["opportunity"]["field_regime"] = json!("NEAR_FIELD");
        let error = validate_episode(&value).expect_err("optical is far-field");
        assert!(error.contains("field_regime does not match"));
    }

    #[test]
    fn unknown_fields_fail_closed() {
        let mut value = positive_value();
        value["unreviewed_override"] = json!(true);
        let error = validate_episode(&value).expect_err("unknown fields must fail");
        assert!(error.contains("unknown property"));
    }

    #[test]
    fn same_frozen_input_produces_same_report() {
        let value = positive_value();
        let first = validate_episode(&value).expect("first pass");
        let second = validate_episode(&value).expect("second pass");
        assert_eq!(first, second);
    }
}
