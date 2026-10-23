use serde_json::Value;
use shipyard_benchmark_audit::{audit_report_json, sha256_hex};
use std::fmt::Display;

const MANIFEST_SCHEMA: &str =
    include_str!("../../shipyard-benchmark-contract/benchmark-manifest.schema.json");
const REPORT_SCHEMA: &str =
    include_str!("../../shipyard-benchmark-contract/run-report.schema.json");

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchemaDiagnostic {
    pub source: String,
    pub instance_path: String,
    pub message: String,
}

impl Display for SchemaDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.instance_path.is_empty() {
            write!(f, "{}: {}", self.source, self.message)
        } else {
            write!(
                f,
                "{} at {}: {}",
                self.source, self.instance_path, self.message
            )
        }
    }
}

pub fn bundled_manifest_schema() -> &'static str {
    MANIFEST_SCHEMA
}

pub fn bundled_report_schema() -> &'static str {
    REPORT_SCHEMA
}

/// Parse and validate a schema document against the Draft 2020-12 meta-schema.
/// The engine is explicitly draft-pinned; no automatic draft inference occurs.
pub fn validate_schema_document(schema_text: &str) -> Result<Value, Vec<SchemaDiagnostic>> {
    let schema: Value = serde_json::from_str(schema_text).map_err(|error| {
        vec![SchemaDiagnostic {
            source: "schema-json".to_owned(),
            instance_path: String::new(),
            message: error.to_string(),
        }]
    })?;

    if schema.get("$schema").and_then(Value::as_str)
        != Some("https://json-schema.org/draft/2020-12/schema")
    {
        return Err(vec![SchemaDiagnostic {
            source: "schema-draft".to_owned(),
            instance_path: "/$schema".to_owned(),
            message: "expected the exact Draft 2020-12 meta-schema URI".to_owned(),
        }]);
    }

    if let Err(error) = jsonschema::draft202012::meta::validate(&schema) {
        return Err(vec![SchemaDiagnostic {
            source: "schema-meta-validation".to_owned(),
            instance_path: error.schema_path().to_string(),
            message: error.to_string(),
        }]);
    }

    Ok(schema)
}

/// Validate one JSON instance against one Draft 2020-12 schema.
/// offline() forbids resolution of unavailable external references.
pub fn validate_instance_value(
    schema: &Value,
    instance: &Value,
) -> Result<(), Vec<SchemaDiagnostic>> {
    let validator = jsonschema::draft202012::options()
        .offline()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| {
            vec![SchemaDiagnostic {
                source: "schema-build".to_owned(),
                instance_path: error.schema_path().to_string(),
                message: error.to_string(),
            }]
        })?;

    let errors: Vec<SchemaDiagnostic> = validator
        .iter_errors(instance)
        .map(|error| SchemaDiagnostic {
            source: "instance-validation".to_owned(),
            instance_path: error.instance_path().to_string(),
            message: error.to_string(),
        })
        .collect();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn validate_instance_text(
    schema_text: &str,
    instance_text: &str,
) -> Result<(), Vec<SchemaDiagnostic>> {
    let schema = validate_schema_document(schema_text)?;
    let instance: Value = serde_json::from_str(instance_text).map_err(|error| {
        vec![SchemaDiagnostic {
            source: "instance-json".to_owned(),
            instance_path: String::new(),
            message: error.to_string(),
        }]
    })?;
    validate_instance_value(&schema, &instance)
}

/// Schema-validate both documents, then run the contract-specific semantic
/// audit. The semantic layer retains duplicate-key rejection, exact manifest
/// digest linkage, oracle truth consistency, and recomputed summary arithmetic.
pub fn audit_benchmark_documents(
    manifest_text: &str,
    report_text: &str,
) -> Result<(), Vec<SchemaDiagnostic>> {
    let manifest_schema = validate_schema_document(MANIFEST_SCHEMA)?;
    let report_schema = validate_schema_document(REPORT_SCHEMA)?;
    let manifest: Value = serde_json::from_str(manifest_text).map_err(|error| {
        vec![SchemaDiagnostic {
            source: "manifest-json".to_owned(),
            instance_path: String::new(),
            message: error.to_string(),
        }]
    })?;
    let report: Value = serde_json::from_str(report_text).map_err(|error| {
        vec![SchemaDiagnostic {
            source: "report-json".to_owned(),
            instance_path: String::new(),
            message: error.to_string(),
        }]
    })?;

    let mut schema_errors = Vec::new();
    if let Err(errors) = validate_instance_value(&manifest_schema, &manifest) {
        schema_errors.extend(errors.into_iter().map(|mut error| {
            error.source = "manifest-schema".to_owned();
            error
        }));
    }
    if let Err(errors) = validate_instance_value(&report_schema, &report) {
        schema_errors.extend(errors.into_iter().map(|mut error| {
            error.source = "report-schema".to_owned();
            error
        }));
    }
    if !schema_errors.is_empty() {
        schema_errors.sort();
        schema_errors.dedup();
        return Err(schema_errors);
    }

    let semantic_errors = audit_report_json(manifest_text, report_text);
    if !semantic_errors.is_empty() {
        let mut diagnostics: Vec<SchemaDiagnostic> = semantic_errors
            .into_iter()
            .map(|message| SchemaDiagnostic {
                source: "semantic-audit".to_owned(),
                instance_path: String::new(),
                message,
            })
            .collect();
        diagnostics.sort();
        diagnostics.dedup();
        return Err(diagnostics);
    }

    Ok(())
}

pub fn manifest_sha256(manifest_text: &str) -> String {
    sha256_hex(manifest_text.as_bytes())
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn embedded_contract_schemas_pass_the_draft_2020_12_meta_schema() {
        validate_schema_document(MANIFEST_SCHEMA)
            .expect("manifest schema must pass its meta-schema");
        validate_schema_document(REPORT_SCHEMA).expect("report schema must pass its meta-schema");
    }

    #[test]
    fn schema_document_rejects_a_non_2020_12_dialect() {
        let error = validate_schema_document(
            r#"{"$schema":"http://json-schema.org/draft-07/schema#","type":"object"}"#,
        )
        .expect_err("wrong dialect must fail");
        assert!(error.iter().any(|item| item.source == "schema-draft"));
    }

    #[test]
    fn external_reference_resolution_is_disabled() {
        let schema = serde_json::json!({
            "$schema":"https://json-schema.org/draft/2020-12/schema",
            "$ref":"https://example.invalid/not-fetchable.json"
        });
        let instance = serde_json::json!({"ok":true});
        let errors = validate_instance_value(&schema, &instance)
            .expect_err("offline external ref must not resolve");
        assert!(errors.iter().any(|item| item.source == "schema-build"));
    }

    #[test]
    fn malformed_schema_keywords_fail_meta_validation() {
        let invalid = r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","type":"definitely-not-a-json-schema-type"}"#;
        let errors = validate_schema_document(invalid)
            .expect_err("invalid type keyword must fail meta-schema");
        assert!(
            errors
                .iter()
                .any(|item| item.source == "schema-meta-validation")
        );
    }

    #[test]
    fn timestamp_format_is_checked_when_schema_declares_date_time() {
        let schema = serde_json::json!({
            "$schema":"https://json-schema.org/draft/2020-12/schema",
            "type":"object",
            "properties":{"observed_at":{"type":"string","format":"date-time"}},
            "required":["observed_at"],
            "additionalProperties":false
        });
        let instance = serde_json::json!({"observed_at":"yesterday at noon"});
        let errors =
            validate_instance_value(&schema, &instance).expect_err("malformed date-time must fail");
        assert!(
            errors
                .iter()
                .any(|item| item.source == "instance-validation")
        );
    }
}
