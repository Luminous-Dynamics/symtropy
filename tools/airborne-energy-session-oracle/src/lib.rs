// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

//! Deterministic semantic verifier for the aerial energy-session v0 contract.
//!
//! The JSON Schema defines the closed record shape. This crate independently
//! validates event ordering, admission gates, safe-off transitions, fresh
//! reacquisition and the source-to-battery arithmetic. It accepts synthetic
//! fixtures only; it is not a physical interlock or flight controller.

use std::collections::BTreeSet;
use std::fmt;

use serde_json::Value;

const POSITIVE_FIXTURE: &str = include_str!(
    "../../../research/airborne-energy-session-v0/session.synthetic-reacquire.example.json"
);
const REVOKED_FIXTURE: &str = include_str!(
    "../../../research/airborne-energy-session-v0/session.adversarial-revoked-authorization.example.json"
);
const NOT_RUN_FIXTURE: &str =
    include_str!("../../../research/airborne-energy-session-v0/session.NOT_RUN.template.json");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalError(pub String);

impl fmt::Display for EvalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for EvalError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    NotRun,
    PassSynthetic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub disposition: Disposition,
    pub episode_id: String,
    pub final_state: String,
    pub event_count: u64,
    pub power_active_intervals: u64,
    pub source_energy_wh: u64,
    pub battery_terminal_useful_energy_wh: u64,
    pub residual_wh: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Unavailable,
    Discovered,
    AdmissionPending,
    AcquisitionPending,
    TrackedNoPower,
    PowerActive,
    DeparturePending,
    SafeOff,
    ReacquisitionPending,
    Complete,
    Aborted,
    RecoveryRequired,
}

impl State {
    fn parse(value: &str, path: &str) -> Result<Self, EvalError> {
        match value {
            "UNAVAILABLE" => Ok(Self::Unavailable),
            "DISCOVERED" => Ok(Self::Discovered),
            "ADMISSION_PENDING" => Ok(Self::AdmissionPending),
            "ACQUISITION_PENDING" => Ok(Self::AcquisitionPending),
            "TRACKED_NO_POWER" => Ok(Self::TrackedNoPower),
            "POWER_ACTIVE" => Ok(Self::PowerActive),
            "DEPARTURE_PENDING" => Ok(Self::DeparturePending),
            "SAFE_OFF" => Ok(Self::SafeOff),
            "REACQUISITION_PENDING" => Ok(Self::ReacquisitionPending),
            "COMPLETE" => Ok(Self::Complete),
            "ABORTED" => Ok(Self::Aborted),
            "RECOVERY_REQUIRED" => Ok(Self::RecoveryRequired),
            _ => Err(fail(path, "unknown session state")),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Unavailable => "UNAVAILABLE",
            Self::Discovered => "DISCOVERED",
            Self::AdmissionPending => "ADMISSION_PENDING",
            Self::AcquisitionPending => "ACQUISITION_PENDING",
            Self::TrackedNoPower => "TRACKED_NO_POWER",
            Self::PowerActive => "POWER_ACTIVE",
            Self::DeparturePending => "DEPARTURE_PENDING",
            Self::SafeOff => "SAFE_OFF",
            Self::ReacquisitionPending => "REACQUISITION_PENDING",
            Self::Complete => "COMPLETE",
            Self::Aborted => "ABORTED",
            Self::RecoveryRequired => "RECOVERY_REQUIRED",
        }
    }
}

fn fail(path: &str, message: &str) -> EvalError {
    EvalError(format!("{path}: {message}"))
}

fn object(value: &Value, path: &str) -> Result<(), EvalError> {
    if value.is_object() {
        Ok(())
    } else {
        Err(fail(path, "must be an object"))
    }
}

fn exact_keys(value: &Value, expected: &[&str], path: &str) -> Result<(), EvalError> {
    object(value, path)?;
    let fields = value
        .as_object()
        .ok_or_else(|| fail(path, "must be an object"))?;
    let expected_set: BTreeSet<&str> = expected.iter().copied().collect();
    if fields.len() != expected_set.len()
        || fields
            .keys()
            .any(|key| !expected_set.contains(key.as_str()))
    {
        return Err(fail(path, "missing or unknown object property"));
    }
    Ok(())
}

fn string_field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a str, EvalError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be a non-empty string"))
}

fn uint_field(value: &Value, key: &str, path: &str) -> Result<u64, EvalError> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be a non-negative integer"))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn require_sha256(value: &str, path: &str) -> Result<(), EvalError> {
    if is_sha256(value) {
        Ok(())
    } else {
        Err(fail(path, "must be lowercase hexadecimal SHA-256"))
    }
}

fn allowed_transition(from: State, to: State) -> bool {
    match from {
        State::Unavailable => matches!(to, State::Discovered | State::Aborted),
        State::Discovered => {
            matches!(
                to,
                State::AdmissionPending | State::Unavailable | State::Aborted
            )
        }
        State::AdmissionPending => {
            matches!(
                to,
                State::AcquisitionPending | State::Aborted | State::RecoveryRequired
            )
        }
        State::AcquisitionPending => {
            matches!(
                to,
                State::TrackedNoPower | State::Aborted | State::RecoveryRequired
            )
        }
        State::TrackedNoPower => matches!(
            to,
            State::PowerActive | State::DeparturePending | State::Aborted | State::RecoveryRequired
        ),
        State::PowerActive => matches!(to, State::DeparturePending | State::RecoveryRequired),
        State::DeparturePending => matches!(to, State::SafeOff | State::RecoveryRequired),
        State::SafeOff => matches!(
            to,
            State::ReacquisitionPending | State::Complete | State::RecoveryRequired
        ),
        State::ReacquisitionPending => {
            matches!(
                to,
                State::AdmissionPending | State::Aborted | State::RecoveryRequired
            )
        }
        State::Complete | State::Aborted | State::RecoveryRequired => false,
    }
}

fn require_evidence_string(
    evidence: &Value,
    key: &str,
    expected: &str,
    event_path: &str,
) -> Result<(), EvalError> {
    let actual = string_field(evidence, key, &format!("{event_path}.evidence"))?;
    if actual == expected {
        Ok(())
    } else {
        Err(fail(
            &format!("{event_path}.evidence.{key}"),
            &format!("expected {expected}, observed {actual}"),
        ))
    }
}

fn validate_admission(evidence: &Value, event_path: &str) -> Result<(), EvalError> {
    for (key, expected) in [
        ("receiver_identity_state", "MATCHED_SYNTHETIC"),
        ("authorization_state", "VALID_SYNTHETIC"),
        ("allocation_state", "VALID_SYNTHETIC"),
        ("envelope_state", "ADMITTED_SYNTHETIC"),
        ("position_state", "IN_DOMAIN_SYNTHETIC"),
    ] {
        require_evidence_string(evidence, key, expected, event_path)?;
    }
    Ok(())
}

fn validate_energy_ledger(ledger: &Value) -> Result<(u64, u64, u64), EvalError> {
    exact_keys(
        ledger,
        &[
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
        ],
        "$.energy_accounting",
    )?;

    if string_field(ledger, "source_boundary_id", "$.energy_accounting")?
        != "SITE_PCC_GRID_AC_INPUT"
    {
        return Err(fail(
            "$.energy_accounting.source_boundary_id",
            "must be SITE_PCC_GRID_AC_INPUT",
        ));
    }
    if string_field(ledger, "useful_energy_boundary_id", "$.energy_accounting")?
        != "AIRCRAFT_BATTERY_TERMINAL_INPUT"
    {
        return Err(fail(
            "$.energy_accounting.useful_energy_boundary_id",
            "must be AIRCRAFT_BATTERY_TERMINAL_INPUT",
        ));
    }
    if string_field(ledger, "accounting_evidence", "$.energy_accounting")?
        != "SYNTHETIC_ARITHMETIC_ONLY"
    {
        return Err(fail(
            "$.energy_accounting.accounting_evidence",
            "v0 accepts synthetic arithmetic only",
        ));
    }
    let _ = string_field(ledger, "channel_input_boundary_id", "$.energy_accounting")?;
    let _ = string_field(ledger, "receiver_dc_boundary_id", "$.energy_accounting")?;

    let source = uint_field(ledger, "source_energy_wh", "$.energy_accounting")?;
    let channel = uint_field(ledger, "channel_input_energy_wh", "$.energy_accounting")?;
    let receiver = uint_field(ledger, "receiver_dc_energy_wh", "$.energy_accounting")?;
    let useful = uint_field(
        ledger,
        "battery_terminal_useful_energy_wh",
        "$.energy_accounting",
    )?;
    let loss_source_channel =
        uint_field(ledger, "source_to_channel_loss_wh", "$.energy_accounting")?;
    let loss_channel_receiver =
        uint_field(ledger, "channel_to_receiver_loss_wh", "$.energy_accounting")?;
    let loss_receiver_battery =
        uint_field(ledger, "receiver_to_battery_loss_wh", "$.energy_accounting")?;
    let residual = uint_field(ledger, "residual_wh", "$.energy_accounting")?;

    let source_rhs = loss_source_channel
        .checked_add(channel)
        .ok_or_else(|| fail("$.energy_accounting", "source balance overflow"))?;
    let channel_rhs = loss_channel_receiver
        .checked_add(receiver)
        .ok_or_else(|| fail("$.energy_accounting", "channel balance overflow"))?;
    let receiver_rhs = loss_receiver_battery
        .checked_add(useful)
        .ok_or_else(|| fail("$.energy_accounting", "receiver balance overflow"))?;
    let aggregate_rhs = loss_source_channel
        .checked_add(loss_channel_receiver)
        .and_then(|sum| sum.checked_add(loss_receiver_battery))
        .and_then(|sum| sum.checked_add(useful))
        .and_then(|sum| sum.checked_add(residual))
        .ok_or_else(|| fail("$.energy_accounting", "aggregate balance overflow"))?;

    if source != source_rhs {
        return Err(fail(
            "$.energy_accounting",
            "source-to-channel energy does not close",
        ));
    }
    if channel != channel_rhs {
        return Err(fail(
            "$.energy_accounting",
            "channel-to-receiver energy does not close",
        ));
    }
    if receiver != receiver_rhs {
        return Err(fail(
            "$.energy_accounting",
            "receiver-to-battery energy does not close",
        ));
    }
    if source != aggregate_rhs {
        return Err(fail(
            "$.energy_accounting",
            "aggregate source-to-battery ledger does not close",
        ));
    }
    Ok((source, useful, residual))
}

pub fn evaluate_json(input: &str) -> Result<Report, EvalError> {
    let value: Value = serde_json::from_str(input)
        .map_err(|error| EvalError(format!("$: invalid JSON: {error}")))?;
    exact_keys(
        &value,
        &[
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
        ],
        "$",
    )?;

    if string_field(&value, "schema_version", "$")? != "airborne-energy-session-v0" {
        return Err(fail("$.schema_version", "unsupported schema version"));
    }
    let episode_id = string_field(&value, "episode_id", "$")?.to_owned();
    let record_state = string_field(&value, "record_state", "$")?;
    let evidence_class = string_field(&value, "evidence_class", "$")?;
    let events = value
        .get("events")
        .and_then(Value::as_array)
        .ok_or_else(|| fail("$.events", "must be an array"))?;
    let _ = string_field(&value, "claim_ceiling", "$")?;

    if record_state == "NOT_RUN" {
        if evidence_class != "NOT_RUN"
            || !value["receiver"].is_null()
            || !value["opportunity"].is_null()
            || !value["site_allocation_receipt"].is_null()
            || !events.is_empty()
            || !value["energy_accounting"].is_null()
        {
            return Err(fail(
                "$",
                "NOT_RUN record must contain no evidence or energy",
            ));
        }
        return Ok(Report {
            disposition: Disposition::NotRun,
            episode_id,
            final_state: "NOT_RUN".to_owned(),
            event_count: 0,
            power_active_intervals: 0,
            source_energy_wh: 0,
            battery_terminal_useful_energy_wh: 0,
            residual_wh: 0,
        });
    }

    if record_state != "FIXTURE" || evidence_class != "SYNTHETIC_KNOWN_TRUTH" {
        return Err(fail("$", "v0 admits synthetic fixtures only"));
    }
    if events.is_empty() {
        return Err(fail("$.events", "fixture must contain at least one event"));
    }
    let receiver = value
        .get("receiver")
        .ok_or_else(|| fail("$.receiver", "missing receiver"))?;
    exact_keys(
        receiver,
        &["receiver_id", "receiver_class", "trajectory_sha256"],
        "$.receiver",
    )?;
    let _ = string_field(receiver, "receiver_id", "$.receiver")?;
    let receiver_class = string_field(receiver, "receiver_class", "$.receiver")?;
    if !matches!(
        receiver_class,
        "UAV_DOCKED"
            | "UAV_HOVERING"
            | "UAV_MOVING"
            | "UAV_FIXED_WING"
            | "EVTOL_DOCKED"
            | "EVTOL_AIRBORNE"
    ) {
        return Err(fail(
            "$.receiver.receiver_class",
            "unsupported receiver class",
        ));
    }
    require_sha256(
        string_field(receiver, "trajectory_sha256", "$.receiver")?,
        "$.receiver.trajectory_sha256",
    )?;

    let opportunity = value
        .get("opportunity")
        .ok_or_else(|| fail("$.opportunity", "missing opportunity"))?;
    exact_keys(
        opportunity,
        &[
            "opportunity_id",
            "bearer",
            "field_regime",
            "channel_id",
            "envelope_sha256",
            "envelope_state",
        ],
        "$.opportunity",
    )?;
    let _ = string_field(opportunity, "opportunity_id", "$.opportunity")?;
    let bearer = string_field(opportunity, "bearer", "$.opportunity")?;
    let field_regime = string_field(opportunity, "field_regime", "$.opportunity")?;
    let expected_regime = match bearer {
        "OPTICAL_LASER" | "RF_MICROWAVE" => "FAR_FIELD",
        "IPT" | "CPT" => "NEAR_FIELD",
        "CONDUCTIVE_CONTACT" | "ROBOTIC_DOCK" => "CONTACT",
        "TETHERED" => "GUIDED",
        "BATTERY_EXCHANGE" => "ENERGY_CARRIER",
        _ => return Err(fail("$.opportunity.bearer", "unsupported bearer")),
    };
    if field_regime != expected_regime {
        return Err(fail("$.opportunity.field_regime", "does not match bearer"));
    }
    let _ = string_field(opportunity, "channel_id", "$.opportunity")?;
    require_sha256(
        string_field(opportunity, "envelope_sha256", "$.opportunity")?,
        "$.opportunity.envelope_sha256",
    )?;
    if string_field(opportunity, "envelope_state", "$.opportunity")? != "ADMITTED_SYNTHETIC" {
        return Err(fail(
            "$.opportunity.envelope_state",
            "fixture envelope must be explicitly admitted synthetic",
        ));
    }

    let allocation = value
        .get("site_allocation_receipt")
        .ok_or_else(|| fail("$.site_allocation_receipt", "missing allocation receipt"))?;
    exact_keys(
        allocation,
        &[
            "site_id",
            "allocation_id",
            "allocation_sha256",
            "valid_from_ms",
            "valid_until_ms",
        ],
        "$.site_allocation_receipt",
    )?;
    let _ = string_field(allocation, "site_id", "$.site_allocation_receipt")?;
    let _ = string_field(allocation, "allocation_id", "$.site_allocation_receipt")?;
    require_sha256(
        string_field(allocation, "allocation_sha256", "$.site_allocation_receipt")?,
        "$.site_allocation_receipt.allocation_sha256",
    )?;
    let valid_from = uint_field(allocation, "valid_from_ms", "$.site_allocation_receipt")?;
    let valid_until = uint_field(allocation, "valid_until_ms", "$.site_allocation_receipt")?;
    if valid_from >= valid_until {
        return Err(fail(
            "$.site_allocation_receipt",
            "allocation validity interval must be ordered",
        ));
    }

    let mut state = State::Unavailable;
    let mut last_timestamp = 0_u64;
    let mut event_ids = BTreeSet::new();
    let mut power_active_intervals = 0_u64;
    let mut admission_epochs = 0_u64;
    let mut active = false;
    let mut safe_off_intervals = 0_u64;

    for (index, event) in events.iter().enumerate() {
        let path = format!("$.events[{index}]");
        exact_keys(
            event,
            &[
                "event_id",
                "sequence",
                "at_ms",
                "from_state",
                "to_state",
                "transition_reason",
                "evidence",
            ],
            &path,
        )?;
        let event_id = string_field(event, "event_id", &path)?.to_owned();
        if !event_ids.insert(event_id) {
            return Err(fail(&path, "duplicate event_id"));
        }
        let sequence = uint_field(event, "sequence", &path)?;
        if sequence != index as u64 {
            return Err(fail(
                &format!("{path}.sequence"),
                "must be contiguous from zero",
            ));
        }
        let timestamp = uint_field(event, "at_ms", &path)?;
        if index > 0 && timestamp < last_timestamp {
            return Err(fail(
                &format!("{path}.at_ms"),
                "timestamps must be monotonic",
            ));
        }
        last_timestamp = timestamp;

        let from = State::parse(string_field(event, "from_state", &path)?, &path)?;
        let to = State::parse(string_field(event, "to_state", &path)?, &path)?;
        if from != state {
            return Err(fail(&path, "from_state does not match previous to_state"));
        }
        if !allowed_transition(from, to) {
            return Err(fail(&path, "transition is not in the v0 state graph"));
        }
        let _ = string_field(event, "transition_reason", &path)?;

        let evidence = event
            .get("evidence")
            .ok_or_else(|| fail(&path, "missing evidence"))?;
        exact_keys(
            evidence,
            &[
                "motion_state",
                "receiver_identity_state",
                "authorization_state",
                "allocation_state",
                "envelope_state",
                "position_state",
                "tracking_state",
                "local_protection_state",
                "deenergized_state",
            ],
            &format!("{path}.evidence"),
        )?;
        let motion_state = string_field(evidence, "motion_state", &path)?;
        if !matches!(
            motion_state,
            "PARKED_LANDED_DOCKED"
                | "QUASI_STATIC_HOVER"
                | "MOVING_HOVER"
                | "TRANSLATION"
                | "CLIMB"
                | "DESCENT"
                | "DYNAMIC_FLIGHT"
                | "DIVERSION"
        ) {
            return Err(fail(
                &format!("{path}.evidence.motion_state"),
                "unsupported motion state",
            ));
        }

        if to == State::AdmissionPending {
            validate_admission(evidence, &path)?;
            if string_field(evidence, "local_protection_state", &path)? != "INHIBIT" {
                return Err(fail(
                    &format!("{path}.evidence.local_protection_state"),
                    "power must remain inhibited while admission is pending",
                ));
            }
            admission_epochs = admission_epochs
                .checked_add(1)
                .ok_or_else(|| fail(&path, "admission epoch overflow"))?;
        }

        if to == State::TrackedNoPower {
            require_evidence_string(evidence, "tracking_state", "TRACKED_SYNTHETIC", &path)?;
            require_evidence_string(evidence, "local_protection_state", "INHIBIT", &path)?;
        }

        if to == State::PowerActive {
            if from != State::TrackedNoPower {
                return Err(fail(&path, "POWER_ACTIVE must follow TRACKED_NO_POWER"));
            }
            validate_admission(evidence, &path)?;
            require_evidence_string(evidence, "tracking_state", "TRACKED_SYNTHETIC", &path)?;
            require_evidence_string(
                evidence,
                "local_protection_state",
                "PERMIT_SYNTHETIC_ONLY",
                &path,
            )?;
            if admission_epochs <= power_active_intervals {
                return Err(fail(
                    &path,
                    "POWER_ACTIVE requires a fresh admission epoch after each departure",
                ));
            }
            if timestamp < valid_from || timestamp > valid_until {
                return Err(fail(
                    &path,
                    "POWER_ACTIVE event is outside the site allocation validity window",
                ));
            }
            active = true;
            power_active_intervals = power_active_intervals
                .checked_add(1)
                .ok_or_else(|| fail(&path, "power interval count overflow"))?;
        }

        if to == State::DeparturePending {
            if from != State::PowerActive {
                return Err(fail(
                    &path,
                    "DEPARTURE_PENDING must close an active power interval",
                ));
            }
            require_evidence_string(evidence, "local_protection_state", "INHIBIT", &path)?;
            require_evidence_string(evidence, "tracking_state", "LOST", &path)?;
        }

        if to == State::SafeOff {
            if from != State::DeparturePending {
                return Err(fail(&path, "SAFE_OFF must follow DEPARTURE_PENDING"));
            }
            require_evidence_string(evidence, "local_protection_state", "INHIBIT", &path)?;
            require_evidence_string(evidence, "deenergized_state", "CONFIRMED_SYNTHETIC", &path)?;
            if !active {
                return Err(fail(&path, "SAFE_OFF without an active interval"));
            }
            active = false;
            safe_off_intervals = safe_off_intervals
                .checked_add(1)
                .ok_or_else(|| fail(&path, "safe-off interval count overflow"))?;
        }

        if to == State::RecoveryRequired && from == State::PowerActive {
            require_evidence_string(evidence, "local_protection_state", "INHIBIT", &path)?;
            require_evidence_string(evidence, "deenergized_state", "CONFIRMED_SYNTHETIC", &path)?;
            active = false;
            safe_off_intervals = safe_off_intervals
                .checked_add(1)
                .ok_or_else(|| fail(&path, "safe-off interval count overflow"))?;
        }

        state = to;
    }

    if active || matches!(state, State::PowerActive | State::DeparturePending) {
        return Err(fail("$.events", "trace ends before a safe-off disposition"));
    }
    if power_active_intervals != safe_off_intervals {
        return Err(fail(
            "$.events",
            "every power-active interval requires a safe-off disposition",
        ));
    }
    if state != State::Complete && state != State::Aborted && state != State::RecoveryRequired {
        return Err(fail("$.events", "trace must end in a terminal disposition"));
    }

    let energy = value
        .get("energy_accounting")
        .ok_or_else(|| fail("$.energy_accounting", "missing energy ledger"))?;
    let (source_energy_wh, battery_terminal_useful_energy_wh, residual_wh) =
        validate_energy_ledger(energy)?;

    Ok(Report {
        disposition: Disposition::PassSynthetic,
        episode_id,
        final_state: state.as_str().to_owned(),
        event_count: events.len() as u64,
        power_active_intervals,
        source_energy_wh,
        battery_terminal_useful_energy_wh,
        residual_wh,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_value() -> Value {
        serde_json::from_str(POSITIVE_FIXTURE).expect("positive fixture JSON parses")
    }

    #[test]
    fn synthetic_acquire_depart_reacquire_trace_passes_with_two_intervals() {
        let report = evaluate_json(POSITIVE_FIXTURE).expect("known-truth fixture passes");
        assert_eq!(report.disposition, Disposition::PassSynthetic);
        assert_eq!(report.final_state, "COMPLETE");
        assert_eq!(report.event_count, 15);
        assert_eq!(report.power_active_intervals, 2);
        assert_eq!(report.source_energy_wh, 40);
        assert_eq!(report.battery_terminal_useful_energy_wh, 24);
        assert_eq!(report.residual_wh, 0);
    }

    #[test]
    fn not_run_template_never_becomes_a_pass() {
        let report = evaluate_json(NOT_RUN_FIXTURE).expect("NOT_RUN is an explicit disposition");
        assert_eq!(report.disposition, Disposition::NotRun);
        assert_eq!(report.final_state, "NOT_RUN");
        assert_eq!(report.event_count, 0);
        assert_eq!(report.power_active_intervals, 0);
    }

    #[test]
    fn revoked_authorization_cannot_enter_power_active() {
        let error = evaluate_json(REVOKED_FIXTURE).expect_err("revoked authorization rejects");
        assert!(error.to_string().contains("authorization_state"));
    }

    #[test]
    fn stale_envelope_cannot_enter_power_active() {
        let mut value = fixture_value();
        value["events"][4]["evidence"]["envelope_state"] = Value::String("STALE".into());
        let error = evaluate_json(&value.to_string()).expect_err("stale envelope rejects");
        assert!(error.to_string().contains("envelope_state"));
    }

    #[test]
    fn reacquisition_must_repeat_admission_before_acquisition() {
        let mut value = fixture_value();
        value["events"][8]["from_state"] = Value::String("REACQUISITION_PENDING".into());
        value["events"][8]["to_state"] = Value::String("ACQUISITION_PENDING".into());
        let error = evaluate_json(&value.to_string()).expect_err("reacquisition must re-admit");
        assert!(
            error
                .to_string()
                .contains("transition is not in the v0 state graph")
        );
    }

    #[test]
    fn safe_off_requires_positive_deenergization_confirmation() {
        let mut value = fixture_value();
        value["events"][6]["evidence"]["deenergized_state"] = Value::String("UNCONFIRMED".into());
        let error = evaluate_json(&value.to_string()).expect_err("unconfirmed safe-off rejects");
        assert!(error.to_string().contains("deenergized_state"));
    }

    #[test]
    fn energy_ledger_must_close_at_each_plane() {
        let mut value = fixture_value();
        value["energy_accounting"]["battery_terminal_useful_energy_wh"] = Value::from(23_u64);
        let error = evaluate_json(&value.to_string()).expect_err("non-closing ledger rejects");
        assert!(
            error.to_string().contains("receiver-to-battery")
                || error.to_string().contains("aggregate source-to-battery")
        );
    }

    #[test]
    fn event_timestamps_must_be_monotonic() {
        let mut value = fixture_value();
        value["events"][4]["at_ms"] = Value::from(50_u64);
        let error = evaluate_json(&value.to_string()).expect_err("time reversal rejects");
        assert!(error.to_string().contains("timestamps must be monotonic"));
    }

    #[test]
    fn event_state_chain_must_be_contiguous() {
        let mut value = fixture_value();
        value["events"][3]["from_state"] = Value::String("DISCOVERED".into());
        let error = evaluate_json(&value.to_string()).expect_err("broken state chain rejects");
        assert!(
            error
                .to_string()
                .contains("from_state does not match previous to_state")
        );
    }

    #[test]
    fn site_allocation_must_be_valid_at_each_power_active_interval() {
        let mut value = fixture_value();
        value["site_allocation_receipt"]["valid_until_ms"] = Value::from(1000_u64);
        let error = evaluate_json(&value.to_string()).expect_err("expired allocation rejects");
        assert!(
            error
                .to_string()
                .contains("outside the site allocation validity window")
        );
    }
}
