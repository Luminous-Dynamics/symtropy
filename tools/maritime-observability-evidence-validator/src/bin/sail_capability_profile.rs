//! Validate a domain-specific sail capability graph against exact-scope cases.
//!
//! This adapter validates graph structure and profile-to-case binding only.
//! The existing qualification_scope binary remains responsible for each
//! capability's exact qualification disposition.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    process::ExitCode,
};

const PROFILE_VERSION: &str = "maritime-sail-capability-profile-v0.1";
const CASE_VERSION: &str = "maritime-qualification-scope-v0.1";
const ALLOWED_DEPENDENCIES: &[&str] = &[
    "sensor_topology", "calibration", "timebase", "source_payload",
    "diagnostic_model", "evaluator", "validation_profile",
];

fn object(value: &Value, at: &str) -> Result<&serde_json::Map<String, Value>, String> {
    value.as_object().ok_or_else(|| format!("{at}: expected object"))
}

fn exact_keys(value: &Value, allowed: &[&str], at: &str) -> Result<(), String> {
    let map = object(value, at)?;
    let expected: BTreeSet<&str> = allowed.iter().copied().collect();
    let actual: BTreeSet<&str> = map.keys().map(String::as_str).collect();
    let missing: Vec<_> = expected.difference(&actual).copied().collect();
    let unknown: Vec<_> = actual.difference(&expected).copied().collect();
    if !missing.is_empty() || !unknown.is_empty() {
        return Err(format!("{at}: missing keys {missing:?}; unknown keys {unknown:?}"));
    }
    Ok(())
}

fn nonempty_string<'a>(value: &'a Value, key: &str, at: &str) -> Result<&'a str, String> {
    value.get(key).and_then(Value::as_str).filter(|text| !text.trim().is_empty())
        .ok_or_else(|| format!("{at}.{key}: expected non-empty string"))
}

fn string_set(value: &Value, key: &str, at: &str) -> Result<BTreeSet<String>, String> {
    let values = value.get(key).and_then(Value::as_array)
        .ok_or_else(|| format!("{at}.{key}: expected array"))?;
    if values.is_empty() {
        return Err(format!("{at}.{key}: set must not be empty"));
    }
    let mut result = BTreeSet::new();
    for item in values {
        let text = item.as_str().filter(|text| !text.trim().is_empty())
            .ok_or_else(|| format!("{at}.{key}: expected non-empty strings"))?;
        if !result.insert(text.to_owned()) {
            return Err(format!("{at}.{key}: duplicate item {text:?}"));
        }
    }
    Ok(result)
}

fn valid_git_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_case_binding(
    capability: &Value,
    capability_id: &str,
    case: &Value,
    asset_id: &str,
    configuration_sha256: &str,
) -> Result<(), String> {
    let case_id = nonempty_string(capability, "qualification_case_id", "$.capability_graph[]")?;
    let at = format!("$.cases[{case_id}]");
    exact_keys(case, &["case_id", "qualification", "request", "expected_disposition"], &at)?;
    if nonempty_string(case, "case_id", &at)? != case_id {
        return Err(format!("{at}: case ID does not match its lookup key"));
    }
    if nonempty_string(case, "expected_disposition", &at)? != "SUPPORTED" {
        return Err(format!("{at}: graph nodes must map to a nominal SUPPORTED case"));
    }

    let qualification = case.get("qualification")
        .ok_or_else(|| format!("{at}: qualification is missing"))?;
    let request = case.get("request")
        .ok_or_else(|| format!("{at}: request is missing"))?;

    let capability_ids = string_set(qualification, "capability_ids", &format!("{at}.qualification"))?;
    if capability_ids != BTreeSet::from([capability_id.to_owned()]) {
        return Err(format!("{at}: qualification must cover exactly capability {capability_id}"));
    }

    let profile_dependencies =
        string_set(capability, "required_dependency_ids", "$.capability_graph[]")?;
    let case_dependencies =
        string_set(qualification, "required_dependency_ids", &format!("{at}.qualification"))?;
    if profile_dependencies != case_dependencies {
        return Err(format!("{at}: profile dependency set differs from exact-scope qualification case"));
    }

    let regime = nonempty_string(capability, "operating_regime", "$.capability_graph[]")?;
    let qualification_regimes =
        string_set(qualification, "operating_regimes", &format!("{at}.qualification"))?;
    if !qualification_regimes.contains(regime) {
        return Err(format!("{at}: operating regime is outside its qualified regime set"));
    }
    if nonempty_string(request, "requested_regime", &format!("{at}.request"))? != regime {
        return Err(format!("{at}: request regime differs from profile regime"));
    }
    if nonempty_string(request, "asset_id", &format!("{at}.request"))? != asset_id {
        return Err(format!("{at}: request asset differs from profile asset"));
    }
    if nonempty_string(request, "configuration_sha256", &format!("{at}.request"))?
        != configuration_sha256
    {
        return Err(format!("{at}: request configuration differs from profile configuration"));
    }
    Ok(())
}

fn topological_order(
    nodes: &BTreeMap<String, BTreeSet<String>>,
) -> Result<Vec<String>, String> {
    let mut indegree: BTreeMap<String, usize> =
        nodes.keys().map(|id| (id.clone(), 0_usize)).collect();
    let mut children: BTreeMap<String, BTreeSet<String>> =
        nodes.keys().map(|id| (id.clone(), BTreeSet::new())).collect();

    for (child, parents) in nodes {
        for parent in parents {
            if !nodes.contains_key(parent) {
                return Err(format!("{child}: unknown prerequisite capability {parent}"));
            }
            if parent == child {
                return Err(format!("{child}: capability cannot depend on itself"));
            }
            *indegree.get_mut(child).expect("child initialized") += 1;
            children.get_mut(parent).expect("parent was checked").insert(child.clone());
        }
    }

    let mut ready: BTreeSet<String> = indegree.iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(id, _)| id.clone()).collect();
    let mut ordered = Vec::with_capacity(nodes.len());

    while let Some(id) = ready.pop_first() {
        ordered.push(id.clone());
        for child in children.get(&id).expect("node initialized") {
            let degree = indegree.get_mut(child).expect("child initialized");
            *degree -= 1;
            if *degree == 0 {
                ready.insert(child.clone());
            }
        }
    }
    if ordered.len() != nodes.len() {
        return Err("capability dependency graph contains a cycle".into());
    }
    Ok(ordered)
}

fn evaluate_profile(profile: &Value, cases_document: &Value) -> Result<Value, String> {
    exact_keys(profile, &[
        "schema_version", "profile_id", "evidence_class", "as_of_utc",
        "asset_configuration", "capability_graph", "change_scenarios",
        "authority_boundary", "claim_ceiling",
    ], "$")?;
    if nonempty_string(profile, "schema_version", "$")? != PROFILE_VERSION
        || nonempty_string(profile, "evidence_class", "$")? != "SYNTHETIC_KNOWN_TRUTH"
    {
        return Err("unsupported sail capability profile schema or evidence class".into());
    }
    if nonempty_string(profile, "as_of_utc", "$")?.len() != 20 {
        return Err("$.as_of_utc: expected canonical UTC timestamp");
    }
    nonempty_string(profile, "profile_id", "$")?;
    nonempty_string(profile, "claim_ceiling", "$")?;

    let asset = profile.get("asset_configuration")
        .ok_or_else(|| "asset_configuration is missing".to_owned())?;
    exact_keys(asset, &["asset_id", "configuration_sha256", "component_identity_policy"], "$.asset_configuration")?;
    let asset_id = nonempty_string(asset, "asset_id", "$.asset_configuration")?;
    let configuration_sha256 =
        nonempty_string(asset, "configuration_sha256", "$.asset_configuration")?;
    if !valid_sha256(configuration_sha256) {
        return Err("$.asset_configuration.configuration_sha256: invalid SHA-256".into());
    }
    nonempty_string(asset, "component_identity_policy", "$.asset_configuration")?;

    let authority = profile.get("authority_boundary")
        .ok_or_else(|| "authority_boundary is missing".to_owned())?;
    exact_keys(authority, &[
        "read_only", "mutation_performed", "authorization_granted", "actuator_commands_emitted",
    ], "$.authority_boundary")?;
    if authority.get("read_only").and_then(Value::as_bool) != Some(true)
        || authority.get("mutation_performed").and_then(Value::as_bool) != Some(false)
        || authority.get("authorization_granted").and_then(Value::as_bool) != Some(false)
        || authority.get("actuator_commands_emitted").and_then(Value::as_bool) != Some(false)
    {
        return Err("authority boundary must remain read-only with no mutation, authorization or actuator output".into());
    }

    exact_keys(cases_document, &["schema_version", "evidence_class", "as_of_utc", "cases"], "$.qualification_cases")?;
    if nonempty_string(cases_document, "schema_version", "$.qualification_cases")? != CASE_VERSION
        || nonempty_string(cases_document, "evidence_class", "$.qualification_cases")? != "SYNTHETIC_KNOWN_TRUTH"
    {
        return Err("qualification case document has unsupported schema or evidence class".into());
    }
    if nonempty_string(cases_document, "as_of_utc", "$.qualification_cases")?
        != nonempty_string(profile, "as_of_utc", "$")?
    {
        return Err("profile and qualification cases use different as-of instants".into());
    }

    let case_items = cases_document.get("cases").and_then(Value::as_array)
        .ok_or_else(|| "$.qualification_cases.cases: expected array".to_owned())?;
    if case_items.is_empty() {
        return Err("qualification case document is empty".into());
    }
    let mut case_map = BTreeMap::new();
    for case in case_items {
        let id = nonempty_string(case, "case_id", "$.qualification_cases.cases[]")?;
        if case_map.insert(id.to_owned(), case).is_some() {
            return Err(format!("duplicate qualification case ID {id}"));
        }
    }

    let capabilities = profile.get("capability_graph").and_then(Value::as_array)
        .ok_or_else(|| "$.capability_graph: expected array".to_owned())?;
    if capabilities.is_empty() || capabilities.len() > 256 {
        return Err("$.capability_graph: node count must be in 1..=256".into());
    }

    let mut graph = BTreeMap::<String, BTreeSet<String>>::new();
    let mut referenced_cases = BTreeSet::new();
    let mut all_components = BTreeSet::<String>::new();
    let mut edge_count = 0_usize;

    for capability in capabilities {
        exact_keys(capability, &[
            "capability_id", "depends_on", "required_dependency_ids",
            "qualification_case_id", "operating_regime",
            "configuration_component_ids", "claim_ceiling",
        ], "$.capability_graph[]")?;
        let id = nonempty_string(capability, "capability_id", "$.capability_graph[]")?;
        if !id.starts_with("sail.")
            || id.chars().any(|ch| !(ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '.' || ch == '_'))
        {
            return Err(format!("invalid sail capability ID {id:?}"));
        }

        let parents = match capability.get("depends_on").and_then(Value::as_array) {
            Some(values) if values.is_empty() => BTreeSet::new(),
            Some(_) => string_set(capability, "depends_on", "$.capability_graph[]")?,
            None => return Err("$.capability_graph[].depends_on: expected array".into()),
        };
        let dependencies =
            string_set(capability, "required_dependency_ids", "$.capability_graph[]")?;
        if dependencies.iter().any(|dependency| !ALLOWED_DEPENDENCIES.contains(&dependency.as_str())) {
            return Err(format!("{id}: unknown evidence dependency class"));
        }
        let components =
            string_set(capability, "configuration_component_ids", "$.capability_graph[]")?;
        all_components.extend(components);
        nonempty_string(capability, "operating_regime", "$.capability_graph[]")?;
        nonempty_string(capability, "claim_ceiling", "$.capability_graph[]")?;

        let case_id = nonempty_string(capability, "qualification_case_id", "$.capability_graph[]")?;
        if !referenced_cases.insert(case_id.to_owned()) {
            return Err(format!("qualification case {case_id} is referenced by multiple capabilities"));
        }
        let case = case_map.get(case_id)
            .ok_or_else(|| format!("{id}: qualification case {case_id} does not exist"))?;
        validate_case_binding(capability, id, case, asset_id, configuration_sha256)?;

        edge_count += parents.len();
        if graph.insert(id.to_owned(), parents).is_some() {
            return Err(format!("duplicate capability ID {id}"));
        }
    }

    let order = topological_order(&graph)?;
    let all_capabilities: BTreeSet<String> = graph.keys().cloned().collect();
    let scenario_items = profile.get("change_scenarios").and_then(Value::as_array)
        .ok_or_else(|| "$.change_scenarios: expected array".to_owned())?;
    if scenario_items.is_empty() || scenario_items.len() > 1024 {
        return Err("$.change_scenarios: scenario count must be in 1..=1024".into());
    }
    let mut scenario_ids = BTreeSet::new();
    let mut scenario_results = Vec::new();

    for scenario in scenario_items {
        exact_keys(scenario, &[
            "scenario_id", "trigger_component_ids", "directly_invalidated_capability_ids",
            "expected_invalidated_capability_ids", "expected_preserved_capability_ids",
            "claim_ceiling",
        ], "$.change_scenarios[]")?;
        let scenario_id = nonempty_string(scenario, "scenario_id", "$.change_scenarios[]")?;
        if !scenario_ids.insert(scenario_id.to_owned()) {
            return Err(format!("duplicate change scenario ID {scenario_id}"));
        }
        let triggers = string_set(scenario, "trigger_component_ids", "$.change_scenarios[]")?;
        if let Some(unknown) = triggers.iter().find(|id| !all_components.contains(*id)) {
            return Err(format!("{scenario_id}: unknown trigger component {unknown}"));
        }
        let direct = string_set(
            scenario,
            "directly_invalidated_capability_ids",
            "$.change_scenarios[]",
        )?;
        if let Some(unknown) = direct.iter().find(|id| !all_capabilities.contains(*id)) {
            return Err(format!("{scenario_id}: unknown directly invalidated capability {unknown}"));
        }

        let mut closure = direct.clone();
        loop {
            let newly_invalidated: Vec<String> = graph.iter()
                .filter(|(id, parents)| {
                    !closure.contains(*id) && parents.iter().any(|parent| closure.contains(parent))
                })
                .map(|(id, _)| id.clone())
                .collect();
            if newly_invalidated.is_empty() {
                break;
            }
            closure.extend(newly_invalidated);
        }

        let expected_invalidated = string_set(
            scenario,
            "expected_invalidated_capability_ids",
            "$.change_scenarios[]",
        )?;
        let expected_preserved = string_set(
            scenario,
            "expected_preserved_capability_ids",
            "$.change_scenarios[]",
        )?;
        if expected_invalidated.intersection(&expected_preserved).next().is_some() {
            return Err(format!("{scenario_id}: invalidated and preserved sets overlap"));
        }
        let declared_partition: BTreeSet<String> =
            expected_invalidated.union(&expected_preserved).cloned().collect();
        if declared_partition != all_capabilities {
            return Err(format!("{scenario_id}: expected invalidated/preserved sets do not partition the capability graph"));
        }
        if expected_invalidated != closure {
            return Err(format!(
                "{scenario_id}: expected invalidation closure differs; expected={expected_invalidated:?}, computed={closure:?}"
            ));
        }
        let computed_preserved: BTreeSet<String> =
            all_capabilities.difference(&closure).cloned().collect();
        if expected_preserved != computed_preserved {
            return Err(format!("{scenario_id}: expected preserved set differs from computed complement"));
        }
        nonempty_string(scenario, "claim_ceiling", "$.change_scenarios[]")?;

        scenario_results.push(json!({
            "scenario_id": scenario_id,
            "trigger_component_ids": triggers,
            "directly_invalidated_capability_ids": direct,
            "invalidated_capability_ids": closure,
            "preserved_capability_ids": computed_preserved,
            "withdrawal_is_selective": !computed_preserved.is_empty()
        }));
    }

    let unreferenced_cases: Vec<String> = case_map.keys()
        .filter(|id| !referenced_cases.contains(*id))
        .cloned().collect();

    Ok(json!({
        "result": "pass",
        "profile_schema_version": PROFILE_VERSION,
        "case_schema_version": CASE_VERSION,
        "profile_id": nonempty_string(profile, "profile_id", "$")?,
        "asset_id": asset_id,
        "configuration_sha256": configuration_sha256,
        "capability_count": graph.len(),
        "dependency_edge_count": edge_count,
        "topological_order": order,
        "qualification_case_count": case_items.len(),
        "capabilities_with_nominal_scope_cases": referenced_cases.len(),
        "unreferenced_validation_case_ids": unreferenced_cases,
        "change_scenario_count": scenario_results.len(),
        "selective_withdrawal_scenario_count": scenario_results.iter()
            .filter(|scenario| scenario["withdrawal_is_selective"] == true).count(),
        "change_scenarios": scenario_results,
        "acyclic": true,
        "mutation_performed": false,
        "authorization_granted": false,
        "actuator_commands_emitted": false,
        "claim_ceiling": nonempty_string(profile, "claim_ceiling", "$")?
    }))
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 9
        || args[3] != "--source-commit"
        || args[5] != "--profile-sha256"
        || args[7] != "--cases-sha256"
    {
        eprintln!("usage: sail_capability_profile <profile.json> <qualification-cases.json> --source-commit <sha> --profile-sha256 <sha256> --cases-sha256 <sha256>");
        return ExitCode::from(2);
    }
    let source_commit = &args[4];
    if !valid_git_sha(source_commit) || !valid_sha256(&args[6]) || !valid_sha256(&args[8]) {
        eprintln!("invalid source commit or artifact digest");
        return ExitCode::from(2);
    }

    let profile_bytes = match fs::read(&args[1]) {
        Ok(bytes) => bytes,
        Err(error) => { eprintln!("cannot read profile: {error}"); return ExitCode::from(2); }
    };
    let cases_bytes = match fs::read(&args[2]) {
        Ok(bytes) => bytes,
        Err(error) => { eprintln!("cannot read qualification cases: {error}"); return ExitCode::from(2); }
    };
    let profile_digest = format!("{:x}", Sha256::digest(&profile_bytes));
    let cases_digest = format!("{:x}", Sha256::digest(&cases_bytes));
    if profile_digest != args[6] || cases_digest != args[8] {
        eprintln!("profile or qualification-case digest mismatch");
        return ExitCode::from(2);
    }
    let profile: Value = match serde_json::from_slice(&profile_bytes) {
        Ok(value) => value,
        Err(error) => { eprintln!("invalid profile JSON: {error}"); return ExitCode::from(2); }
    };
    let cases: Value = match serde_json::from_slice(&cases_bytes) {
        Ok(value) => value,
        Err(error) => { eprintln!("invalid qualification-case JSON: {error}"); return ExitCode::from(2); }
    };

    match evaluate_profile(&profile, &cases) {
        Ok(mut receipt) => {
            receipt["source_commit"] = serde_json::json!(source_commit);
            receipt["profile_sha256"] = serde_json::json!(profile_digest);
            receipt["qualification_cases_sha256"] = serde_json::json!(cases_digest);
            println!("{}", serde_json::to_string_pretty(&receipt).unwrap_or_else(|_| "{}".into()));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("sail capability profile rejected: {error}");
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
        serde_json::from_str(include_str!("../../../../benchmarks/maritime-observability/sail-capability-scope-cases.v0.1.json"))
            .expect("sail qualification cases must parse")
    }

    #[test]
    fn frozen_sail_profile_binds_seven_capabilities_to_scope_cases() {
        let report = evaluate_profile(&profile(), &cases()).expect("profile validates");
        assert_eq!(report["result"], "pass");
        assert_eq!(report["capability_count"], 7);
        assert_eq!(report["dependency_edge_count"], 9);
        assert_eq!(report["qualification_case_count"], 16);
        assert_eq!(report["capabilities_with_nominal_scope_cases"], 7);
        assert_eq!(report["unreferenced_validation_case_ids"].as_array().unwrap().len(), 9);
        assert!(report["acyclic"].as_bool().unwrap_or(false));
        assert!(!report["actuator_commands_emitted"].as_bool().unwrap_or(true));
    }

    #[test]
    fn wind_sensor_loss_withdraws_only_wind_dependent_capabilities() {
        let report = evaluate_profile(&profile(), &cases()).expect("profile validates");
        let scenario = report["change_scenarios"].as_array().unwrap().iter()
            .find(|value| value["scenario_id"] == "wind_sensor_lost").unwrap();
        assert_eq!(scenario["invalidated_capability_ids"].as_array().unwrap().len(), 6);
        assert_eq!(scenario["preserved_capability_ids"], json!(["sail.estimate_navigation"]));
        assert_eq!(scenario["withdrawal_is_selective"], true);
    }

    #[test]
    fn forecast_expiry_does_not_withdraw_non_routing_capabilities() {
        let report = evaluate_profile(&profile(), &cases()).expect("profile validates");
        let scenario = report["change_scenarios"].as_array().unwrap().iter()
            .find(|value| value["scenario_id"] == "forecast_payload_expired").unwrap();
        assert_eq!(scenario["invalidated_capability_ids"], json!(["sail.plan_route_vmg"]));
        assert_eq!(scenario["preserved_capability_ids"].as_array().unwrap().len(), 6);
    }

    #[test]
    fn partial_recovery_preserves_wind_monitoring_while_helm_fault_remains() {
        let report = evaluate_profile(&profile(), &cases()).expect("profile validates");
        let scenario = report["change_scenarios"].as_array().unwrap().iter()
            .find(|value| value["scenario_id"] == "wind_restored_rudder_fault_remains").unwrap();
        assert!(scenario["preserved_capability_ids"].as_array().unwrap()
            .iter().any(|id| id == "sail.monitor_wind"));
        assert!(scenario["invalidated_capability_ids"].as_array().unwrap()
            .iter().any(|id| id == "sail.hold_course"));
    }

    #[test]
    fn understated_or_overstated_invalidation_closure_is_rejected() {
        let mut value = profile();
        value["change_scenarios"][0]["expected_invalidated_capability_ids"]
            .as_array_mut().unwrap().pop();
        let error = evaluate_profile(&value, &cases()).unwrap_err();
        assert!(error.contains("invalidation closure differs"));
    }

    #[test]
    fn unknown_change_trigger_component_is_rejected() {
        let mut value = profile();
        value["change_scenarios"][0]["trigger_component_ids"] =
            json!(["unregistered_actuator"]);
        let error = evaluate_profile(&value, &cases()).unwrap_err();
        assert!(error.contains("unknown trigger component"));
    }

    #[test]
    fn capability_dependency_cycles_are_rejected() {
        let mut value = profile();
        value["capability_graph"][0]["depends_on"] = serde_json::json!(["sail.plan_route_vmg"]);
        let error = evaluate_profile(&value, &cases()).unwrap_err();
        assert!(error.contains("cycle"));
    }

    #[test]
    fn unknown_capability_parent_is_rejected() {
        let mut value = profile();
        value["capability_graph"][0]["depends_on"] = serde_json::json!(["sail.unknown_capability"]);
        let error = evaluate_profile(&value, &cases()).unwrap_err();
        assert!(error.contains("unknown prerequisite"));
    }

    #[test]
    fn capability_scope_case_cannot_widen_its_capability_set() {
        let mut value = cases();
        let case = value["cases"].as_array_mut().unwrap().iter_mut()
            .find(|item| item["case_id"] == "nominal-sail-monitor_wind").unwrap();
        case["qualification"]["capability_ids"] = serde_json::json!(["sail.monitor_wind", "sail.tack"]);
        let error = evaluate_profile(&profile(), &value).unwrap_err();
        assert!(error.contains("exactly capability"));
    }

    #[test]
    fn profile_and_scope_case_dependency_sets_must_match() {
        let mut value = profile();
        value["capability_graph"][0]["required_dependency_ids"] = serde_json::json!(["sensor_topology", "calibration"]);
        let error = evaluate_profile(&value, &cases()).unwrap_err();
        assert!(error.contains("dependency set differs"));
    }

    #[test]
    fn asset_or_configuration_mismatch_is_rejected() {
        let mut value = cases();
        value["cases"][0]["request"]["configuration_sha256"] = serde_json::json!("b".repeat(64));
        let error = evaluate_profile(&profile(), &value).unwrap_err();
        assert!(error.contains("configuration differs"));
    }

    #[test]
    fn authority_expansion_is_rejected() {
        let mut value = profile();
        value["authority_boundary"]["actuator_commands_emitted"] = serde_json::json!(true);
        let error = evaluate_profile(&value, &cases()).unwrap_err();
        assert!(error.contains("authority boundary"));
    }
}
