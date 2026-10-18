use jsonschema::Validator;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const SCHEMA_ID: &str = "urn:luminous-dynamics:appliance-repairability-factorial-plan:0.1.0";
const DESIGN_ID: &str = "APPLIANCE-DOE-001";
const EXPECTED_CELLS: [&str; 8] = [
    "DOE-000", "DOE-001", "DOE-010", "DOE-011", "DOE-100", "DOE-101", "DOE-110", "DOE-111",
];
const EXPECTED_BENCHMARKS: [&str; 6] = [
    "DOE-B-001",
    "DOE-B-002",
    "DOE-B-003",
    "DOE-B-004",
    "DOE-B-005",
    "DOE-B-006",
];

type Errors = Vec<String>;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ExperimentAuditSummary {
    pub(crate) cell_count: usize,
    pub(crate) factor_count: usize,
    pub(crate) planned_trials_per_cell: usize,
    pub(crate) planned_trial_count: usize,
    pub(crate) observed_trial_count: usize,
    pub(crate) benchmark_count: usize,
    pub(crate) schema_sha256: String,
    pub(crate) plan_sha256: String,
}

fn text_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn schema_validator(schema: &Value) -> Result<Validator, String> {
    if text_at(schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema") {
        return Err("factorial plan schema must declare JSON Schema Draft 2020-12".to_owned());
    }
    if text_at(schema, "$id") != Some(SCHEMA_ID) {
        return Err(format!("factorial plan schema $id must be {SCHEMA_ID}"));
    }
    if !jsonschema::draft202012::meta::is_valid(schema) {
        return Err(
            "factorial plan schema does not validate against the Draft 2020-12 meta-schema"
                .to_owned(),
        );
    }
    jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| format!("cannot compile factorial plan schema: {error}"))
}

fn expected_level(levels: &Value, key: &str, bit: char, zero: &str, one: &str) -> bool {
    let expected = if bit == '0' { zero } else { one };
    text_at(levels, key) == Some(expected)
}

fn audit_values(
    schema: &Value,
    plan: &Value,
    schema_raw: &str,
    plan_raw: &str,
) -> Result<ExperimentAuditSummary, Errors> {
    let validator = schema_validator(schema).map_err(|error| vec![error])?;
    let schema_errors: Vec<String> = validator
        .iter_errors(plan)
        .map(|error| error.to_string())
        .collect();
    if !schema_errors.is_empty() {
        return Err(schema_errors);
    }

    let mut errors = Vec::new();
    if text_at(plan, "schema_version") != Some("0.1.0")
        || text_at(plan, "design_id") != Some(DESIGN_ID)
        || text_at(plan, "status") != Some("plan_only_not_frozen")
        || text_at(plan, "claim_ceiling")
            != Some("synthetic_plan_only_no_physical_result_no_winner")
        || plan.get("physical_trials_started").and_then(Value::as_bool) != Some(false)
        || plan.get("observed_trial_count").and_then(Value::as_u64) != Some(0)
        || plan.get("winner_declared").and_then(Value::as_bool) != Some(false)
        || text_at(plan, "allowed_claim_scope") != Some("service_task_mockup_hypotheses_only")
    {
        errors.push(
            "factorial plan must remain not-run, not-frozen, synthetic-only and winner-free"
                .to_owned(),
        );
    }

    let Some(design) = plan.get("factorial_design") else {
        return Err(vec!["factorial_design object is required".to_owned()]);
    };
    if text_at(design, "design_type") != Some("full_2_level_factorial_2_pow_3")
        || design.get("factor_count").and_then(Value::as_u64) != Some(3)
        || text_at(design, "external_control_id") != Some("W0-RETAIL-CONTROL")
        || design.get("frozen_before_outcome").and_then(Value::as_bool) != Some(false)
        || design
            .get("planned_technician_count")
            .and_then(Value::as_u64)
            != Some(3)
        || design
            .get("planned_trials_per_cell")
            .and_then(Value::as_u64)
            != Some(12)
        || text_at(design, "denominator_policy")
            != Some(
                "every_started_task_attempt_including_failed_abandoned_and_safety_stop_stays_in_attempt_denominator",
            )
    {
        errors.push(
            "factorial design, pilot floor, external control or denominator policy changed"
                .to_owned(),
        );
    }
    let expected_interactions = ["A:B", "A:C", "B:C", "A:B:C"];
    let actual_interactions: Vec<&str> = design
        .get("preregistered_interactions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    if actual_interactions != expected_interactions {
        errors.push(
            "all three two-way and the three-way interaction terms must remain preregistered"
                .to_owned(),
        );
    }
    let task_families: Vec<&str> = design
        .get("task_families")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    if task_families.len() != 4 {
        errors.push("initial factorial pilot must retain four declared task families".to_owned());
    }

    let Some(cells) = plan.get("cells").and_then(Value::as_array) else {
        return Err(vec!["factorial cells must be an array".to_owned()]);
    };
    if cells.len() != EXPECTED_CELLS.len() {
        errors.push(format!(
            "full factorial must have exactly {} cells",
            EXPECTED_CELLS.len()
        ));
    }
    let mut cell_ids = BTreeSet::new();
    let mut combinations = BTreeSet::new();
    for (index, cell) in cells.iter().enumerate() {
        let Some(cell_id) = text_at(cell, "cell_id") else {
            errors.push(format!("cells[{index}].cell_id must be a string"));
            continue;
        };
        if !cell_ids.insert(cell_id.to_owned()) {
            errors.push(format!("duplicate factorial cell {cell_id}"));
        }
        if EXPECTED_CELLS.get(index).copied() != Some(cell_id) {
            errors.push(format!(
                "factorial cell order/identity changed at index {index}: {cell_id}"
            ));
        }
        if text_at(cell, "status") != Some("planned_not_run")
            || cell.get("observed_trial_count").and_then(Value::as_u64) != Some(0)
        {
            errors.push(format!(
                "{cell_id}: unrun design cell cannot contain observations"
            ));
        }
        let Some(bits) = cell_id.strip_prefix("DOE-") else {
            errors.push(format!(
                "{cell_id}: cell ID must encode three binary factor levels"
            ));
            continue;
        };
        if bits.len() != 3 || !bits.chars().all(|bit| bit == '0' || bit == '1') {
            errors.push(format!(
                "{cell_id}: cell ID must encode three binary factor levels"
            ));
            continue;
        }
        let Some(levels) = cell.get("factor_levels") else {
            errors.push(format!("{cell_id}: factor_levels are required"));
            continue;
        };
        let chars: Vec<char> = bits.chars().collect();
        let valid = expected_level(
            levels,
            "access_architecture",
            chars[0],
            "conventional_access",
            "service_spine",
        ) && expected_level(
            levels,
            "repair_unit_granularity",
            chars[1],
            "assembly_level",
            "smallest_justified_unit",
        ) && expected_level(
            levels,
            "diagnostic_support",
            chars[2],
            "baseline_documentation",
            "evidence_guided_test_before_replace",
        );
        if !valid {
            errors.push(format!(
                "{cell_id}: factor values do not match the binary cell identity"
            ));
        }
        let combination = format!(
            "{}|{}|{}",
            text_at(levels, "access_architecture").unwrap_or(""),
            text_at(levels, "repair_unit_granularity").unwrap_or(""),
            text_at(levels, "diagnostic_support").unwrap_or("")
        );
        if !combinations.insert(combination) {
            errors.push(format!("{cell_id}: duplicate factor combination"));
        }
        let hypothesis_count = cell
            .get("hypotheses")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        let receipt_count = cell
            .get("required_receipt_types")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        if hypothesis_count < 2 || receipt_count < 3 {
            errors.push(format!(
                "{cell_id}: hypotheses or evidence receipt requirements are incomplete"
            ));
        }
    }
    for expected in EXPECTED_CELLS {
        if !cell_ids.contains(expected) {
            errors.push(format!("required factorial cell {expected} is missing"));
        }
    }
    if combinations.len() != 8 {
        errors.push("all eight unique factor combinations must appear exactly once".to_owned());
    }

    let per_cell = design
        .get("planned_trials_per_cell")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let planned_total = design
        .get("planned_trial_count")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    if per_cell.saturating_mul(cells.len()) != planned_total {
        errors.push("planned trial total must equal trials per cell times cell count".to_owned());
    }
    if planned_total != 96 {
        errors.push(
            "v0.1 feasibility pilot plan must retain the declared 96-attempt floor".to_owned(),
        );
    }

    let Some(benchmarks) = plan.get("benchmarks").and_then(Value::as_array) else {
        return Err(vec!["benchmarks must be an array".to_owned()]);
    };
    if benchmarks.len() != EXPECTED_BENCHMARKS.len() {
        errors.push(format!(
            "factorial plan must retain {} benchmark guards",
            EXPECTED_BENCHMARKS.len()
        ));
    }
    let mut benchmark_ids = BTreeSet::new();
    for (index, benchmark) in benchmarks.iter().enumerate() {
        let Some(id) = text_at(benchmark, "id") else {
            errors.push(format!("benchmarks[{index}].id must be a string"));
            continue;
        };
        if !benchmark_ids.insert(id.to_owned()) {
            errors.push(format!("duplicate benchmark ID {id}"));
        }
        if EXPECTED_BENCHMARKS.get(index).copied() != Some(id) {
            errors.push(format!("benchmark identity changed at index {index}: {id}"));
        }
        if text_at(benchmark, "status").is_none_or(|status| {
            !matches!(status, "proposal_not_frozen" | "guardrail_contract_only")
        }) {
            errors.push(format!(
                "{id}: benchmark cannot be promoted to an observed result"
            ));
        }
    }
    for expected in EXPECTED_BENCHMARKS {
        if !benchmark_ids.contains(expected) {
            errors.push(format!("required benchmark {expected} is missing"));
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(ExperimentAuditSummary {
        cell_count: cells.len(),
        factor_count: design
            .get("factor_count")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
        planned_trials_per_cell: per_cell,
        planned_trial_count: planned_total,
        observed_trial_count: plan
            .get("observed_trial_count")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
        benchmark_count: benchmarks.len(),
        schema_sha256: format!("{:x}", Sha256::digest(schema_raw.as_bytes())),
        plan_sha256: format!("{:x}", Sha256::digest(plan_raw.as_bytes())),
    })
}

pub(crate) fn audit_experiment_texts(
    schema_raw: &str,
    plan_raw: &str,
) -> Result<ExperimentAuditSummary, Errors> {
    let schema: Value = serde_json::from_str(schema_raw)
        .map_err(|error| vec![format!("factorial schema JSON parse failed: {error}")])?;
    let plan: Value = serde_json::from_str(plan_raw)
        .map_err(|error| vec![format!("factorial plan JSON parse failed: {error}")])?;
    audit_values(&schema, &plan, schema_raw, plan_raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SCHEMA: &str = include_str!(
        "../../../schemas/appliances/washer_repairability_factorial_plan_v0_1.schema.json"
    );
    const PLAN: &str =
        include_str!("../../../fixtures/appliances/washer_repairability_factorial_plan_v0_1.json");

    fn canonical_plan() -> Value {
        serde_json::from_str(PLAN).expect("factorial plan fixture must parse")
    }

    fn audit_mutation(plan: &Value) -> Result<ExperimentAuditSummary, Errors> {
        let schema: Value = serde_json::from_str(SCHEMA).expect("factorial schema must parse");
        let schema_raw = serde_json::to_string(&schema).expect("schema serializes");
        let plan_raw = serde_json::to_string(plan).expect("plan serializes");
        audit_values(&schema, plan, &schema_raw, &plan_raw)
    }

    #[test]
    fn canonical_factorial_has_eight_cells_and_ninety_six_planned_trials() {
        let summary =
            audit_experiment_texts(SCHEMA, PLAN).expect("canonical factorial plan qualifies");
        assert_eq!(summary.cell_count, 8);
        assert_eq!(summary.factor_count, 3);
        assert_eq!(summary.planned_trials_per_cell, 12);
        assert_eq!(summary.planned_trial_count, 96);
        assert_eq!(summary.observed_trial_count, 0);
        assert_eq!(summary.benchmark_count, 6);
    }

    #[test]
    fn missing_factorial_cell_is_rejected() {
        let mut plan = canonical_plan();
        plan["cells"].as_array_mut().unwrap().remove(7);
        assert!(audit_mutation(&plan).is_err());
    }

    #[test]
    fn duplicate_cell_identity_is_rejected() {
        let mut plan = canonical_plan();
        plan["cells"][7]["cell_id"] = json!("DOE-110");
        assert!(audit_mutation(&plan).is_err());
    }

    #[test]
    fn factor_values_must_match_binary_cell_identity() {
        let mut plan = canonical_plan();
        plan["cells"][7]["factor_levels"]["diagnostic_support"] = json!("baseline_documentation");
        assert!(audit_mutation(&plan).is_err());
    }

    #[test]
    fn unrun_plan_cannot_report_observed_trials() {
        let mut plan = canonical_plan();
        plan["cells"][0]["observed_trial_count"] = json!(1);
        assert!(audit_mutation(&plan).is_err());
    }

    #[test]
    fn planned_trial_total_must_match_cell_repetitions() {
        let mut plan = canonical_plan();
        plan["factorial_design"]["planned_trial_count"] = json!(88);
        assert!(audit_mutation(&plan).is_err());
    }

    #[test]
    fn winner_cannot_be_declared_from_a_plan() {
        let mut plan = canonical_plan();
        plan["winner_declared"] = json!(true);
        assert!(audit_mutation(&plan).is_err());
    }

    #[test]
    fn protocol_cannot_be_claimed_frozen_before_it_is_reviewed() {
        let mut plan = canonical_plan();
        plan["factorial_design"]["frozen_before_outcome"] = json!(true);
        assert!(audit_mutation(&plan).is_err());
    }

    #[test]
    fn all_interaction_terms_are_preregistered_in_the_plan() {
        let mut plan = canonical_plan();
        plan["factorial_design"]["preregistered_interactions"] = json!(["A:B", "A:C", "B:C"]);
        assert!(audit_mutation(&plan).is_err());
    }

    #[test]
    fn external_retail_control_is_not_silently_made_a_factorial_cell() {
        let mut plan = canonical_plan();
        plan["factorial_design"]["external_control_id"] = json!("DOE-000");
        assert!(audit_mutation(&plan).is_err());
    }

    #[test]
    fn duplicated_factor_combination_is_rejected_even_with_unique_ids() {
        let mut plan = canonical_plan();
        let levels = plan["cells"][6]["factor_levels"].clone();
        plan["cells"][7]["factor_levels"] = levels;
        assert!(audit_mutation(&plan).is_err());
    }

    #[test]
    fn proposed_benchmark_cannot_be_promoted_to_result() {
        let mut plan = canonical_plan();
        plan["benchmarks"][0]["status"] = json!("validated_winner");
        assert!(audit_mutation(&plan).is_err());
    }
}
