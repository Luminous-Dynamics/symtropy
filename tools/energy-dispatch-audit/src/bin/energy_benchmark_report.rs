//! Produce a digest-bound benchmark ledger from the end-to-end Energy Studio run.
//!
//! Only completed, passing independent audits enter the ledger. The report
//! separates generation/validation/compilation/dispatch/audit timings from
//! one-time binary build cost, and records exact byte digests for replay.

use std::{
    collections::BTreeSet,
    env,
    error::Error,
    fs,
    io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use symtropy_energy_dispatch_audit::DispatchObjective;

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn required_str<'a>(value: &'a Value, key: &str) -> Result<&'a str, Box<dyn Error>> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, format!("missing string field {key}")).into())
}

fn required_u64(value: &Value, key: &str) -> Result<u64, Box<dyn Error>> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, format!("missing unsigned field {key}")).into())
}

fn required_bool(value: &Value, key: &str) -> Result<bool, Box<dyn Error>> {
    value
        .get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, format!("missing bool field {key}")).into())
}

fn read_json(path: &Path) -> Result<(Vec<u8>, Value), Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let value = serde_json::from_slice(&bytes)?;
    Ok((bytes, value))
}

fn read_elapsed_ns(dir: &Path, phase: &str) -> Result<u64, Box<dyn Error>> {
    let raw = fs::read_to_string(dir.join(format!("{phase}.elapsed_ns")))?;
    Ok(raw.trim().parse::<u64>()?)
}

fn expected_candidate_ids() -> Vec<String> {
    let mut ids = vec![
        "synthetic-grid-only".to_owned(),
        "synthetic-pv-grid".to_owned(),
        "synthetic-pv-battery-grid".to_owned(),
    ];
    for pv in [1, 2, 3] {
        for capacity in [1, 2, 3] {
            for battery_power in [1, 2] {
                for grid_power in [1, 2] {
                    ids.push(format!(
                        "synthetic-parametric-pv-{pv}kw-battery-{capacity}kwh-battery-power-{battery_power}kw-grid-{grid_power}kw"
                    ));
                }
            }
        }
    }
    ids
}

#[derive(Debug, Serialize)]
struct PhaseTimings {
    generation_ns: u64,
    schema_validation_ns: u64,
    compilation_ns: u64,
    receipt_verification_ns: u64,
    dispatch_ns: u64,
    independent_audit_ns: u64,
    measured_pipeline_total_ns: u64,
}

#[derive(Debug, Serialize)]
struct CandidateBenchmark {
    candidate_id: String,
    candidate_family: String,
    generation_measurement_mode: String,
    candidate_sha256: String,
    mapping_config_sha256: String,
    compilation_sha256: String,
    dispatch_request_sha256: String,
    dispatch_report_sha256: String,
    independent_audit_sha256: String,
    synthetic: bool,
    case_id: String,
    solver_reported_search_is_exact: bool,
    solver_reported_frontier_complete: bool,
    solver_evaluated_action_expansions: u64,
    solver_feasible_complete_schedules: u64,
    solver_pareto_frontier_count: u64,
    independent_audit_status: String,
    independent_reference_frontier_status: String,
    independent_reference_expansions: u64,
    independent_reference_feasible_complete_schedules: u64,
    independent_objective_claim_status: String,
    independently_recovered_frontier: Vec<Value>,
    audit_check_count: usize,
    timings: PhaseTimings,
}


#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct MappingConfigForLedger {
    schema_version: String,
    energy_quantum_kwh: f64,
    max_candidate_expansions: u64,
    grid_cost_units_per_quantum: u32,
    grid_carbon_grams_per_quantum: u32,
    factor_basis: String,
    synthetic: bool,
}

#[derive(Debug, Serialize)]
struct ObjectiveContextBenchmark {
    context_id: String,
    candidate_id: String,
    base_candidate_sha256: String,
    transformed_candidate_sha256: String,
    transform_receipt_sha256: String,
    mapping_config_file_sha256: String,
    mapping_config_sha256: String,
    compilation_sha256: String,
    dispatch_request_sha256: String,
    dispatch_report_sha256: String,
    independent_audit_sha256: String,
    golden_known_answer_sha256: String,
    grid_cost_units_per_quantum: u32,
    grid_carbon_grams_per_quantum: u32,
    solver_feasible_complete_schedules: u64,
    solver_pareto_frontier: Vec<DispatchObjective>,
    independent_reference_feasible_complete_schedules: u64,
    independent_reference_frontier: Vec<DispatchObjective>,
    objective_claim_status: String,
    audit_check_count: usize,
    timings: PhaseTimings,
}

#[derive(Debug, Serialize)]
struct MultiIntervalBenchmark {
    context_id: String,
    case_id: String,
    request_sha256: String,
    dispatch_report_sha256: String,
    independent_audit_sha256: String,
    qualification_log_sha256: String,
    known_answer_sha256: String,
    solver_feasible_complete_schedules: u64,
    solver_pareto_frontier: Vec<DispatchObjective>,
    independent_reference_feasible_complete_schedules: u64,
    independent_reference_frontier: Vec<DispatchObjective>,
    independent_audit_status: String,
    reference_frontier_status: String,
    objective_claim_status: String,
    audit_check_count: usize,
    dispatch_ns: u64,
    independent_audit_ns: u64,
    known_answer_gate_ns: u64,
}

#[derive(Debug, Serialize)]
struct SouthAfricanTariffBenchmark {
    context_id: String,
    tariff_id: String,
    tariff_schedule_sha256: String,
    known_answer_fixture_sha256: String,
    source_registry_sha256: String,
    calculator_report_sha256: String,
    qualification_log_sha256: String,
    source_receipts_sha256: String,
    source_status: String,
    account_verified: bool,
    case_count: usize,
    case_results: Vec<Value>,
    unapplied_charge_ids: Vec<String>,
    qualification_status: String,
    claim_ceiling: String,
}

#[derive(Debug, Serialize)]
struct SourceHeads {
    symthaea: String,
    symtropy_contract: String,
    sol_atlas: String,
    symtropy_auditor: String,
}

#[derive(Debug, Serialize)]
struct BenchmarkSummary {
    expected_candidate_count: usize,
    audited_candidate_count: usize,
    reference_template_count: usize,
    parametric_variant_count: usize,
    failed_or_unknown_candidate_count: usize,
    shared_design_space_generation_ns: u64,
    total_solver_action_expansions: u64,
    total_independent_reference_expansions: u64,
    total_generation_ns: u64,
    total_schema_validation_ns: u64,
    total_compilation_ns: u64,
    total_receipt_verification_ns: u64,
    total_dispatch_ns: u64,
    total_independent_audit_ns: u64,
    max_dispatch_ns: u64,
    mean_dispatch_ns: u64,
}

#[derive(Debug, Serialize)]
struct EnergyStudioBenchmarkReport {
    schema_version: &'static str,
    benchmark_id: String,
    created_by_workflow_run: String,
    git_ref: String,
    source_heads: SourceHeads,
    host_os: &'static str,
    host_arch: &'static str,
    rust_toolchain: &'static str,
    measurement_scope: &'static str,
    design_space_manifest_sha256: String,
    shared_candidate_schema_sha256: String,
    mapping_config_fixture_sha256: String,
    parametric_known_answers_sha256: String,
    symthaea_cargo_lock_sha256: String,
    sol_atlas_cargo_lock_sha256: String,
    symtropy_audit_cargo_lock_sha256: String,
    benchmark_summary: BenchmarkSummary,
    objective_context: ObjectiveContextBenchmark,
    multi_interval_context: MultiIntervalBenchmark,
    south_african_tariff_context: SouthAfricanTariffBenchmark,
    candidates: Vec<CandidateBenchmark>,
}

fn require_sha_match(
    label: &str,
    actual: &str,
    expected: &str,
) -> Result<(), Box<dyn Error>> {
    if actual != expected {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{label} mismatch: reported {actual}, recomputed {expected}"),
        )
        .into());
    }
    Ok(())
}

fn benchmark_candidate(
    root: &Path,
    candidate_id: &str,
) -> Result<CandidateBenchmark, Box<dyn Error>> {
    let dir = root.join(candidate_id);
    let (candidate_bytes, candidate) = read_json(&dir.join("candidate.json"))?;
    let (compilation_bytes, compilation) = read_json(&dir.join("compilation.json"))?;
    let (request_bytes, request) = read_json(&dir.join("dispatch-request.json"))?;
    let (dispatch_bytes, dispatch) = read_json(&dir.join("dispatch-report.json"))?;
    let (audit_bytes, audit) = read_json(&dir.join("independent-audit.json"))?;

    if required_str(&candidate, "candidate_id")? != candidate_id {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "candidate ID does not match output directory",
        )
        .into());
    }
    let candidate_is_synthetic = required_bool(&candidate, "synthetic")?;
    let expected_case_id = format!("candidate:{candidate_id}");
    if required_str(&compilation, "candidate_id")? != candidate_id
        || required_str(&request, "case_id")? != expected_case_id.as_str()
        || !candidate_is_synthetic
        || candidate_is_synthetic != required_bool(&request, "synthetic")?
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "candidate, compilation and dispatch-request identities disagree",
        )
        .into());
    }
    let receipt = dispatch.get("receipt").ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "dispatch report has no receipt object")
    })?;
    let assessment = dispatch.get("assessment").ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "dispatch report has no assessment object")
    })?;

    let candidate_digest = sha256_hex(&candidate_bytes);
    let mapping_config_digest = required_str(&compilation, "mapping_config_sha256")?.to_owned();
    let compilation_digest = sha256_hex(&compilation_bytes);
    let request_digest = sha256_hex(&request_bytes);
    let dispatch_digest = sha256_hex(&dispatch_bytes);
    let audit_digest = sha256_hex(&audit_bytes);

    require_sha_match(
        "compilation candidate digest",
        required_str(&compilation, "candidate_sha256")?,
        &candidate_digest,
    )?;
    require_sha_match(
        "compilation dispatch request digest",
        required_str(&compilation, "dispatch_request_sha256")?,
        &request_digest,
    )?;
    require_sha_match(
        "dispatch receipt request digest",
        required_str(receipt, "input_sha256")?,
        &request_digest,
    )?;
    require_sha_match(
        "audit request digest",
        required_str(&audit, "request_sha256")?,
        &request_digest,
    )?;
    require_sha_match(
        "audit run-report digest",
        required_str(&audit, "run_report_sha256")?,
        &dispatch_digest,
    )?;

    let solver_is_exact = required_bool(assessment, "search_is_exact")?;
    let solver_frontier_complete = required_bool(assessment, "pareto_frontier_complete")?;
    let solver_expansions = required_u64(assessment, "evaluated_action_expansions")?;
    let solver_feasible_schedules = required_u64(assessment, "feasible_complete_schedules")?;
    let solver_frontier_count = required_u64(assessment, "pareto_frontier_count")?;
    let audit_status = required_str(&audit, "status")?.to_owned();
    let reference_status = required_str(&audit, "reference_frontier_status")?.to_owned();
    let reference_expansions = audit
        .get("reference_expansions")
        .and_then(Value::as_u64)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "reference expansions are missing"))?;
    let reference_schedules = audit
        .get("reference_feasible_complete_schedules")
        .and_then(Value::as_u64)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "reference feasible schedule count is missing"))?;
    let objective_claim = required_str(&audit, "objective_claim_status")?.to_owned();
    let frontier = audit
        .get("reference_frontier_objectives")
        .and_then(Value::as_array)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "independent frontier is missing"))?
        .clone();
    let audit_checks = audit
        .get("checks")
        .and_then(Value::as_array)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "audit checks are missing"))?;
    let checks_all_pass = audit_checks.iter().all(|item| {
        item.get("status").and_then(Value::as_str) == Some("pass")
    });

    if audit_status != "pass"
        || reference_status != "pass"
        || objective_claim != "verified_for_bounded_model_case"
        || !checks_all_pass
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "candidate {candidate_id} failed independent qualification (audit={audit_status}, reference={reference_status}, objective={objective_claim})"
            ),
        )
        .into());
    }

    let (generation_ns, generation_measurement_mode) =
        if candidate_id.starts_with("synthetic-parametric-") {
            let raw = fs::read_to_string(root.join("manifest-generation.elapsed_ns"))?;
            (raw.trim().parse::<u64>()? / 36, "amortized_shared_manifest_export".to_owned())
        } else {
            (read_elapsed_ns(&dir, "generation")?, "individual_candidate_export".to_owned())
        };
    let timings = PhaseTimings {
        generation_ns,
        schema_validation_ns: read_elapsed_ns(&dir, "schema")?,
        compilation_ns: read_elapsed_ns(&dir, "compilation")?,
        receipt_verification_ns: read_elapsed_ns(&dir, "receipt")?,
        dispatch_ns: read_elapsed_ns(&dir, "dispatch")?,
        independent_audit_ns: read_elapsed_ns(&dir, "audit")?,
        measured_pipeline_total_ns: 0,
    };
    let measured_total = timings.generation_ns
        .saturating_add(timings.schema_validation_ns)
        .saturating_add(timings.compilation_ns)
        .saturating_add(timings.receipt_verification_ns)
        .saturating_add(timings.dispatch_ns)
        .saturating_add(timings.independent_audit_ns);
    let timings = PhaseTimings {
        measured_pipeline_total_ns: measured_total,
        ..timings
    };

    let family = if candidate_id.starts_with("synthetic-parametric-") {
        "parametric_sizing".to_owned()
    } else {
        "reference_topology".to_owned()
    };
    Ok(CandidateBenchmark {
        candidate_id: candidate_id.to_owned(),
        candidate_family: family,
        generation_measurement_mode,
        candidate_sha256: candidate_digest,
        mapping_config_sha256: mapping_config_digest,
        compilation_sha256: compilation_digest,
        dispatch_request_sha256: request_digest,
        dispatch_report_sha256: dispatch_digest,
        independent_audit_sha256: audit_digest,
        synthetic: required_bool(&candidate, "synthetic")?,
        case_id: required_str(assessment, "case_id")?.to_owned(),
        solver_reported_search_is_exact: solver_is_exact,
        solver_reported_frontier_complete: solver_frontier_complete,
        solver_evaluated_action_expansions: solver_expansions,
        solver_feasible_complete_schedules: solver_feasible_schedules,
        solver_pareto_frontier_count: solver_frontier_count,
        independent_audit_status: audit_status,
        independent_reference_frontier_status: reference_status,
        independent_reference_expansions: reference_expansions,
        independent_reference_feasible_complete_schedules: reference_schedules,
        independent_objective_claim_status: objective_claim,
        independently_recovered_frontier: frontier,
        audit_check_count: audit_checks.len(),
        timings,
    })
}

fn sum_checked<F>(
    candidates: &[CandidateBenchmark],
    mut select: F,
) -> Result<u64, Box<dyn Error>>
where
    F: FnMut(&CandidateBenchmark) -> u64,
{
    candidates.iter().try_fold(0_u64, |total, candidate| {
        total.checked_add(select(candidate)).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "benchmark metric sum overflow").into()
        })
    })
}


fn benchmark_objective_context(root: &Path) -> Result<ObjectiveContextBenchmark, Box<dyn Error>> {
    let dir = root.join("objective-context");
    let (base_bytes, base_candidate) = read_json(&root.join("synthetic-pv-battery-grid/candidate.json"))?;
    let (candidate_bytes, candidate) = read_json(&dir.join("candidate.json"))?;
    let (transform_receipt_bytes, transform_receipt) = read_json(&dir.join("transform-receipt.json"))?;
    let (config_bytes, config_value) = read_json(&dir.join("mapping-config.json"))?;
    let config: MappingConfigForLedger = serde_json::from_value(config_value)?;
    let (compilation_bytes, compilation) = read_json(&dir.join("compilation.json"))?;
    let (request_bytes, request) = read_json(&dir.join("dispatch-request.json"))?;
    let (dispatch_bytes, dispatch) = read_json(&dir.join("dispatch-report.json"))?;
    let (audit_bytes, audit) = read_json(&dir.join("independent-audit.json"))?;

    let base_digest = sha256_hex(&base_bytes);
    let candidate_digest = sha256_hex(&candidate_bytes);
    let transformation_digest = sha256_hex(&transform_receipt_bytes);
    let config_digest = sha256_hex(&config_bytes);
    let compilation_digest = sha256_hex(&compilation_bytes);
    let request_digest = sha256_hex(&request_bytes);
    let dispatch_digest = sha256_hex(&dispatch_bytes);
    let audit_digest = sha256_hex(&audit_bytes);

    let context_id = "synthetic-pv-derate-nonzero-economics";
    let candidate_id = "synthetic-pv-limited-battery-grid";
    if required_str(&base_candidate, "candidate_id")? != "synthetic-pv-battery-grid"
        || required_str(&candidate, "candidate_id")? != candidate_id
        || required_str(&transform_receipt, "base_candidate_sha256")? != base_digest
        || required_str(&transform_receipt, "transformed_candidate_sha256")? != candidate_digest
        || required_str(&transform_receipt, "transform_id")? != "pv-availability-50pct-one-interval"
        || required_str(&compilation, "candidate_sha256")? != candidate_digest
        || required_str(&compilation, "candidate_id")? != candidate_id
        || required_str(&compilation, "dispatch_request_sha256")? != request_digest
        || required_str(&dispatch["receipt"], "input_sha256")? != request_digest.as_str()
        || required_str(&audit, "request_sha256")? != request_digest.as_str()
        || required_str(&audit, "run_report_sha256")? != dispatch_digest.as_str()
        || required_str(&request, "case_id")? != "candidate:synthetic-pv-limited-battery-grid"
        || !required_bool(&candidate, "synthetic")?
        || !required_bool(&request, "synthetic")?
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "objective-context chain of custody or candidate identity mismatch",
        ).into());
    }

    let typed_config_bytes = serde_json::to_vec(&config)?;
    require_sha_match(
        "objective-context mapping config digest",
        required_str(&compilation, "mapping_config_sha256")?,
        &sha256_hex(&typed_config_bytes),
    )?;
    if !config.synthetic
        || config.schema_version != "energy-dispatch-mapping-v0.1"
        || config.energy_quantum_kwh != 1.0
        || config.max_candidate_expansions != 100_000
        || config.grid_cost_units_per_quantum != 5
        || config.grid_carbon_grams_per_quantum != 100
        || config.factor_basis.trim().is_empty()
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "non-zero objective-context mapping factors differ from the frozen context",
        ).into());
    }

    let golden_bytes = include_bytes!("../../fixtures/energy-objective-context-known-answer-v0.1.json");
    let golden: Value = serde_json::from_slice(golden_bytes)?;
    if required_str(&golden, "context_id")? != context_id
        || required_str(&golden, "candidate_id")? != candidate_id
        || required_u64(&golden, "feasible_complete_schedules")? != 11
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "frozen objective-context known answer has the wrong identity/count",
        ).into());
    }
    let expected_frontier: Vec<DispatchObjective> =
        serde_json::from_value(golden["pareto_frontier"].clone())?;
    let solver_frontier: Vec<DispatchObjective> = dispatch["assessment"]["pareto_alternatives"]
        .as_array()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "objective-context solver frontier missing"))?
        .iter()
        .map(|alternative| serde_json::from_value(alternative["objective"].clone()))
        .collect::<Result<Vec<_>, _>>()?;
    let independent_frontier: Vec<DispatchObjective> =
        serde_json::from_value(audit["reference_frontier_objectives"].clone())?;
    let audit_checks = audit["checks"].as_array()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "objective-context audit checks missing"))?;
    if required_str(&audit, "status")? != "pass"
        || required_str(&audit, "reference_frontier_status")? != "pass"
        || required_str(&audit, "objective_claim_status")? != "verified_for_bounded_model_case"
        || required_u64(&audit, "reference_feasible_complete_schedules")? != 11
        || required_u64(&dispatch["assessment"], "feasible_complete_schedules")? != 11
        || required_u64(&dispatch["assessment"], "pareto_frontier_count")? != expected_frontier.len() as u64
        || solver_frontier != expected_frontier
        || independent_frontier != expected_frontier
        || audit_checks.iter().any(|item| item["status"].as_str() != Some("pass"))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "objective-context dispatch/audit disagrees with the frozen non-zero economics Pareto answer",
        ).into());
    }

    let timings = PhaseTimings {
        generation_ns: read_elapsed_ns(&dir, "generation")?,
        schema_validation_ns: read_elapsed_ns(&dir, "schema")?,
        compilation_ns: read_elapsed_ns(&dir, "compilation")?,
        receipt_verification_ns: read_elapsed_ns(&dir, "receipt")?,
        dispatch_ns: read_elapsed_ns(&dir, "dispatch")?,
        independent_audit_ns: read_elapsed_ns(&dir, "audit")?,
        measured_pipeline_total_ns: 0,
    };
    let measured_total = timings.generation_ns
        .saturating_add(timings.schema_validation_ns)
        .saturating_add(timings.compilation_ns)
        .saturating_add(timings.receipt_verification_ns)
        .saturating_add(timings.dispatch_ns)
        .saturating_add(timings.independent_audit_ns);
    let timings = PhaseTimings {
        measured_pipeline_total_ns: measured_total,
        ..timings
    };

    Ok(ObjectiveContextBenchmark {
        context_id: context_id.to_owned(),
        candidate_id: candidate_id.to_owned(),
        base_candidate_sha256: base_digest,
        transformed_candidate_sha256: candidate_digest,
        transform_receipt_sha256: transformation_digest,
        mapping_config_file_sha256: config_digest,
        mapping_config_sha256: sha256_hex(&typed_config_bytes),
        compilation_sha256: compilation_digest,
        dispatch_request_sha256: request_digest,
        dispatch_report_sha256: dispatch_digest,
        independent_audit_sha256: audit_digest,
        golden_known_answer_sha256: sha256_hex(golden_bytes),
        grid_cost_units_per_quantum: config.grid_cost_units_per_quantum,
        grid_carbon_grams_per_quantum: config.grid_carbon_grams_per_quantum,
        solver_feasible_complete_schedules: required_u64(&dispatch["assessment"], "feasible_complete_schedules")?,
        solver_pareto_frontier: solver_frontier,
        independent_reference_feasible_complete_schedules: required_u64(&audit, "reference_feasible_complete_schedules")?,
        independent_reference_frontier: independent_frontier,
        objective_claim_status: required_str(&audit, "objective_claim_status")?.to_owned(),
        audit_check_count: audit_checks.len(),
        timings,
    })
}

fn benchmark_multi_interval_context(
    root: &Path,
) -> Result<MultiIntervalBenchmark, Box<dyn Error>> {
    let dir = root.join("multi-interval-context");
    let (request_bytes, request) = read_json(&dir.join("request.json"))?;
    let (dispatch_bytes, dispatch) = read_json(&dir.join("dispatch-report.json"))?;
    let (audit_bytes, audit) = read_json(&dir.join("independent-audit.json"))?;
    let known_bytes = include_bytes!("../../fixtures/three-interval-stress-known-answer-v0.1.json");
    let known: Value = serde_json::from_slice(known_bytes)?;
    let qualification_log = fs::read(dir.join("qualification.log"))?;

    let request_digest = sha256_hex(&request_bytes);
    let dispatch_digest = sha256_hex(&dispatch_bytes);
    let audit_digest = sha256_hex(&audit_bytes);
    let known_digest = sha256_hex(known_bytes);
    let qualification_digest = sha256_hex(&qualification_log);
    let case_id = "synthetic-three-interval-storage-tradeoff";

    if required_str(&request, "case_id")? != case_id
        || required_str(&known, "case_id")? != case_id
        || required_str(&dispatch["assessment"], "case_id")? != case_id
        || required_str(&dispatch["receipt"], "case_id")? != case_id
        || required_str(&audit, "case_id")? != case_id
        || !required_bool(&request, "synthetic")?
        || !required_bool(&dispatch["assessment"], "synthetic")?
        || request["intervals"].as_array().map(Vec::len) != Some(3)
        || required_str(&dispatch["receipt"], "input_sha256")? != request_digest
        || required_u64(&dispatch["receipt"], "input_bytes")? != request_bytes.len() as u64
        || required_str(&audit, "request_sha256")? != request_digest
        || required_str(&audit, "run_report_sha256")? != dispatch_digest
        || required_u64(&dispatch["assessment"], "feasible_complete_schedules")?
            != required_u64(&known, "feasible_complete_schedules")?
        || required_u64(&audit, "reference_feasible_complete_schedules")?
            != required_u64(&known, "feasible_complete_schedules")?
        || required_str(&audit, "status")? != "pass"
        || required_str(&audit, "reference_frontier_status")? != "pass"
        || required_str(&audit, "objective_claim_status")?
            != "verified_for_bounded_model_case"
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "multi-interval benchmark evidence chain or qualification status mismatch",
        )
        .into());
    }

    let alternatives = dispatch["assessment"]["pareto_alternatives"]
        .as_array()
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "multi-interval dispatch alternatives are missing",
            )
        })?;
    let mut solver_frontier = alternatives
        .iter()
        .map(|item| serde_json::from_value::<DispatchObjective>(item["objective"].clone()))
        .collect::<Result<Vec<_>, _>>()?;
    let mut independent_frontier: Vec<DispatchObjective> =
        serde_json::from_value(audit["reference_frontier_objectives"].clone())?;
    let mut expected_frontier: Vec<DispatchObjective> =
        serde_json::from_value(known["pareto_frontier"].clone())?;
    solver_frontier.sort();
    independent_frontier.sort();
    expected_frontier.sort();

    if required_u64(&dispatch["assessment"], "pareto_frontier_count")?
        != solver_frontier.len() as u64
        || solver_frontier != expected_frontier
        || independent_frontier != expected_frontier
        || alternatives.iter().any(|alternative| {
            alternative["schedule"].as_array().map(Vec::len) != Some(3)
        })
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "multi-interval solver/reference frontier differs from the frozen known answer",
        )
        .into());
    }

    let log_text = String::from_utf8(qualification_log)?;
    if !log_text.contains("PASS: actual Sol-Atlas multi-interval dispatch")
        || !log_text.contains(&format!("request_sha256={request_digest}"))
        || !log_text.contains(&format!("dispatch_report_sha256={dispatch_digest}"))
        || !log_text.contains(&format!("independent_audit_sha256={audit_digest}"))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "multi-interval qualification log does not bind the passing evidence digests",
        )
        .into());
    }

    let audit_checks = audit["checks"].as_array().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "multi-interval audit checks are missing")
    })?;
    if audit_checks.iter().any(|item| item["status"].as_str() != Some("pass")) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "multi-interval audit contains a non-passing or unknown check",
        )
        .into());
    }

    Ok(MultiIntervalBenchmark {
        context_id: "three-interval-storage-tradeoff-known-answer-v0.1".to_owned(),
        case_id: case_id.to_owned(),
        request_sha256: request_digest,
        dispatch_report_sha256: dispatch_digest,
        independent_audit_sha256: audit_digest,
        qualification_log_sha256: qualification_digest,
        known_answer_sha256: known_digest,
        solver_feasible_complete_schedules: required_u64(
            &dispatch["assessment"],
            "feasible_complete_schedules",
        )?,
        solver_pareto_frontier: solver_frontier,
        independent_reference_feasible_complete_schedules: required_u64(
            &audit,
            "reference_feasible_complete_schedules",
        )?,
        independent_reference_frontier: independent_frontier,
        independent_audit_status: required_str(&audit, "status")?.to_owned(),
        reference_frontier_status: required_str(&audit, "reference_frontier_status")?.to_owned(),
        objective_claim_status: required_str(&audit, "objective_claim_status")?.to_owned(),
        audit_check_count: audit_checks.len(),
        dispatch_ns: read_elapsed_ns(&dir, "dispatch")?,
        independent_audit_ns: read_elapsed_ns(&dir, "audit")?,
        known_answer_gate_ns: read_elapsed_ns(&dir, "known-answer-gate")?,
    })
}

fn benchmark_south_african_tariff_context(
    root: &Path,
) -> Result<SouthAfricanTariffBenchmark, Box<dyn Error>> {
    let dir = root.join("sa-tariff-context");
    let (schedule_bytes, schedule) = read_json(&dir.join("tariff-schedule.json"))?;
    let (known_bytes, known) = read_json(&dir.join("known-answers.json"))?;
    let (registry_bytes, registry) = read_json(&dir.join("source-registry.json"))?;
    let (report_bytes, report) = read_json(&dir.join("report.json"))?;
    let log_bytes = fs::read(dir.join("qualification.log"))?;
    let receipt_bytes = fs::read(dir.join("source-receipts.sha256"))?;

    let schedule_digest = sha256_hex(&schedule_bytes);
    let known_digest = sha256_hex(&known_bytes);
    let registry_digest = sha256_hex(&registry_bytes);
    let report_digest = sha256_hex(&report_bytes);
    let log_digest = sha256_hex(&log_bytes);
    let receipt_digest = sha256_hex(&receipt_bytes);
    let case_id_expected = [
        "low-season-2600kwh-under-50kva",
        "high-season-1200kwh-over-50-to-100kva",
    ];
    let registry_site = registry.get("site").ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "source registry site boundary is missing")
    })?;
    let registry_source_ids = registry["sources"]
        .as_array()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "source registry public source list is missing"))?
        .iter()
        .map(|item| required_str(item, "source_id").map(str::to_owned))
        .collect::<Result<Vec<_>, _>>()?;
    let required_source_ids = [
        "joburg-fy2026-27-business-tariff-schedule",
        "augos-city-power-business-conventional-2026-27",
        "dffe-south-africa-2024-grid-emission-factors",
        "ec-jrc-pvgis-v6",
    ];

    if required_str(&registry, "schema_version")?
            != "energy-sa-benchmark-source-registry-v0.1"
        || required_str(&registry, "registry_status")?
            != "source_inventory_only_not_site_calibrated"
        || registry_site.get("measured_load_profile").is_none()
        || registry_site.get("customer_tariff_code").is_none()
        || !registry_site["measured_load_profile"].is_null()
        || !registry_site["customer_tariff_code"].is_null()
        || required_source_ids.iter().any(|required| {
            !registry_source_ids.iter().any(|observed| observed.as_str() == *required)
        })
        || required_str(&schedule, "schema_version")? != "energy-sa-tariff-schedule-v0.1"
        || required_str(&schedule, "tariff_id")?
            != "city-power-business-conventional-2026-27-candidate"
        || required_str(&schedule, "source_status")?
            != "candidate_schedule_not_account_verified"
        || required_bool(&schedule, "account_verified")?
        || required_str(&known, "schema_version")? != "energy-sa-tariff-known-answers-v0.1"
        || required_str(&known, "schedule_id")? != required_str(&schedule, "tariff_id")?
        || required_str(&report, "schema_version")?
            != "energy-sa-tariff-reference-report-v0.1"
        || required_str(&report, "tariff_id")? != required_str(&schedule, "tariff_id")?
        || required_str(&report, "tariff_schedule_sha256")? != schedule_digest.as_str()
        || required_str(&report, "known_answer_fixture_sha256")? != known_digest.as_str()
        || required_bool(&report, "account_verified")?
        || required_str(&report, "source_status")?
            != "candidate_schedule_not_account_verified"
        || required_u64(&report, "result_count")? != case_id_expected.len() as u64
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "South African tariff evidence chain, source status or candidate-only claim boundary mismatch",
        )
        .into());
    }

    let cases = known["cases"].as_array().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "tariff known-answer cases missing")
    })?;
    let results = report["results"].as_array().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "tariff reference results missing")
    })?;
    let expected_unapplied = schedule["deliberately_unapplied_charges"]
        .as_array()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "unapplied-charge declarations missing"))?
        .iter()
        .map(|item| required_str(item, "charge_id").map(str::to_owned))
        .collect::<Result<Vec<_>, _>>()?;

    if cases.len() != case_id_expected.len() || results.len() != cases.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "unexpected number of South African tariff known-answer cases/results",
        )
        .into());
    }

    for index in 0..case_id_expected.len() {
        let case = &cases[index];
        let result = &results[index];
        if required_str(case, "case_id")? != case_id_expected[index]
            || required_str(result, "case_id")? != case_id_expected[index]
            || required_bool(result, "account_verified")?
            || required_u64(result, "energy_charge_microzar")?
                != required_u64(case, "expected_energy_charge_microzar")?
            || required_u64(result, "service_charge_microzar")?
                != required_u64(case, "expected_service_charge_microzar")?
            || required_u64(result, "network_charge_microzar")?
                != required_u64(case, "expected_network_charge_microzar")?
            || required_u64(result, "subtotal_excluding_unapplied_and_tax_microzar")?
                != required_u64(case, "expected_subtotal_excluding_unapplied_and_tax_microzar")?
            || result["computed_block_use"] != case["expected_block_use"]
            || result["unapplied_charge_ids"]
                != Value::Array(expected_unapplied.iter().cloned().map(Value::String).collect())
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("tariff result does not match the frozen arithmetic answer for {}", case_id_expected[index]),
            )
            .into());
        }
        let sum = required_u64(result, "energy_charge_microzar")?
            .checked_add(required_u64(result, "service_charge_microzar")?)
            .and_then(|subtotal| subtotal.checked_add(required_u64(result, "network_charge_microzar").ok()?))
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "tariff ledger subtotal overflow"))?;
        if sum != required_u64(result, "subtotal_excluding_unapplied_and_tax_microzar")? {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "tariff subtotal is not the exact sum of represented energy/service/network charges",
            )
            .into());
        }
    }

    let receipts = String::from_utf8(receipt_bytes)?;
    let log = String::from_utf8(log_bytes)?;
    for digest in [&schedule_digest, &known_digest, &registry_digest, &report_digest] {
        if !receipts.contains(digest) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("tariff source receipt does not include digest {digest}"),
            )
            .into());
        }
    }
    if !log.contains("PASS: computed 2 exact tariff-reference cases")
        || !log.contains(&format!("tariff_schedule_sha256={schedule_digest}"))
        || !log.contains(&format!("known_answer_fixture_sha256={known_digest}"))
        || !log.contains(&format!("report_sha256={report_digest}"))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "tariff qualification log does not bind the exact schedule, answers and calculator report",
        )
        .into());
    }

    Ok(SouthAfricanTariffBenchmark {
        context_id: "south-africa-city-power-business-conventional-tariff-reference-v0.1".to_owned(),
        tariff_id: required_str(&schedule, "tariff_id")?.to_owned(),
        tariff_schedule_sha256: schedule_digest,
        known_answer_fixture_sha256: known_digest,
        source_registry_sha256: registry_digest,
        calculator_report_sha256: report_digest,
        qualification_log_sha256: log_digest,
        source_receipts_sha256: receipt_digest,
        source_status: required_str(&report, "source_status")?.to_owned(),
        account_verified: required_bool(&report, "account_verified")?,
        case_count: results.len(),
        case_results: results.clone(),
        unapplied_charge_ids: expected_unapplied,
        qualification_status: "verified_exact_arithmetic_for_candidate_schedule_only".to_owned(),
        claim_ceiling: "Not a customer bill, account tariff determination, tariff interpretation or savings claim; unresolved charges and taxes remain unapplied.".to_owned(),
    })
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 3 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: energy_benchmark_report <candidate-root-directory> <output-report.json>",
        )
        .into());
    }
    let root = PathBuf::from(&args[1]);
    let output = PathBuf::from(&args[2]);
    let expected_ids = expected_candidate_ids();
    let expected_set = expected_ids.iter().cloned().collect::<BTreeSet<_>>();
    let manifest_path = root.join("energy-design-space-manifest.json");
    let (manifest_bytes, manifest) = read_json(&manifest_path)?;
    let schema_bytes = fs::read(root.join("energy-system-candidate-v0.1.schema.json"))?;
    let mapping_config_bytes = fs::read(
        root.join("sol-atlas/schemas/examples/energy-dispatch-mapping-config.synthetic.json"),
    )?;
    let symthaea_lock_bytes = fs::read(root.join("symthaea.Cargo.lock"))?;
    let sol_atlas_lock_bytes = fs::read(root.join("sol-atlas.Cargo.lock"))?;
    let symtropy_audit_lock_bytes = fs::read(root.join("symtropy-energy-audit.Cargo.lock"))?;
    if manifest.get("requested_candidate_count").and_then(Value::as_u64) != Some(36)
        || manifest.get("candidates").and_then(Value::as_array).map(Vec::len) != Some(36)
        || manifest.get("rejected_candidates").and_then(Value::as_array).map(Vec::len) != Some(0)
    {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "design-space manifest is not the full 36-candidate sweep").into());
    }

    let mut observed_ids = fs::read_dir(&root)?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().ok().is_some_and(|kind| kind.is_dir()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.starts_with("synthetic-"))
        .collect::<Vec<_>>();
    observed_ids.sort();
    let mut expected_sorted = expected_ids.clone();
    expected_sorted.sort();
    if observed_ids != expected_sorted || observed_ids.len() != 39 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "candidate directory set does not match the frozen 39-case benchmark (observed {})",
                observed_ids.len()
            ),
        )
        .into());
    }
    if expected_set.len() != 39 {
        return Err(io::Error::other("internal expected candidate list has duplicate IDs").into());
    }

    let mut candidates = expected_ids
        .iter()
        .map(|id| benchmark_candidate(&root, id))
        .collect::<Result<Vec<_>, _>>()?;
    candidates.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));

    let solver_expansions = sum_checked(&candidates, |candidate| candidate.solver_evaluated_action_expansions)?;
    let reference_expansions = sum_checked(&candidates, |candidate| candidate.independent_reference_expansions)?;
    let total_generation = sum_checked(&candidates, |candidate| candidate.timings.generation_ns)?;
    let shared_design_space_generation = fs::read_to_string(root.join("manifest-generation.elapsed_ns"))?
        .trim()
        .parse::<u64>()?;
    let total_schema = sum_checked(&candidates, |candidate| candidate.timings.schema_validation_ns)?;
    let total_compilation = sum_checked(&candidates, |candidate| candidate.timings.compilation_ns)?;
    let total_receipt = sum_checked(&candidates, |candidate| candidate.timings.receipt_verification_ns)?;
    let total_dispatch = sum_checked(&candidates, |candidate| candidate.timings.dispatch_ns)?;
    let total_audit = sum_checked(&candidates, |candidate| candidate.timings.independent_audit_ns)?;
    let max_dispatch = candidates.iter().map(|candidate| candidate.timings.dispatch_ns).max().unwrap_or(0);
    let mean_dispatch = total_dispatch / candidates.len() as u64;

    let objective_context = benchmark_objective_context(&root)?;
    let multi_interval_context = benchmark_multi_interval_context(&root)?;
    let south_african_tariff_context = benchmark_south_african_tariff_context(&root)?;
    let report = EnergyStudioBenchmarkReport {
        schema_version: "energy-studio-benchmark-report-v0.3",
        benchmark_id: "energy-studio-source-bounded-multi-context-benchmark-v0.3".to_owned(),
        created_by_workflow_run: env::var("GITHUB_RUN_ID").unwrap_or_else(|_| "local".to_owned()),
        git_ref: env::var("GITHUB_SHA").unwrap_or_else(|_| "unknown".to_owned()),
        source_heads: SourceHeads {
            symthaea: env::var("SYMTHAEA_PIN")?,
            symtropy_contract: env::var("SYMTROPY_CONTRACT_PIN")?,
            sol_atlas: env::var("SOL_ATLAS_PIN")?,
            symtropy_auditor: env::var("GITHUB_SHA").unwrap_or_else(|_| "unknown".to_owned()),
        },
        host_os: env::consts::OS,
        host_arch: env::consts::ARCH,
        rust_toolchain: "1.96.0",
        measurement_scope: "Per-candidate wall-clock process runtimes from a warmed build; excludes one-time dependency compilation and runner provisioning. These synthetic finite-case measurements are not a real-site or SOTA comparison.",
        design_space_manifest_sha256: sha256_hex(&manifest_bytes),
        shared_candidate_schema_sha256: sha256_hex(&schema_bytes),
        mapping_config_fixture_sha256: sha256_hex(&mapping_config_bytes),
        parametric_known_answers_sha256: sha256_hex(include_bytes!(
            "../../fixtures/energy-parametric-known-answers-v0.1.json"
        )),
        symthaea_cargo_lock_sha256: sha256_hex(&symthaea_lock_bytes),
        sol_atlas_cargo_lock_sha256: sha256_hex(&sol_atlas_lock_bytes),
        symtropy_audit_cargo_lock_sha256: sha256_hex(&symtropy_audit_lock_bytes),
        benchmark_summary: BenchmarkSummary {
            expected_candidate_count: 39,
            audited_candidate_count: candidates.len(),
            reference_template_count: 3,
            parametric_variant_count: 36,
            failed_or_unknown_candidate_count: 0,
            shared_design_space_generation_ns: shared_design_space_generation,
            total_solver_action_expansions: solver_expansions,
            total_independent_reference_expansions: reference_expansions,
            total_generation_ns: total_generation,
            total_schema_validation_ns: total_schema,
            total_compilation_ns: total_compilation,
            total_receipt_verification_ns: total_receipt,
            total_dispatch_ns: total_dispatch,
            total_independent_audit_ns: total_audit,
            max_dispatch_ns: max_dispatch,
            mean_dispatch_ns: mean_dispatch,
        },
        objective_context,
        multi_interval_context,
        south_african_tariff_context,
        candidates,
    };
    let output_bytes = serde_json::to_vec_pretty(&report)?;
    fs::write(&output, &output_bytes)?;
    println!(
        "PASS: benchmark ledger recorded {} audited candidates; total dispatch={} ns; mean dispatch={} ns; max dispatch={} ns",
        report.benchmark_summary.audited_candidate_count,
        report.benchmark_summary.total_dispatch_ns,
        report.benchmark_summary.mean_dispatch_ns,
        report.benchmark_summary.max_dispatch_ns
    );
    println!("report_sha256={}", sha256_hex(&output_bytes));
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("energy benchmark report failed: {error}");
        std::process::exit(2);
    }
}
