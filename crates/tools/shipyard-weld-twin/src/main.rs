// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

fn value_at<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = root;
    for part in path.split('.') {
        current = match current {
            Value::Object(object) => object.get(part)?,
            Value::Array(array) => array.get(part.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(current)
}

fn text_at<'a>(root: &'a Value, path: &str) -> Option<&'a str> {
    value_at(root, path)?.as_str()
}

fn bool_at(root: &Value, path: &str) -> Option<bool> {
    value_at(root, path)?.as_bool()
}

fn number_at(root: &Value, path: &str) -> Option<u64> {
    value_at(root, path)?.as_u64()
}

fn is_canonical_utc(timestamp: &str) -> bool {
    let bytes = timestamp.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return false;
    }

    for (start, end) in [(0, 4), (5, 7), (8, 10), (11, 13), (14, 16), (17, 19)] {
        if !bytes[start..end].iter().all(u8::is_ascii_digit) {
            return false;
        }
    }

    let Ok(year) = timestamp[0..4].parse::<u32>() else {
        return false;
    };
    let Ok(month) = timestamp[5..7].parse::<u32>() else {
        return false;
    };
    let Ok(day) = timestamp[8..10].parse::<u32>() else {
        return false;
    };
    let Ok(hour) = timestamp[11..13].parse::<u32>() else {
        return false;
    };
    let Ok(minute) = timestamp[14..16].parse::<u32>() else {
        return false;
    };
    let Ok(second) = timestamp[17..19].parse::<u32>() else {
        return false;
    };

    if year == 0
        || !(1..=12).contains(&month)
        || !(0..=23).contains(&hour)
        || !(0..=59).contains(&minute)
        || !(0..=59).contains(&second)
    {
        return false;
    }

    let leap_year = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days_in_month).contains(&day)
}

fn is_current_until(root: &Value, path: &str, cutoff: &str) -> bool {
    text_at(root, path).is_some_and(|until| is_canonical_utc(until) && until > cutoff)
}

fn validate_ruleset_effectivity(weld: &Value, cutoff: &str) -> &'static str {
    let Some(retrieved_at) = text_at(weld, "normative_context.source_retrieved_at_utc") else {
        return "NORMATIVE_CONTEXT_MISSING";
    };
    if !is_canonical_utc(retrieved_at) || retrieved_at > cutoff {
        return "NORMATIVE_CONTEXT_RETRIEVED_AFTER_DECISION_CUTOFF";
    }

    let Some(subject_date) =
        text_at(weld, "normative_context.subject_applicability_date_utc")
    else {
        return "NORMATIVE_CONTEXT_MISSING";
    };
    if !is_canonical_utc(subject_date) {
        return "INVALID_SUBJECT_APPLICABILITY_DATE";
    }

    let Some(applicable) = value_at(weld, "normative_context.applicable_ruleset") else {
        return "NORMATIVE_CONTEXT_MISSING";
    };
    if text_at(applicable, "status") == Some("future_for_subject_effectivity") {
        return "FUTURE_RULESET_NOT_EFFECTIVE";
    }
    if text_at(applicable, "status") != Some("applicable_for_subject_effectivity") {
        return "RULESET_NOT_APPLICABLE_TO_SUBJECT";
    }
    if let Some(start) = text_at(applicable, "effective_from_utc") {
        if !is_canonical_utc(start) {
            return "INVALID_RULESET_EFFECTIVITY";
        }
        if start > subject_date {
            return "RULESET_NOT_YET_EFFECTIVE_FOR_SUBJECT";
        }
    }
    if let Some(end) = text_at(applicable, "effective_until_utc") {
        if !is_canonical_utc(end) {
            return "INVALID_RULESET_EFFECTIVITY";
        }
        if end <= subject_date {
            return "RULESET_EXPIRED_FOR_SUBJECT_EFFECTIVITY";
        }
    }

    let Some(future) = value_at(weld, "normative_context.future_ruleset") else {
        return "FUTURE_RULESET_CONTEXT_MISSING";
    };
    if text_at(future, "status") == Some("future_for_subject_effectivity") {
        let Some(start) = text_at(future, "effective_from_utc") else {
            return "FUTURE_RULESET_CONTEXT_MISSING";
        };
        if !is_canonical_utc(start) || start <= subject_date {
            return "FUTURE_RULESET_EFFECTIVITY_CONFLICT";
        }
    }
    "RULESET_APPLICABLE_TO_SUBJECT"
}

fn resolve_trace_path(workspace_root: &Path, relative: &str) -> Result<PathBuf, &'static str> {
    if relative.is_empty() || relative.contains('\\') || relative.contains(':') {
        return Err("UNSAFE_PROCESS_TRACE_PATH");
    }
    let relative_path = Path::new(relative);
    if relative_path.is_absolute()
        || relative_path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err("UNSAFE_PROCESS_TRACE_PATH");
    }

    let root = workspace_root
        .canonicalize()
        .map_err(|_| "PROCESS_TRACE_ROOT_UNAVAILABLE")?;
    let candidate = root.join(relative_path);
    let canonical = candidate
        .canonicalize()
        .map_err(|_| "PROCESS_TRACE_UNAVAILABLE")?;
    if !canonical.starts_with(&root) {
        return Err("UNSAFE_PROCESS_TRACE_PATH");
    }
    Ok(canonical)
}

fn verify_process_trace(
    weld: &Value,
    pass: &Value,
    workspace_root: &Path,
) -> &'static str {
    let Some(relative) = pass.get("process_trace_ref").and_then(Value::as_str) else {
        return "PROCESS_TRACE_MISSING";
    };
    let path = match resolve_trace_path(workspace_root, relative) {
        Ok(value) => value,
        Err(code) => return code,
    };
    let bytes = match fs::read(path) {
        Ok(value) => value,
        Err(_) => return "PROCESS_TRACE_UNAVAILABLE",
    };
    let observed_digest = format!("{:x}", Sha256::digest(&bytes));
    if pass.get("process_trace_sha256").and_then(Value::as_str)
        != Some(observed_digest.as_str())
    {
        return "PROCESS_TRACE_DIGEST_MISMATCH";
    }

    let trace: Value = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(_) => return "PROCESS_TRACE_INVALID_JSON",
    };
    let bindings = [
        ("fixture_kind", Some("synthetic_fixture")),
        ("workpiece_component_id", text_at(weld, "workpiece.component_id")),
        ("workpiece_serial_id", text_at(weld, "workpiece.serial_id")),
        ("joint_id", text_at(pass, "joint_id")),
        ("pass_id", text_at(pass, "pass_id")),
        ("pass_role", text_at(pass, "role")),
        ("workpiece_revision", text_at(pass, "workpiece_revision")),
        ("wps_revision", text_at(pass, "wps_revision")),
        ("equipment_revision", text_at(pass, "equipment_revision")),
        ("parameter_envelope_ref", text_at(pass, "parameter_envelope_ref")),
        ("observed_envelope_status", text_at(pass, "observed_envelope_status")),
        ("source_time_utc", text_at(pass, "source_time_utc")),
    ];
    if bindings.iter().any(|(field, expected)| {
        trace.get(*field).and_then(Value::as_str) != *expected
    }) {
        return "PROCESS_TRACE_SUBJECT_MISMATCH";
    }
    if text_at(&trace, "evidence_class") != Some("synthetic_fixture") {
        return "EVIDENCE_CLASS_MISMATCH";
    }
    "PROCESS_TRACE_VALID_EXACT_BYTES"
}

fn evaluate(weld: &Value, workspace_root: &Path) -> &'static str {
    let cutoff = match text_at(weld, "evaluation_cutoff_utc") {
        Some(value) if is_canonical_utc(value) => value,
        _ => return "INVALID_DECISION_CUTOFF",
    };

    let ruleset_disposition = validate_ruleset_effectivity(weld, cutoff);
    if ruleset_disposition != "RULESET_APPLICABLE_TO_SUBJECT" {
        return ruleset_disposition;
    }

    let serial = match text_at(weld, "workpiece.serial_id") {
        Some(value) => value,
        None => return "WORKPIECE_IDENTITY_MISSING",
    };
    let joint = match text_at(weld, "workpiece.joint_id") {
        Some(value) => value,
        None => return "JOINT_IDENTITY_MISSING",
    };
    let revision = match text_at(weld, "workpiece.design_revision") {
        Some(value) => value,
        None => return "DESIGN_REVISION_MISSING",
    };

    if text_at(weld, "workpiece.material_heat_lot")
        != text_at(weld, "material_receiving.material_heat_lot")
        || text_at(weld, "workpiece.material_spec_ref")
            != text_at(weld, "material_receiving.material_spec_ref")
        || text_at(weld, "material_receiving.disposition") != Some("accepted_synthetic")
    {
        return "MATERIAL_LOT_MISMATCH";
    }

    if text_at(weld, "procedure.qualified_scope")
        != text_at(weld, "workpiece.required_process_scope")
    {
        return "PQR_SCOPE_MISMATCH";
    }

    if text_at(weld, "procedure.approval_status") != Some("approved") {
        return "PROCEDURE_NOT_APPROVED";
    }
    let procedure_valid_from = match text_at(weld, "procedure.valid_from_utc") {
        Some(value) if is_canonical_utc(value) => value,
        _ => return "INVALID_PROCEDURE_VALID_FROM",
    };
    if procedure_valid_from > cutoff {
        return "PROCEDURE_NOT_YET_EFFECTIVE";
    }
    if !is_current_until(weld, "procedure.valid_until_utc", cutoff) {
        return "PROCEDURE_EXPIRED";
    }

    if text_at(weld, "operator.qualification_status") != Some("current")
        || !is_current_until(weld, "operator.valid_until_utc", cutoff)
    {
        return "OPERATOR_QUALIFICATION_EXPIRED";
    }
    if text_at(weld, "operator.role_scope")
        != text_at(weld, "workpiece.required_operator_scope")
    {
        return "OPERATOR_SCOPE_MISMATCH";
    }

    if !is_current_until(weld, "equipment.calibration_valid_until_utc", cutoff)
        || text_at(weld, "equipment.calibration_state") != Some("current")
    {
        return "CALIBRATION_STALE";
    }
    if text_at(weld, "equipment.equipment_health_state") != Some("nominal_synthetic") {
        return "EQUIPMENT_HEALTH_NOT_QUALIFIED";
    }

    if text_at(weld, "equipment.geometry_sha256")
        != text_at(weld, "workpiece.geometry_sha256")
    {
        return "PROGRAM_GEOMETRY_MISMATCH";
    }

    if text_at(weld, "preprocessing.joint_preparation_state")
        != Some("complete_synthetic")
    {
        return "PREPROCESSING_INCOMPLETE";
    }
    if text_at(weld, "preprocessing.precondition_state") != Some("satisfied_synthetic") {
        return "PRECONDITION_NOT_SATISFIED";
    }

    let consumables = match value_at(weld, "consumables").and_then(Value::as_array) {
        Some(values) => values,
        None => return "CONSUMABLE_SET_MISSING",
    };
    if consumables.iter().any(|item| {
        item.get("lot_id").and_then(Value::as_str).is_none_or(str::is_empty)
    }) {
        return "CONSUMABLE_LOT_MISSING";
    }
    if consumables.iter().any(|item| {
        item.get("compatibility").and_then(Value::as_str) != Some("qualified_synthetic")
    }) {
        return "CONSUMABLE_INCOMPATIBLE";
    }

    let passes = match value_at(weld, "passes").and_then(Value::as_array) {
        Some(values) => values,
        None => return "PASS_SET_MISSING",
    };
    if passes.len() < 3
        || passes.first().and_then(|pass| pass.get("role")).and_then(Value::as_str)
            != Some("root")
    {
        return "ROOT_PASS_MISSING";
    }

    let mut pass_ids = std::collections::BTreeSet::new();
    for pass in passes {
        let pass_id = match pass.get("pass_id").and_then(Value::as_str) {
            Some(value) => value,
            None => return "PASS_ID_MISSING",
        };
        if !pass_ids.insert(pass_id) {
            return "DUPLICATE_PASS_ID";
        }
    }

    let expected_roles = ["root", "fill", "cap"];
    for (index, pass) in passes.iter().enumerate() {
        if index >= expected_roles.len()
            || pass.get("role").and_then(Value::as_str) != Some(expected_roles[index])
        {
            return "PASS_SEQUENCE_INVALID";
        }
        if pass.get("sequence_index").and_then(Value::as_u64) != Some((index + 1) as u64) {
            return "PASS_SEQUENCE_INVALID";
        }
        let pass_time = match pass.get("source_time_utc").and_then(Value::as_str) {
            Some(value) if is_canonical_utc(value) => value,
            _ => return "INVALID_PASS_SOURCE_TIME",
        };
        if pass_time > cutoff {
            return "PROCESS_TRACE_AFTER_DECISION_CUTOFF";
        }
        if pass.get("workpiece_serial_id").and_then(Value::as_str) != Some(serial) {
            return "WORKPIECE_IDENTITY_MISMATCH";
        }
        if pass.get("joint_id").and_then(Value::as_str) != Some(joint) {
            return "JOINT_IDENTITY_MISMATCH";
        }
        if pass.get("workpiece_revision").and_then(Value::as_str) != Some(revision) {
            return "DESIGN_REVISION_MISMATCH";
        }
        if pass.get("wps_revision").and_then(Value::as_str)
            != text_at(weld, "procedure.wps_revision")
        {
            return "WPS_REVISION_MISMATCH";
        }
        if pass.get("parameter_envelope_ref").and_then(Value::as_str)
            != text_at(weld, "procedure.parameter_envelope_ref")
        {
            return "PARAMETER_ENVELOPE_MISMATCH";
        }
        if pass.get("equipment_revision").and_then(Value::as_str)
            != text_at(weld, "equipment.robot_revision")
        {
            return "EQUIPMENT_REVISION_MISMATCH";
        }
        if pass.get("program_revision").and_then(Value::as_str)
            != text_at(weld, "equipment.program_revision")
        {
            return "PROGRAM_REVISION_MISMATCH";
        }
        if pass.get("observed_envelope_status").and_then(Value::as_str)
            != Some("within_declared_envelope")
        {
            return "PROCESS_ENVELOPE_EXCEEDED";
        }
        if pass.get("evidence_class").and_then(Value::as_str) != Some("synthetic_fixture") {
            return "EVIDENCE_CLASS_MISMATCH";
        }
        let trace_disposition = verify_process_trace(weld, pass, workspace_root);
        if trace_disposition != "PROCESS_TRACE_VALID_EXACT_BYTES" {
            return trace_disposition;
        }
    }

    let in_process = match value_at(weld, "inspections.in_process").and_then(Value::as_array) {
        Some(values) => values,
        None => return "IN_PROCESS_INSPECTION_SET_MISSING",
    };
    for (hold_point, code) in [
        ("root", "ROOT_INSPECTION_MISSING"),
        ("fill_sampling", "FILL_SAMPLING_MISSING"),
        ("cap", "CAP_INSPECTION_MISSING"),
    ] {
        let record = in_process.iter().find(|item| {
            item.get("hold_point").and_then(Value::as_str) == Some(hold_point)
        });
        let Some(record) = record else {
            return code;
        };
        let expected_coverage = if hold_point == "fill_sampling" {
            Some("required_sampling")
        } else {
            Some("required")
        };
        if record.get("joint_id").and_then(Value::as_str) != Some(joint)
            || record.get("disposition").and_then(Value::as_str)
                != Some("accepted_synthetic")
            || record.get("coverage").and_then(Value::as_str) != expected_coverage
        {
            return code;
        }
    }

    let ndt_disposition = text_at(weld, "inspections.ndt.disposition");
    if ndt_disposition == Some("rejected") {
        return "INSPECTION_DISPOSITION_CONFLICT";
    }
    if ndt_disposition != Some("accepted_synthetic") {
        return "NDT_NOT_ACCEPTED";
    }
    if text_at(weld, "inspections.ndt.joint_id") != Some(joint)
        || text_at(weld, "inspections.ndt.coverage") != Some("complete_synthetic")
    {
        return "NDT_SCOPE_OR_COVERAGE_MISMATCH";
    }
    if text_at(weld, "inspections.visual.disposition") != Some("accepted_synthetic")
        || text_at(weld, "inspections.visual.joint_id") != Some(joint)
        || text_at(weld, "inspections.visual.coverage") != Some("complete_synthetic")
    {
        return "VISUAL_INSPECTION_NOT_ACCEPTED";
    }

    if bool_at(weld, "post_processing.repair_required") == Some(true)
        && (text_at(weld, "post_processing.repair_route_ref").is_none_or(str::is_empty)
            || text_at(weld, "post_processing.repair_reinspection_ref")
                .is_none_or(str::is_empty))
    {
        return "REPAIR_REINSPECTION_MISSING";
    }
    if bool_at(weld, "post_processing.straightening_required") == Some(true)
        && text_at(weld, "post_processing.dimensional_survey_ref")
            .is_none_or(str::is_empty)
    {
        return "STRAIGHTENING_EVIDENCE_MISSING";
    }

    if text_at(weld, "document_package.status") != Some("complete_synthetic") {
        return "DOCUMENT_PACKAGE_INCOMPLETE";
    }
    let required = value_at(weld, "document_package.required_references")
        .and_then(Value::as_array);
    let included = value_at(weld, "document_package.included_references")
        .and_then(Value::as_array);
    let (Some(required), Some(included)) = (required, included) else {
        return "DOCUMENT_PACKAGE_INCOMPLETE";
    };
    if required.iter().any(|item| !included.contains(item)) {
        return "DOCUMENT_PACKAGE_INCOMPLETE";
    }

    if number_at(weld, "twin_commissioning.observed_latency_ms")
        .zip(number_at(weld, "twin_commissioning.max_latency_ms"))
        .is_none_or(|(observed, maximum)| observed > maximum)
    {
        return "TWIN_LATENCY_EXCEEDED";
    }
    if number_at(weld, "twin_commissioning.observed_update_interval_s")
        .zip(number_at(weld, "twin_commissioning.expected_update_interval_s"))
        .is_none_or(|(observed, expected)| observed > expected)
    {
        return "TWIN_UPDATE_FREQUENCY_EXCEEDED";
    }
    if bool_at(weld, "twin_commissioning.source_writeback") != Some(false) {
        return "TWIN_WRITEBACK_FORBIDDEN";
    }
    if !matches!(
        text_at(weld, "twin_commissioning.mapping_state"),
        Some("exact" | "normalized")
    ) || text_at(weld, "twin_commissioning.observed_fidelity")
        != Some("exact_required_fields")
    {
        return "TWIN_FIDELITY_INSUFFICIENT";
    }

    "WELD_TRACE_VALID_SYNTHETIC_ONLY"
}

fn apply_mutation(root: &mut Value, mutation: &Value) -> Result<(), String> {
    let path = mutation
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| "mutation has no path".to_owned())?;
    let op = mutation
        .get("op")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("mutation {path} has no operation"))?;
    let parts: Vec<&str> = path.split('.').collect();
    if parts.is_empty() {
        return Err("empty mutation path".to_owned());
    }
    let mut current = root;
    for part in &parts[..parts.len() - 1] {
        current = match current {
            Value::Object(object) => object
                .get_mut(*part)
                .ok_or_else(|| format!("missing mutation path prefix {part}"))?,
            Value::Array(array) => array
                .get_mut(part.parse::<usize>().map_err(|_| {
                    format!("invalid array index in mutation path {path}")
                })?)
                .ok_or_else(|| format!("array index missing in mutation path {path}"))?,
            _ => return Err(format!("non-container in mutation path {path}")),
        };
    }
    let leaf = parts[parts.len() - 1];
    match current {
        Value::Object(object) => match op {
            "set" => {
                let value = mutation
                    .get("value")
                    .ok_or_else(|| format!("set mutation {path} has no value"))?;
                let slot = object
                    .get_mut(leaf)
                    .ok_or_else(|| format!("missing mutation leaf {path}"))?;
                *slot = value.clone();
                Ok(())
            }
            "remove" => object
                .remove(leaf)
                .map(|_| ())
                .ok_or_else(|| format!("missing mutation leaf {path}")),
            _ => Err(format!("unsupported mutation operation {op}")),
        },
        Value::Array(array) => {
            let index = leaf
                .parse::<usize>()
                .map_err(|_| format!("invalid array leaf {path}"))?;
            match op {
                "set" => {
                    let value = mutation
                        .get("value")
                        .ok_or_else(|| format!("set mutation {path} has no value"))?;
                    let slot = array
                        .get_mut(index)
                        .ok_or_else(|| format!("missing array leaf {path}"))?;
                    *slot = value.clone();
                    Ok(())
                }
                "remove" => {
                    if index >= array.len() {
                        return Err(format!("missing array leaf {path}"));
                    }
                    array.remove(index);
                    Ok(())
                }
                _ => Err(format!("unsupported mutation operation {op}")),
            }
        }
        _ => Err(format!("non-container at mutation leaf {path}")),
    }
}

fn verify_corpus(corpus: &Value, workspace_root: &Path) -> Result<Value, String> {
    let base = corpus
        .get("base_cases")
        .and_then(|value| value.get("valid_robotic_weld"))
        .ok_or_else(|| "corpus has no valid_robotic_weld base case".to_owned())?;
    let cases = corpus
        .get("cases")
        .and_then(Value::as_array)
        .ok_or_else(|| "corpus cases must be an array".to_owned())?;

    let mut results = Vec::with_capacity(cases.len());
    let mut failures = Vec::new();
    for case in cases {
        let id = case
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| "case has no id".to_owned())?;
        let expected = case
            .get("expected_code")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("case {id} has no expected_code"))?;
        let mutations = case
            .get("mutations")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("case {id} mutations must be an array"))?;
        let mut input = base.clone();
        for mutation in mutations {
            apply_mutation(&mut input, mutation)
                .map_err(|error| format!("case {id}: {error}"))?;
        }
        let observed = evaluate(&input, workspace_root);
        let passed = observed == expected;
        if !passed {
            failures.push(json!({
                "case_id": id,
                "expected": expected,
                "observed": observed
            }));
        }
        results.push(json!({
            "case_id": id,
            "expected": expected,
            "observed": observed,
            "passed": passed
        }));
    }

    Ok(json!({
        "status": if failures.is_empty() { "PASS_SYNTHETIC_CORPUS" } else { "FAIL_SYNTHETIC_CORPUS" },
        "corpus_id": corpus.get("corpus_id").and_then(Value::as_str),
        "corpus_version": corpus.get("corpus_version").and_then(Value::as_str),
        "case_count": cases.len(),
        "passed_count": results.iter().filter(|item| item["passed"] == true).count(),
        "failed_count": failures.len(),
        "failures": failures,
        "results": results,
        "claim_ceiling": corpus.get("claim_ceiling").and_then(Value::as_str)
    }))
}

fn main() -> ExitCode {
    let path = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from("benchmarks/shipyard/weld-twin-corpus-v0.1.json")
        });
    let bytes = match fs::read(&path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot read {}: {error}", path.display());
            return ExitCode::FAILURE;
        }
    };
    let corpus: Value = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid corpus JSON: {error}");
            return ExitCode::FAILURE;
        }
    };
    let workspace_root = match env::current_dir() {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot resolve workspace root: {error}");
            return ExitCode::FAILURE;
        }
    };
    let report = match verify_corpus(&corpus, &workspace_root) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("corpus verification could not run: {error}");
            return ExitCode::FAILURE;
        }
    };
    println!("{}", serde_json::to_string_pretty(&report).expect("JSON report serializes"));
    if report["failed_count"].as_u64() == Some(0) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::{apply_mutation, evaluate, is_canonical_utc, verify_corpus};
    use serde_json::Value;
    use std::path::PathBuf;

    const CORPUS: &str =
        include_str!("../../../../benchmarks/shipyard/weld-twin-corpus-v0.1.json");

    fn workspace_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
    }

    #[test]
    fn frozen_corpus_matches_all_expected_dispositions() {
        let corpus: Value = serde_json::from_str(CORPUS).expect("frozen corpus parses");
        let root = workspace_root();
        let report = verify_corpus(&corpus, &root).expect("frozen corpus evaluates");
        assert_eq!(report["case_count"].as_u64(), Some(49));
        assert_eq!(report["passed_count"].as_u64(), Some(49));
        assert_eq!(report["failed_count"].as_u64(), Some(0));
    }

    #[test]
    fn canonical_utc_validates_calendar_and_clock_components() {
        assert!(is_canonical_utc("2024-02-29T23:59:59Z"));
        assert!(is_canonical_utc("2026-11-10T04:00:00Z"));
        assert!(!is_canonical_utc("2026-02-30T04:00:00Z"));
        assert!(!is_canonical_utc("2025-02-29T04:00:00Z"));
        assert!(!is_canonical_utc("2026-13-01T04:00:00Z"));
        assert!(!is_canonical_utc("2026-11-10T24:00:00Z"));
        assert!(!is_canonical_utc("2026-11-10T04:00:00"));
    }

    #[test]
    fn valid_base_is_not_a_physical_acceptance() {
        let corpus: Value = serde_json::from_str(CORPUS).expect("frozen corpus parses");
        let base = corpus["base_cases"]["valid_robotic_weld"].clone();
        assert_eq!(
            evaluate(&base, &workspace_root()),
            "WELD_TRACE_VALID_SYNTHETIC_ONLY"
        );
    }

    #[test]
    fn mutation_engine_changes_only_the_requested_field() {
        let mut value = serde_json::json!({"passes":[{"role":"root"},{"role":"fill"}]});
        let mutation = serde_json::json!({
            "path":"passes.1.role",
            "op":"set",
            "value":"cap"
        });
        apply_mutation(&mut value, &mutation).expect("mutation path resolves");
        assert_eq!(value["passes"][0]["role"], "root");
        assert_eq!(value["passes"][1]["role"], "cap");
    }
}
