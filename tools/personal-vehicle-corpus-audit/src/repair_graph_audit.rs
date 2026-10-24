use jsonschema::draft202012;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

const REQUIRED_IDS: [&str; 16] = [
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

fn string_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn graph_semantic_errors(graph: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let Some(nodes) = graph.get("task_nodes").and_then(Value::as_array) else {
        return vec!["task_nodes must be an array".to_owned()];
    };
    let mut by_id = BTreeMap::<String, &Value>::new();
    for (index, node) in nodes.iter().enumerate() {
        let Some(id) = string_at(node, "node_id") else {
            errors.push(format!("task_nodes[{index}] has no node_id"));
            continue;
        };
        if by_id.insert(id.to_owned(), node).is_some() {
            errors.push(format!("duplicate node ID {id}"));
        }
    }

    let Some(edges) = graph.get("edges").and_then(Value::as_array) else {
        return vec!["edges must be an array".to_owned()];
    };
    let mut adjacency = BTreeMap::<String, BTreeSet<String>>::new();
    let mut indegree = BTreeMap::<String, usize>::new();
    let mut seen_edges = BTreeSet::<(String, String, String)>::new();
    let mut seen_dependency_pairs = BTreeSet::<(String, String)>::new();
    for id in by_id.keys() {
        adjacency.insert(id.clone(), BTreeSet::new());
        indegree.insert(id.clone(), 0);
    }
    for (index, edge) in edges.iter().enumerate() {
        let (Some(from), Some(to)) = (string_at(edge, "from_node"), string_at(edge, "to_node"))
        else {
            errors.push(format!("edge {index} has missing endpoint"));
            continue;
        };
        if !by_id.contains_key(from) {
            errors.push(format!("edge {index} references unknown source {from}"));
            continue;
        }
        if !by_id.contains_key(to) {
            errors.push(format!("edge {index} references unknown target {to}"));
            continue;
        }
        let relation = string_at(edge, "relation").unwrap_or("");
        if !seen_edges.insert((from.to_owned(), to.to_owned(), relation.to_owned()))
            || !seen_dependency_pairs.insert((from.to_owned(), to.to_owned()))
        {
            errors.push(format!("duplicate dependency edge {from} -> {to}"));
            continue;
        }
        if from == to {
            errors.push(format!("self-cycle at {from}"));
        }
        if adjacency
            .get_mut(from)
            .is_some_and(|targets| targets.insert(to.to_owned()))
        {
            if let Some(degree) = indegree.get_mut(to) {
                *degree += 1;
            }
        }
    }

    let mut ready = VecDeque::<String>::new();
    for (id, degree) in &indegree {
        if *degree == 0 {
            ready.push_back(id.clone());
        }
    }
    let mut visited = 0;
    while let Some(id) = ready.pop_front() {
        visited += 1;
        if let Some(targets) = adjacency.get(&id) {
            for target in targets {
                if let Some(degree) = indegree.get_mut(target) {
                    *degree = degree.saturating_sub(1);
                    if *degree == 0 {
                        ready.push_back(target.clone());
                    }
                }
            }
        }
    }
    if visited != by_id.len() {
        errors.push("dependency graph contains a cycle".to_owned());
    }

    let verification_id = string_at(graph, "post_repair_verification_task_id");
    match verification_id.and_then(|id| by_id.get(id).map(|node| (id, *node))) {
        Some((id, node)) => {
            if string_at(node, "step_kind") != Some("post_repair_verification") {
                errors.push(format!("{id} is not a post-repair verification node"));
            }
            if adjacency.get(id).is_some_and(|targets| !targets.is_empty()) {
                errors.push(format!("post-repair verification {id} must be terminal"));
            }
            for start in by_id.keys() {
                let mut queue = VecDeque::from([start.clone()]);
                let mut seen = BTreeSet::new();
                let mut reaches_postcheck = false;
                while let Some(current) = queue.pop_front() {
                    if !seen.insert(current.clone()) {
                        continue;
                    }
                    if current == id {
                        reaches_postcheck = true;
                        break;
                    }
                    if let Some(targets) = adjacency.get(&current) {
                        for target in targets {
                            queue.push_back(target.clone());
                        }
                    }
                }
                if !reaches_postcheck {
                    errors.push(format!(
                        "task {start} has no path to post-repair verification {id}"
                    ));
                }
            }
        }
        None => errors
            .push("post_repair_verification_task_id must reference a declared node".to_owned()),
    }

    let isolation_ids: Vec<String> = by_id
        .iter()
        .filter(|(_, node)| string_at(node, "step_kind") == Some("safe_isolation"))
        .map(|(id, _)| id.clone())
        .collect();
    if isolation_ids.is_empty() {
        errors.push("graph has no safe_isolation step".to_owned());
    }
    for id in &isolation_ids {
        if let Some(node) = by_id.get(id) {
            let has_hard_stop = node
                .get("safety_preconditions")
                .and_then(Value::as_array)
                .is_some_and(|conditions| {
                    conditions.iter().any(|condition| {
                        string_at(condition, "criticality") == Some("hard_stop")
                            && condition
                                .get("must_hold_before_task")
                                .and_then(Value::as_bool)
                                == Some(true)
                    })
                });
            if !has_hard_stop {
                errors.push(format!(
                    "safe-isolation step {id} lacks a mandatory hard-stop precondition"
                ));
            }
        }
    }
    let mut downstream_of_isolation = BTreeSet::<String>::new();
    let mut queue: VecDeque<String> = isolation_ids.iter().cloned().collect();
    while let Some(id) = queue.pop_front() {
        if !downstream_of_isolation.insert(id.clone()) {
            continue;
        }
        if let Some(targets) = adjacency.get(&id) {
            for target in targets {
                queue.push_back(target.clone());
            }
        }
    }
    for (id, node) in &by_id {
        let step_kind = string_at(node, "step_kind").unwrap_or("");
        let explicitly_requires =
            node.get("requires_isolation").and_then(Value::as_bool) == Some(true);
        let implicitly_requires =
            ["access", "removal", "repair_replace", "reassembly"].contains(&step_kind);
        let safety_preconditions = node.get("safety_preconditions").and_then(Value::as_array);
        if safety_preconditions.is_some_and(|conditions| {
            conditions.iter().any(|condition| {
                string_at(condition, "criticality") == Some("hard_stop")
                    && condition
                        .get("must_hold_before_task")
                        .and_then(Value::as_bool)
                        != Some(true)
            })
        }) {
            errors.push(format!(
                "{id} declares a hard-stop precondition that is not required before the task"
            ));
        }
        if explicitly_requires || implicitly_requires {
            if !downstream_of_isolation.contains(id) {
                errors.push(format!(
                    "{id} requires isolation but is not downstream of a safe-isolation step"
                ));
            }
            let has_hard_stop = safety_preconditions.is_some_and(|conditions| {
                conditions.iter().any(|condition| {
                    string_at(condition, "criticality") == Some("hard_stop")
                        && condition
                            .get("must_hold_before_task")
                            .and_then(Value::as_bool)
                            == Some(true)
                })
            });
            if !has_hard_stop {
                errors.push(format!(
                    "{id} requires isolation but lacks a mandatory hard-stop precondition"
                ));
            }
        }
    }
    errors
}

pub(crate) fn validate_fixture_corpus() -> Result<usize, Vec<String>> {
    let schema: Value = serde_json::from_str(include_str!(
        "../../../schemas/transport/repair_task_graph_v0_1.schema.json"
    ))
    .map_err(|error| vec![format!("repair graph schema JSON parse error: {error}")])?;
    let document: Value = serde_json::from_str(include_str!(
        "../../../fixtures/personal_vehicle/repair_task_graph_fixtures_v0_1.json"
    ))
    .map_err(|error| vec![format!("repair graph fixtures JSON parse error: {error}")])?;
    if !draft202012::meta::is_valid(&schema) {
        return Err(vec![
            "repair graph schema failed JSON Schema Draft 2020-12 meta-validation".to_owned(),
        ]);
    }
    let validator = draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
        .map_err(|error| vec![format!("repair graph schema compilation failed: {error}")])?;
    if string_at(&document, "corpus_version") != Some("repair-task-graph-fixtures/0.1")
        || string_at(&document, "status")
            != Some("synthetic-contract-fixtures-not-repair-instructions")
    {
        return Err(vec![
            "repair graph fixture corpus identity/status mismatch".to_owned(),
        ]);
    }
    let fixtures = document
        .get("fixtures")
        .and_then(Value::as_array)
        .ok_or_else(|| vec!["repair graph fixtures must be an array".to_owned()])?;
    if fixtures.len() != REQUIRED_IDS.len() {
        return Err(vec![format!(
            "expected {} repair fixtures, found {}",
            REQUIRED_IDS.len(),
            fixtures.len()
        )]);
    }
    let mut errors = Vec::new();
    let mut seen = BTreeSet::new();
    for (index, fixture) in fixtures.iter().enumerate() {
        let at = format!("repair fixtures[{index}]");
        let Some(id) = string_at(fixture, "id") else {
            errors.push(format!("{at}: missing ID"));
            continue;
        };
        if !seen.insert(id.to_owned()) {
            errors.push(format!("{at}: duplicate ID {id}"));
        }
        if !REQUIRED_IDS.contains(&id) {
            errors.push(format!("{at}: unregistered ID {id}"));
        }
        let expectation = string_at(fixture, "expectation").unwrap_or("");
        let Some(graph) = fixture.get("input") else {
            errors.push(format!("{at}: missing graph input"));
            continue;
        };
        let schema_errors: Vec<String> = validator
            .iter_errors(graph)
            .map(|e| e.to_string())
            .collect();
        let semantic_errors = graph_semantic_errors(graph);
        match expectation {
            "accept" => {
                if !schema_errors.is_empty() {
                    errors.push(format!("{id}: valid graph failed schema"));
                }
                if !semantic_errors.is_empty() {
                    errors.push(format!(
                        "{id}: valid graph failed semantic checks: {}",
                        semantic_errors.join("; ")
                    ));
                }
            }
            "reject_schema" => {
                if schema_errors.is_empty() {
                    errors.push(format!("{id}: invalid graph passed schema"));
                }
            }
            "reject_semantics" => {
                if !schema_errors.is_empty() {
                    errors.push(format!("{id}: semantic negative control also fails schema"));
                } else if semantic_errors.is_empty() {
                    errors.push(format!("{id}: semantic negative control passed"));
                }
            }
            _ => errors.push(format!("{id}: unknown expectation {expectation}")),
        }
    }
    for id in REQUIRED_IDS {
        if !seen.contains(id) {
            errors.push(format!("missing repair graph fixture {id}"));
        }
    }
    let claim_ceiling = document.get("claim_ceiling").and_then(Value::as_array);
    if !claim_ceiling.is_some_and(|items| {
        items
            .iter()
            .filter_map(Value::as_str)
            .any(|s| s.contains("workshop measurements"))
            && items
                .iter()
                .filter_map(Value::as_str)
                .any(|s| s.contains("approved or safe procedure for a real vehicle"))
    }) {
        errors.push("repair graph fixture claim ceiling has been weakened".to_owned());
    }
    if errors.is_empty() {
        Ok(fixtures.len())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixtures() -> Value {
        serde_json::from_str(include_str!(
            "../../../fixtures/personal_vehicle/repair_task_graph_fixtures_v0_1.json"
        ))
        .expect("repair graph fixtures JSON")
    }

    #[test]
    fn frozen_repair_graph_corpus_validates() {
        validate_fixture_corpus().expect("repair task graph schema and fixtures must validate");
    }

    #[test]
    fn cycle_negative_control_cannot_be_relabelled_as_valid() {
        let mut document = fixtures();
        let item = document["fixtures"]
            .as_array_mut()
            .expect("fixture array")
            .iter_mut()
            .find(|item| item["id"] == "RTG03_CYCLIC_GRAPH")
            .expect("cycle fixture");
        item["expectation"] = Value::String("accept".to_owned());
        let schema: Value = serde_json::from_str(include_str!(
            "../../../schemas/transport/repair_task_graph_v0_1.schema.json"
        ))
        .expect("schema");
        let validator = draft202012::options()
            .should_validate_formats(true)
            .build(&schema)
            .expect("schema validator");
        let graph = &item["input"];
        assert!(validator.is_valid(graph));
        assert!(!graph_semantic_errors(graph).is_empty());
    }

    #[test]
    fn missing_safety_isolation_is_a_semantic_failure() {
        let document = fixtures();
        let graph = document["fixtures"]
            .as_array()
            .expect("fixtures")
            .iter()
            .find(|item| item["id"] == "RTG07_MISSING_SAFETY_ISOLATION")
            .expect("safety fixture");
        let schema: Value = serde_json::from_str(include_str!(
            "../../../schemas/transport/repair_task_graph_v0_1.schema.json"
        ))
        .expect("schema");
        let validator = draft202012::options()
            .should_validate_formats(true)
            .build(&schema)
            .expect("schema validator");
        assert!(validator.is_valid(&graph["input"]));
        assert!(!graph_semantic_errors(&graph["input"]).is_empty());
    }

    #[test]
    fn one_timing_observation_cannot_be_called_measured() {
        let document = fixtures();
        let graph = document["fixtures"]
            .as_array()
            .expect("fixtures")
            .iter()
            .find(|item| item["id"] == "RTG10_SINGLE_SAMPLE_PROMOTED_TO_OBSERVED")
            .expect("sample fixture");
        let schema: Value = serde_json::from_str(include_str!(
            "../../../schemas/transport/repair_task_graph_v0_1.schema.json"
        ))
        .expect("schema");
        let validator = draft202012::options()
            .should_validate_formats(true)
            .build(&schema)
            .expect("schema validator");
        assert!(!validator.is_valid(&graph["input"]));
    }
}
