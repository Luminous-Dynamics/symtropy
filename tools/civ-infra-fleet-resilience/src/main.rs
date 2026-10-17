use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::{env, fs, process};

const EMBEDDED_FIXTURE: &str =
    include_str!("../../../docs/research/civ-infra-resilience/fixtures/fleet-resilience-v0.1.json");

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    schema_version: String,
    fixture_id: String,
    classification: String,
    claim_ceiling: String,
    warning: String,
    evidence_vocabulary: Vec<String>,
    clock: ClockSpec,
    policies: Vec<PolicySpec>,
    resources: Vec<ResourceSpec>,
    task_graph: Vec<TaskSpec>,
    disturbance_cases: Vec<DisturbanceCase>,
    expected_evaluation_rules: Vec<EvaluationRule>,
    qualification_status: String,
    required_future_checks: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ClockSpec {
    unit: String,
    interval_semantics: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PolicySpec {
    policy_id: String,
    name: String,
    access_to_unobserved_state: bool,
    #[serde(default)]
    deployable_policy: Option<bool>,
    #[serde(default)]
    purpose: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ResourceSpec {
    resource_id: String,
    capacity: u32,
    resource_kind: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct TaskSpec {
    task_id: String,
    duration_ticks: u32,
    requires: Vec<String>,
    acceptance_evidence: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct DisturbanceCase {
    case_id: String,
    name: String,
    injection_tick: u64,
    observable_tick: u64,
    event: String,
    affected_resources: Vec<String>,
    affected_tasks: Vec<String>,
    invalidates: Vec<String>,
    #[serde(default)]
    recoverability: Option<String>,
    recovery_evidence: Vec<String>,
    required_invariants: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct EvaluationRule {
    rule_id: String,
    assertion: String,
}

#[derive(Clone, Debug, Serialize)]
struct Counts {
    policies: usize,
    resources: usize,
    task_nodes: usize,
    disturbance_cases: usize,
    evidence_vocabulary_items: usize,
    evaluation_rules: usize,
}

#[derive(Clone, Debug, Serialize)]
struct CaseSummary {
    case_id: String,
    name: String,
    injection_tick: u64,
    observable_tick: u64,
    affected_resource_count: usize,
    affected_task_count: usize,
    recovery_evidence_count: usize,
    required_invariant_count: usize,
    status: String,
}

#[derive(Clone, Debug, Serialize)]
struct ValidationReport {
    schema_version: String,
    fixture_id: String,
    input_sha256: String,
    validation_status: String,
    benchmark_executed: bool,
    policy_runs_performed: usize,
    claim_ceiling: String,
    counts: Counts,
    policy_ids: Vec<String>,
    disturbance_cases: Vec<CaseSummary>,
    evaluation_rule_ids: Vec<String>,
    explicit_limitations: Vec<String>,
}

fn main() {
    let mut args = env::args_os().skip(1);
    let input_bytes = match args.next() {
        Some(path) => match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) => {
                eprintln!("cannot read fixture {}: {error}", path.to_string_lossy());
                process::exit(2);
            }
        },
        None => EMBEDDED_FIXTURE.as_bytes().to_vec(),
    };

    if args.next().is_some() {
        eprintln!("usage: civ-infra-fleet-resilience [fixture.json]");
        process::exit(2);
    }

    match verify_fixture_bytes(&input_bytes) {
        Ok(report) => match serde_json::to_string_pretty(&report) {
            Ok(json) => println!("{json}"),
            Err(error) => {
                eprintln!("cannot serialize validation report: {error}");
                process::exit(2);
            }
        },
        Err(errors) => {
            for error in errors {
                eprintln!("fixture rejected: {error}");
            }
            process::exit(2);
        }
    }
}

fn verify_fixture_bytes(input_bytes: &[u8]) -> Result<ValidationReport, Vec<String>> {
    let fixture: Fixture = serde_json::from_slice(input_bytes).map_err(|error| {
        vec![format!("fixture JSON does not match the strict v0.1 schema: {error}")]
    })?;

    let mut errors = validate_fixture(&fixture);
    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }

    let mut policy_ids: Vec<String> = fixture
        .policies
        .iter()
        .map(|policy| policy.policy_id.clone())
        .collect();
    policy_ids.sort();

    let mut evaluation_rule_ids: Vec<String> = fixture
        .expected_evaluation_rules
        .iter()
        .map(|rule| rule.rule_id.clone())
        .collect();
    evaluation_rule_ids.sort();

    let mut disturbance_cases: Vec<CaseSummary> = fixture
        .disturbance_cases
        .iter()
        .map(|case| CaseSummary {
            case_id: case.case_id.clone(),
            name: case.name.clone(),
            injection_tick: case.injection_tick,
            observable_tick: case.observable_tick,
            affected_resource_count: case.affected_resources.len(),
            affected_task_count: case.affected_tasks.len(),
            recovery_evidence_count: case.recovery_evidence.len(),
            required_invariant_count: case.required_invariants.len(),
            status: "declared_not_simulated".to_string(),
        })
        .collect();
    disturbance_cases.sort_by(|left, right| left.case_id.cmp(&right.case_id));

    let digest = Sha256::digest(input_bytes);
    Ok(ValidationReport {
        schema_version: "luminous.civ_infra.fleet_resilience_manifest_report.v0.1".to_string(),
        fixture_id: fixture.fixture_id.clone(),
        input_sha256: format!("{digest:x}"),
        validation_status: "fixture_schema_valid_only".to_string(),
        benchmark_executed: false,
        policy_runs_performed: 0,
        claim_ceiling: fixture.claim_ceiling.clone(),
        counts: Counts {
            policies: fixture.policies.len(),
            resources: fixture.resources.len(),
            task_nodes: fixture.task_graph.len(),
            disturbance_cases: fixture.disturbance_cases.len(),
            evidence_vocabulary_items: fixture.evidence_vocabulary.len(),
            evaluation_rules: fixture.expected_evaluation_rules.len(),
        },
        policy_ids,
        disturbance_cases,
        evaluation_rule_ids,
        explicit_limitations: vec![
            "No policy, schedule, robot, or physical process was simulated.".to_string(),
            "This report verifies the manifest schema and cross-references only.".to_string(),
            "Synthetic fixture validity is not evidence of construction safety or field productivity.".to_string(),
        ],
    })
}

fn validate_fixture(fixture: &Fixture) -> Vec<String> {
    let mut errors = Vec::new();

    if fixture.schema_version != "civ-infra-fleet-resilience/0.1" {
        errors.push("unsupported schema_version".to_string());
    }
    if fixture.fixture_id.trim().is_empty() {
        errors.push("fixture_id must not be empty".to_string());
    }
    if fixture.classification != "synthetic_only" {
        errors.push("classification must remain synthetic_only".to_string());
    }
    if fixture.claim_ceiling.trim().is_empty() || fixture.warning.trim().is_empty() {
        errors.push("claim_ceiling and warning must be explicit".to_string());
    }
    if fixture.qualification_status != "proposed_not_executed" {
        errors.push("qualification_status must not imply benchmark execution".to_string());
    }
    if fixture.clock.unit != "fixture_tick" {
        errors.push("clock unit must be fixture_tick in v0.1".to_string());
    }
    if fixture.clock.interval_semantics != "half_open" {
        errors.push("clock interval semantics must be half_open".to_string());
    }
    if fixture.required_future_checks.is_empty() {
        errors.push("required_future_checks must not be empty".to_string());
    }

    check_unique(
        fixture.policies.iter().map(|item| item.policy_id.as_str()),
        "policy_id",
        &mut errors,
    );
    check_unique(
        fixture.resources.iter().map(|item| item.resource_id.as_str()),
        "resource_id",
        &mut errors,
    );
    check_unique(
        fixture.task_graph.iter().map(|item| item.task_id.as_str()),
        "task_id",
        &mut errors,
    );
    check_unique(
        fixture
            .disturbance_cases
            .iter()
            .map(|item| item.case_id.as_str()),
        "case_id",
        &mut errors,
    );
    check_unique(
        fixture.evidence_vocabulary.iter().map(String::as_str),
        "evidence vocabulary item",
        &mut errors,
    );
    check_unique(
        fixture
            .expected_evaluation_rules
            .iter()
            .map(|item| item.rule_id.as_str()),
        "evaluation rule id",
        &mut errors,
    );

    for policy in &fixture.policies {
        if policy.policy_id.trim().is_empty() || policy.name.trim().is_empty() {
            errors.push("policy ID and name must not be empty".to_string());
        }
        if policy.access_to_unobserved_state {
            errors.push(format!(
                "policy {} is allowed to observe an event before its observable_tick",
                policy.policy_id
            ));
        }
        if policy.policy_id == "B3" {
            if policy.deployable_policy != Some(false) {
                errors.push("B3 must be explicitly marked non-deployable".to_string());
            }
            if policy.purpose.as_deref() != Some("reference_bound_only") {
                errors.push("B3 must be limited to the finite reference bound".to_string());
            }
        }
    }
    let policy_ids: BTreeSet<&str> = fixture
        .policies
        .iter()
        .map(|policy| policy.policy_id.as_str())
        .collect();
    for required in ["B0", "B1", "B2", "B3"] {
        if !policy_ids.contains(required) {
            errors.push(format!("required comparison policy {required} is absent"));
        }
    }

    for resource in &fixture.resources {
        if resource.resource_id.trim().is_empty() || resource.resource_kind.trim().is_empty() {
            errors.push("resource ID and kind must not be empty".to_string());
        }
        if resource.capacity == 0 {
            errors.push(format!(
                "resource {} must have positive capacity",
                resource.resource_id
            ));
        }
    }
    let resource_ids: BTreeSet<&str> = fixture
        .resources
        .iter()
        .map(|resource| resource.resource_id.as_str())
        .collect();

    for task in &fixture.task_graph {
        if task.task_id.trim().is_empty() {
            errors.push("task ID must not be empty".to_string());
        }
        if task.duration_ticks == 0 {
            errors.push(format!("task {} duration must be positive", task.task_id));
        }
        if task.acceptance_evidence.is_empty() {
            errors.push(format!(
                "task {} must declare its acceptance evidence",
                task.task_id
            ));
        }
        check_unique(
            task.requires.iter().map(String::as_str),
            &format!("dependency for task {}", task.task_id),
            &mut errors,
        );
        for evidence in &task.acceptance_evidence {
            if !fixture.evidence_vocabulary.iter().any(|known| known == evidence) {
                errors.push(format!(
                    "task {} references unknown acceptance evidence {}",
                    task.task_id, evidence
                ));
            }
        }
    }
    let task_ids: BTreeSet<&str> = fixture
        .task_graph
        .iter()
        .map(|task| task.task_id.as_str())
        .collect();
    for task in &fixture.task_graph {
        for dependency in &task.requires {
            if !task_ids.contains(dependency.as_str()) {
                errors.push(format!(
                    "task {} references missing dependency {}",
                    task.task_id, dependency
                ));
            }
        }
    }

    let dependencies: BTreeMap<String, Vec<String>> = fixture
        .task_graph
        .iter()
        .map(|task| (task.task_id.clone(), task.requires.clone()))
        .collect();
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for task_id in dependencies.keys() {
        visit_task(
            task_id,
            &dependencies,
            &mut visiting,
            &mut visited,
            &mut errors,
        );
    }

    let evidence_ids: BTreeSet<&str> = fixture
        .evidence_vocabulary
        .iter()
        .map(String::as_str)
        .collect();
    for evidence in &fixture.evidence_vocabulary {
        if evidence.trim().is_empty() {
            errors.push("evidence vocabulary items must not be empty".to_string());
        }
    }

    for case in &fixture.disturbance_cases {
        if case.case_id.trim().is_empty()
            || case.name.trim().is_empty()
            || case.event.trim().is_empty()
        {
            errors.push("disturbance case ID, name, and event must not be empty".to_string());
        }
        if case.observable_tick < case.injection_tick {
            errors.push(format!(
                "case {} is observable before it is injected",
                case.case_id
            ));
        }
        if case.recovery_evidence.is_empty() {
            errors.push(format!(
                "case {} must specify recovery evidence",
                case.case_id
            ));
        }
        if case.required_invariants.is_empty() {
            errors.push(format!(
                "case {} must specify required invariants",
                case.case_id
            ));
        }
        for resource_id in &case.affected_resources {
            if !resource_ids.contains(resource_id.as_str()) {
                errors.push(format!(
                    "case {} references missing resource {}",
                    case.case_id, resource_id
                ));
            }
        }
        for task_id in &case.affected_tasks {
            if !task_ids.contains(task_id.as_str()) {
                errors.push(format!(
                    "case {} references missing task {}",
                    case.case_id, task_id
                ));
            }
        }
        for evidence in &case.recovery_evidence {
            if !evidence_ids.contains(evidence.as_str()) {
                errors.push(format!(
                    "case {} references unknown recovery evidence {}",
                    case.case_id, evidence
                ));
            }
        }
    }

    if fixture.disturbance_cases.len() != 13 {
        errors.push("v0.1 frozen corpus must contain exactly 13 cases".to_string());
    }
    let actual_case_ids: BTreeSet<&str> = fixture
        .disturbance_cases
        .iter()
        .map(|case| case.case_id.as_str())
        .collect();
    for number in 1..=13 {
        let required = format!("R-{number:03}");
        if !actual_case_ids.contains(required.as_str()) {
            errors.push(format!("required frozen case {required} is absent"));
        }
    }

    for rule in &fixture.expected_evaluation_rules {
        if rule.rule_id.trim().is_empty() || rule.assertion.trim().is_empty() {
            errors.push("evaluation rule ID and assertion must not be empty".to_string());
        }
    }
    if fixture.expected_evaluation_rules.len() != 7 {
        errors.push("v0.1 must declare exactly seven evaluation rules".to_string());
    }

    errors
}

fn check_unique<'a>(
    values: impl IntoIterator<Item = &'a str>,
    label: &str,
    errors: &mut Vec<String>,
) {
    let mut seen = BTreeSet::new();
    for value in values {
        if value.trim().is_empty() {
            errors.push(format!("{label} must not be empty"));
        } else if !seen.insert(value) {
            errors.push(format!("duplicate {label}: {value}"));
        }
    }
}

fn visit_task(
    task_id: &str,
    dependencies: &BTreeMap<String, Vec<String>>,
    visiting: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
    errors: &mut Vec<String>,
) {
    if visited.contains(task_id) {
        return;
    }
    if !visiting.insert(task_id.to_string()) {
        errors.push(format!("task dependency cycle includes {task_id}"));
        return;
    }
    if let Some(required_tasks) = dependencies.get(task_id) {
        for required in required_tasks {
            if dependencies.contains_key(required) {
                visit_task(required, dependencies, visiting, visited, errors);
            }
        }
    }
    visiting.remove(task_id);
    visited.insert(task_id.to_string());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Fixture {
        serde_json::from_str(EMBEDDED_FIXTURE).expect("embedded fixture is valid JSON")
    }

    fn encode(fixture: &Fixture) -> Vec<u8> {
        serde_json::to_vec(fixture).expect("fixture serialization must succeed")
    }

    #[test]
    fn frozen_fixture_is_schema_valid_but_not_a_benchmark_result() {
        let report = verify_fixture_bytes(EMBEDDED_FIXTURE.as_bytes())
            .expect("frozen manifest must satisfy schema and cross-reference checks");
        assert_eq!(report.validation_status, "fixture_schema_valid_only");
        assert!(!report.benchmark_executed);
        assert_eq!(report.policy_runs_performed, 0);
        assert_eq!(report.counts.policies, 4);
        assert_eq!(report.counts.resources, 5);
        assert_eq!(report.counts.task_nodes, 4);
        assert_eq!(report.counts.disturbance_cases, 13);
        assert_eq!(report.counts.evidence_vocabulary_items, 32);
        assert_eq!(report.counts.evaluation_rules, 7);
    }

    #[test]
    fn exact_input_bytes_are_bound_by_sha256() {
        let bytes = EMBEDDED_FIXTURE.as_bytes();
        let report = verify_fixture_bytes(bytes).expect("fixture validates");
        let expected = format!("{:x}", Sha256::digest(bytes));
        assert_eq!(report.input_sha256, expected);
    }

    #[test]
    fn deterministic_replay_emits_identical_report_bytes() {
        let bytes = EMBEDDED_FIXTURE.as_bytes();
        let first = verify_fixture_bytes(bytes).expect("first verification");
        let second = verify_fixture_bytes(bytes).expect("second verification");
        assert_eq!(
            serde_json::to_vec(&first).expect("serialize first report"),
            serde_json::to_vec(&second).expect("serialize second report")
        );
    }

    #[test]
    fn report_case_order_is_canonical_but_input_digest_remains_exact() {
        let original_bytes = EMBEDDED_FIXTURE.as_bytes();
        let original = verify_fixture_bytes(original_bytes).expect("original validates");

        let mut reordered = fixture();
        reordered.disturbance_cases.reverse();
        let reordered_bytes = encode(&reordered);
        let reordered_report =
            verify_fixture_bytes(&reordered_bytes).expect("reordered fixture validates");

        let original_ids: Vec<&str> = original
            .disturbance_cases
            .iter()
            .map(|case| case.case_id.as_str())
            .collect();
        let reordered_ids: Vec<&str> = reordered_report
            .disturbance_cases
            .iter()
            .map(|case| case.case_id.as_str())
            .collect();
        assert_eq!(original_ids, reordered_ids);
        assert_ne!(original.input_sha256, reordered_report.input_sha256);
    }

    #[test]
    fn duplicate_case_ids_are_rejected() {
        let mut candidate = fixture();
        candidate.disturbance_cases[1].case_id = candidate.disturbance_cases[0].case_id.clone();
        let errors = validate_fixture(&candidate);
        assert!(errors.iter().any(|error| error.contains("duplicate case_id")));
    }

    #[test]
    fn observations_cannot_precede_event_injection() {
        let mut candidate = fixture();
        candidate.disturbance_cases[0].observable_tick = 1;
        let errors = validate_fixture(&candidate);
        assert!(errors.iter().any(|error| error.contains("observable before")));
    }

    #[test]
    fn unresolved_recovery_evidence_is_rejected() {
        let mut candidate = fixture();
        candidate.disturbance_cases[0]
            .recovery_evidence
            .push("not-in-the-evidence-vocabulary".to_string());
        let errors = validate_fixture(&candidate);
        assert!(errors.iter().any(|error| error.contains("unknown recovery evidence")));
    }

    #[test]
    fn unobserved_state_access_is_rejected() {
        let mut candidate = fixture();
        candidate.policies[1].access_to_unobserved_state = true;
        let errors = validate_fixture(&candidate);
        assert!(errors.iter().any(|error| error.contains("observable_tick")));
    }

    #[test]
    fn missing_resource_references_are_rejected() {
        let mut candidate = fixture();
        candidate.disturbance_cases[0]
            .affected_resources
            .push("robot-that-does-not-exist".to_string());
        let errors = validate_fixture(&candidate);
        assert!(errors.iter().any(|error| error.contains("missing resource")));
    }

    #[test]
    fn dependency_cycles_are_rejected() {
        let mut candidate = fixture();
        candidate.task_graph[0]
            .requires
            .push("inspect-water-pipe-01".to_string());
        let errors = validate_fixture(&candidate);
        assert!(errors.iter().any(|error| error.contains("dependency cycle")));
    }

    #[test]
    fn unknown_schema_fields_are_rejected() {
        let mut value: serde_json::Value =
            serde_json::from_str(EMBEDDED_FIXTURE).expect("fixture JSON");
        value["unexpected_authority"] = serde_json::json!(true);
        let bytes = serde_json::to_vec(&value).expect("serialize mutated JSON");
        let errors = verify_fixture_bytes(&bytes).expect_err("unknown field must fail closed");
        assert!(errors.iter().any(|error| error.contains("unexpected_authority")));
    }

    #[test]
    fn missing_frozen_case_is_rejected() {
        let mut candidate = fixture();
        candidate
            .disturbance_cases
            .retain(|case| case.case_id != "R-013");
        let errors = validate_fixture(&candidate);
        assert!(errors.iter().any(|error| error.contains("exactly 13 cases")));
        assert!(errors.iter().any(|error| error.contains("R-013 is absent")));
    }
}
