//! Standards-conforming JSON Schema Draft 2020-12 validation for transition cases.
//!
//! This structural layer is intentionally separate from semantic cross-record
//! checks in lib.rs. It validates each case against the committed schema using
//! a pinned standards implementation; it does not prove physical semantics.

use serde_json::Value;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaValidationIssue {
    pub location: String,
    pub message: String,
}

impl fmt::Display for SchemaValidationIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.location, self.message)
    }
}

/// Validate the fixture case array using an explicit JSON Schema Draft 2020-12
/// validator. The validator is compiled once and reused for all cases.
pub fn validate_fixture_schema(
    schema: &Value,
    fixture: &Value,
) -> Result<usize, Vec<SchemaValidationIssue>> {
    let validator = jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| {
            vec![SchemaValidationIssue {
                location: "$schema".to_owned(),
                message: format!("invalid Draft 2020-12 schema: {error}"),
            }]
        })?;

    let cases = fixture
        .get("cases")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            vec![SchemaValidationIssue {
                location: "$.cases".to_owned(),
                message: "fixture must contain a cases array".to_owned(),
            }]
        })?;

    let mut errors = Vec::new();
    for (index, case) in cases.iter().enumerate() {
        for error in validator.iter_errors(case) {
            errors.push(SchemaValidationIssue {
                location: format!("$.cases[{index}]{}", error.instance_path()),
                message: error.to_string(),
            });
        }
    }

    if errors.is_empty() {
        Ok(cases.len())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_schema() -> Value {
        serde_json::from_str(include_str!(
            "../../../docs/research/navigation-transition-v0.1/nav_transition_case.schema.json"
        ))
        .expect("committed JSON Schema parses")
    }

    fn parse_fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../docs/research/navigation-transition-v0.1/fixtures/nav_transition_cases.v0.1.json"
        ))
        .expect("committed case fixture parses")
    }

    #[test]
    fn committed_fourteen_case_corpus_passes_draft_2020_12() {
        let schema = parse_schema();
        let fixture = parse_fixture();
        assert_eq!(validate_fixture_schema(&schema, &fixture).unwrap(), 14);
    }

    #[test]
    fn unknown_fault_kind_is_rejected_by_schema_enum() {
        let schema = parse_schema();
        let mut fixture = parse_fixture();
        fixture["cases"][0]["faults"][0]["fault_kind"] =
            Value::String("not_a_declared_fault".to_owned());
        let errors = validate_fixture_schema(&schema, &fixture).unwrap_err();
        assert!(errors.iter().any(|error| {
            error.location.contains("fault_kind") || error.message.contains("not_a_declared_fault")
        }));
    }

    #[test]
    fn missing_required_case_identity_is_rejected() {
        let schema = parse_schema();
        let mut fixture = parse_fixture();
        fixture["cases"][0]
            .as_object_mut()
            .unwrap()
            .remove("case_id");
        let errors = validate_fixture_schema(&schema, &fixture).unwrap_err();
        assert!(errors.iter().any(|error| {
            error.message.contains("case_id") || error.location.contains("case_id")
        }));
    }
}
