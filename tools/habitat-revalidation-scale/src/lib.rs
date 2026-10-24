// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const GENERATOR_VERSION: &str = "0.1";
pub const MATRIX_SCHEMA: &str = "luminous.habitat_revalidation_scale_benchmark.v0.1";
pub const WORKLOAD_SCHEMA: &str = "luminous.habitat_revalidation_scale_instance.v0.1";
pub const RECEIPT_SCHEMA: &str = "luminous.habitat_revalidation_scale_receipt.v0.1";
pub const CLAIM_CEILING: &str =
    "deterministic_synthetic_scale_input_not_measured_performance_or_real_world_authority";
const MAX_ARTIFACTS: usize = 100_000;
const MAX_TASKS: usize = 10_000;
const MAX_GATES: usize = 10_000;
const MAX_TARGETS: usize = 256;
const MAX_ALTERNATIVES: usize = 1_024;
const MAX_DEPENDENCY_EDGES: u128 = 1_000_000;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Matrix {
    schema_version: String,
    suite_id: String,
    status: String,
    synthetic_only: bool,
    claim_ceiling: String,
    generator_contract: Value,
    measurement_protocol: Value,
    correctness_gates: Value,
    provisional_performance_targets: Value,
    profiles: Vec<ProfileSpec>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileSpec {
    profile_id: String,
    seed: u64,
    artifact_count: usize,
    task_count: usize,
    reachable_task_count: usize,
    target_count: usize,
    gate_count: usize,
    max_dependency_depth: usize,
    unrelated_artifact_fraction_ppm: u32,
    shared_task_fraction_ppm: u32,
    tradeoff_alternatives: usize,
    max_expanded_states: usize,
    independent_exhaustive_oracle_required: bool,
    required_expected_class: String,
    purpose: String,
    #[serde(default)]
    budget_cases: Option<Vec<BudgetCase>>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BudgetCase {
    case_id: String,
    #[serde(default)]
    max_expanded_states: Option<usize>,
    #[serde(default)]
    stop_after_first_candidate: Option<bool>,
    expected_status: String,
    expected_frontier_complete: bool,
    expected_known_plan_count_min: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub id: String,
    pub status: String,
    pub role: String,
    pub depends_on: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RepairTask {
    pub id: String,
    pub produces_artifact_id: String,
    pub required_artifact_ids: Vec<String>,
    pub required_gate_ids: Vec<String>,
    pub cost_units: u64,
    pub duration_units: u64,
    pub target_reachable: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ObjectiveAlternative {
    pub id: String,
    pub cost_units: u64,
    pub duration_units: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BudgetScenario {
    pub case_id: String,
    pub max_expanded_states: Option<usize>,
    pub stop_after_first_candidate: Option<bool>,
    pub expected_status: String,
    pub expected_frontier_complete: bool,
    pub expected_known_plan_count_min: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Workload {
    pub schema_version: String,
    pub workload_id: String,
    pub suite_id: String,
    pub profile_id: String,
    pub seed: u64,
    pub synthetic_only: bool,
    pub claim_ceiling: String,
    pub artifact_count: usize,
    pub task_count: usize,
    pub reachable_task_count: usize,
    pub target_artifact_ids: Vec<String>,
    pub declared_gate_ids: Vec<String>,
    pub available_gate_ids: Vec<String>,
    pub max_expanded_states: usize,
    pub artifacts: Vec<Artifact>,
    pub tasks: Vec<RepairTask>,
    pub objective_alternatives: Vec<ObjectiveAlternative>,
    pub expected_pareto_candidate_ids: Vec<String>,
    pub budget_scenarios: Vec<BudgetScenario>,
    pub independent_exhaustive_oracle_required: bool,
    pub required_expected_class: String,
    pub profile_purpose: String,
    pub generator_version: String,
    pub generator_parameters: GeneratorParameters,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GeneratorParameters {
    pub unrelated_artifact_fraction_ppm: u32,
    pub shared_task_fraction_ppm: u32,
    pub max_dependency_depth: usize,
    pub tradeoff_alternatives: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub schema_version: String,
    pub generator_version: String,
    pub suite_id: String,
    pub profile_id: String,
    pub seed: u64,
    pub generator_source_commit: String,
    pub profile_matrix_sha256: String,
    pub exact_input_sha256: String,
    pub exact_input_bytes: usize,
    pub artifact_count: usize,
    pub task_count: usize,
    pub reachable_task_count: usize,
    pub target_count: usize,
    pub claim_ceiling: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedWorkload {
    pub workload_bytes: Vec<u8>,
    pub receipt: Receipt,
}

fn hash_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut result = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut result, "{byte:02x}");
    }
    result
}

fn mix64(mut value: u64) -> u64 {
    // SplitMix64's fixed-width mixing function: no platform RNG or map-order dependence.
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn artifact_for_task(index: usize) -> String {
    format!("D-ARTIFACT-{index:06}")
}

fn task_id(index: usize) -> String {
    format!("T-REPAIR-{index:06}")
}

fn support_id(index: usize) -> String {
    format!("E-SUPPORT-{index:06}")
}

fn unrelated_id(index: usize) -> String {
    format!("E-UNRELATED-{index:06}")
}

fn gate_id(index: usize) -> String {
    format!("G-{index:04}")
}

fn parse_matrix(bytes: &[u8]) -> Result<Matrix, String> {
    let matrix: Matrix = serde_json::from_slice(bytes)
        .map_err(|error| format!("scale profile matrix rejected: {error}"))?;
    if matrix.schema_version != MATRIX_SCHEMA {
        return Err(format!(
            "unsupported matrix schema: {}",
            matrix.schema_version
        ));
    }
    if matrix.suite_id.trim().is_empty() || matrix.status.trim().is_empty() {
        return Err("matrix suite_id and status must be non-empty".to_owned());
    }
    if !matrix.synthetic_only || matrix.claim_ceiling.trim().is_empty() {
        return Err("matrix must explicitly constrain claims to synthetic use".to_owned());
    }
    if matrix.generator_contract.get("version").and_then(Value::as_str)
        != Some(GENERATOR_VERSION)
    {
        return Err("generator version does not match the matrix contract".to_owned());
    }
    if matrix.measurement_protocol.is_null()
        || matrix.correctness_gates.is_null()
        || matrix.provisional_performance_targets.is_null()
    {
        return Err("matrix is missing a required contract section".to_owned());
    }
    let mut ids = BTreeSet::new();
    for profile in &matrix.profiles {
        if profile.profile_id.trim().is_empty() || !ids.insert(profile.profile_id.clone()) {
            return Err("profile ids must be non-empty and unique".to_owned());
        }
        validate_profile(profile)?;
    }
    if matrix.profiles.is_empty() {
        return Err("matrix must contain at least one profile".to_owned());
    }
    Ok(matrix)
}

fn validate_profile(profile: &ProfileSpec) -> Result<(), String> {
    let prefix = format!("profile {}: ", profile.profile_id);
    if profile.artifact_count == 0
        || profile.task_count == 0
        || profile.reachable_task_count == 0
        || profile.target_count == 0
        || profile.gate_count == 0
        || profile.max_dependency_depth < 2
        || profile.max_expanded_states == 0
        || profile.tradeoff_alternatives == 0
    {
        return Err(format!(
            "{prefix}counts and the search budget must be positive; depth must be at least two"
        ));
    }
    if profile.artifact_count > MAX_ARTIFACTS
        || profile.task_count > MAX_TASKS
        || profile.gate_count > MAX_GATES
        || profile.target_count > MAX_TARGETS
        || profile.tradeoff_alternatives > MAX_ALTERNATIVES
        || profile.max_dependency_depth > MAX_ARTIFACTS
    {
        return Err(format!("{prefix}requested workload exceeds generator safety limits"));
    }
    if profile.task_count > profile.artifact_count
        || profile.reachable_task_count > profile.task_count
        || profile.target_count > profile.reachable_task_count
    {
        return Err(format!("{prefix}task, artifact, and target bounds are inconsistent"));
    }
    if profile.unrelated_artifact_fraction_ppm > 1_000_000
        || profile.shared_task_fraction_ppm > 1_000_000
    {
        return Err(format!("{prefix}fraction must be between zero and one million ppm"));
    }
    let unrelated_count = ((profile.artifact_count as u128
        * profile.unrelated_artifact_fraction_ppm as u128)
        / 1_000_000u128) as usize;
    let detached_tasks = profile.task_count - profile.reachable_task_count;
    let relevant_count = profile.artifact_count - unrelated_count;
    if relevant_count < profile.reachable_task_count {
        return Err(format!(
            "{prefix}unrelated fraction leaves too few artifacts for the target-reachable graph"
        ));
    }
    if unrelated_count < detached_tasks {
        return Err(format!(
            "{prefix}unrelated fraction cannot accommodate detached task outputs"
        ));
    }
    if let Some(cases) = &profile.budget_cases {
        let mut seen = BTreeSet::new();
        for case in cases {
            if case.case_id.trim().is_empty() || !seen.insert(case.case_id.clone()) {
                return Err(format!("{prefix}budget case ids must be non-empty and unique"));
            }
            if case.expected_status != "SEARCH_INCOMPLETE"
                || case.expected_frontier_complete
            {
                return Err(format!(
                    "{prefix}budget exhaustion cases must require truthful incomplete status"
                ));
            }
            if case.max_expanded_states == Some(0) {
                return Err(format!("{prefix}budget case cap must be positive"));
            }
        }
    }
    Ok(())
}

fn chunk_tails(prefix: &str, count: usize, max_depth: usize) -> Vec<String> {
    if count == 0 {
        return Vec::new();
    }
    let mut tails = Vec::new();
    let mut start = 0;
    while start < count {
        let end = (start + max_depth).min(count);
        tails.push(format!("{prefix}{:06}", end - 1));
        start = end;
    }
    tails
}

fn objective_alternatives(count: usize) -> Vec<ObjectiveAlternative> {
    (0..count)
        .map(|index| {
            if index % 3 == 0 {
                ObjectiveAlternative {
                    id: format!("PARETO-{index:04}"),
                    cost_units: (index + 1) as u64,
                    duration_units: (count - index) as u64,
                }
            } else {
                // A deliberately dominated candidate for the independent frontier probe.
                let value = (count + index + 1) as u64;
                ObjectiveAlternative {
                    id: format!("PARETO-{index:04}"),
                    cost_units: value,
                    duration_units: value,
                }
            }
        })
        .collect()
}

/// O(n^2) pairwise implementation retained as a transparent reference oracle.
pub fn independent_pareto_ids(candidates: &[ObjectiveAlternative]) -> Vec<String> {
    let mut frontier: Vec<&ObjectiveAlternative> = candidates
        .iter()
        .filter(|candidate| {
            !candidates.iter().any(|other| {
                other.id != candidate.id
                    && other.cost_units <= candidate.cost_units
                    && other.duration_units <= candidate.duration_units
                    && (other.cost_units < candidate.cost_units
                        || other.duration_units < candidate.duration_units)
            })
        })
        .collect();
    frontier.sort_by(|left, right| left.id.cmp(&right.id));
    frontier.into_iter().map(|item| item.id.clone()).collect()
}

fn expected_frontier_formula(count: usize) -> Vec<String> {
    (0..count)
        .filter(|index| index % 3 == 0)
        .map(|index| format!("PARETO-{index:04}"))
        .collect()
}

fn build_workload(matrix: &Matrix, profile: &ProfileSpec) -> Result<Workload, String> {
    let n = profile.artifact_count;
    let reachable_count = profile.reachable_task_count;
    let detached_count = profile.task_count - reachable_count;
    let unrelated_count = ((n as u128 * profile.unrelated_artifact_fraction_ppm as u128)
        / 1_000_000u128) as usize;
    let extra_unrelated_count = unrelated_count - detached_count;
    let support_count = n - unrelated_count - reachable_count;
    let mut predecessors: Vec<Vec<usize>> = vec![Vec::new(); profile.task_count];
    let mut target_task_indices = Vec::new();
    let mut shared_task_count = 0usize;

    if profile.profile_id == "deep_chain" {
        for index in 1..reachable_count {
            predecessors[index].push(index - 1);
        }
        for target_index in 0..profile.target_count {
            let position = ((target_index + 1) * reachable_count / profile.target_count) - 1;
            target_task_indices.push(position);
        }
    } else {
        let branch_count = profile.target_count;
        let available_shared = reachable_count.saturating_sub(branch_count);
        if available_shared > 0 {
            let requested = (available_shared as u128
                * profile.shared_task_fraction_ppm as u128
                / 1_000_000u128) as usize;
            let minimum_for_depth =
                reachable_count.saturating_sub(branch_count * (profile.max_dependency_depth - 1));
            let maximum_shared = available_shared.min(profile.max_dependency_depth);
            if minimum_for_depth > maximum_shared {
                return Err(format!(
                    "profile {} cannot fit its reachable tasks within declared dependency depth",
                    profile.profile_id
                ));
            }
            shared_task_count = requested
                .max(1)
                .max(minimum_for_depth)
                .min(maximum_shared);
        }
        let branch_tasks = reachable_count - shared_task_count;
        let base_size = branch_tasks / branch_count;
        let remainder = branch_tasks % branch_count;
        let mut cursor = shared_task_count;
        for branch in 0..branch_count {
            let branch_size = base_size + if branch < remainder { 1 } else { 0 };
            let mut previous = None;
            for _ in 0..branch_size {
                if let Some(prior) = previous {
                    predecessors[cursor].push(prior);
                } else {
                    predecessors[cursor].extend(0..shared_task_count);
                }
                previous = Some(cursor);
                cursor += 1;
            }
            target_task_indices.push(previous.ok_or_else(|| {
                format!("profile {} produced an empty target branch", profile.profile_id)
            })?);
        }
    }

    let mut task_edge_depth = vec![0usize; reachable_count];
    for index in 0..reachable_count {
        if let Some(max_prior_depth) = predecessors[index]
            .iter()
            .map(|prior| task_edge_depth[*prior])
            .max()
        {
            task_edge_depth[index] = max_prior_depth + 1;
        }
    }
    let longest_target_task_path = target_task_indices
        .iter()
        .map(|index| task_edge_depth[*index])
        .max()
        .unwrap_or(0);
    if longest_target_task_path >= profile.max_dependency_depth {
        return Err(format!(
            "profile {} task graph exceeds its declared dependency depth",
            profile.profile_id
        ));
    }
    let support_chunk_depth = profile.max_dependency_depth - longest_target_task_path;
    let unrelated_chunk_depth = (profile.max_dependency_depth - 1).max(1);
    let support_tails = chunk_tails("E-SUPPORT-", support_count, support_chunk_depth);
    let unrelated_tails = chunk_tails(
        "E-UNRELATED-",
        extra_unrelated_count,
        unrelated_chunk_depth,
    );
    let support_roots = if profile.profile_id == "deep_chain" {
        1usize
    } else if shared_task_count > 0 {
        shared_task_count
    } else {
        profile.target_count
    };
    let estimated_edges = (support_tails.len() as u128 * support_roots as u128)
        + (unrelated_tails.len() as u128 * detached_count as u128)
        + (shared_task_count as u128 * profile.target_count as u128)
        + profile.artifact_count as u128
        + profile.task_count as u128;
    if estimated_edges > MAX_DEPENDENCY_EDGES {
        return Err(format!(
            "profile {} would generate too many dependency edges",
            profile.profile_id
        ));
    }

    let mut task_requirements: Vec<Vec<String>> = vec![Vec::new(); profile.task_count];
    for index in 0..profile.task_count {
        let mut requirements: Vec<String> = predecessors[index]
            .iter()
            .map(|prior| artifact_for_task(*prior))
            .collect();
        if predecessors[index].is_empty() {
            if index < reachable_count {
                requirements.extend(support_tails.iter().cloned());
            } else {
                requirements.extend(unrelated_tails.iter().cloned());
            }
        }
        requirements.sort();
        requirements.dedup();
        task_requirements[index] = requirements;
    }

    let target_artifact_ids: Vec<String> = target_task_indices
        .iter()
        .map(|index| artifact_for_task(*index))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    let declared_gate_ids: Vec<String> = (0..profile.gate_count).map(gate_id).collect();
    let available_gate_ids = declared_gate_ids.clone();

    let mut artifacts: Vec<Artifact> = Vec::with_capacity(n);
    for index in 0..support_count {
        let group_start = (index / support_chunk_depth) * support_chunk_depth;
        let mut depends_on = if index > group_start {
            vec![support_id(index - 1)]
        } else {
            Vec::new()
        };
        depends_on.sort();
        artifacts.push(Artifact {
            id: support_id(index),
            status: "valid_for_claim".to_owned(),
            role: "support_evidence".to_owned(),
            depends_on,
        });
    }

    let mut tasks = Vec::with_capacity(profile.task_count);
    for index in 0..profile.task_count {
        let seed_mix = mix64(profile.seed ^ (index as u64).wrapping_mul(0x9e37_79b9));
        let required_gate_ids = vec![gate_id(index % profile.gate_count)];
        let produced_id = artifact_for_task(index);
        let requirements = task_requirements[index].clone();
        artifacts.push(Artifact {
            id: produced_id.clone(),
            status: "stale".to_owned(),
            role: if index < reachable_count {
                "repair_output".to_owned()
            } else {
                "detached_task_output".to_owned()
            },
            depends_on: requirements.clone(),
        });
        tasks.push(RepairTask {
            id: task_id(index),
            produces_artifact_id: produced_id,
            required_artifact_ids: requirements,
            required_gate_ids,
            cost_units: (seed_mix % 1000) + 1,
            duration_units: ((seed_mix >> 17) % 1000) + 1,
            target_reachable: index < reachable_count,
        });
    }

    for index in 0..extra_unrelated_count {
        let group_start = (index / unrelated_chunk_depth) * unrelated_chunk_depth;
        let depends_on = if index > group_start {
            vec![unrelated_id(index - 1)]
        } else {
            Vec::new()
        };
        artifacts.push(Artifact {
            id: unrelated_id(index),
            status: "unknown".to_owned(),
            role: "unrelated_evidence".to_owned(),
            depends_on,
        });
    }

    artifacts.sort_by(|left, right| left.id.cmp(&right.id));
    for artifact in &mut artifacts {
        artifact.depends_on.sort();
        artifact.depends_on.dedup();
    }
    tasks.sort_by(|left, right| left.id.cmp(&right.id));

    let alternatives = objective_alternatives(profile.tradeoff_alternatives);
    let expected_pareto_candidate_ids = expected_frontier_formula(profile.tradeoff_alternatives);
    let budget_scenarios = profile
        .budget_cases
        .clone()
        .unwrap_or_default()
        .into_iter()
        .map(|case| BudgetScenario {
            case_id: case.case_id,
            max_expanded_states: case.max_expanded_states,
            stop_after_first_candidate: case.stop_after_first_candidate,
            expected_status: case.expected_status,
            expected_frontier_complete: case.expected_frontier_complete,
            expected_known_plan_count_min: case.expected_known_plan_count_min,
        })
        .collect();

    Ok(Workload {
        schema_version: WORKLOAD_SCHEMA.to_owned(),
        workload_id: format!("{}-seed-{}", profile.profile_id, profile.seed),
        suite_id: matrix.suite_id.clone(),
        profile_id: profile.profile_id.clone(),
        seed: profile.seed,
        synthetic_only: true,
        claim_ceiling: CLAIM_CEILING.to_owned(),
        artifact_count: n,
        task_count: profile.task_count,
        reachable_task_count: reachable_count,
        target_artifact_ids,
        declared_gate_ids,
        available_gate_ids,
        max_expanded_states: profile.max_expanded_states,
        artifacts,
        tasks,
        objective_alternatives: alternatives,
        expected_pareto_candidate_ids,
        budget_scenarios,
        independent_exhaustive_oracle_required: profile.independent_exhaustive_oracle_required,
        required_expected_class: profile.required_expected_class.clone(),
        profile_purpose: profile.purpose.clone(),
        generator_version: GENERATOR_VERSION.to_owned(),
        generator_parameters: GeneratorParameters {
            unrelated_artifact_fraction_ppm: profile.unrelated_artifact_fraction_ppm,
            shared_task_fraction_ppm: profile.shared_task_fraction_ppm,
            max_dependency_depth: profile.max_dependency_depth,
            tradeoff_alternatives: profile.tradeoff_alternatives,
        },
    })
}

fn verify_workload(workload: &Workload) -> Result<(), String> {
    if workload.schema_version != WORKLOAD_SCHEMA || !workload.synthetic_only {
        return Err("generated workload schema or claim ceiling is invalid".to_owned());
    }
    if workload.artifacts.len() != workload.artifact_count
        || workload.tasks.len() != workload.task_count
        || workload.target_artifact_ids.is_empty()
    {
        return Err("generated counts do not match materialized lists".to_owned());
    }

    let mut artifacts_by_id = BTreeMap::new();
    for artifact in &workload.artifacts {
        if artifacts_by_id.insert(artifact.id.as_str(), artifact).is_some() {
            return Err(format!("duplicate artifact id: {}", artifact.id));
        }
    }
    for artifact in &workload.artifacts {
        let mut dependencies = BTreeSet::new();
        for dependency in &artifact.depends_on {
            if !artifacts_by_id.contains_key(dependency.as_str()) {
                return Err(format!("{} has unknown dependency {}", artifact.id, dependency));
            }
            if !dependencies.insert(dependency) {
                return Err(format!("{} repeats dependency {}", artifact.id, dependency));
            }
            if dependency == &artifact.id {
                return Err(format!("{} depends on itself", artifact.id));
            }
        }
    }

    // Kahn's algorithm over dependency edges; avoids recursion depth limits on large profiles.
    let mut remaining_dependencies: BTreeMap<&str, usize> = workload
        .artifacts
        .iter()
        .map(|artifact| (artifact.id.as_str(), artifact.depends_on.len()))
        .collect();
    let mut dependents: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for artifact in &workload.artifacts {
        for dependency in &artifact.depends_on {
            dependents
                .entry(dependency.as_str())
                .or_default()
                .push(artifact.id.as_str());
        }
    }
    let mut ready: VecDeque<&str> = remaining_dependencies
        .iter()
        .filter_map(|(id, count)| (*count == 0).then_some(*id))
        .collect();
    let mut depth: BTreeMap<&str, usize> =
        remaining_dependencies.keys().map(|id| (*id, 0usize)).collect();
    let mut visited = 0usize;
    while let Some(id) = ready.pop_front() {
        visited += 1;
        if let Some(children) = dependents.get(id) {
            for child in children {
                let next_depth = depth[id].saturating_add(1);
                let child_depth = depth
                    .get_mut(*child)
                    .ok_or_else(|| format!("internal graph depth index missing {child}"))?;
                *child_depth = (*child_depth).max(next_depth);
                let count = remaining_dependencies
                    .get_mut(*child)
                    .ok_or_else(|| format!("internal graph index missing {child}"))?;
                *count -= 1;
                if *count == 0 {
                    ready.push_back(*child);
                }
            }
        }
    }
    if visited != workload.artifact_count {
        return Err("generated artifact dependency graph contains a cycle".to_owned());
    }
    let maximum_depth = depth.values().copied().max().unwrap_or(0);
    if maximum_depth > workload.generator_parameters.max_dependency_depth {
        return Err(format!(
            "generated dependency depth {maximum_depth} exceeds declared bound {}",
            workload.generator_parameters.max_dependency_depth
        ));
    }

    let mut task_ids = BTreeSet::new();
    let mut task_outputs = BTreeSet::new();
    for task in &workload.tasks {
        if !task_ids.insert(task.id.as_str()) {
            return Err(format!("duplicate task id: {}", task.id));
        }
        if !task_outputs.insert(task.produces_artifact_id.as_str()) {
            return Err(format!("multiple tasks produce {}", task.produces_artifact_id));
        }
        let artifact = artifacts_by_id
            .get(task.produces_artifact_id.as_str())
            .ok_or_else(|| format!("task {} outputs a missing artifact", task.id))?;
        if artifact.depends_on != task.required_artifact_ids {
            return Err(format!(
                "task {} requirements disagree with output artifact dependencies",
                task.id
            ));
        }
        if task.required_gate_ids.iter().any(|gate| !workload.declared_gate_ids.contains(gate)) {
            return Err(format!("task {} requires an undeclared gate", task.id));
        }
    }
    if workload
        .available_gate_ids
        .iter()
        .any(|gate| !workload.declared_gate_ids.contains(gate))
    {
        return Err("available gates must be a subset of declared gates".to_owned());
    }
    for target in &workload.target_artifact_ids {
        if !artifacts_by_id.contains_key(target.as_str()) {
            return Err(format!("unknown target artifact {target}"));
        }
    }

    let mut reachable = BTreeSet::new();
    let mut stack = workload.target_artifact_ids.clone();
    while let Some(id) = stack.pop() {
        if !reachable.insert(id.clone()) {
            continue;
        }
        let artifact = artifacts_by_id
            .get(id.as_str())
            .ok_or_else(|| format!("reachability traversal lost artifact {id}"))?;
        stack.extend(artifact.depends_on.iter().cloned());
    }
    let reachable_tasks = workload
        .tasks
        .iter()
        .filter(|task| reachable.contains(&task.produces_artifact_id))
        .count();
    if reachable_tasks != workload.reachable_task_count {
        return Err(format!(
            "expected {} target-reachable tasks but found {reachable_tasks}",
            workload.reachable_task_count
        ));
    }
    for task in &workload.tasks {
        let actual = reachable.contains(&task.produces_artifact_id);
        if actual != task.target_reachable {
            return Err(format!("task {} has incorrect reachability label", task.id));
        }
    }

    let measured_frontier = independent_pareto_ids(&workload.objective_alternatives);
    if measured_frontier != workload.expected_pareto_candidate_ids {
        return Err("objective-alternative frontier disagrees with independent oracle".to_owned());
    }
    Ok(())
}

pub fn generate_workload(
    matrix_bytes: &[u8],
    profile_id: &str,
    source_commit: &str,
) -> Result<GeneratedWorkload, String> {
    if source_commit.len() != 40 || !source_commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(
            "generator source commit must be an exact 40-character hexadecimal SHA".to_owned(),
        );
    }
    let matrix = parse_matrix(matrix_bytes)?;
    let profile = matrix
        .profiles
        .iter()
        .find(|profile| profile.profile_id == profile_id)
        .ok_or_else(|| format!("unknown profile id: {profile_id}"))?;
    let workload = build_workload(&matrix, profile)?;
    verify_workload(&workload)?;
    let mut workload_bytes = serde_json::to_vec_pretty(&workload)
        .map_err(|error| format!("could not serialize workload: {error}"))?;
    workload_bytes.push(b'\n');
    let receipt = Receipt {
        schema_version: RECEIPT_SCHEMA.to_owned(),
        generator_version: GENERATOR_VERSION.to_owned(),
        suite_id: matrix.suite_id.clone(),
        profile_id: profile.profile_id.clone(),
        seed: profile.seed,
        generator_source_commit: source_commit.to_ascii_lowercase(),
        profile_matrix_sha256: hash_hex(matrix_bytes),
        exact_input_sha256: hash_hex(&workload_bytes),
        exact_input_bytes: workload_bytes.len(),
        artifact_count: workload.artifact_count,
        task_count: workload.task_count,
        reachable_task_count: workload.reachable_task_count,
        target_count: workload.target_artifact_ids.len(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
    };
    Ok(GeneratedWorkload {
        workload_bytes,
        receipt,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MATRIX_BYTES: &[u8] = include_bytes!(
        "../../../docs/research/fixtures/HABITAT_REVALIDATION_SCALE_BENCHMARK_V0_1.json"
    );
    const TEST_COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

    #[test]
    fn every_frozen_profile_materializes_exact_counts_and_repeats_identically() {
        let matrix = parse_matrix(MATRIX_BYTES).unwrap();
        for profile in &matrix.profiles {
            let first = generate_workload(MATRIX_BYTES, &profile.profile_id, TEST_COMMIT).unwrap();
            let second = generate_workload(MATRIX_BYTES, &profile.profile_id, TEST_COMMIT).unwrap();
            assert_eq!(first.workload_bytes, second.workload_bytes, "{}", profile.profile_id);
            assert_eq!(first.receipt, second.receipt, "{}", profile.profile_id);
            assert_eq!(first.receipt.exact_input_sha256, hash_hex(&first.workload_bytes));
            assert_eq!(first.receipt.exact_input_bytes, first.workload_bytes.len());
            let workload: Workload = serde_json::from_slice(&first.workload_bytes).unwrap();
            verify_workload(&workload).unwrap();
            assert_eq!(workload.artifacts.len(), profile.artifact_count);
            assert_eq!(workload.tasks.len(), profile.task_count);
            assert_eq!(workload.target_artifact_ids.len(), profile.target_count);
        }
    }

    #[test]
    fn pairwise_reference_oracle_matches_the_declared_frontier_formula() {
        for count in [1, 2, 3, 8, 16, 32, 64] {
            let candidates = objective_alternatives(count);
            assert_eq!(
                independent_pareto_ids(&candidates),
                expected_frontier_formula(count)
            );
        }
    }

    #[test]
    fn unknown_matrix_fields_are_rejected() {
        let mut value: Value = serde_json::from_slice(MATRIX_BYTES).unwrap();
        value["surprise_override"] = Value::Bool(true);
        assert!(parse_matrix(value.to_string().as_bytes()).is_err());
    }

    #[test]
    fn invalid_source_commit_and_unknown_profile_fail_closed() {
        assert!(generate_workload(MATRIX_BYTES, "micro_oracle", "main").is_err());
        assert!(generate_workload(MATRIX_BYTES, "missing-profile", TEST_COMMIT).is_err());
    }

    #[test]
    fn absurd_workload_sizes_are_rejected_before_materialization() {
        let mut value: Value = serde_json::from_slice(MATRIX_BYTES).unwrap();
        value["profiles"][0]["artifact_count"] = Value::from(1_000_000_000u64);
        assert!(parse_matrix(value.to_string().as_bytes()).is_err());
    }

    #[test]
    fn unrelated_fraction_cannot_consume_required_target_graph() {
        let mut value: Value = serde_json::from_slice(MATRIX_BYTES).unwrap();
        value["profiles"][0]["unrelated_artifact_fraction_ppm"] = Value::from(999_999u64);
        assert!(parse_matrix(value.to_string().as_bytes()).is_err());
    }

    #[test]
    fn cyclic_or_mismatched_graphs_fail_closed() {
        let generated =
            generate_workload(MATRIX_BYTES, "micro_oracle", TEST_COMMIT).unwrap();
        let mut workload: Workload = serde_json::from_slice(&generated.workload_bytes).unwrap();
        let target_id = workload.target_artifact_ids[0].clone();
        let target = workload
            .artifacts
            .iter_mut()
            .find(|artifact| artifact.id == target_id)
            .unwrap();
        target.depends_on.push(target_id);
        assert!(verify_workload(&workload).is_err());

        let mut workload: Workload = serde_json::from_slice(&generated.workload_bytes).unwrap();
        workload.tasks[0].required_artifact_ids.push("E-UNKNOWN".to_owned());
        assert!(verify_workload(&workload).is_err());
    }

    #[test]
    fn known_sha256_answer_is_correct() {
        assert_eq!(
            hash_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
