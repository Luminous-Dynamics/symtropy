use jsonschema::Validator;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const SCHEMA_ID: &str = "urn:luminous-dynamics:washer-fault-to-repair-curriculum:0.1.0";
const CURRICULUM_ID: &str = "WASHER-FAULT-CURRICULUM-001";
const EXPECTED_TASKS: [(&str, &str); 8] = [
    ("WFT-01", "U0"),
    ("WFT-02", "P"),
    ("WFT-03", "P"),
    ("WFT-04", "P"),
    ("WFT-05", "P"),
    ("WFT-06", "P"),
    ("WFT-07", "P"),
    ("WFT-08", "P"),
];

type Errors = Vec<String>;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct FaultCurriculumSummary {
    pub(crate) task_count: usize,
    pub(crate) user_tier_count: usize,
    pub(crate) qualified_tier_count: usize,
    pub(crate) reference_count: usize,
    pub(crate) schema_sha256: String,
    pub(crate) curriculum_sha256: String,
}

fn text_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn schema_validator(schema: &Value) -> Result<Validator, String> {
    if text_at(schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema") {
        return Err("fault curriculum schema must declare JSON Schema Draft 2020-12".to_owned());
    }
    if text_at(schema, "$id") != Some(SCHEMA_ID) {
        return Err(format!("fault curriculum schema $id must be {SCHEMA_ID}"));
    }
    if !jsonschema::draft202012::meta::is_valid(schema) {
        return Err("fault curriculum schema does not validate against the Draft 2020-12 meta-schema".to_owned());
    }
    jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| format!("cannot compile fault curriculum schema: {error}"))
}

fn audit_values(
    schema: &Value,
    curriculum: &Value,
    schema_raw: &str,
    curriculum_raw: &str,
) -> Result<FaultCurriculumSummary, Errors> {
    let validator = schema_validator(schema).map_err(|error| vec![error])?;
    let schema_errors: Vec<String> = validator
        .iter_errors(curriculum)
        .map(|error| error.to_string())
        .collect();
    if !schema_errors.is_empty() {
        return Err(schema_errors);
    }

    let mut errors = Vec::new();
    if text_at(curriculum, "schema_version") != Some("0.1.0")
        || text_at(curriculum, "curriculum_id") != Some(CURRICULUM_ID)
    {
        errors.push("fault curriculum identity/version changed".to_owned());
    }
    if text_at(curriculum, "status") != Some("benchmark_design_only")
        || text_at(curriculum, "claim_ceiling")
            != Some("no_local_prevalence_no_physical_repair_or_superiority_claim")
    {
        errors.push("fault curriculum cannot claim prevalence, physical repair results or superiority".to_owned());
    }
    if text_at(curriculum, "selection_basis")
        != Some("coverage_balanced_not_prevalence_weighted")
    {
        errors.push("fault curriculum must remain coverage-balanced, not prevalence-weighted".to_owned());
    }

    let Some(references) = curriculum.get("references").and_then(Value::as_array) else {
        return Err(vec!["fault curriculum references must be an array".to_owned()]);
    };
    let mut reference_ids = BTreeSet::new();
    for reference in references {
        if let Some(id) = text_at(reference, "id") {
            if !reference_ids.insert(id.to_owned()) {
                errors.push(format!("duplicate fault curriculum reference {id}"));
            }
        }
    }
    let study_scope = references.iter().find(|reference| {
        text_at(reference, "id") == Some("REPAIR-DIAG-11K")
    });
    let scope_note = study_scope.and_then(|reference| text_at(reference, "scope_note"))
        .unwrap_or("")
        .to_ascii_lowercase();
    if !scope_note.contains("not a target-market prevalence")
        || !scope_note.contains("calibrated washer prior")
    {
        errors.push("11k diagnosis source must retain target-market prevalence/calibration caveat".to_owned());
    }

    let Some(tasks) = curriculum.get("tasks").and_then(Value::as_array) else {
        return Err(vec!["fault curriculum tasks must be an array".to_owned()]);
    };
    if tasks.len() != EXPECTED_TASKS.len() {
        errors.push(format!("fault curriculum must contain {} task families", EXPECTED_TASKS.len()));
    }
    let mut task_ids = BTreeSet::new();
    let mut user_tier_count = 0usize;
    let mut qualified_tier_count = 0usize;
    for (index, task) in tasks.iter().enumerate() {
        let Some(id) = text_at(task, "id") else {
            errors.push(format!("tasks[{index}].id must be a string"));
            continue;
        };
        if !task_ids.insert(id.to_owned()) {
            errors.push(format!("duplicate fault task ID {id}"));
        }
        if let Some((expected_id, expected_tier)) = EXPECTED_TASKS.get(index) {
            if id != *expected_id || text_at(task, "service_tier") != Some(*expected_tier) {
                errors.push(format!(
                    "task[{index}] must retain reviewed identity/tier {expected_id}/{expected_tier}"
                ));
            }
        }
        if text_at(task, "status") != Some("curriculum_design_only") {
            errors.push(format!("{id}: task cannot claim physical validation"));
        }
        match text_at(task, "service_tier") {
            Some("U0") => user_tier_count += 1,
            Some("P") => qualified_tier_count += 1,
            other => errors.push(format!("{id}: unsupported role boundary {other:?}")),
        }

        let task_refs: BTreeSet<&str> = task
            .get("source_ids")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        if task_refs.is_empty() || task_refs.iter().any(|reference| !reference_ids.contains(*reference)) {
            errors.push(format!("{id}: source_ids must reference known curriculum sources"));
        }
        for (field, minimum) in [
            ("competing_hypotheses", 2usize),
            ("safe_discriminating_checks", 2usize),
            ("repair_scope_options", 2usize),
            ("post_repair_verification", 2usize),
        ] {
            if task.get(field).and_then(Value::as_array).is_none_or(|items| items.len() < minimum) {
                errors.push(format!("{id}: {field} must retain at least {minimum} entries"));
            }
        }
        let boundary = text_at(task, "hazard_boundary").unwrap_or("").to_ascii_lowercase();
        if !boundary.contains("qualified")
            && (boundary.contains("mains") || boundary.contains("heater") || boundary.contains("rotating"))
        {
            errors.push(format!("{id}: hazardous task boundary must identify qualified-service controls"));
        }
        if text_at(task, "service_tier") == Some("U0") {
            if !boundary.contains("user")
                || boundary.contains("may open internal mains")
                || boundary.contains("may perform refrigerant charging")
            {
                errors.push("U0 task boundary must remain limited to intended safe user maintenance".to_owned());
            }
        }
    }
    for (expected, _) in EXPECTED_TASKS {
        if !task_ids.contains(expected) {
            errors.push(format!("required fault task {expected} is missing"));
        }
    }
    if user_tier_count != 1 || qualified_tier_count != 7 {
        errors.push(format!(
            "reviewed task tier census must remain U0=1/P=7; got U0={user_tier_count}/P={qualified_tier_count}"
        ));
    }

    let policy = curriculum.get("measurement_policy").unwrap_or(&Value::Null);
    if text_at(policy, "aggregate_policy")
        != Some("per_class_plus_macro_average_no_easy_task_masking")
        || text_at(policy, "prevalence_policy")
            != Some("no_prevalence_weights_without_validated_target_market_denominator")
        || text_at(policy, "unknown_policy") != Some("unknown_not_zero")
        || text_at(policy, "safety_gate") != Some("non_compensatory")
        || policy.get("success_receipt_required").and_then(Value::as_bool) != Some(true)
    {
        errors.push("fault measurement policy must preserve class coverage, unknown-not-zero, hard safety gates and repair receipts".to_owned());
    }

    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(FaultCurriculumSummary {
        task_count: tasks.len(),
        user_tier_count,
        qualified_tier_count,
        reference_count: references.len(),
        schema_sha256: format!("{:x}", Sha256::digest(schema_raw.as_bytes())),
        curriculum_sha256: format!("{:x}", Sha256::digest(curriculum_raw.as_bytes())),
    })
}

pub(crate) fn audit_fault_curriculum_texts(
    schema_raw: &str,
    curriculum_raw: &str,
) -> Result<FaultCurriculumSummary, Errors> {
    let schema: Value = serde_json::from_str(schema_raw)
        .map_err(|error| vec![format!("fault curriculum schema parse failed: {error}")])?;
    let curriculum: Value = serde_json::from_str(curriculum_raw)
        .map_err(|error| vec![format!("fault curriculum parse failed: {error}")])?;
    audit_values(&schema, &curriculum, schema_raw, curriculum_raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SCHEMA: &str = include_str!(
        "../../../schemas/appliances/washer_fault_to_repair_curriculum_v0_1.schema.json"
    );
    const CURRICULUM: &str = include_str!(
        "../../../fixtures/appliances/washer_fault_to_repair_curriculum_v0_1.json"
    );

    fn canonical_curriculum() -> Value {
        serde_json::from_str(CURRICULUM).expect("fault curriculum must parse")
    }

    fn audit_mutation(curriculum: &Value) -> Result<FaultCurriculumSummary, Errors> {
        let schema: Value = serde_json::from_str(SCHEMA).expect("schema must parse");
        let schema_raw = serde_json::to_string(&schema).expect("schema serializes");
        let curriculum_raw = serde_json::to_string(curriculum).expect("curriculum serializes");
        audit_values(&schema, curriculum, &schema_raw, &curriculum_raw)
    }

    #[test]
    fn canonical_fault_curriculum_has_full_coverage_and_no_prevalence_weighting() {
        let summary = audit_fault_curriculum_texts(SCHEMA, CURRICULUM)
            .expect("canonical fault curriculum must qualify as design-only");
        assert_eq!(summary.task_count, 8);
        assert_eq!(summary.user_tier_count, 1);
        assert_eq!(summary.qualified_tier_count, 7);
        assert_eq!(summary.reference_count, 3);
        assert_eq!(summary.schema_sha256.len(), 64);
        assert_eq!(summary.curriculum_sha256.len(), 64);
    }

    #[test]
    fn duplicate_fault_family_id_is_rejected() {
        let mut curriculum = canonical_curriculum();
        curriculum["tasks"][1]["id"] = json!("WFT-01");
        let errors = audit_mutation(&curriculum).expect_err("duplicate task ID must fail");
        assert!(!errors.is_empty());
    }

    #[test]
    fn removing_post_repair_verification_is_rejected() {
        let mut curriculum = canonical_curriculum();
        curriculum["tasks"][7]["post_repair_verification"] = json!([]);
        let errors = audit_mutation(&curriculum).expect_err("post-repair proof is required");
        assert!(!errors.is_empty());
    }

    #[test]
    fn literature_cannot_be_promoted_to_south_african_prevalence() {
        let mut curriculum = canonical_curriculum();
        curriculum["references"][0]["scope_note"] =
            json!("This source is a calibrated South African washer prevalence prior.");
        let errors = audit_mutation(&curriculum).expect_err("source caveat mutation must fail");
        assert!(!errors.is_empty());
    }

    #[test]
    fn a_qualified_internal_task_cannot_be_downgraded_to_user_maintenance() {
        let mut curriculum = canonical_curriculum();
        curriculum["tasks"][6]["service_tier"] = json!("U0");
        let errors = audit_mutation(&curriculum).expect_err("drive internals must remain qualified");
        assert!(!errors.is_empty());
    }

    #[test]
    fn a_weighted_fault_prevalence_score_is_rejected() {
        let mut curriculum = canonical_curriculum();
        curriculum["selection_basis"] = json!("frequency_weighted_by_repair_center_incidence");
        let errors = audit_mutation(&curriculum).expect_err("prevalence weighting must fail");
        assert!(!errors.is_empty());
    }

    #[test]
    fn missing_competing_diagnostic_hypothesis_is_rejected() {
        let mut curriculum = canonical_curriculum();
        curriculum["tasks"][7]["competing_hypotheses"] = json!(["bearing wear"]);
        let errors = audit_mutation(&curriculum).expect_err("single hypothesis is not a diagnostic task");
        assert!(!errors.is_empty());
    }

    #[test]
    fn no_task_can_remove_the_non_compensatory_safety_gate() {
        let mut curriculum = canonical_curriculum();
        curriculum["measurement_policy"]["safety_gate"] = json!("weighted_cost_penalty");
        let errors = audit_mutation(&curriculum).expect_err("safety gate cannot be weighted away");
        assert!(!errors.is_empty());
    }

    #[test]
    fn success_receipt_cannot_be_optional() {
        let mut curriculum = canonical_curriculum();
        curriculum["measurement_policy"]["success_receipt_required"] = json!(false);
        let errors = audit_mutation(&curriculum).expect_err("success receipt is mandatory");
        assert!(!errors.is_empty());
    }
}
