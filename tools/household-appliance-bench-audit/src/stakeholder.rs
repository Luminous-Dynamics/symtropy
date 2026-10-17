use jsonschema::Validator;
use serde_json::{Value, json};
use std::collections::BTreeSet;

const SCHEMA_ID: &str = "urn:luminous-dynamics:repairability-architecture-decision-engine:0.1.0";
const EXPECTED_PROFILES: [&str; 5] = [
    "P1-HOUSEHOLD-OWNER",
    "P2-INDEPENDENT-TECHNICIAN",
    "P3-PARTS-AND-SERVICE-NETWORK",
    "P4-LIFECYCLE-MATERIAL-STEWARD",
    "P5-REMOTE-INSTITUTIONAL-OPERATOR",
];

type Errors = Vec<String>;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct StakeholderAuditSummary {
    pub(crate) profile_count: usize,
    pub(crate) requirement_count: usize,
    pub(crate) frozen_requirement_count: usize,
    pub(crate) profile_ready_count: usize,
}

fn text_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn criterion_by_id<'a>(criteria: &'a [Value], id: &str) -> Option<&'a Value> {
    criteria
        .iter()
        .find(|criterion| text_at(criterion, "id") == Some(id))
}

fn schema_validator(schema: &Value) -> Result<Validator, String> {
    if text_at(schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema") {
        return Err("decision schema must declare JSON Schema Draft 2020-12".to_owned());
    }
    if text_at(schema, "$id") != Some(SCHEMA_ID) {
        return Err(format!("decision schema $id must be {SCHEMA_ID}"));
    }
    if !jsonschema::draft202012::meta::is_valid(schema) {
        return Err(
            "decision schema does not validate against the Draft 2020-12 meta-schema".to_owned(),
        );
    }
    jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| format!("cannot compile decision schema: {error}"))
}

fn observation_by_id<'a>(candidate: &'a Value, criterion_id: &str) -> Option<&'a Value> {
    candidate
        .get("observations")?
        .as_array()?
        .iter()
        .find(|observation| text_at(observation, "criterion_id") == Some(criterion_id))
}

fn candidate_evidence_status(decision: &Value, candidate: &Value) -> &'static str {
    let Some(gates) = candidate.get("hard_gates").and_then(Value::as_array) else {
        return "blocked_unknown_gate";
    };
    let mut seen_pass = BTreeSet::new();
    for gate in gates {
        let Some(id) = text_at(gate, "gate_id") else {
            return "blocked_unknown_gate";
        };
        if text_at(gate, "status") == Some("fail") {
            return "rejected_hard_gate";
        }
        if text_at(gate, "status") != Some("pass")
            || text_at(gate, "evidence_id").is_none_or(str::is_empty)
            || text_at(gate, "evidence_id")
                .unwrap_or("")
                .starts_with("SYNTHETIC-")
        {
            return "blocked_unknown_gate";
        }
        if !seen_pass.insert(id.to_owned()) {
            return "blocked_unknown_gate";
        }
    }
    let Some(criteria) = decision.get("criteria").and_then(Value::as_array) else {
        return "insufficient_eligible_evidence";
    };
    if criteria.is_empty() {
        return "insufficient_eligible_evidence";
    }
    for criterion in criteria {
        let Some(id) = text_at(criterion, "id") else {
            return "insufficient_eligible_evidence";
        };
        if text_at(criterion, "status") != Some("method_frozen") {
            return "insufficient_eligible_evidence";
        }
        let Some(observation) = observation_by_id(candidate, id) else {
            return "insufficient_eligible_evidence";
        };
        if !matches!(
            text_at(observation, "evidence_class"),
            Some("physical_measurement" | "independent_replication")
        ) || observation.get("method_frozen").and_then(Value::as_bool) != Some(true)
            || text_at(observation, "unit") != text_at(criterion, "unit")
            || text_at(observation, "evidence_id").is_none_or(str::is_empty)
            || text_at(observation, "method_id").is_none_or(str::is_empty)
            || text_at(observation, "evidence_id")
                .unwrap_or("")
                .starts_with("SYNTHETIC-")
        {
            return "insufficient_eligible_evidence";
        }
        let (Some(lower), Some(upper)) = (
            observation.get("lower").and_then(Value::as_f64),
            observation.get("upper").and_then(Value::as_f64),
        ) else {
            return "insufficient_eligible_evidence";
        };
        if !lower.is_finite() || !upper.is_finite() || lower < 0.0 || lower > upper {
            return "insufficient_eligible_evidence";
        }
        if matches!(
            id,
            "first_visit_verified_repair_rate" | "target_window_part_fulfilment_rate"
        ) && upper > 1.0
        {
            return "insufficient_eligible_evidence";
        }
    }
    "eligible_for_profile_comparison"
}

fn candidate_meets_profile_requirements(
    decision: &Value,
    profile: &Value,
    candidate: &Value,
) -> &'static str {
    if text_at(profile, "status") != Some("requirements_frozen") {
        return "requirements_not_frozen";
    }
    let Some(requirements) = profile.get("requirements").and_then(Value::as_array) else {
        return "requirements_not_frozen";
    };
    for requirement in requirements {
        if text_at(requirement, "status") != Some("frozen_before_candidate_measurements")
            || requirement
                .get("frozen_before_candidate_measurements")
                .and_then(Value::as_bool)
                != Some(true)
        {
            return "requirements_not_frozen";
        }
        let Some(threshold) = requirement.get("threshold").and_then(Value::as_f64) else {
            return "requirements_not_frozen";
        };
        let Some(criterion_id) = text_at(requirement, "criterion_id") else {
            return "requirements_not_frozen";
        };
        let Some(criterion) =
            decision
                .get("criteria")
                .and_then(Value::as_array)
                .and_then(|criteria| {
                    criteria
                        .iter()
                        .find(|criterion| text_at(criterion, "id") == Some(criterion_id))
                })
        else {
            return "requirements_not_frozen";
        };
        let Some(observation) = observation_by_id(candidate, criterion_id) else {
            return "insufficient_eligible_evidence";
        };
        let (Some(lower), Some(upper)) = (
            observation.get("lower").and_then(Value::as_f64),
            observation.get("upper").and_then(Value::as_f64),
        ) else {
            return "insufficient_eligible_evidence";
        };
        match text_at(requirement, "direction") {
            Some("minimize") if upper <= threshold => {}
            Some("minimize") if lower > threshold => return "fails_profile_constraint",
            Some("maximize") if lower >= threshold => {}
            Some("maximize") if upper < threshold => return "fails_profile_constraint",
            Some("minimize" | "maximize") => return "profile_constraint_unresolved",
            _ => return "requirements_not_frozen",
        }
    }
    "meets_profile_constraints"
}

fn robustly_dominates_for_profile(decision: &Value, profile: &Value, a: &Value, b: &Value) -> bool {
    let Some(ids) = profile.get("criteria").and_then(Value::as_array) else {
        return false;
    };
    let mut strict_improvement = false;
    for id_value in ids {
        let Some(id) = id_value.as_str() else {
            return false;
        };
        let Some(criterion) =
            decision
                .get("criteria")
                .and_then(Value::as_array)
                .and_then(|criteria| {
                    criteria
                        .iter()
                        .find(|criterion| text_at(criterion, "id") == Some(id))
                })
        else {
            return false;
        };
        let (Some(a_obs), Some(b_obs)) = (observation_by_id(a, id), observation_by_id(b, id))
        else {
            return false;
        };
        let (Some(a_lower), Some(a_upper), Some(b_lower), Some(b_upper)) = (
            a_obs.get("lower").and_then(Value::as_f64),
            a_obs.get("upper").and_then(Value::as_f64),
            b_obs.get("lower").and_then(Value::as_f64),
            b_obs.get("upper").and_then(Value::as_f64),
        ) else {
            return false;
        };
        match text_at(criterion, "direction") {
            Some("minimize") => {
                if a_upper > b_lower {
                    return false;
                }
                strict_improvement |= a_upper < b_lower;
            }
            Some("maximize") => {
                if a_lower < b_upper {
                    return false;
                }
                strict_improvement |= a_lower > b_upper;
            }
            _ => return false,
        }
    }
    strict_improvement
}

fn audit_values(
    schema: &Value,
    decision: &Value,
) -> Result<(StakeholderAuditSummary, Value), Errors> {
    let validator = schema_validator(schema).map_err(|error| vec![error])?;
    let schema_errors: Vec<String> = validator
        .iter_errors(decision)
        .map(|error| error.to_string())
        .collect();
    if !schema_errors.is_empty() {
        return Err(schema_errors);
    }

    let mut errors = Vec::new();
    let profile_policy = decision.get("profile_policy").unwrap_or(&Value::Null);
    if text_at(profile_policy, "profile_policy_id")
        != Some("STAKEHOLDER-REPAIRABILITY-PROFILES-001")
        || profile_policy
            .get("hard_gate_override")
            .and_then(Value::as_bool)
            != Some(false)
        || profile_policy
            .get("selection_before_evidence")
            .and_then(Value::as_bool)
            != Some(false)
        || text_at(profile_policy, "threshold_policy") != Some("unfrozen_threshold_remains_unknown")
        || text_at(profile_policy, "weighting_policy") != Some("no_hidden_global_weights")
    {
        errors.push(
            "stakeholder profiles cannot override hard gates, select before evidence, or invent hidden weights"
                .to_owned(),
        );
    }

    let Some(criteria) = decision.get("criteria").and_then(Value::as_array) else {
        return Err(vec!["decision criteria must be an array".to_owned()]);
    };
    let Some(profiles) = decision
        .get("stakeholder_profiles")
        .and_then(Value::as_array)
    else {
        return Err(vec!["stakeholder_profiles must be an array".to_owned()]);
    };
    if profiles.len() != EXPECTED_PROFILES.len() {
        errors.push(format!(
            "decision must contain {} reviewed stakeholder profiles",
            EXPECTED_PROFILES.len()
        ));
    }

    let mut profile_ids = BTreeSet::new();
    let mut total_requirements = 0usize;
    let mut frozen_requirements = 0usize;
    let mut profile_results = Vec::new();

    for (index, profile) in profiles.iter().enumerate() {
        let Some(profile_id) = text_at(profile, "id") else {
            errors.push(format!("stakeholder_profiles[{index}].id must be a string"));
            continue;
        };
        if !profile_ids.insert(profile_id.to_owned()) {
            errors.push(format!("duplicate stakeholder profile {profile_id}"));
        }
        if EXPECTED_PROFILES.get(index).copied() != Some(profile_id) {
            errors.push(format!(
                "stakeholder profile order must preserve {:?}; got {profile_id}",
                EXPECTED_PROFILES.get(index)
            ));
        }

        let profile_criteria: BTreeSet<&str> = profile
            .get("criteria")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        if profile_criteria.is_empty() {
            errors.push(format!(
                "{profile_id}: at least one mapped criterion is required"
            ));
        }
        for criterion_id in &profile_criteria {
            if criterion_by_id(criteria, criterion_id).is_none() {
                errors.push(format!(
                    "{profile_id}: unknown criterion mapping {criterion_id}"
                ));
            }
        }

        let Some(requirements) = profile.get("requirements").and_then(Value::as_array) else {
            errors.push(format!("{profile_id}: requirements must be an array"));
            continue;
        };
        total_requirements += requirements.len();
        let mut req_ids = BTreeSet::new();
        let mut all_frozen = !requirements.is_empty();

        for requirement in requirements {
            let Some(req_id) = text_at(requirement, "id") else {
                errors.push(format!("{profile_id}: requirement ID must be a string"));
                all_frozen = false;
                continue;
            };
            if !req_ids.insert(req_id.to_owned()) {
                errors.push(format!("{profile_id}: duplicate requirement ID {req_id}"));
            }
            let Some(criterion_id) = text_at(requirement, "criterion_id") else {
                errors.push(format!("{profile_id}.{req_id}: criterion_id is required"));
                all_frozen = false;
                continue;
            };
            let Some(criterion) = criterion_by_id(criterion_id) else {
                errors.push(format!(
                    "{profile_id}.{req_id}: unknown criterion {criterion_id}"
                ));
                all_frozen = false;
                continue;
            };
            if !profile_criteria.contains(criterion_id) {
                errors.push(format!(
                    "{profile_id}.{req_id}: requirement must map to a declared profile criterion"
                ));
            }
            if text_at(requirement, "direction") != text_at(criterion, "direction")
                || text_at(requirement, "unit") != text_at(criterion, "unit")
            {
                errors.push(format!(
                    "{profile_id}.{req_id}: direction and unit must match the canonical criterion"
                ));
            }

            let threshold = requirement.get("threshold").unwrap_or(&Value::Null);
            let frozen = text_at(requirement, "status")
                == Some("frozen_before_candidate_measurements")
                && requirement
                    .get("frozen_before_candidate_measurements")
                    .and_then(Value::as_bool)
                    == Some(true)
                && threshold
                    .as_f64()
                    .is_some_and(|value| value.is_finite() && value >= 0.0);
            let unfrozen = text_at(requirement, "status") == Some("threshold_not_frozen")
                && requirement
                    .get("frozen_before_candidate_measurements")
                    .and_then(Value::as_bool)
                    == Some(false)
                && threshold.is_null();
            if frozen
                && matches!(
                    criterion_id,
                    "first_visit_verified_repair_rate" | "target_window_part_fulfilment_rate"
                )
                && threshold.as_f64().is_some_and(|value| value > 1.0)
            {
                errors.push(format!(
                    "{profile_id}.{req_id}: proportion thresholds must not exceed 1.0"
                ));
            }
            if frozen {
                frozen_requirements += 1;
            } else if !unfrozen {
                errors.push(format!(
                    "{profile_id}.{req_id}: threshold value, status and preregistration flag disagree"
                ));
            }
            all_frozen &= frozen;
        }

        let status = text_at(profile, "status");
        if status == Some("requirements_frozen") && !all_frozen {
            errors.push(format!(
                "{profile_id}: cannot freeze a profile while a requirement is unfrozen"
            ));
        }
        if status == Some("requirements_not_frozen") && all_frozen {
            errors.push(format!(
                "{profile_id}: profile status disagrees with frozen requirements"
            ));
        }

        if profile_id == "P1-HOUSEHOLD-OWNER" {
            let boundary = text_at(profile, "required_task_boundary")
                .unwrap_or("")
                .to_ascii_lowercase();
            if !boundary.contains("no opening internal mains")
                || !boundary.contains("refrigerant")
                || boundary.contains("may open internal mains")
                || boundary.contains("may perform refrigerant charging")
                || boundary.contains("refrigerant work is permitted")
            {
                errors.push(
                    "P1 household task boundary must explicitly prohibit hazardous internal mains/refrigerant work"
                        .to_owned(),
                );
            }
        }

        let candidates = decision
            .get("candidates")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut candidate_assessments = Vec::new();
        let mut qualifying_candidates = Vec::new();
        for candidate in &candidates {
            let universal = candidate_evidence_status(decision, candidate);
            let candidate_status = if universal != "eligible_for_profile_comparison" {
                universal
            } else {
                candidate_meets_profile_requirements(decision, profile, candidate)
            };
            if candidate_status == "meets_profile_constraints" {
                qualifying_candidates.push(candidate.clone());
            }
            candidate_assessments.push(json!({
                "candidate_id": text_at(candidate, "id").unwrap_or("unknown"),
                "status": candidate_status,
                "robust_pareto_frontier": false,
                "robustly_dominated_by": []
            }));
        }
        let mut frontier_count = 0usize;
        for candidate in &qualifying_candidates {
            let dominators: Vec<String> = qualifying_candidates
                .iter()
                .filter(|other| {
                    text_at(other, "id") != text_at(candidate, "id")
                        && robustly_dominates_for_profile(decision, profile, other, candidate)
                })
                .filter_map(|other| text_at(other, "id").map(str::to_owned))
                .collect();
            let is_frontier = dominators.is_empty();
            if is_frontier {
                frontier_count += 1;
            }
            if let Some(outcome) = candidate_assessments
                .iter_mut()
                .find(|outcome| text_at(outcome, "candidate_id") == text_at(candidate, "id"))
            {
                outcome["robust_pareto_frontier"] = json!(is_frontier);
                outcome["robustly_dominated_by"] = json!(dominators);
            }
        }
        profile_results.push(json!({
            "profile_id": profile_id,
            "status": status.unwrap_or("unknown"),
            "criteria_count": profile_criteria.len(),
            "requirement_count": requirements.len(),
            "frozen_requirement_count": requirements.iter().filter(|requirement| {
                text_at(requirement, "status") == Some("frozen_before_candidate_measurements")
                    && requirement.get("threshold").and_then(Value::as_f64).is_some()
                    && requirement.get("frozen_before_candidate_measurements").and_then(Value::as_bool) == Some(true)
            }).count(),
            "candidate_assessments": candidate_assessments,
            "candidate_meeting_constraints_count": qualifying_candidates.len(),
            "profile_pareto_frontier_count": frontier_count,
            "profile_frontier_status": if status == Some("requirements_frozen") {
                "evaluated_only_when_universal_gates_and_candidate_evidence_are_eligible"
            } else {
                "requirements_not_frozen"
            },
            "profile_specific_selection_claim_permitted": false
        }));
    }
    for expected in EXPECTED_PROFILES {
        if !profile_ids.contains(expected) {
            errors.push(format!(
                "required stakeholder profile {expected} is missing"
            ));
        }
    }
    if total_requirements != 18 {
        errors.push(format!(
            "reviewed stakeholder requirement census must remain 18; got {total_requirements}"
        ));
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let summary = StakeholderAuditSummary {
        profile_count: profiles.len(),
        requirement_count: total_requirements,
        frozen_requirement_count: frozen_requirements,
        profile_ready_count: profiles
            .iter()
            .filter(|profile| text_at(profile, "status") == Some("requirements_frozen"))
            .count(),
    };
    let report = json!({
        "policy": "stakeholder_constraints_without_hidden_global_weights",
        "profile_count": summary.profile_count,
        "requirement_count": summary.requirement_count,
        "frozen_requirement_count": summary.frozen_requirement_count,
        "profile_ready_count": summary.profile_ready_count,
        "profile_specific_selection_claim_permitted": false,
        "profiles": profile_results
    });
    Ok((summary, report))
}

pub(crate) fn audit_stakeholder_texts(
    schema_raw: &str,
    decision_raw: &str,
) -> Result<(StakeholderAuditSummary, Value), Errors> {
    let schema: Value = serde_json::from_str(schema_raw)
        .map_err(|error| vec![format!("decision schema JSON parse failed: {error}")])?;
    let decision: Value = serde_json::from_str(decision_raw)
        .map_err(|error| vec![format!("decision profile JSON parse failed: {error}")])?;
    audit_values(&schema, &decision)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SCHEMA: &str = include_str!(
        "../../../schemas/appliances/repairability_architecture_decision_v0_1.schema.json"
    );
    const DECISION: &str =
        include_str!("../../../fixtures/appliances/repairability_architecture_decision_v0_1.json");

    fn canonical_decision() -> Value {
        serde_json::from_str(DECISION).expect("decision profile must parse")
    }

    fn synthetic_frozen_decision() -> Value {
        let mut decision = canonical_decision();
        for criterion in decision["criteria"].as_array_mut().expect("criteria") {
            criterion["status"] = json!("method_frozen");
        }
        for profile in decision["stakeholder_profiles"]
            .as_array_mut()
            .expect("profiles")
        {
            profile["status"] = json!("requirements_frozen");
            for requirement in profile["requirements"]
                .as_array_mut()
                .expect("requirements")
            {
                requirement["status"] = json!("frozen_before_candidate_measurements");
                requirement["frozen_before_candidate_measurements"] = json!(true);
                requirement["threshold"] = if text_at(requirement, "direction") == Some("minimize")
                {
                    json!(100000.0)
                } else {
                    json!(0.0)
                };
            }
        }
        let criteria = decision["criteria"].as_array().expect("criteria").clone();
        for (candidate_index, candidate) in decision["candidates"]
            .as_array_mut()
            .expect("candidates")
            .iter_mut()
            .enumerate()
        {
            for gate in candidate["hard_gates"].as_array_mut().expect("gates") {
                gate["status"] = json!("pass");
                gate["evidence_id"] = json!(format!("LAB-RECEIPT-W{candidate_index}"));
            }
            let mut observations = Vec::new();
            for (criterion_index, criterion) in criteria.iter().enumerate() {
                let id = text_at(criterion, "id").unwrap_or("unknown");
                let direction = text_at(criterion, "direction").unwrap_or("minimize");
                let value = if direction == "minimize" {
                    (candidate_index * 10 + criterion_index + 2) as f64
                } else {
                    0.95 - candidate_index as f64 * 0.1
                };
                observations.push(json!({
                    "criterion_id": id,
                    "lower": value,
                    "upper": value,
                    "unit": text_at(criterion, "unit").unwrap_or("unknown"),
                    "evidence_class": "physical_measurement",
                    "evidence_id": format!("LAB-RECEIPT-W{candidate_index}-{id}"),
                    "method_id": format!("FROZEN-METHOD-{id}"),
                    "method_frozen": true,
                    "uncertainty_note": "Synthetic logic test fixture; not physical appliance evidence."
                }));
            }
            candidate["observations"] = json!(observations);
        }
        decision
    }

    fn audit_mutation(decision: &Value) -> Result<(StakeholderAuditSummary, Value), Errors> {
        let schema: Value = serde_json::from_str(SCHEMA).expect("schema must parse");
        audit_values(&schema, decision)
    }

    #[test]
    fn canonical_profiles_keep_all_thresholds_unknown() {
        let (summary, report) = audit_stakeholder_texts(SCHEMA, DECISION)
            .expect("stakeholder profile fixture must qualify");
        assert_eq!(summary.profile_count, 5);
        assert_eq!(summary.requirement_count, 18);
        assert_eq!(summary.frozen_requirement_count, 0);
        assert_eq!(summary.profile_ready_count, 0);
        assert_eq!(
            report["profile_specific_selection_claim_permitted"],
            json!(false)
        );
        for profile in report["profiles"].as_array().expect("profiles") {
            assert_eq!(profile["status"], "requirements_not_frozen");
            assert_eq!(
                profile["profile_specific_selection_claim_permitted"],
                json!(false)
            );
        }
    }

    #[test]
    fn frozen_profiles_apply_constraints_then_compute_separate_frontiers() {
        let decision = synthetic_frozen_decision();
        let raw = serde_json::to_string(&decision).unwrap();
        let schema: Value = serde_json::from_str(SCHEMA).unwrap();
        let (summary, report) = audit_values(&schema, &decision)
            .expect("synthetic stakeholder logic fixture must validate");
        assert_eq!(summary.profile_ready_count, 5);
        assert_eq!(summary.frozen_requirement_count, 18);
        assert_eq!(
            report["profile_specific_selection_claim_permitted"],
            json!(false)
        );
        for profile in report["profiles"].as_array().expect("profiles") {
            assert_eq!(profile["status"], "requirements_frozen");
            assert_eq!(profile["candidate_meeting_constraints_count"], json!(5));
            assert_eq!(profile["profile_pareto_frontier_count"], json!(1));
            assert_eq!(
                profile["profile_specific_selection_claim_permitted"],
                json!(false)
            );
        }
        assert!(!raw.is_empty());
    }

    #[test]
    fn overlapping_uncertainty_at_profile_threshold_is_unresolved() {
        let mut decision = synthetic_frozen_decision();
        decision["stakeholder_profiles"][0]["requirements"][0]["threshold"] = json!(3.0);
        decision["candidates"][0]["observations"][0]["lower"] = json!(2.0);
        decision["candidates"][0]["observations"][0]["upper"] = json!(4.0);
        let raw = serde_json::to_string(&decision).unwrap();
        let schema: Value = serde_json::from_str(SCHEMA).unwrap();
        let (_, report) = audit_values(&schema, &decision)
            .expect("overlapping profile interval must remain valid evidence");
        assert_eq!(
            report["profiles"][0]["candidate_assessments"][0]["status"],
            json!("profile_constraint_unresolved")
        );
        assert!(!raw.is_empty());
    }

    #[test]
    fn unmet_profile_threshold_is_reported_without_global_score() {
        let mut decision = synthetic_frozen_decision();
        decision["stakeholder_profiles"][0]["requirements"][0]["threshold"] = json!(0.0);
        let schema: Value = serde_json::from_str(SCHEMA).unwrap();
        let (_, report) =
            audit_values(&schema, &decision).expect("profile threshold is a valid constraint");
        assert_eq!(
            report["profiles"][0]["candidate_assessments"][0]["status"],
            json!("fails_profile_constraint")
        );
        assert_eq!(
            report["policy"],
            json!("stakeholder_constraints_without_hidden_global_weights")
        );
    }

    #[test]
    fn duplicate_stakeholder_profile_is_rejected() {
        let mut decision = canonical_decision();
        decision["stakeholder_profiles"][1]["id"] = json!("P1-HOUSEHOLD-OWNER");
        let errors = audit_mutation(&decision).expect_err("duplicate profile must fail");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("duplicate stakeholder profile"))
        );
    }

    #[test]
    fn unfrozen_threshold_cannot_be_filled_with_a_numeric_value() {
        let mut decision = canonical_decision();
        decision["stakeholder_profiles"][0]["requirements"][0]["threshold"] = json!(0.0);
        let errors = audit_mutation(&decision).expect_err("unfrozen value must remain null");
        assert!(errors.iter().any(|error| {
            error.contains("threshold value, status and preregistration flag disagree")
        }));
    }

    #[test]
    fn household_profile_cannot_expand_into_hazardous_diy_work() {
        let mut decision = canonical_decision();
        decision["stakeholder_profiles"][0]["required_task_boundary"] =
            json!("DIY users may open internal mains and may perform refrigerant charging.");
        let errors = audit_mutation(&decision).expect_err("hazardous user scope must fail");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("household task boundary"))
        );
    }

    #[test]
    fn profile_requirement_cannot_redefine_criterion_units() {
        let mut decision = canonical_decision();
        decision["stakeholder_profiles"][0]["requirements"][0]["unit"] = json!("invented_units");
        let errors = audit_mutation(&decision).expect_err("profile units must be canonical");
        assert!(
            errors
                .iter()
                .any(|error| { error.contains("direction and unit must match") })
        );
    }

    #[test]
    fn profiles_cannot_override_universal_hard_gates() {
        let mut decision = canonical_decision();
        decision["profile_policy"]["hard_gate_override"] = json!(true);
        let errors = audit_mutation(&decision).expect_err("profile hard-gate override must fail");
        assert!(!errors.is_empty());
    }
}
