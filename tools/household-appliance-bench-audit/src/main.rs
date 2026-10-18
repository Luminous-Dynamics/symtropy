use jsonschema::Validator;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::process::ExitCode;

mod compatibility;
mod decision;
mod experiment;
mod faults;
mod morphology;
mod ords;
mod portfolio;
mod restoration;
mod sourcing;
mod stakeholder;

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

static TEMP_OUTPUT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn write_ords_output_atomically(
    schema_raw: &str,
    mapping_raw: &str,
    csv_raw: &[u8],
    output_path: &PathBuf,
) -> Result<ords::OrdsAuditSummary, String> {
    let sequence = TEMP_OUTPUT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let mut temporary_os = output_path.as_os_str().to_os_string();
    temporary_os.push(format!(".partial.{}.{}", std::process::id(), sequence));
    let temporary_path = PathBuf::from(temporary_os);

    let result = (|| {
        let output_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
            .map_err(|error| {
                format!(
                    "cannot create temporary ORDS output {}: {error}",
                    temporary_path.display()
                )
            })?;
        let mut output = BufWriter::new(output_file);
        let summary = ords::audit_csv_to_writer(schema_raw, mapping_raw, csv_raw, &mut output)
            .map_err(|errors| errors.join("\n"))?;
        output.flush().map_err(|error| {
            format!(
                "cannot flush temporary ORDS output {}: {error}",
                temporary_path.display()
            )
        })?;
        drop(output);
        fs::rename(&temporary_path, output_path).map_err(|error| {
            format!(
                "cannot atomically publish ORDS output {}: {error}",
                output_path.display()
            )
        })?;
        Ok(summary)
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    result
}

fn run() -> Result<(), String> {
    let schema_path = read_arg_path("--schema").unwrap_or_else(default_schema_path);
    let corpus_path = read_arg_path("--corpus").unwrap_or_else(default_corpus_path);
    let portfolio_schema_path = read_arg_path("--portfolio-schema");
    let portfolio_path = read_arg_path("--portfolio");
    if portfolio_schema_path.is_some() != portfolio_path.is_some() {
        return Err("--portfolio-schema and --portfolio must be supplied together".to_owned());
    }
    let sourcing_schema_path = read_arg_path("--sourcing-schema");
    let sourcing_snapshot_path = read_arg_path("--sourcing-snapshot");
    if sourcing_schema_path.is_some() != sourcing_snapshot_path.is_some() {
        return Err(
            "--sourcing-schema and --sourcing-snapshot must be supplied together".to_owned(),
        );
    }
    let decision_schema_path = read_arg_path("--decision-schema");
    let decision_profile_path = read_arg_path("--decision-profile");
    if decision_schema_path.is_some() != decision_profile_path.is_some() {
        return Err(
            "--decision-schema and --decision-profile must be supplied together".to_owned(),
        );
    }
    let faults_schema_path = read_arg_path("--faults-schema");
    let faults_curriculum_path = read_arg_path("--faults-curriculum");
    if faults_schema_path.is_some() != faults_curriculum_path.is_some() {
        return Err("--faults-schema and --faults-curriculum must be supplied together".to_owned());
    }
    let morphology_schema_path = read_arg_path("--morphology-schema");
    let morphology_portfolio_path = read_arg_path("--morphology-portfolio");
    if morphology_schema_path.is_some() != morphology_portfolio_path.is_some() {
        return Err(
            "--morphology-schema and --morphology-portfolio must be supplied together".to_owned(),
        );
    }
    let restoration_schema_path = read_arg_path("--restoration-schema");
    let restoration_ledger_path = read_arg_path("--restoration-ledger");
    if restoration_schema_path.is_some() != restoration_ledger_path.is_some() {
        return Err(
            "--restoration-schema and --restoration-ledger must be supplied together".to_owned(),
        );
    }
    let experiment_schema_path = read_arg_path("--experiment-schema");
    let experiment_plan_path = read_arg_path("--experiment-plan");
    if experiment_schema_path.is_some() != experiment_plan_path.is_some() {
        return Err(
            "--experiment-schema and --experiment-plan must be supplied together".to_owned(),
        );
    }
    let compatibility_schema_path = read_arg_path("--compatibility-schema");
    let compatibility_graph_path = read_arg_path("--compatibility-graph");
    if compatibility_schema_path.is_some() != compatibility_graph_path.is_some() {
        return Err(
            "--compatibility-schema and --compatibility-graph must be supplied together".to_owned(),
        );
    }
    let ords_schema_path = read_arg_path("--ords-schema");
    let ords_mapping_path = read_arg_path("--ords-mapping");
    let ords_csv_path = read_arg_path("--ords-csv");
    let ords_output_path = read_arg_path("--ords-output");
    let ords_any = ords_schema_path.is_some()
        || ords_mapping_path.is_some()
        || ords_csv_path.is_some()
        || ords_output_path.is_some();
    let ords_all = ords_schema_path.is_some()
        && ords_mapping_path.is_some()
        && ords_csv_path.is_some()
        && ords_output_path.is_some();
    if ords_any && !ords_all {
        return Err(
            "--ords-schema, --ords-mapping, --ords-csv, and --ords-output must be supplied together"
                .to_owned(),
        );
    }

    let schema_raw = fs::read_to_string(&schema_path)
        .map_err(|error| format!("cannot read {}: {error}", schema_path.display()))?;
    let corpus_raw = fs::read_to_string(&corpus_path)
        .map_err(|error| format!("cannot read {}: {error}", corpus_path.display()))?;

    let summary = audit_texts(&schema_raw, &corpus_raw).map_err(|errors| errors.join("\n"))?;
    let mut report = json!({
        "report_version": "household-appliance-bench-audit/0.2",
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

    if let (Some(portfolio_schema_path), Some(portfolio_path)) =
        (portfolio_schema_path, portfolio_path)
    {
        let portfolio_schema_raw = fs::read_to_string(&portfolio_schema_path)
            .map_err(|error| format!("cannot read {}: {error}", portfolio_schema_path.display()))?;
        let portfolio_raw = fs::read_to_string(&portfolio_path)
            .map_err(|error| format!("cannot read {}: {error}", portfolio_path.display()))?;
        let portfolio_summary =
            portfolio::audit_portfolio_texts(&portfolio_schema_raw, &portfolio_raw)
                .map_err(|errors| errors.join("\n"))?;

        report["large_appliance_portfolio"] = json!({
            "status": "PASS_SYNTHETIC_PORTFOLIO_CONTRACT_ONLY",
            "family_count": portfolio_summary.family_count,
            "washer_variant_count": portfolio_summary.washer_variant_count,
            "washer_protocol_metric_count": portfolio_summary.washer_protocol_metric_count,
            "proposed_target_count": portfolio_summary.proposed_target_count,
            "schema_sha256": portfolio_summary.schema_sha256,
            "portfolio_sha256": portfolio_summary.portfolio_sha256,
            "schema_path": portfolio_schema_path.display().to_string(),
            "portfolio_path": portfolio_path.display().to_string(),
            "physical_performance_established": false,
            "product_safety_assessed": false
        });
    }

    if let (Some(sourcing_schema_path), Some(sourcing_snapshot_path)) =
        (sourcing_schema_path, sourcing_snapshot_path)
    {
        let sourcing_schema_raw = fs::read_to_string(&sourcing_schema_path)
            .map_err(|error| format!("cannot read {}: {error}", sourcing_schema_path.display()))?;
        let sourcing_snapshot_raw =
            fs::read_to_string(&sourcing_snapshot_path).map_err(|error| {
                format!("cannot read {}: {error}", sourcing_snapshot_path.display())
            })?;
        let sourcing_summary =
            sourcing::audit_sourcing_texts(&sourcing_schema_raw, &sourcing_snapshot_raw)
                .map_err(|errors| errors.join("\n"))?;

        report["washer_parts_resilience_sourcing"] = json!({
            "status": "PASS_DESK_SNAPSHOT_CONTRACT_ONLY",
            "candidate_lead_count": sourcing_summary.candidate_count,
            "price_observation_count": sourcing_summary.price_observation_count,
            "parts_catalogue_probe_count": sourcing_summary.part_probe_count,
            "model_fit_confirmed_probe_count": sourcing_summary.model_fit_confirmed_probe_count,
            "selected_control_count": sourcing_summary.selected_control_count,
            "purchased_candidate_count": sourcing_summary.purchased_candidate_count,
            "schema_sha256": sourcing_summary.schema_sha256,
            "snapshot_sha256": sourcing_summary.snapshot_sha256,
            "schema_path": sourcing_schema_path.display().to_string(),
            "snapshot_path": sourcing_snapshot_path.display().to_string(),
            "physical_performance_established": false,
            "product_safety_assessed": false,
            "purchase_authorized": false
        });
    }

    if let (Some(decision_schema_path), Some(decision_profile_path)) =
        (decision_schema_path, decision_profile_path)
    {
        let decision_schema_raw = fs::read_to_string(&decision_schema_path)
            .map_err(|error| format!("cannot read {}: {error}", decision_schema_path.display()))?;
        let decision_profile_raw = fs::read_to_string(&decision_profile_path)
            .map_err(|error| format!("cannot read {}: {error}", decision_profile_path.display()))?;
        let (decision_summary, assessment) =
            decision::audit_decision_texts(&decision_schema_raw, &decision_profile_raw)
                .map_err(|errors| errors.join("\n"))?;

        let (stakeholder_summary, stakeholder_report) =
            stakeholder::audit_stakeholder_texts(&decision_schema_raw, &decision_profile_raw)
                .map_err(|errors| errors.join("\n"))?;

        report["stakeholder_repairability_profiles"] = json!({
            "status": "PASS_STAKEHOLDER_PROFILE_CONTRACT_ONLY",
            "profile_count": stakeholder_summary.profile_count,
            "requirement_count": stakeholder_summary.requirement_count,
            "frozen_requirement_count": stakeholder_summary.frozen_requirement_count,
            "profile_ready_count": stakeholder_summary.profile_ready_count,
            "report": stakeholder_report,
            "profile_specific_selection_claim_permitted": false,
            "physical_user_research_completed": false
        });

        report["repairability_architecture_decision"] = json!({
            "status": "PASS_DECISION_PROFILE_CONTRACT_ONLY",
            "candidate_count": decision_summary.candidate_count,
            "hard_gate_count": decision_summary.hard_gate_count,
            "criterion_count": decision_summary.criterion_count,
            "measurement_observation_count": decision_summary.observation_count,
            "eligible_candidate_count": decision_summary.eligible_candidate_count,
            "robust_pareto_frontier_count": decision_summary.pareto_frontier_count,
            "schema_sha256": decision_summary.schema_sha256,
            "decision_sha256": decision_summary.decision_sha256,
            "schema_path": decision_schema_path.display().to_string(),
            "decision_profile_path": decision_profile_path.display().to_string(),
            "assessment": assessment,
            "eligible_comparative_evidence_available": decision_summary.eligible_candidate_count >= 2,
            "product_winner_declared": false,
            "product_safety_assessed": false
        });
    }

    if let (Some(faults_schema_path), Some(faults_curriculum_path)) =
        (faults_schema_path, faults_curriculum_path)
    {
        let faults_schema_raw = fs::read_to_string(&faults_schema_path)
            .map_err(|error| format!("cannot read {}: {error}", faults_schema_path.display()))?;
        let faults_curriculum_raw =
            fs::read_to_string(&faults_curriculum_path).map_err(|error| {
                format!("cannot read {}: {error}", faults_curriculum_path.display())
            })?;
        let faults_summary =
            faults::audit_fault_curriculum_texts(&faults_schema_raw, &faults_curriculum_raw)
                .map_err(|errors| errors.join("\n"))?;

        report["washer_fault_to_repair_curriculum"] = json!({
            "status": "PASS_FAULT_CURRICULUM_CONTRACT_ONLY",
            "task_count": faults_summary.task_count,
            "user_tier_task_count": faults_summary.user_tier_count,
            "qualified_tier_task_count": faults_summary.qualified_tier_count,
            "reference_count": faults_summary.reference_count,
            "schema_sha256": faults_summary.schema_sha256,
            "curriculum_sha256": faults_summary.curriculum_sha256,
            "schema_path": faults_schema_path.display().to_string(),
            "curriculum_path": faults_curriculum_path.display().to_string(),
            "physical_fault_trials_completed": false,
            "local_prevalence_established": false,
            "product_safety_assessed": false
        });
    }

    if let (Some(morphology_schema_path), Some(morphology_portfolio_path)) =
        (morphology_schema_path, morphology_portfolio_path)
    {
        let morphology_schema_raw =
            fs::read_to_string(&morphology_schema_path).map_err(|error| {
                format!("cannot read {}: {error}", morphology_schema_path.display())
            })?;
        let morphology_portfolio_raw =
            fs::read_to_string(&morphology_portfolio_path).map_err(|error| {
                format!(
                    "cannot read {}: {error}",
                    morphology_portfolio_path.display()
                )
            })?;
        let morphology_summary =
            morphology::audit_morphology_texts(&morphology_schema_raw, &morphology_portfolio_raw)
                .map_err(|errors| errors.join("\n"))?;

        report["cross_appliance_morphology"] = json!({
            "status": "PASS_MORPHOLOGY_CONTRACT_ONLY",
            "family_count": morphology_summary.family_count,
            "variant_count": morphology_summary.variant_count,
            "design_pattern_count": morphology_summary.pattern_count,
            "hybrid_candidate_count": morphology_summary.hybrid_count,
            "schema_sha256": morphology_summary.schema_sha256,
            "portfolio_sha256": morphology_summary.portfolio_sha256,
            "schema_path": morphology_schema_path.display().to_string(),
            "portfolio_path": morphology_portfolio_path.display().to_string(),
            "physical_interchangeability_established": false,
            "winner_declared": false
        });
    }

    if let (Some(restoration_schema_path), Some(restoration_ledger_path)) =
        (restoration_schema_path, restoration_ledger_path)
    {
        let restoration_schema_raw =
            fs::read_to_string(&restoration_schema_path).map_err(|error| {
                format!("cannot read {}: {error}", restoration_schema_path.display())
            })?;
        let restoration_ledger_raw =
            fs::read_to_string(&restoration_ledger_path).map_err(|error| {
                format!("cannot read {}: {error}", restoration_ledger_path.display())
            })?;
        let restoration_summary =
            restoration::audit_restoration_texts(&restoration_schema_raw, &restoration_ledger_raw)
                .map_err(|errors| errors.join("\n"))?;

        report["repair_restoration_evidence"] = json!({
            "status": "PASS_SYNTHETIC_RESTORATION_CONTRACT_ONLY",
            "episode_count": restoration_summary.episode_count,
            "synthetic_episode_count": restoration_summary.synthetic_episode_count,
            "empirical_episode_count": restoration_summary.empirical_episode_count,
            "started_attempt_count_in_synthetic_fixture": restoration_summary.started_attempt_count,
            "verified_restoration_examples_in_synthetic_fixture": restoration_summary.verified_restoration_example_count,
            "completed_followup_examples_in_synthetic_fixture": restoration_summary.completed_followup_example_count,
            "recurrence_examples_in_synthetic_fixture": restoration_summary.recurrence_example_count,
            "durable_success_examples_in_synthetic_fixture": restoration_summary.durable_success_example_count,
            "benchmark_count": restoration_summary.benchmark_count,
            "schema_sha256": restoration_summary.schema_sha256,
            "ledger_sha256": restoration_summary.ledger_sha256,
            "schema_path": restoration_schema_path.display().to_string(),
            "ledger_path": restoration_ledger_path.display().to_string(),
            "physical_repair_outcomes_established": false,
            "product_safety_assessed": false,
            "architecture_winner_declared": false
        });
    }

    if let (Some(experiment_schema_path), Some(experiment_plan_path)) =
        (experiment_schema_path, experiment_plan_path)
    {
        let experiment_schema_raw =
            fs::read_to_string(&experiment_schema_path).map_err(|error| {
                format!("cannot read {}: {error}", experiment_schema_path.display())
            })?;
        let experiment_plan_raw = fs::read_to_string(&experiment_plan_path)
            .map_err(|error| format!("cannot read {}: {error}", experiment_plan_path.display()))?;
        let experiment_summary =
            experiment::audit_experiment_texts(&experiment_schema_raw, &experiment_plan_raw)
                .map_err(|errors| errors.join("\n"))?;

        report["repairability_factorial_experiment"] = json!({
            "status": "PASS_FACTORIAL_PLAN_CONTRACT_ONLY",
            "cell_count": experiment_summary.cell_count,
            "factor_count": experiment_summary.factor_count,
            "planned_trials_per_cell": experiment_summary.planned_trials_per_cell,
            "planned_trial_count": experiment_summary.planned_trial_count,
            "observed_trial_count": experiment_summary.observed_trial_count,
            "benchmark_count": experiment_summary.benchmark_count,
            "schema_sha256": experiment_summary.schema_sha256,
            "plan_sha256": experiment_summary.plan_sha256,
            "schema_path": experiment_schema_path.display().to_string(),
            "plan_path": experiment_plan_path.display().to_string(),
            "physical_trials_started": false,
            "observed_outcome_count": 0,
            "winner_declared": false
        });
    }

    if let (Some(compatibility_schema_path), Some(compatibility_graph_path)) =
        (compatibility_schema_path, compatibility_graph_path)
    {
        let compatibility_schema_raw =
            fs::read_to_string(&compatibility_schema_path).map_err(|error| {
                format!(
                    "cannot read {}: {error}",
                    compatibility_schema_path.display()
                )
            })?;
        let compatibility_graph_raw =
            fs::read_to_string(&compatibility_graph_path).map_err(|error| {
                format!(
                    "cannot read {}: {error}",
                    compatibility_graph_path.display()
                )
            })?;
        let compatibility_summary = compatibility::audit_compatibility_texts(
            &compatibility_schema_raw,
            &compatibility_graph_raw,
        )
        .map_err(|errors| errors.join("\n"))?;

        report["parts_compatibility_evidence_graph"] = json!({
            "status": "PASS_SYNTHETIC_PARTS_COMPATIBILITY_CONTRACT_ONLY",
            "model_count": compatibility_summary.model_count,
            "part_count": compatibility_summary.part_count,
            "claim_count": compatibility_summary.claim_count,
            "evidence_count": compatibility_summary.evidence_count,
            "supplier_path_count": compatibility_summary.supplier_path_count,
            "functional_claim_examples_in_synthetic_fixture": compatibility_summary.functional_claim_examples,
            "incompatibility_examples_in_synthetic_fixture": compatibility_summary.incompatible_claim_examples,
            "unknown_claim_examples_in_synthetic_fixture": compatibility_summary.unknown_claim_examples,
            "independent_delivered_groups_in_synthetic_fixture": compatibility_summary.independent_delivered_groups_in_fixture,
            "benchmark_count": compatibility_summary.benchmark_count,
            "schema_sha256": compatibility_summary.schema_sha256,
            "graph_sha256": compatibility_summary.graph_sha256,
            "schema_path": compatibility_schema_path.display().to_string(),
            "graph_path": compatibility_graph_path.display().to_string(),
            "real_compatibility_claims_established": false,
            "purchase_authorized": false,
            "product_safety_assessed": false
        });
    }

    if let (
        Some(ords_schema_path),
        Some(ords_mapping_path),
        Some(ords_csv_path),
        Some(ords_output_path),
    ) = (
        ords_schema_path,
        ords_mapping_path,
        ords_csv_path,
        ords_output_path,
    ) {
        let ords_schema_raw = fs::read_to_string(&ords_schema_path)
            .map_err(|error| format!("cannot read {}: {error}", ords_schema_path.display()))?;
        let ords_mapping_raw = fs::read_to_string(&ords_mapping_path)
            .map_err(|error| format!("cannot read {}: {error}", ords_mapping_path.display()))?;
        let ords_csv_raw = fs::read(&ords_csv_path)
            .map_err(|error| format!("cannot read {}: {error}", ords_csv_path.display()))?;
        let ords_summary = write_ords_output_atomically(
            &ords_schema_raw,
            &ords_mapping_raw,
            &ords_csv_raw,
            &ords_output_path,
        )?;

        report["ords_repair_data_interop"] = json!({
            "status": if ords_summary.quarantined_row_count == 0 {
                "PASS_IMPORT_CONTRACT_NO_QUARANTINES"
            } else {
                "PASS_IMPORT_CONTRACT_WITH_EXPLICIT_QUARANTINES"
            },
            "row_count": ords_summary.row_count,
            "accepted_row_count": ords_summary.accepted_row_count,
            "quarantined_row_count": ords_summary.quarantined_row_count,
            "fixed_row_count": ords_summary.fixed_row_count,
            "repairable_row_count": ords_summary.repairable_row_count,
            "end_of_life_row_count": ords_summary.end_of_life_row_count,
            "unknown_status_row_count": ords_summary.unknown_status_row_count,
            "category_candidate_row_count": ords_summary.category_candidate_row_count,
            "category_ambiguous_row_count": ords_summary.category_ambiguous_row_count,
            "category_unmapped_row_count": ords_summary.category_unmapped_row_count,
            "category_id_label_mismatch_row_count": ords_summary.category_id_label_mismatch_row_count,
            "duplicate_identity_row_count": ords_summary.duplicate_identity_row_count,
            "missing_identity_row_count": ords_summary.missing_identity_row_count,
            "invalid_status_row_count": ords_summary.invalid_status_row_count,
            "barrier_status_gate_violation_count": ords_summary.barrier_status_gate_violation_count,
            "invalid_end_of_life_barrier_count": ords_summary.invalid_end_of_life_barrier_count,
            "dataset_release_id": ords_summary.dataset_release_id,
            "source_license": ords_summary.source_license,
            "profile_status": ords_summary.profile_status,
            "schema_sha256": ords_summary.schema_sha256,
            "mapping_sha256": ords_summary.mapping_sha256,
            "input_sha256": ords_summary.input_sha256,
            "schema_path": ords_schema_path.display().to_string(),
            "mapping_path": ords_mapping_path.display().to_string(),
            "input_path": ords_csv_path.display().to_string(),
            "output_path": ords_output_path.display().to_string(),
            "all_rows_accounted_for": ords_summary.accepted_row_count + ords_summary.quarantined_row_count == ords_summary.row_count,
            "verified_restoration_claim_generated": false,
            "physical_safety_claim_generated": false,
            "exact_part_fit_claim_generated": false,
            "population_prevalence_claim_permitted": false
        });
    }

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
    fn failed_ords_import_preserves_existing_output_file() {
        let schema_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
            "../../schemas/appliances/ords_adapter_mapping_v0_1.schema.json",
        );
        let mapping_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
            "../../fixtures/appliances/ords_adapter_mapping_v0_1.json",
        );
        let csv_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/appliances/ords_v0_3_synthetic.csv");
        let schema = fs::read_to_string(schema_path).unwrap();
        let mut mapping: Value =
            serde_json::from_str(&fs::read_to_string(mapping_path).unwrap()).unwrap();
        mapping["policy"]["population_prevalence_claim_allowed"] = json!(true);
        let mapping = serde_json::to_string(&mapping).unwrap();
        let csv = fs::read(csv_path).unwrap();
        let output_path = std::env::temp_dir().join(format!(
            "symtropy-ords-atomic-output-{}.jsonl",
            std::process::id()
        ));
        fs::write(&output_path, b"preserve-existing-output").unwrap();

        let result = write_ords_output_atomically(&schema, &mapping, &csv, &output_path);
        assert!(result.is_err(), "weakened evidence policy must fail closed");
        assert_eq!(
            fs::read(&output_path).unwrap(),
            b"preserve-existing-output"
        );
        fs::remove_file(output_path).unwrap();
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
