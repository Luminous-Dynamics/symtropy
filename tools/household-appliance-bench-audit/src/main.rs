use jsonschema::Validator;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

const SCHEMA_ID: &str = "urn:luminous-dynamics:household-appliance-benchmark-corpus:0.1.0";
const EXPECTED_CASE_IDS: [&str; 30] = [
    "HAP-001", "HAP-002", "HAP-003", "HAP-004", "HAP-005", "HAP-006", "HAP-007", "HAP-008",
    "HAP-009", "HAP-010", "HAP-011", "HAP-012", "HAP-013", "HAP-014", "HAP-015", "HAP-016",
    "HAP-017", "HAP-018", "HAP-019", "HAP-020", "HAP-021", "HAP-022", "HAP-023", "HAP-024",
    "HAP-025", "HAP-026", "HAP-027", "HAP-028", "HAP-029", "HAP-030",
];

type Errors = Vec<String>;

#[derive(Debug, PartialEq, Eq)]
struct AuditSummary {
    fixture_count: usize,
    positive_controls: usize,
    negative_controls: usize,
    indeterminate_controls: usize,
    schema_sha256: String,
    corpus_sha256: String,
}

fn text_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn default_schema_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas/appliances/benchmark_scenario_corpus_v0_1.schema.json")
}

fn default_corpus_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/household_appliance/benchmark_scenarios_v0_1.json")
}

fn schema_validator(schema: &Value) -> Result<Validator, String> {
    if text_at(schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema") {
        return Err("schema must declare JSON Schema Draft 2020-12".to_owned());
    }
    if text_at(schema, "$id") != Some(SCHEMA_ID) {
        return Err(format!("schema $id must be {SCHEMA_ID}"));
    }
    if !jsonschema::draft202012::meta::is_valid(schema) {
        return Err("schema does not validate against the Draft 2020-12 meta-schema".to_owned());
    }
    jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| format!("cannot compile schema: {error}"))
}

fn case_by_id<'a>(cases: &'a [Value], id: &str) -> Option<&'a Value> {
    cases
        .iter()
        .find(|case| text_at(case, "case_id") == Some(id))
}

fn input_by_id<'a>(cases: &'a [Value], id: &str) -> Option<&'a Value> {
    case_by_id(cases, id).and_then(|case| case.get("input"))
}

fn require_disposition(cases: &[Value], id: &str, expected: &str, errors: &mut Errors) {
    let actual = case_by_id(cases, id).and_then(|case| text_at(case, "expected_disposition"));
    if actual != Some(expected) {
        errors.push(format!(
            "{id}: expected_disposition must be {expected}, got {actual:?}"
        ));
    }
}

fn require_case_text(cases: &[Value], id: &str, field: &str, expected: &str, errors: &mut Errors) {
    let actual = case_by_id(cases, id).and_then(|case| text_at(case, field));
    if actual != Some(expected) {
        errors.push(format!("{id}.{field}: expected {expected}, got {actual:?}"));
    }
}

fn require_input_text(cases: &[Value], id: &str, field: &str, expected: &str, errors: &mut Errors) {
    let actual = input_by_id(cases, id).and_then(|input| text_at(input, field));
    if actual != Some(expected) {
        errors.push(format!(
            "{id}.input.{field}: expected {expected}, got {actual:?}"
        ));
    }
}

fn require_input_bool(cases: &[Value], id: &str, field: &str, expected: bool, errors: &mut Errors) {
    let actual = input_by_id(cases, id)
        .and_then(|input| input.get(field))
        .and_then(Value::as_bool);
    if actual != Some(expected) {
        errors.push(format!(
            "{id}.input.{field}: expected {expected}, got {actual:?}"
        ));
    }
}

fn require_input_number(
    cases: &[Value],
    id: &str,
    field: &str,
    expected: f64,
    errors: &mut Errors,
) {
    let actual = input_by_id(cases, id)
        .and_then(|input| input.get(field))
        .and_then(Value::as_f64);
    if actual.is_none_or(|value| (value - expected).abs() > f64::EPSILON) {
        errors.push(format!(
            "{id}.input.{field}: expected {expected}, got {actual:?}"
        ));
    }
}

fn require_input_number_pair_different(
    cases: &[Value],
    id: &str,
    first: &str,
    second: &str,
    errors: &mut Errors,
) {
    let input = input_by_id(cases, id);
    let a = input
        .and_then(|value| value.get(first))
        .and_then(Value::as_f64);
    let b = input
        .and_then(|value| value.get(second))
        .and_then(Value::as_f64);
    if a.is_none() || b.is_none() || a == b {
        errors.push(format!(
            "{id}.input: {first} and {second} must both exist and differ for this negative control"
        ));
    }
}

fn require_input_number_is_zero(cases: &[Value], id: &str, field: &str, errors: &mut Errors) {
    require_input_number(cases, id, field, 0.0, errors);
}

fn require_case_has_assertion(cases: &[Value], id: &str, needle: &str, errors: &mut Errors) {
    let found = case_by_id(cases, id)
        .and_then(|case| case.get("assertions"))
        .and_then(Value::as_array)
        .is_some_and(|assertions| {
            assertions
                .iter()
                .filter_map(Value::as_str)
                .any(|assertion| assertion.contains(needle))
        });
    if !found {
        errors.push(format!(
            "{id}.assertions: missing required invariant text containing {needle:?}"
        ));
    }
}

fn check_critical_controls(cases: &[Value]) -> Errors {
    let mut errors = Vec::new();

    // Work-order status cannot replace evidence that the appliance's function was restored.
    require_disposition(cases, "HAP-005", "reject_claim", &mut errors);
    require_input_text(
        cases,
        "HAP-005",
        "work_order_status",
        "completed",
        &mut errors,
    );
    require_input_text(cases, "HAP-005", "post_repair_test", "missing", &mut errors);
    require_case_has_assertion(
        cases,
        "HAP-005",
        "Work-order completion cannot establish restored capability",
        &mut errors,
    );

    // A single nominal sample cannot establish a recurrence-free outcome.
    require_disposition(cases, "HAP-007", "reject_claim", &mut errors);
    require_input_text(
        cases,
        "HAP-007",
        "recurrence_window",
        "missing",
        &mut errors,
    );
    require_input_bool(cases, "HAP-007", "claim_recurrence_free", true, &mut errors);

    // Performance comparisons need matched delivered function.
    require_disposition(cases, "HAP-009", "reject_claim", &mut errors);
    require_input_number_pair_different(
        cases,
        "HAP-009",
        "candidate_airflow_m3_h",
        "baseline_airflow_m3_h",
        &mut errors,
    );
    require_input_bool(
        cases,
        "HAP-009",
        "efficiency_superiority_claim",
        true,
        &mut errors,
    );

    // Unknown waiting time cannot be interpreted as zero.
    require_disposition(cases, "HAP-011", "reject_claim", &mut errors);
    require_input_text(cases, "HAP-011", "parts_wait_days", "unknown", &mut errors);
    require_input_number_is_zero(cases, "HAP-011", "cost_model_substituted_days", &mut errors);

    // Hardware protection takes precedence over model confidence.
    require_disposition(cases, "HAP-014", "reject_claim", &mut errors);
    require_input_bool(
        cases,
        "HAP-014",
        "critical_interlock_triggered",
        true,
        &mut errors,
    );
    require_input_text(
        cases,
        "HAP-014",
        "model_output",
        "safe_to_operate",
        &mut errors,
    );

    // Qualification scope cannot broaden from one unit to a batch.
    require_disposition(cases, "HAP-016", "reject_claim", &mut errors);
    require_input_number(cases, "HAP-016", "qualified_units", 1.0, &mut errors);
    require_input_bool(
        cases,
        "HAP-016",
        "batch_qualification_claim",
        true,
        &mut errors,
    );

    // Stress screening does not automatically prove a decade of field life.
    require_disposition(cases, "HAP-018", "reject_claim", &mut errors);
    require_input_number(cases, "HAP-018", "stress_screen_hours", 1000.0, &mut errors);
    require_input_bool(
        cases,
        "HAP-018",
        "validated_acceleration_model",
        false,
        &mut errors,
    );

    // Unknown material burden cannot be coerced to zero.
    require_disposition(cases, "HAP-019", "reject_claim", &mut errors);
    require_input_text(cases, "HAP-019", "material_mass", "unknown", &mut errors);
    require_input_number_is_zero(cases, "HAP-019", "modelled_material_mass", &mut errors);

    // Essential operation must not require a cloud or network service.
    require_disposition(cases, "HAP-021", "reject_claim", &mut errors);
    require_input_bool(
        cases,
        "HAP-021",
        "essential_function_requires_network",
        true,
        &mut errors,
    );
    require_input_text(
        cases,
        "HAP-021",
        "offline_function_test",
        "fail",
        &mut errors,
    );

    // Event data cannot be represented as unbiased population probability.
    require_disposition(cases, "HAP-023", "reject_claim", &mut errors);
    require_input_text(
        cases,
        "HAP-023",
        "claim_scope",
        "general_population_probability",
        &mut errors,
    );

    // Synthetic results cannot be promoted to physical field evidence.
    require_disposition(cases, "HAP-027", "reject_claim", &mut errors);
    require_input_text(
        cases,
        "HAP-027",
        "evidence_class",
        "synthetic_simulation",
        &mut errors,
    );
    require_input_text(
        cases,
        "HAP-027",
        "claimed_evidence_class",
        "physical_field",
        &mut errors,
    );

    // Pilot-size repair trials do not justify an unplanned confirmatory quantile claim.
    require_disposition(cases, "HAP-028", "reject_claim", &mut errors);
    require_input_bool(
        cases,
        "HAP-028",
        "analysis_plan_preregistered",
        false,
        &mut errors,
    );

    // A diagnosis hypothesis is not an observed cause.
    require_disposition(cases, "HAP-013", "reject_claim", &mut errors);
    require_input_text(
        cases,
        "HAP-013",
        "source_class",
        "observed_cause",
        &mut errors,
    );
    require_input_bool(cases, "HAP-013", "inspection_evidence", false, &mut errors);

    // More module boundaries are not, by themselves, proof of better repairability.
    require_disposition(cases, "HAP-025", "reject_claim", &mut errors);
    require_input_bool(cases, "HAP-025", "repair_time_measured", false, &mut errors);
    require_input_bool(
        cases,
        "HAP-025",
        "repairability_superiority_claim",
        true,
        &mut errors,
    );

    // Positive controls check that evidence remains scoped and that unknowns can be preserved.
    require_disposition(cases, "HAP-001", "accept_synthetic_contract", &mut errors);
    require_case_text(cases, "HAP-001", "kind", "positive_control", &mut errors);
    require_disposition(cases, "HAP-015", "accept_synthetic_contract", &mut errors);
    require_input_text(cases, "HAP-015", "diagnosis", "unknown", &mut errors);
    require_input_bool(cases, "HAP-015", "hardware_actuation", false, &mut errors);
    require_disposition(cases, "HAP-022", "accept_synthetic_contract", &mut errors);
    require_input_text(
        cases,
        "HAP-022",
        "offline_function_test",
        "pass",
        &mut errors,
    );
    require_input_bool(cases, "HAP-022", "remote_actuation", false, &mut errors);
    require_disposition(cases, "HAP-024", "accept_synthetic_contract", &mut errors);
    require_input_bool(cases, "HAP-024", "attribution", true, &mut errors);
    require_input_bool(
        cases,
        "HAP-024",
        "full_dataset_embedded",
        false,
        &mut errors,
    );

    errors
}

fn audit_values(
    schema: &Value,
    corpus: &Value,
    schema_raw: &str,
    corpus_raw: &str,
) -> Result<AuditSummary, Errors> {
    let validator = schema_validator(schema).map_err(|error| vec![error])?;
    let schema_errors: Vec<String> = validator
        .iter_errors(corpus)
        .map(|error| error.to_string())
        .collect();
    if !schema_errors.is_empty() {
        return Err(schema_errors);
    }

    let Some(cases) = corpus.get("cases").and_then(Value::as_array) else {
        return Err(vec!["corpus.cases must be an array".to_owned()]);
    };

    let mut errors = Vec::new();
    let mut seen = BTreeSet::<String>::new();
    let mut positive_controls = 0usize;
    let mut negative_controls = 0usize;
    let mut indeterminate_controls = 0usize;

    for (index, case) in cases.iter().enumerate() {
        let Some(id) = text_at(case, "case_id") else {
            errors.push(format!("cases[{index}].case_id must be a string"));
            continue;
        };
        if !seen.insert(id.to_owned()) {
            errors.push(format!("duplicate case_id {id}"));
        }
        let kind = text_at(case, "kind").unwrap_or("");
        let disposition = text_at(case, "expected_disposition").unwrap_or("");
        let expected_disposition = match kind {
            "positive_control" => {
                positive_controls += 1;
                "accept_synthetic_contract"
            }
            "negative_control" => {
                negative_controls += 1;
                "reject_claim"
            }
            "indeterminate_control" => {
                indeterminate_controls += 1;
                "remain_indeterminate"
            }
            _ => {
                errors.push(format!("{id}: unsupported kind {kind:?}"));
                continue;
            }
        };
        if disposition != expected_disposition {
            errors.push(format!(
                "{id}: kind {kind} requires disposition {expected_disposition}, got {disposition}"
            ));
        }
    }

    let expected_ids: BTreeSet<String> = EXPECTED_CASE_IDS
        .iter()
        .map(|id| (*id).to_owned())
        .collect();
    for missing in expected_ids.difference(&seen) {
        errors.push(format!("required case {missing} is missing"));
    }
    for extra in seen.difference(&expected_ids) {
        errors.push(format!("unexpected case_id {extra}"));
    }

    if positive_controls != 11 || negative_controls != 16 || indeterminate_controls != 3 {
        errors.push(format!(
            "control census changed: positives={positive_controls}, negatives={negative_controls}, indeterminate={indeterminate_controls}; expected 11/16/3"
        ));
    }

    errors.extend(check_critical_controls(cases));
    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(AuditSummary {
        fixture_count: cases.len(),
        positive_controls,
        negative_controls,
        indeterminate_controls,
        schema_sha256: format!("{:x}", Sha256::digest(schema_raw.as_bytes())),
        corpus_sha256: format!("{:x}", Sha256::digest(corpus_raw.as_bytes())),
    })
}

fn audit_texts(schema_raw: &str, corpus_raw: &str) -> Result<AuditSummary, Errors> {
    let schema: Value = serde_json::from_str(schema_raw)
        .map_err(|error| vec![format!("schema JSON parse failed: {error}")])?;
    let corpus: Value = serde_json::from_str(corpus_raw)
        .map_err(|error| vec![format!("corpus JSON parse failed: {error}")])?;
    audit_values(&schema, &corpus, schema_raw, corpus_raw)
}

fn read_arg_path(flag: &str) -> Option<PathBuf> {
    let mut args = env::args_os().skip(1);
    while let Some(argument) = args.next() {
        if argument == flag {
            return args.next().map(PathBuf::from);
        }
    }
    None
}

fn run() -> Result<(), String> {
    let schema_path = read_arg_path("--schema").unwrap_or_else(default_schema_path);
    let corpus_path = read_arg_path("--corpus").unwrap_or_else(default_corpus_path);
    let schema_raw = fs::read_to_string(&schema_path)
        .map_err(|error| format!("cannot read {}: {error}", schema_path.display()))?;
    let corpus_raw = fs::read_to_string(&corpus_path)
        .map_err(|error| format!("cannot read {}: {error}", corpus_path.display()))?;

    let summary = audit_texts(&schema_raw, &corpus_raw).map_err(|errors| errors.join("\n"))?;
    let report = json!({
        "report_version": "household-appliance-bench-audit/0.1",
        "status": "PASS_SYNTHETIC_CONTRACT_ONLY",
        "physical_qualification": false,
        "product_safety_assessment": false,
        "fixture_count": summary.fixture_count,
        "positive_controls": summary.positive_controls,
        "negative_controls": summary.negative_controls,
        "indeterminate_controls": summary.indeterminate_controls,
        "schema_sha256": summary.schema_sha256,
        "corpus_sha256": summary.corpus_sha256,
        "schema_path": schema_path.display().to_string(),
        "corpus_path": corpus_path.display().to_string()
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("household-appliance-bench-audit: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canonical_schema() -> Value {
        serde_json::from_str(include_str!(
            "../../../schemas/appliances/benchmark_scenario_corpus_v0_1.schema.json"
        ))
        .expect("canonical schema JSON must parse")
    }

    fn canonical_corpus() -> Value {
        serde_json::from_str(include_str!(
            "../../../fixtures/household_appliance/benchmark_scenarios_v0_1.json"
        ))
        .expect("canonical corpus JSON must parse")
    }

    fn audit_mutation(corpus: &Value) -> Result<AuditSummary, Errors> {
        let schema = canonical_schema();
        let schema_raw = serde_json::to_string(&schema).expect("schema serialize");
        let corpus_raw = serde_json::to_string(corpus).expect("corpus serialize");
        audit_values(&schema, corpus, &schema_raw, &corpus_raw)
    }

    #[test]
    fn canonical_corpus_passes_synthetic_contract_audit() {
        let schema_raw =
            include_str!("../../../schemas/appliances/benchmark_scenario_corpus_v0_1.schema.json");
        let corpus_raw =
            include_str!("../../../fixtures/household_appliance/benchmark_scenarios_v0_1.json");
        let summary = audit_texts(schema_raw, corpus_raw).expect("canonical corpus must qualify");
        assert_eq!(summary.fixture_count, 30);
        assert_eq!(summary.positive_controls, 11);
        assert_eq!(summary.negative_controls, 16);
        assert_eq!(summary.indeterminate_controls, 3);
        assert_eq!(summary.schema_sha256.len(), 64);
        assert_eq!(summary.corpus_sha256.len(), 64);
    }

    #[test]
    fn duplicate_case_identity_is_rejected() {
        let mut corpus = canonical_corpus();
        corpus["cases"][1]["case_id"] = json!("HAP-001");
        let errors = audit_mutation(&corpus).expect_err("duplicate identity must fail");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("duplicate case_id HAP-001"))
        );
    }

    #[test]
    fn negative_control_cannot_be_relabelled_as_accepted() {
        let mut corpus = canonical_corpus();
        corpus["cases"][4]["expected_disposition"] = json!("accept_synthetic_contract");
        let errors = audit_mutation(&corpus).expect_err("contradictory negative control must fail");
        assert!(errors.iter().any(|error| error.contains("HAP-005")));
    }

    #[test]
    fn critical_interlock_case_cannot_become_safe_to_operate() {
        let mut corpus = canonical_corpus();
        corpus["cases"][13]["input"]["model_output"] = json!("unknown");
        let errors = audit_mutation(&corpus).expect_err("critical fixture mutation must fail");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("HAP-014.input.model_output"))
        );
    }

    #[test]
    fn missing_required_negative_control_is_rejected() {
        let mut corpus = canonical_corpus();
        corpus["cases"]
            .as_array_mut()
            .expect("cases array")
            .remove(26);
        let errors = audit_mutation(&corpus).expect_err("missing negative control must fail");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("required case HAP-027 is missing"))
        );
    }

    #[test]
    fn a_shortened_source_registry_fails_the_schema() {
        let mut corpus = canonical_corpus();
        corpus["provenance"]["source_urls"]
            .as_array_mut()
            .expect("source URLs array")
            .truncate(2);
        let errors = audit_mutation(&corpus).expect_err("too few source URLs must fail");
        assert!(!errors.is_empty());
    }
}
