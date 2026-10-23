//! Gate the explicit non-zero-cost/non-zero-carbon PV stress context end-to-end.
//!
//! Verifies provenance from the generated base candidate through the named
//! candidate transformation, compilation receipt, exact dispatch bytes, and
//! independent audit against a frozen objective-frontier known answer.

use std::{env, error::Error, fs, io, path::Path};

use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use symtropy_energy_dispatch_audit::DispatchObjective;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MappingConfig {
    schema_version: String,
    energy_quantum_kwh: f64,
    max_candidate_expansions: u64,
    grid_cost_units_per_quantum: u32,
    grid_carbon_grams_per_quantum: u32,
    factor_basis: String,
    synthetic: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedContext {
    schema_version: String,
    context_id: String,
    candidate_id: String,
    base_candidate_id: String,
    transformation_id: String,
    cost_basis: CostBasis,
    carbon_basis: CarbonBasis,
    input_context: Value,
    feasible_complete_schedules: u64,
    pareto_frontier: Vec<DispatchObjective>,
    derivation: String,
    claim_ceiling: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CostBasis {
    grid_cost_units_per_quantum: u32,
    unit: String,
    meaning: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CarbonBasis {
    grid_carbon_grams_per_quantum: u32,
    unit: String,
    meaning: String,
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn read_bytes(path: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(fs::read(Path::new(path))?)
}

fn read_json(path: &str) -> Result<(Vec<u8>, Value), Box<dyn Error>> {
    let bytes = read_bytes(path)?;
    let value = serde_json::from_slice(&bytes)?;
    Ok((bytes, value))
}

fn string_field<'a>(value: &'a Value, field: &str) -> Result<&'a str, Box<dyn Error>> {
    value.get(field).and_then(Value::as_str).ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, format!("missing string field {field}")).into()
    })
}

fn reject(reason: &str) -> Box<dyn Error> {
    io::Error::new(io::ErrorKind::InvalidData, reason.to_owned()).into()
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 9 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: energy_objective_context_gate <base-candidate.json> <transformed-candidate.json> <transform-receipt.json> <mapping-config.json> <compilation.json> <dispatch-request.json> <dispatch-report.json> <independent-audit.json>",
        ).into());
    }

    let (base_bytes, base) = read_json(&args[1])?;
    let (candidate_bytes, candidate) = read_json(&args[2])?;
    let (_, transform_receipt) = read_json(&args[3])?;
    let (_, config_value) = read_json(&args[4])?;
    let (_, compilation) = read_json(&args[5])?;
    let (request_bytes, request) = read_json(&args[6])?;
    let (dispatch_bytes, dispatch) = read_json(&args[7])?;
    let (_, audit) = read_json(&args[8])?;

    let known: ExpectedContext = serde_json::from_str(include_str!(
        "../../fixtures/energy-objective-context-known-answer-v0.1.json"
    ))?;
    let config: MappingConfig = serde_json::from_value(config_value.clone())?;

    if known.schema_version != "energy-objective-context-known-answer-v0.1"
        || known.context_id != "synthetic-pv-derate-nonzero-economics"
        || known.candidate_id != "synthetic-pv-limited-battery-grid"
        || known.base_candidate_id != "synthetic-pv-battery-grid"
        || known.transformation_id != "pv-availability-50pct-one-interval"
        || known.feasible_complete_schedules != 11
        || known.pareto_frontier.len() != 5
        || known.derivation.trim().is_empty()
        || known.claim_ceiling.trim().is_empty()
    {
        return Err(reject("frozen objective-context known-answer fixture failed its header contract"));
    }

    if string_field(&base, "candidate_id")? != known.base_candidate_id
        || string_field(&candidate, "candidate_id")? != known.candidate_id
        || base.get("synthetic").and_then(Value::as_bool) != Some(true)
        || candidate.get("synthetic").and_then(Value::as_bool) != Some(true)
    {
        return Err(reject("base/transformed candidate identity or synthetic boundary mismatch"));
    }

    let base_digest = sha256_hex(&base_bytes);
    let candidate_digest = sha256_hex(&candidate_bytes);
    if string_field(&transform_receipt, "base_candidate_sha256")? != base_digest
        || string_field(&transform_receipt, "transformed_candidate_sha256")? != candidate_digest
        || string_field(&transform_receipt, "base_candidate_id")? != known.base_candidate_id
        || string_field(&transform_receipt, "transformed_candidate_id")? != known.candidate_id
        || string_field(&transform_receipt, "transform_id")? != known.transformation_id
    {
        return Err(reject("PV stress-transform receipt does not bind the exact parent and transformed bytes"));
    }
    if (candidate["scenarios"][0]["intervals"][0]["resource_available_kwh"]
        .as_array()
        .is_none_or(|resources| {
            !resources.iter().any(|resource| {
                resource.get("component_id").and_then(Value::as_str) == Some("pv")
                    && resource.get("available_kwh").and_then(Value::as_f64) == Some(1.0)
            })
        })
    {
        return Err(reject("transformed candidate does not contain the frozen 1 kWh PV stress sample"));
    }

    if config.schema_version != "energy-dispatch-mapping-v0.1"
        || !config.synthetic
        || config.energy_quantum_kwh != 1.0
        || config.max_candidate_expansions != 100_000
        || config.grid_cost_units_per_quantum != known.cost_basis.grid_cost_units_per_quantum
        || config.grid_carbon_grams_per_quantum != known.carbon_basis.grid_carbon_grams_per_quantum
        || config.factor_basis.trim().is_empty()
        || known.cost_basis.unit != "synthetic_integer_cost_units"
        || known.carbon_basis.unit != "grams_per_declared_grid_quantum"
        || known.cost_basis.meaning.trim().is_empty()
        || known.carbon_basis.meaning.trim().is_empty()
        || known.input_context["pv_available_energy_kwh_first_interval"] != 1
        || known.input_context["base_pv_available_energy_kwh_first_interval"] != 2
    {
        return Err(reject("mapping config or golden objective-context factors do not match the declared synthetic case"));
    }

    let typed_config_bytes = serde_json::to_vec(&config)?;
    if string_field(&compilation, "candidate_sha256")? != candidate_digest
        || string_field(&compilation, "candidate_id")? != known.candidate_id
        || string_field(&compilation, "mapping_config_sha256")? != sha256_hex(&typed_config_bytes)
        || request.get("case_id").and_then(Value::as_str) != Some("candidate:synthetic-pv-limited-battery-grid")
        || request.get("synthetic").and_then(Value::as_bool) != Some(true)
        || request.get("energy_quantum_kwh").and_then(Value::as_f64) != Some(config.energy_quantum_kwh)
        || request.get("max_candidate_expansions").and_then(Value::as_u64) != Some(config.max_candidate_expansions)
        || request["intervals"].as_array().is_none_or(|intervals| {
            intervals.iter().any(|interval| {
                interval.get("grid_cost_units_per_quantum").and_then(Value::as_u64)
                    != Some(u64::from(config.grid_cost_units_per_quantum))
                    || interval.get("grid_carbon_grams_per_quantum").and_then(Value::as_u64)
                        != Some(u64::from(config.grid_carbon_grams_per_quantum))
            })
        })
    {
        return Err(reject("compilation receipt/request does not bind the candidate and non-zero economic context"));
    }

    if string_field(&compilation, "dispatch_request_sha256")? != sha256_hex(&request_bytes)
        || string_field(&dispatch["receipt"], "input_sha256")? != sha256_hex(&request_bytes)
        || string_field(&audit, "request_sha256")? != sha256_hex(&request_bytes)
        || string_field(&audit, "run_report_sha256")? != sha256_hex(&dispatch_bytes)
    {
        return Err(reject("compiled request, dispatch receipt, and independent audit do not bind identical bytes"));
    }

    let audit_status = string_field(&audit, "status")?;
    let reference_status = string_field(&audit, "reference_frontier_status")?;
    let objective_status = string_field(&audit, "objective_claim_status")?;
    let independent_schedules = audit["reference_feasible_complete_schedules"]
        .as_u64()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "reference schedule count missing"))?;
    let independent_frontier: Vec<DispatchObjective> =
        serde_json::from_value(audit["reference_frontier_objectives"].clone())?;
    let solver_frontier: Vec<DispatchObjective> = dispatch["assessment"]["pareto_alternatives"]
        .as_array()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "solver frontier missing"))?
        .iter()
        .map(|alternative| serde_json::from_value(alternative["objective"].clone()))
        .collect::<Result<Vec<_>, _>>()?;
    let checks_all_pass = audit["checks"].as_array().is_some_and(|checks| {
        checks.iter().all(|check| check["status"].as_str() == Some("pass"))
    });

    let accepted = audit_status == "pass"
        && reference_status == "pass"
        && objective_status == "verified_for_bounded_model_case"
        && independent_schedules == known.feasible_complete_schedules
        && independent_frontier == known.pareto_frontier
        && solver_frontier == known.pareto_frontier
        && checks_all_pass
        && dispatch["assessment"]["pareto_frontier_count"].as_u64()
            == Some(known.pareto_frontier.len() as u64)
        && dispatch["assessment"]["feasible_complete_schedules"].as_u64()
            == Some(known.feasible_complete_schedules);

    println!(
        "context_id={} candidate_id={} audit_status={} reference_frontier={} schedules={}/{} frontier_vectors={}/{} grid_cost_units_per_quantum={} grid_carbon_grams_per_quantum={}",
        known.context_id,
        known.candidate_id,
        audit_status,
        reference_status,
        independent_schedules,
        known.feasible_complete_schedules,
        independent_frontier.len(),
        known.pareto_frontier.len(),
        config.grid_cost_units_per_quantum,
        config.grid_carbon_grams_per_quantum
    );
    if !accepted {
        return Err(reject("non-zero economics known-answer gate rejected the solver report"));
    }
    println!("PASS: generated candidate transform, non-zero cost/carbon, dispatch, independent enumeration and frozen Pareto answer agree.");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("energy objective context gate failed: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn golden_nonzero_context_has_five_frontier_vectors_and_11_feasible_schedules() {
        let known: ExpectedContext = serde_json::from_str(include_str!(
            "../../fixtures/energy-objective-context-known-answer-v0.1.json"
        )).expect("golden objective context is valid JSON");
        assert_eq!(known.feasible_complete_schedules, 11);
        assert_eq!(known.pareto_frontier.len(), 5);
        assert_eq!(known.cost_basis.grid_cost_units_per_quantum, 5);
        assert_eq!(known.carbon_basis.grid_carbon_grams_per_quantum, 100);
    }

    #[test]
    fn golden_frontier_order_matches_dispatch_objective_order() {
        let known: ExpectedContext = serde_json::from_str(include_str!(
            "../../fixtures/energy-objective-context-known-answer-v0.1.json"
        )).expect("golden objective context is valid JSON");
        let mut sorted = known.pareto_frontier.clone();
        sorted.sort_by_key(|value| (
            value.essential_unserved_quanta,
            value.flexible_unserved_quanta,
            value.operating_cost_units,
            value.grid_carbon_grams,
            value.curtailed_pv_quanta,
            std::cmp::Reverse(value.terminal_soc_quanta)
        ));
        assert_eq!(sorted, known.pareto_frontier);
    }
}
