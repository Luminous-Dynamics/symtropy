use jsonschema::Validator;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

const SCHEMA_ID: &str = "urn:luminous-dynamics:mobility:repair-task-graph:0.1";
const CORPUS_VERSION: &str = "repair-task-graph-fixtures/0.1";
const FIXTURE_IDS: [&str; 16] = [
    "RTG01_VALID_ESTIMATED_GRAPH",
    "RTG02_VALID_OBSERVED_SHAPE",
    "RTG03_CYCLIC_GRAPH",
    "RTG04_DUPLICATE_NODE_ID",
    "RTG05_DANGLING_EDGE",
    "RTG06_MISSING_POSTCHECK_NODE",
    "RTG07_MISSING_SAFETY_ISOLATION",
    "RTG08_NEGATIVE_DURATION",
    "RTG09_INVALID_TIME_UNIT",
    "RTG10_SINGLE_SAMPLE_PROMOTED_TO_OBSERVED",
    "RTG11_UNBOUND_COMPONENT_REVISION",
    "RTG12_UNDECLARED_PROCEDURE_RIGHTS",
    "RTG13_OBSERVED_GRAPH_WITH_ESTIMATED_TIMES",
    "RTG14_MISSING_SAFETY_PRECONDITION",
    "RTG15_MISSING_CLAIM_CEILING",
    "RTG16_DUPLICATE_EDGE",
];

type Errors = Vec<String>;
type AuditResult<T> = Result<T, Errors>;

#[derive(Debug, PartialEq, Eq)]
struct AuditSummary {
    fixture_count: usize,
    accepted_count: usize,
    schema_rejection_count: usize,
    semantic_rejection_count: usize,
    schema_sha256: String,
    fixture_sha256: String,
}

fn text_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn default_schema_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas/transport/repair_task_graph_v0_1.schema.json")
}

fn default_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/personal_vehicle/repair_task_graph_fixtures_v0_1.json")
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

fn schema_errors(validator: &Validator, instance: &Value) -> Vec<String> {
    validator.iter_errors(instance).map(|error| error.to_string()).collect()
}

fn require_string(value: &Value, key: &str, at: &str, errors: &mut Errors) -> Option<String> {
    match text_at(value, key) {
        Some(text) if !text.trim().is_empty() => Some(text.to_owned()),
        _ => {
            errors.push(format!("{at}.{key}: missing, blank, or not a string"));
            None
        }
    }
}

fn reaches(
    start: &str,
    target: &str,
    graph: &BTreeMap<String, BTreeSet<String>>,
) -> bool {
    let mut pending = vec![start.to_owned()];
    let mut visited = BTreeSet::new();
    while let Some(node) = pending.pop() {
        if node == target {
            return true;
        }
        if !visited.insert(node.clone()) {
            continue;
        }
        if let Some(next) = graph.get(&node) {
            pending.extend(next.iter().cloned());
        }
    }
    false
}

fn has_upstream_isolation(
    node_id: &str,
    reverse: &BTreeMap<String, BTreeSet<String>>,
    kinds: &BTreeMap<String, String>,
) -> bool {
    let mut pending = vec![node_id.to_owned()];
    let mut visited = BTreeSet::new();
    while let Some(node) = pending.pop() {
        if !visited.insert(node.clone()) {
            continue;
        }
        if node != node_id && kinds.get(&node).is_some_and(|kind| kind == "safe_isolation") {
            return true;
        }
        if let Some(previous) = reverse.get(&node) {
            pending.extend(previous.iter().cloned());
        }
    }
    false
}

fn validate_graph_semantics(graph: &Value) -> Errors {
    let mut errors = Vec::new();
    let Some(nodes) = graph.get("task_nodes").and_then(Value::as_array) else {
        return vec!["task_nodes must be an array".to_owned()];
    };
    let Some(edges) = graph.get("edges").and_then(Value::as_array) else {
        return vec!["edges must be an array".to_owned()];
    };

    let mut kinds = BTreeMap::<String, String>::new();
    let mut isolation_required = BTreeMap::<String, bool>::new();
    for (index, node) in nodes.iter().enumerate() {
        let at = format!("task_nodes[{index}]");
        let Some(id) = require_string(node, "node_id", &at, &mut errors) else {
            continue;
        };
        let Some(kind) = require_string(node, "step_kind", &at, &mut errors) else {
            continue;
        };
        if kinds.insert(id.clone(), kind.clone()).is_some() {
            errors.push(format!("duplicate task node id {id}"));
        }
        isolation_required.insert(
            id,
            node.get("requires_isolation").and_then(Value::as_bool).unwrap_or(false),
        );
        for (precondition_index, precondition) in node
            .get("safety_preconditions")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .enumerate()
        {
            let criticality = text_at(precondition, "criticality").unwrap_or("");
            let must_hold = precondition
                .get("must_hold_before_task")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if criticality == "hard_stop" && !must_hold {
                errors.push(format!(
                    "{at}.safety_preconditions[{precondition_index}]: hard-stop precondition must hold before the task"
                ));
            }
        }
    }

    let node_ids: BTreeSet<String> = kinds.keys().cloned().collect();
    let mut graph_edges = BTreeMap::<String, BTreeSet<String>>::new();
    let mut reverse_edges = BTreeMap::<String, BTreeSet<String>>::new();
    let mut indegree: BTreeMap<String, usize> =
        node_ids.iter().cloned().map(|id| (id, 0)).collect();
    let mut edge_pairs = BTreeSet::<(String, String)>::new();

    for (index, edge) in edges.iter().enumerate() {
        let at = format!("edges[{index}]");
        let from = text_at(edge, "from_node").unwrap_or("").to_owned();
        let to = text_at(edge, "to_node").unwrap_or("").to_owned();
        if from.is_empty() || to.is_empty() {
            errors.push(format!("{at}: from_node and to_node are required"));
            continue;
        }
        if !node_ids.contains(&from) {
            errors.push(format!("{at}: source node {from} does not exist"));
            continue;
        }
        if !node_ids.contains(&to) {
            errors.push(format!("{at}: target node {to} does not exist"));
            continue;
        }
        if !edge_pairs.insert((from.clone(), to.clone())) {
            errors.push(format!("{at}: duplicate dependency edge {from} -> {to}"));
            continue;
        }
        graph_edges.entry(from.clone()).or_default().insert(to.clone());
        reverse_edges.entry(to.clone()).or_default().insert(from.clone());
        *indegree.entry(to).or_default() += 1;
    }

    let mut ready: VecDeque<String> = indegree
        .iter()
        .filter_map(|(id, degree)| (*degree == 0).then_some(id.clone()))
        .collect();
    let mut visited_count = 0usize;
    while let Some(node) = ready.pop_front() {
        visited_count += 1;
        if let Some(children) = graph_edges.get(&node) {
            for child in children {
                if let Some(degree) = indegree.get_mut(child) {
                    *degree = degree.saturating_sub(1);
                    if *degree == 0 {
                        ready.push_back(child.clone());
                    }
                }
            }
        }
    }
    if visited_count != node_ids.len() {
        errors.push("dependency graph contains a cycle".to_owned());
    }

    let postcheck_id = text_at(graph, "post_repair_verification_task_id").unwrap_or("");
    match kinds.get(postcheck_id) {
        None => errors.push(format!("post-repair verification node {postcheck_id} does not exist")),
        Some(kind) if kind != "post_repair_verification" => {
            errors.push(format!(
                "post-repair verification reference {postcheck_id} has step kind {kind}, not post_repair_verification"
            ));
        }
        Some(_) => {
            if graph_edges.get(postcheck_id).is_some_and(|next| !next.is_empty()) {
                errors.push("post-repair verification node must be terminal".to_owned());
            }
            for id in &node_ids {
                if !reaches(id, postcheck_id, &graph_edges) {
                    errors.push(format!("task node {id} cannot reach the post-repair verification node"));
                }
            }
        }
    }

    for node in nodes {
        let Some(id) = text_at(node, "node_id") else {
            continue;
        };
        let kind = text_at(node, "step_kind").unwrap_or("");
        let explicitly_requires = isolation_required.get(id).copied().unwrap_or(false);
        let implicitly_requires =
            ["access", "removal", "repair_replace", "reassembly"].contains(&kind);
        if (explicitly_requires || implicitly_requires)
            && !has_upstream_isolation(id, &reverse_edges, &kinds)
        {
            errors.push(format!(
                "task node {id} requires upstream safe_isolation (declared flag: {explicitly_requires})"
            ));
        }
    }

    let evidence_class = text_at(graph, "evidence_class").unwrap_or("");
    for node in nodes {
        let id = text_at(node, "node_id").unwrap_or("<unknown>");
        let basis = node
            .get("duration")
            .and_then(|duration| duration.get("basis"))
            .and_then(Value::as_str)
            .unwrap_or("");
        match evidence_class {
            "procedure_hypothesis" if basis != "estimated" => errors.push(format!(
                "procedure_hypothesis requires estimated duration for node {id}"
            )),
            "observed_workshop_trial" if basis != "observed" => errors.push(format!(
                "observed_workshop_trial requires observed duration for node {id}"
            )),
            _ => {}
        }
    }

    errors
}

fn validate_fixtures(schema: &Value, fixtures_document: &Value) -> AuditResult<AuditSummary> {
    let validator = schema_validator(schema).map_err(|error| vec![error])?;
    let mut errors = Vec::new();

    if text_at(fixtures_document, "corpus_version") != Some(CORPUS_VERSION) {
        errors.push(format!("fixture corpus version must be {CORPUS_VERSION}"));
    }
    if text_at(fixtures_document, "status")
        != Some("synthetic-contract-fixtures-not-repair-instructions")
    {
        errors.push("fixture corpus must preserve its synthetic/not-an-instruction status".to_owned());
    }
    let Some(fixtures) = fixtures_document.get("fixtures").and_then(Value::as_array) else {
        return Err(vec!["fixtures must be an array".to_owned()]);
    };
    if fixtures.len() != FIXTURE_IDS.len() {
        errors.push(format!("expected {} fixtures, found {}", FIXTURE_IDS.len(), fixtures.len()));
    }

    let mut seen = BTreeSet::new();
    let mut accepted_count = 0;
    let mut schema_rejection_count = 0;
    let mut semantic_rejection_count = 0;

    for (index, fixture) in fixtures.iter().enumerate() {
        let at = format!("fixtures[{index}]");
        let id = require_string(fixture, "id", &at, &mut errors);
        let expectation = require_string(fixture, "expectation", &at, &mut errors);
        let input = fixture.get("input");
        if let Some(id) = id.as_ref() {
            if !seen.insert(id.clone()) {
                errors.push(format!("{at}: duplicate fixture ID {id}"));
            }
            if !FIXTURE_IDS.contains(&id.as_str()) {
                errors.push(format!("{at}: unexpected fixture ID {id}"));
            }
        }
        let Some(expectation) = expectation else {
            continue;
        };
        let Some(input) = input else {
            errors.push(format!("{at}.input is missing"));
            continue;
        };
        let validation_errors = schema_errors(&validator, input);
        match expectation.as_str() {
            "accept" => {
                accepted_count += 1;
                if !validation_errors.is_empty() {
                    errors.push(format!("{at}: expected schema acceptance, got {}", validation_errors.join("; ")));
                } else {
                    let semantic_errors = validate_graph_semantics(input);
                    if !semantic_errors.is_empty() {
                        errors.push(format!("{at}: expected semantic acceptance, got {}", semantic_errors.join("; ")));
                    }
                }
            }
            "reject_schema" => {
                schema_rejection_count += 1;
                if validation_errors.is_empty() {
                    errors.push(format!("{at}: expected schema rejection, but input was schema-valid"));
                }
            }
            "reject_semantics" => {
                semantic_rejection_count += 1;
                if !validation_errors.is_empty() {
                    errors.push(format!("{at}: semantic negative control is schema-invalid: {}", validation_errors.join("; ")));
                } else {
                    let semantic_errors = validate_graph_semantics(input);
                    if semantic_errors.is_empty() {
                        errors.push(format!("{at}: expected semantic rejection, but graph was accepted"));
                    }
                }
            }
            other => errors.push(format!("{at}: unknown expectation {other}")),
        }
    }

    for id in FIXTURE_IDS {
        if !seen.contains(id) {
            errors.push(format!("missing required fixture {id}"));
        }
    }

    let claim_ceiling = fixtures_document.get("claim_ceiling").and_then(Value::as_array);
    if !claim_ceiling.is_some_and(|items| {
        items.iter().filter_map(Value::as_str).any(|value| value.contains("schema shape and graph invariants only"))
            && items.iter().filter_map(Value::as_str).any(|value| value.contains("synthetic and do not represent actual workshop measurements"))
    }) {
        errors.push("claim ceiling must preserve the synthetic timing/repair-instruction limits".to_owned());
    }

    if errors.is_empty() {
        Ok(AuditSummary {
            fixture_count: fixtures.len(),
            accepted_count,
            schema_rejection_count,
            semantic_rejection_count,
            schema_sha256: String::new(),
            fixture_sha256: String::new(),
        })
    } else {
        Err(errors)
    }
}

fn main() -> ExitCode {
    let mut args = env::args_os().skip(1);
    let mut schema_path = default_schema_path();
    let mut fixture_path = default_fixture_path();
    while let Some(arg) = args.next() {
        if arg == "--help" || arg == "-h" {
            println!("Usage: repair-task-graph-audit [--schema PATH] [--fixtures PATH]");
            println!("Validates graph shape and dependency/safety preconditions; does not validate a real repair procedure.");
            return ExitCode::SUCCESS;
        } else if arg == "--schema" {
            let Some(path) = args.next() else {
                eprintln!("REPAIR_GRAPH_AUDIT_FAIL: --schema requires a path");
                return ExitCode::from(2);
            };
            schema_path = PathBuf::from(path);
        } else if arg == "--fixtures" {
            let Some(path) = args.next() else {
                eprintln!("REPAIR_GRAPH_AUDIT_FAIL: --fixtures requires a path");
                return ExitCode::from(2);
            };
            fixture_path = PathBuf::from(path);
        } else {
            eprintln!("REPAIR_GRAPH_AUDIT_FAIL: unknown argument {}", arg.to_string_lossy());
            return ExitCode::from(2);
        }
    }

    let load = |path: &PathBuf, label: &str| -> Result<(Vec<u8>, Value), String> {
        let bytes = fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let value = serde_json::from_slice(&bytes).map_err(|error| format!("invalid {label} JSON: {error}"))?;
        Ok((bytes, value))
    };

    let (schema_raw, schema) = match load(&schema_path, "schema") {
        Ok(value) => value,
        Err(error) => {
            eprintln!("REPAIR_GRAPH_AUDIT_FAIL: {error}");
            return ExitCode::from(2);
        }
    };
    let (fixtures_raw, fixtures) = match load(&fixture_path, "fixtures") {
        Ok(value) => value,
        Err(error) => {
            eprintln!("REPAIR_GRAPH_AUDIT_FAIL: {error}");
            return ExitCode::from(2);
        }
    };

    match validate_fixtures(&schema, &fixtures) {
        Ok(mut summary) => {
            summary.schema_sha256 = format!("{:x}", Sha256::digest(&schema_raw));
            summary.fixture_sha256 = format!("{:x}", Sha256::digest(&fixtures_raw));
            println!(
                "REPAIR_GRAPH_AUDIT_PASS fixtures={} accepted={} schema_rejections={} semantic_rejections={} schema_sha256={} fixtures_sha256={}",
                summary.fixture_count,
                summary.accepted_count,
                summary.schema_rejection_count,
                summary.semantic_rejection_count,
                summary.schema_sha256,
                summary.fixture_sha256
            );
            println!("CLAIM_CEILING task-graph/schema semantics only; no repair instructions or measured improvement");
            ExitCode::SUCCESS
        }
        Err(errors) => {
            eprintln!("REPAIR_GRAPH_AUDIT_FAIL errors={}", errors.len());
            for error in errors {
                eprintln!("- {error}");
            }
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema() -> Value {
        serde_json::from_str(include_str!("../../../schemas/transport/repair_task_graph_v0_1.schema.json"))
            .expect("repair task graph schema must parse")
    }

    fn fixtures() -> Value {
        serde_json::from_str(include_str!("../../../fixtures/personal_vehicle/repair_task_graph_fixtures_v0_1.json"))
            .expect("repair task graph fixtures must parse")
    }

    fn errors_for_graph(graph: &Value) -> Vec<String> {
        let schema = schema();
        let validator = schema_validator(&schema).expect("schema should compile");
        let mut errors = schema_errors(&validator, graph);
        if errors.is_empty() {
            errors.extend(validate_graph_semantics(graph));
        }
        errors
    }

    #[test]
    fn schema_and_all_frozen_fixtures_match_their_oracles() {
        let schema = schema();
        let fixtures = fixtures();
        assert!(schema_validator(&schema).is_ok());
        let result = validate_fixtures(&schema, &fixtures).expect("frozen repair graph corpus");
        assert_eq!(result.fixture_count, 16);
        assert_eq!(result.accepted_count, 2);
        assert_eq!(result.schema_rejection_count, 8);
        assert_eq!(result.semantic_rejection_count, 6);
    }

    #[test]
    fn cyclic_graph_is_rejected() {
        let fixtures = fixtures();
        let graph = &fixtures["fixtures"][2]["input"];
        assert!(!validate_graph_semantics(graph).is_empty());
        assert!(errors_for_graph(graph).iter().any(|error| error.contains("cycle")));
    }

    #[test]
    fn disconnected_or_dangling_tasks_are_rejected() {
        let fixtures = fixtures();
        let graph = &fixtures["fixtures"][4]["input"];
        assert!(!errors_for_graph(graph).is_empty());
    }

    #[test]
    fn access_cannot_skip_upstream_isolation() {
        let fixtures = fixtures();
        let graph = &fixtures["fixtures"][6]["input"];
        assert!(validate_graph_semantics(graph).iter().any(|error| error.contains("upstream safe_isolation")));
    }

    #[test]
    fn duplicate_dependency_edge_is_rejected() {
        let fixtures = fixtures();
        let graph = &fixtures["fixtures"][15]["input"];
        assert!(validate_graph_semantics(graph).iter().any(|error| error.contains("duplicate dependency edge")));
    }

    #[test]
    fn estimated_time_cannot_be_claimed_as_observed() {
        let fixtures = fixtures();
        let graph = &fixtures["fixtures"][12]["input"];
        let schema = schema();
        let validator = schema_validator(&schema).expect("schema compiles");
        assert!(!schema_errors(&validator, graph).is_empty());
    }

    #[test]
    fn fixture_oracle_cannot_be_weakened_to_accept_a_cycle() {
        let schema = schema();
        let mut fixtures = fixtures();
        fixtures["fixtures"][2]["expectation"] = Value::String("accept".to_owned());
        assert!(validate_fixtures(&schema, &fixtures).is_err());
    }

    #[test]
    fn duplicate_fixture_identity_is_rejected() {
        let schema = schema();
        let mut fixtures = fixtures();
        fixtures["fixtures"][1]["id"] = fixtures["fixtures"][0]["id"].clone();
        assert!(validate_fixtures(&schema, &fixtures).is_err());
    }
}
