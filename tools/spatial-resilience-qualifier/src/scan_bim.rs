//! Scan-to-BIM adapter for task-conditional, finite-state decision-sufficient capture.
//!
//! This module does not reconstruct geometry, estimate survey uncertainty, plan
//! sensor motion, or implement Value of Information. It checks the bindings
//! needed to qualify scan actions, then delegates finite-state feasibility and
//! Pareto evaluation to the existing spatial-resilience decision-impact oracle.

use crate::decision_impact::{DecisionCaseResult, DecisionImpactFailure, evaluate_decision_case_value};
use crate::sha256_hex;
use jsonschema::validator_for;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub const EMBEDDED_SCAN_BIM_DECISION_CASE: &str = include_str!(
    "../../../research/scan-bim-decision-capture-001/fixtures/door-clearance-v0_1.json"
);
pub const EMBEDDED_SCAN_BIM_DECISION_SCHEMA: &str = include_str!(
    "../../../research/scan-bim-decision-capture-001/schema/scan-bim-decision-case-v0_1.schema.json"
);

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ScanBimAdapterFailure {
    pub code: String,
    pub path: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ScanBimActionQualification {
    pub action_id: String,
    pub evidence_state: String,
    pub target_count: usize,
    pub qualified_target_count: usize,
    pub visibility_receipt_sha256: String,
    pub pose_evidence_sha256: String,
    pub rejection_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanBimDecisionReport {
    pub schema_version: String,
    pub case_id: String,
    pub building_group_id: String,
    pub scene_id: String,
    pub model_revision: String,
    pub requirement_revision: String,
    pub input_sha256: String,
    pub schema_sha256: String,
    pub action_count: usize,
    /// Actions with sufficient declared sensor/pose/visibility bindings.
    pub evidence_qualified_action_count: usize,
    /// Actions that the shared finite-state engine accepted after partition/revision checks.
    pub decision_accepted_action_count: usize,
    /// Accepted actions that change critical unknowns, hard feasibility or Pareto choices.
    pub decision_relevant_action_count: usize,
    /// Actions rejected by the shared finite-state engine.
    pub decision_rejected_action_count: usize,
    pub decision: DecisionCaseResult,
    pub action_qualifications: Vec<ScanBimActionQualification>,
    pub claim_ceiling: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScanBimDecisionCase {
    schema_version: String,
    case_id: String,
    building_group_id: String,
    scene_id: String,
    model_revision: String,
    frame_id: String,
    task: TaskProfile,
    possible_states: Vec<PossibleState>,
    current_possible_state_ids: Vec<String>,
    candidates: Vec<DecisionCandidate>,
    actions: Vec<ScanAction>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TaskProfile {
    task_id: String,
    requirement_revision: String,
    property_id: String,
    comparison: RequirementComparison,
    threshold_m: f64,
    target_entity_ids: Vec<String>,
    minimum_observable_area_fraction: f64,
    unresolved_critical_unknown_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RequirementComparison {
    GreaterThanOrEqual,
    LessThanOrEqual,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PossibleState {
    state_id: String,
    interpretation: String,
    property_value_m: f64,
    requirement_satisfied: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DecisionCandidate {
    candidate_id: String,
    state_results: Vec<CandidateStateResult>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateStateResult {
    state_id: String,
    feasible: bool,
    objectives: CandidateObjectives,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateObjectives {
    service_hours: u64,
    energy_units: u64,
    repair_ticks: u64,
    cost_units: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScanAction {
    action_id: String,
    sensor_id: String,
    frame_id: String,
    model_revision: String,
    pose_revision_id: String,
    pose_evidence_sha256: String,
    sensor_profile_sha256: String,
    visibility_profile_sha256: String,
    visibility_receipt_sha256: String,
    target_visibility: Vec<TargetVisibility>,
    estimated_cost_units: u64,
    estimated_duration_ticks: u64,
    cost_basis_id: String,
    duration_basis_id: String,
    resolves_unknown_ids: Vec<String>,
    outcomes: Vec<ActionOutcome>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TargetVisibility {
    target_entity_id: String,
    status: VisibilityStatus,
    observable_area_m2: f64,
    theoretical_area_m2: f64,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum VisibilityStatus {
    Observable,
    Grazing,
    SelfOccluded,
    ExternallyOccluded,
    OutsideSensorCoverage,
    Unknown,
    Contradictory,
    MixedOcclusion,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ActionOutcome {
    outcome_id: String,
    compatible_state_ids: Vec<String>,
}

fn sha256_is_valid(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn push_rejection(codes: &mut BTreeSet<String>, code: &str) {
    codes.insert(code.to_owned());
}

fn qualify_action(
    action: &ScanAction,
    case: &ScanBimDecisionCase,
) -> ScanBimActionQualification {
    let mut rejection_codes = BTreeSet::new();
    if action.sensor_id.trim().is_empty() || action.pose_revision_id.trim().is_empty() {
        push_rejection(&mut rejection_codes, "SENSOR_OR_POSE_ID_MISSING");
    }
    if action.frame_id != case.frame_id {
        push_rejection(&mut rejection_codes, "SCAN_FRAME_MISMATCH");
    }
    if action.model_revision != case.model_revision {
        push_rejection(&mut rejection_codes, "SCAN_MODEL_REVISION_STALE");
    }
    for (field, digest) in [
        ("pose_evidence", action.pose_evidence_sha256.as_str()),
        ("sensor_profile", action.sensor_profile_sha256.as_str()),
        ("visibility_profile", action.visibility_profile_sha256.as_str()),
        ("visibility_receipt", action.visibility_receipt_sha256.as_str()),
    ] {
        if !sha256_is_valid(digest) {
            push_rejection(
                &mut rejection_codes,
                &format!("{}_DIGEST_INVALID", field.to_ascii_uppercase()),
            );
        }
    }

    let expected_targets: BTreeSet<&str> = case
        .task
        .target_entity_ids
        .iter()
        .map(String::as_str)
        .collect();
    let mut seen_targets = BTreeSet::new();
    let mut qualified_target_count = 0usize;
    for target in &action.target_visibility {
        if target.target_entity_id.trim().is_empty()
            || !expected_targets.contains(target.target_entity_id.as_str())
            || !seen_targets.insert(target.target_entity_id.as_str())
        {
            push_rejection(&mut rejection_codes, "TARGET_VISIBILITY_ID_INVALID");
        }
        if !target.observable_area_m2.is_finite()
            || target.observable_area_m2 < 0.0
            || !target.theoretical_area_m2.is_finite()
            || target.theoretical_area_m2 <= 0.0
            || target.observable_area_m2 > target.theoretical_area_m2
        {
            push_rejection(&mut rejection_codes, "TARGET_VISIBILITY_AREA_INVALID");
            continue;
        }
        let visible_fraction = target.observable_area_m2 / target.theoretical_area_m2;
        if target.status != VisibilityStatus::Observable {
            push_rejection(&mut rejection_codes, "TARGET_NOT_THEORETICALLY_OBSERVABLE");
        } else if visible_fraction + 1.0e-12
            < case.task.minimum_observable_area_fraction
        {
            push_rejection(&mut rejection_codes, "TARGET_COVERAGE_BELOW_TASK_THRESHOLD");
        } else {
            qualified_target_count += 1;
        }
    }
    if seen_targets.len() != expected_targets.len() {
        push_rejection(&mut rejection_codes, "TARGET_VISIBILITY_INCOMPLETE");
    }

    let evidence_state = if rejection_codes.is_empty() {
        "valid"
    } else {
        "invalid"
    };
    ScanBimActionQualification {
        action_id: action.action_id.clone(),
        evidence_state: evidence_state.to_owned(),
        target_count: action.target_visibility.len(),
        qualified_target_count,
        visibility_receipt_sha256: action.visibility_receipt_sha256.clone(),
        pose_evidence_sha256: action.pose_evidence_sha256.clone(),
        rejection_codes: rejection_codes.into_iter().collect(),
    }
}

fn case_to_decision_value(
    case: &ScanBimDecisionCase,
    qualifications: &[ScanBimActionQualification],
) -> Value {
    let qualification_by_id: std::collections::BTreeMap<&str, &ScanBimActionQualification> =
        qualifications
            .iter()
            .map(|qualification| (qualification.action_id.as_str(), qualification))
            .collect();
    let candidates = case
        .candidates
        .iter()
        .map(|candidate| {
            json!({
                "candidate_id": candidate.candidate_id,
                "state_results": candidate.state_results.iter().map(|state| json!({
                    "state_id": state.state_id,
                    "feasible": state.feasible,
                    "objectives": {
                        "service_hours": state.objectives.service_hours,
                        "energy_units": state.objectives.energy_units,
                        "repair_ticks": state.objectives.repair_ticks,
                        "cost_units": state.objectives.cost_units
                    }
                })).collect::<Vec<_>>()
            })
        })
        .collect::<Vec<_>>();
    let actions = case
        .actions
        .iter()
        .map(|action| {
            let evidence_state = qualification_by_id
                .get(action.action_id.as_str())
                .map_or("invalid", |qualification| qualification.evidence_state.as_str());
            json!({
                "action_id": action.action_id,
                "action_type": "scan",
                "evidence_state": evidence_state,
                "source_revision": action.model_revision,
                "estimated_cost_units": action.estimated_cost_units,
                "estimated_duration_ticks": action.estimated_duration_ticks,
                "cost_basis_id": action.cost_basis_id,
                "duration_basis_id": action.duration_basis_id,
                "resolves_unknown_ids": action.resolves_unknown_ids,
                "outcomes": action.outcomes.iter().map(|outcome| json!({
                    "outcome_id": outcome.outcome_id,
                    "compatible_state_ids": outcome.compatible_state_ids
                })).collect::<Vec<_>>()
            })
        })
        .collect::<Vec<_>>();
    json!({
        "id": case.case_id,
        "state_revision": case.model_revision,
        "state_ids": case.possible_states.iter().map(|state| state.state_id.clone()).collect::<Vec<_>>(),
        "current_possible_state_ids": case.current_possible_state_ids,
        "unresolved_critical_unknown_ids": case.task.unresolved_critical_unknown_ids,
        "candidates": candidates,
        "actions": actions
    })
}

/// Validate and evaluate a Scan-to-BIM decision case.
///
/// The adapter authenticates neither the referenced sensor/pose/visibility
/// artifacts nor the physical hypothesis set. It checks declared digests,
/// frames, revisions and visibility-area semantics, then delegates decision
/// evaluation to the shared finite-state engine.
pub fn evaluate_scan_bim_decision_bytes(
    input_bytes: &[u8],
    schema_bytes: &[u8],
) -> Result<ScanBimDecisionReport, Vec<ScanBimAdapterFailure>> {
    let schema_value: Value = serde_json::from_slice(schema_bytes).map_err(|error| {
        vec![ScanBimAdapterFailure {
            code: "SCAN_BIM_SCHEMA_JSON_INVALID".to_owned(),
            path: "$".to_owned(),
            detail: error.to_string(),
        }]
    })?;
    let input_value: Value = serde_json::from_slice(input_bytes).map_err(|error| {
        vec![ScanBimAdapterFailure {
            code: "SCAN_BIM_CASE_JSON_INVALID".to_owned(),
            path: "$".to_owned(),
            detail: error.to_string(),
        }]
    })?;
    let validator = validator_for(&schema_value).map_err(|error| {
        vec![ScanBimAdapterFailure {
            code: "SCAN_BIM_SCHEMA_COMPILE_FAILED".to_owned(),
            path: "$".to_owned(),
            detail: error.to_string(),
        }]
    })?;
    let schema_errors: Vec<_> = validator.iter_errors(&input_value).collect();
    if !schema_errors.is_empty() {
        return Err(schema_errors
            .into_iter()
            .map(|error| ScanBimAdapterFailure {
                code: "SCAN_BIM_SCHEMA_VALIDATION_FAILED".to_owned(),
                path: error.instance_path().to_string(),
                detail: error.to_string(),
            })
            .collect());
    }
    let case: ScanBimDecisionCase =
        serde_json::from_value(input_value).map_err(|error| {
            vec![ScanBimAdapterFailure {
                code: "SCAN_BIM_TYPED_CASE_INVALID".to_owned(),
                path: "$".to_owned(),
                detail: error.to_string(),
            }]
        })?;
    validate_case_header(&case)?;

    let mut qualifications: Vec<ScanBimActionQualification> = case
        .actions
        .iter()
        .map(|action| qualify_action(action, &case))
        .collect();
    qualifications.sort_by(|left, right| left.action_id.cmp(&right.action_id));

    let decision_value = case_to_decision_value(&case, &qualifications);
    let decision = evaluate_decision_case_value(&decision_value).map_err(|failures| {
        failures
            .into_iter()
            .map(|failure: DecisionImpactFailure| ScanBimAdapterFailure {
                code: failure.code,
                path: failure.path,
                detail: failure.detail,
            })
            .collect::<Vec<_>>()
    })?;
    let evidence_qualified_action_count = qualifications
        .iter()
        .filter(|qualification| qualification.evidence_state == "valid")
        .count();
    let decision_accepted_action_count = decision
        .actions
        .iter()
        .filter(|action| action.status == "evaluated")
        .count();
    let decision_relevant_action_count = decision
        .actions
        .iter()
        .filter(|action| action.decision_relevant)
        .count();
    let decision_rejected_action_count = decision
        .actions
        .iter()
        .filter(|action| action.status == "reject_action")
        .count();
    Ok(ScanBimDecisionReport {
        schema_version: case.schema_version,
        case_id: case.case_id,
        building_group_id: case.building_group_id,
        scene_id: case.scene_id,
        model_revision: case.model_revision,
        requirement_revision: case.task.requirement_revision,
        input_sha256: sha256_hex(input_bytes),
        schema_sha256: sha256_hex(schema_bytes),
        action_count: qualifications.len(),
        evidence_qualified_action_count,
        decision_accepted_action_count,
        decision_relevant_action_count,
        decision_rejected_action_count,
        decision,
        action_qualifications: qualifications,
        claim_ceiling: "FINITE_STATE_SYNTHETIC_DECISION_ADAPTER_ONLY: declared visibility, pose, sensor, hypothesis and cost evidence are not authenticated or calibrated by this adapter; no probability, physical survey accuracy, or real building decision claim is made.".to_owned(),
    })
}

fn validate_case_header(case: &ScanBimDecisionCase) -> Result<(), Vec<ScanBimAdapterFailure>> {
    let mut failures = Vec::new();
    for (field, value) in [
        ("case_id", case.case_id.as_str()),
        ("building_group_id", case.building_group_id.as_str()),
        ("scene_id", case.scene_id.as_str()),
        ("model_revision", case.model_revision.as_str()),
        ("frame_id", case.frame_id.as_str()),
        ("task.task_id", case.task.task_id.as_str()),
        ("task.requirement_revision", case.task.requirement_revision.as_str()),
        ("task.property_id", case.task.property_id.as_str()),
    ] {
        if value.trim().is_empty() {
            failures.push(ScanBimAdapterFailure {
                code: "SCAN_BIM_IDENTITY_EMPTY".to_owned(),
                path: field.to_owned(),
                detail: "Required identity/revision field must not be empty.".to_owned(),
            });
        }
    }
    if case.schema_version != "0.1" {
        failures.push(ScanBimAdapterFailure {
            code: "SCAN_BIM_SCHEMA_VERSION_UNSUPPORTED".to_owned(),
            path: "schema_version".to_owned(),
            detail: "Only version 0.1 is supported.".to_owned(),
        });
    }
    if !case.task.threshold_m.is_finite() || case.task.threshold_m < 0.0 {
        failures.push(ScanBimAdapterFailure {
            code: "SCAN_BIM_REQUIREMENT_THRESHOLD_INVALID".to_owned(),
            path: "task.threshold_m".to_owned(),
            detail: "Requirement threshold must be finite and non-negative.".to_owned(),
        });
    }
    if !case.task.minimum_observable_area_fraction.is_finite()
        || !(0.0..=1.0).contains(&case.task.minimum_observable_area_fraction)
    {
        failures.push(ScanBimAdapterFailure {
            code: "SCAN_BIM_VISIBILITY_THRESHOLD_INVALID".to_owned(),
            path: "task.minimum_observable_area_fraction".to_owned(),
            detail: "Visibility threshold must be finite and in [0, 1].".to_owned(),
        });
    }

    let mut targets = BTreeSet::new();
    for target in &case.task.target_entity_ids {
        if target.trim().is_empty() || !targets.insert(target.as_str()) {
            failures.push(ScanBimAdapterFailure {
                code: "SCAN_BIM_TARGET_ID_INVALID".to_owned(),
                path: "task.target_entity_ids".to_owned(),
                detail: "Target entity IDs must be unique and non-empty.".to_owned(),
            });
        }
    }
    if targets.is_empty() {
        failures.push(ScanBimAdapterFailure {
            code: "SCAN_BIM_TARGETS_EMPTY".to_owned(),
            path: "task.target_entity_ids".to_owned(),
            detail: "At least one target entity is required.".to_owned(),
        });
    }

    let mut state_ids = BTreeSet::new();
    for state in &case.possible_states {
        if state.state_id.trim().is_empty()
            || !state_ids.insert(state.state_id.as_str())
            || state.interpretation.trim().is_empty()
            || !state.property_value_m.is_finite()
        {
            failures.push(ScanBimAdapterFailure {
                code: "SCAN_BIM_HYPOTHESIS_INVALID".to_owned(),
                path: "possible_states".to_owned(),
                detail: "Hypothesis identities, interpretations and finite property values are required.".to_owned(),
            });
        }
        let meets_requirement = match case.task.comparison {
            RequirementComparison::GreaterThanOrEqual => {
                state.property_value_m >= case.task.threshold_m
            }
            RequirementComparison::LessThanOrEqual => {
                state.property_value_m <= case.task.threshold_m
            }
        };
        if meets_requirement != state.requirement_satisfied {
            failures.push(ScanBimAdapterFailure {
                code: "SCAN_BIM_HYPOTHESIS_REQUIREMENT_CONTRADICTION".to_owned(),
                path: "possible_states".to_owned(),
                detail: format!(
                    "State {:?} declares requirement_satisfied={} but property value {} and threshold {} imply {}.",
                    state.state_id,
                    state.requirement_satisfied,
                    state.property_value_m,
                    case.task.threshold_m,
                    meets_requirement
                ),
            });
        }
    }
    if case.possible_states.is_empty() || state_ids.len() != case.possible_states.len() {
        failures.push(ScanBimAdapterFailure {
            code: "SCAN_BIM_HYPOTHESIS_SET_INVALID".to_owned(),
            path: "possible_states".to_owned(),
            detail: "Possible-state set must be non-empty and have unique IDs.".to_owned(),
        });
    }

    let mut action_ids = BTreeSet::new();
    for action in &case.actions {
        if action.action_id.trim().is_empty() || !action_ids.insert(action.action_id.as_str()) {
            failures.push(ScanBimAdapterFailure {
                code: "SCAN_BIM_ACTION_ID_INVALID".to_owned(),
                path: "actions".to_owned(),
                detail: "Action IDs must be unique and non-empty.".to_owned(),
            });
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        failures.sort_by(|left, right| {
            (&left.path, &left.code, &left.detail).cmp(&(&right.path, &right.code, &right.detail))
        });
        Err(failures)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        EMBEDDED_SCAN_BIM_DECISION_CASE, EMBEDDED_SCAN_BIM_DECISION_SCHEMA,
        evaluate_scan_bim_decision_bytes,
    };
    use serde_json::Value;

    fn evaluate(value: &Value) -> super::ScanBimDecisionReport {
        let bytes = serde_json::to_vec(value).expect("serialize Scan-to-BIM decision case");
        evaluate_scan_bim_decision_bytes(
            &bytes,
            EMBEDDED_SCAN_BIM_DECISION_SCHEMA.as_bytes(),
        )
        .expect("valid Scan-to-BIM decision adapter case")
    }

    fn fixture_value() -> Value {
        serde_json::from_str(EMBEDDED_SCAN_BIM_DECISION_CASE)
            .expect("embedded Scan-to-BIM decision fixture")
    }

    #[test]
    fn targeted_oblique_capture_precedes_expensive_full_room_sweep() {
        let report = evaluate(&fixture_value());
        assert_eq!(
            report.decision.recommended_action_order,
            vec![
                "targeted-oblique-clearance-scan",
                "full-room-tls-sweep"
            ]
        );
        assert_eq!(report.decision.case_id, "SCAN-BIM-DOOR-CLEARANCE-001");
        assert_eq!(report.evidence_qualified_action_count, 3);
        assert_eq!(report.decision_accepted_action_count, 3);
        assert_eq!(report.decision_relevant_action_count, 2);
        assert_eq!(report.decision_rejected_action_count, 1);
    }

    #[test]
    fn dense_frontal_scan_that_does_not_partition_the_decision_is_not_recommended() {
        let report = evaluate(&fixture_value());
        let wall = report.decision.actions.iter()
            .find(|action| action.action_id == "dense-frontal-wall-scan")
            .expect("wall scan action");
        assert!(!wall.decision_relevant);
        assert!(!report.decision.recommended_action_order.contains(&wall.action_id));
    }

    #[test]
    fn self_occluded_scan_is_rejected_before_decision_evaluation() {
        let report = evaluate(&fixture_value());
        let action = report.decision.actions.iter()
            .find(|action| action.action_id == "blocked-backside-scan")
            .expect("blocked scan action");
        assert_eq!(action.status, "reject_action");
        assert!(action.rejection_codes.contains(&"ACTION_EVIDENCE_NOT_VALID".to_owned()));
        let qualification = report.action_qualifications.iter()
            .find(|item| item.action_id == action.action_id)
            .expect("blocked scan qualification");
        assert!(qualification.rejection_codes.contains(&"TARGET_NOT_THEORETICALLY_OBSERVABLE".to_owned()));
    }

    #[test]
    fn stale_model_revision_is_rejected_by_shared_decision_engine() {
        let mut value = fixture_value();
        value["actions"][0]["model_revision"] = Value::String("MODEL-REV-STALE".to_owned());
        let report = evaluate(&value);
        let action = report.decision.actions.iter()
            .find(|action| action.action_id == "targeted-oblique-clearance-scan")
            .expect("targeted scan action");
        assert_eq!(action.status, "reject_action");
        assert!(action.rejection_codes.contains(&"ACTION_REVISION_MISMATCH".to_owned()));
    }

    #[test]
    fn insufficient_visible_area_is_rejected_against_task_requirement() {
        let mut value = fixture_value();
        value["actions"][0]["target_visibility"][0]["observable_area_m2"] = Value::from(0.4);
        let report = evaluate(&value);
        let qualification = report.action_qualifications.iter()
            .find(|item| item.action_id == "targeted-oblique-clearance-scan")
            .expect("targeted scan qualification");
        assert!(qualification.rejection_codes.contains(&"TARGET_COVERAGE_BELOW_TASK_THRESHOLD".to_owned()));
    }

    #[test]
    fn incomplete_target_visibility_is_rejected() {
        let mut value = fixture_value();
        value["task"]["target_entity_ids"] = serde_json::json!(["DOOR-001", "DOOR-002"]);
        let report = evaluate(&value);
        let qualification = report.action_qualifications.iter()
            .find(|item| item.action_id == "targeted-oblique-clearance-scan")
            .expect("targeted scan qualification");
        assert!(qualification.rejection_codes.contains(&"TARGET_VISIBILITY_INCOMPLETE".to_owned()));
    }

    #[test]
    fn incomplete_outcome_partition_is_rejected_by_shared_engine() {
        let mut value = fixture_value();
        value["actions"][0]["outcomes"].as_array_mut().expect("outcome array").pop();
        let report = evaluate(&value);
        let action = report.decision.actions.iter()
            .find(|action| action.action_id == "targeted-oblique-clearance-scan")
            .expect("targeted action");
        assert_eq!(action.status, "reject_action");
        assert!(action.rejection_codes.contains(&"ACTION_PARTITION_INCOMPLETE".to_owned()));
    }

    #[test]
    fn resolving_door_width_does_not_clear_unrelated_structural_unknown() {
        let mut value = fixture_value();
        value["task"]["unresolved_critical_unknown_ids"] =
            serde_json::json!(["door-clear-width", "structural-load-review"]);
        let report = evaluate(&value);
        let action = report.decision.actions.iter()
            .find(|action| action.action_id == "targeted-oblique-clearance-scan")
            .expect("targeted scan action");
        assert!(action.changes_critical_unknown_set);
        assert!(!action.changes_hard_feasible_set);
        assert!(!action.changes_pareto_set);
        assert!(action.outcomes.iter().all(|outcome| {
            outcome.remaining_critical_unknown_ids == vec!["structural-load-review"]
                && outcome.robust_feasible_candidate_ids.is_empty()
        }));
    }

    #[test]
    fn input_array_permutations_preserve_scan_bim_decision_result() {
        let original = evaluate(&fixture_value());
        let mut value = fixture_value();
        value["possible_states"].as_array_mut().unwrap().reverse();
        value["current_possible_state_ids"].as_array_mut().unwrap().reverse();
        value["candidates"].as_array_mut().unwrap().reverse();
        for candidate in value["candidates"].as_array_mut().unwrap() {
            candidate["state_results"].as_array_mut().unwrap().reverse();
        }
        value["actions"].as_array_mut().unwrap().reverse();
        for action in value["actions"].as_array_mut().unwrap() {
            action["outcomes"].as_array_mut().unwrap().reverse();
            action["target_visibility"].as_array_mut().unwrap().reverse();
        }
        let reordered = evaluate(&value);
        assert_eq!(original.decision, reordered.decision);
        assert_eq!(original.action_qualifications, reordered.action_qualifications);
    }

    #[test]
    fn contradictory_hypothesis_requirement_flag_is_rejected() {
        let mut value = fixture_value();
        value["possible_states"][0]["requirement_satisfied"] = Value::Bool(false);
        let bytes = serde_json::to_vec(&value).expect("serialize contradictory hypothesis");
        let errors = evaluate_scan_bim_decision_bytes(
            &bytes,
            EMBEDDED_SCAN_BIM_DECISION_SCHEMA.as_bytes(),
        )
        .expect_err("requirement flag must match the numerical hypothesis and threshold");
        assert!(errors.iter().any(
            |error| error.code == "SCAN_BIM_HYPOTHESIS_REQUIREMENT_CONTRADICTION"
        ));
    }

    #[test]
    fn unknown_probability_fields_fail_closed() {
        let mut value = fixture_value();
        value["actions"][0]["probability"] = Value::from(0.9);
        let bytes = serde_json::to_vec(&value).expect("serialize malformed case");
        let errors = evaluate_scan_bim_decision_bytes(
            &bytes,
            EMBEDDED_SCAN_BIM_DECISION_SCHEMA.as_bytes(),
        )
        .expect_err("probability must not be silently accepted");
        assert!(errors.iter().any(|error| error.code == "SCAN_BIM_SCHEMA_VALIDATION_FAILED"));
    }
}
