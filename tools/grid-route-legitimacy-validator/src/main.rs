use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process;

fn array<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn string<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}

fn strings(value: &Value, key: &str) -> Vec<&str> {
    array(value, key).iter().filter_map(Value::as_str).collect()
}

fn ids(
    items: &[Value],
    field: &str,
    label: &str,
    errors: &mut Vec<String>,
) -> BTreeSet<String> {
    let mut result = BTreeSet::new();
    for item in items {
        let id = string(item, field);
        if id.is_empty() {
            errors.push(format!("{label} has an empty {field}"));
        } else if !result.insert(id.to_string()) {
            errors.push(format!("duplicate {label} id: {id}"));
        }
    }
    result
}

fn evidence_exists(
    evidence_ids: &[&str],
    known: &BTreeSet<String>,
    owner: &str,
    errors: &mut Vec<String>,
) {
    for evidence_id in evidence_ids {
        if !known.contains(*evidence_id) {
            errors.push(format!(
                "{owner} references unknown evidence {evidence_id}"
            ));
        }
    }
}

fn gate_decision_type_compatible(gate_type: &str, decision_type: &str) -> bool {
    match gate_type {
        "public_participation" => decision_type == "consultation_record",
        "land_access_and_servitude" => {
            matches!(decision_type, "land_access" | "servitude")
        }
        "statutory_decisions" => matches!(
            decision_type,
            "statutory_authorisation"
                | "environmental_authorisation"
                | "other_authority_decision"
        ),
        "compensation_and_mitigation" => decision_type == "compensation_agreement",
        _ => true,
    }
}

fn gate_requires_decision(gate_type: &str) -> bool {
    matches!(
        gate_type,
        "public_participation"
            | "land_access_and_servitude"
            | "statutory_decisions"
            | "compensation_and_mitigation"
    )
}

fn validate(case: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let maturity = string(case, "caseMaturity");
    let snapshot_date = string(case, "snapshotDate");
    if snapshot_date.is_empty() {
        errors.push("snapshotDate is required".to_string());
    }
    let allowed_maturity = [
        "synthetic_fixture",
        "engineering_estimate",
        "field_observed",
        "independently_reviewed",
    ];
    if !allowed_maturity.contains(&maturity) {
        errors.push("unsupported caseMaturity".to_string());
    }

    let stakeholders = array(case, "stakeholders");
    let evidence = array(case, "evidenceRecords");
    let decisions = array(case, "decisions");
    let gates = array(case, "gates");
    let engagements = array(case, "engagements");
    let impacts = array(case, "impacts");
    let workfronts = array(case, "workfronts");

    let stakeholder_ids = ids(stakeholders, "stakeholderId", "stakeholder", &mut errors);
    let evidence_ids = ids(evidence, "evidenceId", "evidence", &mut errors);
    let decision_ids = ids(decisions, "decisionId", "decision", &mut errors);
    let gate_ids = ids(gates, "gateId", "gate", &mut errors);
    let engagement_ids = ids(engagements, "engagementId", "engagement", &mut errors);
    let impact_ids = ids(impacts, "impactId", "impact", &mut errors);
    let workfront_ids = ids(workfronts, "workfrontId", "workfront", &mut errors);
    let mut evidence_classes = BTreeMap::new();

    for record in evidence {
        let evidence_id = string(record, "evidenceId");
        let class = string(record, "evidenceClass");
        evidence_classes.insert(evidence_id.to_string(), class.to_string());

        if string(record, "description").trim().is_empty() {
            errors.push(format!("{evidence_id} has empty evidence description"));
        }
        if maturity == "synthetic_fixture" && class != "synthetic_fixture" {
            errors.push(format!(
                "synthetic case promotes {evidence_id} to non-synthetic evidence"
            ));
        }
        if maturity != "synthetic_fixture" && class == "synthetic_fixture" {
            errors.push(format!(
                "non-synthetic case relies on synthetic evidence {evidence_id}"
            ));
        }
        if maturity != "synthetic_fixture"
            && record
                .get("sourceUri")
                .and_then(Value::as_str)
                .is_none_or(str::is_empty)
        {
            errors.push(format!(
                "{evidence_id} requires sourceUri outside synthetic fixtures"
            ));
        }
    }

    for decision in decisions {
        let id = string(decision, "decisionId");
        for party in strings(decision, "partyStakeholderIds") {
            if !stakeholder_ids.contains(party) {
                errors.push(format!("{id} references unknown stakeholder {party}"));
            }
        }
        evidence_exists(
            &strings(decision, "evidenceIds"),
            &evidence_ids,
            id,
            &mut errors,
        );

        let status = string(decision, "status");
        if matches!(status, "granted" | "executed") {
            if array(decision, "evidenceIds").is_empty() {
                errors.push(format!("{id} is {status} without evidence"));
            }
            let effective_from = string(decision, "effectiveFrom");
            if effective_from.is_empty() {
                errors.push(format!("{id} is {status} without effectiveFrom"));
            } else if !snapshot_date.is_empty() && effective_from > snapshot_date {
                errors.push(format!("{id} is not effective at snapshotDate"));
            }
        }
        let expires_on = string(decision, "expiresOn");
        if !expires_on.is_empty() && !snapshot_date.is_empty() && expires_on < snapshot_date
            && status != "expired"
        {
            errors.push(format!("{id} expired before snapshotDate but status is {status}"));
        }
        if status == "expired" && expires_on.is_empty() {
            errors.push(format!("{id} is marked expired without expiresOn"));
        }
        if maturity != "synthetic_fixture" && matches!(status, "granted" | "executed") {
            let accepted = strings(decision, "evidenceIds").iter().any(|evidence_id| {
                matches!(
                    evidence_classes.get(*evidence_id).map(String::as_str),
                    Some("authority_decision" | "executed_instrument")
                )
            });
            if !accepted {
                errors.push(format!(
                    "{id} lacks authority-decision or executed-instrument evidence"
                ));
            }
        }
    }

    for gate in gates {
        let id = string(gate, "gateId");
        let status = string(gate, "status");
        evidence_exists(
            &strings(gate, "evidenceIds"),
            &evidence_ids,
            id,
            &mut errors,
        );

        if status == "complete" {
            if array(gate, "evidenceIds").is_empty() {
                errors.push(format!("{id} is complete without evidence"));
            }
            if string(gate, "accountableRole").trim().is_empty() {
                errors.push(format!("{id} is complete without accountableRole"));
            }
        }
        if status == "not_applicable"
            && string(gate, "applicabilityRationale").trim().is_empty()
        {
            errors.push(format!("{id} is not_applicable without rationale"));
        }

        let gate_type = string(gate, "gateType");
        let required_type = string(gate, "requiredDecisionType");
        let decision_id = string(gate, "decisionId");
        if !required_type.is_empty() && !gate_decision_type_compatible(gate_type, required_type) {
            errors.push(format!(
                "{id} gate type {gate_type} cannot be satisfied by decision type {required_type}"
            ));
        }
        if status == "complete" && gate_requires_decision(gate_type) && required_type.is_empty() {
            errors.push(format!("{id} cannot be complete without an explicit requiredDecisionType"));
        }
        if !required_type.is_empty() {
            if decision_id.is_empty() {
                if status == "complete" {
                    errors.push(format!(
                        "{id} is complete but lacks required decisionId"
                    ));
                }
            } else if let Some(decision) = decisions
                .iter()
                .find(|item| string(item, "decisionId") == decision_id)
            {
                if string(decision, "decisionType") != required_type {
                    errors.push(format!(
                        "{id} requires decision type {required_type}, but {decision_id} is {}",
                        string(decision, "decisionType")
                    ));
                }
                let acceptable = match required_type {
                    "statutory_authorisation"
                    | "environmental_authorisation"
                    | "other_authority_decision" => string(decision, "status") == "granted",
                    "land_access"
                    | "servitude"
                    | "compensation_agreement"
                    | "community_agreement" => string(decision, "status") == "executed",
                    "consultation_record" => {
                        matches!(string(decision, "status"), "granted" | "executed")
                    }
                    _ => false,
                };
                if status == "complete" && !acceptable {
                    errors.push(format!(
                        "{id} cannot be complete while {decision_id} status is {}",
                        string(decision, "status")
                    ));
                }
            } else {
                errors.push(format!(
                    "{id} references unknown decision {decision_id}"
                ));
            }
        }
    }

    for engagement in engagements {
        let id = string(engagement, "engagementId");
        for stakeholder_id in strings(engagement, "stakeholderIds") {
            if !stakeholder_ids.contains(stakeholder_id) {
                errors.push(format!(
                    "{id} references unknown stakeholder {stakeholder_id}"
                ));
            }
        }
        evidence_exists(
            &strings(engagement, "noticeEvidenceIds"),
            &evidence_ids,
            id,
            &mut errors,
        );

        if string(engagement, "windowEnd") < string(engagement, "windowStart") {
            errors.push(format!("{id} has windowEnd before windowStart"));
        }
        for concern in array(engagement, "concerns") {
            let concern_id = string(concern, "concernId");
            let response_evidence = strings(concern, "responseEvidenceIds");
            evidence_exists(
                &response_evidence,
                &evidence_ids,
                &format!("{id} concern {concern_id}"),
                &mut errors,
            );
            if matches!(string(concern, "status"), "responded" | "resolved" | "referred")
                && response_evidence.is_empty()
            {
                errors.push(format!(
                    "{id} concern {concern_id} has a response state without response evidence"
                ));
            }
        }
    }

    for impact in impacts {
        let id = string(impact, "impactId");
        evidence_exists(
            &strings(impact, "evidenceIds"),
            &evidence_ids,
            id,
            &mut errors,
        );
    }

    for workfront in workfronts {
        let id = string(workfront, "workfrontId");
        for gate_id in strings(workfront, "requiredGateIds") {
            if !gate_ids.contains(gate_id) {
                errors.push(format!(
                    "{id} references unknown required gate {gate_id}"
                ));
            }
        }
        evidence_exists(
            &strings(workfront, "releaseEvidenceIds"),
            &evidence_ids,
            id,
            &mut errors,
        );

        if string(workfront, "status") == "released" {
            for gate_id in strings(workfront, "requiredGateIds") {
                let complete = gates.iter().any(|gate| {
                    string(gate, "gateId") == gate_id
                        && string(gate, "status") == "complete"
                });
                if !complete {
                    errors.push(format!(
                        "{id} is released while required gate {gate_id} is not complete"
                    ));
                }
            }
            if array(workfront, "releaseEvidenceIds").is_empty() {
                errors.push(format!("{id} is released without release evidence"));
            }
            if maturity == "synthetic_fixture" {
                errors.push(format!(
                    "{id} cannot be represented as released in a synthetic fixture"
                ));
            } else {
                let release_supported = strings(workfront, "releaseEvidenceIds")
                    .iter()
                    .any(|evidence_id| {
                        matches!(
                            evidence_classes.get(*evidence_id).map(String::as_str),
                            Some("authority_decision" | "executed_instrument")
                        )
                    });
                if !release_supported {
                    errors.push(format!(
                        "{id} lacks authority/instrument release evidence"
                    ));
                }
            }
        }
    }

    // Keep this reference set live as a defence against accidental empty-ID drift.
    if decision_ids.len() != decisions.len()
        || engagement_ids.len() != engagements.len()
        || impact_ids.len() != impacts.len()
        || workfront_ids.len() != workfronts.len()
    {
        errors.push("one or more entity ID sets are incomplete".to_string());
    }

    errors.sort();
    errors
}

fn main() {
    let Some(path) = env::args().nth(1) else {
        eprintln!("usage: grid-route-legitimacy-validator <case.json>");
        process::exit(2);
    };
    let input = fs::read_to_string(&path).unwrap_or_else(|error| {
        eprintln!("cannot read {path}: {error}");
        process::exit(2);
    });
    let case: Value = serde_json::from_str(&input).unwrap_or_else(|error| {
        eprintln!("invalid JSON: {error}");
        process::exit(2);
    });
    let errors = validate(&case);
    if errors.is_empty() {
        println!("PASS: route-legitimacy evidence graph is semantically coherent");
    } else {
        for error in errors {
            eprintln!("FAIL: {error}");
        }
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!(
        "../../../docs/research/grid-route-legitimacy/data/synthetic-route-legitimacy-v0.1.json"
    );

    fn fixture() -> Value {
        serde_json::from_str(FIXTURE).expect("fixture JSON")
    }

    #[test]
    fn accepts_synthetic_unreleased_workfront() {
        let case = fixture();
        assert!(validate(&case).is_empty());
    }

    #[test]
    fn rejects_workfront_release_with_open_gates() {
        let mut case = fixture();
        case["workfronts"][0]["status"] = Value::String("released".to_string());
        case["workfronts"][0]["releaseEvidenceIds"] =
            serde_json::json!(["EV-JURISDICTION"]);
        assert!(validate(&case)
            .iter()
            .any(|x| x.contains("is released while required gate")));
    }

    #[test]
    fn rejects_decision_type_substitution() {
        let mut case = fixture();
        case["decisions"][3]["decisionType"] =
            Value::String("community_agreement".to_string());
        case["gates"][2]["status"] = Value::String("complete".to_string());
        assert!(validate(&case)
            .iter()
            .any(|x| x.contains("requires decision type consultation_record")));
    }

    #[test]
    fn rejects_complete_gate_without_evidence() {
        let mut case = fixture();
        case["gates"][1]["status"] = Value::String("complete".to_string());
        case["gates"][1]["evidenceIds"] = serde_json::json!([]);
        assert!(validate(&case)
            .iter()
            .any(|x| x.contains("is complete without evidence")));
    }

    #[test]
    fn rejects_synthetic_evidence_promotion() {
        let mut case = fixture();
        case["caseMaturity"] = Value::String("field_observed".to_string());
        assert!(validate(&case).iter().any(|x| x.contains("promotes")));
    }

    #[test]
    fn rejects_expired_authority_decision_at_snapshot() {
        let mut case = fixture();
        case["decisions"][0]["status"] = Value::String("granted".to_string());
        case["decisions"][0]["effectiveFrom"] = Value::String("2026-01-01".to_string());
        case["decisions"][0]["expiresOn"] = Value::String("2026-10-01".to_string());
        assert!(validate(&case)
            .iter()
            .any(|x| x.contains("expired before snapshotDate")));
    }

    #[test]
    fn rejects_completed_land_access_gate_without_exact_decision_type() {
        let mut case = fixture();
        case["gates"][4]["status"] = Value::String("complete".to_string());
        case["gates"][4].as_object_mut().unwrap().remove("requiredDecisionType");
        assert!(validate(&case)
            .iter()
            .any(|x| x.contains("without an explicit requiredDecisionType")));
    }

    #[test]
    fn rejects_concern_response_without_evidence() {
        let mut case = fixture();
        case["engagements"][0]["concerns"][0]["status"] = Value::String("resolved".to_string());
        assert!(validate(&case)
            .iter()
            .any(|x| x.contains("response state without response evidence")));
    }

    #[test]
    fn rejects_released_workfront_in_synthetic_fixture() {
        let mut case = fixture();
        case["workfronts"][0]["status"] = Value::String("released".to_string());
        assert!(validate(&case)
            .iter()
            .any(|x| x.contains("cannot be represented as released in a synthetic fixture")));
    }
}
