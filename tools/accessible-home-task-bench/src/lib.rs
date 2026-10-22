// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic triage of declared evidence for synthetic resident task chains.
//! This is not a physical-home simulator or a real-building qualification result.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const RULE_VERSION: &str = "accessible-home-task-bench/0.1.0";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Fixture {
    pub schema_version: String,
    pub fixture_kind: String,
    pub fixture_id: String,
    pub claim_boundary: String,
    pub scenarios: Vec<Scenario>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Scenario {
    pub id: String,
    pub title: String,
    pub selected_needs: Vec<String>,
    pub context: String,
    pub expected_assessment: Assessment,
    pub tasks: Vec<Task>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub expected_assessment: Assessment,
    pub steps: Vec<TaskStep>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct TaskStep {
    pub id: String,
    pub title: String,
    pub capabilities: Vec<CapabilityEvidence>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct CapabilityEvidence {
    pub id: String,
    pub required: bool,
    pub evidence_state: EvidenceState,
    pub evidence_ref: String,
    pub review_with: String,
    pub note: String,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceState {
    VerifiedForDeclaredScope,
    KnownBarrier,
    Unknown,
    Proposed,
    NeedsSiteMeasurement,
    NeedsProfessionalReview,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Assessment {
    BlockedByDeclaredBarrier,
    NotEvaluable,
    EvidenceCompleteForFixture,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FindingState {
    VerifiedForFixtureOnly,
    RequiredBarrier,
    RequiredEvidenceGap,
    OptionalAdvisory,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct CapabilityFinding {
    pub step_id: String,
    pub capability_id: String,
    pub required: bool,
    pub evidence_state: EvidenceState,
    pub finding_state: FindingState,
    pub evidence_ref: String,
    pub review_with: String,
    pub note: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct TaskResult {
    pub id: String,
    pub title: String,
    pub expected_assessment: Assessment,
    pub actual_assessment: Assessment,
    pub findings: Vec<CapabilityFinding>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ScenarioResult {
    pub id: String,
    pub title: String,
    pub selected_needs: Vec<String>,
    pub expected_assessment: Assessment,
    pub actual_assessment: Assessment,
    pub tasks: Vec<TaskResult>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Report {
    pub report_schema_version: String,
    pub rule_version: String,
    pub fixture_id: String,
    pub fixture_kind: String,
    pub input_sha256: String,
    pub claim_boundary: String,
    pub scenario_count: usize,
    pub task_count: usize,
    pub result_counts: ResultCounts,
    pub scenarios: Vec<ScenarioResult>,
    pub claim_limit: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ResultCounts {
    // Counts scenario-level assessments, not individual capability rows.
    pub blocked_by_declared_barrier: usize,
    pub not_evaluable: usize,
    pub evidence_complete_for_fixture: usize,
}
impl ResultCounts {
    fn from_results(results: &[ScenarioResult]) -> Self {
        let mut counts = Self {
            blocked_by_declared_barrier: 0,
            not_evaluable: 0,
            evidence_complete_for_fixture: 0,
        };
        for result in results {
            match result.actual_assessment {
                Assessment::BlockedByDeclaredBarrier => counts.blocked_by_declared_barrier += 1,
                Assessment::NotEvaluable => counts.not_evaluable += 1,
                Assessment::EvidenceCompleteForFixture => counts.evidence_complete_for_fixture += 1,
            }
        }
        counts
    }
}

pub fn evaluate_fixture_bytes(input: &[u8]) -> Result<Report, String> {
    let fixture: Fixture = serde_json::from_slice(input)
        .map_err(|error| format!("invalid fixture JSON/schema: {error}"))?;
    validate_fixture(&fixture)?;
    let report = evaluate_fixture(&fixture, input);
    for scenario in &report.scenarios {
        if scenario.expected_assessment != scenario.actual_assessment {
            return Err(format!(
                "scenario '{}' expected {:?}, evaluator produced {:?}",
                scenario.id, scenario.expected_assessment, scenario.actual_assessment
            ));
        }
        for task in &scenario.tasks {
            if task.expected_assessment != task.actual_assessment {
                return Err(format!(
                    "task '{}::{}' expected {:?}, evaluator produced {:?}",
                    scenario.id, task.id, task.expected_assessment, task.actual_assessment
                ));
            }
        }
    }
    Ok(report)
}

pub fn validate_fixture(fixture: &Fixture) -> Result<(), String> {
    if fixture.schema_version != "0.1.0" {
        return Err(format!("unsupported schema_version '{}'", fixture.schema_version));
    }
    if fixture.fixture_kind != "synthetic_benchmark" {
        return Err("fixture_kind must be synthetic_benchmark; real-home evidence is not accepted by this harness".to_owned());
    }
    require_text("fixture_id", &fixture.fixture_id)?;
    require_text("claim_boundary", &fixture.claim_boundary)?;
    if fixture.scenarios.is_empty() {
        return Err("fixture must contain at least one scenario".to_owned());
    }
    let mut scenario_ids = BTreeSet::new();
    for scenario in &fixture.scenarios {
        require_text("scenario.id", &scenario.id)?;
        require_text("scenario.title", &scenario.title)?;
        require_text("scenario.context", &scenario.context)?;
        if !scenario_ids.insert(scenario.id.as_str()) {
            return Err(format!("duplicate scenario id '{}'", scenario.id));
        }
        if scenario.tasks.is_empty() {
            return Err(format!("scenario '{}' has no tasks", scenario.id));
        }
        let mut task_ids = BTreeSet::new();
        for task in &scenario.tasks {
            require_text("task.id", &task.id)?;
            require_text("task.title", &task.title)?;
            if !task_ids.insert(task.id.as_str()) {
                return Err(format!("duplicate task id '{}' in scenario '{}'", task.id, scenario.id));
            }
            if task.steps.is_empty() {
                return Err(format!("task '{}::{}' has no steps", scenario.id, task.id));
            }
            let mut step_ids = BTreeSet::new();
            let mut required_count = 0_usize;
            for step in &task.steps {
                require_text("step.id", &step.id)?;
                require_text("step.title", &step.title)?;
                if !step_ids.insert(step.id.as_str()) {
                    return Err(format!("duplicate step id '{}' in task '{}::{}'", step.id, scenario.id, task.id));
                }
                if step.capabilities.is_empty() {
                    return Err(format!("step '{}::{}::{}' has no capability evidence", scenario.id, task.id, step.id));
                }
                let mut capability_ids = BTreeSet::new();
                for capability in &step.capabilities {
                    require_text("capability.id", &capability.id)?;
                    require_text("capability.evidence_ref", &capability.evidence_ref)?;
                    require_text("capability.review_with", &capability.review_with)?;
                    require_text("capability.note", &capability.note)?;
                    if !capability_ids.insert(capability.id.as_str()) {
                        return Err(format!("duplicate capability id '{}' in step '{}::{}::{}'", capability.id, scenario.id, task.id, step.id));
                    }
                    if capability.required {
                        required_count += 1;
                    }
                }
            }
            if required_count == 0 {
                return Err(format!("task '{}::{}' has no required capabilities", scenario.id, task.id));
            }
        }
    }
    Ok(())
}

pub fn assess_task(task: &Task) -> Assessment {
    let mut has_required_barrier = false;
    let mut has_required_gap = false;
    for capability in task.steps.iter().flat_map(|step| &step.capabilities).filter(|item| item.required) {
        match capability.evidence_state {
            EvidenceState::KnownBarrier => has_required_barrier = true,
            EvidenceState::VerifiedForDeclaredScope => {}
            EvidenceState::Unknown
            | EvidenceState::Proposed
            | EvidenceState::NeedsSiteMeasurement
            | EvidenceState::NeedsProfessionalReview => has_required_gap = true,
        }
    }
    if has_required_barrier {
        Assessment::BlockedByDeclaredBarrier
    } else if has_required_gap {
        Assessment::NotEvaluable
    } else {
        Assessment::EvidenceCompleteForFixture
    }
}

fn assess_scenario(tasks: &[TaskResult]) -> Assessment {
    if tasks.iter().any(|task| task.actual_assessment == Assessment::BlockedByDeclaredBarrier) {
        Assessment::BlockedByDeclaredBarrier
    } else if tasks.iter().any(|task| task.actual_assessment == Assessment::NotEvaluable) {
        Assessment::NotEvaluable
    } else {
        Assessment::EvidenceCompleteForFixture
    }
}

fn evaluate_fixture(fixture: &Fixture, input: &[u8]) -> Report {
    let scenarios = fixture.scenarios.iter().map(|scenario| {
        let tasks = scenario.tasks.iter().map(|task| {
            let findings = task.steps.iter().flat_map(|step| {
                step.capabilities.iter().map(move |capability| {
                    let finding_state = if !capability.required {
                        FindingState::OptionalAdvisory
                    } else {
                        match capability.evidence_state {
                            EvidenceState::KnownBarrier => FindingState::RequiredBarrier,
                            EvidenceState::VerifiedForDeclaredScope => FindingState::VerifiedForFixtureOnly,
                            EvidenceState::Unknown
                            | EvidenceState::Proposed
                            | EvidenceState::NeedsSiteMeasurement
                            | EvidenceState::NeedsProfessionalReview => FindingState::RequiredEvidenceGap,
                        }
                    };
                    CapabilityFinding {
                        step_id: step.id.clone(),
                        capability_id: capability.id.clone(),
                        required: capability.required,
                        evidence_state: capability.evidence_state,
                        finding_state,
                        evidence_ref: capability.evidence_ref.clone(),
                        review_with: capability.review_with.clone(),
                        note: capability.note.clone(),
                    }
                })
            }).collect();
            TaskResult {
                id: task.id.clone(),
                title: task.title.clone(),
                expected_assessment: task.expected_assessment,
                actual_assessment: assess_task(task),
                findings,
            }
        }).collect::<Vec<_>>();
        ScenarioResult {
            id: scenario.id.clone(),
            title: scenario.title.clone(),
            selected_needs: scenario.selected_needs.clone(),
            expected_assessment: scenario.expected_assessment,
            actual_assessment: assess_scenario(&tasks),
            tasks,
        }
    }).collect::<Vec<_>>();
    let task_count = scenarios.iter().map(|scenario| scenario.tasks.len()).sum();
    Report {
        report_schema_version: "0.1.0".to_owned(),
        rule_version: RULE_VERSION.to_owned(),
        fixture_id: fixture.fixture_id.clone(),
        fixture_kind: fixture.fixture_kind.clone(),
        input_sha256: hex_digest(input),
        claim_boundary: fixture.claim_boundary.clone(),
        scenario_count: scenarios.len(),
        task_count,
        result_counts: ResultCounts::from_results(&scenarios),
        scenarios,
        claim_limit: "SYNTHETIC FIXTURE ONLY: evidence_complete_for_fixture does not certify or establish the safety, accessibility, usability, compliance, dimensions, or suitability of any real home, product, alarm, route, or building.".to_owned(),
    }
}

fn hex_digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}
fn require_text(field: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() { Err(format!("{field} must be non-empty")) } else { Ok(()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE: &str = include_str!("../fixtures/accessibility-task-benchmark-v0.1.json");

    fn capability(state: EvidenceState, required: bool, id: String) -> CapabilityEvidence {
        CapabilityEvidence {
            id, required, evidence_state: state,
            evidence_ref: "synthetic://test/capability".to_owned(),
            review_with: "resident and qualified reviewer".to_owned(),
            note: "synthetic test assertion; not a real building observation".to_owned(),
        }
    }
    fn task_with(states: &[(EvidenceState, bool)]) -> Task {
        Task {
            id: "task".to_owned(), title: "Synthetic test task".to_owned(),
            expected_assessment: Assessment::NotEvaluable,
            steps: vec![TaskStep {
                id: "step".to_owned(), title: "Synthetic step".to_owned(),
                capabilities: states.iter().enumerate().map(|(index,(state,required))|
                    capability(*state,*required,format!("capability-{index}"))
                ).collect(),
            }],
        }
    }

    #[test]
    fn committed_fixture_reproduces_every_expected_assessment() {
        let report = evaluate_fixture_bytes(FIXTURE.as_bytes()).expect("fixture must qualify");
        assert_eq!(report.scenario_count, 9);
        assert_eq!(report.task_count, 9);
        assert_eq!(report.result_counts.blocked_by_declared_barrier, 2);
        assert_eq!(report.result_counts.not_evaluable, 6);
        assert_eq!(report.result_counts.evidence_complete_for_fixture, 1);
    }
    #[test]
    fn known_required_barrier_takes_precedence_over_unknown_evidence() {
        assert_eq!(assess_task(&task_with(&[(EvidenceState::Unknown,true),(EvidenceState::KnownBarrier,true)])), Assessment::BlockedByDeclaredBarrier);
    }
    #[test]
    fn required_unknown_is_not_treated_as_verified() {
        assert_eq!(assess_task(&task_with(&[(EvidenceState::Unknown,true)])), Assessment::NotEvaluable);
    }
    #[test]
    fn site_measurement_and_professional_review_are_not_passes() {
        for state in [EvidenceState::NeedsSiteMeasurement,EvidenceState::NeedsProfessionalReview] {
            assert_eq!(assess_task(&task_with(&[(state,true)])), Assessment::NotEvaluable);
        }
    }
    #[test]
    fn proposed_capability_is_not_a_pass() {
        assert_eq!(assess_task(&task_with(&[(EvidenceState::Proposed,true)])), Assessment::NotEvaluable);
    }
    #[test]
    fn all_required_verified_states_are_only_fixture_complete() {
        assert_eq!(assess_task(&task_with(&[(EvidenceState::VerifiedForDeclaredScope,true),(EvidenceState::VerifiedForDeclaredScope,true)])), Assessment::EvidenceCompleteForFixture);
    }
    #[test]
    fn optional_barrier_does_not_block_a_declared_task() {
        assert_eq!(assess_task(&task_with(&[(EvidenceState::VerifiedForDeclaredScope,true),(EvidenceState::KnownBarrier,false)])), Assessment::EvidenceCompleteForFixture);
    }
    #[test]
    fn real_home_fixture_kind_is_rejected() {
        let mut fixture: Fixture = serde_json::from_str(FIXTURE).expect("valid baseline fixture");
        fixture.fixture_kind = "real_home".to_owned();
        assert!(validate_fixture(&fixture).unwrap_err().contains("synthetic_benchmark"));
    }
    #[test]
    fn duplicate_scenario_identifiers_are_rejected() {
        let mut fixture: Fixture = serde_json::from_str(FIXTURE).expect("valid baseline fixture");
        fixture.scenarios[1].id = fixture.scenarios[0].id.clone();
        assert!(validate_fixture(&fixture).unwrap_err().contains("duplicate scenario id"));
    }
    #[test]
    fn missing_evidence_provenance_is_rejected() {
        let mut fixture: Fixture = serde_json::from_str(FIXTURE).expect("valid baseline fixture");
        fixture.scenarios[0].tasks[0].steps[0].capabilities[0].evidence_ref.clear();
        assert!(validate_fixture(&fixture).unwrap_err().contains("evidence_ref must be non-empty"));
    }
    #[test]
    fn evaluator_receipt_is_deterministic_for_identical_input_bytes() {
        let first = evaluate_fixture_bytes(FIXTURE.as_bytes()).expect("valid fixture");
        let second = evaluate_fixture_bytes(FIXTURE.as_bytes()).expect("valid fixture");
        assert_eq!(first, second);
        assert_eq!(serde_json::to_vec(&first).unwrap(),serde_json::to_vec(&second).unwrap());
        assert_eq!(first.input_sha256,hex_digest(FIXTURE.as_bytes()));
    }
    #[test]
    fn expected_status_mismatch_fails_closed() {
        let mut fixture: Fixture = serde_json::from_str(FIXTURE).expect("valid baseline fixture");
        fixture.scenarios[0].expected_assessment = Assessment::EvidenceCompleteForFixture;
        let bytes = serde_json::to_vec(&fixture).expect("serialize modified fixture");
        assert!(evaluate_fixture_bytes(&bytes).unwrap_err().contains("expected"));
    }
    #[test]
    fn unknown_evidence_state_is_rejected_by_schema_parser() {
        let invalid = FIXTURE.replace("verified_for_declared_scope","probably_safe");
        assert!(evaluate_fixture_bytes(invalid.as_bytes()).unwrap_err().contains("invalid fixture JSON/schema"));
    }
}
