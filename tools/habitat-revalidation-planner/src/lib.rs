// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const REPORT_SCHEMA: &str = "luminous.habitat_revalidation_report.v0.1";
pub const CLAIM_CEILING: &str = "synthetic_revalidation_planning_not_real_world_authority";
const FIXTURE_SHA256: &str = "d7f8aa78dc81dac52752916c85955ce697928f374670f9317591c15d84c76758";
const ALLOWED_ARTIFACT_STATUSES: [&str; 7] = [
    "valid_for_claim", "unknown", "stale", "revoked", "incomplete_coverage",
    "contradictory", "out_of_scope",
];

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Fixture {
    pub schema_version: String,
    pub fixture_id: String,
    pub status: String,
    pub synthetic_only: bool,
    pub claim_ceiling: String,
    pub intended_use: String,
    pub target_artifact_id: String,
    pub artifacts: Vec<ArtifactState>,
    pub declared_gate_ids: Vec<String>,
    pub available_gate_ids: Vec<String>,
    pub tasks: Vec<RepairTask>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ArtifactState {
    pub id: String,
    pub status: String,
    pub depends_on: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RepairTask {
    pub id: String,
    pub version: u64,
    pub produces_artifact_id: String,
    pub required_artifact_ids: Vec<String>,
    pub required_gate_ids: Vec<String>,
    pub cost_units: u64,
    pub duration_units: u64,
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PlanStep {
    pub task_id: String,
    pub task_version: u64,
    pub produces_artifact_id: String,
    pub required_artifact_ids: Vec<String>,
    pub required_gate_ids: Vec<String>,
    pub cost_units: u64,
    pub duration_units: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Plan {
    pub task_ids: Vec<String>,
    pub task_steps: Vec<PlanStep>,
    pub target_artifact_id_to_restore: String,
    pub execution_status: String,
    pub cost_units: u64,
    pub duration_units: u64,
    pub task_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Report {
    pub schema_version: String,
    pub fixture_id: String,
    pub input_sha256: String,
    pub verification_status: String,
    pub claim_ceiling: String,
    pub target_artifact_id: String,
    pub planning_status: String,
    pub artifact_count: usize,
    pub task_count: usize,
    pub enumerated_feasible_plan_count: usize,
    pub pareto_plan_count: usize,
    pub pareto_plans: Vec<Plan>,
    pub blocking_reasons: Vec<String>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn verify_fixture_bytes(bytes: &[u8]) -> Result<Report, Vec<String>> {
    let digest = sha256_hex(bytes);
    if digest != FIXTURE_SHA256 {
        return Err(vec![format!(
            "frozen revalidation fixture digest mismatch: expected {FIXTURE_SHA256}, got {digest}"
        )]);
    }
    let fixture: Fixture = serde_json::from_slice(bytes)
        .map_err(|error| vec![format!("revalidation fixture rejected: {error}")])?;
    evaluate_fixture(&fixture, digest)
}

fn make_plan(
    fixture: &Fixture,
    target: &str,
    task_ids: &[String],
    cost_units: u64,
    duration_units: u64,
) -> Plan {
    let task_map: BTreeMap<&str, &RepairTask> = fixture
        .tasks
        .iter()
        .map(|task| (task.id.as_str(), task))
        .collect();
    let task_steps = task_ids
        .iter()
        .filter_map(|id| task_map.get(id.as_str()))
        .map(|task| PlanStep {
            task_id: task.id.clone(),
            task_version: task.version,
            produces_artifact_id: task.produces_artifact_id.clone(),
            required_artifact_ids: {
                let mut ids = task.required_artifact_ids.clone();
                ids.sort();
                ids
            },
            required_gate_ids: {
                let mut ids = task.required_gate_ids.clone();
                ids.sort();
                ids
            },
            cost_units: task.cost_units,
            duration_units: task.duration_units,
        })
        .collect();
    Plan {
        task_ids: task_ids.to_vec(),
        task_steps,
        target_artifact_id_to_restore: target.to_owned(),
        execution_status: "PROPOSED_NOT_EXECUTED".to_owned(),
        cost_units,
        duration_units,
        task_count: task_ids.len(),
    }
}

fn evaluate_fixture(fixture: &Fixture, input_sha256: String) -> Result<Report, Vec<String>> {
    let errors = validate_fixture(fixture);
    if !errors.is_empty() {
        return Err(errors);
    }

    let initially_valid: BTreeSet<String> = fixture
        .artifacts
        .iter()
        .filter(|artifact| artifact.status == "valid_for_claim")
        .map(|artifact| artifact.id.clone())
        .collect();
    if initially_valid.contains(&fixture.target_artifact_id) {
        let mut plan = make_plan(fixture, &fixture.target_artifact_id, &[], 0, 0);
        plan.execution_status = "TARGET_ALREADY_VALID".to_owned();
        return Ok(Report {
            schema_version: REPORT_SCHEMA.to_owned(),
            fixture_id: fixture.fixture_id.clone(),
            input_sha256,
            verification_status: "SYNTHETIC_PLAN_CONTRACT_ONLY".to_owned(),
            claim_ceiling: CLAIM_CEILING.to_owned(),
            target_artifact_id: fixture.target_artifact_id.clone(),
            planning_status: "TARGET_ALREADY_VALID".to_owned(),
            artifact_count: fixture.artifacts.len(),
            task_count: fixture.tasks.len(),
            enumerated_feasible_plan_count: 1,
            pareto_plan_count: 1,
            pareto_plans: vec![plan],
            blocking_reasons: Vec::new(),
        });
    }

    let mut feasible_plans = Vec::new();
    enumerate_plans(
        fixture,
        &fixture.target_artifact_id,
        &initially_valid,
        &mut BTreeSet::new(),
        &mut Vec::new(),
        0,
        0,
        &mut feasible_plans,
    );
    feasible_plans.sort_by(compare_plans);
    feasible_plans.dedup_by(|left, right| left.task_ids == right.task_ids);
    let frontier = pareto_frontier(&feasible_plans);

    let mut blockers = Vec::new();
    if frontier.is_empty() {
        for task in &fixture.tasks {
            let missing: Vec<&str> = task
                .required_gate_ids
                .iter()
                .filter(|gate| !fixture.available_gate_ids.contains(gate))
                .map(String::as_str)
                .collect();
            if !missing.is_empty() {
                blockers.push(format!(
                    "{} blocked by unavailable required gates: {}",
                    task.id, missing.join(", ")
                ));
            }
        }
        if blockers.is_empty() {
            blockers.push(
                "no dependency-valid task sequence can restore the requested target".to_owned(),
            );
        }
        blockers.sort();
        blockers.dedup();
    }

    Ok(Report {
        schema_version: REPORT_SCHEMA.to_owned(),
        fixture_id: fixture.fixture_id.clone(),
        input_sha256,
        verification_status: "SYNTHETIC_PLAN_CONTRACT_ONLY".to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        target_artifact_id: fixture.target_artifact_id.clone(),
        planning_status: if frontier.is_empty() {
            "BLOCKED_NO_VALID_PLAN".to_owned()
        } else {
            "PLAN_AVAILABLE".to_owned()
        },
        artifact_count: fixture.artifacts.len(),
        task_count: fixture.tasks.len(),
        enumerated_feasible_plan_count: feasible_plans.len(),
        pareto_plan_count: frontier.len(),
        pareto_plans: frontier,
        blocking_reasons: blockers,
    })
}

#[allow(clippy::too_many_arguments)]
fn enumerate_plans(
    fixture: &Fixture,
    target: &str,
    valid_artifacts: &BTreeSet<String>,
    used_tasks: &mut BTreeSet<String>,
    current_task_ids: &mut Vec<String>,
    cost: u64,
    duration: u64,
    results: &mut Vec<Plan>,
) {
    let mut tasks: Vec<&RepairTask> = fixture
        .tasks
        .iter()
        .collect();
    tasks.sort_by(|left, right| left.id.cmp(&right.id));
    for task in tasks {
        if used_tasks.contains(&task.id)
            || valid_artifacts.contains(&task.produces_artifact_id)
            || !task
                .required_artifact_ids
                .iter()
                .all(|id| valid_artifacts.contains(id))
            || !task
                .required_gate_ids
                .iter()
                .all(|id| fixture.available_gate_ids.contains(id))
        {
            continue;
        }
        let Some(next_cost) = cost.checked_add(task.cost_units) else {
            continue;
        };
        let Some(next_duration) = duration.checked_add(task.duration_units) else {
            continue;
        };
        used_tasks.insert(task.id.clone());
        current_task_ids.push(task.id.clone());
        let mut next_valid = valid_artifacts.clone();
        next_valid.insert(task.produces_artifact_id.clone());
        if task.produces_artifact_id == target {
            results.push(make_plan(
                fixture,
                target,
                current_task_ids,
                next_cost,
                next_duration,
            ));
        } else {
            enumerate_plans(
                fixture,
                target,
                &next_valid,
                used_tasks,
                current_task_ids,
                next_cost,
                next_duration,
                results,
            );
        }
        current_task_ids.pop();
        used_tasks.remove(&task.id);
    }
}

fn compare_plans(left: &Plan, right: &Plan) -> std::cmp::Ordering {
    left.cost_units
        .cmp(&right.cost_units)
        .then_with(|| left.duration_units.cmp(&right.duration_units))
        .then_with(|| left.task_ids.cmp(&right.task_ids))
}

fn pareto_frontier(plans: &[Plan]) -> Vec<Plan> {
    let mut frontier: Vec<Plan> = plans
        .iter()
        .filter(|candidate| {
            !plans.iter().any(|other| {
                other.cost_units <= candidate.cost_units
                    && other.duration_units <= candidate.duration_units
                    && (other.cost_units < candidate.cost_units
                        || other.duration_units < candidate.duration_units)
            })
        })
        .cloned()
        .collect();
    frontier.sort_by(compare_plans);
    frontier.dedup_by(|left, right| left.task_ids == right.task_ids);
    frontier
}

fn validate_fixture(fixture: &Fixture) -> Vec<String> {
    let mut errors = Vec::new();
    if fixture.schema_version != "luminous.habitat_revalidation_fixture.v0.1" {
        errors.push(format!("unsupported schema_version {:?}", fixture.schema_version));
    }
    if fixture.fixture_id.trim().is_empty() || fixture.intended_use.trim().is_empty() {
        errors.push("fixture ID and intended use must not be blank".to_owned());
    }
    if fixture.status != "proposed_synthetic_revalidation_planner_benchmark" {
        errors.push(format!("unsupported status {:?}", fixture.status));
    }
    if !fixture.synthetic_only {
        errors.push("synthetic_only must remain true".to_owned());
    }
    if fixture.claim_ceiling != CLAIM_CEILING {
        errors.push(format!("claim ceiling must be {CLAIM_CEILING:?}"));
    }

    let mut artifacts = BTreeMap::new();
    for artifact in &fixture.artifacts {
        if artifact.id.trim().is_empty() {
            errors.push("artifact ID must not be blank".to_owned());
        }
        if artifacts.insert(artifact.id.as_str(), artifact).is_some() {
            errors.push(format!("duplicate artifact ID {:?}", artifact.id));
        }
        if !ALLOWED_ARTIFACT_STATUSES.contains(&artifact.status.as_str()) {
            errors.push(format!(
                "{} has unsupported status {:?}",
                artifact.id, artifact.status
            ));
        }
        let mut deps = BTreeSet::new();
        for dep in &artifact.depends_on {
            if dep == &artifact.id {
                errors.push(format!("{} cannot depend on itself", artifact.id));
            }
            if !deps.insert(dep.as_str()) {
                errors.push(format!("{} repeats dependency {:?}", artifact.id, dep));
            }
        }
    }
    if !artifacts.contains_key(fixture.target_artifact_id.as_str()) {
        errors.push(format!("unknown target artifact {:?}", fixture.target_artifact_id));
    }
    for artifact in &fixture.artifacts {
        for dependency in &artifact.depends_on {
            if !artifacts.contains_key(dependency.as_str()) {
                errors.push(format!(
                    "{} references unknown dependency {:?}",
                    artifact.id, dependency
                ));
            } else if artifact.status == "valid_for_claim"
                && artifacts
                    .get(dependency.as_str())
                    .is_some_and(|dep| dep.status != "valid_for_claim")
            {
                errors.push(format!(
                    "{} cannot remain valid_for_claim while dependency {} is not valid",
                    artifact.id, dependency
                ));
            }
        }
    }

    let declared_gates: BTreeSet<&str> = fixture
        .declared_gate_ids
        .iter()
        .map(String::as_str)
        .collect();
    if declared_gates.len() != fixture.declared_gate_ids.len() {
        errors.push("declared gate IDs must be unique".to_owned());
    }
    let available_gates: BTreeSet<&str> = fixture
        .available_gate_ids
        .iter()
        .map(String::as_str)
        .collect();
    if available_gates.len() != fixture.available_gate_ids.len() {
        errors.push("available gate IDs must be unique".to_owned());
    }
    for gate in &available_gates {
        if !declared_gates.contains(gate) {
            errors.push(format!("available gate {:?} was not declared", gate));
        }
    }

    let mut task_ids = BTreeSet::new();
    for task in &fixture.tasks {
        if task.id.trim().is_empty() || task.description.trim().is_empty() {
            errors.push("task ID and description must not be blank".to_owned());
        }
        if !task_ids.insert(task.id.as_str()) {
            errors.push(format!("duplicate task ID {:?}", task.id));
        }
        if task.version == 0 || task.cost_units == 0 || task.duration_units == 0 {
            errors.push(format!("{} version, cost, and duration must be positive", task.id));
        }
        let Some(producer_artifact) = artifacts.get(task.produces_artifact_id.as_str()) else {
            errors.push(format!(
                "{} produces unknown artifact {:?}",
                task.id, task.produces_artifact_id
            ));
            continue;
        };
        let required: BTreeSet<&str> = task
            .required_artifact_ids
            .iter()
            .map(String::as_str)
            .collect();
        let expected: BTreeSet<&str> = producer_artifact
            .depends_on
            .iter()
            .map(String::as_str)
            .collect();
        if required.len() != task.required_artifact_ids.len() || required != expected {
            errors.push(format!(
                "{} prerequisites must exactly match {} direct evidence dependencies",
                task.id, task.produces_artifact_id
            ));
        }
        let task_gates: BTreeSet<&str> = task
            .required_gate_ids
            .iter()
            .map(String::as_str)
            .collect();
        if task_gates.len() != task.required_gate_ids.len() {
            errors.push(format!("{} repeats a required gate", task.id));
        }
        for gate in &task.required_gate_ids {
            if !declared_gates.contains(gate.as_str()) {
                errors.push(format!("{} references undeclared gate {:?}", task.id, gate));
            }
        }
    }

    // Kahn-style source removal rejects cycles in the evidence graph.
    let mut remaining: BTreeSet<&str> = artifacts.keys().copied().collect();
    while !remaining.is_empty() {
        let ready: Vec<&str> = remaining
            .iter()
            .copied()
            .filter(|id| {
                artifacts.get(id).is_some_and(|artifact| {
                    artifact
                        .depends_on
                        .iter()
                        .all(|dep| !remaining.contains(dep.as_str()))
                })
            })
            .collect();
        if ready.is_empty() {
            errors.push("artifact dependency graph contains a cycle".to_owned());
            break;
        }
        for id in ready {
            remaining.remove(id);
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_BYTES: &[u8] = include_bytes!(
        "../../../docs/research/fixtures/HABITAT_REVALIDATION_PLAN_V0_1.json"
    );

    fn fixture() -> Fixture {
        serde_json::from_slice(FIXTURE_BYTES).expect("frozen fixture should parse")
    }
    fn report() -> Report {
        verify_fixture_bytes(FIXTURE_BYTES).expect("frozen fixture should verify")
    }

    fn exhaustive_oracle(fixture: &Fixture) -> Vec<Plan> {
        fn permutations(
            remaining: &mut Vec<RepairTask>,
            prefix: &mut Vec<RepairTask>,
            output: &mut Vec<Vec<RepairTask>>,
        ) {
            output.push(prefix.clone());
            for index in 0..remaining.len() {
                let task = remaining.remove(index);
                prefix.push(task.clone());
                permutations(remaining, prefix, output);
                prefix.pop();
                remaining.insert(index, task);
            }
        }
        let mut remaining = fixture.tasks.clone();
        let mut all_orders = Vec::new();
        permutations(&mut remaining, &mut Vec::new(), &mut all_orders);
        let initially_valid: BTreeSet<String> = fixture
            .artifacts
            .iter()
            .filter(|artifact| artifact.status == "valid_for_claim")
            .map(|artifact| artifact.id.clone())
            .collect();
        let mut feasible = Vec::new();
        for order in all_orders {
            let mut valid = initially_valid.clone();
            let mut ids = Vec::new();
            let mut cost = 0u64;
            let mut duration = 0u64;
            for task in order {
                if valid.contains(&fixture.target_artifact_id) {
                    break;
                }
                if valid.contains(&task.produces_artifact_id)
                    || !task
                        .required_artifact_ids
                        .iter()
                        .all(|id| valid.contains(id))
                    || !task
                        .required_gate_ids
                        .iter()
                        .all(|id| fixture.available_gate_ids.contains(id))
                {
                    break;
                }
                valid.insert(task.produces_artifact_id.clone());
                ids.push(task.id.clone());
                cost += task.cost_units;
                duration += task.duration_units;
                if task.produces_artifact_id == fixture.target_artifact_id {
                    feasible.push(make_plan(
                        fixture,
                        &fixture.target_artifact_id,
                        &ids,
                        cost,
                        duration,
                    ));
                    break;
                }
            }
        }
        feasible.sort_by(compare_plans);
        feasible.dedup_by(|left, right| left.task_ids == right.task_ids);
        let mut frontier = Vec::new();
        for candidate in &feasible {
            let dominated = feasible.iter().any(|other| {
                other.cost_units <= candidate.cost_units
                    && other.duration_units <= candidate.duration_units
                    && (other.cost_units < candidate.cost_units
                        || other.duration_units < candidate.duration_units)
            });
            if !dominated {
                frontier.push(candidate.clone());
            }
        }
        frontier.sort_by(compare_plans);
        frontier.dedup_by(|left, right| left.task_ids == right.task_ids);
        frontier
    }

    #[test]
    fn frozen_fixture_produces_two_nondominated_plans() {
        let report = report();
        assert_eq!(report.input_sha256, FIXTURE_SHA256);
        assert_eq!(report.planning_status, "PLAN_AVAILABLE");
        assert_eq!(report.artifact_count, 10);
        assert_eq!(report.task_count, 4);
        assert_eq!(report.enumerated_feasible_plan_count, 2);
        assert_eq!(report.pareto_plan_count, 2);
        assert_eq!(
            report.pareto_plans.iter().map(|p| p.cost_units).collect::<Vec<_>>(),
            vec![12, 15]
        );
        assert_eq!(
            report.pareto_plans.iter().map(|p| p.duration_units).collect::<Vec<_>>(),
            vec![8, 5]
        );
        assert_eq!(
            report.pareto_plans[0].task_ids,
            vec![
                "T-MATERIAL-REVIEW-LOW-COST".to_owned(),
                "T-ASBUILT-REINSPECTION".to_owned(),
                "T-RECOMMISSION".to_owned(),
            ]
        );
        assert_eq!(
            report.pareto_plans[1].task_ids,
            vec![
                "T-MATERIAL-REVIEW-FAST".to_owned(),
                "T-ASBUILT-REINSPECTION".to_owned(),
                "T-RECOMMISSION".to_owned(),
            ]
        );
        assert!(report.pareto_plans.iter().all(|plan| {
            plan.task_steps.len() == 3
                && plan.task_steps.iter().all(|step| step.task_version == 1)
                && plan.execution_status == "PROPOSED_NOT_EXECUTED"
                && plan.target_artifact_id_to_restore == "D-COMMISSIONING-RECEIPT"
        }));
        assert!(report.blocking_reasons.is_empty());
    }

    #[test]
    fn frozen_fixture_matches_independent_exhaustive_sequence_oracle() {
        assert_eq!(report().pareto_plans, exhaustive_oracle(&fixture()));
    }

    #[test]
    fn all_gate_subsets_match_the_exhaustive_oracle() {
        let base = fixture();
        let combinations = 1usize << base.declared_gate_ids.len();
        for mask in 0..combinations {
            let mut candidate = base.clone();
            candidate.available_gate_ids = base
                .declared_gate_ids
                .iter()
                .enumerate()
                .filter(|(index, _)| mask & (1usize << *index) != 0)
                .map(|(_, gate)| gate.clone())
                .collect();
            let digest = sha256_hex(&serde_json::to_vec(&candidate).unwrap());
            let actual = evaluate_fixture(&candidate, digest).unwrap();
            assert_eq!(
                actual.pareto_plans,
                exhaustive_oracle(&candidate),
                "gate subset mask {mask:#06b} differed from exhaustive oracle"
            );
            if !candidate
                .available_gate_ids
                .iter()
                .any(|gate| gate == "PROCUREMENT_REVIEW")
            {
                assert_eq!(actual.planning_status, "BLOCKED_NO_VALID_PLAN");
                assert!(!actual.blocking_reasons.is_empty());
            }
        }
    }

    #[test]
    fn alternative_material_producers_match_exhaustive_oracle() {
        let mut fixture = fixture();
        let base = fixture
            .tasks
            .iter()
            .find(|task| task.id == "T-MATERIAL-REVIEW-LOW-COST")
            .unwrap()
            .clone();

        let mut cheaper_slower = base.clone();
        cheaper_slower.id = "T-MATERIAL-REVIEW-CHEAPER-SLOWER".to_owned();
        cheaper_slower.description =
            "Generated synthetic cheaper but slower review alternative.".to_owned();
        cheaper_slower.cost_units = 4;
        cheaper_slower.duration_units = 7;

        let mut faster = base;
        faster.id = "T-MATERIAL-REVIEW-EXPEDITED".to_owned();
        faster.description = "Generated synthetic expedited review alternative.".to_owned();
        faster.required_gate_ids.push("INDEPENDENT_QA".to_owned());
        faster.cost_units = 9;
        faster.duration_units = 1;

        fixture.tasks.push(cheaper_slower);
        fixture.tasks.push(faster);
        let digest = sha256_hex(&serde_json::to_vec(&fixture).unwrap());
        let actual = evaluate_fixture(&fixture, digest).unwrap();
        assert_eq!(actual.pareto_plans, exhaustive_oracle(&fixture));
        assert_eq!(actual.pareto_plan_count, 4);
        assert_eq!(
            actual.pareto_plans.iter().map(|plan| plan.cost_units).collect::<Vec<_>>(),
            vec![11, 12, 15, 16]
        );
        assert_eq!(
            actual.pareto_plans
                .iter()
                .map(|plan| plan.duration_units)
                .collect::<Vec<_>>(),
            vec![10, 8, 5, 4]
        );
    }

    #[test]
    fn exact_fixture_bytes_are_required() {
        let mut changed = FIXTURE_BYTES.to_vec();
        changed.push(b' ');
        assert!(verify_fixture_bytes(&changed).is_err());
    }

    #[test]
    fn sha256_known_answer() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn missing_mandatory_gate_blocks_all_plans() {
        let mut fixture = fixture();
        fixture.available_gate_ids.retain(|gate| gate != "PROCUREMENT_REVIEW");
        let report = evaluate_fixture(&fixture, FIXTURE_SHA256.to_owned()).unwrap();
        assert_eq!(report.planning_status, "BLOCKED_NO_VALID_PLAN");
        assert!(report.pareto_plans.is_empty());
        assert!(!report.blocking_reasons.is_empty());
    }

    #[test]
    fn missing_fast_path_gate_preserves_low_cost_plan() {
        let mut fixture = fixture();
        fixture.available_gate_ids.retain(|gate| gate != "INDEPENDENT_QA");
        let report = evaluate_fixture(&fixture, FIXTURE_SHA256.to_owned()).unwrap();
        assert_eq!(report.planning_status, "PLAN_AVAILABLE");
        assert_eq!(report.pareto_plan_count, 1);
        assert_eq!(report.pareto_plans[0].cost_units, 12);
        assert_eq!(report.pareto_plans[0].duration_units, 8);
    }

    #[test]
    fn planner_cannot_skip_producer_dependencies() {
        let mut fixture = fixture();
        let task = fixture
            .tasks
            .iter_mut()
            .find(|task| task.id == "T-RECOMMISSION")
            .unwrap();
        task.required_artifact_ids.retain(|id| id != "D-STRUCTURAL-CALC");
        assert!(
            validate_fixture(&fixture)
                .iter()
                .any(|error| error.contains("prerequisites must exactly match"))
        );
    }

    #[test]
    fn valid_target_requires_all_its_dependencies_to_be_valid() {
        let mut fixture = fixture();
        for id in ["D-MATERIAL-ORDER", "D-ASBUILT-RECEIPT", "D-COMMISSIONING-RECEIPT"] {
            fixture
                .artifacts
                .iter_mut()
                .find(|artifact| artifact.id == id)
                .unwrap()
                .status = "valid_for_claim".to_owned();
        }
        let report = evaluate_fixture(&fixture, FIXTURE_SHA256.to_owned()).unwrap();
        assert_eq!(report.planning_status, "TARGET_ALREADY_VALID");
        assert_eq!(report.pareto_plans[0].task_count, 0);
        assert_eq!(report.pareto_plans[0].execution_status, "TARGET_ALREADY_VALID");
    }

    #[test]
    fn task_and_artifact_order_do_not_change_report() {
        let mut fixture = fixture();
        let baseline = evaluate_fixture(&fixture, FIXTURE_SHA256.to_owned()).unwrap();
        fixture.tasks.reverse();
        fixture.artifacts.reverse();
        fixture.available_gate_ids.reverse();
        for artifact in &mut fixture.artifacts {
            artifact.depends_on.reverse();
        }
        for task in &mut fixture.tasks {
            task.required_artifact_ids.reverse();
            task.required_gate_ids.reverse();
        }
        let reordered = evaluate_fixture(&fixture, FIXTURE_SHA256.to_owned()).unwrap();
        assert_eq!(baseline, reordered);
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let mut value: serde_json::Value = serde_json::from_slice(FIXTURE_BYTES).unwrap();
        value["unreviewed_safety_bypass"] = serde_json::Value::Bool(true);
        assert!(serde_json::from_value::<Fixture>(value).is_err());
    }
}
