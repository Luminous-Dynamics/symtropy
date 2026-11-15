//! Compute configuration/evidence change impact over a sail capability DAG.
//!
//! This is a read-only synthetic impact adapter. It does not issue an operational
//! restriction or mutate any qualification record; the owning qualification
//! compiler remains responsible for exact disposition of each affected scope.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    env, fs,
    process::ExitCode,
};

const PROFILE_VERSION: &str = "maritime-sail-capability-profile-v0.1";
const IMPACT_VERSION: &str = "maritime-sail-capability-impact-v0.1";
const ALLOWED_DEPENDENCIES: &[&str] = &[
    "sensor_topology", "calibration", "timebase", "source_payload",
    "diagnostic_model", "evaluator", "validation_profile",
];

#[derive(Clone, Debug)]
struct Capability {
    parents: BTreeSet<String>,
    dependencies: BTreeSet<String>,
    components: BTreeSet<String>,
}

type Graph = BTreeMap<String, Capability>;

fn exact_keys(value: &Value, allowed: &[&str], at: &str) -> Result<(), String> {
    let object = value.as_object().ok_or_else(|| format!("{at}: expected object"))?;
    let expected: BTreeSet<&str> = allowed.iter().copied().collect();
    let actual: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    let missing: Vec<_> = expected.difference(&actual).copied().collect();
    let unknown: Vec<_> = actual.difference(&expected).copied().collect();
    if !missing.is_empty() || !unknown.is_empty() {
        return Err(format!("{at}: missing keys {missing:?}; unknown keys {unknown:?}"));
    }
    Ok(())
}

fn nonempty<'a>(value: &'a Value, key: &str, at: &str) -> Result<&'a str, String> {
    value.get(key).and_then(Value::as_str).filter(|s| !s.trim().is_empty())
        .ok_or_else(|| format!("{at}.{key}: expected non-empty string"))
}

fn string_set(value: &Value, key: &str, at: &str, allow_empty: bool) -> Result<BTreeSet<String>, String> {
    let values = value.get(key).and_then(Value::as_array)
        .ok_or_else(|| format!("{at}.{key}: expected array"))?;
    if values.is_empty() && !allow_empty {
        return Err(format!("{at}.{key}: set must not be empty"));
    }
    let mut result = BTreeSet::new();
    for item in values {
        let text = item.as_str().filter(|s| !s.trim().is_empty())
            .ok_or_else(|| format!("{at}.{key}: expected non-empty strings"))?;
        if !result.insert(text.to_owned()) {
            return Err(format!("{at}.{key}: duplicate value {text:?}"));
        }
    }
    Ok(result)
}

fn valid_git_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn load_graph(profile: &Value) -> Result<Graph, String> {
    exact_keys(
        profile,
        &[
            "schema_version",
            "profile_id",
            "evidence_class",
            "as_of_utc",
            "asset_configuration",
            "capability_graph",
            "authority_boundary",
            "claim_ceiling",
        ],
        "$",
    )?;
    if nonempty(profile, "schema_version", "$")? != PROFILE_VERSION
        || nonempty(profile, "evidence_class", "$")? != "SYNTHETIC_KNOWN_TRUTH"
    {
        return Err("unsupported sail capability profile schema or evidence class".into());
    }
    nonempty(profile, "profile_id", "$")?;
    nonempty(profile, "as_of_utc", "$")?;
    nonempty(profile, "claim_ceiling", "$")?;
    let asset = profile.get("asset_configuration")
        .ok_or_else(|| "profile asset configuration is missing".to_owned())?;
    exact_keys(
        asset,
        &["asset_id", "configuration_sha256", "component_identity_policy"],
        "$.asset_configuration",
    )?;
    nonempty(asset, "asset_id", "$.asset_configuration")?;
    let config_digest = nonempty(asset, "configuration_sha256", "$.asset_configuration")?;
    if !valid_sha256(config_digest) {
        return Err("profile asset configuration digest is invalid".into());
    }
    nonempty(asset, "component_identity_policy", "$.asset_configuration")?;

    let authority = profile.get("authority_boundary")
        .ok_or_else(|| "profile authority boundary is missing".to_owned())?;
    exact_keys(
        authority,
        &[
            "read_only",
            "mutation_performed",
            "authorization_granted",
            "actuator_commands_emitted",
        ],
        "$.authority_boundary",
    )?;
    if authority.get("read_only").and_then(Value::as_bool) != Some(true)
        || authority.get("mutation_performed").and_then(Value::as_bool) != Some(false)
        || authority.get("authorization_granted").and_then(Value::as_bool) != Some(false)
        || authority.get("actuator_commands_emitted").and_then(Value::as_bool) != Some(false)
    {
        return Err("profile authority boundary is not read-only".into());
    }

    let nodes = profile.get("capability_graph").and_then(Value::as_array)
        .ok_or_else(|| "profile capability_graph must be an array".to_owned())?;
    if nodes.is_empty() || nodes.len() > 256 {
        return Err("capability graph node count must be in 1..=256".into());
    }

    let mut graph = Graph::new();
    for node in nodes {
        exact_keys(
            node,
            &[
                "capability_id",
                "depends_on",
                "required_dependency_ids",
                "qualification_case_id",
                "operating_regime",
                "configuration_component_ids",
                "claim_ceiling",
            ],
            "$.capability_graph[]",
        )?;
        let id = nonempty(node, "capability_id", "$.capability_graph[]")?.to_owned();
        if !id.starts_with("sail.") {
            return Err(format!("invalid capability ID {id:?}"));
        }
        let parents = string_set(node, "depends_on", "$.capability_graph[]", true)?;
        let dependencies = string_set(node, "required_dependency_ids", "$.capability_graph[]", false)?;
        if dependencies.iter().any(|value| !ALLOWED_DEPENDENCIES.contains(&value.as_str())) {
            return Err(format!("{id}: unknown evidence dependency class"));
        }
        let components = string_set(node, "configuration_component_ids", "$.capability_graph[]", false)?;
        let item = Capability { parents, dependencies, components };
        if graph.insert(id.clone(), item).is_some() {
            return Err(format!("duplicate capability ID {id}"));
        }
    }

    for (id, node) in &graph {
        for parent in &node.parents {
            if !graph.contains_key(parent) {
                return Err(format!("{id}: unknown prerequisite capability {parent}"));
            }
            if parent == id {
                return Err(format!("{id}: self-dependency is invalid"));
            }
        }
    }
    topological_order(&graph)?;
    Ok(graph)
}

fn topological_order(graph: &Graph) -> Result<Vec<String>, String> {
    let mut indegree: BTreeMap<String, usize> =
        graph.keys().map(|id| (id.clone(), 0)).collect();
    let mut children: BTreeMap<String, BTreeSet<String>> =
        graph.keys().map(|id| (id.clone(), BTreeSet::new())).collect();

    for (child, node) in graph {
        for parent in &node.parents {
            *indegree.get_mut(child).expect("child exists") += 1;
            children.get_mut(parent).expect("parent checked").insert(child.clone());
        }
    }
    let mut ready: BTreeSet<String> = indegree.iter()
        .filter(|(_, value)| **value == 0)
        .map(|(id, _)| id.clone()).collect();
    let mut result = Vec::with_capacity(graph.len());
    while let Some(id) = ready.pop_first() {
        result.push(id.clone());
        for child in children.get(&id).expect("node initialized") {
            let degree = indegree.get_mut(child).expect("child initialized");
            *degree -= 1;
            if *degree == 0 {
                ready.insert(child.clone());
            }
        }
    }
    if result.len() != graph.len() {
        return Err("capability dependency graph contains a cycle".into());
    }
    Ok(result)
}

fn compute_impact(
    graph: &Graph,
    event: &Value,
) -> Result<Value, String> {
    let event_id = nonempty(event, "event_id", "$.events[]")?;
    let changed_components = string_set(event, "changed_component_ids", "$.events[]", true)?;
    let changed_dependencies = string_set(event, "changed_dependency_ids", "$.events[]", true)?;
    let expected_impacted = string_set(event, "expected_impacted_capability_ids", "$.events[]", false)?;
    let expected_unaffected = string_set(event, "expected_unaffected_capability_ids", "$.events[]", true)?;

    if changed_components.is_empty() && changed_dependencies.is_empty() {
        return Err(format!("{event_id}: change event must identify at least one changed component or evidence dependency"));
    }
    if changed_dependencies.iter().any(|value| !ALLOWED_DEPENDENCIES.contains(&value.as_str())) {
        return Err(format!("{event_id}: unknown changed dependency class"));
    }

    let known_components: BTreeSet<String> = graph.values()
        .flat_map(|node| node.components.iter().cloned()).collect();
    for component in &changed_components {
        if !known_components.contains(component) {
            return Err(format!("{event_id}: unrecognized component {component}"));
        }
    }

    let known_capabilities: BTreeSet<String> = graph.keys().cloned().collect();
    if expected_impacted.iter().any(|id| !known_capabilities.contains(id))
        || expected_unaffected.iter().any(|id| !known_capabilities.contains(id))
    {
        return Err(format!("{event_id}: expected result contains an unknown capability"));
    }
    if !expected_impacted.is_disjoint(&expected_unaffected)
        || expected_impacted.union(&expected_unaffected).cloned().collect::<BTreeSet<_>>() != known_capabilities
    {
        return Err(format!("{event_id}: expected impacted/unaffected sets must be disjoint and partition every capability"));
    }

    // Directly affected capabilities have an explicit changed component or
    // evidence dependency in their own prerequisite set.
    let mut direct = BTreeSet::new();
    let mut reasons = BTreeMap::<String, Vec<String>>::new();
    for (id, node) in graph {
        let mut node_reasons = Vec::new();
        for component in &changed_components {
            if node.components.contains(component) {
                node_reasons.push(format!("component:{component}"));
            }
        }
        for dependency in &changed_dependencies {
            if node.dependencies.contains(dependency) {
                node_reasons.push(format!("evidence_dependency:{dependency}"));
            }
        }
        if !node_reasons.is_empty() {
            direct.insert(id.clone());
            reasons.insert(id.clone(), node_reasons);
        }
    }

    if direct.is_empty() {
        return Err(format!("{event_id}: change did not match any declared capability prerequisite"));
    }

    // Descendant closure: a child capability cannot remain supported when a
    // capability it explicitly depends on is withdrawn.
    let mut impacted = direct.clone();
    let mut queue: VecDeque<String> = direct.iter().cloned().collect();
    while let Some(parent) = queue.pop_front() {
        for (child_id, child) in graph {
            if child.parents.contains(&parent) && impacted.insert(child_id.clone()) {
                reasons.insert(child_id.clone(), vec![format!("prerequisite_capability:{parent}")]);
                queue.push_back(child_id.clone());
            }
        }
    }
    let unaffected: BTreeSet<String> = known_capabilities.difference(&impacted).cloned().collect();

    if impacted != expected_impacted || unaffected != expected_unaffected {
        return Err(format!(
            "{event_id}: impact closure mismatch; expected impacted={expected_impacted:?}, actual={impacted:?}, expected unaffected={expected_unaffected:?}, actual unaffected={unaffected:?}"
        ));
    }

    let transitive: BTreeSet<String> = impacted.difference(&direct).cloned().collect();

    Ok(json!({
        "event_id": event_id,
        "changed_component_ids": changed_components,
        "changed_dependency_ids": changed_dependencies,
        "directly_impacted_capability_ids": direct.clone(),
        "transitively_impacted_capability_ids": transitive,
        "impacted_capability_ids": impacted,
        "unaffected_capability_ids": unaffected,
        "impact_reasons": reasons,
        "expected_result_matched": true
    }))
}

fn evaluate(profile: &Value, document: &Value) -> Result<Value, String> {
    let graph = load_graph(profile)?;
    if nonempty(document, "schema_version", "$")? != IMPACT_VERSION
        || nonempty(document, "profile_id", "$")? != nonempty(profile, "profile_id", "$")?
        || nonempty(document, "evidence_class", "$")? != "SYNTHETIC_KNOWN_TRUTH"
        || nonempty(document, "as_of_utc", "$")? != nonempty(profile, "as_of_utc", "$")?
    {
        return Err("impact case document does not match profile identity, evidence class or as-of instant".into());
    }
    exact_keys(
        document,
        &["schema_version", "profile_id", "evidence_class", "as_of_utc", "events"],
        "$.impact_cases",
    )?;
    let events = document.get("events").and_then(Value::as_array)
        .ok_or_else(|| "impact events must be an array".to_owned())?;
    if events.is_empty() || events.len() > 10_000 {
        return Err("impact event count must be in 1..=10000".into());
    }

    let mut seen = BTreeSet::new();
    let mut results = Vec::new();
    for event in events {
        exact_keys(
            event,
            &[
                "event_id",
                "changed_component_ids",
                "changed_dependency_ids",
                "expected_impacted_capability_ids",
                "expected_unaffected_capability_ids",
            ],
            "$.events[]",
        )?;
        let event_id = nonempty(event, "event_id", "$.events[]")?.to_owned();
        if !seen.insert(event_id.clone()) {
            return Err(format!("duplicate impact event ID {event_id}"));
        }
        results.push(compute_impact(&graph, event)?);
    }

    Ok(json!({
        "result": "pass",
        "profile_id": nonempty(profile, "profile_id", "$")?,
        "capability_count": graph.len(),
        "event_count": results.len(),
        "event_results": results,
        "all_expected_impact_closures_matched": true,
        "mutation_performed": false,
        "authorization_granted": false,
        "actuator_commands_emitted": false,
        "claim_ceiling": "synthetic capability-impact closure conformance only; no physical vessel capability or operational restriction is established"
    }))
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 9
        || args[3] != "--source-commit"
        || args[5] != "--profile-sha256"
        || args[7] != "--events-sha256"
    {
        eprintln!("usage: sail_capability_impact <profile.json> <impact-cases.json> --source-commit <sha> --profile-sha256 <sha256> --events-sha256 <sha256>");
        return ExitCode::from(2);
    }
    if !valid_git_sha(&args[4]) || !valid_sha256(&args[6]) || !valid_sha256(&args[8]) {
        eprintln!("invalid source commit or input digest");
        return ExitCode::from(2);
    }

    let profile_bytes = match fs::read(&args[1]) {
        Ok(value) => value,
        Err(error) => { eprintln!("cannot read profile: {error}"); return ExitCode::from(2); }
    };
    let event_bytes = match fs::read(&args[2]) {
        Ok(value) => value,
        Err(error) => { eprintln!("cannot read impact cases: {error}"); return ExitCode::from(2); }
    };
    let profile_digest = format!("{:x}", Sha256::digest(&profile_bytes));
    let event_digest = format!("{:x}", Sha256::digest(&event_bytes));
    if profile_digest != args[6] || event_digest != args[8] {
        eprintln!("profile or impact-case digest mismatch");
        return ExitCode::from(2);
    }
    let profile: Value = match serde_json::from_slice(&profile_bytes) {
        Ok(value) => value,
        Err(error) => { eprintln!("invalid profile JSON: {error}"); return ExitCode::from(2); }
    };
    let document: Value = match serde_json::from_slice(&event_bytes) {
        Ok(value) => value,
        Err(error) => { eprintln!("invalid impact-case JSON: {error}"); return ExitCode::from(2); }
    };

    match evaluate(&profile, &document) {
        Ok(mut receipt) => {
            receipt["source_commit"] = json!(args[4]);
            receipt["profile_sha256"] = json!(profile_digest);
            receipt["impact_cases_sha256"] = json!(event_digest);
            println!("{}", serde_json::to_string_pretty(&receipt).unwrap_or_else(|_| "{}".into()));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("sail capability impact evaluation rejected: {error}");
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> Value {
        serde_json::from_str(include_str!("../../../../benchmarks/maritime-observability/sail-capability-profile.v0.1.json"))
            .expect("sail capability profile must parse")
    }

    fn cases() -> Value {
        serde_json::from_str(include_str!("../../../../benchmarks/maritime-observability/sail-capability-impact-cases.v0.1.json"))
            .expect("sail capability impact cases must parse")
    }

    #[test]
    fn all_eight_frozen_change_events_match_the_expected_minimal_closure() {
        let result = evaluate(&profile(), &cases()).expect("impact corpus passes");
        assert_eq!(result["result"], "pass");
        assert_eq!(result["capability_count"], 7);
        assert_eq!(result["event_count"], 8);
        assert!(result["all_expected_impact_closures_matched"].as_bool().unwrap_or(false));
        assert_eq!(result["event_results"][0]["impacted_capability_ids"].as_array().unwrap().len(), 6);
        assert_eq!(result["event_results"][1]["impacted_capability_ids"].as_array().unwrap().len(), 3);
        assert_eq!(result["event_results"][3]["impacted_capability_ids"].as_array().unwrap().len(), 1);
        assert_eq!(result["event_results"][6]["unaffected_capability_ids"].as_array().unwrap().len(), 6);
    }

    #[test]
    fn unknown_changed_component_is_not_silently_ignored() {
        let mut document = cases();
        document["events"][0]["changed_component_ids"] = json!(["unknown_sail_sensor"]);
        let error = evaluate(&profile(), &document).unwrap_err();
        assert!(error.contains("unrecognized component"));
    }

    #[test]
    fn changed_shared_timebase_reaches_every_capability_that_requires_it() {
        let result = evaluate(&profile(), &cases()).expect("impact corpus passes");
        let event = &result["event_results"][5];
        assert_eq!(event["directly_impacted_capability_ids"].as_array().unwrap().len(), 7);
        assert_eq!(event["transitively_impacted_capability_ids"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn graph_cycle_is_rejected_before_change_evaluation() {
        let mut p = profile();
        p["capability_graph"][0]["depends_on"] = json!(["sail.plan_route_vmg"]);
        let error = evaluate(&p, &cases()).unwrap_err();
        assert!(error.contains("cycle"));
    }

    #[test]
    fn incomplete_or_overlapping_expected_partition_is_rejected() {
        let mut document = cases();
        document["events"][0]["expected_unaffected_capability_ids"] = json!([]);
        let error = evaluate(&profile(), &document).unwrap_err();
        assert!(error.contains("partition every capability"));
    }

    #[test]
    fn event_without_declared_change_is_rejected() {
        let mut document = cases();
        document["events"][0]["changed_component_ids"] = json!([]);
        let error = evaluate(&profile(), &document).unwrap_err();
        assert!(error.contains("must identify at least one changed component"));
    }

    #[test]
    fn duplicate_event_identity_is_rejected() {
        let mut document = cases();
        let duplicate_id = document["events"][0]["event_id"].clone();
        document["events"][1]["event_id"] = duplicate_id;
        let error = evaluate(&profile(), &document).unwrap_err();
        assert!(error.contains("duplicate impact event ID"));
    }

    #[test]
    fn unknown_profile_keys_are_rejected() {
        let mut value = profile();
        value["unreviewed_operating_flag"] = json!(true);
        let error = evaluate(&value, &cases()).unwrap_err();
        assert!(error.contains("unknown keys"));
    }

    #[test]
    fn malformed_configuration_digest_is_rejected() {
        let mut value = profile();
        value["asset_configuration"]["configuration_sha256"] = json!("not-a-digest");
        let error = evaluate(&value, &cases()).unwrap_err();
        assert!(error.contains("configuration digest is invalid"));
    }

    #[test]
    fn unknown_event_keys_are_rejected() {
        let mut document = cases();
        document["events"][0]["implicit_authorization"] = json!(true);
        let error = evaluate(&profile(), &document).unwrap_err();
        assert!(error.contains("unknown keys"));
    }

    #[test]
    fn authority_boundary_cannot_be_expanded() {
        let mut p = profile();
        p["authority_boundary"]["actuator_commands_emitted"] = json!(true);
        let error = evaluate(&p, &cases()).unwrap_err();
        assert!(error.contains("authority boundary"));
    }
}
