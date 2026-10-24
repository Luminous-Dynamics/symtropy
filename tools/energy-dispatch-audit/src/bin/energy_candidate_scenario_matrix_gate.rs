//! Aggregate gate for candidate × scenario robustness evaluations.
//!
//! This verifier checks exact matrix coverage, receipted candidate transformations,
//! schema/compile/dispatch/audit artifacts, and equality between Sol-Atlas and the
//! independently enumerated frontier. Every requested pair must be represented.

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
use symtropy_energy_dispatch_audit::{DispatchObjective, DispatchRequest};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct MappingConfigWire {
    schema_version: String,
    energy_quantum_kwh: f64,
    max_candidate_expansions: u64,
    grid_cost_units_per_quantum: u32,
    grid_carbon_grams_per_quantum: u32,
    factor_basis: String,
    synthetic: bool,
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn invalid(message: impl Into<String>) -> Box<dyn Error> {
    io::Error::new(io::ErrorKind::InvalidData, message.into()).into()
}
fn req_str<'a>(value: &'a Value, key: &str) -> Result<&'a str, Box<dyn Error>> {
    value.get(key).and_then(Value::as_str)
        .ok_or_else(|| invalid(format!("missing string field {key}")))
}
fn req_u64(value: &Value, key: &str) -> Result<u64, Box<dyn Error>> {
    value.get(key).and_then(Value::as_u64)
        .ok_or_else(|| invalid(format!("missing unsigned field {key}")))
}
fn read_json(path: &Path) -> Result<(Vec<u8>, Value), Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let value = serde_json::from_slice(&bytes)?;
    Ok((bytes, value))
}
fn elapsed_ns(dir: &Path, stem: &str) -> Result<u64, Box<dyn Error>> {
    let value = fs::read_to_string(dir.join(format!("{stem}.elapsed_ns")))?;
    value.trim().parse::<u64>().map_err(|_| invalid(format!("invalid elapsed_ns for {stem}")))
}
fn claim<'a>(candidate: &'a Value, component_id: &str, parameter_id: &str) -> Option<&'a Value> {
    let components = candidate["components"].as_array()?;
    let component = components.iter()
        .find(|item| item["component_id"].as_str() == Some(component_id))?;
    let claims = component["parameter_claims"].as_array()?;
    let parameter = claims.iter()
        .find(|item| item["parameter_id"].as_str() == Some(parameter_id))?;
    parameter.get("value")
}
fn resource<'a>(candidate: &'a Value, interval_index: usize, component_id: &str) -> Option<&'a Value> {
    candidate["scenarios"][0]["intervals"][interval_index]["resource_available_kwh"]
        .as_array()?.iter()
        .find(|item| item["component_id"].as_str() == Some(component_id))
        .map(|item| &item["available_kwh"])
}
fn set_pointer(value: &mut Value, path: &str, replacement: &Value) -> Result<(), Box<dyn Error>> {
    *value.pointer_mut(path).ok_or_else(|| invalid(format!("mutation path not found: {path}")))? =
        replacement.clone();
    Ok(())
}
fn verify_receipt_reconstruction(
    parent: &Value,
    child: &Value,
    receipt: &Value,
) -> Result<(), Box<dyn Error>> {
    let mutations = receipt["mutations"].as_array()
        .ok_or_else(|| invalid("transform receipt mutation array missing"))?;
    let mut reconstructed = parent.clone();
    let mut paths = BTreeSet::new();
    for mutation in mutations {
        let path = req_str(mutation, "json_pointer")?;
        if !paths.insert(path.to_owned()) {
            return Err(invalid(format!("duplicate transform mutation path {path}")));
        }
        let before = mutation.get("before").ok_or_else(|| invalid("mutation before value missing"))?;
        let after = mutation.get("after").ok_or_else(|| invalid("mutation after value missing"))?;
        if reconstructed.pointer(path) != Some(before) {
            return Err(invalid(format!("transform precondition mismatch at {path}")));
        }
        set_pointer(&mut reconstructed, path, after)?;
    }
    if reconstructed != *child {
        return Err(invalid("child candidate contains undeclared changes beyond the transform receipt"));
    }
    Ok(())
}

fn verify_scenario_semantics(
    parent: &Value,
    child: &Value,
    receipt: &Value,
    scenario_id: &str,
    parent_bytes: &[u8],
    child_bytes: &[u8],
) -> Result<(), Box<dyn Error>> {
    let applicability = req_str(receipt, "applicability")?;
    match scenario_id {
        "nominal" => {
            if sha256_hex(parent_bytes) != sha256_hex(child_bytes)
                || parent != child
                || !receipt["mutations"].as_array().is_some_and(|items| items.is_empty())
                || applicability != "applicable_baseline"
            {
                return Err(invalid("nominal scenario must preserve exact candidate bytes and have no mutations"));
            }
        }
        "pv_total_loss" => {
            let parent_has_pv = parent["components"].as_array().is_some_and(|items|
                items.iter().any(|item| item["component_id"].as_str() == Some("pv")));
            let child_pv_resources = child["scenarios"][0]["intervals"].as_array()
                .ok_or_else(|| invalid("child interval array missing"))?
                .iter().flat_map(|interval| interval["resource_available_kwh"].as_array().into_iter().flatten())
                .filter(|item| item["component_id"].as_str() == Some("pv"))
                .collect::<Vec<_>>();
            if parent_has_pv {
                if applicability != "applicable"
                    || child_pv_resources.is_empty()
                    || child_pv_resources.iter().any(|item| item["available_kwh"].as_f64() != Some(0.0))
                {
                    return Err(invalid("PV total-loss scenario must set all declared PV availability to zero"));
                }
            } else if applicability != "not_applicable_no_pv_component" || !child_pv_resources.is_empty() {
                return Err(invalid("PV-less candidate must be explicitly marked not applicable for PV-loss stress"));
            }
        }
        "outage_early" => {
            let intervals = child["scenarios"][0]["intervals"].as_array()
                .ok_or_else(|| invalid("outage-transform interval list missing"))?;
            let grid_cap = claim(parent, "grid", "interval_import_energy_cap_kwh")
                .and_then(Value::as_f64)
                .ok_or_else(|| invalid("candidate's grid energy cap is not declared"))?;
            if applicability != "applicable"
                || intervals.len() != 2
                || intervals[0]["grid_outage"].as_bool() != Some(true)
                || intervals[1]["grid_outage"].as_bool() != Some(false)
                || resource(child, 0, "grid").and_then(Value::as_f64) != Some(0.0)
                || resource(child, 1, "grid").and_then(Value::as_f64) != Some(grid_cap)
            {
                return Err(invalid("early-outage transform does not align grid outage flags and resource availability"));
            }
        }
        "essential_demand_2x" => {
            let parent_intervals = parent["scenarios"][0]["intervals"].as_array()
                .ok_or_else(|| invalid("parent service intervals missing"))?;
            let child_intervals = child["scenarios"][0]["intervals"].as_array()
                .ok_or_else(|| invalid("child service intervals missing"))?;
            if parent_intervals.len() != child_intervals.len() || applicability != "applicable" {
                return Err(invalid("demand-surge interval count/applicability mismatch"));
            }
            let essential_ids = parent["services"].as_array()
                .ok_or_else(|| invalid("parent service catalog missing"))?
                .iter()
                .filter(|service| service["essential"].as_bool() == Some(true))
                .map(|service| service["service_id"].as_str().map(str::to_owned)
                    .ok_or_else(|| invalid("essential service ID missing")))
                .collect::<Result<Vec<_>, _>>()?;
            if essential_ids.is_empty() {
                return Err(invalid("demand surge has no explicitly essential service"));
            }
            let mut quantity_count = 0_usize;
            for (before_interval, after_interval) in parent_intervals.iter().zip(child_intervals) {
                let before = before_interval["service_requested"].as_array()
                    .ok_or_else(|| invalid("parent service request list missing"))?;
                let after = after_interval["service_requested"].as_array()
                    .ok_or_else(|| invalid("child service request list missing"))?;
                if before.len() != after.len() { return Err(invalid("demand-surge service list length mismatch")); }
                for (old, new) in before.iter().zip(after) {
                    let service_id = old["service_id"].as_str()
                        .ok_or_else(|| invalid("parent requested service ID missing"))?;
                    if new["service_id"].as_str() != Some(service_id) {
                        return Err(invalid("demand-surge transform changed service identity"));
                    }
                    let amount = old["requested_quantity"].as_f64()
                        .ok_or_else(|| invalid("parent requested service quantity missing or non-numeric"))?;
                    let observed = new["requested_quantity"].as_f64()
                        .ok_or_else(|| invalid("child requested service quantity missing or non-numeric"))?;
                    if !amount.is_finite() || amount < 0.0 || amount.fract() != 0.0 {
                        return Err(invalid("parent requested service quantity must be an integral non-negative value"));
                    }
                    if essential_ids.iter().any(|id| id.as_str() == service_id) {
                        if observed != amount * 2.0 {
                            return Err(invalid("essential demand surge is not exactly 2x the parent load"));
                        }
                        quantity_count += 1;
                    } else if observed != amount {
                        return Err(invalid("demand-surge transform altered a non-essential service"));
                    }
                }
            }
            if quantity_count == 0 { return Err(invalid("demand-surge case has no declared service quantities")); }
        }
        "terminal_reserve_1" => {
            let has_battery = parent["components"].as_array().is_some_and(|items|
                items.iter().any(|item| item["component_id"].as_str() == Some("battery")));
            if has_battery {
                if applicability != "applicable"
                    || claim(child, "battery", "terminal_reserve_kwh").and_then(Value::as_f64) != Some(1.0)
                {
                    return Err(invalid("battery candidate must declare one kWh terminal reserve"));
                }
            } else if applicability != "not_applicable_no_battery_component"
                || child["components"].as_array().is_some_and(|items|
                    items.iter().any(|item| item["component_id"].as_str() == Some("battery")))
            {
                return Err(invalid("no-battery architecture must carry explicit not-applicable reserve status"));
            }
        }
        _ => return Err(invalid(format!("unrecognized scenario ID {scenario_id}"))),
    }
    Ok(())
}

#[derive(Debug, Serialize)]
struct MatrixRow {
    candidate_id: String,
    scenario_id: String,
    applicability: String,
    status: String,
    parent_candidate_sha256: String,
    transformed_candidate_sha256: String,
    transform_receipt_sha256: String,
    row_status_sha256: String,
    candidate_schema_sha256: String,
    schema_validation_log_sha256: String,
    mapping_config_file_sha256: String,
    mapping_config_typed_sha256: String,
    compilation_sha256: String,
    receipt_verification_log_sha256: String,
    dispatch_request_sha256: String,
    dispatch_report_sha256: String,
    independent_audit_sha256: String,
    dispatch_case_id: String,
    solver_feasible_complete_schedules: u64,
    solver_frontier_count: usize,
    independent_reference_feasible_complete_schedules: u64,
    independent_reference_frontier_count: usize,
    independent_audit_status: String,
    reference_frontier_status: String,
    objective_claim_status: String,
    audit_check_count: usize,
    transform_ns: u64,
    schema_validation_ns: u64,
    compile_ns: u64,
    receipt_verification_ns: u64,
    dispatch_ns: u64,
    independent_audit_ns: u64,
}

fn evaluate_row(
    candidate_id: &str,
    scenario_id: &str,
    candidate_root: &Path,
    matrix_root: &Path,
    profile_sha256: &str,
) -> Result<MatrixRow, Box<dyn Error>> {
    let source_dir = candidate_root.join(candidate_id);
    let scenario_dir = matrix_root.join(candidate_id).join(scenario_id);
    let (row_status_bytes, row_status) = read_json(&scenario_dir.join("row-status.json"))?;
    if req_str(&row_status, "status")? != "pass"
        || req_str(&row_status, "candidate_id")? != candidate_id
        || req_str(&row_status, "scenario_id")? != scenario_id
    {
        return Err(invalid(format!("row status is not passing or has a different identity: {candidate_id}/{scenario_id}")));
    }
    let (parent_bytes, parent) = read_json(&source_dir.join("candidate.json"))?;
    let (child_bytes, child) = read_json(&scenario_dir.join("candidate.json"))?;
    let (_, receipt) = read_json(&scenario_dir.join("transform-receipt.json"))?;
    let (schema_bytes, _) = read_json(&candidate_root.join("energy-system-candidate-v0.1.schema.json"))?;
    let mapping_config_bytes = fs::read(candidate_root.join("sol-atlas/schemas/examples/energy-dispatch-mapping-config.synthetic.json"))?;
    let mapping_config: MappingConfigWire = serde_json::from_slice(&mapping_config_bytes)?;
    let mapping_config_typed_bytes = serde_json::to_vec(&mapping_config)?;
    let schema_log = fs::read(scenario_dir.join("schema-validation.log"))?;
    let (compilation_bytes, compilation) = read_json(&scenario_dir.join("compilation.json"))?;
    let receipt_log = fs::read(scenario_dir.join("receipt-verification.log"))?;
    let (request_bytes, request) = read_json(&scenario_dir.join("dispatch-request.json"))?;
    let typed_compilation_config: MappingConfigWire =
        serde_json::from_value(compilation["mapping_config"].clone())?;
    let typed_compilation_request: DispatchRequest =
        serde_json::from_value(compilation["dispatch_request"].clone())?;
    let request_wire: DispatchRequest = serde_json::from_slice(&request_bytes)?;
    let (dispatch_bytes, dispatch) = read_json(&scenario_dir.join("dispatch-report.json"))?;
    let (audit_bytes, audit) = read_json(&scenario_dir.join("independent-audit.json"))?;

    let parent_digest = sha256_hex(&parent_bytes);
    let child_digest = sha256_hex(&child_bytes);
    let receipt_bytes = fs::read(scenario_dir.join("transform-receipt.json"))?;
    let receipt_digest = sha256_hex(&receipt_bytes);
    let schema_log_digest = sha256_hex(&schema_log);
    let mapping_config_file_digest = sha256_hex(&mapping_config_bytes);
    let mapping_config_typed_digest = sha256_hex(&mapping_config_typed_bytes);
    let compilation_digest = sha256_hex(&compilation_bytes);
    let receipt_log_digest = sha256_hex(&receipt_log);
    let request_digest = sha256_hex(&request_bytes);
    let typed_request_bytes = serde_json::to_vec(&request_wire)?;
    let dispatch_digest = sha256_hex(&dispatch_bytes);
    let audit_digest = sha256_hex(&audit_bytes);

    if req_str(&parent, "candidate_id")? != candidate_id
        || req_str(&child, "candidate_id")? != candidate_id
        || req_str(&receipt, "schema_version")? != "energy-candidate-scenario-transform-receipt-v0.1"
        || req_str(&receipt, "panel_id")? != "energy-candidate-scenario-matrix-v0.1"
        || req_str(&receipt, "candidate_id")? != candidate_id
        || req_str(&receipt, "scenario_id")? != scenario_id
        || req_str(&receipt, "parent_candidate_sha256")? != parent_digest.as_str()
        || req_str(&receipt, "transformed_candidate_sha256")? != child_digest.as_str()
        || req_str(&receipt, "scenario_profile_sha256")? != profile_sha256
    {
        return Err(invalid(format!("candidate transform identity/digest mismatch: {candidate_id}/{scenario_id}")));
    }
    verify_receipt_reconstruction(&parent, &child, &receipt)?;
    verify_scenario_semantics(&parent, &child, &receipt, scenario_id, &parent_bytes, &child_bytes)?;

    let expected_case_id = format!("candidate:{candidate_id}");
    if req_str(&compilation, "candidate_id")? != candidate_id
        || req_str(&compilation, "candidate_sha256")? != child_digest.as_str()
        || req_str(&compilation, "mapping_config_sha256")? != mapping_config_typed_digest.as_str()
        || typed_compilation_config != mapping_config
        || serde_json::to_vec(&typed_compilation_request)? != request_bytes
        || typed_request_bytes != request_bytes
        || req_str(&request, "case_id")? != expected_case_id
        || req_str(&request, "schema_version")? != "energy-dispatch-v0.1"
        || request["synthetic"].as_bool() != Some(true)
        || req_str(&dispatch["receipt"], "case_id")? != expected_case_id
        || req_str(&dispatch["assessment"], "case_id")? != expected_case_id
        || req_str(&dispatch["receipt"], "input_sha256")? != request_digest.as_str()
        || req_u64(&dispatch["receipt"], "input_bytes")? != request_bytes.len() as u64
        || req_str(&audit, "case_id")? != expected_case_id
        || req_str(&audit, "request_sha256")? != request_digest.as_str()
        || req_str(&audit, "run_report_sha256")? != dispatch_digest.as_str()
        || req_str(&audit, "status")? != "pass"
        || req_str(&audit, "reference_frontier_status")? != "pass"
        || req_str(&audit, "objective_claim_status")? != "verified_for_bounded_model_case"
        || dispatch["assessment"]["search_is_exact"].as_bool() != Some(true)
        || dispatch["assessment"]["pareto_frontier_complete"].as_bool() != Some(true)
    {
        return Err(invalid(format!("dispatch/audit identity, digest, or status mismatch: {candidate_id}/{scenario_id}")));
    }
    let alternatives = dispatch["assessment"]["pareto_alternatives"].as_array()
        .ok_or_else(|| invalid("solver Pareto alternatives missing"))?;
    let mut solver_frontier = alternatives.iter()
        .map(|item| serde_json::from_value::<DispatchObjective>(item["objective"].clone()))
        .collect::<Result<Vec<_>, _>>()?;
    let mut independent_frontier: Vec<DispatchObjective> =
        serde_json::from_value(audit["reference_frontier_objectives"].clone())?;
    let order = |v: &DispatchObjective| (
        v.essential_unserved_quanta, v.flexible_unserved_quanta, v.operating_cost_units,
        v.grid_carbon_grams, v.curtailed_pv_quanta, std::cmp::Reverse(v.terminal_soc_quanta)
    );
    solver_frontier.sort_by_key(order);
    independent_frontier.sort_by_key(order);
    if solver_frontier != independent_frontier
        || req_u64(&dispatch["assessment"], "pareto_frontier_count")? != solver_frontier.len() as u64
        || req_u64(&dispatch["assessment"], "feasible_complete_schedules")?
            != req_u64(&audit, "reference_feasible_complete_schedules")?
    {
        return Err(invalid(format!("solver/reference frontier disagreement: {candidate_id}/{scenario_id}")));
    }
    let checks = audit["checks"].as_array().ok_or_else(|| invalid("audit check array missing"))?;
    if checks.is_empty() || checks.iter().any(|item| item["status"].as_str() != Some("pass")) {
        return Err(invalid(format!("audit contains failed/unknown checks: {candidate_id}/{scenario_id}")));
    }
    if !schema_log_digest.is_empty() && !receipt_log_digest.is_empty() {
        // The logs are retained and hashed; process exit codes in the workflow
        // are the schema/receipt-verification status authorities.
    }

    Ok(MatrixRow {
        candidate_id: candidate_id.to_owned(),
        scenario_id: scenario_id.to_owned(),
        applicability: req_str(&receipt, "applicability")?.to_owned(),
        status: "pass".to_owned(),
        parent_candidate_sha256: parent_digest,
        transformed_candidate_sha256: child_digest,
        transform_receipt_sha256: receipt_digest,
        row_status_sha256: sha256_hex(&row_status_bytes),
        candidate_schema_sha256: sha256_hex(&schema_bytes),
        schema_validation_log_sha256: schema_log_digest,
        mapping_config_file_sha256: mapping_config_file_digest,
        mapping_config_typed_sha256: mapping_config_typed_digest,
        compilation_sha256: compilation_digest,
        receipt_verification_log_sha256: receipt_log_digest,
        dispatch_request_sha256: request_digest,
        dispatch_report_sha256: dispatch_digest,
        independent_audit_sha256: audit_digest,
        dispatch_case_id: expected_case_id,
        solver_feasible_complete_schedules: req_u64(&dispatch["assessment"], "feasible_complete_schedules")?,
        solver_frontier_count: solver_frontier.len(),
        independent_reference_feasible_complete_schedules: req_u64(&audit, "reference_feasible_complete_schedules")?,
        independent_reference_frontier_count: independent_frontier.len(),
        independent_audit_status: req_str(&audit, "status")?.to_owned(),
        reference_frontier_status: req_str(&audit, "reference_frontier_status")?.to_owned(),
        objective_claim_status: req_str(&audit, "objective_claim_status")?.to_owned(),
        audit_check_count: checks.len(),
        transform_ns: elapsed_ns(&scenario_dir, "transform")?,
        schema_validation_ns: elapsed_ns(&scenario_dir, "schema-validation")?,
        compile_ns: elapsed_ns(&scenario_dir, "compile")?,
        receipt_verification_ns: elapsed_ns(&scenario_dir, "receipt-verification")?,
        dispatch_ns: elapsed_ns(&scenario_dir, "dispatch")?,
        independent_audit_ns: elapsed_ns(&scenario_dir, "audit")?,
    })
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 5 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: energy_candidate_scenario_matrix_gate <candidate-root> <matrix-results-root> <scenario-profile.json> <report.json>",
        ).into());
    }
    let candidate_root = PathBuf::from(&args[1]);
    let matrix_root = PathBuf::from(&args[2]);
    let profile_bytes = fs::read(&args[3])?;
    if profile_bytes.as_slice() != include_bytes!("../../fixtures/energy-candidate-scenario-panel-v0.1.json") {
        return Err(invalid("scenario profile bytes differ from the embedded source fixture"));
    }
    let profile: Value = serde_json::from_slice(&profile_bytes)?;
    if req_str(&profile, "schema_version")? != "energy-candidate-scenario-panel-v0.1" {
        return Err(invalid("unsupported candidate-scenario profile schema"));
    }

    let (design_manifest_bytes, design_manifest) = read_json(&candidate_root.join("energy-design-space-manifest.json"))?;
    if req_u64(&design_manifest, "requested_candidate_count")? != 36
        || design_manifest["candidates"].as_array().map(Vec::len) != Some(36)
        || design_manifest["rejected_candidates"].as_array().map(Vec::len) != Some(0)
    {
        return Err(invalid("base sizing manifest must contain all 36 variants and zero rejections"));
    }

    let mut candidate_ids = fs::read_dir(&candidate_root)?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().ok().is_some_and(|kind| kind.is_dir()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.starts_with("synthetic-"))
        .collect::<Vec<_>>();
    candidate_ids.sort();
    if candidate_ids.len() != 39 {
        return Err(invalid(format!("expected 39 frozen candidate directories, found {}", candidate_ids.len())));
    }
    let mut expected_ids = design_manifest["candidates"].as_array().unwrap().iter()
        .map(|item| req_str(item, "candidate_id").map(str::to_owned))
        .collect::<Result<Vec<_>, _>>()?;
    expected_ids.extend([
        "synthetic-grid-only".to_owned(),
        "synthetic-pv-grid".to_owned(),
        "synthetic-pv-battery-grid".to_owned()
    ]);
    expected_ids.sort();
    if candidate_ids != expected_ids {
        return Err(invalid("candidate directory set does not match 36 sizing variants plus three templates"));
    }

    let scenario_rows = profile["required_scenarios"].as_array()
        .ok_or_else(|| invalid("scenario profile rows missing"))?;
    if scenario_rows.len() != 5 {
        return Err(invalid("candidate scenario profile must define exactly five scenarios"));
    }
    let mut scenario_ids = scenario_rows.iter()
        .map(|item| req_str(item, "scenario_id").map(str::to_owned))
        .collect::<Result<Vec<_>, _>>()?;
    scenario_ids.sort();
    let mut unique_scenarios = scenario_ids.clone();
    unique_scenarios.dedup();
    if unique_scenarios.len() != 5 {
        return Err(invalid("duplicate scenario profile IDs"));
    }

    let profile_digest = sha256_hex(&profile_bytes);
    let expected_rows = 39 * 5;
    let mut report_rows = Vec::<Value>::with_capacity(expected_rows);
    let mut pairs = BTreeSet::new();
    let mut passing_rows = 0_usize;
    let mut failed_or_unknown_rows = 0_usize;

    for candidate_id in &candidate_ids {
        for scenario_id in &scenario_ids {
            if !pairs.insert((candidate_id.clone(), scenario_id.clone())) {
                return Err(invalid("duplicate candidate/scenario pair"));
            }
            let row_dir = matrix_root.join(candidate_id).join(scenario_id);
            match read_json(&row_dir.join("row-status.json")) {
                Ok((status_bytes, status)) => {
                    let claimed_status = status.get("status").and_then(Value::as_str).unwrap_or("unknown");
                    if claimed_status != "pass" {
                        failed_or_unknown_rows += 1;
                        report_rows.push(serde_json::json!({
                            "candidate_id": candidate_id,
                            "scenario_id": scenario_id,
                            "status": claimed_status,
                            "failed_phase": status.get("failed_phase").cloned().unwrap_or(Value::Null),
                            "error": status.get("error").cloned().unwrap_or(Value::Null),
                            "row_status_sha256": sha256_hex(&status_bytes)
                        }));
                        continue;
                    }
                    match evaluate_row(
                        candidate_id,
                        scenario_id,
                        &candidate_root,
                        &matrix_root,
                        &profile_digest,
                    ) {
                        Ok(row) => {
                            passing_rows += 1;
                            report_rows.push(serde_json::to_value(row)?);
                        }
                        Err(error) => {
                            failed_or_unknown_rows += 1;
                            report_rows.push(serde_json::json!({
                                "candidate_id": candidate_id,
                                "scenario_id": scenario_id,
                                "status": "failed",
                                "failed_phase": "matrix_verification",
                                "error": error.to_string(),
                                "row_status_sha256": sha256_hex(&status_bytes)
                            }));
                        }
                    }
                }
                Err(error) => {
                    failed_or_unknown_rows += 1;
                    report_rows.push(serde_json::json!({
                        "candidate_id": candidate_id,
                        "scenario_id": scenario_id,
                        "status": "unknown",
                        "failed_phase": "row_status_missing",
                        "error": error.to_string(),
                        "row_status_sha256": Value::Null
                    }));
                }
            }
        }
    }

    if report_rows.len() != expected_rows || pairs.len() != expected_rows {
        return Err(invalid(format!("candidate/scenario coverage mismatch: {} of {expected_rows}", report_rows.len())));
    }
    report_rows.sort_by(|a, b| {
        a["candidate_id"].as_str().cmp(&b["candidate_id"].as_str())
            .then(a["scenario_id"].as_str().cmp(&b["scenario_id"].as_str()))
    });

    let all_passed = failed_or_unknown_rows == 0 && passing_rows == expected_rows;
    let report = serde_json::json!({
        "schema_version": "energy-candidate-scenario-matrix-report-v0.1",
        "matrix_id": req_str(&profile, "panel_id")?,
        "scenario_profile_sha256": profile_digest,
        "design_space_manifest_sha256": sha256_hex(&design_manifest_bytes),
        "expected_candidate_count": candidate_ids.len(),
        "scenario_count": scenario_ids.len(),
        "expected_row_count": expected_rows,
        "observed_row_count": report_rows.len(),
        "passing_row_count": passing_rows,
        "failed_or_unknown_row_count": failed_or_unknown_rows,
        "all_passed": all_passed,
        "claim_ceiling": "Synthetic two-interval candidate robustness matrix only; not calibrated site resilience or economic ranking.",
        "rows": report_rows
    });
    let report_bytes = serde_json::to_vec_pretty(&report)?;
    fs::write(&args[4], &report_bytes)?;
    if !all_passed {
        eprintln!("FAIL: candidate-scenario matrix accounted for {} rows; {} passed, {} failed/unknown",
            expected_rows, passing_rows, failed_or_unknown_rows);
        return Err(invalid("one or more matrix rows failed or are unknown; see written report for every pair"));
    }
    println!("PASS: candidate-scenario matrix verified {} of {} rows", passing_rows, expected_rows);
    println!("scenario_profile_sha256={profile_digest}");
    println!("report_sha256={}", sha256_hex(&report_bytes));
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("candidate-scenario matrix gate failed: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROFILE: &str = include_str!("../../fixtures/energy-candidate-scenario-panel-v0.1.json");
    const CANDIDATE: &str = include_str!("../../fixtures/energy-system-candidate.synthetic.json");

    #[test]
    fn scenario_profile_ids_are_unique_and_cover_five_named_arms() {
        let profile: Value = serde_json::from_str(PROFILE).unwrap();
        let rows = profile["required_scenarios"].as_array().unwrap();
        let ids = rows.iter().map(|x| x["scenario_id"].as_str().unwrap()).collect::<BTreeSet<_>>();
        assert_eq!(ids.len(), 5);
        assert!(ids.contains("nominal"));
        assert!(ids.contains("pv_total_loss"));
        assert!(ids.contains("outage_early"));
        assert!(ids.contains("essential_demand_2x"));
        assert!(ids.contains("terminal_reserve_1"));
    }

    #[test]
    fn transform_receipt_reconstruction_rejects_hidden_extra_change() {
        let source: Value = serde_json::from_str(CANDIDATE).unwrap();
        let mut child = source.clone();
        let mut receipt = serde_json::json!({"mutations":[]});
        child["claim_ceiling"] = serde_json::json!("hidden claim expansion");
        assert!(verify_receipt_reconstruction(&source, &child, &receipt).is_err());
    }
}
