// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const REPORT_SCHEMA: &str = "luminous.habitat_revalidation_portfolio_report.v0.1";
pub const CLAIM_CEILING: &str = "synthetic_multi_target_revalidation_not_real_world_authority";
const FIXTURE_SHA256: &str = "d6c40f80ba89187d180a13b09628455dcf12d597797f9bc5d9fa9cc59a9ed714";
const ALLOWED_STATUSES: [&str; 7] = [
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
    pub target_artifact_ids: Vec<String>,
    pub max_expanded_states: usize,
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
    pub target_artifact_ids_to_restore: Vec<String>,
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
    pub target_artifact_ids: Vec<String>,
    pub planning_status: String,
    pub frontier_complete: bool,
    pub max_expanded_states: usize,
    pub expanded_states: usize,
    pub artifact_count: usize,
    pub task_count: usize,
    pub known_feasible_plan_count: usize,
    pub pareto_plan_count: usize,
    pub pareto_plans: Vec<Plan>,
    pub blocking_reasons: Vec<String>,
}

struct SearchState {
    expanded_states: usize,
    max_expanded_states: usize,
    incomplete: bool,
    incomplete_reason: Option<String>,
    feasible_plans: Vec<Plan>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn verify_fixture_bytes(bytes: &[u8]) -> Result<Report, Vec<String>> {
    let digest = sha256_hex(bytes);
    if digest != FIXTURE_SHA256 {
        return Err(vec![format!(
            "frozen portfolio fixture digest mismatch: expected {FIXTURE_SHA256}, got {digest}"
        )]);
    }
    let fixture: Fixture = serde_json::from_slice(bytes)
        .map_err(|error| vec![format!("portfolio fixture rejected: {error}")])?;
    evaluate_fixture(&fixture, digest)
}

fn task_map(fixture: &Fixture) -> BTreeMap<&str, &RepairTask> {
    fixture.tasks.iter().map(|task| (task.id.as_str(), task)).collect()
}

fn make_plan(fixture: &Fixture, task_ids: &[String], cost: u64, duration: u64) -> Plan {
    let by_id = task_map(fixture);
    let task_steps = task_ids.iter()
        .filter_map(|id| by_id.get(id.as_str()))
        .map(|task| {
            let mut required_artifact_ids = task.required_artifact_ids.clone();
            let mut required_gate_ids = task.required_gate_ids.clone();
            required_artifact_ids.sort();
            required_gate_ids.sort();
            PlanStep {
                task_id: task.id.clone(),
                task_version: task.version,
                produces_artifact_id: task.produces_artifact_id.clone(),
                required_artifact_ids,
                required_gate_ids,
                cost_units: task.cost_units,
                duration_units: task.duration_units,
            }
        })
        .collect();
    let mut targets = fixture.target_artifact_ids.clone();
    targets.sort();
    Plan {
        task_ids: task_ids.to_vec(),
        task_steps,
        target_artifact_ids_to_restore: targets,
        execution_status: "PROPOSED_NOT_EXECUTED".to_owned(),
        cost_units: cost,
        duration_units: duration,
        task_count: task_ids.len(),
    }
}

fn evaluate_fixture(fixture: &Fixture, input_sha256: String) -> Result<Report, Vec<String>> {
    let errors = validate_fixture(fixture);
    if !errors.is_empty() {
        return Err(errors);
    }
    let initially_valid: BTreeSet<String> = fixture.artifacts.iter()
        .filter(|artifact| artifact.status == "valid_for_claim")
        .map(|artifact| artifact.id.clone())
        .collect();
    if fixture.target_artifact_ids.iter().all(|id| initially_valid.contains(id)) {
        let mut plan = make_plan(fixture, &[], 0, 0);
        plan.execution_status = "TARGETS_ALREADY_VALID".to_owned();
        return Ok(Report {
            schema_version: REPORT_SCHEMA.to_owned(),
            fixture_id: fixture.fixture_id.clone(),
            input_sha256,
            verification_status: "SYNTHETIC_PORTFOLIO_PLAN_ONLY".to_owned(),
            claim_ceiling: CLAIM_CEILING.to_owned(),
            target_artifact_ids: sorted_targets(fixture),
            planning_status: "TARGET_ALREADY_VALID".to_owned(),
            frontier_complete: true,
            max_expanded_states: fixture.max_expanded_states,
            expanded_states: 0,
            artifact_count: fixture.artifacts.len(),
            task_count: fixture.tasks.len(),
            known_feasible_plan_count: 1,
            pareto_plan_count: 1,
            pareto_plans: vec![plan],
            blocking_reasons: Vec::new(),
        });
    }

    let mut state = SearchState {
        expanded_states: 0,
        max_expanded_states: fixture.max_expanded_states,
        incomplete: false,
        incomplete_reason: None,
        feasible_plans: Vec::new(),
    };
    enumerate_portfolios(
        fixture,
        &initially_valid,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        0,
        0,
        &mut state,
    );
    state.feasible_plans = deduplicate_task_sets(state.feasible_plans);
    state.feasible_plans.sort_by(compare_plans);
    let frontier = pareto_frontier(&state.feasible_plans);
    let mut blockers = Vec::new();

    let planning_status = if state.incomplete {
        "SEARCH_INCOMPLETE"
    } else if frontier.is_empty() {
        "NO_VALID_PLAN_PROVEN"
    } else {
        "OPTIMAL_EXACT"
    };
    if planning_status == "NO_VALID_PLAN_PROVEN" {
        for task in &fixture.tasks {
            let missing: Vec<&str> = task.required_gate_ids.iter()
                .filter(|gate| !fixture.available_gate_ids.contains(gate))
                .map(String::as_str)
                .collect();
            if !missing.is_empty() {
                blockers.push(format!(
                    "{} blocked by unavailable required gates: {}",
                    task.id,
                    missing.join(", ")
                ));
            }
        }
        if blockers.is_empty() {
            blockers.push("exhaustive search proved no valid portfolio".to_owned());
        }
        blockers.sort();
        blockers.dedup();
    } else if planning_status == "SEARCH_INCOMPLETE" {
        blockers.push(state.incomplete_reason.clone().unwrap_or_else(|| {
            "search incomplete; found plans are provisional".to_owned()
        }));
    }

    Ok(Report {
        schema_version: REPORT_SCHEMA.to_owned(),
        fixture_id: fixture.fixture_id.clone(),
        input_sha256,
        verification_status: "SYNTHETIC_PORTFOLIO_PLAN_ONLY".to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        target_artifact_ids: sorted_targets(fixture),
        planning_status: planning_status.to_owned(),
        frontier_complete: !state.incomplete,
        max_expanded_states: fixture.max_expanded_states,
        expanded_states: state.expanded_states,
        artifact_count: fixture.artifacts.len(),
        task_count: fixture.tasks.len(),
        known_feasible_plan_count: state.feasible_plans.len(),
        pareto_plan_count: frontier.len(),
        pareto_plans: frontier,
        blocking_reasons: blockers,
    })
}

fn deduplicate_task_sets(plans: Vec<Plan>) -> Vec<Plan> {
    let mut unique: BTreeMap<Vec<String>, Plan> = BTreeMap::new();
    for plan in plans {
        let mut key = plan.task_ids.clone();
        key.sort();
        match unique.get_mut(&key) {
            Some(existing) if plan.task_ids < existing.task_ids => *existing = plan,
            Some(_) => {}
            None => {
                unique.insert(key, plan);
            }
        }
    }
    unique.into_values().collect()
}

fn sorted_targets(fixture: &Fixture) -> Vec<String> {
    let mut targets = fixture.target_artifact_ids.clone();
    targets.sort();
    targets
}

fn enumerate_portfolios(
    fixture: &Fixture,
    valid_artifacts: &BTreeSet<String>,
    current_task_ids: &mut Vec<String>,
    used_tasks: &mut BTreeSet<String>,
    cost: u64,
    duration: u64,
    state: &mut SearchState,
) {
    if state.expanded_states >= state.max_expanded_states {
        state.incomplete = true;
        state.incomplete_reason = Some(format!(
            "state expansion budget exhausted at {} states",
            state.max_expanded_states
        ));
        return;
    }
    state.expanded_states += 1;
    if fixture.target_artifact_ids.iter().all(|id| valid_artifacts.contains(id)) {
        state.feasible_plans.push(make_plan(fixture, current_task_ids, cost, duration));
        return;
    }
    let mut candidates: Vec<&RepairTask> = fixture.tasks.iter().collect();
    candidates.sort_by(|left, right| left.id.cmp(&right.id));
    for task in candidates {
        if used_tasks.contains(&task.id)
            || valid_artifacts.contains(&task.produces_artifact_id)
            || !task.required_artifact_ids.iter().all(|id| valid_artifacts.contains(id))
            || !task
                .required_gate_ids
                .iter()
                .all(|id| fixture.available_gate_ids.contains(id))
        {
            continue;
        }
        let Some(next_cost) = cost.checked_add(task.cost_units) else {
            state.incomplete = true;
            state.incomplete_reason = Some(format!(
                "cost accumulation overflow at task {}",
                task.id
            ));
            return;
        };
        let Some(next_duration) = duration.checked_add(task.duration_units) else {
            state.incomplete = true;
            state.incomplete_reason = Some(format!(
                "duration accumulation overflow at task {}",
                task.id
            ));
            return;
        };
        used_tasks.insert(task.id.clone());
        current_task_ids.push(task.id.clone());
        let mut next_valid = valid_artifacts.clone();
        next_valid.insert(task.produces_artifact_id.clone());
        enumerate_portfolios(
            fixture,
            &next_valid,
            current_task_ids,
            used_tasks,
            next_cost,
            next_duration,
            state,
        );
        current_task_ids.pop();
        used_tasks.remove(&task.id);
        if state.incomplete {
            break;
        }
    }
}

fn compare_plans(left: &Plan, right: &Plan) -> std::cmp::Ordering {
    left.cost_units.cmp(&right.cost_units)
        .then_with(|| left.duration_units.cmp(&right.duration_units))
        .then_with(|| left.task_ids.cmp(&right.task_ids))
}

fn pareto_frontier(plans: &[Plan]) -> Vec<Plan> {
    let mut frontier: Vec<Plan> = plans.iter().filter(|candidate| {
        !plans.iter().any(|other| {
            other.cost_units <= candidate.cost_units
                && other.duration_units <= candidate.duration_units
                && (other.cost_units < candidate.cost_units
                    || other.duration_units < candidate.duration_units)
        })
    }).cloned().collect();
    frontier.sort_by(compare_plans);
    frontier.dedup_by(|left, right| left.task_ids == right.task_ids);
    frontier
}

fn validate_fixture(fixture: &Fixture) -> Vec<String> {
    let mut errors = Vec::new();
    if fixture.schema_version != "luminous.habitat_revalidation_portfolio_fixture.v0.1" {
        errors.push(format!("unsupported schema_version {:?}", fixture.schema_version));
    }
    if fixture.fixture_id.trim().is_empty() || fixture.intended_use.trim().is_empty() {
        errors.push("fixture ID and intended use must not be blank".to_owned());
    }
    if fixture.status != "proposed_synthetic_revalidation_portfolio_benchmark" {
        errors.push(format!("unsupported status {:?}", fixture.status));
    }
    if !fixture.synthetic_only {
        errors.push("synthetic_only must remain true".to_owned());
    }
    if fixture.claim_ceiling != CLAIM_CEILING {
        errors.push(format!("claim ceiling must be {CLAIM_CEILING:?}"));
    }
    if fixture.max_expanded_states == 0 {
        errors.push("max_expanded_states must be positive".to_owned());
    }
    if fixture.target_artifact_ids.is_empty() {
        errors.push("at least one target artifact is required".to_owned());
    }
    let target_set: BTreeSet<&str> =
        fixture.target_artifact_ids.iter().map(String::as_str).collect();
    if target_set.len() != fixture.target_artifact_ids.len() {
        errors.push("target artifact IDs must be unique".to_owned());
    }
    let mut artifacts = BTreeMap::new();
    for artifact in &fixture.artifacts {
        if artifact.id.trim().is_empty() {
            errors.push("artifact ID must not be blank".to_owned());
        }
        if artifacts.insert(artifact.id.as_str(), artifact).is_some() {
            errors.push(format!("duplicate artifact ID {:?}", artifact.id));
        }
        if !ALLOWED_STATUSES.contains(&artifact.status.as_str()) {
            errors.push(format!("{} has unsupported status {:?}", artifact.id, artifact.status));
        }
        let mut dependencies = BTreeSet::new();
        for dependency in &artifact.depends_on {
            if dependency == &artifact.id {
                errors.push(format!("{} cannot depend on itself", artifact.id));
            }
            if !dependencies.insert(dependency.as_str()) {
                errors.push(format!("{} repeats dependency {:?}", artifact.id, dependency));
            }
        }
    }
    for target in &fixture.target_artifact_ids {
        if !artifacts.contains_key(target.as_str()) {
            errors.push(format!("unknown target artifact {:?}", target));
        }
    }
    for artifact in &fixture.artifacts {
        for dependency in &artifact.depends_on {
            match artifacts.get(dependency.as_str()) {
                None => errors.push(format!(
                    "{} references unknown dependency {:?}", artifact.id, dependency
                )),
                Some(parent) if artifact.status == "valid_for_claim"
                    && parent.status != "valid_for_claim" =>
                {
                    errors.push(format!(
                        "{} cannot remain valid_for_claim while dependency {} is invalid",
                        artifact.id, dependency
                    ));
                }
                _ => {}
            }
        }
    }
    let declared_gates: BTreeSet<&str> =
        fixture.declared_gate_ids.iter().map(String::as_str).collect();
    let available_gates: BTreeSet<&str> =
        fixture.available_gate_ids.iter().map(String::as_str).collect();
    if declared_gates.len() != fixture.declared_gate_ids.len() {
        errors.push("declared gate IDs must be unique".to_owned());
    }
    if available_gates.len() != fixture.available_gate_ids.len() {
        errors.push("available gate IDs must be unique".to_owned());
    }
    if available_gates.iter().any(|gate| !declared_gates.contains(gate)) {
        errors.push("available gate IDs must be declared".to_owned());
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
            errors.push(format!("{} version, cost and duration must be positive", task.id));
        }
        let Some(producer) = artifacts.get(task.produces_artifact_id.as_str()) else {
            errors.push(format!(
                "{} produces unknown artifact {:?}", task.id, task.produces_artifact_id
            ));
            continue;
        };
        let required: BTreeSet<&str> = task
            .required_artifact_ids
            .iter()
            .map(String::as_str)
            .collect();
        let expected: BTreeSet<&str> =
            producer.depends_on.iter().map(String::as_str).collect();
        if required.len() != task.required_artifact_ids.len() || required != expected {
            errors.push(format!(
                "{} prerequisites must exactly match {} direct dependencies",
                task.id, task.produces_artifact_id
            ));
        }
        let gates: BTreeSet<&str> = task
            .required_gate_ids
            .iter()
            .map(String::as_str)
            .collect();
        if gates.len() != task.required_gate_ids.len() {
            errors.push(format!("{} repeats a required gate", task.id));
        }
        if gates.iter().any(|gate| !declared_gates.contains(gate)) {
            errors.push(format!("{} has an undeclared required gate", task.id));
        }
    }
    let mut remaining: BTreeSet<&str> = artifacts.keys().copied().collect();
    while !remaining.is_empty() {
        let ready: Vec<&str> = remaining.iter().copied().filter(|id| {
            artifacts.get(id).is_some_and(|artifact| {
                artifact.depends_on.iter().all(|dep| !remaining.contains(dep.as_str()))
            })
        }).collect();
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
        "../../../docs/research/fixtures/HABITAT_REVALIDATION_PORTFOLIO_V0_1.json"
    );

    fn fixture() -> Fixture {
        serde_json::from_slice(FIXTURE_BYTES).expect("frozen portfolio fixture should parse")
    }
    fn report() -> Report {
        verify_fixture_bytes(FIXTURE_BYTES).expect("frozen portfolio fixture should verify")
    }
    fn oracle(fixture: &Fixture) -> Vec<Plan> {
        let mut output = Vec::new();
        let count = 1usize << fixture.tasks.len();
        let initial: BTreeSet<String> = fixture
            .artifacts
            .iter()
            .filter(|artifact| artifact.status == "valid_for_claim")
            .map(|artifact| artifact.id.clone())
            .collect();
        for mask in 0..count {
            let mut selected: Vec<RepairTask> = fixture
                .tasks
                .iter()
                .enumerate()
                .filter(|(index, _)| mask & (1usize << *index) != 0)
                .map(|(_, task)| task.clone())
                .collect();
            selected.sort_by(|a, b| a.id.cmp(&b.id));
            let mut pending = selected;
            let mut valid = initial.clone();
            let mut sequence = Vec::new();
            let mut cost = 0u64;
            let mut duration = 0u64;
            let mut failed = false;
            loop {
                if fixture.target_artifact_ids.iter().all(|id| valid.contains(id)) {
                    break;
                }
                if pending.is_empty() {
                    break;
                }
                let Some(index) = pending.iter().position(|task| {
                    !valid.contains(&task.produces_artifact_id)
                        && task.required_artifact_ids.iter().all(|id| valid.contains(id))
                        && task
                            .required_gate_ids
                            .iter()
                            .all(|id| fixture.available_gate_ids.contains(id))
                }) else {
                    failed = true;
                    break;
                };
                let task = pending.remove(index);
                valid.insert(task.produces_artifact_id.clone());
                sequence.push(task.clone());
                cost += task.cost_units;
                duration += task.duration_units;
            }
            if !failed && pending.is_empty()
                && fixture.target_artifact_ids.iter().all(|id| valid.contains(id))
            {
                let ids: Vec<String> = sequence
                    .iter()
                    .map(|task| task.id.clone())
                    .collect();
                let p = make_plan(fixture, &ids, cost, duration);
                if !output.iter().any(|prior: &Plan| prior.task_ids == p.task_ids) {
                    output.push(p);
                }
            }
        }
        let mut nondominated: Vec<Plan> = output
            .iter()
            .filter(|candidate| {
                !output.iter().any(|other| {
                    other.cost_units <= candidate.cost_units
                        && other.duration_units <= candidate.duration_units
                        && (other.cost_units < candidate.cost_units
                            || other.duration_units < candidate.duration_units)
                })
            })
            .cloned()
            .collect();
        nondominated.sort_by(|left, right| {
            left.cost_units
                .cmp(&right.cost_units)
                .then_with(|| left.duration_units.cmp(&right.duration_units))
                .then_with(|| left.task_ids.cmp(&right.task_ids))
        });
        nondominated
    }

    #[test]
    fn frozen_joint_target_fixture_produces_two_pareto_plans() {
        let report = report();
        assert_eq!(report.input_sha256, FIXTURE_SHA256);
        assert_eq!(report.planning_status, "OPTIMAL_EXACT");
        assert!(report.frontier_complete);
        assert_eq!(report.artifact_count, 7);
        assert_eq!(report.task_count, 5);
        assert_eq!(report.pareto_plan_count, 2);
        assert_eq!(
            report.pareto_plans.iter().map(|plan| plan.cost_units).collect::<Vec<_>>(),
            vec![14, 17]
        );
        assert_eq!(
            report.pareto_plans.iter().map(|plan| plan.duration_units).collect::<Vec<_>>(),
            vec![9, 6]
        );
        for plan in &report.pareto_plans {
            assert_eq!(plan.task_count, 4);
            assert_eq!(
                plan.task_ids
                    .iter()
                    .filter(|id| id.starts_with("T-MATERIAL-REVIEW"))
                    .count(),
                1
            );
            assert_eq!(
                plan.task_ids
                    .iter()
                    .filter(|id| id.as_str() == "T-ASBUILT-REINSPECTION")
                    .count(),
                1
            );
            assert_eq!(plan.execution_status, "PROPOSED_NOT_EXECUTED");
            assert_eq!(plan.target_artifact_ids_to_restore.len(), 2);
        }
    }

    #[test]
    fn frozen_joint_target_frontier_matches_independent_subset_oracle() {
        assert_eq!(report().pareto_plans, oracle(&fixture()));
    }

    #[test]
    fn all_32_gate_subsets_match_the_independent_oracle() {
        let base = fixture();
        for mask in 0..(1usize << base.declared_gate_ids.len()) {
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
            let expected = oracle(&candidate);
            assert_eq!(actual.pareto_plans, expected, "gate subset {mask:#07b}");
            assert!(actual.frontier_complete);
            assert_eq!(
                actual.planning_status,
                if expected.is_empty() {
                    "NO_VALID_PLAN_PROVEN"
                } else {
                    "OPTIMAL_EXACT"
                }
            );
        }
    }

    #[test]
    fn third_shared_target_matches_oracle_across_all_64_gate_subsets() {
        let mut base = fixture();
        base.target_artifact_ids
            .push("D-SERVICE-READINESS".to_owned());
        base.artifacts.push(ArtifactState {
            id: "D-SERVICE-READINESS".to_owned(),
            status: "invalidated".to_owned(),
            depends_on: vec!["D-ASBUILT-RECEIPT".to_owned()],
        });
        base.declared_gate_ids
            .push("SERVICE_READINESS_REVIEW".to_owned());
        base.available_gate_ids
            .push("SERVICE_READINESS_REVIEW".to_owned());
        base.tasks.push(RepairTask {
            id: "T-SERVICE-READINESS".to_owned(),
            version: 1,
            produces_artifact_id: "D-SERVICE-READINESS".to_owned(),
            required_artifact_ids: vec!["D-ASBUILT-RECEIPT".to_owned()],
            required_gate_ids: vec!["SERVICE_READINESS_REVIEW".to_owned()],
            cost_units: 2,
            duration_units: 3,
            description: "Restore a third synthetic target that shares as-built evidence."
                .to_owned(),
        });

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
            let expected = oracle(&candidate);
            assert_eq!(
                actual.pareto_plans,
                expected,
                "third-target gate subset {mask:#08b}"
            );
            assert!(actual.frontier_complete);
            assert_eq!(
                actual.planning_status,
                if expected.is_empty() {
                    "NO_VALID_PLAN_PROVEN"
                } else {
                    "OPTIMAL_EXACT"
                }
            );
            if mask == combinations - 1 {
                assert_eq!(
                    actual.pareto_plans.iter().map(|plan| plan.cost_units).collect::<Vec<_>>(),
                    vec![16, 19]
                );
                assert_eq!(
                    actual.pareto_plans
                        .iter()
                        .map(|plan| plan.duration_units)
                        .collect::<Vec<_>>(),
                    vec![12, 9]
                );
            }
        }
    }

    #[test]
    fn work_budget_exhaustion_is_never_reported_as_optimal_or_no_plan() {
        let mut fixture = fixture();
        fixture.max_expanded_states = 1;
        let digest = sha256_hex(&serde_json::to_vec(&fixture).unwrap());
        let report = evaluate_fixture(&fixture, digest).unwrap();
        assert_eq!(report.planning_status, "SEARCH_INCOMPLETE");
        assert!(!report.frontier_complete);
        assert!(report.expanded_states <= 1);
        assert!(!report.blocking_reasons.is_empty());
    }

    #[test]
    fn budget_expiry_after_one_plan_is_still_incomplete() {
        let mut fixture = fixture();
        fixture.max_expanded_states = 5;
        let digest = sha256_hex(&serde_json::to_vec(&fixture).unwrap());
        let result = evaluate_fixture(&fixture, digest).unwrap();
        assert_eq!(result.planning_status, "SEARCH_INCOMPLETE");
        assert!(!result.frontier_complete);
        assert_eq!(result.known_feasible_plan_count, 1);
        assert_eq!(result.pareto_plan_count, 1);
        assert!(result.blocking_reasons[0].contains("budget exhausted"));
    }

    #[test]
    fn identical_shared_prerequisites_are_counted_only_once() {
        let report = report();
        for plan in &report.pareto_plans {
            assert_eq!(plan.task_ids.len(), plan.task_ids.iter().collect::<BTreeSet<_>>().len());
            assert_eq!(
                plan.cost_units,
                plan.task_steps.iter().map(|step| step.cost_units).sum::<u64>()
            );
            assert_eq!(
                plan.duration_units,
                plan.task_steps.iter().map(|step| step.duration_units).sum::<u64>()
            );
            assert_eq!(
                plan.task_steps
                    .iter()
                    .filter(|step| step.task_id == "T-ASBUILT-REINSPECTION")
                    .count(),
                1
            );
            assert!(plan.task_steps.iter().any(|step| step.task_id == "T-ASBUILT-REINSPECTION"));
        }
    }

    #[test]
    fn unavailable_mandatory_gate_proves_no_valid_plan_when_search_completes() {
        let mut fixture = fixture();
        fixture.available_gate_ids.retain(|gate| gate != "COMMISSIONING_AUTHORITY");
        let digest = sha256_hex(&serde_json::to_vec(&fixture).unwrap());
        let actual = evaluate_fixture(&fixture, digest).unwrap();
        assert_eq!(actual.planning_status, "NO_VALID_PLAN_PROVEN");
        assert!(actual.frontier_complete);
        assert!(actual.pareto_plans.is_empty());
    }

    #[test]
    fn task_prerequisite_bypass_is_rejected() {
        let mut fixture = fixture();
        let task = fixture.tasks.iter_mut()
            .find(|task| task.id == "T-COMMISSIONING").unwrap();
        task.required_artifact_ids.retain(|id| id != "D-STRUCTURAL-CALC");
        assert!(validate_fixture(&fixture)
            .iter().any(|error| error.contains("prerequisites must exactly match")));
    }

    #[test]
    fn input_order_changes_do_not_change_plans_or_search_status() {
        let mut fixture = fixture();
        let base = evaluate_fixture(&fixture, "base-digest".to_owned()).unwrap();
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
        let reordered = evaluate_fixture(&fixture, "different-digest".to_owned()).unwrap();
        assert_eq!(base.pareto_plans, reordered.pareto_plans);
        assert_eq!(base.planning_status, reordered.planning_status);
        assert_eq!(base.expanded_states, reordered.expanded_states);
    }

    #[test]
    fn invalid_dependencies_cannot_be_marked_valid_and_unknown_fields_fail_closed() {
        let mut fixture = fixture();
        let target = fixture.artifacts.iter_mut()
            .find(|artifact| artifact.id == "D-COMMISSIONING-RECEIPT").unwrap();
        target.status = "valid_for_claim".to_owned();
        assert!(validate_fixture(&fixture)
            .iter().any(|error| error.contains("cannot remain valid_for_claim")));
        let mut value: serde_json::Value = serde_json::from_slice(FIXTURE_BYTES).unwrap();
        value["approval_override"] = serde_json::Value::Bool(true);
        assert!(serde_json::from_value::<Fixture>(value).is_err());
    }

    #[test]
    fn exact_fixture_bytes_and_sha256_known_answer_are_required() {
        let mut changed = FIXTURE_BYTES.to_vec();
        changed.push(b' ');
        assert!(verify_fixture_bytes(&changed).is_err());
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
