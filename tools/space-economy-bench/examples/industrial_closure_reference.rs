use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::process::ExitCode;

#[derive(Clone, Copy, Debug)]
struct Capability {
    id: &'static str,
    reported_operational_now: bool,
    evidence_verified: bool,
    locally_operated: bool,
    recurring_import_dependency: bool,
    dependencies: &'static [&'static str],
}

#[derive(Clone, Copy, Debug)]
struct Portfolio {
    id: &'static str,
    claimed_local_content_percent: u64,
    lifecycle_cost_scu: u64,
    target_capability_id: &'static str,
    capabilities: &'static [Capability],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Blocker {
    capability_id: String,
    reason_code: String,
}

#[derive(Clone, Debug)]
struct Analysis {
    locally_closed: bool,
    blockers: Vec<Blocker>,
}

const POWER_DEPS: [&str; 0] = [];
const EXCAVATION_DEPS: [&str; 1] = ["power_generation"];
const BENEFICIATION_DEPS: [&str; 2] = ["excavation", "power_generation"];
const OXYGEN_DEPS: [&str; 2] = ["beneficiation", "power_generation"];
const METAL_DEPS: [&str; 2] = ["beneficiation", "oxygen_extraction"];
const SPARE_FAB_DEPS: [&str; 2] = ["metal_refining", "power_generation"];
const REPAIR_DEPS: [&str; 1] = ["spare_fabrication"];
const SENSOR_LOCAL_DEPS: [&str; 2] = ["spare_fabrication", "power_generation"];
const MINE_DEPS: [&str; 3] = ["excavation", "repair_service", "sensor_module"];

const HIGH_LOCAL_CONTENT_CAPABILITIES: [Capability; 9] = [
    Capability { id: "power_generation", reported_operational_now: true, evidence_verified: true, locally_operated: true, recurring_import_dependency: false, dependencies: &POWER_DEPS },
    Capability { id: "excavation", reported_operational_now: true, evidence_verified: true, locally_operated: true, recurring_import_dependency: false, dependencies: &EXCAVATION_DEPS },
    Capability { id: "beneficiation", reported_operational_now: true, evidence_verified: true, locally_operated: true, recurring_import_dependency: false, dependencies: &BENEFICIATION_DEPS },
    Capability { id: "oxygen_extraction", reported_operational_now: true, evidence_verified: true, locally_operated: true, recurring_import_dependency: false, dependencies: &OXYGEN_DEPS },
    Capability { id: "metal_refining", reported_operational_now: true, evidence_verified: false, locally_operated: true, recurring_import_dependency: false, dependencies: &METAL_DEPS },
    Capability { id: "spare_fabrication", reported_operational_now: true, evidence_verified: true, locally_operated: true, recurring_import_dependency: false, dependencies: &SPARE_FAB_DEPS },
    Capability { id: "repair_service", reported_operational_now: true, evidence_verified: true, locally_operated: true, recurring_import_dependency: false, dependencies: &REPAIR_DEPS },
    Capability { id: "sensor_module", reported_operational_now: true, evidence_verified: true, locally_operated: false, recurring_import_dependency: true, dependencies: &POWER_DEPS },
    Capability { id: "autonomous_mine_service", reported_operational_now: true, evidence_verified: true, locally_operated: true, recurring_import_dependency: false, dependencies: &MINE_DEPS },
];

const CLOSURE_FIRST_CAPABILITIES: [Capability; 9] = [
    Capability { id: "power_generation", reported_operational_now: true, evidence_verified: true, locally_operated: true, recurring_import_dependency: false, dependencies: &POWER_DEPS },
    Capability { id: "excavation", reported_operational_now: true, evidence_verified: true, locally_operated: true, recurring_import_dependency: false, dependencies: &EXCAVATION_DEPS },
    Capability { id: "beneficiation", reported_operational_now: true, evidence_verified: true, locally_operated: true, recurring_import_dependency: false, dependencies: &BENEFICIATION_DEPS },
    Capability { id: "oxygen_extraction", reported_operational_now: true, evidence_verified: true, locally_operated: true, recurring_import_dependency: false, dependencies: &OXYGEN_DEPS },
    Capability { id: "metal_refining", reported_operational_now: true, evidence_verified: true, locally_operated: true, recurring_import_dependency: false, dependencies: &METAL_DEPS },
    Capability { id: "spare_fabrication", reported_operational_now: true, evidence_verified: true, locally_operated: true, recurring_import_dependency: false, dependencies: &SPARE_FAB_DEPS },
    Capability { id: "repair_service", reported_operational_now: true, evidence_verified: true, locally_operated: true, recurring_import_dependency: false, dependencies: &REPAIR_DEPS },
    Capability { id: "sensor_module", reported_operational_now: true, evidence_verified: true, locally_operated: true, recurring_import_dependency: false, dependencies: &SENSOR_LOCAL_DEPS },
    Capability { id: "autonomous_mine_service", reported_operational_now: true, evidence_verified: true, locally_operated: true, recurring_import_dependency: false, dependencies: &MINE_DEPS },
];

const PORTFOLIOS: [Portfolio; 2] = [
    Portfolio {
        id: "high_local_content",
        claimed_local_content_percent: 92,
        lifecycle_cost_scu: 95,
        target_capability_id: "autonomous_mine_service",
        capabilities: &HIGH_LOCAL_CONTENT_CAPABILITIES,
    },
    Portfolio {
        id: "closure_first",
        claimed_local_content_percent: 78,
        lifecycle_cost_scu: 120,
        target_capability_id: "autonomous_mine_service",
        capabilities: &CLOSURE_FIRST_CAPABILITIES,
    },
];

fn validate_portfolio(portfolio: &Portfolio) -> Result<(), String> {
    if portfolio.id.is_empty() || portfolio.lifecycle_cost_scu == 0 {
        return Err("portfolio id and lifecycle cost must be non-empty/positive".to_owned());
    }
    if portfolio.claimed_local_content_percent > 100 {
        return Err(format!("{}: local-content percentage exceeds 100", portfolio.id));
    }
    let mut ids = BTreeSet::new();
    for capability in portfolio.capabilities {
        if capability.id.is_empty() || !ids.insert(capability.id) {
            return Err(format!("{}: empty or duplicate capability id", portfolio.id));
        }
    }
    if !ids.contains(portfolio.target_capability_id) {
        return Err(format!("{}: target capability is missing", portfolio.id));
    }
    Ok(())
}

fn analyze(
    id: &str,
    nodes: &BTreeMap<&'static str, &Capability>,
    visiting: &mut BTreeSet<String>,
    memo: &mut BTreeMap<String, Analysis>,
) -> Analysis {
    if let Some(cached) = memo.get(id) {
        return cached.clone();
    }
    if !nodes.contains_key(id) {
        return Analysis {
            locally_closed: false,
            blockers: vec![Blocker {
                capability_id: id.to_owned(),
                reason_code: "MISSING_CAPABILITY".to_owned(),
            }],
        };
    }
    if visiting.contains(id) {
        return Analysis {
            locally_closed: false,
            blockers: vec![Blocker {
                capability_id: id.to_owned(),
                reason_code: "DEPENDENCY_CYCLE".to_owned(),
            }],
        };
    }

    let capability = nodes[id];
    visiting.insert(id.to_owned());
    let mut blockers = Vec::new();

    if !capability.reported_operational_now {
        blockers.push(Blocker { capability_id: id.to_owned(), reason_code: "NOT_OPERATIONAL".to_owned() });
    }
    if !capability.evidence_verified {
        blockers.push(Blocker { capability_id: id.to_owned(), reason_code: "EVIDENCE_UNVERIFIED".to_owned() });
    }
    if !capability.locally_operated {
        blockers.push(Blocker { capability_id: id.to_owned(), reason_code: "NONLOCAL_OPERATION".to_owned() });
    }
    if capability.recurring_import_dependency {
        blockers.push(Blocker { capability_id: id.to_owned(), reason_code: "RECURRING_IMPORT_DEPENDENCY".to_owned() });
    }

    for dependency_id in capability.dependencies {
        let dependency = analyze(dependency_id, nodes, visiting, memo);
        blockers.extend(dependency.blockers);
    }

    visiting.remove(id);
    blockers.sort();
    blockers.dedup();
    let result = Analysis {
        locally_closed: blockers.is_empty(),
        blockers,
    };
    memo.insert(id.to_owned(), result.clone());
    result
}

fn analyze_portfolio(portfolio: &Portfolio) -> Result<Value, String> {
    validate_portfolio(portfolio)?;
    let nodes: BTreeMap<&'static str, &Capability> = portfolio.capabilities.iter()
        .map(|capability| (capability.id, capability))
        .collect();
    let target = nodes[portfolio.target_capability_id];
    let analysis = analyze(
        portfolio.target_capability_id,
        &nodes,
        &mut BTreeSet::new(),
        &mut BTreeMap::new(),
    );

    Ok(json!({
        "portfolio_id": portfolio.id,
        "claimed_local_content_percent": portfolio.claimed_local_content_percent,
        "lifecycle_cost_scu": portfolio.lifecycle_cost_scu,
        "target_capability_id": portfolio.target_capability_id,
        "capability_count": portfolio.capabilities.len(),
        "target_state": {
            "reported_operational_now": target.reported_operational_now,
            "operational_claim_evidence_verified": target.evidence_verified,
            "locally_closed": analysis.locally_closed,
            "blocker_count": analysis.blockers.len(),
            "blockers": analysis.blockers.iter().map(|blocker| json!({
                "capability_id": blocker.capability_id,
                "reason_code": blocker.reason_code
            })).collect::<Vec<_>>()
        },
        "claim_ceiling": "Synthetic capability-dependency graph only; not an assessment of real lunar industrial feasibility, local-content accounting, hardware qualification, or settlement self-sufficiency."
    }))
}

fn control_result(capabilities: &[Capability], target_id: &str) -> Result<Analysis, String> {
    let mut nodes = BTreeMap::new();
    for capability in capabilities {
        if nodes.insert(capability.id, capability).is_some() {
            return Err("duplicate capability id in negative control".to_owned());
        }
    }
    Ok(analyze(target_id, &nodes, &mut BTreeSet::new(), &mut BTreeMap::new()))
}

fn execute(source_commit: &str) -> Result<Value, String> {
    if source_commit.len() != 40
        || !source_commit.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err("source commit must be a 40-character lowercase Git SHA-1".to_owned());
    }

    let high = analyze_portfolio(&PORTFOLIOS[0])?;
    let closure = analyze_portfolio(&PORTFOLIOS[1])?;
    if high["target_state"]["locally_closed"] != false {
        return Err("high-local-content negative control incorrectly passed closure".to_owned());
    }
    if high["target_state"]["reported_operational_now"] != true {
        return Err("reference must distinguish current operations from local closure".to_owned());
    }
    if closure["target_state"]["locally_closed"] != true {
        return Err("closure-first reference did not pass the local-closure oracle".to_owned());
    }
    if PORTFOLIOS[0].lifecycle_cost_scu >= PORTFOLIOS[1].lifecycle_cost_scu {
        return Err("fixture must retain a lower-cost but non-closed nominal plan".to_owned());
    }

    let missing_nodes: [Capability; 1] = [
        Capability { id: "target", reported_operational_now: true, evidence_verified: true, locally_operated: true, recurring_import_dependency: false, dependencies: &["absent_dependency"] },
    ];
    let missing = control_result(&missing_nodes, "target")?;
    if missing.locally_closed || !missing.blockers.iter().any(|b| b.reason_code == "MISSING_CAPABILITY") {
        return Err("missing-dependency negative control did not fail closed".to_owned());
    }

    let cycle_nodes: [Capability; 2] = [
        Capability { id: "cycle_a", reported_operational_now: true, evidence_verified: true, locally_operated: true, recurring_import_dependency: false, dependencies: &["cycle_b"] },
        Capability { id: "cycle_b", reported_operational_now: true, evidence_verified: true, locally_operated: true, recurring_import_dependency: false, dependencies: &["cycle_a"] },
    ];
    let cycle = control_result(&cycle_nodes, "cycle_a")?;
    if cycle.locally_closed || !cycle.blockers.iter().any(|b| b.reason_code == "DEPENDENCY_CYCLE") {
        return Err("cycle negative control did not fail closed".to_owned());
    }

    let high_blockers = high["target_state"]["blockers"].as_array().ok_or("missing blocker array")?;
    for expected in [
        ("metal_refining", "EVIDENCE_UNVERIFIED"),
        ("sensor_module", "NONLOCAL_OPERATION"),
        ("sensor_module", "RECURRING_IMPORT_DEPENDENCY"),
    ] {
        if !high_blockers.iter().any(|b| b["capability_id"] == expected.0 && b["reason_code"] == expected.1) {
            return Err(format!("high-local-content fixture missing expected blocker {}:{}", expected.0, expected.1));
        }
    }

    let source_digest = format!("{:x}", Sha256::digest(include_bytes!("industrial_closure_reference.rs")));
    Ok(json!({
        "receipt_schema_version": "0.1.0",
        "reference_version": "industrial-capability-closure-v0.1",
        "source_commit": source_commit,
        "reference_source_sha256": source_digest,
        "result": "pass",
        "evidence_class": "synthetic_dependency_graph_contract",
        "definition": "A target is locally closed only when its evidence-verified, operational, locally operated capability and every recursive dependency satisfy the same requirements without recurring imports.",
        "metric_separation": [
            "claimed_local_content_percent",
            "reported_operational_now",
            "operational_claim_evidence_verified",
            "locally_closed",
            "recurring_import_dependency"
        ],
        "portfolios": [high, closure],
        "negative_controls": [
            {"id":"missing_dependency","result":"pass","expected_reason":"MISSING_CAPABILITY"},
            {"id":"dependency_cycle","result":"pass","expected_reason":"DEPENDENCY_CYCLE"},
            {"id":"unverified_refinery","result":"pass","expected_reason":"EVIDENCE_UNVERIFIED"},
            {"id":"recurring_imported_sensor","result":"pass","expected_reason":"RECURRING_IMPORT_DEPENDENCY"}
        ],
        "claim_ceiling": "Synthetic dependency-graph decision contract only. A passing receipt does not show that lunar metallurgy, electronics manufacturing, power or repair processes are physically achievable at stated cost, scale or reliability."
    }))
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 || args[1] != "--source-commit" {
        return Err("usage: cargo run --example industrial_closure_reference -- --source-commit <40-char-lowercase-git-sha>".to_owned());
    }
    println!("{}", serde_json::to_string_pretty(&execute(&args[2])?).map_err(|e| e.to_string())?);
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("industrial_closure_reference: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_local_content_does_not_override_unverified_and_imported_bottlenecks() {
        let report = analyze_portfolio(&PORTFOLIOS[0]).unwrap();
        assert_eq!(report["claimed_local_content_percent"], 92);
        assert_eq!(report["target_state"]["reported_operational_now"], true);
        assert_eq!(report["target_state"]["locally_closed"], false);
        let blockers = report["target_state"]["blockers"].as_array().unwrap();
        assert!(blockers.iter().any(|b| b["capability_id"] == "metal_refining" && b["reason_code"] == "EVIDENCE_UNVERIFIED"));
        assert!(blockers.iter().any(|b| b["capability_id"] == "sensor_module" && b["reason_code"] == "RECURRING_IMPORT_DEPENDENCY"));
    }

    #[test]
    fn lower_local_content_closure_first_plan_passes_target_closure() {
        let report = analyze_portfolio(&PORTFOLIOS[1]).unwrap();
        assert_eq!(report["claimed_local_content_percent"], 78);
        assert_eq!(report["lifecycle_cost_scu"], 120);
        assert_eq!(report["target_state"]["locally_closed"], true);
        assert_eq!(report["target_state"]["blocker_count"], 0);
    }

    #[test]
    fn operational_status_and_local_closure_are_separate_metrics() {
        let report = analyze_portfolio(&PORTFOLIOS[0]).unwrap();
        assert_eq!(report["target_state"]["reported_operational_now"], true);
        assert_eq!(report["target_state"]["operational_claim_evidence_verified"], true);
        assert_eq!(report["target_state"]["locally_closed"], false);
    }

    #[test]
    fn missing_dependencies_fail_closed_with_specific_reason() {
        let missing_nodes: [Capability; 1] = [
            Capability { id: "target", reported_operational_now: true, evidence_verified: true, locally_operated: true, recurring_import_dependency: false, dependencies: &["not_supplied"] },
        ];
        let result = control_result(&missing_nodes, "target").unwrap();
        assert!(!result.locally_closed);
        assert!(result.blockers.iter().any(|b| b.reason_code == "MISSING_CAPABILITY"));
    }

    #[test]
    fn dependency_cycles_fail_closed_without_infinite_recursion() {
        let cycle_nodes: [Capability; 2] = [
            Capability { id: "a", reported_operational_now: true, evidence_verified: true, locally_operated: true, recurring_import_dependency: false, dependencies: &["b"] },
            Capability { id: "b", reported_operational_now: true, evidence_verified: true, locally_operated: true, recurring_import_dependency: false, dependencies: &["a"] },
        ];
        let result = control_result(&cycle_nodes, "a").unwrap();
        assert!(!result.locally_closed);
        assert!(result.blockers.iter().any(|b| b.reason_code == "DEPENDENCY_CYCLE"));
    }

    #[test]
    fn duplicate_capability_ids_are_rejected() {
        let duplicate: [Capability; 2] = [
            Capability { id: "same", reported_operational_now: true, evidence_verified: true, locally_operated: true, recurring_import_dependency: false, dependencies: &POWER_DEPS },
            Capability { id: "same", reported_operational_now: true, evidence_verified: true, locally_operated: true, recurring_import_dependency: false, dependencies: &POWER_DEPS },
        ];
        let portfolio = Portfolio { id: "duplicate", claimed_local_content_percent: 50, lifecycle_cost_scu: 10, target_capability_id: "same", capabilities: &duplicate };
        assert!(validate_portfolio(&portfolio).is_err());
    }

    #[test]
    fn invalid_percent_and_missing_target_fail_validation() {
        let invalid_percent = Portfolio { id: "bad_percent", claimed_local_content_percent: 101, lifecycle_cost_scu: 10, target_capability_id: "power_generation", capabilities: &CLOSURE_FIRST_CAPABILITIES };
        assert!(validate_portfolio(&invalid_percent).is_err());
        let missing_target = Portfolio { id: "missing_target", claimed_local_content_percent: 50, lifecycle_cost_scu: 10, target_capability_id: "absent", capabilities: &CLOSURE_FIRST_CAPABILITIES };
        assert!(validate_portfolio(&missing_target).is_err());
    }

    #[test]
    fn receipt_binds_source_and_negative_control_outcomes() {
        let receipt = execute(&"a".repeat(40)).unwrap();
        assert_eq!(receipt["result"], "pass");
        assert_eq!(receipt["negative_controls"].as_array().unwrap().len(), 4);
        assert!(receipt["reference_source_sha256"].as_str().unwrap().len() == 64);
        assert_eq!(receipt["portfolios"][1]["target_state"]["locally_closed"], true);
    }
}
