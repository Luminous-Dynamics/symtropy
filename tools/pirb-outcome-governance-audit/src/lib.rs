// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::Value;
#[cfg(test)]
use serde_json::json;

const SCHEMA: &str =
    include_str!("../../../docs/schemas/peacebuilding-outcome-governance-v0.2.schema.json");
const EXAMPLE: &str =
    include_str!("../../../docs/fixtures/peacebuilding-outcome-governance-v0.2.json");

fn parsed_schema() -> Value {
    serde_json::from_str(SCHEMA).expect("checked-in v0.2 JSON Schema must parse")
}

fn example_contract() -> Value {
    serde_json::from_str(EXAMPLE).expect("checked-in v0.2 example must parse")
}

fn state_implied_by_event(event: &Value) -> Option<&'static str> {
    match event.get("event_type").and_then(Value::as_str) {
        Some("proposed") => Some("proposed"),
        Some("approved") => Some("approved"),
        Some("activated") => Some("active"),
        Some("review_required") => Some("review_required"),
        Some("blocked") => Some("blocked"),
        Some("suspended") => Some("suspended"),
        Some("superseded") => Some("superseded"),
        Some("retired") => Some("retired"),
        Some("challenged")
            if event.get("decision").and_then(Value::as_str) == Some("review_required") =>
        {
            Some("review_required")
        }
        _ => None,
    }
}

fn semantic_errors(contract: &Value) -> Vec<String> {
    let mut findings = Vec::new();
    let Some(version) = contract.get("version").and_then(Value::as_u64) else {
        return vec!["metric version must be a positive integer".to_owned()];
    };
    let Some(events) = contract.get("governance_events").and_then(Value::as_array) else {
        return vec!["governance_events must be an array".to_owned()];
    };

    let mut seen_event_ids = std::collections::BTreeSet::new();
    let mut previous_time: Option<chrono::DateTime<chrono::FixedOffset>> = None;
    let mut current_version_events = Vec::new();

    for (index, event) in events.iter().enumerate() {
        let event_path = format!("$.governance_events[{index}]");
        let event_id = event.get("event_id").and_then(Value::as_str);
        match event_id {
            Some(id) if seen_event_ids.insert(id.to_owned()) => {}
            Some(_) => findings.push(format!("{event_path}.event_id is duplicated")),
            None => findings.push(format!("{event_path}.event_id is missing")),
        }

        let event_version = event.get("metric_version").and_then(Value::as_u64);
        match event_version {
            Some(event_version) if event_version > 0 && event_version <= version => {}
            _ => findings.push(format!(
                "{event_path}.metric_version must be positive and cannot exceed the current metric version"
            )),
        }
        if event_version == Some(version) {
            current_version_events.push(event);
        }

        let event_time = event.get("event_at").and_then(Value::as_str);
        let parsed_time =
            event_time.and_then(|time| chrono::DateTime::parse_from_rfc3339(time).ok());
        match parsed_time {
            Some(event_time) => {
                if previous_time.as_ref().is_some_and(|previous| {
                    event_time.timestamp_millis() < previous.timestamp_millis()
                }) {
                    findings.push(format!(
                        "{event_path}.event_at moves backward; governance event history must be chronological"
                    ));
                }
                previous_time = Some(event_time);
            }
            None => findings.push(format!(
                "{event_path}.event_at must be a parseable RFC 3339 timestamp"
            )),
        }
    }

    if contract.get("status").and_then(Value::as_str) == Some("active") {
        let approved_at = current_version_events
            .iter()
            .position(|event| event.get("event_type").and_then(Value::as_str) == Some("approved"));
        let activated_at = current_version_events
            .iter()
            .position(|event| event.get("event_type").and_then(Value::as_str) == Some("activated"));
        match (approved_at, activated_at) {
            (Some(approved), Some(activated)) if approved < activated => {}
            _ => findings.push(
                "active metric version must contain an approval event before its activation event"
                    .to_owned(),
            ),
        }
    }

    let declared_status = contract.get("status").and_then(Value::as_str);
    let implied_status = current_version_events
        .iter()
        .rev()
        .find_map(|event| state_implied_by_event(event));
    if let Some(implied_status) = implied_status
        && declared_status != Some(implied_status)
    {
        findings.push(format!(
            "declared status {declared_status:?} disagrees with final governance event state {implied_status:?}"
        ));
    }

    findings
}

/// Validate the checked-in schema against JSON Schema Draft 2020-12.
pub fn validate_schema_document() -> Result<(), String> {
    let schema = parsed_schema();
    jsonschema::draft202012::meta::validate(&schema)
        .map_err(|error| format!("Draft 2020-12 schema meta-validation failed: {error}"))
}

/// Validate one metric contract structurally and against governance-event invariants.
pub fn validate_metric_contract(contract: &Value) -> Result<(), Vec<String>> {
    if let Err(error) = validate_schema_document() {
        return Err(vec![error]);
    }

    let mut findings = validation_errors(contract);
    if findings.is_empty() {
        findings.extend(semantic_errors(contract));
    }
    if findings.is_empty() {
        Ok(())
    } else {
        Err(findings)
    }
}

/// Validate the checked-in synthetic example through the same public API consumers use.
pub fn validate_example_metric() -> Result<(), Vec<String>> {
    validate_metric_contract(&example_contract())
}

fn validation_errors(instance: &Value) -> Vec<String> {
    let validator = jsonschema::draft202012::new(&parsed_schema())
        .expect("checked-in v0.2 schema must compile");
    validator
        .iter_errors(instance)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect()
}

#[test]
fn example_governance_history_is_semantically_consistent() {
    let errors = semantic_errors(&example_contract());
    assert!(
        errors.is_empty(),
        "governance history rejected: {errors:#?}"
    );
}

#[test]
fn duplicate_governance_event_ids_are_rejected() {
    let mut value = example_contract();
    let duplicate_id = value["governance_events"][2]["event_id"].clone();
    value["governance_events"][3]["event_id"] = duplicate_id;
    let errors = semantic_errors(&value);
    assert!(
        errors
            .iter()
            .any(|error| error.contains("event_id is duplicated"))
    );
}

#[test]
fn governance_event_history_must_be_chronological() {
    let mut value = example_contract();
    value["governance_events"][3]["event_at"] = json!("2026-09-20T00:00:00Z");
    let errors = semantic_errors(&value);
    assert!(errors.iter().any(|error| error.contains("moves backward")));
}

#[test]
fn active_metric_requires_current_version_activation_event() {
    let mut value = example_contract();
    value["governance_events"].as_array_mut().unwrap().pop();
    let errors = semantic_errors(&value);
    assert!(
        errors
            .iter()
            .any(|error| error.contains("approval event before its activation event"))
    );
}

#[test]
fn future_version_events_are_rejected() {
    let mut value = example_contract();
    value["governance_events"][3]["metric_version"] = json!(2);
    let errors = semantic_errors(&value);
    assert!(
        errors
            .iter()
            .any(|error| error.contains("cannot exceed the current metric version"))
    );
}

#[test]
fn challenge_after_activation_prevents_active_status_from_remaining_silent() {
    let mut value = example_contract();
    value["governance_events"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "event_id": "event-challenged-v1",
            "event_type": "challenged",
            "metric_version": 1,
            "event_at": "2026-10-02T00:00:00Z",
            "actor_authority_reference": "synthetic:community-reviewer",
            "evidence_references": ["synthetic:challenge-record:v1"],
            "decision": "review_required"
        }));
    let errors = semantic_errors(&value);
    assert!(
        errors
            .iter()
            .any(|error| error.contains("status") && error.contains("disagrees"))
    );

    value["status"] = json!("review_required");
    let review_status_errors = semantic_errors(&value);
    assert!(
        !review_status_errors
            .iter()
            .any(|error| error.contains("status") && error.contains("disagrees"))
    );
}

#[test]
fn public_validator_accepts_checked_in_example() {
    assert!(validate_example_metric().is_ok());
}

#[test]
fn public_validator_rejects_lifecycle_and_history_mutations() {
    let mut value = example_contract();
    value["status"] = json!("active");
    value["approval"]["state"] = json!("contested");
    assert!(validate_metric_contract(&value).is_err());
}

#[test]
fn schema_validates_against_draft_2020_12_meta_schema() {
    let schema = parsed_schema();
    let result = jsonschema::draft202012::meta::validate(&schema);
    assert!(result.is_ok(), "schema meta-validation failed: {result:?}");
}

#[test]
fn checked_in_synthetic_example_validates() {
    let errors = validation_errors(&example_contract());
    assert!(errors.is_empty(), "example rejected: {errors:#?}");
}

#[test]
fn active_metric_requires_approved_authority_and_timestamp() {
    let mut value = example_contract();
    value["approval"]["state"] = json!("contested");
    let errors = validation_errors(&value);
    assert!(
        !errors.is_empty(),
        "active metric cannot use contested approval"
    );
}

#[test]
fn active_metric_requires_assessed_geographic_scope() {
    let mut value = example_contract();
    value["geographic_scope"]["coverage_complete"] = json!(false);
    let errors = validation_errors(&value);
    assert!(
        !errors.is_empty(),
        "active metric cannot hide unassessed scope"
    );
}

#[test]
fn post_observation_amendment_requires_prior_version_and_reason() {
    let mut value = example_contract();
    value["version_control"]["post_observation_amendment"] = json!(true);
    value["version_control"]["outcomes_observed"] = json!(false);
    value["version_control"]["predecessor_version"] = Value::Null;
    value["version_control"]["amendment_reason"] = Value::Null;
    let errors = validation_errors(&value);
    assert!(
        !errors.is_empty(),
        "post-observation change must be versioned and explained"
    );
}

#[test]
fn second_metric_version_requires_predecessor() {
    let mut value = example_contract();
    value["version"] = json!(2);
    value["version_control"]["predecessor_version"] = Value::Null;
    let errors = validation_errors(&value);
    assert!(!errors.is_empty(), "version 2 cannot omit predecessor");
}

#[test]
fn person_level_sensitive_fields_are_rejected() {
    let mut value = example_contract();
    value["political_affiliation"] = json!("not allowed");
    let errors = validation_errors(&value);
    assert!(
        !errors.is_empty(),
        "unknown identity-bearing fields must be rejected"
    );
}

#[test]
fn unknown_metric_status_is_rejected() {
    let mut value = example_contract();
    value["status"] = json!("success");
    let errors = validation_errors(&value);
    assert!(
        !errors.is_empty(),
        "unregistered lifecycle states must be rejected"
    );
}

#[test]
fn malformed_schema_fails_meta_validation() {
    let mut schema = parsed_schema();
    schema["properties"]["status"]["enum"] = json!("active");
    assert!(
        jsonschema::draft202012::meta::validate(&schema).is_err(),
        "invalid JSON Schema must fail closed"
    );
}

#[test]
fn active_metric_requires_preregistration_reference() {
    let mut value = example_contract();
    value["version_control"]["preregistration_reference"] = Value::Null;
    let errors = validation_errors(&value);
    assert!(
        !errors.is_empty(),
        "active metric must bind to preregistration"
    );
}
