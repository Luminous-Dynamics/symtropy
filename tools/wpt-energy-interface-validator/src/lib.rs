// SPDX-License-Identifier: AGPL-3.0-or-later
//! Stateful conformance checks for the proposed typed WPT energy-service profile.
//!
//! This validates event ordering, digest linkage, typed services and energization
//! gates. It is not a transport-security implementation or hardware safety case.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct SessionState {
    sequence: u64,
    timestamp_ns: u64,
    event_digest: String,
    state: String,
}

#[derive(Default)]
pub struct SessionValidator {
    sessions: HashMap<String, SessionState>,
}

fn string_field(value: &Value, key: &str, path: &str) -> Result<String, String> {
    let item = value
        .get(key)
        .ok_or_else(|| format!("missing {path}.{key}"))?;
    let text = item
        .as_str()
        .ok_or_else(|| format!("{path}.{key} must be a string"))?;
    if text.trim().is_empty() {
        return Err(format!("{path}.{key} must not be empty"));
    }
    Ok(text.to_owned())
}

fn bool_field(value: &Value, key: &str, path: &str) -> Result<bool, String> {
    value
        .get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("{path}.{key} must be a boolean"))
}

fn u64_field(value: &Value, key: &str, path: &str) -> Result<u64, String> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{path}.{key} must be a nonnegative integer"))
}

fn finite_field(value: &Value, key: &str, path: &str) -> Result<f64, String> {
    let number = value
        .get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("{path}.{key} must be a number"))?;
    if !number.is_finite() {
        return Err(format!("{path}.{key} must be finite"));
    }
    Ok(number)
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn digest_value(value: &Value) -> Result<String, String> {
    let mut payload = value.clone();
    let object = payload
        .as_object_mut()
        .ok_or_else(|| "event must be a JSON object".to_owned())?;
    object.remove("event_digest");
    let encoded = serde_json::to_vec(&payload)
        .map_err(|error| format!("cannot serialize event for digest: {error}"))?;
    let digest = Sha256::digest(encoded);
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn service_profile(service: &Value) -> Result<(), String> {
    let service_type = string_field(service, "service_type", "service")?;
    let quantity = string_field(service, "service_quantity", "service")?;
    let method = string_field(service, "measurement_method", "service")?;
    let unit = string_field(service, "service_unit", "service")?;
    if unit != "J" {
        return Err("service_unit must be joules (J)".into());
    }
    let expected = match service_type.as_str() {
        "BATTERY_DC" => ("ELECTRICAL_DC_ENERGY", "DC_REAL_POWER_INTEGRATION"),
        "SHAFT_MECHANICAL" => ("MECHANICAL_SHAFT_WORK", "TORQUE_SPEED_INTEGRATION"),
        "PROCESS_HEAT" => ("THERMAL_PROCESS_ENERGY", "CALORIMETRY_OR_ENTHALPY"),
        "OPTICAL_RADIANT" => ("OPTICAL_RADIANT_ENERGY", "OPTICAL_RADIOMETRY"),
        "CHEMICAL_PROCESS" => (
            "CHEMICAL_FREE_ENERGY_CHANGE",
            "REACTION_EXTENT_AND_CHEMICAL_POTENTIAL",
        ),
        _ => return Err(format!("unsupported service_type {service_type}")),
    };
    if quantity != expected.0 || method != expected.1 {
        return Err(format!(
            "service type {service_type} requires quantity {} and method {}",
            expected.0, expected.1
        ));
    }
    if service_type == "CHEMICAL_PROCESS" {
        let mode = string_field(service, "chemical_energy_mode", "service")?;
        if mode != "STORED" && mode != "RELEASED" {
            return Err("chemical_energy_mode must be STORED or RELEASED".into());
        }
    }
    let requested = finite_field(service, "requested_useful_energy_j", "service")?;
    let minimum = finite_field(service, "minimum_useful_energy_j", "service")?;
    let maximum_input = finite_field(service, "maximum_input_power_w", "service")?;
    if requested < minimum || minimum < 0.0 || maximum_input <= 0.0 {
        return Err("service energy request or input-power envelope is invalid".into());
    }
    if string_field(service, "source_input_boundary_id", "service")?.is_empty()
        || string_field(service, "useful_output_boundary_id", "service")?.is_empty()
    {
        return Err("service measurement boundaries must be explicit".into());
    }
    let profile = service
        .get("power_profile")
        .and_then(Value::as_array)
        .ok_or_else(|| "service.power_profile must be an array".to_owned())?;
    if profile.is_empty() {
        return Err("service.power_profile must not be empty".into());
    }
    for (index, point) in profile.iter().enumerate() {
        let power = finite_field(
            point,
            "requested_input_power_w",
            &format!("service.power_profile[{index}]"),
        )?;
        if power < 0.0 || power > maximum_input {
            return Err(format!(
                "service power profile point {index} exceeds its declared envelope"
            ));
        }
        let _ = u64_field(point, "time_ns", &format!("service.power_profile[{index}]"))?;
    }
    Ok(())
}

fn common_evidence(record: &Value) -> Result<(), String> {
    let source = record
        .get("source")
        .ok_or_else(|| "missing source".to_owned())?;
    let receiver = record
        .get("receiver")
        .ok_or_else(|| "missing receiver".to_owned())?;
    for key in ["capability_sha256", "firmware_sha256", "topology_sha256"] {
        let digest = string_field(source, key, "source")?;
        if !valid_sha256(&digest) {
            return Err(format!("source.{key} must be a SHA-256 digest"));
        }
    }
    for key in ["capability_sha256", "firmware_sha256"] {
        let digest = string_field(receiver, key, "receiver")?;
        if !valid_sha256(&digest) {
            return Err(format!("receiver.{key} must be a SHA-256 digest"));
        }
    }
    let service = record
        .get("service")
        .ok_or_else(|| "missing service".to_owned())?;
    service_profile(service)?;
    let service_type = string_field(service, "service_type", "service")?;
    if string_field(receiver, "supported_service_type", "receiver")? != service_type {
        return Err("receiver does not support the requested typed service".into());
    }
    let source_power = finite_field(source, "maximum_input_power_w", "source")?;
    let service_power = finite_field(service, "maximum_input_power_w", "service")?;
    if !source_power.is_finite() || source_power <= 0.0 || service_power > source_power {
        return Err("service input-power envelope exceeds source capability".into());
    }

    let authorization = record
        .get("authorization")
        .ok_or_else(|| "missing authorization".to_owned())?;
    for key in ["nonce_sha256", "credential_receipt_sha256"] {
        if !valid_sha256(&string_field(authorization, key, "authorization")?) {
            return Err(format!("authorization.{key} must be a SHA-256 digest"));
        }
    }
    let positioning = record
        .get("positioning")
        .ok_or_else(|| "missing positioning".to_owned())?;
    if let Some(raw_digest) = positioning.get("position_receipt_sha256")
        && !raw_digest.is_null()
    {
        let digest = raw_digest
            .as_str()
            .ok_or_else(|| "position receipt must be a digest or null".to_owned())?;
        if !valid_sha256(digest) {
            return Err("position receipt must be a SHA-256 digest".into());
        }
    }
    if string_field(positioning, "status", "positioning")? == "CONFIRMED"
        && positioning
            .get("position_receipt_sha256")
            .and_then(Value::as_str)
            .is_none()
    {
        return Err("confirmed positioning requires a position receipt".into());
    }
    Ok(())
}

fn energization_gate(record: &Value) -> Result<(), String> {
    let timestamp_ns = u64_field(record, "timestamp_ns", "record")?;
    let authorization = record
        .get("authorization")
        .ok_or_else(|| "missing authorization".to_owned())?;
    if !bool_field(authorization, "authenticated", "authorization")?
        || !bool_field(authorization, "authorized", "authorization")?
        || !bool_field(authorization, "fresh", "authorization")?
    {
        return Err(
            "energization requires authenticated, authorized and fresh session credentials".into(),
        );
    }
    if timestamp_ns >= u64_field(authorization, "expires_ns", "authorization")? {
        return Err("energization authorization has expired".into());
    }

    let positioning = record
        .get("positioning")
        .ok_or_else(|| "missing positioning".to_owned())?;
    if string_field(positioning, "status", "positioning")? != "CONFIRMED" {
        return Err("energization requires confirmed receiver positioning".into());
    }

    let safety = record
        .get("safety")
        .ok_or_else(|| "missing safety".to_owned())?;
    if !bool_field(safety, "protection_ready", "safety")?
        || !bool_field(safety, "foreign_object_clear", "safety")?
        || !bool_field(safety, "hard_limits_clear", "safety")?
        || string_field(safety, "sensor_health", "safety")? != "HEALTHY"
    {
        return Err(
            "energization blocked by a protection, foreign-object, sensor or hard-limit gate"
                .into(),
        );
    }

    let service = record
        .get("service")
        .ok_or_else(|| "missing service".to_owned())?;
    let maximum = finite_field(service, "maximum_input_power_w", "service")?;
    let source = record
        .get("source")
        .ok_or_else(|| "missing source".to_owned())?;
    let source_maximum = finite_field(source, "maximum_input_power_w", "source")?;
    let profile = service
        .get("power_profile")
        .and_then(Value::as_array)
        .ok_or_else(|| "missing service power profile".to_owned())?;
    for point in profile {
        let requested = finite_field(point, "requested_input_power_w", "service.power_profile")?;
        if requested > maximum || requested > source_maximum {
            return Err("energization power request exceeds source or service envelope".into());
        }
    }
    Ok(())
}

fn verify_measurement(record: &Value) -> Result<(), String> {
    let measurement = record
        .get("measurement")
        .ok_or_else(|| "missing measurement".to_owned())?;
    let status = string_field(measurement, "status", "measurement")?;
    if status == "NOT_MEASURED" || status == "UNRESOLVED" {
        return Err("settlement measurement is not available or unresolved".into());
    }
    if status != "MEASURED" {
        return Err("unknown measurement status".into());
    }
    for key in ["measurement_receipt_sha256", "calibration_receipt_sha256"] {
        if !valid_sha256(&string_field(measurement, key, "measurement")?) {
            return Err(format!("measurement.{key} must be a SHA-256 digest"));
        }
    }
    let source = finite_field(measurement, "source_input_energy_j", "measurement")?;
    let useful = finite_field(measurement, "useful_service_energy_j", "measurement")?;
    let auxiliary = finite_field(measurement, "auxiliary_energy_j", "measurement")?;
    let stored_delta = finite_field(measurement, "stored_energy_delta_j", "measurement")?;
    let accounted_loss = finite_field(measurement, "accounted_loss_energy_j", "measurement")?;
    let residual = finite_field(measurement, "energy_residual_j", "measurement")?;
    let uncertainty = finite_field(measurement, "expanded_uncertainty_j", "measurement")?;
    if source < 0.0 || useful < 0.0 || auxiliary < 0.0 || accounted_loss < 0.0 || uncertainty < 0.0
    {
        return Err("measured energies, losses and uncertainty must be nonnegative".into());
    }
    let closure_residual = source - useful - auxiliary - stored_delta - accounted_loss;
    if !closure_residual.is_finite()
        || (closure_residual - residual).abs() > uncertainty
        || residual.abs() > uncertainty
    {
        return Err(
            "source-to-service energy closure is unresolved within the declared uncertainty".into(),
        );
    }
    let service = record
        .get("service")
        .ok_or_else(|| "missing service".to_owned())?;
    let minimum = finite_field(service, "minimum_useful_energy_j", "service")?;
    if useful < minimum {
        return Err("measured useful energy does not meet the contracted minimum".into());
    }
    Ok(())
}

fn next_state(event_type: &str, prior: Option<&str>) -> Result<&'static str, String> {
    match (event_type, prior) {
        ("DISCOVER", None) => Ok("DISCOVERED"),
        ("OFFER", Some("DISCOVERED")) => Ok("OFFERED"),
        ("AUTHENTICATE", Some("OFFERED")) => Ok("AUTHENTICATED"),
        ("RESERVE", Some("AUTHENTICATED")) => Ok("RESERVED"),
        ("POSITION_CONFIRM", Some("RESERVED")) => Ok("POSITION_CONFIRMED"),
        ("ENERGIZE", Some("POSITION_CONFIRMED")) => Ok("ENERGIZED"),
        ("POWER_UPDATE", Some("ENERGIZED")) => Ok("ENERGIZED"),
        ("HANDOVER_BEGIN", Some("ENERGIZED")) => Ok("ENERGIZED"),
        ("RAMP_DOWN", Some("ENERGIZED")) => Ok("RAMPING_DOWN"),
        ("DEENERGIZE", Some("RAMPING_DOWN")) => Ok("DEENERGIZED"),
        ("VERIFY", Some("DEENERGIZED")) => Ok("VERIFIED"),
        ("ABORT", Some(state)) if state != "VERIFIED" && state != "ABORTED" => Ok("ABORTED"),
        _ => Err(format!(
            "invalid event transition: {event_type} from {}",
            prior.unwrap_or("START")
        )),
    }
}

fn evaluate_checked(record: &Value, validator: &mut SessionValidator) -> Result<String, String> {
    let schema_version = string_field(record, "schema_version", "record")?;
    if schema_version != "1.0.0" {
        return Err(format!("unsupported schema_version {schema_version}"));
    }
    let event_id = string_field(record, "event_id", "record")?;
    let session_id = string_field(record, "session_id", "record")?;
    let sequence = u64_field(record, "sequence", "record")?;
    let timestamp_ns = u64_field(record, "timestamp_ns", "record")?;
    let event_type = string_field(record, "event_type", "record")?;
    let claimed_digest = string_field(record, "event_digest", "record")?;
    if !valid_sha256(&claimed_digest) || digest_value(record)? != claimed_digest {
        return Err(format!("event digest mismatch for {event_id}"));
    }
    let previous_digest = record
        .get("previous_event_sha256")
        .ok_or_else(|| "missing previous_event_sha256".to_owned())?;
    let prior = validator.sessions.get(&session_id).cloned();
    match (&prior, previous_digest) {
        (None, Value::Null) if sequence == 0 => {}
        (None, _) => {
            return Err(
                "first session event must use sequence 0 and a null previous digest".into(),
            );
        }
        (Some(previous), Value::String(digest))
            if previous.sequence.checked_add(1) == Some(sequence)
                && timestamp_ns > previous.timestamp_ns
                && digest == &previous.event_digest => {}
        (Some(_), _) => {
            return Err(
                "session event sequence, timestamp or previous digest is not continuous".into(),
            );
        }
    }

    common_evidence(record)?;
    let service = record
        .get("service")
        .ok_or_else(|| "missing service".to_owned())?;
    let service_type = string_field(service, "service_type", "service")?;
    let receiver = record
        .get("receiver")
        .ok_or_else(|| "missing receiver".to_owned())?;
    if string_field(receiver, "supported_service_type", "receiver")? != service_type {
        return Err("receiver service capability does not match request".into());
    }

    let prior_state = prior.as_ref().map(|value| value.state.as_str());
    let current_state = prior_state.unwrap_or("");
    let mut disposition = "ACCEPTED";
    let target_state = match event_type.as_str() {
        "ENERGIZE" => {
            if prior_state != Some("POSITION_CONFIRMED") || energization_gate(record).is_err() {
                disposition = "REJECTED";
                current_state
            } else {
                "ENERGIZED"
            }
        }
        "VERIFY" => {
            if prior_state != Some("DEENERGIZED") {
                disposition = "REJECTED";
                current_state
            } else if verify_measurement(record).is_err() {
                disposition = "UNRESOLVED";
                current_state
            } else {
                "VERIFIED"
            }
        }
        "ABORT" => {
            let safety = record
                .get("safety")
                .ok_or_else(|| "missing safety".to_owned())?;
            if matches!(prior_state, Some("ENERGIZED") | Some("RAMPING_DOWN"))
                && !bool_field(safety, "safe_state_confirmed", "safety")?
            {
                disposition = "REJECTED";
                current_state
            } else {
                "ABORTED"
            }
        }
        _ => match next_state(&event_type, prior_state) {
            Ok(target) => target,
            Err(_) => {
                disposition = "REJECTED";
                current_state
            }
        },
    };

    let reported_state = string_field(record, "session_state", "record")?;
    if reported_state != target_state {
        return Err(format!(
            "session_state {reported_state} does not match validated state {target_state}"
        ));
    }
    let digest = claimed_digest;
    validator.sessions.insert(
        session_id,
        SessionState {
            sequence,
            timestamp_ns,
            event_digest: digest,
            state: target_state.to_owned(),
        },
    );
    Ok(disposition.to_owned())
}

pub fn evaluate_record(record: &Value, validator: &mut SessionValidator) -> Value {
    let event_id = record
        .get("event_id")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_owned();
    match evaluate_checked(record, validator) {
        Ok(disposition) => json!({
            "event_id": event_id,
            "disposition": disposition,
            "claim_ceiling": "synthetic protocol conformance only; not hardware safety or standards certification"
        }),
        Err(message) => json!({
            "event_id": event_id,
            "disposition": "INVALID_INPUT",
            "message": message
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::{SessionValidator, evaluate_record, service_profile};
    use serde_json::Value;

    const FIXTURE: &str =
        include_str!("../../../benchmarks/wpt-energy-interface/v1/known-truth.jsonl");

    #[test]
    fn frozen_service_interface_corpus_matches_known_truth() {
        let mut sessions = SessionValidator::default();
        let mut records = 0usize;
        for line in FIXTURE.lines().filter(|line| !line.trim().is_empty()) {
            let record: Value = serde_json::from_str(line).expect("valid fixture JSON");
            let expected = record["expected_disposition"]
                .as_str()
                .expect("expected disposition");
            let report = evaluate_record(&record, &mut sessions);
            assert_eq!(
                report["disposition"], expected,
                "event {}",
                record["event_id"]
            );
            records += 1;
        }
        assert!(
            records >= 60,
            "the frozen corpus must cover full sessions, not isolated schema objects"
        );
    }

    #[test]
    fn accepts_chemical_free_energy_service_with_declared_method() {
        let service = serde_json::json!({
            "service_type": "CHEMICAL_PROCESS",
            "service_quantity": "CHEMICAL_FREE_ENERGY_CHANGE",
            "measurement_method": "REACTION_EXTENT_AND_CHEMICAL_POTENTIAL",
            "chemical_energy_mode": "STORED",
            "service_unit": "J",
            "requested_useful_energy_j": 500.0,
            "minimum_useful_energy_j": 450.0,
            "maximum_input_power_w": 250.0,
            "source_input_boundary_id": "transmitter-dc-input",
            "useful_output_boundary_id": "chemical-state-output",
            "power_profile": [
                {"requested_input_power_w": 200.0, "time_ns": 0},
                {"requested_input_power_w": 225.0, "time_ns": 1_000_000},
            ]
        });
        assert!(service_profile(&service).is_ok());

        let mut released = service.clone();
        released["chemical_energy_mode"] = serde_json::json!("RELEASED");
        assert!(service_profile(&released).is_ok());
    }

    #[test]
    fn rejects_chemical_service_when_measurement_method_is_electrical_input() {
        let service = serde_json::json!({
            "service_type": "CHEMICAL_PROCESS",
            "service_quantity": "CHEMICAL_FREE_ENERGY_CHANGE",
            "measurement_method": "DC_REAL_POWER_INTEGRATION",
            "chemical_energy_mode": "STORED",
            "service_unit": "J",
            "requested_useful_energy_j": 500.0,
            "minimum_useful_energy_j": 450.0,
            "maximum_input_power_w": 250.0,
            "source_input_boundary_id": "transmitter-dc-input",
            "useful_output_boundary_id": "chemical-state-output",
            "power_profile": [
                {"requested_input_power_w": 200.0, "time_ns": 0}
            ]
        });
        assert!(
            service_profile(&service)
                .unwrap_err()
                .contains("requires quantity")
        );
    }

    #[test]
    fn rejects_unknown_chemical_energy_mode() {
        let service = serde_json::json!({
            "service_type": "CHEMICAL_PROCESS",
            "service_quantity": "CHEMICAL_FREE_ENERGY_CHANGE",
            "measurement_method": "REACTION_EXTENT_AND_CHEMICAL_POTENTIAL",
            "chemical_energy_mode": "ABSOLUTE_VALUE",
            "service_unit": "J",
            "requested_useful_energy_j": 500.0,
            "minimum_useful_energy_j": 450.0,
            "maximum_input_power_w": 250.0,
            "source_input_boundary_id": "transmitter-dc-input",
            "useful_output_boundary_id": "chemical-state-output",
            "power_profile": [
                {"requested_input_power_w": 200.0, "time_ns": 0}
            ]
        });
        assert!(service_profile(&service)
            .unwrap_err()
            .contains("STORED or RELEASED"));
    }

    #[test]
    fn rejects_chemical_service_without_explicit_free_energy_sign_mode() {
        let service = serde_json::json!({
            "service_type": "CHEMICAL_PROCESS",
            "service_quantity": "CHEMICAL_FREE_ENERGY_CHANGE",
            "measurement_method": "REACTION_EXTENT_AND_CHEMICAL_POTENTIAL",
            "service_unit": "J",
            "requested_useful_energy_j": 500.0,
            "minimum_useful_energy_j": 450.0,
            "maximum_input_power_w": 250.0,
            "source_input_boundary_id": "transmitter-dc-input",
            "useful_output_boundary_id": "chemical-state-output",
            "power_profile": [
                {"requested_input_power_w": 200.0, "time_ns": 0}
            ]
        });
        assert!(service_profile(&service)
            .unwrap_err()
            .contains("chemical_energy_mode"));
    }

    #[test]
    fn rejects_event_payload_tampering_when_digest_is_not_recomputed() {
        let first = FIXTURE
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap();
        let mut record: Value = serde_json::from_str(first).unwrap();
        record["service"]["requested_useful_energy_j"] = serde_json::json!(999_999);
        let report = evaluate_record(&record, &mut SessionValidator::default());
        assert_eq!(report["disposition"], "INVALID_INPUT");
    }
}
