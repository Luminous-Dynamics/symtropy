use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::process::ExitCode;

#[derive(Clone, Copy, Debug)]
struct Provider {
    id: &'static str,
    capacity: u64,
    power_domain: &'static str,
}

#[derive(Clone, Copy, Debug)]
struct Architecture {
    id: &'static str,
    cost_scu: u64,
    providers: &'static [Provider],
    required_faults: &'static [&'static str],
    claim_ceiling: &'static str,
}

#[derive(Clone, Copy, Debug)]
struct FaultCase {
    id: &'static str,
    failed_component: Option<&'static str>,
}

const DEMAND: u64 = 60;
const SHARED_PROVIDERS: [Provider; 2] = [
    Provider { id: "relay-a", capacity: 70, power_domain: "power/shared-bus" },
    Provider { id: "relay-b", capacity: 70, power_domain: "power/shared-bus" },
];
const DIVERSE_PROVIDERS: [Provider; 2] = [
    Provider { id: "relay-a", capacity: 70, power_domain: "power/bus-a" },
    Provider { id: "relay-b", capacity: 70, power_domain: "power/bus-b" },
];
const SHARED_REQUIRED: [&str; 3] = [
    "provider/relay-a",
    "provider/relay-b",
    "power/shared-bus",
];
const DIVERSE_REQUIRED: [&str; 4] = [
    "provider/relay-a",
    "provider/relay-b",
    "power/bus-a",
    "power/bus-b",
];
const ARCHITECTURES: [Architecture; 2] = [
    Architecture {
        id: "shared_power_controller",
        cost_scu: 65,
        providers: &SHARED_PROVIDERS,
        required_faults: &SHARED_REQUIRED,
        claim_ceiling: "Synthetic topology contract only; no empirical failure probability or independence claim.",
    },
    Architecture {
        id: "separate_power_controllers",
        cost_scu: 85,
        providers: &DIVERSE_PROVIDERS,
        required_faults: &DIVERSE_REQUIRED,
        claim_ceiling: "Synthetic topology contract only; separate IDs do not establish statistically independent failures.",
    },
];
const FAULTS: [FaultCase; 5] = [
    FaultCase { id: "provider/relay-a", failed_component: Some("provider/relay-a") },
    FaultCase { id: "provider/relay-b", failed_component: Some("provider/relay-b") },
    FaultCase { id: "power/shared-bus", failed_component: Some("power/shared-bus") },
    FaultCase { id: "power/bus-a", failed_component: Some("power/bus-a") },
    FaultCase { id: "power/bus-b", failed_component: Some("power/bus-b") },
];

fn capacity_after_fault(architecture: &Architecture, failed: &BTreeSet<&str>) -> u64 {
    architecture.providers.iter()
        .filter(|provider| {
            !failed.contains(format!("provider/{}", provider.id).as_str())
                && !failed.contains(provider.power_domain)
        })
        .map(|provider| provider.capacity)
        .sum()
}

fn fault_is_applicable(architecture: &Architecture, fault_id: &str) -> bool {
    if let Some(provider_id) = fault_id.strip_prefix("provider/") {
        return architecture.providers.iter().any(|p| p.id == provider_id);
    }
    architecture.providers.iter().any(|p| p.power_domain == fault_id)
}

fn run_case(architecture: &Architecture, fault_id: &str) -> Result<Value, String> {
    if !fault_is_applicable(architecture, fault_id) {
        return Ok(json!({
            "fault_id": fault_id,
            "applicable": false,
            "status": "not_applicable",
            "remaining_capacity_units": null,
            "demand_met": null,
            "reason_code": "FAULT_DOMAIN_NOT_PRESENT_IN_ARCHITECTURE"
        }));
    }
    let failed = [fault_id].into_iter().collect::<BTreeSet<_>>();
    let remaining = capacity_after_fault(architecture, &failed);
    Ok(json!({
        "fault_id": fault_id,
        "applicable": true,
        "status": if remaining >= DEMAND { "pass" } else { "fail" },
        "remaining_capacity_units": remaining,
        "demand_units": DEMAND,
        "unmet_demand_units": DEMAND.saturating_sub(remaining),
        "demand_met": remaining >= DEMAND,
        "reason_code": if remaining >= DEMAND { "SERVICE_FLOOR_MET" } else { "COMMON_CAUSE_OR_CAPACITY_FAILURE" }
    }))
}

fn evaluate(architecture: &Architecture) -> Result<Value, String> {
    let evaluations = FAULTS.iter()
        .map(|fault| run_case(architecture, fault.id))
        .collect::<Result<Vec<_>, _>>()?;
    let mut applicable = 0_u64;
    let mut passed = 0_u64;
    for result in &evaluations {
        if result["applicable"].as_bool() == Some(true) {
            applicable += 1;
            if result["demand_met"].as_bool() == Some(true) {
                passed += 1;
            }
        }
    }
    let mut required_results = Vec::new();
    for fault_id in architecture.required_faults {
        let result = run_case(architecture, fault_id)?;
        if result["applicable"] != true {
            return Err(format!("{}: required fault {} is not present in the architecture", architecture.id, fault_id));
        }
        required_results.push(result);
    }
    let mandatory_fault_set_pass = required_results.iter().all(|r| r["demand_met"] == true);
    let provider_faults_pass = ["provider/relay-a", "provider/relay-b"].iter()
        .all(|id| run_case(architecture, id).is_ok_and(|r| r["demand_met"] == true));
    Ok(json!({
        "architecture_id": architecture.id,
        "cost_scu": architecture.cost_scu,
        "demand_capacity_units": DEMAND,
        "nominal_capacity_units": architecture.providers.iter().map(|p| p.capacity).sum::<u64>(),
        "n_minus_one_provider_coverage": provider_faults_pass,
        "applicable_fault_domains_passed": passed,
        "applicable_fault_domains_tested": applicable,
        "fault_domain_coverage_ppm": if applicable == 0 { 0 } else { passed * 1_000_000 / applicable },
        "mandatory_fault_set_pass": mandatory_fault_set_pass,
        "claim_ceiling": architecture.claim_ceiling,
        "fault_results": evaluations
    }))
}

fn is_commit_sha(value: &str) -> bool {
    value.len() == 40
        && value.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn execute(source_commit: &str) -> Result<Value, String> {
    if !is_commit_sha(source_commit) {
        return Err("source commit must be a 40-character lowercase Git SHA-1".to_owned());
    }
    let shared = evaluate(&ARCHITECTURES[0])?;
    let diverse = evaluate(&ARCHITECTURES[1])?;
    let shared_common_cause = run_case(&ARCHITECTURES[0], "power/shared-bus")?;
    let diverse_bus_a = run_case(&ARCHITECTURES[1], "power/bus-a")?;
    let diverse_bus_b = run_case(&ARCHITECTURES[1], "power/bus-b")?;

    if shared["n_minus_one_provider_coverage"] != true {
        return Err("shared architecture must pass both single-provider failures in this fixture".to_owned());
    }
    if shared_common_cause["demand_met"] != false {
        return Err("shared-controller negative control unexpectedly survived common-cause outage".to_owned());
    }
    if diverse["mandatory_fault_set_pass"] != true
        || diverse_bus_a["demand_met"] != true
        || diverse_bus_b["demand_met"] != true
    {
        return Err("separate-controller architecture failed a required individual fault".to_owned());
    }
    if shared["cost_scu"].as_u64().unwrap_or(u64::MAX)
        >= diverse["cost_scu"].as_u64().unwrap_or(0)
    {
        return Err("fixture must retain a cost premium for the diverse architecture".to_owned());
    }

    let source_digest = format!("{:x}", Sha256::digest(include_bytes!("common_cause_reference.rs")));
    Ok(json!({
        "receipt_schema_version": "0.1.0",
        "reference_version": "common-cause-resilience-v0.1",
        "source_commit": source_commit,
        "reference_source_sha256": source_digest,
        "result": "pass",
        "evidence_class": "synthetic_deterministic_fault_set",
        "claim_ceiling": "Tests named component-dependency fault cases only. No calibrated failure rates, probabilistic availability, flight qualification, or proof of statistical independence.",
        "service_demand_units": DEMAND,
        "architectures": [shared, diverse],
        "summary": {
            "shared_architecture_passes_single_provider_outage": true,
            "shared_architecture_fails_shared_power_bus_outage": true,
            "separate_architecture_passes_each_required_single_component_fault": true,
            "diverse_cost_premium_scu": ARCHITECTURES[1].cost_scu - ARCHITECTURES[0].cost_scu,
            "required_gate_rule": "Reject any architecture that fails a mandatory fault-domain case before comparing lifecycle cost."
        }
    }))
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 || args[1] != "--source-commit" {
        return Err("usage: cargo run --example common_cause_reference -- --source-commit <40-char-lowercase-git-sha>".to_owned());
    }
    println!("{}", serde_json::to_string_pretty(&execute(&args[2])?).map_err(|e| e.to_string())?);
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("common_cause_reference: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_providers_do_not_hide_shared_power_controller_failure() {
        let report = evaluate(&ARCHITECTURES[0]).expect("valid shared fixture");
        assert_eq!(report["n_minus_one_provider_coverage"], true);
        assert_eq!(report["mandatory_fault_set_pass"], false);
        let shared_power = run_case(&ARCHITECTURES[0], "power/shared-bus").unwrap();
        assert_eq!(shared_power["remaining_capacity_units"], 0);
        assert_eq!(shared_power["demand_met"], false);
    }

    #[test]
    fn separated_power_domains_survive_each_declared_single_component_fault() {
        let report = evaluate(&ARCHITECTURES[1]).expect("valid diverse fixture");
        assert_eq!(report["n_minus_one_provider_coverage"], true);
        assert_eq!(report["mandatory_fault_set_pass"], true);
        assert_eq!(report["fault_domain_coverage_ppm"], 1_000_000);
    }

    #[test]
    fn not_applicable_fault_domain_is_never_reported_as_a_pass() {
        let result = run_case(&ARCHITECTURES[1], "power/shared-bus").unwrap();
        assert_eq!(result["applicable"], false);
        assert_eq!(result["status"], "not_applicable");
        assert_eq!(result["demand_met"], Value::Null);
    }

    #[test]
    fn cost_is_compared_only_after_mandatory_fault_gates() {
        let shared = evaluate(&ARCHITECTURES[0]).unwrap();
        let diverse = evaluate(&ARCHITECTURES[1]).unwrap();
        assert!(ARCHITECTURES[0].cost_scu < ARCHITECTURES[1].cost_scu);
        assert_eq!(shared["mandatory_fault_set_pass"], false);
        assert_eq!(diverse["mandatory_fault_set_pass"], true);
    }

    #[test]
    fn rejects_invalid_source_commit_before_receipt() {
        assert!(execute("not-a-commit-sha").is_err());
    }
}
