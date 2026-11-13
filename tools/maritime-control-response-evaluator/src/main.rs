use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    process::ExitCode,
};

const VERSION: &str = "0.1.0";
const FROZEN: &str =
    include_str!("../../../benchmarks/maritime-control-response/control-response-corpus-v0.1.json");
type Check<T> = Result<T, String>;

fn object<'a>(v: &'a Value, at: &str) -> Check<&'a serde_json::Map<String, Value>> {
    v.as_object()
        .ok_or_else(|| format!("{at}: expected object"))
}
fn exact_keys(v: &Value, keys: &[&str], at: &str) -> Check<()> {
    let map = object(v, at)?;
    let expected: BTreeSet<&str> = keys.iter().copied().collect();
    let actual: BTreeSet<&str> = map.keys().map(String::as_str).collect();
    let missing: Vec<_> = expected.difference(&actual).copied().collect();
    let unknown: Vec<_> = actual.difference(&expected).copied().collect();
    if !missing.is_empty() || !unknown.is_empty() {
        return Err(format!(
            "{at}: missing keys {missing:?}; unknown keys {unknown:?}"
        ));
    }
    Ok(())
}
fn field<'a>(v: &'a Value, key: &str, at: &str) -> Check<&'a Value> {
    object(v, at)?
        .get(key)
        .ok_or_else(|| format!("{at}: missing {key}"))
}
fn string<'a>(v: &'a Value, key: &str, at: &str) -> Check<&'a str> {
    field(v, key, at)?
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| format!("{at}.{key}: expected non-empty string"))
}
fn number(v: &Value, key: &str, at: &str) -> Check<f64> {
    let n = field(v, key, at)?
        .as_f64()
        .ok_or_else(|| format!("{at}.{key}: expected number"))?;
    if !n.is_finite() {
        return Err(format!("{at}.{key}: non-finite number"));
    }
    Ok(n)
}
fn unsigned(v: &Value, key: &str, at: &str) -> Check<u64> {
    field(v, key, at)?
        .as_u64()
        .ok_or_else(|| format!("{at}.{key}: expected non-negative integer"))
}
fn boolean(v: &Value, key: &str, at: &str) -> Check<bool> {
    field(v, key, at)?
        .as_bool()
        .ok_or_else(|| format!("{at}.{key}: expected boolean"))
}
fn string_set(v: &Value, key: &str, at: &str) -> Check<BTreeSet<String>> {
    let values = field(v, key, at)?
        .as_array()
        .ok_or_else(|| format!("{at}.{key}: expected array"))?;
    let mut result = BTreeSet::new();
    for item in values {
        let s = item
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| format!("{at}.{key}: expected non-empty strings"))?;
        if !result.insert(s.to_owned()) {
            return Err(format!("{at}.{key}: duplicate item"));
        }
    }
    Ok(result)
}
fn quality(v: &Value, at: &str) -> Check<&'static str> {
    match string(v, "quality", at)? {
        "CURRENT" => Ok("CURRENT"),
        "MISSING" => Ok("MISSING"),
        "STALE" => Ok("STALE"),
        "REJECTED" => Ok("REJECTED"),
        "CONFLICTED" => Ok("CONFLICTED"),
        "UNKNOWN" => Ok("UNKNOWN"),
        other => Err(format!("{at}.quality: unknown quality {other}")),
    }
}
#[derive(Debug)]
struct Signal {
    role: String,
    value: f64,
    unit: String,
    observed_at_ms: u64,
    timebase_id: String,
    timestamp_uncertainty_ms: u64,
    timestamp_evidence_id: String,
    timestamp_evidence_sha256: String,
    timestamp_capture_point: String,
    qualified_uncertainty_bound_ms: u64,
    quality: String,
    dependencies: BTreeSet<String>,
    derived_from: BTreeSet<String>,
}
fn signal(v: &Value, at: &str, response: bool) -> Check<Signal> {
    let _signal_id = string(v, "signal_id", at)?;
    if response {
        exact_keys(
            v,
            &[
                "signal_id",
                "role",
                "value",
                "unit",
                "observed_at_ms",
                "timebase_id",
                "timestamp_uncertainty_ms",
                "timestamp_evidence_id",
                "timestamp_evidence_sha256",
                "timestamp_capture_point",
                "qualified_uncertainty_bound_ms",
                "quality",
                "dependency_groups",
                "derived_from_signal_ids",
            ],
            at,
        )?;
    } else {
        exact_keys(
            v,
            &[
                "signal_id",
                "role",
                "value",
                "unit",
                "observed_at_ms",
                "timebase_id",
                "timestamp_uncertainty_ms",
                "timestamp_evidence_id",
                "timestamp_evidence_sha256",
                "timestamp_capture_point",
                "qualified_uncertainty_bound_ms",
                "quality",
                "dependency_groups",
            ],
            at,
        )?;
    }
    let derived = if response {
        string_set(v, "derived_from_signal_ids", at)?
    } else {
        BTreeSet::new()
    };
    Ok(Signal {
        role: string(v, "role", at)?.to_owned(),
        value: number(v, "value", at)?,
        unit: string(v, "unit", at)?.to_owned(),
        observed_at_ms: unsigned(v, "observed_at_ms", at)?,
        timebase_id: string(v, "timebase_id", at)?.to_owned(),
        timestamp_uncertainty_ms: unsigned(v, "timestamp_uncertainty_ms", at)?,
        timestamp_evidence_id: string(v, "timestamp_evidence_id", at)?.to_owned(),
        timestamp_evidence_sha256: string(v, "timestamp_evidence_sha256", at)?.to_owned(),
        timestamp_capture_point: string(v, "timestamp_capture_point", at)?.to_owned(),
        qualified_uncertainty_bound_ms: unsigned(v, "qualified_uncertainty_bound_ms", at)?,
        quality: quality(v, at)?.to_owned(),
        dependencies: string_set(v, "dependency_groups", at)?,
        derived_from: derived,
    })
}
fn overlaps(a: &BTreeSet<String>, b: &BTreeSet<String>) -> bool {
    a.iter().any(|item| b.contains(item))
}
/// Inclusive event-time interval after mapping to the declared qualified
/// common timebase. Overflow/underflow is invalid; it is never clamped.
fn time_bounds(timestamp_ms: u64, uncertainty_ms: u64) -> Option<(u64, u64)> {
    Some((
        timestamp_ms.checked_sub(uncertainty_ms)?,
        timestamp_ms.checked_add(uncertainty_ms)?,
    ))
}
fn result(disposition: &str, reasons: &[&str], feedback: &str) -> Value {
    json!({"disposition":disposition,"reason_codes":reasons,"fault_isolation":"NOT_ATTEMPTED",
        "actuator_delivery_confirmation":"NOT_ESTABLISHED","actuator_feedback_state":feedback})
}
fn evaluate_case(case: &Value, max_age: u64, max_horizon: u64) -> Check<(String, Value)> {
    let at = "$.cases[]";
    exact_keys(
        case,
        &[
            "case_id",
            "asset_configuration_id",
            "regime_id",
            "regime_qualified",
            "timebase",
            "prediction_time_ms",
            "prediction_timestamp_uncertainty_ms",
            "prediction_timestamp_evidence_id",
            "prediction_timestamp_evidence_sha256",
            "prediction_timestamp_capture_point",
            "prediction_qualified_uncertainty_bound_ms",
            "command",
            "context",
            "response",
            "response_envelope",
            "actuator_feedback",
            "expected_disposition",
        ],
        at,
    )?;
    let id = string(case, "case_id", at)?.to_owned();
    let prediction = unsigned(case, "prediction_time_ms", at)?;
    let configuration = string(case, "asset_configuration_id", at)?;
    let regime = string(case, "regime_id", at)?;
    let regime_qualified = boolean(case, "regime_qualified", at)?;
    let timebase = field(case, "timebase", at)?;
    exact_keys(
        timebase,
        &[
            "timebase_id",
            "qualification_state",
            "qualification_id",
            "qualification_sha256",
            "time_scale",
            "synchronization_method",
            "timing_profile_id",
            "clock_source_id",
            "integrity_state",
            "valid_from_ms",
            "valid_until_ms",
            "qualified_at_ms",
            "qualified_at_uncertainty_ms",
            "max_offset_bound_ms",
            "max_drift_bound_ppm",
            "path_asymmetry_bound_ms",
            "measurement_evidence_id",
            "measurement_evidence_sha256",
        ],
        &format!("{at}.timebase"),
    )?;
    let timebase_id = string(timebase, "timebase_id", &format!("{at}.timebase"))?;
    let clock_state = string(timebase, "qualification_state", &format!("{at}.timebase"))?;
    let clock_qualification_id = string(timebase, "qualification_id", &format!("{at}.timebase"))?;
    let clock_qualification_sha256 =
        string(timebase, "qualification_sha256", &format!("{at}.timebase"))?;
    let time_scale = string(timebase, "time_scale", &format!("{at}.timebase"))?;
    let sync_method = string(
        timebase,
        "synchronization_method",
        &format!("{at}.timebase"),
    )?;
    let _timing_profile = string(timebase, "timing_profile_id", &format!("{at}.timebase"))?;
    let _clock_source = string(timebase, "clock_source_id", &format!("{at}.timebase"))?;
    let integrity_state = string(timebase, "integrity_state", &format!("{at}.timebase"))?;
    let valid_from = unsigned(timebase, "valid_from_ms", &format!("{at}.timebase"))?;
    let valid_until = unsigned(timebase, "valid_until_ms", &format!("{at}.timebase"))?;
    let qualified_at = unsigned(timebase, "qualified_at_ms", &format!("{at}.timebase"))?;
    let qualified_at_uncertainty = unsigned(
        timebase,
        "qualified_at_uncertainty_ms",
        &format!("{at}.timebase"),
    )?;
    let max_offset = number(timebase, "max_offset_bound_ms", &format!("{at}.timebase"))?;
    let max_drift = number(timebase, "max_drift_bound_ppm", &format!("{at}.timebase"))?;
    let path_asymmetry = number(
        timebase,
        "path_asymmetry_bound_ms",
        &format!("{at}.timebase"),
    )?;
    let _measurement_id = string(
        timebase,
        "measurement_evidence_id",
        &format!("{at}.timebase"),
    )?;
    let measurement_sha = string(
        timebase,
        "measurement_evidence_sha256",
        &format!("{at}.timebase"),
    )?;
    if !valid_hex(clock_qualification_sha256, 64) || !valid_hex(measurement_sha, 64) {
        return Ok((
            id,
            result(
                "INVALID_CONTRACT",
                &["INVALID_CLOCK_QUALIFICATION_DIGEST"],
                "NOT_ASSESSED",
            ),
        ));
    }
    if !matches!(time_scale, "UTC" | "TAI" | "MONOTONIC" | "ENGINE_RELATIVE")
        || !matches!(
            sync_method,
            "IEEE_1588_PTP"
                | "IRIG_B"
                | "GNSS_DISCIPLINED"
                | "COMMON_DAQ_CLOCK"
                | "OTHER_QUALIFIED"
        )
    {
        return Ok((
            id,
            result(
                "INVALID_CONTRACT",
                &["UNSUPPORTED_TIMEBASE_METHOD_OR_SCALE"],
                "NOT_ASSESSED",
            ),
        ));
    }
    if max_offset < 0.0
        || max_drift < 0.0
        || path_asymmetry < 0.0
        || integrity_state == "COMPROMISED"
    {
        return Ok((
            id,
            result(
                "INVALID_CONTRACT",
                &["INVALID_OR_COMPROMISED_CLOCK_QUALIFICATION"],
                "NOT_ASSESSED",
            ),
        ));
    }
    if integrity_state != "ATTESTED" {
        return Ok((
            id,
            result(
                "NOT_EVALUATED",
                &["CLOCK_INTEGRITY_NOT_ATTESTED"],
                "NOT_ASSESSED",
            ),
        ));
    }
    if clock_state != "QUALIFIED" {
        return Ok((
            id,
            result(
                "NOT_EVALUATED",
                &["CLOCK_QUALIFICATION_UNAVAILABLE"],
                "NOT_ASSESSED",
            ),
        ));
    }
    if valid_until < valid_from {
        return Ok((
            id,
            result(
                "INVALID_CONTRACT",
                &["CLOCK_QUALIFICATION_VALIDITY_INVERTED"],
                "NOT_ASSESSED",
            ),
        ));
    }
    let prediction_uncertainty = unsigned(case, "prediction_timestamp_uncertainty_ms", at)?;
    let prediction_bounds = match time_bounds(prediction, prediction_uncertainty) {
        Some(bounds) => bounds,
        None => {
            return Ok((
                id,
                result(
                    "INVALID_CONTRACT",
                    &["PREDICTION_TIMESTAMP_INTERVAL_OVERFLOW"],
                    "NOT_ASSESSED",
                ),
            ));
        }
    };
    let prediction_evidence_id = string(case, "prediction_timestamp_evidence_id", at)?;
    let prediction_evidence_sha = string(case, "prediction_timestamp_evidence_sha256", at)?;
    let prediction_capture_point = string(case, "prediction_timestamp_capture_point", at)?;
    let prediction_qualified_bound =
        unsigned(case, "prediction_qualified_uncertainty_bound_ms", at)?;
    if !valid_hex(prediction_evidence_sha, 64)
        || !matches!(
            prediction_capture_point,
            "HARDWARE_CAPTURE"
                | "DAQ_CAPTURE"
                | "CONTROLLER_SCAN"
                | "GATEWAY_RECEIVE"
                | "HOST_RECEIVE"
                | "RECONSTRUCTED"
        )
        || prediction_uncertainty < prediction_qualified_bound
    {
        return Ok((
            id,
            result(
                "INVALID_CONTRACT",
                &["PREDICTION_TIMESTAMP_EVIDENCE_INSUFFICIENT"],
                "NOT_ASSESSED",
            ),
        ));
    }
    let qualification_time_bounds = match time_bounds(qualified_at, qualified_at_uncertainty) {
        Some(bounds) => bounds,
        None => {
            return Ok((
                id,
                result(
                    "INVALID_CONTRACT",
                    &["CLOCK_QUALIFICATION_TIME_OVERFLOW"],
                    "NOT_ASSESSED",
                ),
            ));
        }
    };
    if qualification_time_bounds.0 > prediction_bounds.1 {
        return Ok((
            id,
            result(
                "INVALID_CONTRACT",
                &["FUTURE_CLOCK_QUALIFICATION"],
                "NOT_ASSESSED",
            ),
        ));
    }
    if qualification_time_bounds.1 > prediction_bounds.0 {
        return Ok((
            id,
            result(
                "NOT_EVALUATED",
                &["CLOCK_QUALIFICATION_ORDER_UNPROVEN"],
                "NOT_ASSESSED",
            ),
        ));
    }
    if valid_from > prediction_bounds.0 || valid_until < prediction_bounds.1 {
        return Ok((
            id,
            result(
                "NOT_EVALUATED",
                &["CLOCK_QUALIFICATION_NOT_VALID_FOR_PREDICTION"],
                "NOT_ASSESSED",
            ),
        ));
    }
    let command = signal(field(case, "command", at)?, &format!("{at}.command"), false)?;
    let context = signal(field(case, "context", at)?, &format!("{at}.context"), false)?;
    let response = signal(
        field(case, "response", at)?,
        &format!("{at}.response"),
        true,
    )?;
    if command.role != "CONTROL_INTENT"
        || context.role != "DISTURBANCE_CONTEXT"
        || response.role != "PHYSICAL_RESPONSE"
    {
        return Ok((
            id,
            result(
                "INVALID_CONTRACT",
                &["SIGNAL_ROLE_MISMATCH"],
                "NOT_ASSESSED",
            ),
        ));
    }
    let feedback = field(case, "actuator_feedback", at)?;
    let feedback_state = if feedback.is_null() {
        "FEEDBACK_UNAVAILABLE"
    } else {
        let fb = signal(feedback, &format!("{at}.actuator_feedback"), false)?;
        if fb.role != "ACTUATOR_FEEDBACK" {
            return Ok((
                id,
                result(
                    "INVALID_CONTRACT",
                    &["ACTUATOR_FEEDBACK_ROLE_MISMATCH"],
                    "FEEDBACK_ROLE_INVALID",
                ),
            ));
        }
        if fb.timebase_id != timebase_id {
            "FEEDBACK_TIMEBASE_UNQUALIFIED"
        } else if fb.quality == "CURRENT" {
            "FEEDBACK_PRESENT_NOT_DELIVERY_PROOF"
        } else {
            "FEEDBACK_NOT_CURRENT"
        }
    };
    let envelope = field(case, "response_envelope", at)?;
    exact_keys(
        envelope,
        &[
            "model_id",
            "configuration_id",
            "regime_id",
            "timebase_id",
            "issued_at_ms",
            "issued_at_uncertainty_ms",
            "trained_through_ms",
            "trained_through_uncertainty_ms",
            "temporal_provenance_id",
            "temporal_provenance_sha256",
            "qualified",
            "lower",
            "mean",
            "upper",
            "unit",
        ],
        &format!("{at}.response_envelope"),
    )?;
    let model_id = string(envelope, "model_id", at)?;
    let envelope_config = string(envelope, "configuration_id", at)?;
    let envelope_regime = string(envelope, "regime_id", at)?;
    let envelope_timebase_id = string(envelope, "timebase_id", at)?;
    let issued = unsigned(envelope, "issued_at_ms", at)?;
    let issued_uncertainty = unsigned(envelope, "issued_at_uncertainty_ms", at)?;
    let trained = unsigned(envelope, "trained_through_ms", at)?;
    let trained_uncertainty = unsigned(envelope, "trained_through_uncertainty_ms", at)?;
    let temporal_provenance_id = string(envelope, "temporal_provenance_id", at)?;
    let temporal_provenance_sha256 = string(envelope, "temporal_provenance_sha256", at)?;
    let model_qualified = boolean(envelope, "qualified", at)?;
    if !valid_hex(temporal_provenance_sha256, 64) {
        return Ok((
            id,
            result(
                "INVALID_CONTRACT",
                &["INVALID_MODEL_TEMPORAL_PROVENANCE_DIGEST"],
                feedback_state,
            ),
        ));
    }
    let lower = number(envelope, "lower", at)?;
    let mean = number(envelope, "mean", at)?;
    let upper = number(envelope, "upper", at)?;
    let envelope_unit = string(envelope, "unit", at)?;
    if lower > mean || mean > upper {
        return Ok((
            id,
            result(
                "INVALID_CONTRACT",
                &["INVALID_RESPONSE_INTERVAL"],
                feedback_state,
            ),
        ));
    }
    if envelope_timebase_id != timebase_id
        || command.timebase_id != timebase_id
        || context.timebase_id != timebase_id
        || response.timebase_id != timebase_id
    {
        return Ok((
            id,
            result(
                "NOT_EVALUATED",
                &["TIMEBASE_MAPPING_NOT_QUALIFIED"],
                feedback_state,
            ),
        ));
    }
    for signal in [&command, &context, &response] {
        if !valid_hex(&signal.timestamp_evidence_sha256, 64) {
            return Ok((
                id,
                result(
                    "INVALID_CONTRACT",
                    &["INVALID_SIGNAL_TIMESTAMP_EVIDENCE_DIGEST"],
                    feedback_state,
                ),
            ));
        }
        if !matches!(
            signal.timestamp_capture_point.as_str(),
            "HARDWARE_CAPTURE"
                | "DAQ_CAPTURE"
                | "CONTROLLER_SCAN"
                | "GATEWAY_RECEIVE"
                | "HOST_RECEIVE"
                | "RECONSTRUCTED"
        ) {
            return Ok((
                id,
                result(
                    "INVALID_CONTRACT",
                    &["INVALID_TIMESTAMP_CAPTURE_POINT"],
                    feedback_state,
                ),
            ));
        }
        if signal.timestamp_uncertainty_ms < signal.qualified_uncertainty_bound_ms {
            return Ok((
                id,
                result(
                    "INVALID_CONTRACT",
                    &["TIMESTAMP_UNCERTAINTY_UNDERSTATES_QUALIFIED_PATH_BOUND"],
                    feedback_state,
                ),
            ));
        }
    }
    if !feedback.is_null() {
        let fb = signal(feedback, &format!("{at}.actuator_feedback"), false)?;
        if fb.timebase_id != timebase_id
            || !valid_hex(&fb.timestamp_evidence_sha256, 64)
            || fb.timestamp_uncertainty_ms < fb.qualified_uncertainty_bound_ms
            || !matches!(
                fb.timestamp_capture_point.as_str(),
                "HARDWARE_CAPTURE"
                    | "DAQ_CAPTURE"
                    | "CONTROLLER_SCAN"
                    | "GATEWAY_RECEIVE"
                    | "HOST_RECEIVE"
                    | "RECONSTRUCTED"
            )
        {
            return Ok((
                id,
                result(
                    "NOT_EVALUATED",
                    &["ACTUATOR_FEEDBACK_TIMING_NOT_QUALIFIED"],
                    feedback_state,
                ),
            ));
        }
    }
    let command_bounds = match time_bounds(command.observed_at_ms, command.timestamp_uncertainty_ms)
    {
        Some(bounds) => bounds,
        None => {
            return Ok((
                id,
                result(
                    "INVALID_CONTRACT",
                    &["COMMAND_TIMESTAMP_INTERVAL_OVERFLOW"],
                    feedback_state,
                ),
            ));
        }
    };
    let context_bounds = match time_bounds(context.observed_at_ms, context.timestamp_uncertainty_ms)
    {
        Some(bounds) => bounds,
        None => {
            return Ok((
                id,
                result(
                    "INVALID_CONTRACT",
                    &["CONTEXT_TIMESTAMP_INTERVAL_OVERFLOW"],
                    feedback_state,
                ),
            ));
        }
    };
    let response_bounds =
        match time_bounds(response.observed_at_ms, response.timestamp_uncertainty_ms) {
            Some(bounds) => bounds,
            None => {
                return Ok((
                    id,
                    result(
                        "INVALID_CONTRACT",
                        &["RESPONSE_TIMESTAMP_INTERVAL_OVERFLOW"],
                        feedback_state,
                    ),
                ));
            }
        };
    let issued_bounds = match time_bounds(issued, issued_uncertainty) {
        Some(bounds) => bounds,
        None => {
            return Ok((
                id,
                result(
                    "INVALID_CONTRACT",
                    &["MODEL_ISSUE_INTERVAL_OVERFLOW"],
                    feedback_state,
                ),
            ));
        }
    };
    let trained_bounds = match time_bounds(trained, trained_uncertainty) {
        Some(bounds) => bounds,
        None => {
            return Ok((
                id,
                result(
                    "INVALID_CONTRACT",
                    &["MODEL_TRAINING_INTERVAL_OVERFLOW"],
                    feedback_state,
                ),
            ));
        }
    };
    for (bounds, label) in [(command_bounds, "COMMAND"), (context_bounds, "CONTEXT")] {
        if bounds.0 > prediction_bounds.1 {
            return Ok((
                id,
                result(
                    "INVALID_CONTRACT",
                    &[if label == "COMMAND" {
                        "FUTURE_COMMAND_INFORMATION"
                    } else {
                        "FUTURE_CONTEXT_INFORMATION"
                    }],
                    feedback_state,
                ),
            ));
        }
        if bounds.1 > prediction_bounds.0 {
            return Ok((
                id,
                result(
                    "NOT_EVALUATED",
                    &[if label == "COMMAND" {
                        "COMMAND_PREDICTION_ORDER_UNPROVEN"
                    } else {
                        "CONTEXT_PREDICTION_ORDER_UNPROVEN"
                    }],
                    feedback_state,
                ),
            ));
        }
    }
    if issued_bounds.0 > prediction_bounds.1 || trained_bounds.0 > prediction_bounds.1 {
        return Ok((
            id,
            result(
                "INVALID_CONTRACT",
                &["FUTURE_MODEL_INFORMATION"],
                feedback_state,
            ),
        ));
    }
    if issued_bounds.1 > prediction_bounds.0 || trained_bounds.1 > prediction_bounds.0 {
        return Ok((
            id,
            result(
                "NOT_EVALUATED",
                &["MODEL_PREDICTION_ORDER_UNPROVEN"],
                feedback_state,
            ),
        ));
    }
    if trained_bounds.0 > issued_bounds.1 {
        return Ok((
            id,
            result(
                "INVALID_CONTRACT",
                &["MODEL_ISSUED_BEFORE_TRAINING_DATA"],
                feedback_state,
            ),
        ));
    }
    if trained_bounds.1 > issued_bounds.0 {
        return Ok((
            id,
            result(
                "NOT_EVALUATED",
                &["MODEL_TRAINING_ISSUE_ORDER_UNPROVEN"],
                feedback_state,
            ),
        ));
    }
    if response_bounds.1 <= prediction_bounds.0 {
        return Ok((
            id,
            result(
                "INVALID_CONTRACT",
                &["RESPONSE_PRECEDES_PREDICTION"],
                feedback_state,
            ),
        ));
    }
    if response_bounds.0 <= prediction_bounds.1 {
        return Ok((
            id,
            result(
                "NOT_EVALUATED",
                &["RESPONSE_PREDICTION_ORDER_UNPROVEN"],
                feedback_state,
            ),
        ));
    }
    if prediction_bounds.1 - command_bounds.0 > max_age
        || prediction_bounds.1 - context_bounds.0 > max_age
    {
        return Ok((
            id,
            result(
                "NOT_EVALUATED",
                &["INPUT_FRESHNESS_NOT_PROVEN"],
                feedback_state,
            ),
        ));
    }
    if response_bounds.1 - prediction_bounds.0 > max_horizon {
        if response_bounds.0 - prediction_bounds.1 > max_horizon {
            return Ok((
                id,
                result(
                    "INVALID_CONTRACT",
                    &["RESPONSE_OUTSIDE_PREDICTION_HORIZON"],
                    feedback_state,
                ),
            ));
        }
        return Ok((
            id,
            result(
                "NOT_EVALUATED",
                &["RESPONSE_HORIZON_NOT_PROVEN"],
                feedback_state,
            ),
        ));
    }
    if response.unit != envelope_unit {
        return Ok((
            id,
            result(
                "INVALID_CONTRACT",
                &["RESPONSE_ENVELOPE_UNIT_MISMATCH"],
                feedback_state,
            ),
        ));
    }
    if field(case, "expected_disposition", at)?.as_str().is_none() {
        return Err(format!("{id}: expected disposition must be a string"));
    }
    if prediction.saturating_sub(command.observed_at_ms) > max_age
        || prediction.saturating_sub(context.observed_at_ms) > max_age
    {
        return Ok((
            id,
            result("NOT_EVALUATED", &["INPUT_TOO_OLD"], feedback_state),
        ));
    }
    if command.quality != "CURRENT" || context.quality != "CURRENT" || response.quality != "CURRENT"
    {
        return Ok((
            id,
            result(
                "NOT_EVALUATED",
                &["INPUT_QUALITY_NOT_CURRENT"],
                feedback_state,
            ),
        ));
    }
    if !response.derived_from.is_empty() {
        return Ok((
            id,
            result(
                "NOT_EVALUATED",
                &["RESPONSE_HAS_DERIVED_PARENTS"],
                feedback_state,
            ),
        ));
    }
    if !regime_qualified || !model_qualified {
        return Ok((
            id,
            result(
                "NOT_EVALUATED",
                &["REGIME_OR_MODEL_NOT_QUALIFIED"],
                feedback_state,
            ),
        ));
    }
    if configuration != envelope_config || regime != envelope_regime {
        return Ok((
            id,
            result(
                "NOT_EVALUATED",
                &["CONFIGURATION_OR_REGIME_MISMATCH"],
                feedback_state,
            ),
        ));
    }
    if overlaps(&command.dependencies, &context.dependencies)
        || overlaps(&command.dependencies, &response.dependencies)
        || overlaps(&context.dependencies, &response.dependencies)
    {
        return Ok((
            id,
            result(
                "NOT_EVALUATED",
                &["SHARED_COMMON_CAUSE_DEPENDENCY"],
                feedback_state,
            ),
        ));
    }
    let disposition = if response.value >= lower && response.value <= upper {
        "RESPONSE_CONSISTENT"
    } else {
        "RESPONSE_MISMATCH_UNISOLATED"
    };
    let reasons: &[&str] = if disposition == "RESPONSE_CONSISTENT" {
        &[]
    } else {
        &["OBSERVED_RESPONSE_OUTSIDE_CONDITIONED_ENVELOPE"]
    };
    let mut out = result(disposition, reasons, feedback_state);
    out["timebase_id"] = json!(timebase_id);
    out["timebase_qualification_id"] = json!(clock_qualification_id);
    out["timebase_qualification_sha256"] = json!(clock_qualification_sha256);
    out["timebase_method"] = json!(sync_method);
    out["time_scale"] = json!(time_scale);
    out["timebase_integrity_state"] = json!(integrity_state);
    out["timebase_measurement_evidence_sha256"] = json!(measurement_sha);
    out["model_temporal_provenance_id"] = json!(temporal_provenance_id);
    out["model_temporal_provenance_sha256"] = json!(temporal_provenance_sha256);
    out["prediction_timestamp_evidence"] = json!({
        "evidence_id":prediction_evidence_id,"sha256":prediction_evidence_sha,
        "capture_point":prediction_capture_point,"qualified_bound_ms":prediction_qualified_bound
    });
    out["signal_timestamp_evidence"] = json!({
        "command": {"evidence_id":command.timestamp_evidence_id,"sha256":command.timestamp_evidence_sha256,"capture_point":command.timestamp_capture_point,"qualified_bound_ms":command.qualified_uncertainty_bound_ms},
        "context": {"evidence_id":context.timestamp_evidence_id,"sha256":context.timestamp_evidence_sha256,"capture_point":context.timestamp_capture_point,"qualified_bound_ms":context.qualified_uncertainty_bound_ms},
        "response": {"evidence_id":response.timestamp_evidence_id,"sha256":response.timestamp_evidence_sha256,"capture_point":response.timestamp_capture_point,"qualified_bound_ms":response.qualified_uncertainty_bound_ms}
    });
    out["temporal_intervals_ms"] = json!({
        "command_available": [command_bounds.0,command_bounds.1],
        "context_observed": [context_bounds.0,context_bounds.1],
        "prediction_issued": [prediction_bounds.0,prediction_bounds.1],
        "response_observed": [response_bounds.0,response_bounds.1],
        "command_age_worst_case": prediction_bounds.1 - command_bounds.0,
        "response_lag_minimum": response_bounds.0 - prediction_bounds.1,
        "response_lag_maximum": response_bounds.1 - prediction_bounds.0
    });
    out["response_envelope_model_id"] = json!(model_id);
    out["observed_response"] = json!(response.value);
    out["expected_response_mean"] = json!(mean);
    out["expected_response_interval"] = json!([lower, upper]);
    out["command_value"] = json!(command.value);
    out["context_value"] = json!(context.value);
    Ok((id, out))
}
fn pair_check(
    pair: &Value,
    outcomes: &BTreeMap<String, Value>,
    cases: &BTreeMap<String, &Value>,
) -> Check<Value> {
    let at = "$.counterfactual_pairs[]";
    let kind = string(pair, "kind", at)?;
    let ids = field(pair, "case_ids", at)?
        .as_array()
        .ok_or_else(|| format!("{at}.case_ids: expected array"))?;
    if ids.len() != 2 {
        return Err(format!("{at}: exactly two case IDs required"));
    }
    let a_id = ids[0]
        .as_str()
        .ok_or_else(|| format!("{at}: case ID must be text"))?;
    let b_id = ids[1]
        .as_str()
        .ok_or_else(|| format!("{at}: case ID must be text"))?;
    let a = *cases
        .get(a_id)
        .ok_or_else(|| format!("{at}: unknown case {a_id}"))?;
    let b = *cases
        .get(b_id)
        .ok_or_else(|| format!("{at}: unknown case {b_id}"))?;
    for id in [a_id, b_id] {
        if outcomes.get(id).and_then(|v| v["disposition"].as_str()) != Some("RESPONSE_CONSISTENT") {
            return Err(format!("{at}: pair member {id} is not response-consistent"));
        }
    }
    let ca = field(a, "command", at)?;
    let cb = field(b, "command", at)?;
    let xa = field(a, "context", at)?;
    let xb = field(b, "context", at)?;
    let ra = field(a, "response", at)?;
    let rb = field(b, "response", at)?;
    let ea = field(a, "response_envelope", at)?;
    let eb = field(b, "response_envelope", at)?;
    let delta = |x: f64, y: f64| (x - y).abs();
    match kind {
        "SAME_COMMAND_DIFFERENT_CONTEXT" => {
            exact_keys(
                pair,
                &[
                    "pair_id",
                    "kind",
                    "case_ids",
                    "minimum_context_delta",
                    "minimum_expected_response_delta",
                    "minimum_observed_response_delta",
                ],
                at,
            )?;
            let same = number(ca, "value", at)? == number(cb, "value", at)?
                && string(ca, "unit", at)? == string(cb, "unit", at)?;
            if !same
                || string(xa, "unit", at)? != string(xb, "unit", at)?
                || delta(number(xa, "value", at)?, number(xb, "value", at)?)
                    < number(pair, "minimum_context_delta", at)?
                || delta(number(ea, "mean", at)?, number(eb, "mean", at)?)
                    < number(pair, "minimum_expected_response_delta", at)?
                || delta(number(ra, "value", at)?, number(rb, "value", at)?)
                    < number(pair, "minimum_observed_response_delta", at)?
            {
                return Err(format!("{at}: same-command/different-context pair failed"));
            }
        }
        "DIFFERENT_COMMAND_SAME_RESPONSE" => {
            exact_keys(
                pair,
                &[
                    "pair_id",
                    "kind",
                    "case_ids",
                    "minimum_command_delta",
                    "maximum_expected_response_delta",
                    "maximum_observed_response_delta",
                ],
                at,
            )?;
            if string(ca, "unit", at)? != string(cb, "unit", at)?
                || string(ra, "unit", at)? != string(rb, "unit", at)?
                || delta(number(ca, "value", at)?, number(cb, "value", at)?)
                    < number(pair, "minimum_command_delta", at)?
                || delta(number(ea, "mean", at)?, number(eb, "mean", at)?)
                    > number(pair, "maximum_expected_response_delta", at)?
                || delta(number(ra, "value", at)?, number(rb, "value", at)?)
                    > number(pair, "maximum_observed_response_delta", at)?
            {
                return Err(format!("{at}: different-command/same-response pair failed"));
            }
        }
        other => return Err(format!("{at}: unknown pair kind {other}")),
    }
    Ok(
        json!({"pair_id":string(pair,"pair_id",at)?,"kind":kind,"status":"PASS","case_ids":[a_id,b_id]}),
    )
}
fn valid_hex(s: &str, n: usize) -> bool {
    s.len() == n
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn run(path: &str, source: &str, digest: &str) -> Check<Value> {
    if !valid_hex(source, 40) || !valid_hex(digest, 64) {
        return Err("invalid source commit or corpus digest".to_owned());
    }
    let raw = fs::read_to_string(path).map_err(|e| format!("read corpus: {e}"))?;
    let corpus: Value = serde_json::from_str(&raw).map_err(|e| format!("parse corpus: {e}"))?;
    exact_keys(
        &corpus,
        &[
            "schema_version",
            "temporal_contract_version",
            "corpus_id",
            "evidence_class",
            "max_input_age_ms",
            "max_response_horizon_ms",
            "cases",
            "counterfactual_pairs",
            "authority_boundary",
            "claim_ceiling",
        ],
        "$",
    )?;
    if string(&corpus, "schema_version", "$")? != VERSION
        || string(&corpus, "temporal_contract_version", "$")? != "bounded-time-interval-v0.1"
        || string(&corpus, "evidence_class", "$")? != "SYNTHETIC_KNOWN_TRUTH"
    {
        return Err("wrong schema version, temporal contract or evidence class".to_owned());
    }
    let authority = field(&corpus, "authority_boundary", "$")?;
    exact_keys(
        authority,
        &[
            "read_only",
            "controller_write",
            "actuation",
            "maintenance_authorization",
            "mode_transition",
            "restriction_clearance",
        ],
        "$.authority_boundary",
    )?;
    if !boolean(authority, "read_only", "$.authority_boundary")? {
        return Err("read_only must be true".to_owned());
    }
    for key in [
        "controller_write",
        "actuation",
        "maintenance_authorization",
        "mode_transition",
        "restriction_clearance",
    ] {
        if boolean(authority, key, "$.authority_boundary")? {
            return Err(format!("authority boundary violation: {key}"));
        }
    }
    let max_age = unsigned(&corpus, "max_input_age_ms", "$")?;
    let max_horizon = unsigned(&corpus, "max_response_horizon_ms", "$")?;
    let items = field(&corpus, "cases", "$")?
        .as_array()
        .ok_or_else(|| "cases must be array".to_owned())?;
    if items.is_empty() || items.len() > 10000 {
        return Err("case count outside 1..=10000".to_owned());
    }
    let mut outcomes = BTreeMap::new();
    let mut case_map = BTreeMap::new();
    for case in items {
        let (id, out) = evaluate_case(case, max_age, max_horizon)?;
        let expected = string(case, "expected_disposition", "$.cases[]")?;
        if out["disposition"].as_str() != Some(expected) {
            return Err(format!(
                "{id}: expected {expected}, got {}",
                out["disposition"]
            ));
        }
        if case_map.insert(id.clone(), case).is_some() {
            return Err(format!("duplicate case ID {id}"));
        }
        outcomes.insert(id, out);
    }
    let pairs = field(&corpus, "counterfactual_pairs", "$")?
        .as_array()
        .ok_or_else(|| "counterfactual_pairs must be array".to_owned())?;
    let mut pair_receipts = Vec::new();
    let mut pair_ids = BTreeSet::new();
    for pair in pairs {
        let id = string(pair, "pair_id", "$.counterfactual_pairs[]")?.to_owned();
        if !pair_ids.insert(id.clone()) {
            return Err(format!("duplicate pair ID {id}"));
        }
        pair_receipts.push(pair_check(pair, &outcomes, &case_map)?);
    }
    let case_results: Vec<Value> = outcomes
        .iter()
        .map(|(id, out)| json!({"case_id":id,"result":out}))
        .collect();
    Ok(json!({"result":"pass","validator_version":VERSION,
        "temporal_contract_version":string(&corpus,"temporal_contract_version","$")?,
        "corpus_id":string(&corpus,"corpus_id","$")?,
        "source_commit":source,"corpus_sha256":digest,"scenario_count":items.len(),
        "counterfactual_pair_count":pair_receipts.len(),"cases":case_results,
        "counterfactual_pairs":pair_receipts,"claim_ceiling":string(&corpus,"claim_ceiling","$")?}))
}
fn main() -> ExitCode {
    let a: Vec<String> = env::args().collect();
    if a.len() != 6 || a[2] != "--source-commit" || a[4] != "--corpus-sha256" {
        eprintln!(
            "usage: evaluator <corpus.json> --source-commit <40-hex> --corpus-sha256 <64-hex>"
        );
        return ExitCode::from(2);
    }
    match run(&a[1], &a[3], &a[5]) {
        Ok(v) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&v)
                    .unwrap_or_else(|_| "{\"result\":\"fail\"}".to_owned())
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("benchmark failed closed: {e}");
            ExitCode::FAILURE
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn corpus() -> Value {
        serde_json::from_str(FROZEN).expect("frozen fixture JSON")
    }
    #[test]
    fn all_twenty_five_synthetic_cases_match_known_truth() {
        let c = corpus();
        let items = c["cases"].as_array().expect("cases");
        assert_eq!(items.len(), 30);
        for case in items {
            let (id, out) = evaluate_case(case, 2000, 5000).expect("case evaluates");
            assert_eq!(out["disposition"], case["expected_disposition"], "{id}");
        }
    }
    #[test]
    fn both_counterfactual_pairs_pass() {
        let c = corpus();
        let mut outcomes = BTreeMap::new();
        let mut cases = BTreeMap::new();
        for case in c["cases"].as_array().unwrap() {
            let (id, out) = evaluate_case(case, 2000, 5000).unwrap();
            outcomes.insert(id.clone(), out);
            cases.insert(id, case);
        }
        let pairs = c["counterfactual_pairs"].as_array().unwrap();
        assert_eq!(pairs.len(), 2);
        for pair in pairs {
            assert_eq!(
                pair_check(pair, &outcomes, &cases).unwrap()["status"],
                "PASS"
            );
        }
    }
    #[test]
    fn future_trained_model_and_command_fail_closed() {
        let mut c = corpus();
        c["cases"][0]["response_envelope"]["trained_through_ms"] = json!(2000);
        assert_eq!(
            evaluate_case(&c["cases"][0], 2000, 5000).unwrap().1["disposition"],
            "INVALID_CONTRACT"
        );
        c = corpus();
        c["cases"][0]["command"]["observed_at_ms"] = json!(2000);
        assert_eq!(
            evaluate_case(&c["cases"][0], 2000, 5000).unwrap().1["disposition"],
            "INVALID_CONTRACT"
        );
    }
    #[test]
    fn timestamp_uncertainty_and_timebase_fail_closed() {
        let c = corpus();
        for index in 16..30 {
            let (_, out) = evaluate_case(&c["cases"][index], 2000, 5000).unwrap();
            assert_eq!(
                out["disposition"], c["cases"][index]["expected_disposition"],
                "case {index}"
            );
        }
        let (_, out) = evaluate_case(&c["cases"][24], 2000, 5000).unwrap();
        assert_eq!(out["disposition"], "RESPONSE_CONSISTENT");
        assert_eq!(
            out["temporal_intervals_ms"]["command_available"],
            json!([997, 999])
        );
    }
    #[test]
    fn timestamp_uncertainty_overflow_is_invalid_not_clamped() {
        let mut c = corpus();
        c["cases"][0]["prediction_time_ms"] = json!(u64::MAX);
        c["cases"][0]["prediction_timestamp_uncertainty_ms"] = json!(1);
        let (_, out) = evaluate_case(&c["cases"][0], 2000, 5000).unwrap();
        assert_eq!(out["disposition"], "INVALID_CONTRACT");
    }
    #[test]
    fn unqualified_clock_never_proves_temporal_order() {
        let mut c = corpus();
        c["cases"][0]["timebase"]["qualification_state"] = json!("UNKNOWN");
        let (_, out) = evaluate_case(&c["cases"][0], 2000, 5000).unwrap();
        assert_eq!(out["disposition"], "NOT_EVALUATED");
        assert_eq!(out["reason_codes"][0], "CLOCK_QUALIFICATION_UNAVAILABLE");
    }
    #[test]
    fn clock_qualification_and_signal_path_evidence_are_enforced() {
        let mut c = corpus();
        c["cases"][25]["timebase"]["valid_until_ms"] = json!(999);
        assert_eq!(
            evaluate_case(&c["cases"][25], 2000, 5000).unwrap().1["disposition"],
            "NOT_EVALUATED"
        );
        c["cases"][26]["timebase"]["qualified_at_ms"] = json!(1200);
        assert_eq!(
            evaluate_case(&c["cases"][26], 2000, 5000).unwrap().1["disposition"],
            "INVALID_CONTRACT"
        );
        c["cases"][27]["timebase"]["integrity_state"] = json!("COMPROMISED");
        assert_eq!(
            evaluate_case(&c["cases"][27], 2000, 5000).unwrap().1["disposition"],
            "INVALID_CONTRACT"
        );
        c["cases"][28]["response"]["timestamp_uncertainty_ms"] = json!(0);
        c["cases"][28]["response"]["qualified_uncertainty_bound_ms"] = json!(1);
        assert_eq!(
            evaluate_case(&c["cases"][28], 2000, 5000).unwrap().1["disposition"],
            "INVALID_CONTRACT"
        );
        c["cases"][29]["response"]["timestamp_evidence_sha256"] = json!("bad");
        assert_eq!(
            evaluate_case(&c["cases"][29], 2000, 5000).unwrap().1["disposition"],
            "INVALID_CONTRACT"
        );
    }
    #[test]
    fn role_promotion_fails_closed() {
        let mut c = corpus();
        c["cases"][0]["response"]["role"] = json!("CONTROL_INTENT");
        assert_eq!(
            evaluate_case(&c["cases"][0], 2000, 5000).unwrap().1["disposition"],
            "INVALID_CONTRACT"
        );
    }
    #[test]
    fn derived_response_is_not_independent() {
        let mut c = corpus();
        c["cases"][0]["response"]["derived_from_signal_ids"] = json!(["fuel_index_command"]);
        assert_eq!(
            evaluate_case(&c["cases"][0], 2000, 5000).unwrap().1["disposition"],
            "NOT_EVALUATED"
        );
    }
    #[test]
    fn common_cause_withdraws_qualification() {
        let mut c = corpus();
        c["cases"][0]["response"]["dependency_groups"] = json!(["controller-power"]);
        assert_eq!(
            evaluate_case(&c["cases"][0], 2000, 5000).unwrap().1["disposition"],
            "NOT_EVALUATED"
        );
    }
    #[test]
    fn command_never_confirms_delivery_or_fault() {
        let c = corpus();
        let out = evaluate_case(&c["cases"][0], 2000, 5000).unwrap().1;
        assert_eq!(out["fault_isolation"], "NOT_ATTEMPTED");
        assert_eq!(out["actuator_delivery_confirmation"], "NOT_ESTABLISHED");
    }
    #[test]
    fn out_of_envelope_is_unisolated() {
        let c = corpus();
        let out = evaluate_case(&c["cases"][5], 2000, 5000).unwrap().1;
        assert_eq!(out["disposition"], "RESPONSE_MISMATCH_UNISOLATED");
        assert_eq!(out["actuator_delivery_confirmation"], "NOT_ESTABLISHED");
    }
}
