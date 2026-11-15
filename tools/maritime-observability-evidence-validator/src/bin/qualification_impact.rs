// SPDX-License-Identifier: AGPL-3.0-or-later
use maritime_observability_evidence_validator::verify_sha256;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    env, fs,
    process::ExitCode,
};

const VERSION: &str = "0.1.0";
#[cfg(test)]
const FIXTURE: &str = include_str!(
    "../../../../benchmarks/maritime-observability/qualification-dependency-manifest.v0.1.json"
);

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_git_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn keys(value: &Value, required: &[&str], path: &str, violations: &mut Vec<Value>) -> bool {
    let Some(object) = value.as_object() else {
        violations.push(json!({
            "code": "EXPECTED_OBJECT",
            "path": path,
            "detail": "expected JSON object"
        }));
        return false;
    };

    let allowed: BTreeSet<&str> = required.iter().copied().collect();
    for key in required {
        if !object.contains_key(*key) {
            violations.push(json!({
                "code": "MISSING_KEY",
                "path": format!("{path}.{key}"),
                "detail": "required key is missing"
            }));
        }
    }
    for key in object.keys() {
        if !allowed.contains(key.as_str()) {
            violations.push(json!({
                "code": "UNKNOWN_KEY",
                "path": format!("{path}.{key}"),
                "detail": "unknown key is rejected"
            }));
        }
    }
    true
}

fn hash_field(value: &Value, key: &str, path: &str, violations: &mut Vec<Value>) {
    if !text(value, key).is_some_and(is_sha256) {
        violations.push(json!({
            "code": "INVALID_SHA256",
            "path": format!("{path}.{key}"),
            "detail": "expected lowercase SHA-256 digest"
        }));
    }
}

fn validate_boundary(value: &Value, violations: &mut Vec<Value>) {
    let path = "$.authority_boundary";
    keys(
        value,
        &[
            "read_only",
            "actuation",
            "maintenance_authorization",
            "mode_transition_authorization",
            "restriction_clearance",
        ],
        path,
        violations,
    );
    if value["read_only"].as_bool() != Some(true) {
        violations.push(json!({
            "code": "READ_ONLY_REQUIRED",
            "path": "$.authority_boundary.read_only",
            "detail": "must be true"
        }));
    }
    for field in [
        "actuation",
        "maintenance_authorization",
        "mode_transition_authorization",
        "restriction_clearance",
    ] {
        if value[field].as_bool() != Some(false) {
            violations.push(json!({
                "code": "AUTHORITY_BOUNDARY_VIOLATION",
                "path": format!("$.authority_boundary.{field}"),
                "detail": "must be false"
            }));
        }
    }
}

fn validate(manifest: &Value) -> Result<Value, Vec<Value>> {
    let mut violations = Vec::new();
    keys(
        manifest,
        &[
            "schema_version",
            "change_set_id",
            "generated_at",
            "evidence_class",
            "asset_scope",
            "nodes",
            "dependency_edges",
            "changes",
            "authority_boundary",
            "claim_ceiling",
        ],
        "$",
        &mut violations,
    );

    if text(manifest, "schema_version") != Some(VERSION) {
        violations.push(json!({
            "code": "SCHEMA_VERSION",
            "path": "$.schema_version",
            "detail": "expected 0.1.0"
        }));
    }
    if text(manifest, "evidence_class") != Some("SYNTHETIC_KNOWN_TRUTH") {
        violations.push(json!({
            "code": "EVIDENCE_CLASS",
            "path": "$.evidence_class",
            "detail": "this evaluator accepts only explicitly synthetic known-truth fixtures"
        }));
    }
    for field in ["change_set_id", "generated_at", "claim_ceiling"] {
        if text(manifest, field).is_none() {
            violations.push(json!({
                "code": "REQUIRED_TEXT",
                "path": format!("$.{field}"),
                "detail": "non-empty string required"
            }));
        }
    }

    let scope = &manifest["asset_scope"];
    keys(
        scope,
        &["asset_id", "configuration_revision", "configuration_sha256"],
        "$.asset_scope",
        &mut violations,
    );
    hash_field(
        scope,
        "configuration_sha256",
        "$.asset_scope",
        &mut violations,
    );

    let node_values = manifest["nodes"].as_array().cloned().unwrap_or_default();
    if node_values.is_empty() || node_values.len() > 10000 {
        violations.push(json!({
            "code": "NODE_COUNT",
            "path": "$.nodes",
            "detail": "node count must be between 1 and 10000"
        }));
    }

    let mut nodes: BTreeMap<String, &Value> = BTreeMap::new();
    for (index, node) in node_values.iter().enumerate() {
        let path = format!("$.nodes[{index}]");
        keys(
            node,
            &[
                "node_id",
                "node_kind",
                "sha256",
                "scope",
                "qualification_state",
            ],
            &path,
            &mut violations,
        );

        let Some(id) = text(node, "node_id") else {
            violations.push(json!({
                "code": "NODE_ID_REQUIRED",
                "path": path,
                "detail": "node ID is required"
            }));
            continue;
        };
        if nodes.insert(id.to_owned(), node).is_some() {
            violations.push(json!({
                "code": "DUPLICATE_NODE_ID",
                "path": format!("$.nodes[{index}].node_id"),
                "detail": "node IDs must be unique"
            }));
        }

        if !matches!(text(node, "node_kind"), Some("evidence" | "qualification")) {
            violations.push(json!({
                "code": "NODE_KIND_INVALID",
                "path": format!("$.nodes[{index}].node_kind"),
                "detail": "expected evidence or qualification"
            }));
        }
        if !matches!(
            text(node, "qualification_state"),
            Some("not_applicable" | "qualified" | "expired" | "revoked" | "not_evaluated")
        ) {
            violations.push(json!({
                "code": "QUALIFICATION_STATE_INVALID",
                "path": format!("$.nodes[{index}].qualification_state"),
                "detail": "unsupported qualification state"
            }));
        }
        if text(node, "node_kind") == Some("evidence")
            && text(node, "qualification_state") != Some("not_applicable")
        {
            violations.push(json!({
                "code": "EVIDENCE_NODE_STATE_INVALID",
                "path": format!("$.nodes[{index}].qualification_state"),
                "detail": "evidence nodes must use not_applicable"
            }));
        }
        if text(node, "node_kind") == Some("qualification")
            && text(node, "qualification_state") == Some("not_applicable")
        {
            violations.push(json!({
                "code": "QUALIFICATION_NODE_STATE_INVALID",
                "path": format!("$.nodes[{index}].qualification_state"),
                "detail": "qualification nodes require an actual qualification state"
            }));
        }
        hash_field(node, "sha256", &path, &mut violations);
    }

    let edge_values = manifest["dependency_edges"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut edge_set = BTreeSet::new();
    let mut dependents_by_prerequisite: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut dependency_type_by_edge: BTreeMap<(String, String), String> = BTreeMap::new();
    let mut adjacency: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for (index, edge) in edge_values.iter().enumerate() {
        let path = format!("$.dependency_edges[{index}]");
        keys(
            edge,
            &["dependent_id", "prerequisite_id", "dependency_type"],
            &path,
            &mut violations,
        );
        let Some(dependent) = text(edge, "dependent_id") else {
            violations.push(json!({
                "code": "EDGE_DEPENDENT_MISSING",
                "path": format!("{path}.dependent_id"),
                "detail": "dependent node ID required"
            }));
            continue;
        };
        let Some(prerequisite) = text(edge, "prerequisite_id") else {
            violations.push(json!({
                "code": "EDGE_PREREQUISITE_MISSING",
                "path": format!("{path}.prerequisite_id"),
                "detail": "prerequisite node ID required"
            }));
            continue;
        };
        if !matches!(
            text(edge, "dependency_type"),
            Some("requires" | "uses" | "calibrated_against" | "qualified_against" | "derived_from")
        ) {
            violations.push(json!({
                "code": "DEPENDENCY_TYPE_INVALID",
                "path": format!("{path}.dependency_type"),
                "detail": "unsupported dependency type"
            }));
        }
        if !nodes.contains_key(dependent) || !nodes.contains_key(prerequisite) {
            violations.push(json!({
                "code": "UNKNOWN_DEPENDENCY_NODE",
                "path": path,
                "detail": "both edge endpoints must reference declared nodes"
            }));
            continue;
        }
        if dependent == prerequisite {
            violations.push(json!({
                "code": "SELF_DEPENDENCY",
                "path": path,
                "detail": "a node cannot depend on itself"
            }));
        }
        if !edge_set.insert((dependent.to_owned(), prerequisite.to_owned())) {
            violations.push(json!({
                "code": "DUPLICATE_DEPENDENCY_EDGE",
                "path": path,
                "detail": "duplicate dependent/prerequisite edge"
            }));
        }
        adjacency
            .entry(dependent.to_owned())
            .or_default()
            .insert(prerequisite.to_owned());
        dependents_by_prerequisite
            .entry(prerequisite.to_owned())
            .or_default()
            .insert(dependent.to_owned());
        dependency_type_by_edge.insert(
            (dependent.to_owned(), prerequisite.to_owned()),
            text(edge, "dependency_type").unwrap_or("unknown").to_owned(),
        );
    }

    // Kahn topological pass: a cycle makes qualification propagation ambiguous.
    let mut indegree: BTreeMap<String, usize> = nodes.keys().map(|id| (id.clone(), 0)).collect();
    for prerequisites in adjacency.values() {
        for prerequisite in prerequisites {
            *indegree.entry(prerequisite.clone()).or_default() += 1;
        }
    }
    let mut ready: VecDeque<String> = indegree
        .iter()
        .filter_map(|(id, degree)| (*degree == 0).then_some(id.clone()))
        .collect();
    let mut visited = 0usize;
    while let Some(node) = ready.pop_front() {
        visited += 1;
        if let Some(prerequisites) = adjacency.get(&node) {
            for prerequisite in prerequisites {
                if let Some(degree) = indegree.get_mut(prerequisite) {
                    *degree -= 1;
                    if *degree == 0 {
                        ready.push_back(prerequisite.clone());
                    }
                }
            }
        }
    }
    if visited != nodes.len() {
        violations.push(json!({
            "code": "DEPENDENCY_CYCLE",
            "path": "$.dependency_edges",
            "detail": "qualification dependency graph must be acyclic"
        }));
    }

    let change_values = manifest["changes"].as_array().cloned().unwrap_or_default();
    if change_values.is_empty() || change_values.len() > 1000 {
        violations.push(json!({
            "code": "CHANGE_COUNT",
            "path": "$.changes",
            "detail": "change count must be between 1 and 1000"
        }));
    }

    let mut change_ids = BTreeSet::new();
    let mut changed_nodes = BTreeSet::new();
    let mut change_records = Vec::new();

    for (index, change) in change_values.iter().enumerate() {
        let path = format!("$.changes[{index}]");
        keys(
            change,
            &[
                "change_id",
                "node_id",
                "previous_sha256",
                "new_sha256",
                "change_kind",
                "effective_at",
            ],
            &path,
            &mut violations,
        );
        let Some(change_id) = text(change, "change_id") else {
            violations.push(json!({
                "code": "CHANGE_ID_REQUIRED",
                "path": format!("{path}.change_id"),
                "detail": "change ID required"
            }));
            continue;
        };
        if !change_ids.insert(change_id.to_owned()) {
            violations.push(json!({
                "code": "DUPLICATE_CHANGE_ID",
                "path": format!("{path}.change_id"),
                "detail": "change IDs must be unique"
            }));
        }
        let Some(node_id) = text(change, "node_id") else {
            violations.push(json!({
                "code": "CHANGE_NODE_REQUIRED",
                "path": format!("{path}.node_id"),
                "detail": "changed node ID required"
            }));
            continue;
        };
        let Some(node) = nodes.get(node_id) else {
            violations.push(json!({
                "code": "UNKNOWN_CHANGE_NODE",
                "path": format!("{path}.node_id"),
                "detail": "changed node must exist in the manifest"
            }));
            continue;
        };
        if text(node, "node_kind") != Some("evidence") {
            violations.push(json!({
                "code": "CHANGE_TARGET_NOT_EVIDENCE",
                "path": format!("{path}.node_id"),
                "detail": "this change set accepts evidence-node changes only"
            }));
        }
        let old = text(change, "previous_sha256").unwrap_or("");
        let new = text(change, "new_sha256").unwrap_or("");
        if !is_sha256(old) || !is_sha256(new) {
            violations.push(json!({
                "code": "CHANGE_DIGEST_INVALID",
                "path": path,
                "detail": "previous and new digests must be lowercase SHA-256"
            }));
        } else if old == new {
            violations.push(json!({
                "code": "NO_OP_CHANGE",
                "path": path,
                "detail": "new digest must differ from previous digest"
            }));
        }
        if old != text(node, "sha256").unwrap_or("") {
            violations.push(json!({
                "code": "STALE_CHANGE_PRECONDITION",
                "path": format!("{path}.previous_sha256"),
                "detail": "previous digest does not match the manifest snapshot; reconcile before applying"
            }));
        }
        if !matches!(
            text(change, "change_kind"),
            Some(
                "ASSET_RECONFIGURATION"
                    | "SENSOR_REPLACEMENT"
                    | "CALIBRATION_CHANGE"
                    | "TIMEBASE_REQUALIFICATION"
                    | "MODEL_UPDATE"
                    | "SOFTWARE_UPDATE"
                    | "EVIDENCE_REVISION"
            )
        ) {
            violations.push(json!({
                "code": "CHANGE_KIND_INVALID",
                "path": format!("{path}.change_kind"),
                "detail": "unsupported change kind"
            }));
        }
        if text(change, "effective_at").is_none() {
            violations.push(json!({
                "code": "CHANGE_EFFECTIVE_TIME_REQUIRED",
                "path": format!("{path}.effective_at"),
                "detail": "effective time required"
            }));
        }
        if !changed_nodes.insert(node_id.to_owned()) {
            violations.push(json!({
                "code": "DUPLICATE_CHANGED_NODE",
                "path": format!("{path}.node_id"),
                "detail": "one change event per node is required in a change set"
            }));
        }
        change_records.push(json!({
            "change_id": change_id,
            "node_id": node_id,
            "change_kind": text(change, "change_kind").unwrap_or("unknown"),
            "previous_sha256": old,
            "new_sha256": new,
            "effective_at": text(change, "effective_at").unwrap_or("")
        }));
    }

    validate_boundary(&manifest["authority_boundary"], &mut violations);
    if !violations.is_empty() {
        return Err(violations);
    }

    // Enumerate every simple changed-evidence -> qualification path in the
    // validated DAG. This preserves converging causes instead of only counting
    // the reachable qualification once. A hard cap makes path explosion fail closed.
    const MAX_IMPACT_PATHS: usize = 100_000;
    const MAX_IMPACT_TRAVERSAL_STATES: usize = 250_000;
    const MAX_IMPACT_PATH_DEPTH: usize = 512;
    let mut impacted_nodes = BTreeSet::new();
    let mut traversed_edges = BTreeSet::new();
    let mut impact_paths: Vec<Value> = Vec::new();
    let mut impact_traversal_state_count = 0usize;

    for change in &change_records {
        let source_id = change["node_id"].as_str().expect("validated changed node");
        let change_id = change["change_id"].as_str().expect("validated change ID");
        let mut stack: Vec<(String, Vec<String>, Vec<Value>)> = vec![(
            source_id.to_owned(),
            vec![source_id.to_owned()],
            Vec::new(),
        )];

        while let Some((current, node_path, edge_path)) = stack.pop() {
            impact_traversal_state_count += 1;
            if impact_traversal_state_count > MAX_IMPACT_TRAVERSAL_STATES {
                violations.push(json!({
                    "code": "IMPACT_TRAVERSAL_STATE_LIMIT",
                    "path": "$.dependency_edges",
                    "detail": format!("more than {MAX_IMPACT_TRAVERSAL_STATES} path states; qualify a narrower dependency scope")
                }));
                return Err(violations);
            }
            if let Some(dependents) = dependents_by_prerequisite.get(&current) {
                for dependent in dependents.iter().rev() {
                    if node_path.iter().any(|visited| visited == dependent) {
                        continue;
                    }
                    let edge_key = (dependent.clone(), current.clone());
                    let dependency_type = dependency_type_by_edge
                        .get(&edge_key)
                        .map(String::as_str)
                        .unwrap_or("unknown");
                    traversed_edges.insert(edge_key);

                    let mut next_nodes = node_path.clone();
                    next_nodes.push(dependent.clone());
                    if next_nodes.len() > MAX_IMPACT_PATH_DEPTH + 1 {
                        violations.push(json!({
                            "code": "IMPACT_PATH_DEPTH_LIMIT",
                            "path": "$.dependency_edges",
                            "detail": format!("impact path exceeds {MAX_IMPACT_PATH_DEPTH} edges")
                        }));
                        return Err(violations);
                    }
                    let mut next_edges = edge_path.clone();
                    next_edges.push(json!({
                        "from": current,
                        "to": dependent,
                        "dependency_type": dependency_type
                    }));

                    impacted_nodes.insert(dependent.clone());
                    if nodes
                        .get(dependent)
                        .is_some_and(|node| text(node, "node_kind") == Some("qualification"))
                    {
                        if impact_paths.len() >= MAX_IMPACT_PATHS {
                            violations.push(json!({
                                "code": "IMPACT_PATH_LIMIT",
                                "path": "$.dependency_edges",
                                "detail": "more than 100000 impact paths; qualify a narrower dependency scope"
                            }));
                            return Err(violations);
                        }
                        impact_paths.push(json!({
                            "change_id": change_id,
                            "changed_node_id": source_id,
                            "qualification_id": dependent,
                            "node_path": next_nodes,
                            "edges": next_edges,
                            "edge_count": next_edges.len()
                        }));
                    }
                    stack.push((dependent.clone(), next_nodes, next_edges));
                }
            }
        }
    }

    let max_impact_path_depth = impact_paths
        .iter()
        .filter_map(|path| path["edge_count"].as_u64())
        .max()
        .unwrap_or(0);

    impact_paths.sort_by(|a, b| {
        (
            a["change_id"].as_str().unwrap_or(""),
            a["qualification_id"].as_str().unwrap_or(""),
            a["edge_count"].as_u64().unwrap_or(0),
            a["node_path"].to_string(),
        )
            .cmp(&(
                b["change_id"].as_str().unwrap_or(""),
                b["qualification_id"].as_str().unwrap_or(""),
                b["edge_count"].as_u64().unwrap_or(0),
                b["node_path"].to_string(),
            ))
    });

    let mut impacted_qualifications = Vec::new();
    let mut unaffected_qualifications = Vec::new();
    for (id, node) in &nodes {
        if text(node, "node_kind") != Some("qualification") {
            continue;
        }
        let prior_state = text(node, "qualification_state").unwrap_or("not_evaluated");
        let qualification_paths: Vec<&Value> = impact_paths
            .iter()
            .filter(|path| path["qualification_id"].as_str() == Some(id.as_str()))
            .collect();
        let triggering_change_ids: BTreeSet<&str> = qualification_paths
            .iter()
            .filter_map(|path| path["change_id"].as_str())
            .collect();

        if impacted_nodes.contains(id) {
            let disposition = if prior_state == "qualified" {
                "REQUALIFICATION_REQUIRED"
            } else {
                "ALREADY_NOT_QUALIFIED"
            };
            impacted_qualifications.push(json!({
                "qualification_id": id,
                "scope": text(node, "scope").unwrap_or("unknown"),
                "prior_state": prior_state,
                "disposition": disposition,
                "impact_path_count": qualification_paths.len(),
                "triggering_change_ids": triggering_change_ids
            }));
        } else {
            unaffected_qualifications.push(json!({
                "qualification_id": id,
                "scope": text(node, "scope").unwrap_or("unknown"),
                "prior_state": prior_state,
                "disposition": "UNCHANGED_BY_THIS_CHANGE_SET",
                "impact_path_count": 0,
                "triggering_change_ids": []
            }));
        }
    }

    let required = impacted_qualifications
        .iter()
        .filter(|item| item["disposition"] == "REQUALIFICATION_REQUIRED")
        .count();
    let already_not = impacted_qualifications.len() - required;

    Ok(json!({
        "result": "pass",
        "impact_evaluator_version": VERSION,
        "change_set_id": text(manifest, "change_set_id").unwrap_or(""),
        "asset_id": text(&manifest["asset_scope"], "asset_id").unwrap_or(""),
        "configuration_revision": text(&manifest["asset_scope"], "configuration_revision").unwrap_or(""),
        "change_count": change_records.len(),
        "changed_node_ids": changed_nodes.into_iter().collect::<Vec<_>>(),
        "changed_evidence": change_records,
        "impacted_qualification_count": impacted_qualifications.len(),
        "requalification_required_count": required,
        "already_not_qualified_count": already_not,
        "unaffected_qualification_count": unaffected_qualifications.len(),
        "impact_traversal_node_count": impacted_nodes.len(),
        "impact_traversal_edge_count": traversed_edges.len(),
        "impact_traversal_state_count": impact_traversal_state_count,
        "impact_path_count": impact_paths.len(),
        "max_impact_path_depth": max_impact_path_depth,
        "impact_paths": impact_paths,
        "impacted_qualifications": impacted_qualifications,
        "unaffected_qualifications": unaffected_qualifications,
        "mutation_performed": false,
        "authorization_granted": false,
        "claim_ceiling": text(manifest, "claim_ceiling").unwrap_or("")
    }))
}

fn run(path: &str, source_commit: &str, manifest_sha256: &str) -> Result<Value, String> {
    if !is_git_sha(source_commit) || !is_sha256(manifest_sha256) {
        return Err("invalid source commit or manifest SHA-256".to_owned());
    }
    let raw = fs::read_to_string(path).map_err(|error| format!("read change set: {error}"))?;
    verify_sha256(raw.as_bytes(), manifest_sha256)?;
    let manifest: Value =
        serde_json::from_str(&raw).map_err(|error| format!("parse change set: {error}"))?;
    match validate(&manifest) {
        Ok(mut receipt) => {
            receipt["source_commit"] = json!(source_commit);
            receipt["change_set_sha256"] = json!(manifest_sha256);
            Ok(receipt)
        }
        Err(violations) => Err(serde_json::to_string(&json!({ "violations": violations }))
            .unwrap_or_else(|_| "qualification impact validation failed".to_owned())),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 6 || args[2] != "--source-commit" || args[4] != "--change-set-sha256" {
        eprintln!(
            "usage: qualification-impact <change-set.json> --source-commit <40-hex> --change-set-sha256 <64-hex>"
        );
        return ExitCode::from(2);
    }
    match run(&args[1], &args[3], &args[5]) {
        Ok(receipt) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&receipt)
                    .unwrap_or_else(|_| "{\"result\":\"fail\"}".to_owned())
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("qualification impact analysis failed closed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        serde_json::from_str(FIXTURE).expect("qualification fixture parses")
    }

    fn validate_fixture() -> Value {
        validate(&fixture()).expect("fixture is valid")
    }

    fn has_error(errors: &[Value], code: &str) -> bool {
        errors.iter().any(|error| error["code"] == code)
    }

    #[test]
    fn fixture_has_expected_transitive_closure() {
        let report = validate_fixture();
        assert_eq!(report["change_count"], 2);
        assert_eq!(report["impacted_qualification_count"], 5);
        assert_eq!(report["requalification_required_count"], 5);
        assert_eq!(report["unaffected_qualification_count"], 2);
        let impacted: BTreeSet<&str> = report["impacted_qualifications"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|item| item["qualification_id"].as_str())
            .collect();
        assert_eq!(
            impacted,
            BTreeSet::from([
                "Q-CONTROL-RESPONSE",
                "Q-ENGINE-HEALTH",
                "Q-OBSERVABILITY",
                "Q-RUL",
                "Q-TIMEBASE"
            ])
        );
    }

    #[test]
    fn every_impacted_qualification_has_a_source_bound_dependency_path() {
        let report = validate_fixture();
        let paths = report["impact_paths"].as_array().unwrap();
        assert_eq!(report["impact_path_count"], paths.len());
        for qualification in report["impacted_qualifications"].as_array().unwrap() {
            let id = qualification["qualification_id"].as_str().unwrap();
            let matching: Vec<&Value> = paths
                .iter()
                .filter(|path| path["qualification_id"] == id)
                .collect();
            assert_eq!(qualification["impact_path_count"], matching.len());
            assert!(!matching.is_empty(), "{id} must have an explanation path");
            for path in matching {
                let nodes = path["node_path"].as_array().unwrap();
                let edges = path["edges"].as_array().unwrap();
                assert_eq!(nodes.len(), edges.len() + 1);
                assert_eq!(nodes.first().unwrap(), &path["changed_node_id"]);
                assert_eq!(nodes.last().unwrap(), &path["qualification_id"]);
                for (index, edge) in edges.iter().enumerate() {
                    assert_eq!(edge["from"], nodes[index]);
                    assert_eq!(edge["to"], nodes[index + 1]);
                    assert!(edge["dependency_type"].as_str().is_some());
                }
            }
        }
    }

    #[test]
    fn converging_changes_preserve_multiple_explanations() {
        let report = validate_fixture();
        let find = |id: &str| {
            report["impacted_qualifications"]
                .as_array()
                .unwrap()
                .iter()
                .find(|item| item["qualification_id"] == id)
                .unwrap()
        };
        assert_eq!(find("Q-CONTROL-RESPONSE")["impact_path_count"], 3);
        assert_eq!(find("Q-ENGINE-HEALTH")["impact_path_count"], 3);
        assert_eq!(find("Q-RUL")["impact_path_count"], 3);
        let triggering: BTreeSet<&str> = find("Q-CONTROL-RESPONSE")["triggering_change_ids"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect();
        assert_eq!(triggering, BTreeSet::from([
            "CHG-SENSOR-TOPOLOGY-001",
            "CHG-TIMEBASE-QUAL-001"
        ]));
    }

    #[test]
    fn unaffected_qualifications_have_no_impact_paths() {
        let report = validate_fixture();
        for item in report["unaffected_qualifications"].as_array().unwrap() {
            assert_eq!(item["impact_path_count"], 0);
            assert_eq!(item["triggering_change_ids"].as_array().unwrap().len(), 0);
        }
    }

    #[test]
    fn deterministic_path_order_is_stable() {
        let first = validate_fixture();
        let second = validate_fixture();
        assert_eq!(first["impact_paths"], second["impact_paths"]);
    }

    #[test]
    fn unaffected_calibration_and_auxiliary_qualification_are_preserved() {
        let report = validate_fixture();
        let unaffected: BTreeSet<&str> = report["unaffected_qualifications"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|item| item["qualification_id"].as_str())
            .collect();
        assert_eq!(
            unaffected,
            BTreeSet::from(["Q-AUX-ALARM", "Q-PRESSURE-CAL"])
        );
    }

    #[test]
    fn stale_digest_precondition_fails_closed() {
        let mut input = fixture();
        input["changes"][0]["previous_sha256"] = json!("0".repeat(64));
        let errors = validate(&input).unwrap_err();
        assert!(has_error(&errors, "STALE_CHANGE_PRECONDITION"));
    }

    #[test]
    fn unknown_change_target_fails_closed() {
        let mut input = fixture();
        input["changes"][0]["node_id"] = json!("E-UNKNOWN");
        let errors = validate(&input).unwrap_err();
        assert!(has_error(&errors, "UNKNOWN_CHANGE_NODE"));
    }

    #[test]
    fn duplicate_change_ids_and_targets_are_rejected() {
        let mut input = fixture();
        input["changes"][1]["change_id"] = input["changes"][0]["change_id"].clone();
        input["changes"][1]["node_id"] = input["changes"][0]["node_id"].clone();
        let errors = validate(&input).unwrap_err();
        assert!(has_error(&errors, "DUPLICATE_CHANGE_ID"));
        assert!(has_error(&errors, "DUPLICATE_CHANGED_NODE"));
    }

    #[test]
    fn dependency_cycle_fails_closed() {
        let mut input = fixture();
        input["dependency_edges"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "dependent_id": "E-SENSOR-TOPOLOGY",
                "prerequisite_id": "Q-OBSERVABILITY",
                "dependency_type": "requires"
            }));
        let errors = validate(&input).unwrap_err();
        assert!(has_error(&errors, "DEPENDENCY_CYCLE"));
    }

    #[test]
    fn unknown_edge_endpoint_is_rejected() {
        let mut input = fixture();
        input["dependency_edges"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "dependent_id": "Q-OBSERVABILITY",
                "prerequisite_id": "E-UNKNOWN",
                "dependency_type": "requires"
            }));
        let errors = validate(&input).unwrap_err();
        assert!(has_error(&errors, "UNKNOWN_DEPENDENCY_NODE"));
    }

    #[test]
    fn no_op_digest_change_is_rejected() {
        let mut input = fixture();
        input["changes"][0]["new_sha256"] = input["changes"][0]["previous_sha256"].clone();
        let errors = validate(&input).unwrap_err();
        assert!(has_error(&errors, "NO_OP_CHANGE"));
    }

    #[test]
    fn authority_boundary_is_non_mutating() {
        let report = validate_fixture();
        assert_eq!(report["mutation_performed"], false);
        assert_eq!(report["authorization_granted"], false);

        let mut input = fixture();
        input["authority_boundary"]["actuation"] = json!(true);
        let errors = validate(&input).unwrap_err();
        assert!(has_error(&errors, "AUTHORITY_BOUNDARY_VIOLATION"));
    }

    #[test]
    fn multiple_changed_paths_do_not_duplicate_qualification_impact() {
        let report = validate_fixture();
        let ids: Vec<&str> = report["impacted_qualifications"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|item| item["qualification_id"].as_str())
            .collect();
        assert_eq!(ids.len(), 5);
        assert_eq!(ids.iter().collect::<BTreeSet<_>>().len(), 5);
    }

    #[test]
    fn qualification_already_expired_is_not_reported_as_current() {
        let mut input = fixture();
        for node in input["nodes"].as_array_mut().unwrap() {
            if node["node_id"] == "Q-TIMEBASE" {
                node["qualification_state"] = json!("expired");
            }
        }
        let report = validate(&input).unwrap();
        let q = report["impacted_qualifications"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["qualification_id"] == "Q-TIMEBASE")
            .unwrap();
        assert_eq!(q["disposition"], "ALREADY_NOT_QUALIFIED");
    }
}
