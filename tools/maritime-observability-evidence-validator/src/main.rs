use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    process::ExitCode,
};

const VERSION: &str = "0.1.0";

fn text<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}
fn boolean(v: &Value, key: &str) -> Option<bool> {
    v.get(key).and_then(Value::as_bool)
}
fn number(v: &Value, key: &str) -> Option<f64> {
    v.get(key).and_then(Value::as_f64).filter(|x| x.is_finite())
}
fn sha(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn push(errors: &mut Vec<Value>, code: &str, path: &str, detail: &str) {
    errors.push(json!({"code":code,"path":path,"detail":detail}));
}
fn keys(v: &Value, required: &[&str], optional: &[&str], path: &str, errors: &mut Vec<Value>) {
    let Some(map) = v.as_object() else {
        push(errors, "EXPECTED_OBJECT", path, "expected object");
        return;
    };
    let allowed: BTreeSet<&str> = required.iter().chain(optional.iter()).copied().collect();
    for k in required {
        if !map.contains_key(*k) {
            push(
                errors,
                "MISSING_KEY",
                &format!("{path}.{k}"),
                "required key missing",
            );
        }
    }
    for k in map.keys() {
        if !allowed.contains(k.as_str()) {
            push(
                errors,
                "UNKNOWN_KEY",
                &format!("{path}.{k}"),
                "unknown key rejected",
            );
        }
    }
}
fn string_set(v: &Value, key: &str, path: &str, errors: &mut Vec<Value>) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let Some(items) = v.get(key).and_then(Value::as_array) else {
        push(
            errors,
            "EXPECTED_ARRAY",
            &format!("{path}.{key}"),
            "expected array",
        );
        return out;
    };
    for (i, item) in items.iter().enumerate() {
        let Some(s) = item.as_str().filter(|s| !s.trim().is_empty()) else {
            push(
                errors,
                "INVALID_ARRAY_ITEM",
                &format!("{path}.{key}[{i}]"),
                "expected non-empty string",
            );
            continue;
        };
        if !out.insert(s.to_owned()) {
            push(
                errors,
                "DUPLICATE_ARRAY_ITEM",
                &format!("{path}.{key}[{i}]"),
                "duplicate item",
            );
        }
    }
    out
}
fn nullable_text<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    match v.get(key) {
        Some(Value::String(s)) if !s.trim().is_empty() => Some(s),
        _ => None,
    }
}
fn check_sha(v: &Value, key: &str, path: &str, nullable: bool, errors: &mut Vec<Value>) {
    if nullable && v.get(key).is_some_and(Value::is_null) {
        return;
    }
    if !v.get(key).and_then(Value::as_str).is_some_and(sha) {
        push(
            errors,
            "INVALID_SHA256",
            &format!("{path}.{key}"),
            "expected lowercase SHA-256",
        );
    }
}
fn pair_key(a: &str, b: &str) -> (String, String) {
    if a <= b {
        (a.into(), b.into())
    } else {
        (b.into(), a.into())
    }
}
fn cycle_visit(
    node: &str,
    graph: &BTreeMap<String, BTreeSet<String>>,
    marks: &mut BTreeMap<String, u8>,
    errors: &mut Vec<Value>,
) {
    match marks.get(node).copied().unwrap_or(0) {
        2 => return,
        1 => {
            push(
                errors,
                "DERIVATION_CYCLE",
                "$.sensor_inventory",
                "derived channel graph contains a cycle",
            );
            return;
        }
        _ => {}
    }
    marks.insert(node.into(), 1);
    if let Some(parents) = graph.get(node) {
        for parent in parents {
            if graph.contains_key(parent) {
                cycle_visit(parent, graph, marks, errors);
            }
        }
    }
    marks.insert(node.into(), 2);
}
fn physically_eligible(sensor: &Value) -> bool {
    boolean(sensor, "installed") == Some(true)
        && text(sensor, "source_role") == Some("measured")
        && text(sensor, "quality_state") == Some("current")
        && text(sensor, "calibration_state") == Some("current")
        && text(sensor, "timebase_qualification_state") == Some("qualified")
        && nullable_text(sensor, "timebase_id").is_some()
        && number(sensor, "timestamp_uncertainty_ms").is_some_and(|v| v >= 0.0)
        && sensor
            .get("timebase_qualification_sha256")
            .and_then(Value::as_str)
            .is_some_and(sha)
}
fn audit_pair(pair: &Value, sensors: &BTreeMap<String, &Value>, errors: &mut Vec<Value>) -> Value {
    let id = text(pair, "pair_id").unwrap_or("UNKNOWN");
    let mut reasons = BTreeSet::new();
    let mut channels = BTreeSet::new();
    let mut principles = BTreeSet::new();
    let mut dependencies: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut timebases = BTreeSet::new();
    let refs = pair.get("evidence_refs").and_then(Value::as_array);
    if let Some(refs) = refs {
        if refs.is_empty() {
            reasons.insert("NO_EVIDENCE");
        }
        for (i, e) in refs.iter().enumerate() {
            let path = format!("$.pair_assessments[{id}].evidence_refs[{i}]");
            check_sha(e, "sha256", &path, false, errors);
            match text(e, "evidence_role") {
                Some("measured") => {}
                Some("derived") => {
                    reasons.insert("DERIVED_EVIDENCE_NOT_INDEPENDENT");
                }
                Some("simulated") => {
                    reasons.insert("SIMULATION_ONLY_EVIDENCE");
                }
                Some("expert_authored") => {
                    reasons.insert("EXPERT_AUTHORED_NOT_MEASURED");
                }
                _ => {
                    reasons.insert("UNKNOWN_EVIDENCE_ROLE");
                }
            }
            for channel in string_set(e, "channel_ids", &path, errors) {
                channels.insert(channel.clone());
                let Some(sensor) = sensors.get(&channel) else {
                    push(
                        errors,
                        "UNKNOWN_EVIDENCE_CHANNEL",
                        &path,
                        &format!("unknown channel {channel}"),
                    );
                    reasons.insert("UNKNOWN_CHANNEL");
                    continue;
                };
                if !physically_eligible(sensor) {
                    reasons.insert("CHANNEL_NOT_PHYSICALLY_QUALIFIED");
                }
                if let Some(p) = text(sensor, "measurement_principle") {
                    principles.insert(p.to_owned());
                }
                if let Some(tb) = nullable_text(sensor, "timebase_id") {
                    timebases.insert(tb.to_owned());
                }
                for dep in string_set(
                    sensor,
                    "dependency_groups",
                    &format!("$.sensor_inventory[{channel}]"),
                    errors,
                ) {
                    dependencies.entry(dep).or_default().insert(channel.clone());
                }
            }
        }
    } else {
        push(
            errors,
            "EVIDENCE_REFS_REQUIRED",
            &format!("$.pair_assessments[{id}].evidence_refs"),
            "evidence_refs array required",
        );
        reasons.insert("NO_EVIDENCE");
    }
    if timebases.len() > 1 {
        reasons.insert("MIXED_TIMEBASES");
    }
    let shared: Vec<String> = dependencies
        .iter()
        .filter(|(_, channels)| channels.len() > 1)
        .map(|(dep, _)| dep.clone())
        .collect();
    let disposition = if reasons.is_empty() {
        "MEASURED_INPUTS_ELIGIBLE_FOR_INDEPENDENT_REVIEW"
    } else {
        "NOT_PHYSICALLY_ELIGIBLE"
    };
    let independence = if !shared.is_empty() {
        "SHARED_DECLARED_DEPENDENCY_REQUIRES_REVIEW"
    } else if principles.len() > 1 {
        "DISTINCT_DECLARED_PRINCIPLES_REQUIRES_REVIEW"
    } else {
        "SINGLE_DECLARED_PRINCIPLE_NO_INDEPENDENCE_CLAIM"
    };
    json!({"pair_id":id,"declared_pair_state":text(pair,"state").unwrap_or("unknown"),
        "evidence_disposition":disposition,"reason_codes":reasons.into_iter().collect::<Vec<_>>(),
        "referenced_channel_ids":channels.into_iter().collect::<Vec<_>>(),
        "declared_measurement_principle_count":principles.len(),"shared_dependency_groups":shared,
        "independence_review":independence,
        "claim_boundary":"Evidence admission only; this does not recompute fault-pair separability or certify physical diagnosability."})
}
fn validate(report: &Value) -> Result<Value, Vec<Value>> {
    let mut errors = Vec::new();
    keys(
        report,
        &[
            "schema_version",
            "assessment_id",
            "generated_at",
            "asset",
            "operating_domain",
            "evidence_digests",
            "sensor_inventory",
            "fault_assessments",
            "pair_assessments",
            "sensor_loss_scenarios",
            "metrics",
            "authority_boundary",
            "claim_ceiling",
        ],
        &[],
        "$",
        &mut errors,
    );
    if text(report, "schema_version") != Some("0.1.0") {
        push(
            &mut errors,
            "SCHEMA_VERSION",
            "$.schema_version",
            "expected 0.1.0",
        );
    }
    for k in ["assessment_id", "generated_at", "claim_ceiling"] {
        if text(report, k).is_none() {
            push(
                &mut errors,
                "MISSING_TEXT",
                &format!("$.{k}"),
                "non-empty string required",
            );
        }
    }
    keys(
        &report["asset"],
        &[
            "asset_id",
            "asset_class",
            "configuration_revision",
            "configuration_sha256",
        ],
        &["vessel_id", "engine_id"],
        "$.asset",
        &mut errors,
    );
    check_sha(
        &report["asset"],
        "configuration_sha256",
        "$.asset",
        false,
        &mut errors,
    );
    keys(
        &report["evidence_digests"],
        &[
            "sensor_topology_sha256",
            "fault_model_sha256",
            "evaluator_sha256",
            "dataset_sha256",
            "split_manifest_sha256",
            "model_sha256",
        ],
        &[],
        "$.evidence_digests",
        &mut errors,
    );
    for k in [
        "sensor_topology_sha256",
        "fault_model_sha256",
        "evaluator_sha256",
    ] {
        check_sha(
            &report["evidence_digests"],
            k,
            "$.evidence_digests",
            false,
            &mut errors,
        );
    }
    for k in ["dataset_sha256", "split_manifest_sha256", "model_sha256"] {
        check_sha(
            &report["evidence_digests"],
            k,
            "$.evidence_digests",
            true,
            &mut errors,
        );
    }
    let inventory = report["sensor_inventory"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if inventory.is_empty() {
        push(
            &mut errors,
            "SENSOR_INVENTORY_REQUIRED",
            "$.sensor_inventory",
            "at least one sensor/channel required",
        );
    }
    if inventory.len() > 4096 {
        push(
            &mut errors,
            "SENSOR_INVENTORY_LIMIT",
            "$.sensor_inventory",
            "max 4096 sensor records",
        );
    }
    let req = [
        "sensor_id",
        "channel_id",
        "installed",
        "measurement_principle",
        "unit",
        "source_role",
        "quality_state",
        "calibration_state",
        "dependency_groups",
        "derived_from_channel_ids",
        "timebase_id",
        "timestamp_uncertainty_ms",
        "timestamp_uncertainty_source",
        "timebase_qualification_state",
        "timebase_qualification_sha256",
    ];
    let optional = ["physical_location", "clock_domain", "acquisition_path"];
    let mut sensors: BTreeMap<String, &Value> = BTreeMap::new();
    let mut sensor_ids = BTreeSet::new();
    let mut graph: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (i, s) in inventory.iter().enumerate() {
        let path = format!("$.sensor_inventory[{i}]");
        keys(s, &req, &optional, &path, &mut errors);
        let Some(sid) = text(s, "sensor_id") else {
            push(
                &mut errors,
                "SENSOR_ID_REQUIRED",
                &path,
                "sensor ID required",
            );
            continue;
        };
        let Some(cid) = text(s, "channel_id") else {
            push(
                &mut errors,
                "CHANNEL_ID_REQUIRED",
                &path,
                "channel ID required",
            );
            continue;
        };
        if !sensor_ids.insert(sid.to_owned()) {
            push(
                &mut errors,
                "DUPLICATE_SENSOR_ID",
                &path,
                "sensor IDs must be unique",
            );
        }
        if sensors.insert(cid.to_owned(), s).is_some() {
            push(
                &mut errors,
                "DUPLICATE_CHANNEL_ID",
                &path,
                "channel IDs must be unique",
            );
        }
        if boolean(s, "installed").is_none() {
            push(
                &mut errors,
                "INSTALLED_FLAG_REQUIRED",
                &path,
                "installed must be boolean",
            );
        }
        if !matches!(
            text(s, "source_role"),
            Some("measured" | "derived" | "reconstructed" | "simulated")
        ) {
            push(
                &mut errors,
                "SOURCE_ROLE_INVALID",
                &path,
                "source role invalid",
            );
        }
        if !matches!(
            text(s, "quality_state"),
            Some("current" | "missing" | "stale" | "rejected" | "conflicted" | "unknown")
        ) {
            push(
                &mut errors,
                "QUALITY_STATE_INVALID",
                &path,
                "quality state invalid",
            );
        }
        if !matches!(
            text(s, "calibration_state"),
            Some("current" | "expired" | "unknown" | "not_applicable")
        ) {
            push(
                &mut errors,
                "CALIBRATION_STATE_INVALID",
                &path,
                "calibration state invalid",
            );
        }
        if !matches!(
            text(s, "timebase_qualification_state"),
            Some("qualified" | "unqualified" | "unknown")
        ) {
            push(
                &mut errors,
                "TIMEBASE_STATE_INVALID",
                &path,
                "timebase state invalid",
            );
        }
        if !matches!(
            text(s, "timestamp_uncertainty_source"),
            Some(
                "measured_offset_bound"
                    | "instrument_specification"
                    | "vendor_specification"
                    | "estimated"
                    | "unknown"
            )
        ) {
            push(
                &mut errors,
                "TIMESTAMP_SOURCE_INVALID",
                &path,
                "timestamp uncertainty source invalid",
            );
        }
        let parents = string_set(s, "derived_from_channel_ids", &path, &mut errors);
        graph.insert(cid.to_owned(), parents.clone());
        let _deps = string_set(s, "dependency_groups", &path, &mut errors);
        if text(s, "source_role") == Some("measured") && !parents.is_empty() {
            push(
                &mut errors,
                "MEASURED_HAS_PARENTS",
                &path,
                "directly measured channels cannot claim derivation parents",
            );
        }
        if matches!(text(s, "source_role"), Some("derived" | "reconstructed")) && parents.is_empty()
        {
            push(
                &mut errors,
                "DERIVED_PARENTS_REQUIRED",
                &path,
                "derived/reconstructed channels require parentage",
            );
        }
        if text(s, "timebase_qualification_state") == Some("qualified")
            && (nullable_text(s, "timebase_id").is_none()
                || number(s, "timestamp_uncertainty_ms").is_none_or(|n| n < 0.0)
                || !s["timebase_qualification_sha256"].as_str().is_some_and(sha))
        {
            push(
                &mut errors,
                "QUALIFIED_TIMEBASE_INCOMPLETE",
                &path,
                "qualified clock needs timebase ID, uncertainty bound and qualification SHA-256",
            );
        }
        if !s["timebase_qualification_sha256"].is_null()
            && !s["timebase_qualification_sha256"].as_str().is_some_and(sha)
        {
            push(
                &mut errors,
                "TIMEBASE_DIGEST_INVALID",
                &path,
                "invalid timebase qualification digest",
            );
        }
        if !s["timestamp_uncertainty_ms"].is_null()
            && number(s, "timestamp_uncertainty_ms").is_none_or(|n| n < 0.0)
        {
            push(
                &mut errors,
                "TIMESTAMP_UNCERTAINTY_INVALID",
                &path,
                "uncertainty must be finite/non-negative or null",
            );
        }
    }
    for (channel, parents) in &graph {
        for parent in parents {
            if !sensors.contains_key(parent) {
                push(
                    &mut errors,
                    "UNKNOWN_DERIVATION_PARENT",
                    &format!("$.sensor_inventory[{channel}]"),
                    &format!("unknown parent channel {parent}"),
                );
            }
        }
    }
    let mut marks = BTreeMap::new();
    for channel in graph.keys() {
        cycle_visit(channel, &graph, &mut marks, &mut errors);
    }
    let faults = report["fault_assessments"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut fault_ids = BTreeSet::new();
    for (i, f) in faults.iter().enumerate() {
        let path = format!("$.fault_assessments[{i}]");
        let Some(id) = text(f, "fault_id") else {
            push(&mut errors, "FAULT_ID_REQUIRED", &path, "fault id required");
            continue;
        };
        if !fault_ids.insert(id.to_owned()) {
            push(
                &mut errors,
                "DUPLICATE_FAULT_ID",
                &path,
                "fault IDs must be unique",
            );
        }
        for ev in f["signature_evidence"].as_array().into_iter().flatten() {
            check_sha(ev, "sha256", &path, false, &mut errors);
            for channel in string_set(ev, "channel_ids", &path, &mut errors) {
                if !sensors.contains_key(&channel) {
                    push(
                        &mut errors,
                        "UNKNOWN_SIGNATURE_CHANNEL",
                        &path,
                        &format!("unknown channel {channel}"),
                    );
                }
            }
        }
    }
    let pairs = report["pair_assessments"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut pair_ids = BTreeSet::new();
    let mut pair_keys = BTreeSet::new();
    let mut pair_audits = Vec::new();
    for (i, p) in pairs.iter().enumerate() {
        let path = format!("$.pair_assessments[{i}]");
        let id = text(p, "pair_id").unwrap_or("");
        if id.is_empty() || !pair_ids.insert(id.to_owned()) {
            push(
                &mut errors,
                "PAIR_ID_MISSING_OR_DUPLICATE",
                &path,
                "pair IDs must be unique and non-empty",
            );
        }
        let a = text(p, "fault_a").unwrap_or("");
        let b = text(p, "fault_b").unwrap_or("");
        if a == b || !fault_ids.contains(a) || !fault_ids.contains(b) {
            push(
                &mut errors,
                "PAIR_FAULT_REFERENCE_INVALID",
                &path,
                "pair must reference two distinct declared faults",
            );
        }
        if !pair_keys.insert(pair_key(a, b)) {
            push(
                &mut errors,
                "DUPLICATE_UNORDERED_PAIR",
                &path,
                "unordered fault pair duplicated",
            );
        }
        if !matches!(
            text(p, "state"),
            Some("isolable" | "non_isolable" | "not_evaluated")
        ) {
            push(
                &mut errors,
                "PAIR_STATE_INVALID",
                &path,
                "invalid pair state",
            );
        }
        pair_audits.push(audit_pair(p, &sensors, &mut errors));
    }
    let losses = report["sensor_loss_scenarios"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut loss_ids = BTreeSet::new();
    for (i, scenario) in losses.iter().enumerate() {
        let path = format!("$.sensor_loss_scenarios[{i}]");
        let id = text(scenario, "scenario_id").unwrap_or("");
        if id.is_empty() || !loss_ids.insert(id.to_owned()) {
            push(
                &mut errors,
                "LOSS_SCENARIO_ID_INVALID",
                &path,
                "scenario IDs must be unique",
            );
        }
        for sensor in string_set(scenario, "removed_sensor_ids", &path, &mut errors) {
            if !sensor_ids.contains(&sensor) {
                push(
                    &mut errors,
                    "UNKNOWN_REMOVED_SENSOR",
                    &path,
                    &format!("unknown sensor {sensor}"),
                );
            }
        }
        for dep in string_set(scenario, "removed_dependency_groups", &path, &mut errors) {
            if !sensors.values().any(|s| {
                s["dependency_groups"]
                    .as_array()
                    .is_some_and(|a| a.iter().any(|x| x.as_str() == Some(dep.as_str())))
            }) {
                push(
                    &mut errors,
                    "UNKNOWN_REMOVED_DEPENDENCY",
                    &path,
                    &format!("unknown dependency {dep}"),
                );
            }
        }
    }
    let metrics = report["metrics"].as_array().cloned().unwrap_or_default();
    let mut metric_ids = BTreeSet::new();
    for (i, m) in metrics.iter().enumerate() {
        let path = format!("$.metrics[{i}]");
        let id = text(m, "metric_id").unwrap_or("");
        if id.is_empty() || !metric_ids.insert(id.to_owned()) {
            push(
                &mut errors,
                "METRIC_ID_INVALID",
                &path,
                "metric ID must be unique",
            );
        }
        if number(m, "value").is_none()
            || m["sample_count"].as_u64().is_none_or(|n| n == 0)
            || text(m, "unit").is_none()
            || text(m, "stratum").is_none()
        {
            push(
                &mut errors,
                "METRIC_INVALID",
                &path,
                "metric requires finite value, positive sample count, unit and stratum",
            );
        }
    }
    keys(
        &report["authority_boundary"],
        &[
            "read_only",
            "actuation",
            "maintenance_authorization",
            "mode_transition_authorization",
            "restriction_clearance",
        ],
        &[],
        "$.authority_boundary",
        &mut errors,
    );
    if boolean(&report["authority_boundary"], "read_only") != Some(true) {
        push(
            &mut errors,
            "READ_ONLY_REQUIRED",
            "$.authority_boundary.read_only",
            "must be true",
        );
    }
    for key in [
        "actuation",
        "maintenance_authorization",
        "mode_transition_authorization",
        "restriction_clearance",
    ] {
        if boolean(&report["authority_boundary"], key) != Some(false) {
            push(
                &mut errors,
                "AUTHORITY_BOUNDARY_VIOLATION",
                &format!("$.authority_boundary.{key}"),
                "must be false",
            );
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let eligible = pair_audits
        .iter()
        .filter(|p| p["evidence_disposition"] == "MEASURED_INPUTS_ELIGIBLE_FOR_INDEPENDENT_REVIEW")
        .count();
    Ok(
        json!({"result":"pass","validator_version":VERSION,"assessment_id":text(report,"assessment_id").unwrap_or(""),
        "asset_configuration_sha256":report["asset"]["configuration_sha256"],
        "sensor_count":inventory.len(),"derived_or_reconstructed_sensor_count":inventory.iter().filter(|s|matches!(text(s,"source_role"),Some("derived"|"reconstructed"))).count(),
        "fault_count":faults.len(),"pair_count":pairs.len(),"pair_audits":pair_audits,
        "measured_input_pairs_eligible_for_independent_review":eligible,
        "claim_ceiling":"Evidence-admission preflight only; no pairwise fault separation, physical diagnosability, machinery health, safety or operating authority is established."}),
    )
}
fn run(path: &str, source: &str, digest: &str) -> Result<Value, String> {
    if source.len() != 40 || !source.bytes().all(|b| b.is_ascii_hexdigit()) || !sha(digest) {
        return Err("invalid source commit or report digest".into());
    }
    let raw = fs::read_to_string(path).map_err(|e| format!("read report: {e}"))?;
    let report: Value = serde_json::from_str(&raw).map_err(|e| format!("parse report: {e}"))?;
    match validate(&report) {
        Ok(mut receipt) => {
            receipt["source_commit"] = json!(source);
            receipt["report_sha256"] = json!(digest);
            Ok(receipt)
        }
        Err(errors) => Err(serde_json::to_string(&json!({"violations":errors}))
            .unwrap_or_else(|_| "validation failed".into())),
    }
}
fn main() -> ExitCode {
    let a: Vec<String> = env::args().collect();
    if a.len() != 6 || a[2] != "--source-commit" || a[4] != "--report-sha256" {
        eprintln!(
            "usage: maritime-observability-evidence-validator <report.json> --source-commit <40-hex> --report-sha256 <64-hex>"
        );
        return ExitCode::from(2);
    }
    match run(&a[1], &a[3], &a[5]) {
        Ok(v) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&v).unwrap_or_else(|_| "{\"result\":\"fail\"}".into())
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("evidence admission failed closed: {e}");
            ExitCode::FAILURE
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str =
        include_str!("../../../benchmarks/maritime-observability/synthetic-report-v0.1.json");
    fn fixture() -> Value {
        serde_json::from_str(FIXTURE).unwrap()
    }
    #[test]
    fn synthetic_report_passes() {
        let r = validate(&fixture()).unwrap();
        assert_eq!(r["sensor_count"], 6);
        assert_eq!(r["fault_count"], 3);
        assert_eq!(r["pair_count"], 3);
        assert_eq!(r["measured_input_pairs_eligible_for_independent_review"], 2);
    }
    #[test]
    fn derived_channel_cannot_be_measured_evidence() {
        let mut x = fixture();
        x["pair_assessments"][0]["evidence_refs"][0]["channel_ids"] = json!(["inferred-load"]);
        let r = validate(&x).unwrap();
        assert_eq!(
            r["pair_audits"][0]["evidence_disposition"],
            "NOT_PHYSICALLY_ELIGIBLE"
        );
    }
    #[test]
    fn expired_calibration_withdraws_eligibility() {
        let mut x = fixture();
        x["sensor_inventory"][0]["calibration_state"] = json!("expired");
        let r = validate(&x).unwrap();
        assert_eq!(
            r["pair_audits"][0]["evidence_disposition"],
            "NOT_PHYSICALLY_ELIGIBLE"
        );
    }
    #[test]
    fn unknown_clock_is_not_eligible() {
        let mut x = fixture();
        x["sensor_inventory"][0]["timebase_qualification_state"] = json!("unknown");
        x["sensor_inventory"][0]["timebase_id"] = Value::Null;
        x["sensor_inventory"][0]["timestamp_uncertainty_ms"] = Value::Null;
        x["sensor_inventory"][0]["timebase_qualification_sha256"] = Value::Null;
        let r = validate(&x).unwrap();
        assert_eq!(
            r["pair_audits"][0]["evidence_disposition"],
            "NOT_PHYSICALLY_ELIGIBLE"
        );
    }
    #[test]
    fn duplicate_channel_rejected() {
        let mut x = fixture();
        x["sensor_inventory"][1]["channel_id"] = x["sensor_inventory"][0]["channel_id"].clone();
        assert!(
            validate(&x)
                .unwrap_err()
                .iter()
                .any(|e| e["code"] == "DUPLICATE_CHANNEL_ID")
        );
    }
    #[test]
    fn duplicate_sensor_rejected() {
        let mut x = fixture();
        x["sensor_inventory"][1]["sensor_id"] = x["sensor_inventory"][0]["sensor_id"].clone();
        assert!(
            validate(&x)
                .unwrap_err()
                .iter()
                .any(|e| e["code"] == "DUPLICATE_SENSOR_ID")
        );
    }
    #[test]
    fn derived_cycle_rejected() {
        let mut x = fixture();
        x["sensor_inventory"][4]["derived_from_channel_ids"] = json!(["inferred-load"]);
        x["sensor_inventory"][5]["derived_from_channel_ids"] = json!(["fuel-flow"]);
        assert!(
            validate(&x)
                .unwrap_err()
                .iter()
                .any(|e| e["code"] == "DERIVATION_CYCLE")
        );
    }
    #[test]
    fn unknown_derived_parent_rejected() {
        let mut x = fixture();
        x["sensor_inventory"][4]["derived_from_channel_ids"] = json!(["missing"]);
        assert!(
            validate(&x)
                .unwrap_err()
                .iter()
                .any(|e| e["code"] == "UNKNOWN_DERIVATION_PARENT")
        );
    }
    #[test]
    fn unknown_evidence_channel_rejected() {
        let mut x = fixture();
        x["pair_assessments"][0]["evidence_refs"][0]["channel_ids"] = json!(["ghost"]);
        assert!(
            validate(&x)
                .unwrap_err()
                .iter()
                .any(|e| e["code"] == "UNKNOWN_EVIDENCE_CHANNEL")
        );
    }
    #[test]
    fn shared_dependency_is_surfaced() {
        let mut x = fixture();
        x["sensor_inventory"][1]["dependency_groups"]
            .as_array_mut()
            .unwrap()
            .push(json!("shared-clock"));
        x["sensor_inventory"][0]["dependency_groups"]
            .as_array_mut()
            .unwrap()
            .push(json!("shared-clock"));
        let r = validate(&x).unwrap();
        assert_eq!(
            r["pair_audits"][0]["independence_review"],
            "SHARED_DECLARED_DEPENDENCY_REQUIRES_REVIEW"
        );
    }
    #[test]
    fn mixed_common_timebases_withdraw_pair_eligibility() {
        let mut x = fixture();
        x["sensor_inventory"][1]["timebase_id"] = json!("another-timebase");
        let r = validate(&x).unwrap();
        assert_eq!(
            r["pair_audits"][0]["evidence_disposition"],
            "NOT_PHYSICALLY_ELIGIBLE"
        );
        assert!(
            r["pair_audits"][0]["reason_codes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v == "MIXED_TIMEBASES")
        );
    }
    #[test]
    fn authority_violation_rejected() {
        let mut x = fixture();
        x["authority_boundary"]["actuation"] = json!(true);
        assert!(
            validate(&x)
                .unwrap_err()
                .iter()
                .any(|e| e["code"] == "AUTHORITY_BOUNDARY_VIOLATION")
        );
    }
    #[test]
    fn invalid_digest_rejected() {
        let mut x = fixture();
        x["pair_assessments"][0]["evidence_refs"][0]["sha256"] = json!("bad");
        assert!(
            validate(&x)
                .unwrap_err()
                .iter()
                .any(|e| e["code"] == "INVALID_SHA256")
        );
    }
    #[test]
    fn reverse_duplicate_pair_rejected() {
        let mut x = fixture();
        x["pair_assessments"][1]["fault_a"] = x["pair_assessments"][0]["fault_b"].clone();
        x["pair_assessments"][1]["fault_b"] = x["pair_assessments"][0]["fault_a"].clone();
        assert!(
            validate(&x)
                .unwrap_err()
                .iter()
                .any(|e| e["code"] == "DUPLICATE_UNORDERED_PAIR")
        );
    }
    #[test]
    fn unknown_removed_dependency_rejected() {
        let mut x = fixture();
        x["sensor_loss_scenarios"][0]["removed_dependency_groups"] = json!(["unknown"]);
        assert!(
            validate(&x)
                .unwrap_err()
                .iter()
                .any(|e| e["code"] == "UNKNOWN_REMOVED_DEPENDENCY")
        );
    }
}
