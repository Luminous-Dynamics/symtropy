// SPDX-License-Identifier: AGPL-3.0-or-later
//! Evidence-only join between a validated WPT session event and a registered
//! Power Fabric transport-path revision.
//!
//! The caller MUST resolve the trusted context independently from the
//! validated session stream and authoritative service/path registries. The
//! sidecar is not its own trust anchor. A successful result is metadata
//! agreement only: it never authorizes energization, changes session state,
//! or overrides an independent local protection system.

use serde_json::{Value, json};

fn at<'a>(root: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = root;
    for segment in path {
        current = current.get(*segment)?;
    }
    Some(current)
}

fn text_at<'a>(root: &'a Value, path: &[&str]) -> Option<&'a str> {
    at(root, path)?
        .as_str()
        .filter(|value| !value.trim().is_empty())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn report(disposition: &str, reason: Option<&str>, claimed: Option<&str>) -> Value {
    let reasons: Vec<&str> = reason.into_iter().collect();
    json!({
        "disposition": disposition,
        "reason_codes": reasons,
        "reported_binding_status": claimed,
        "binding_is_evidence_only": true,
        "energization_authority": false,
        "interlock_override": false,
        "claim_ceiling": "metadata agreement only; not physical-path proof, authentication, measurement, safety, permission to energize, standards conformity or performance"
    })
}

fn incomplete(claimed: Option<&str>) -> Value {
    report("UNRESOLVED", Some("BINDING_CONTEXT_INCOMPLETE"), claimed)
}

fn compare_text(
    binding: &Value,
    trusted: &Value,
    path: &[&str],
    reason: &str,
) -> Result<(), Value> {
    let actual = text_at(binding, path);
    let expected = text_at(trusted, path);
    let (Some(actual), Some(expected)) = (actual, expected) else {
        return Err(incomplete(text_at(binding, &["binding_status"])));
    };
    if actual != expected {
        return Err(report(
            "NOT_COMPARABLE",
            Some(reason),
            text_at(binding, &["binding_status"]),
        ));
    }
    Ok(())
}

fn compare_digest(
    binding: &Value,
    trusted: &Value,
    path: &[&str],
    mismatch_reason: &str,
) -> Result<(), Value> {
    let actual = text_at(binding, path);
    let expected = text_at(trusted, path);
    let (Some(actual), Some(expected)) = (actual, expected) else {
        return Err(incomplete(text_at(binding, &["binding_status"])));
    };
    if !valid_sha256(actual) || !valid_sha256(expected) {
        return Err(report(
            "UNRESOLVED",
            Some("INVALID_OR_MISSING_DIGEST"),
            text_at(binding, &["binding_status"]),
        ));
    }
    if actual != expected {
        let disposition = if mismatch_reason == "STALE_TRANSPORT_PATH_REVISION" {
            "UNRESOLVED"
        } else {
            "NOT_COMPARABLE"
        };
        return Err(report(
            disposition,
            Some(mismatch_reason),
            text_at(binding, &["binding_status"]),
        ));
    }
    Ok(())
}

fn compare_json(
    binding: &Value,
    trusted: &Value,
    path: &[&str],
    mismatch_reason: &str,
) -> Result<(), Value> {
    let (Some(actual), Some(expected)) = (at(binding, path), at(trusted, path)) else {
        return Err(incomplete(text_at(binding, &["binding_status"])));
    };
    if actual != expected {
        return Err(report(
            "NOT_COMPARABLE",
            Some(mismatch_reason),
            text_at(binding, &["binding_status"]),
        ));
    }
    Ok(())
}

fn validate_authority(binding: &Value) -> Result<(), Value> {
    let authority = binding.get("authority");
    let valid = authority.is_some_and(|value| {
        value
            .get("binding_is_evidence_only")
            .and_then(Value::as_bool)
            == Some(true)
            && value.get("energization_authority").and_then(Value::as_bool) == Some(false)
            && value.get("interlock_override").and_then(Value::as_bool) == Some(false)
    });
    if valid {
        Ok(())
    } else {
        Err(report(
            "REJECT_SCHEMA",
            Some("NON_AUTHORITY_INVARIANT_VIOLATION"),
            text_at(binding, &["binding_status"]),
        ))
    }
}

fn validate_primitive_chain(binding: &Value, trusted: &Value) -> Result<(), Value> {
    let claimed = text_at(binding, &["binding_status"]);
    let Some(path) = binding.get("transport_path") else {
        return Err(incomplete(claimed));
    };
    let Some(trusted_path) = trusted.get("transport_path") else {
        return Err(incomplete(claimed));
    };
    let Some(primitives) = path.get("primitives").and_then(Value::as_array) else {
        return Err(report(
            "UNRESOLVED",
            Some("PRIMITIVE_EVIDENCE_NOT_ADMISSIBLE"),
            claimed,
        ));
    };
    let Some(trusted_primitives) = trusted_path.get("primitives").and_then(Value::as_array) else {
        return Err(incomplete(claimed));
    };
    if primitives.is_empty() || trusted_primitives.is_empty() {
        return Err(report(
            "UNRESOLVED",
            Some("PRIMITIVE_EVIDENCE_NOT_ADMISSIBLE"),
            claimed,
        ));
    }
    if primitives.len() < trusted_primitives.len() {
        return Err(report(
            "UNRESOLVED",
            Some("MISSING_CONVERSION_EVIDENCE"),
            claimed,
        ));
    }
    if primitives.len() != trusted_primitives.len() {
        return Err(report(
            "NOT_COMPARABLE",
            Some("TRANSPORT_PRIMITIVE_SET_MISMATCH"),
            claimed,
        ));
    }

    let source_port = text_at(path, &["source_port_id"]);
    let useful_port = text_at(path, &["useful_service_port_id"]);
    let first_source = text_at(&primitives[0], &["source_port_id"]);
    let last_destination = primitives
        .last()
        .and_then(|item| text_at(item, &["destination_port_id"]));
    if source_port.is_none()
        || useful_port.is_none()
        || first_source != source_port
        || last_destination != useful_port
    {
        return Err(report(
            "NOT_COMPARABLE",
            Some("PRIMITIVE_CHAIN_DISCONTINUITY"),
            claimed,
        ));
    }

    for (index, primitive) in primitives.iter().enumerate() {
        let Some(primitive_digest) = text_at(primitive, &["primitive_digest"]) else {
            return Err(report(
                "UNRESOLVED",
                Some("INVALID_OR_MISSING_DIGEST"),
                claimed,
            ));
        };
        if !valid_sha256(primitive_digest) {
            return Err(report(
                "UNRESOLVED",
                Some("INVALID_OR_MISSING_DIGEST"),
                claimed,
            ));
        }
        let evidence_class = text_at(primitive, &["evidence_class"]).unwrap_or("Unknown");
        let evidence_id = text_at(primitive, &["evidence_id"]);
        if evidence_class == "Unknown" || evidence_id.is_none() {
            return Err(report(
            "UNRESOLVED",
            Some("PRIMITIVE_EVIDENCE_NOT_ADMISSIBLE"),
            claimed,
        ));
        }
        let Some(source) = text_at(primitive, &["source_port_id"]) else {
            return Err(report(
            "UNRESOLVED",
            Some("PRIMITIVE_EVIDENCE_NOT_ADMISSIBLE"),
            claimed,
        ));
        };
        let Some(destination) = text_at(primitive, &["destination_port_id"]) else {
            return Err(report(
            "UNRESOLVED",
            Some("PRIMITIVE_EVIDENCE_NOT_ADMISSIBLE"),
            claimed,
        ));
        };
        if index > 0 {
            let prior_destination =
                text_at(&primitives[index - 1], &["destination_port_id"]).unwrap_or("");
            if prior_destination != source {
                return Err(report(
            "NOT_COMPARABLE",
            Some("PRIMITIVE_CHAIN_DISCONTINUITY"),
            claimed,
        ));
            }
        }
        if destination.trim().is_empty() {
            return Err(report(
            "UNRESOLVED",
            Some("PRIMITIVE_EVIDENCE_NOT_ADMISSIBLE"),
            claimed,
        ));
        }

        let Some(expected_stages) = trusted_primitives[index]
            .get("conversion_stages")
            .and_then(Value::as_array)
        else {
            return Err(incomplete(claimed));
        };
        let Some(actual_stages) = primitive.get("conversion_stages").and_then(Value::as_array)
        else {
            return Err(report(
            "UNRESOLVED",
            Some("MISSING_CONVERSION_EVIDENCE"),
            claimed,
        ));
        };
        if expected_stages.iter().any(|stage| !actual_stages.contains(stage)) {
            return Err(report(
            "UNRESOLVED",
            Some("MISSING_CONVERSION_EVIDENCE"),
            claimed,
        ));
        }
    }

    if primitives != trusted_primitives {
        return Err(report(
            "NOT_COMPARABLE",
            Some("TRANSPORT_PRIMITIVE_MISMATCH"),
            claimed,
        ));
    }
    Ok(())
}

/// Recompute the metadata-binding disposition against an independently
/// resolved trusted context. The context has the same session_event, assets,
/// service, transport_path, measurement_profile_digest, and
/// calibration_receipt_digests fields as the sidecar, but it MUST be built
/// from the accepted WPT event plus authoritative profile/path registries.
pub fn evaluate_transport_session_binding(binding: &Value, trusted: &Value) -> Value {
    let claimed = text_at(binding, &["binding_status"]);
    if text_at(binding, &["schema_version"]) != Some("0.1.0") {
        return report("REJECT_SCHEMA", Some("UNSUPPORTED_BINDING_SCHEMA"), claimed);
    }
    if text_at(binding, &["binding_id"]).is_none() {
        return incomplete(claimed);
    }
    if let Err(result) = validate_authority(binding) {
        return result;
    }

    if let Err(result) = compare_text(
        binding,
        trusted,
        &["session_event", "session_id"],
        "SESSION_EVENT_IDENTITY_MISMATCH",
    ) {
        return result;
    }
    if let Err(result) = compare_text(
        binding,
        trusted,
        &["session_event", "event_id"],
        "SESSION_EVENT_IDENTITY_MISMATCH",
    ) {
        return result;
    }
    if let Err(result) = compare_digest(
        binding,
        trusted,
        &["session_event", "event_digest"],
        "SESSION_EVENT_DIGEST_MISMATCH",
    ) {
        return result;
    }

    if let Err(result) = compare_text(
        binding,
        trusted,
        &["assets", "source_id"],
        "SOURCE_IDENTITY_MISMATCH",
    ) {
        return result;
    }
    for field in [
        "source_capability_digest",
        "source_firmware_digest",
        "source_topology_digest",
    ] {
        if let Err(result) = compare_digest(
            binding,
            trusted,
            &["assets", field],
            "SOURCE_IDENTITY_MISMATCH",
        ) {
            return result;
        }
    }
    if let Err(result) = compare_text(
        binding,
        trusted,
        &["assets", "receiver_id"],
        "RECEIVER_IDENTITY_MISMATCH",
    ) {
        return result;
    }
    for field in ["receiver_capability_digest", "receiver_firmware_digest"] {
        if let Err(result) = compare_digest(
            binding,
            trusted,
            &["assets", field],
            "RECEIVER_IDENTITY_MISMATCH",
        ) {
            return result;
        }
    }

    if let Err(result) = compare_text(
        binding,
        trusted,
        &["service", "service_type"],
        "SERVICE_PROFILE_MISMATCH",
    ) {
        return result;
    }
    for field in [
        "service_profile_digest",
        "quality_profile_digest",
        "duty_cycle_digest",
        "scenario_digest",
    ] {
        if let Err(result) = compare_digest(
            binding,
            trusted,
            &["service", field],
            "SERVICE_PROFILE_MISMATCH",
        ) {
            return result;
        }
    }
    if let Err(result) = compare_text(
        binding,
        trusted,
        &["service", "source_input_boundary_id"],
        "SOURCE_BOUNDARY_MISMATCH",
    ) {
        return result;
    }
    if let Err(result) = compare_digest(
        binding,
        trusted,
        &["service", "source_boundary_profile_digest"],
        "SOURCE_BOUNDARY_MISMATCH",
    ) {
        return result;
    }
    if let Err(result) = compare_text(
        binding,
        trusted,
        &["service", "useful_output_boundary_id"],
        "USEFUL_OUTPUT_BOUNDARY_MISMATCH",
    ) {
        return result;
    }
    if let Err(result) = compare_digest(
        binding,
        trusted,
        &["service", "useful_boundary_profile_digest"],
        "USEFUL_OUTPUT_BOUNDARY_MISMATCH",
    ) {
        return result;
    }

    if let Err(result) = compare_text(
        binding,
        trusted,
        &["transport_path", "path_id"],
        "TRANSPORT_PATH_IDENTITY_MISMATCH",
    ) {
        return result;
    }
    if let Err(result) = compare_text(
        binding,
        trusted,
        &["transport_path", "path_revision_id"],
        "STALE_TRANSPORT_PATH_REVISION",
    ) {
        return result;
    }
    for field in ["path_digest", "operating_regime_profile_digest"] {
        if let Err(result) = compare_digest(
            binding,
            trusted,
            &["transport_path", field],
            "STALE_TRANSPORT_PATH_REVISION",
        ) {
            return result;
        }
    }
    if let Err(result) = compare_text(
        binding,
        trusted,
        &["transport_path", "source_port_id"],
        "PRIMITIVE_CHAIN_DISCONTINUITY",
    ) {
        return result;
    }
    if let Err(result) = compare_text(
        binding,
        trusted,
        &["transport_path", "useful_service_port_id"],
        "PRIMITIVE_CHAIN_DISCONTINUITY",
    ) {
        return result;
    }

    let source_boundary = text_at(binding, &["service", "source_input_boundary_id"]);
    let path_source = text_at(binding, &["transport_path", "source_port_id"]);
    if source_boundary.is_none() || source_boundary != path_source {
        return report("NOT_COMPARABLE", Some("SOURCE_BOUNDARY_MISMATCH"), claimed);
    }
    let useful_boundary = text_at(binding, &["service", "useful_output_boundary_id"]);
    let path_useful = text_at(binding, &["transport_path", "useful_service_port_id"]);
    if useful_boundary.is_none() || useful_boundary != path_useful {
        return report(
            "NOT_COMPARABLE",
            Some("USEFUL_OUTPUT_BOUNDARY_MISMATCH"),
            claimed,
        );
    }

    if let Err(result) = validate_primitive_chain(binding, trusted) {
        return result;
    }
    if let Err(result) = compare_json(
        binding,
        trusted,
        &["measurement_profile_digest"],
        "MEASUREMENT_PROFILE_MISMATCH",
    ) {
        return result;
    }
    if let Err(result) = compare_json(
        binding,
        trusted,
        &["calibration_receipt_digests"],
        "MEASUREMENT_PROFILE_MISMATCH",
    ) {
        return result;
    }
    if let Some(digest) = text_at(binding, &["measurement_profile_digest"]) {
        if !valid_sha256(digest) {
            return report("UNRESOLVED", Some("INVALID_OR_MISSING_DIGEST"), claimed);
        }
    } else {
        return incomplete(claimed);
    }

    report("METADATA_BOUND", None, claimed)
}

#[cfg(test)]
mod tests {
    use super::evaluate_transport_session_binding;
    use serde_json::{Value, json};

    const BINDING: &str = include_str!(
        "../../../benchmarks/wpt-energy-interface/v1/transport-session-binding.example.json"
    );
    const CONTEXT: &str = include_str!(
        "../../../benchmarks/wpt-energy-interface/v1/transport-session-binding-context.example.json"
    );
    const CONTROLS: &str = include_str!(
        "../../../benchmarks/wpt-energy-interface/v1/transport-session-binding-negative-controls.v0.1.jsonl"
    );

    fn fixtures() -> (Value, Value) {
        (
            serde_json::from_str(BINDING).expect("valid binding fixture"),
            serde_json::from_str(CONTEXT).expect("valid trusted context fixture"),
        )
    }

    fn mutate(binding: &mut Value, scenario: &str) {
        match scenario {
            "event-digest-mismatch" => {
                binding["session_event"]["event_digest"] = Value::String("0".repeat(64));
            }
            "source-identity-mismatch" => {
                binding["assets"]["source_id"] = Value::String("other-source".into());
            }
            "receiver-identity-mismatch" => {
                binding["assets"]["receiver_id"] = Value::String("other-receiver".into());
            }
            "source-input-boundary-mismatch" => {
                binding["service"]["source_input_boundary_id"] =
                    Value::String("other-source-boundary".into());
            }
            "useful-output-boundary-mismatch" => {
                binding["service"]["useful_output_boundary_id"] =
                    Value::String("other-useful-boundary".into());
            }
            "stale-transport-path-digest" => {
                binding["transport_path"]["path_digest"] = Value::String("0".repeat(64));
            }
            "discontinuous-primitive-chain" => {
                binding["transport_path"]["primitives"][1]["source_port_id"] =
                    Value::String("disconnected-port".into());
            }
            "missing-conversion-stage" => {
                binding["transport_path"]["primitives"][1]["conversion_stages"] = json!([]);
            }
            "unknown-primitive-evidence" => {
                binding["transport_path"]["primitives"][0]["evidence_class"] =
                    Value::String("Unknown".into());
                binding["transport_path"]["primitives"][0]["evidence_id"] = Value::Null;
            }
            "authority-escalation-attempt" => {
                binding["authority"]["energization_authority"] = Value::Bool(true);
            }
            other => panic!("unknown frozen scenario: {other}"),
        }
    }

    #[test]
    fn accepts_only_exact_metadata_binding_to_independent_context() {
        let (binding, context) = fixtures();
        let result = evaluate_transport_session_binding(&binding, &context);
        assert_eq!(result["disposition"], "METADATA_BOUND");
        assert_eq!(result["reason_codes"], json!([]));
        assert_eq!(result["energization_authority"], false);
        assert_eq!(result["interlock_override"], false);
    }

    #[test]
    fn executes_all_frozen_session_binding_negative_controls() {
        let (_, context) = fixtures();
        let mut count = 0usize;
        for line in CONTROLS.lines().filter(|line| !line.trim().is_empty()) {
            let control: Value = serde_json::from_str(line).expect("valid negative control");
            let mut binding: Value = serde_json::from_str(BINDING).expect("valid binding");
            mutate(
                &mut binding,
                control["scenario_id"].as_str().expect("scenario id"),
            );
            let result = evaluate_transport_session_binding(&binding, &context);
            assert_eq!(
                result["disposition"], control["expected_disposition"],
                "scenario {}",
                control["scenario_id"]
            );
            assert_eq!(
                result["reason_codes"][0], control["expected_reason_codes"][0],
                "scenario {}",
                control["scenario_id"]
            );
            assert_eq!(result["energization_authority"], false);
            assert_eq!(result["interlock_override"], false);
            count += 1;
        }
        assert_eq!(count, 10, "all ten frozen controls must execute");
    }

    #[test]
    fn missing_trusted_path_is_unresolved_not_self_authorized() {
        let (binding, mut context) = fixtures();
        context
            .as_object_mut()
            .expect("object")
            .remove("transport_path");
        let result = evaluate_transport_session_binding(&binding, &context);
        assert_eq!(result["disposition"], "UNRESOLVED");
        assert_eq!(result["energization_authority"], false);
    }

    #[test]
    fn malformed_authority_is_rejected_even_when_all_metadata_matches() {
        let (mut binding, context) = fixtures();
        binding["authority"]["interlock_override"] = Value::Bool(true);
        let result = evaluate_transport_session_binding(&binding, &context);
        assert_eq!(result["disposition"], "REJECT_SCHEMA");
        assert_eq!(
            result["reason_codes"][0],
            "NON_AUTHORITY_INVARIANT_VIOLATION"
        );
    }
}
