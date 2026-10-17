use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::process::ExitCode;

const HORIZON_MONTHS: u8 = 12;
const SERVICE_AVAILABILITY_FLOOR_BPS: u64 = 7_500;

const FAILURE_MONTHS: [u8; 3] = [2, 6, 10];
const NOMINAL_ARRIVALS: [u8; 3] = [4, 8, 11];
const DELAYED_ARRIVALS: [u8; 2] = [7, 11];
const NO_ARRIVALS: [u8; 0] = [];

#[derive(Clone, Copy, Debug)]
struct Scenario {
    scenario_id: &'static str,
    architecture_id: &'static str,
    lifecycle_cost_scu: u64,
    initial_spares: u64,
    delivery_months: &'static [u8],
    failure_months: &'static [u8],
    horizon_months: u8,
    claimed_local_repair: bool,
    local_repair_evidence_verified: bool,
    repair_delay_months: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Simulation {
    service_months: u8,
    downtime_months: u8,
    max_consecutive_downtime_months: u8,
    triggered_failures: u8,
    suppressed_failure_events: u8,
    local_repair_events_credited: u8,
    externally_sourced_spares_consumed: u64,
    delivery_units_arrived: u64,
    remaining_spares: u64,
    availability_basis_points: u64,
    availability_gate_pass: bool,
}

const SCENARIOS: [Scenario; 5] = [
    Scenario {
        scenario_id: "import_spares_nominal",
        architecture_id: "import_spares",
        lifecycle_cost_scu: 100,
        initial_spares: 1,
        delivery_months: &NOMINAL_ARRIVALS,
        failure_months: &FAILURE_MONTHS,
        horizon_months: HORIZON_MONTHS,
        claimed_local_repair: false,
        local_repair_evidence_verified: false,
        repair_delay_months: 1,
    },
    Scenario {
        scenario_id: "import_spares_resupply_delayed",
        architecture_id: "import_spares",
        lifecycle_cost_scu: 100,
        initial_spares: 1,
        delivery_months: &DELAYED_ARRIVALS,
        failure_months: &FAILURE_MONTHS,
        horizon_months: HORIZON_MONTHS,
        claimed_local_repair: false,
        local_repair_evidence_verified: false,
        repair_delay_months: 1,
    },
    Scenario {
        scenario_id: "verified_local_repair_nominal",
        architecture_id: "verified_local_repair",
        lifecycle_cost_scu: 125,
        initial_spares: 0,
        delivery_months: &NO_ARRIVALS,
        failure_months: &FAILURE_MONTHS,
        horizon_months: HORIZON_MONTHS,
        claimed_local_repair: true,
        local_repair_evidence_verified: true,
        repair_delay_months: 1,
    },
    Scenario {
        scenario_id: "verified_local_repair_resupply_shock",
        architecture_id: "verified_local_repair",
        lifecycle_cost_scu: 125,
        initial_spares: 0,
        delivery_months: &NO_ARRIVALS,
        failure_months: &FAILURE_MONTHS,
        horizon_months: HORIZON_MONTHS,
        claimed_local_repair: true,
        local_repair_evidence_verified: true,
        repair_delay_months: 1,
    },
    Scenario {
        scenario_id: "unverified_repair_negative_control",
        architecture_id: "unverified_local_repair",
        lifecycle_cost_scu: 90,
        initial_spares: 0,
        delivery_months: &NO_ARRIVALS,
        failure_months: &FAILURE_MONTHS,
        horizon_months: HORIZON_MONTHS,
        claimed_local_repair: true,
        local_repair_evidence_verified: false,
        repair_delay_months: 1,
    },
];

fn validate_scenario(scenario: &Scenario) -> Result<(), String> {
    if scenario.scenario_id.is_empty()
        || scenario.architecture_id.is_empty()
        || scenario.lifecycle_cost_scu == 0
        || scenario.horizon_months == 0
    {
        return Err("scenario identity, positive lifecycle cost, and positive horizon are required".to_owned());
    }
    if scenario.claimed_local_repair && scenario.repair_delay_months == 0 {
        return Err(format!("{}: local repair delay must be at least one month", scenario.scenario_id));
    }
    let mut months = BTreeSet::new();
    for month in scenario.delivery_months {
        if *month == 0 || *month > scenario.horizon_months || !months.insert(*month) {
            return Err(format!("{}: delivery months must be unique and inside the horizon", scenario.scenario_id));
        }
    }
    let mut failures = BTreeSet::new();
    for month in scenario.failure_months {
        if *month == 0 || *month > scenario.horizon_months || !failures.insert(*month) {
            return Err(format!("{}: failure months must be unique and inside the horizon", scenario.scenario_id));
        }
    }
    Ok(())
}

fn simulate(scenario: &Scenario) -> Result<Simulation, String> {
    validate_scenario(scenario)?;
    let repair_is_usable = scenario.claimed_local_repair && scenario.local_repair_evidence_verified;
    let mut service_up = true;
    let mut waiting_for_spare = false;
    let mut recovery_month: Option<u8> = None;
    let mut spare_stock = scenario.initial_spares;
    let mut delivery_units_arrived = 0_u64;
    let mut externally_sourced_spares_consumed = 0_u64;
    let mut local_repair_events_credited = 0_u8;
    let mut triggered_failures = 0_u8;
    let mut suppressed_failure_events = 0_u8;
    let mut service_months = 0_u8;
    let mut downtime_months = 0_u8;
    let mut current_downtime_streak = 0_u8;
    let mut max_consecutive_downtime_months = 0_u8;

    for month in 1..=scenario.horizon_months {
        if scenario.delivery_months.contains(&month) {
            spare_stock = spare_stock.checked_add(1)
                .ok_or_else(|| format!("{}: spare stock overflow", scenario.scenario_id))?;
            delivery_units_arrived = delivery_units_arrived.checked_add(1)
                .ok_or_else(|| format!("{}: delivery count overflow", scenario.scenario_id))?;
        }

        if recovery_month.is_some_and(|due| due <= month) {
            service_up = true;
            recovery_month = None;
        }

        if !service_up && waiting_for_spare && spare_stock > 0 {
            spare_stock -= 1;
            externally_sourced_spares_consumed += 1;
            waiting_for_spare = false;
            recovery_month = Some(month.saturating_add(1));
        }

        if scenario.failure_months.contains(&month) {
            if service_up {
                triggered_failures += 1;
                service_up = false;
                if repair_is_usable {
                    local_repair_events_credited += 1;
                    recovery_month = Some(month.saturating_add(scenario.repair_delay_months));
                } else if spare_stock > 0 {
                    spare_stock -= 1;
                    externally_sourced_spares_consumed += 1;
                    recovery_month = Some(month.saturating_add(1));
                } else {
                    waiting_for_spare = true;
                }
            } else {
                suppressed_failure_events += 1;
            }
        }

        if service_up {
            service_months += 1;
            current_downtime_streak = 0;
        } else {
            downtime_months += 1;
            current_downtime_streak += 1;
            max_consecutive_downtime_months =
                max_consecutive_downtime_months.max(current_downtime_streak);
        }
    }

    let availability_basis_points =
        u64::from(service_months) * 10_000 / u64::from(scenario.horizon_months);
    Ok(Simulation {
        service_months,
        downtime_months,
        max_consecutive_downtime_months,
        triggered_failures,
        suppressed_failure_events,
        local_repair_events_credited,
        externally_sourced_spares_consumed,
        delivery_units_arrived,
        remaining_spares: spare_stock,
        availability_basis_points,
        availability_gate_pass: availability_basis_points >= SERVICE_AVAILABILITY_FLOOR_BPS,
    })
}

fn to_json(scenario: &Scenario, result: &Simulation) -> Value {
    json!({
        "scenario_id": scenario.scenario_id,
        "architecture_id": scenario.architecture_id,
        "lifecycle_cost_scu": scenario.lifecycle_cost_scu,
        "horizon_months": scenario.horizon_months,
        "failure_months": scenario.failure_months,
        "delivery_months": scenario.delivery_months,
        "initial_spares": scenario.initial_spares,
        "claimed_local_repair": scenario.claimed_local_repair,
        "local_repair_evidence_verified": scenario.local_repair_evidence_verified,
        "credited_local_repair": scenario.claimed_local_repair && scenario.local_repair_evidence_verified,
        "service_months": result.service_months,
        "downtime_months": result.downtime_months,
        "max_consecutive_downtime_months": result.max_consecutive_downtime_months,
        "triggered_failures": result.triggered_failures,
        "suppressed_failure_events_during_outage": result.suppressed_failure_events,
        "local_repair_events_credited": result.local_repair_events_credited,
        "externally_sourced_spares_consumed": result.externally_sourced_spares_consumed,
        "delivery_units_arrived": result.delivery_units_arrived,
        "remaining_spares": result.remaining_spares,
        "availability_basis_points": result.availability_basis_points,
        "availability_floor_basis_points": SERVICE_AVAILABILITY_FLOOR_BPS,
        "availability_gate_pass": result.availability_gate_pass
    })
}

fn execute(source_commit: &str) -> Result<Value, String> {
    if source_commit.len() != 40
        || !source_commit.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err("source commit must be a 40-character lowercase Git SHA-1".to_owned());
    }

    let mut scenario_ids = BTreeSet::new();
    let mut results: Vec<(Scenario, Simulation)> = Vec::new();
    for scenario in SCENARIOS {
        if !scenario_ids.insert(scenario.scenario_id) {
            return Err(format!("duplicate scenario id {}", scenario.scenario_id));
        }
        results.push((scenario, simulate(&scenario)?));
    }

    let lookup = |scenario_id: &str| -> Result<&Simulation, String> {
        results.iter().find(|(scenario, _)| scenario.scenario_id == scenario_id)
            .map(|(_, result)| result)
            .ok_or_else(|| format!("missing oracle scenario {scenario_id}"))
    };
    let nominal_import = lookup("import_spares_nominal")?;
    let delayed_import = lookup("import_spares_resupply_delayed")?;
    let local_nominal = lookup("verified_local_repair_nominal")?;
    let local_shock = lookup("verified_local_repair_resupply_shock")?;
    let unverified_repair = lookup("unverified_repair_negative_control")?;

    if nominal_import.service_months != 9
        || nominal_import.externally_sourced_spares_consumed != 3
        || !nominal_import.availability_gate_pass
    {
        return Err("nominal import-spares oracle mismatch".to_owned());
    }
    if delayed_import.service_months != 7
        || delayed_import.max_consecutive_downtime_months != 2
        || delayed_import.availability_gate_pass
    {
        return Err("delayed-resupply oracle mismatch".to_owned());
    }
    if local_nominal.service_months != 9
        || local_nominal.local_repair_events_credited != 3
        || local_nominal.externally_sourced_spares_consumed != 0
        || !local_nominal.availability_gate_pass
        || local_shock.service_months != local_nominal.service_months
    {
        return Err("verified local-repair oracle mismatch".to_owned());
    }
    if unverified_repair.local_repair_events_credited != 0
        || unverified_repair.service_months != 1
        || unverified_repair.availability_gate_pass
    {
        return Err("unverified local-repair negative control incorrectly received repair credit".to_owned());
    }

    let mut architecture_status: BTreeMap<&str, (u64, bool, u64)> = BTreeMap::new();
    for (scenario, result) in &results {
        let entry = architecture_status.entry(scenario.architecture_id)
            .or_insert((scenario.lifecycle_cost_scu, true, 0));
        if entry.0 != scenario.lifecycle_cost_scu {
            return Err(format!("{}: architecture lifecycle cost differs across scenarios", scenario.architecture_id));
        }
        entry.1 &= result.availability_gate_pass;
        entry.2 += 1;
    }
    let selected_architecture = architecture_status.iter()
        .filter(|(_, (_, robust_pass, _))| *robust_pass)
        .min_by_key(|(_, (cost, _, _))| *cost)
        .map(|(architecture, _)| *architecture)
        .ok_or_else(|| "no architecture passed the service floor across its declared scenarios".to_owned())?;
    if selected_architecture != "verified_local_repair" {
        return Err("robust architecture-selection oracle mismatch".to_owned());
    }

    let source_digest =
        format!("{:x}", Sha256::digest(include_bytes!("temporal_resilience_reference.rs")));
    Ok(json!({
        "receipt_schema_version": "0.1.0",
        "reference_version": "temporal-service-resilience-v0.1",
        "source_commit": source_commit,
        "reference_source_sha256": source_digest,
        "result": "pass",
        "evidence_class": "synthetic_deterministic_timeline",
        "horizon_months": HORIZON_MONTHS,
        "service_availability_floor_basis_points": SERVICE_AVAILABILITY_FLOOR_BPS,
        "time_model": "Discrete monthly periods; arrivals occur at period start; failures occur within a period; local repair or spare replacement recovers service no earlier than the following month.",
        "decision_rule": "An architecture is robust-feasible only when it meets the availability floor in every scenario declared for that architecture; lifecycle cost ranks only robust-feasible architectures.",
        "selected_robust_feasible_architecture": selected_architecture,
        "architectures": architecture_status.iter().map(|(id, (cost, pass, count))| json!({
            "architecture_id": id,
            "lifecycle_cost_scu": cost,
            "all_declared_scenarios_pass_service_floor": pass,
            "scenario_count": count
        })).collect::<Vec<_>>(),
        "scenarios": results.iter().map(|(scenario, result)| to_json(scenario, result)).collect::<Vec<_>>(),
        "claim_ceiling": "Synthetic timeline contract only; no empirical lunar failure rates, actual repair durations, inventory optimization, mission reliability or flight qualification."
    }))
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 || args[1] != "--source-commit" {
        return Err("usage: cargo run --example temporal_resilience_reference -- --source-commit <40-char-lowercase-git-sha>".to_owned());
    }
    println!("{}", serde_json::to_string_pretty(&execute(&args[2])?).map_err(|e| e.to_string())?);
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("temporal_resilience_reference: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nominal_import_spares_meet_floor_but_consume_three_spares() {
        let result = simulate(&SCENARIOS[0]).unwrap();
        assert_eq!(result.service_months, 9);
        assert_eq!(result.availability_basis_points, 7_500);
        assert_eq!(result.externally_sourced_spares_consumed, 3);
        assert!(result.availability_gate_pass);
    }

    #[test]
    fn delayed_resupply_increases_downtime_and_fails_floor() {
        let nominal = simulate(&SCENARIOS[0]).unwrap();
        let delayed = simulate(&SCENARIOS[1]).unwrap();
        assert_eq!(delayed.service_months, 7);
        assert_eq!(delayed.max_consecutive_downtime_months, 2);
        assert!(delayed.downtime_months > nominal.downtime_months);
        assert!(!delayed.availability_gate_pass);
    }

    #[test]
    fn verified_local_repair_maintains_service_without_imported_spares() {
        let nominal = simulate(&SCENARIOS[2]).unwrap();
        let shock = simulate(&SCENARIOS[3]).unwrap();
        assert_eq!(nominal.service_months, 9);
        assert_eq!(nominal.local_repair_events_credited, 3);
        assert_eq!(nominal.externally_sourced_spares_consumed, 0);
        assert_eq!(nominal.service_months, shock.service_months);
        assert!(nominal.availability_gate_pass);
        assert!(shock.availability_gate_pass);
    }

    #[test]
    fn unverified_repair_claim_is_not_credited() {
        let result = simulate(&SCENARIOS[4]).unwrap();
        assert_eq!(result.local_repair_events_credited, 0);
        assert_eq!(result.service_months, 1);
        assert!(!result.availability_gate_pass);
    }

    #[test]
    fn selection_uses_stress_case_feasibility_before_lifecycle_cost() {
        let report = execute(&"a".repeat(40)).unwrap();
        assert_eq!(report["selected_robust_feasible_architecture"], "verified_local_repair");
        let architectures = report["architectures"].as_array().unwrap();
        assert!(architectures.iter().any(|a| a["architecture_id"] == "import_spares" && a["all_declared_scenarios_pass_service_floor"] == false));
        assert!(architectures.iter().any(|a| a["architecture_id"] == "verified_local_repair" && a["all_declared_scenarios_pass_service_floor"] == true));
    }

    #[test]
    fn failure_or_delivery_outside_horizon_fails_validation() {
        let invalid = Scenario {
            scenario_id: "invalid",
            architecture_id: "invalid",
            lifecycle_cost_scu: 10,
            initial_spares: 0,
            delivery_months: &[13],
            failure_months: &[],
            horizon_months: HORIZON_MONTHS,
            claimed_local_repair: false,
            local_repair_evidence_verified: false,
            repair_delay_months: 1,
        };
        assert!(simulate(&invalid).is_err());
    }

    #[test]
    fn duplicate_event_months_fail_validation() {
        let invalid = Scenario {
            scenario_id: "duplicate",
            architecture_id: "duplicate",
            lifecycle_cost_scu: 10,
            initial_spares: 0,
            delivery_months: &[3, 3],
            failure_months: &[],
            horizon_months: HORIZON_MONTHS,
            claimed_local_repair: false,
            local_repair_evidence_verified: false,
            repair_delay_months: 1,
        };
        assert!(simulate(&invalid).is_err());
    }

    #[test]
    fn invalid_source_commit_is_rejected() {
        assert!(execute("not-a-commit").is_err());
    }

    #[test]
    fn receipt_binds_source_and_all_synthetic_scenarios() {
        let report = execute(&"a".repeat(40)).unwrap();
        assert_eq!(report["result"], "pass");
        assert_eq!(report["scenarios"].as_array().unwrap().len(), 5);
        assert_eq!(report["reference_source_sha256"].as_str().unwrap().len(), 64);
        assert_eq!(report["horizon_months"], 12);
    }
}
