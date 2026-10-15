// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Validation of resident-owned home design briefs.
//!
//! A valid brief is well-formed intent, not an evaluated design, an engineering
//! result, a code-compliance determination, or permission to construct.

use jsonschema::draft202012;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::Path};

const EMBEDDED_SCHEMA: &str =
    include_str!("../../../../docs/built-environment/home-design-brief.schema.json");

#[derive(Debug, Serialize)]
pub struct BriefValidationReport {
    pub validator_version: String,
    pub schema_sha256: String,
    pub brief_id: Option<String>,
    pub schema_valid: bool,
    pub semantic_valid: bool,
    pub contract_status: String,
    pub schema_issues: Vec<String>,
    pub semantic_issues: Vec<String>,
    pub limitations: Vec<String>,
}

pub fn validate_path(path: &Path) -> Result<BriefValidationReport, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let brief: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("{} is not valid JSON: {error}", path.display()))?;
    validate_value(&brief)
}

pub fn validate_value(brief: &Value) -> Result<BriefValidationReport, String> {
    let schema: Value = serde_json::from_str(EMBEDDED_SCHEMA)
        .map_err(|error| format!("embedded home-design-brief schema is invalid JSON: {error}"))?;
    if !all_refs_are_local(&schema) {
        return Err("home-design-brief schema contains an external reference".to_string());
    }
    draft202012::meta::validate(&schema).map_err(|error| {
        format!("home-design-brief schema fails Draft 2020-12 meta-validation: {error}")
    })?;

    let validator = draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
        .map_err(|error| format!("cannot compile home-design-brief schema: {error}"))?;
    let schema_issues: Vec<String> = validator
        .iter_errors(brief)
        .map(|error| format!("schema validation: {error}"))
        .collect();
    let schema_valid = schema_issues.is_empty();
    let semantic_issues = if schema_valid {
        semantic_issues(brief)
    } else {
        Vec::new()
    };
    let semantic_valid = schema_valid && semantic_issues.is_empty();
    let contract_status = if semantic_valid {
        "BRIEF_VALID"
    } else {
        "BRIEF_INVALID"
    };
    let brief_id = brief
        .get("brief_id")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let mut hasher = Sha256::new();
    hasher.update(EMBEDDED_SCHEMA.as_bytes());
    let schema_sha256 = format!("{:x}", hasher.finalize());

    Ok(BriefValidationReport {
        validator_version: env!("CARGO_PKG_VERSION").to_string(),
        schema_sha256,
        brief_id,
        schema_valid,
        semantic_valid,
        contract_status: contract_status.to_string(),
        schema_issues,
        semantic_issues,
        limitations: vec![
            "BRIEF_VALID means only that the intent document conforms to this schema and its implemented cross-field checks.".to_string(),
            "This validation does not generate or verify building geometry, costs, climate behavior, engineering calculations, code compliance, permits, or construction readiness.".to_string(),
            "Opaque evidence references are not authenticated, resolved, or independently reviewed by this command.".to_string(),
            "Free-text fields still require a privacy review before sharing.".to_string(),
        ],
    })
}

fn all_refs_are_local(value: &Value) -> bool {
    match value {
        Value::Object(map) => map.iter().all(|(key, child)| {
            (key != "$ref"
                || child
                    .as_str()
                    .is_some_and(|reference| reference.starts_with("#/")))
                && all_refs_are_local(child)
        }),
        Value::Array(values) => values.iter().all(all_refs_are_local),
        _ => true,
    }
}

fn semantic_issues(brief: &Value) -> Vec<String> {
    let mut issues = Vec::new();

    let spaces = brief.get("spaces").and_then(Value::as_array);
    check_unique_ids(spaces.map(Vec::as_slice), "space_id", "spaces", &mut issues);
    if let Some(spaces) = spaces {
        let space_ids: BTreeSet<&str> = spaces
            .iter()
            .filter_map(|space| space.get("space_id").and_then(Value::as_str))
            .collect();
        let mut adjacency_keys = BTreeSet::new();
        let mut dimension_constraint_ids = BTreeSet::new();
        for (space_index, space) in spaces.iter().enumerate() {
            if let Some(constraints) = space
                .get("dimension_constraints")
                .and_then(Value::as_array)
            {
                for (constraint_index, constraint) in constraints.iter().enumerate() {
                    if let Some(constraint_id) =
                        constraint.get("constraint_id").and_then(Value::as_str)
                        && !dimension_constraint_ids.insert(constraint_id.to_owned())
                    {
                        issues.push(format!(
                            "spaces[{space_index}].dimension_constraints[{constraint_index}].constraint_id duplicates {constraint_id:?}"
                        ));
                    }
                }
            }
            let Some(source_id) = space.get("space_id").and_then(Value::as_str) else {
                continue;
            };
            let Some(adjacencies) = space.get("adjacency_preferences").and_then(Value::as_array)
            else {
                continue;
            };
            for (adjacency_index, adjacency) in adjacencies.iter().enumerate() {
                let Some(target_id) = adjacency.get("other_space_id").and_then(Value::as_str)
                else {
                    continue;
                };
                if !space_ids.contains(target_id) {
                    issues.push(format!(
                        "spaces[{space_index}].adjacency_preferences[{adjacency_index}] references unknown space_id {target_id:?}"
                    ));
                }
                if target_id == source_id {
                    issues.push(format!(
                        "spaces[{space_index}].adjacency_preferences[{adjacency_index}] refers to its own space_id {source_id:?}"
                    ));
                }
                let relation = adjacency
                    .get("relationship")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let key = (
                    source_id.to_owned(),
                    target_id.to_owned(),
                    relation.to_owned(),
                );
                if !adjacency_keys.insert(key) {
                    issues.push(format!(
                        "duplicate adjacency relation from space {source_id:?} to {target_id:?} ({relation})"
                    ));
                }
            }
        }
    }

    let priorities = brief.get("priorities").and_then(Value::as_array);
    if let Some(priorities) = priorities {
        let mut axes = BTreeSet::new();
        for (index, priority) in priorities.iter().enumerate() {
            if let Some(axis) = priority.get("axis").and_then(Value::as_str)
                && !axes.insert(axis)
            {
                issues.push(format!(
                    "priorities[{index}] repeats axis {axis:?}; use one explicit importance per axis"
                ));
            }
        }
    }

    let requirements = brief.get("requirements").and_then(Value::as_array);
    check_unique_ids(
        requirements.map(Vec::as_slice),
        "requirement_id",
        "requirements",
        &mut issues,
    );

    if let Some(budget) = brief
        .get("project")
        .and_then(|project| project.get("target_budget"))
        .filter(|budget| !budget.is_null())
        && let (Some(minimum), Some(maximum)) = (
            budget.get("minimum_total").and_then(Value::as_f64),
            budget.get("maximum_total").and_then(Value::as_f64),
        )
        && minimum > maximum
    {
        issues.push(
            "project.target_budget.minimum_total must not exceed maximum_total".to_string(),
        );
    }

    let site = brief.get("site_context");
    if site
        .and_then(|value| value.get("readiness"))
        .and_then(Value::as_str)
        == Some("survey_available")
        && site
            .and_then(|value| value.get("site_evidence_refs"))
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty)
    {
        issues.push(
            "site_context.readiness is survey_available but site_evidence_refs is empty"
                .to_string(),
        );
    }

    issues
}

fn check_unique_ids(
    items: Option<&[Value]>,
    key: &str,
    collection: &str,
    issues: &mut Vec<String>,
) {
    let mut seen = BTreeSet::new();
    if let Some(items) = items {
        for (index, item) in items.iter().enumerate() {
            if let Some(id) = item.get(key).and_then(Value::as_str)
                && !seen.insert(id.to_owned())
            {
                issues.push(format!("{collection}[{index}].{key} duplicates {id:?}"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../docs/built-environment/fixtures/example-home-design-brief.json");
        serde_json::from_slice(&fs::read(path).expect("home-design-brief fixture exists"))
            .expect("home-design-brief fixture parses")
    }

    #[test]
    fn synthetic_fixture_is_contract_valid() {
        let result = validate_value(&fixture()).expect("embedded schema must compile");
        assert!(result.schema_valid, "{:?}", result.schema_issues);
        assert!(result.semantic_valid, "{:?}", result.semantic_issues);
        assert_eq!(result.contract_status, "BRIEF_VALID");
    }

    #[test]
    fn duplicate_space_and_requirement_ids_are_rejected() {
        let mut brief = fixture();
        let duplicate_space_id = brief["spaces"][0]["space_id"].clone();
        brief["spaces"][1]["space_id"] = duplicate_space_id;
        let duplicate_requirement_id = brief["requirements"][0]["requirement_id"].clone();
        brief["requirements"][1]["requirement_id"] = duplicate_requirement_id;
        let result = validate_value(&brief).expect("schema must compile");
        assert!(result.schema_valid, "{:?}", result.schema_issues);
        assert!(!result.semantic_valid);
        assert!(
            result
                .semantic_issues
                .iter()
                .any(|issue| { issue.contains("spaces[1].space_id") })
        );
        assert!(
            result
                .semantic_issues
                .iter()
                .any(|issue| { issue.contains("requirements[1].requirement_id") })
        );
    }

    #[test]
    fn dimension_constraint_ids_are_unique_across_all_spaces() {
        let mut brief = fixture();
        brief["spaces"][1]["dimension_constraints"] =
            brief["spaces"][0]["dimension_constraints"].clone();
        let result = validate_value(&brief).expect("embedded schema must compile");
        assert!(result.schema_valid, "{:?}", result.schema_issues);
        assert!(!result.semantic_valid);
        assert!(result.semantic_issues.iter().any(|issue| {
            issue.contains("dimension_constraints") && issue.contains("duplicates")
        }));
    }

    #[test]
    fn unknown_adjacency_targets_are_rejected() {
        let mut brief = fixture();
        brief["spaces"][0]["adjacency_preferences"][0]["other_space_id"] =
            serde_json::json!("missing-space");
        let result = validate_value(&brief).expect("schema must compile");
        assert!(result.schema_valid, "{:?}", result.schema_issues);
        assert!(!result.semantic_valid);
        assert!(
            result
                .semantic_issues
                .iter()
                .any(|issue| { issue.contains("unknown space_id") })
        );
    }

    #[test]
    fn duplicate_priority_axes_are_rejected() {
        let mut brief = fixture();
        let duplicate_axis = brief["priorities"][0]["axis"].clone();
        brief["priorities"][1]["axis"] = duplicate_axis;
        let result = validate_value(&brief).expect("schema must compile");
        assert!(result.schema_valid, "{:?}", result.schema_issues);
        assert!(!result.semantic_valid);
        assert!(
            result
                .semantic_issues
                .iter()
                .any(|issue| { issue.contains("repeats axis") })
        );
    }

    #[test]
    fn invalid_budget_order_is_rejected() {
        let mut brief = fixture();
        brief["project"]["target_budget"]["minimum_total"] = serde_json::json!(2000);
        brief["project"]["target_budget"]["maximum_total"] = serde_json::json!(1000);
        let result = validate_value(&brief).expect("schema must compile");
        assert!(result.schema_valid, "{:?}", result.schema_issues);
        assert!(!result.semantic_valid);
        assert!(
            result
                .semantic_issues
                .iter()
                .any(|issue| { issue.contains("must not exceed") })
        );
    }

    #[test]
    fn survey_available_requires_evidence_reference() {
        let mut brief = fixture();
        brief["site_context"]["readiness"] = serde_json::json!("survey_available");
        brief["site_context"]["site_evidence_refs"] = serde_json::json!([]);
        let result = validate_value(&brief).expect("schema must compile");
        assert!(result.schema_valid, "{:?}", result.schema_issues);
        assert!(!result.semantic_valid);
        assert!(
            result
                .semantic_issues
                .iter()
                .any(|issue| { issue.contains("site_evidence_refs is empty") })
        );
    }
}
