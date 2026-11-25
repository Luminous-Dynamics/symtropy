// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Execute the public command-semantic known-answer corpus against the same
//! typed evaluator used by the library. The fixtures are not merely schema
//! examples: every case must produce its declared AcceptProfile or exact
//! typed HoldReason result.

use std::collections::BTreeSet;

use serde::Deserialize;

use crate::{
    CommandSemanticProfile, DeviceCapabilityProfile, SemanticDecision,
    SemanticProfileInput, evaluate,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum ExpectedDecision {
    AcceptProfile,
    Hold,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureCase {
    case_id: String,
    class: String,
    mutation: String,
    expected_decision: ExpectedDecision,
    expected_reason: Option<String>,
    device: DeviceCapabilityProfile,
    command: CommandSemanticProfile,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureCorpus {
    schema_version: String,
    benchmark: String,
    claim_ceiling: String,
    cases: Vec<FixtureCase>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixtureFailure {
    pub case_id: String,
    pub mutation: String,
    pub expected: String,
    pub actual: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixtureReport {
    pub total: usize,
    pub passed: usize,
    pub failures: Vec<FixtureFailure>,
}

impl FixtureReport {
    pub fn is_success(&self) -> bool {
        self.failures.is_empty() && self.total == self.passed
    }
}

/// Parse and execute a v0.2 JSON known-answer corpus with the production
/// reference evaluator. Returns an error for malformed corpus metadata,
/// duplicate case IDs, or invalid data shapes. Individual semantic mismatches
/// are retained as typed report failures.
pub fn evaluate_fixture_corpus(json: &str) -> Result<FixtureReport, String> {
    let corpus: FixtureCorpus =
        serde_json::from_str(json).map_err(|error| format!("fixture parse: {error}"))?;

    if corpus.schema_version != "0.2" {
        return Err(format!(
            "unsupported fixture schema version: {}",
            corpus.schema_version
        ));
    }
    if corpus.benchmark != "RRVR-COMMAND-SEMANTICS" {
        return Err(format!("unexpected benchmark identifier: {}", corpus.benchmark));
    }
    if corpus.claim_ceiling.trim().len() < 30 {
        return Err("fixture claim ceiling is missing or too short".into());
    }
    if corpus.cases.is_empty() {
        return Err("fixture corpus has no cases".into());
    }

    let mut ids = BTreeSet::new();
    let total = corpus.cases.len();
    let mut passed = 0;
    let mut failures = Vec::new();

    for case in corpus.cases {
        if case.case_id.trim().is_empty() || !ids.insert(case.case_id.clone()) {
            return Err(format!("empty or duplicate case id: {:?}", case.case_id));
        }
        if !matches!(case.class.as_str(), "positive" | "negative") {
            return Err(format!("invalid fixture class for {}", case.case_id));
        }
        match (case.class.as_str(), case.expected_decision) {
            ("positive", ExpectedDecision::AcceptProfile)
            | ("negative", ExpectedDecision::Hold) => {}
            _ => {
                return Err(format!(
                    "fixture class and expected decision disagree for {}",
                    case.case_id
                ));
            }
        }

        let input = SemanticProfileInput {
            device: case.device,
            command: case.command,
        };
        let actual = evaluate(&input);

        let (matches, actual_text) = match (case.expected_decision, actual) {
            (ExpectedDecision::AcceptProfile, SemanticDecision::AcceptProfile { effect_class }) => {
                (
                    case.expected_reason.is_none(),
                    format!("AcceptProfile({effect_class:?})"),
                )
            }
            (ExpectedDecision::Hold, SemanticDecision::Hold(reason)) => {
                let actual_reason = format!("{reason:?}");
                (
                    case.expected_reason.as_deref() == Some(actual_reason.as_str()),
                    format!("Hold({actual_reason})"),
                )
            }
            (ExpectedDecision::AcceptProfile, SemanticDecision::Hold(reason)) => {
                (false, format!("Hold({reason:?})"))
            }
            (ExpectedDecision::Hold, SemanticDecision::AcceptProfile { effect_class }) => {
                (false, format!("AcceptProfile({effect_class:?})"))
            }
        };

        if matches {
            passed += 1;
        } else {
            let expected = match case.expected_decision {
                ExpectedDecision::AcceptProfile => "AcceptProfile".to_owned(),
                ExpectedDecision::Hold => format!(
                    "Hold({})",
                    case.expected_reason.as_deref().unwrap_or("<missing reason>")
                ),
            };
            failures.push(FixtureFailure {
                case_id: case.case_id,
                mutation: case.mutation,
                expected,
                actual: actual_text,
            });
        }
    }

    Ok(FixtureReport {
        total,
        passed,
        failures,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CORPUS: &str = include_str!("../fixtures/command_semantics_cases_v0_2.json");

    #[test]
    fn public_known_answer_corpus_executes_against_typed_evaluator() {
        let report = evaluate_fixture_corpus(CORPUS).expect("fixture corpus should parse");
        assert_eq!(report.total, 16);
        assert_eq!(report.passed, 16, "{:#?}", report.failures);
        assert!(report.is_success());
    }

    #[test]
    fn json_schema_covers_every_fixture_field_and_expected_reason() {
        let fixture: serde_json::Value =
            serde_json::from_str(CORPUS).expect("known-answer corpus JSON");
        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../schemas/command_semantics_cases_v0_2.schema.json"
        ))
        .expect("command semantics JSON Schema");

        let schema_cases = schema["$defs"]["case"]["properties"]
            .as_object()
            .expect("case schema properties");
        let schema_device = schema["$defs"]["device"]["properties"]
            .as_object()
            .expect("device schema properties");
        let schema_command = schema["$defs"]["command"]["properties"]
            .as_object()
            .expect("command schema properties");
        let case_required = schema["$defs"]["case"]["required"]
            .as_array()
            .expect("case required fields");
        let device_required = schema["$defs"]["device"]["required"]
            .as_array()
            .expect("device required fields");
        let command_required = schema["$defs"]["command"]["required"]
            .as_array()
            .expect("command required fields");

        for case in fixture["cases"].as_array().expect("fixture cases") {
            let case_object = case.as_object().expect("case object");
            for required in case_required {
                let name = required.as_str().expect("required case field");
                assert!(case_object.contains_key(name), "missing case field {name}");
            }
            for name in case_object.keys() {
                assert!(schema_cases.contains_key(name), "schema lacks case field {name}");
            }

            let device_object = case["device"].as_object().expect("device object");
            for required in device_required {
                let name = required.as_str().expect("required device field");
                assert!(device_object.contains_key(name), "missing device field {name}");
            }
            for name in device_object.keys() {
                assert!(schema_device.contains_key(name), "schema lacks device field {name}");
            }

            let command_object = case["command"].as_object().expect("command object");
            for required in command_required {
                let name = required.as_str().expect("required command field");
                assert!(command_object.contains_key(name), "missing command field {name}");
            }
            for name in command_object.keys() {
                assert!(schema_command.contains_key(name), "schema lacks command field {name}");
            }

            let expected_reason = &case["expected_reason"];
            let allowed_reasons = schema_cases["expected_reason"]["enum"]
                .as_array()
                .expect("expected reason enum");
            assert!(
                allowed_reasons.contains(expected_reason),
                "schema does not enumerate expected reason for {}",
                case["case_id"]
            );
        }
    }

    #[test]
    fn duplicate_case_ids_are_rejected_before_evaluation() {
        let mut value: serde_json::Value =
            serde_json::from_str(CORPUS).expect("known-answer corpus JSON");
        let first = value["cases"][0].clone();
        value["cases"][1]["case_id"] = first["case_id"].clone();
        let changed = serde_json::to_string(&value).expect("serialize mutated fixture");
        let error = evaluate_fixture_corpus(&changed).expect_err("duplicate ID must fail");
        assert!(error.contains("duplicate case id"));
    }

    #[test]
    fn unsupported_fixture_version_is_rejected() {
        let changed = CORPUS.replace("\"schema_version\": \"0.2\"", "\"schema_version\": \"0.1\"");
        let error = evaluate_fixture_corpus(&changed).expect_err("v0.1 is not this corpus contract");
        assert!(error.contains("unsupported fixture schema version"));
    }

    #[test]
    fn expected_hold_reason_is_compared_exactly() {
        let changed = CORPUS.replace(
            "\"expected_reason\": \"DesiredStateRequiresAbsoluteMotion\"",
            "\"expected_reason\": \"OneShotTokenMissing\"",
        );
        let report = evaluate_fixture_corpus(&changed).expect("corpus shape remains valid");
        assert!(!report.is_success());
        assert!(report.failures.iter().any(|failure| failure.case_id == "SEM-005"));
    }
}
