//! Synthetic reference oracle for reversible panel disassembly and reuse.
//! It validates a bounded evidence chain; it does not compute robot motion,
//! removal forces, structural stability, safety interlocks or material truth.

use serde_json::{Value, json};
use std::collections::BTreeSet;

fn text<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or("")
}
fn number(v: &Value, key: &str) -> Option<f64> {
    v.get(key).and_then(Value::as_f64).filter(|n| n.is_finite())
}
fn array<'a>(v: &'a Value, key: &str) -> Option<&'a [Value]> {
    v.get(key).and_then(Value::as_array).map(Vec::as_slice)
}
fn contains(values: &[Value], needle: &str) -> bool {
    values.iter().any(|v| v.as_str() == Some(needle))
}
fn empty(case_id: &str, disposition: &str, reasons: BTreeSet<String>) -> Value {
    json!({
        "case_id": case_id,
        "disposition": disposition,
        "reason_codes": reasons.into_iter().collect::<Vec<_>>(),
        "removal": "NotEvaluated",
        "component": "NotEvaluated",
        "reassembly": "NotEvaluated",
        "shared_resource": "NotEvaluated",
        "final_state": disposition
    })
}

fn validate(input: &Value) -> Result<(), BTreeSet<String>> {
    let mut errors = BTreeSet::new();
    if text(input, "scope_digest").is_empty() {
        errors.insert("SCOPE_DIGEST_MISSING".to_owned());
    }
    let panel = input.get("panel").unwrap_or(&Value::Null);
    let connection = input.get("connection").unwrap_or(&Value::Null);
    let tool = input.get("tool").unwrap_or(&Value::Null);
    let component = input.get("component").unwrap_or(&Value::Null);
    let resources = input.get("shared_resources").unwrap_or(&Value::Null);

    if text(panel, "panel_id").is_empty()
        || text(panel, "design_revision").is_empty()
        || text(panel, "bom_digest").is_empty()
        || text(component, "component_id").is_empty()
        || array(panel, "component_ids").is_none()
    {
        errors.insert("PANEL_OR_COMPONENT_IDENTITY_INVALID".to_owned());
    }
    for (value, key) in [
        (connection, "removal_force_n"),
        (connection, "declared_limit_n"),
        (resources, "capacity"),
    ] {
        if number(value, key).is_none_or(|n| n < 0.0) {
            errors.insert("NONNEGATIVE_QUANTITY_INVALID".to_owned());
        }
    }
    let Some(compatible) = array(tool, "compatible_types") else {
        errors.insert("TOOL_COMPATIBILITY_MISSING".to_owned());
        return Err(errors);
    };
    if compatible.iter().any(|v| v.as_str().is_none()) {
        errors.insert("TOOL_COMPATIBILITY_ENTRY_INVALID".to_owned());
    }
    let Some(claims) = array(resources, "claims") else {
        errors.insert("SHARED_RESOURCE_CLAIMS_MISSING".to_owned());
        return Err(errors);
    };
    let mut claim_ids = BTreeSet::<String>::new();
    for claim in claims {
        let id = text(claim, "claim_id");
        if id.is_empty() || !claim_ids.insert(id.to_owned()) {
            errors.insert("DUPLICATE_OR_MISSING_SHARED_RESOURCE_CLAIM_ID".to_owned());
        }
        if number(claim, "use").is_none_or(|n| n < 0.0) {
            errors.insert("SHARED_RESOURCE_USE_INVALID".to_owned());
        }
    }

    let state_fields = [
        (
            panel,
            "evidence_state",
            &["current", "stale", "unknown"][..],
        ),
        (tool, "evidence_state", &["current", "stale", "unknown"][..]),
        (
            tool,
            "calibration_state",
            &["current", "stale", "unknown"][..],
        ),
        (
            connection,
            "sequence_state",
            &["current", "stale", "unknown"][..],
        ),
        (
            connection,
            "access_state",
            &["clear", "occluded", "blocked"][..],
        ),
        (
            connection,
            "release_process_state",
            &["qualified", "unknown", "unqualified"][..],
        ),
        (
            component,
            "condition_evidence_state",
            &["current", "stale", "unknown"][..],
        ),
        (
            input.get("repair").unwrap_or(&Value::Null),
            "evidence_state",
            &["current", "stale", "unknown"][..],
        ),
        (
            input.get("repair").unwrap_or(&Value::Null),
            "process_state",
            &["not_required", "qualified", "unknown", "unqualified"][..],
        ),
        (
            input.get("repair").unwrap_or(&Value::Null),
            "result",
            &["not_required", "passed", "failed", "pending"][..],
        ),
        (
            input.get("requalification").unwrap_or(&Value::Null),
            "evidence_state",
            &["current", "stale", "unknown"][..],
        ),
        (
            input.get("requalification").unwrap_or(&Value::Null),
            "result",
            &["passed", "failed", "pending"][..],
        ),
        (
            input.get("reassembly").unwrap_or(&Value::Null),
            "evidence_state",
            &["current", "stale", "unknown"][..],
        ),
        (
            resources,
            "evidence_state",
            &["current", "stale", "unknown"][..],
        ),
    ];
    for (value, key, allowed) in state_fields {
        if !allowed.contains(&text(value, key)) {
            errors.insert(format!("STATE_FIELD_UNSUPPORTED:{key}"));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Recompute one synthetic episode from input facts only. The expected output
/// is never read and cannot influence the decision.
pub fn evaluate_case(case: &Value) -> Value {
    let case_id = text(case, "case_id");
    let input = case.get("input").unwrap_or(&Value::Null);
    if let Err(errors) = validate(input) {
        return empty(case_id, "Unsupported", errors);
    }

    let panel = input.get("panel").unwrap_or(&Value::Null);
    let connection = input.get("connection").unwrap_or(&Value::Null);
    let tool = input.get("tool").unwrap_or(&Value::Null);
    let component = input.get("component").unwrap_or(&Value::Null);
    let repair = input.get("repair").unwrap_or(&Value::Null);
    let requalification = input.get("requalification").unwrap_or(&Value::Null);
    let reassembly_input = input.get("reassembly").unwrap_or(&Value::Null);
    let resources = input.get("shared_resources").unwrap_or(&Value::Null);
    let mut reasons = BTreeSet::<String>::new();

    let mut removal = "Feasible";
    if text(panel, "evidence_state") != "current"
        || text(tool, "evidence_state") != "current"
        || text(tool, "calibration_state") != "current"
        || text(connection, "sequence_state") != "current"
        || text(component, "condition_evidence_state") != "current"
    {
        removal = "Unknown";
        reasons.insert("REMOVAL_PREREQUISITE_EVIDENCE_UNKNOWN".to_owned());
    }
    if text(connection, "access_state") != "clear" {
        if text(connection, "access_state") == "occluded" {
            removal = "Unknown";
            reasons.insert("CONNECTION_NOT_OBSERVED".to_owned());
        } else {
            removal = "Blocked";
            reasons.insert("CONNECTION_ACCESS_BLOCKED".to_owned());
        }
    }
    if text(connection, "type") != text(connection, "observed_type") {
        removal = "NotComparable";
        reasons.insert("AS_BUILT_CONNECTION_TYPE_MISMATCH".to_owned());
    }
    let compatible = array(tool, "compatible_types").expect("validated tool compatibility");
    if !contains(compatible, text(connection, "observed_type")) {
        removal = "Blocked";
        reasons.insert("TOOL_CONNECTION_INCOMPATIBLE".to_owned());
    }
    if text(tool, "calibration_state") != "current" || text(tool, "evidence_state") != "current" {
        removal = "Unknown";
        reasons.insert("TOOL_EVIDENCE_UNKNOWN".to_owned());
    }
    if text(connection, "release_process_state") != "qualified" {
        removal = "Unknown";
        reasons.insert("CONNECTION_RELEASE_PROCESS_NOT_QUALIFIED".to_owned());
    }
    if number(connection, "removal_force_n").unwrap_or(f64::INFINITY)
        > number(connection, "declared_limit_n").unwrap_or(0.0)
    {
        removal = "Blocked";
        reasons.insert("REMOVAL_LIMIT_EXCEEDED".to_owned());
    }
    if text(connection, "support_state") == "required_missing" {
        removal = "Blocked";
        reasons.insert("REQUIRED_TEMPORARY_SUPPORT_MISSING".to_owned());
    }
    let component_ids = array(panel, "component_ids").expect("validated BOM component list");
    if !contains(component_ids, text(component, "component_id")) {
        removal = "NotComparable";
        reasons.insert("COMPONENT_ID_NOT_IN_RELEASED_BOM".to_owned());
    }

    let mut component_state = "ReusableCandidate";
    if removal == "Unknown" {
        component_state = "Unknown";
    } else if matches!(removal, "Blocked" | "NotComparable") {
        component_state = "NotEvaluated";
    } else if matches!(
        text(component, "after_removal_condition"),
        "adjacent_damage" | "dimensions_failed" | "wear_limit_exceeded"
    ) {
        component_state = "Rejected";
        reasons.insert("COMPONENT_CONDITION_FAILED".to_owned());
    } else if component
        .get("single_use")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && component
            .get("intended_reuse")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    {
        component_state = "Rejected";
        reasons.insert("SINGLE_USE_COMPONENT_REUSE_PROHIBITED".to_owned());
    } else if repair
        .get("required")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        if text(repair, "evidence_state") != "current" || text(repair, "process_state") == "unknown"
        {
            component_state = "Unknown";
            reasons.insert("REPAIR_PROCESS_EVIDENCE_UNKNOWN".to_owned());
        } else if text(repair, "process_state") != "qualified" {
            component_state = "RepairRequired";
            reasons.insert("QUALIFIED_REPAIR_PROCESS_MISSING".to_owned());
        } else if text(repair, "result") == "failed" {
            component_state = "Rejected";
            reasons.insert("REPAIR_ACCEPTANCE_FAILED".to_owned());
        } else if text(repair, "result") != "passed" {
            component_state = "RepairRequired";
            reasons.insert("REPAIR_PENDING".to_owned());
        } else if text(requalification, "evidence_state") != "current"
            || text(requalification, "result") == "pending"
        {
            component_state = "Unknown";
            reasons.insert("REQUALIFICATION_PENDING_OR_UNKNOWN".to_owned());
        } else if text(requalification, "result") == "failed" {
            component_state = "Rejected";
            reasons.insert("REQUALIFICATION_FAILED".to_owned());
        }
    } else if text(requalification, "evidence_state") != "current"
        || text(requalification, "result") == "pending"
    {
        component_state = "Unknown";
        reasons.insert("REQUALIFICATION_PENDING_OR_UNKNOWN".to_owned());
    } else if text(requalification, "result") == "failed" {
        component_state = "Rejected";
        reasons.insert("REQUALIFICATION_FAILED".to_owned());
    }

    let shared_resource = if text(resources, "evidence_state") != "current" {
        reasons.insert("SHARED_RESOURCE_EVIDENCE_UNKNOWN".to_owned());
        "Unknown"
    } else {
        let total = array(resources, "claims")
            .expect("validated resource claims")
            .iter()
            .map(|claim| number(claim, "use").unwrap_or(0.0))
            .sum::<f64>();
        if total > number(resources, "capacity").unwrap_or(0.0) {
            reasons.insert("SHARED_RESOURCE_OVERBOOKED".to_owned());
            "Blocked"
        } else {
            "Feasible"
        }
    };

    let mut reassembly = "NotEvaluated";
    if component_state == "ReusableCandidate" {
        reassembly = match shared_resource {
            "Blocked" => "Blocked",
            "Unknown" => "Unknown",
            _ => "Accepted",
        };
        if text(reassembly_input, "evidence_state") != "current" {
            reassembly = "Unknown";
            reasons.insert("REASSEMBLY_EVIDENCE_UNKNOWN".to_owned());
        }
        for (field, expected, failure) in [
            ("fastening_state", "passed", "REASSEMBLY_FASTENING_FAILED"),
            ("torque_state", "in_range", "REASSEMBLY_TORQUE_OUT_OF_RANGE"),
            ("geometry_state", "passed", "REASSEMBLY_GEOMETRY_FAILED"),
            ("function_state", "passed", "REASSEMBLY_FUNCTION_FAILED"),
        ] {
            let state = text(reassembly_input, field);
            if state == "unknown" {
                reassembly = "Unknown";
                reasons.insert(format!("{failure}_UNKNOWN"));
            } else if state != expected {
                reassembly = "Rejected";
                reasons.insert(failure.to_owned());
            }
        }
    }

    let final_state = if removal == "NotComparable" {
        "NotComparable"
    } else if removal == "Unknown" || component_state == "Unknown" || reassembly == "Unknown" {
        "Unknown"
    } else if removal == "Blocked" || shared_resource == "Blocked" || reassembly == "Blocked" {
        "Blocked"
    } else if component_state == "Rejected" || reassembly == "Rejected" {
        "Rejected"
    } else if component_state == "RepairRequired" {
        "RepairRequired"
    } else if reassembly == "Accepted" {
        "AcceptedReusableAssembly"
    } else {
        "NotEvaluated"
    };

    json!({
        "case_id": case_id,
        "disposition": "Evaluated",
        "reason_codes": reasons.into_iter().collect::<Vec<_>>(),
        "removal": removal,
        "component": component_state,
        "reassembly": reassembly,
        "shared_resource": shared_resource,
        "final_state": final_state
    })
}

fn equivalent(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Number(left), Value::Number(right)) => left
            .as_f64()
            .zip(right.as_f64())
            .is_some_and(|(left, right)| left == right),
        (Value::Array(left), Value::Array(right)) => {
            left.len() == right.len()
                && left.iter().zip(right.iter()).all(|(a, b)| equivalent(a, b))
        }
        (Value::Object(left), Value::Object(right)) => {
            left.len() == right.len()
                && right.iter().all(|(key, value)| {
                    left.get(key)
                        .is_some_and(|actual| equivalent(actual, value))
                })
        }
        _ => actual == expected,
    }
}

pub fn matches_expected(actual: &Value, expected: &Value) -> bool {
    expected.as_object().is_some_and(|fields| {
        fields.iter().all(|(key, value)| {
            actual
                .get(key)
                .is_some_and(|actual| equivalent(actual, value))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::{evaluate_case, matches_expected};
    use serde_json::Value;
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    fn corpus() -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/research/panel-disassembly/PANEL_DISASSEMBLY_CORPUS_V0_1.json");
        let bytes = std::fs::read(path).expect("panel disassembly corpus must exist");
        serde_json::from_slice(&bytes).expect("panel disassembly corpus must parse")
    }

    fn case(id: &str) -> Value {
        corpus()["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["case_id"] == id)
            .unwrap()
            .clone()
    }

    #[test]
    fn corpus_has_24_unique_synthetic_cases() {
        let c = corpus();
        assert_eq!(c["classification"], "synthetic_fixture_only");
        assert_eq!(c["corpus_id"], "PANEL-DISASSEMBLY-CORPUS-V0.1");
        let cases = c["cases"].as_array().unwrap();
        assert_eq!(cases.len(), 24);
        let ids = cases
            .iter()
            .map(|x| x["case_id"].as_str().unwrap().to_owned())
            .collect::<BTreeSet<_>>();
        assert_eq!(ids.len(), cases.len());
    }

    #[test]
    fn independent_oracle_matches_every_known_answer() {
        for item in corpus()["cases"].as_array().unwrap() {
            let actual = evaluate_case(item);
            assert!(
                matches_expected(&actual, &item["expected"]),
                "{} mismatch\nactual: {}\nexpected: {}",
                item["case_id"],
                serde_json::to_string_pretty(&actual).unwrap(),
                serde_json::to_string_pretty(&item["expected"]).unwrap()
            );
        }
    }

    #[test]
    fn unknown_or_wrong_identity_never_becomes_reusable() {
        for id in [
            "PD-002", "PD-008", "PD-012", "PD-013", "PD-016", "PD-021", "PD-022",
        ] {
            assert_ne!(
                evaluate_case(&case(id))["final_state"],
                "AcceptedReusableAssembly"
            );
        }
        assert_eq!(
            evaluate_case(&case("PD-011"))["final_state"],
            "NotComparable"
        );
    }

    #[test]
    fn damaged_single_use_or_failed_components_are_rejected() {
        for id in ["PD-014", "PD-015", "PD-017", "PD-020", "PD-023"] {
            assert_eq!(evaluate_case(&case(id))["final_state"], "Rejected");
        }
    }

    #[test]
    fn removal_limit_tool_and_support_failures_block() {
        for id in ["PD-003", "PD-006", "PD-009", "PD-010"] {
            assert_eq!(evaluate_case(&case(id))["final_state"], "Blocked");
        }
    }

    #[test]
    fn repair_requalification_and_reassembly_are_distinct() {
        assert_eq!(evaluate_case(&case("PD-018"))["final_state"], "Unknown");
        assert_eq!(
            evaluate_case(&case("PD-019"))["final_state"],
            "AcceptedReusableAssembly"
        );
        assert_eq!(evaluate_case(&case("PD-021"))["reassembly"], "Unknown");
        assert_eq!(evaluate_case(&case("PD-022"))["reassembly"], "Unknown");
    }

    #[test]
    fn shared_fixture_overbooking_blocks_reassembly() {
        let result = evaluate_case(&case("PD-024"));
        assert_eq!(result["shared_resource"], "Blocked");
        assert_eq!(result["final_state"], "Blocked");
    }

    #[test]
    fn resource_claim_permutation_does_not_change_result() {
        let mut item = case("PD-024");
        let baseline = evaluate_case(&item);
        item["input"]["shared_resources"]["claims"]
            .as_array_mut()
            .unwrap()
            .reverse();
        assert_eq!(baseline, evaluate_case(&item));
    }
}
