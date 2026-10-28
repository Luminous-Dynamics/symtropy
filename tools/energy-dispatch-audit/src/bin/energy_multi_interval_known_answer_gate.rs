//! Fail-closed gate for the actual Sol-Atlas dispatch CLI on the frozen
//! three-interval known-answer case.

use std::{
    cmp::Reverse,
    env,
    error::Error,
    fs,
    io,
};

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
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, format!("missing string field {key}"))
                .into()
        })
}

fn required_u64(value: &Value, key: &str) -> Result<u64, Box<dyn Error>> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, format!("missing unsigned field {key}"))
                .into()
        })
}

fn require(condition: bool, detail: &str) -> Result<(), Box<dyn Error>> {
    if condition {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::InvalidData, detail).into())
    }
}

fn objective_order(objective: &DispatchObjective) -> (u64, u64, u64, u64, u64, Reverse<u16>) {
    (
        objective.essential_unserved_quanta,
        objective.flexible_unserved_quanta,
        objective.operating_cost_units,
        objective.grid_carbon_grams,
        objective.curtailed_pv_quanta,
        Reverse(objective.terminal_soc_quanta),
    )
}

fn parse_frontier(values: &Value) -> Result<Vec<DispatchObjective>, Box<dyn Error>> {
    let items = values.as_array().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "frontier must be a JSON array")
    })?;
    let mut objectives = items
        .iter()
        .map(|item| serde_json::from_value::<DispatchObjective>(item.clone()))
        .collect::<Result<Vec<_>, _>>()?;
    objectives.sort_by_key(objective_order);
    Ok(objectives)
}

fn weakly_dominates(lhs: &DispatchObjective, rhs: &DispatchObjective) -> bool {
    lhs.essential_unserved_quanta <= rhs.essential_unserved_quanta
        && lhs.flexible_unserved_quanta <= rhs.flexible_unserved_quanta
        && lhs.operating_cost_units <= rhs.operating_cost_units
        && lhs.grid_carbon_grams <= rhs.grid_carbon_grams
        && lhs.curtailed_pv_quanta <= rhs.curtailed_pv_quanta
        && lhs.terminal_soc_quanta >= rhs.terminal_soc_quanta
}

fn derive_frontier_from_complete_schedule_objectives(
    values: &Value,
) -> Result<Vec<DispatchObjective>, Box<dyn Error>> {
    let mut distinct = parse_frontier(values)?;
    distinct.sort();
    distinct.dedup();
    let mut frontier = distinct
        .iter()
        .filter(|candidate| {
            !distinct.iter().any(|other| {
                other != *candidate && weakly_dominates(other, candidate)
            })
        })
        .cloned()
        .collect::<Vec<_>>();
    frontier.sort_by_key(objective_order);
    Ok(frontier)
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 5 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: energy_multi_interval_known_answer_gate <request.json> <dispatch-report.json> <independent-audit.json> <known-answer.json>",
        )
        .into());
    }

    let request_bytes = fs::read(&args[1])?;
    let report_bytes = fs::read(&args[2])?;
    let audit_bytes = fs::read(&args[3])?;
    let known_bytes = fs::read(&args[4])?;
    let request: Value = serde_json::from_slice(&request_bytes)?;
    let report: Value = serde_json::from_slice(&report_bytes)?;
    let audit: Value = serde_json::from_slice(&audit_bytes)?;
    let known: Value = serde_json::from_slice(&known_bytes)?;

    let request_sha256 = sha256_hex(&request_bytes);
    let report_sha256 = sha256_hex(&report_bytes);
    let case_id = required_str(&request, "case_id")?;

    require(
        required_str(&known, "schema_version")? == "energy-dispatch-known-answer-v0.1",
        "unsupported known-answer schema",
    )?;
    require(required_str(&known, "case_id")? == case_id, "frozen known-answer case ID mismatch")?;
    require(
        request["intervals"].as_array().is_some_and(|intervals| intervals.len() == 3),
        "request must contain exactly three intervals",
    )?;
    require(
        required_u64(&known, "feasible_complete_schedules")?
            == known["feasible_schedule_objectives"].as_array().map_or(0, |items| items.len() as u64),
        "known-answer objective multiset must include every feasible schedule",
    )?;

    require(
        report["receipt"]["input_sha256"].as_str() == Some(request_sha256.as_str())
            && required_u64(&report["receipt"], "input_bytes")? == request_bytes.len() as u64
            && required_str(&report["receipt"], "case_id")? == case_id,
        "Sol-Atlas receipt does not bind the exact request bytes and case ID",
    )?;
    let assessment = &report["assessment"];
    require(
        required_str(assessment, "assessment_schema_version")? == "energy-dispatch-report-v1"
            && required_str(assessment, "case_id")? == case_id
            && assessment["synthetic"].as_bool() == Some(true),
        "dispatch assessment identity/schema/synthetic boundary mismatch",
    )?;
    require(
        assessment["search_is_exact"].as_bool() == Some(true)
            && assessment["pareto_frontier_complete"].as_bool() == Some(true),
        "Sol-Atlas did not report an exact complete search",
    )?;
    require(
        required_u64(assessment, "feasible_complete_schedules")?
            == required_u64(&known, "feasible_complete_schedules")?,
        "actual Sol-Atlas feasible-complete-schedule count differs from the frozen known answer",
    )?;

    let alternatives = assessment["pareto_alternatives"].as_array().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "Sol-Atlas frontier alternatives missing")
    })?;
    require(
        required_u64(assessment, "pareto_frontier_count")? == alternatives.len() as u64,
        "Sol-Atlas frontier count does not match the returned alternatives",
    )?;
    for (index, alternative) in alternatives.iter().enumerate() {
        let schedule = alternative["schedule"].as_array().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "frontier schedule missing")
        })?;
        require(schedule.len() == 3, "each returned Pareto schedule must span all three intervals")?;
        for interval_index in 0..3 {
            require(
                schedule[interval_index]["interval_id"].as_str()
                    == request["intervals"][interval_index]["interval_id"].as_str(),
                &format!("Pareto alternative {index} does not preserve interval order/identity"),
            )?;
        }
    }

    let expected_frontier = parse_frontier(&known["pareto_frontier"])?;
    let schedule_objectives = known["feasible_schedule_objectives"].as_array().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "frozen feasible-schedule objective multiset missing")
    })?;
    require(
        schedule_objectives.len() as u64
            == required_u64(&known, "feasible_complete_schedules")?,
        "frozen schedule objective multiset must contain every feasible complete schedule",
    )?;
    let mut distinct_schedule_objectives = parse_frontier(&known["feasible_schedule_objectives"])?;
    distinct_schedule_objectives.sort();
    distinct_schedule_objectives.dedup();
    require(
        distinct_schedule_objectives.len() as u64
            == required_u64(&known, "distinct_objective_count")?,
        "frozen distinct objective count does not match its schedule objective multiset",
    )?;
    let derived_golden_frontier =
        derive_frontier_from_complete_schedule_objectives(&known["feasible_schedule_objectives"])?;
    require(
        derived_golden_frontier == expected_frontier,
        "frozen Pareto frontier is inconsistent with the complete feasible-schedule objective multiset",
    )?;
    let solver_frontier = parse_frontier(
        &Value::Array(
            alternatives
                .iter()
                .map(|alternative| alternative["objective"].clone())
                .collect(),
        ),
    )?;
    let audit_frontier = parse_frontier(&audit["reference_frontier_objectives"])?;
    require(
        solver_frontier == expected_frontier,
        "actual Sol-Atlas objective frontier differs from the frozen three-interval known answer",
    )?;
    require(
        audit_frontier == expected_frontier,
        "independent reference frontier differs from the frozen three-interval known answer",
    )?;
    require(
        required_str(&audit, "status")? == "pass"
            && required_str(&audit, "reference_frontier_status")? == "pass"
            && required_str(&audit, "objective_claim_status")? == "verified_for_bounded_model_case"
            && required_u64(&audit, "reference_feasible_complete_schedules")?
                == required_u64(&known, "feasible_complete_schedules")?,
        "independent auditor did not independently qualify the dispatch report and frontier",
    )?;
    require(
        required_str(&audit, "request_sha256")? == request_sha256
            && required_str(&audit, "run_report_sha256")? == report_sha256,
        "independent audit report is not bound to these exact request/report bytes",
    )?;
    let checks = audit["checks"].as_array().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "independent audit check list missing")
    })?;
    require(
        checks.iter().all(|item| item["status"].as_str() == Some("pass")),
        "independent audit has a non-passing or unknown check",
    )?;

    println!(
        "PASS: actual Sol-Atlas multi-interval dispatch scenario {} matches {} schedules and {} frozen Pareto vectors",
        case_id,
        required_u64(&known, "feasible_complete_schedules")?,
        expected_frontier.len()
    );
    println!("request_sha256={request_sha256}");
    println!("dispatch_report_sha256={report_sha256}");
    println!("independent_audit_sha256={}", sha256_hex(&audit_bytes));
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("energy scenario known-answer gate failed: {error}");
        std::process::exit(2);
    }
}
