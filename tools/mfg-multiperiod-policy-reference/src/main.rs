use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::process::ExitCode;

const CORPUS_JSON: &str =
    include_str!("../../../fixtures/manufacturing/mfg_multiperiod_policy_cases_v0_1.json");
const ANSWERS_JSON: &str =
    include_str!("../../../fixtures/manufacturing/mfg_multiperiod_policy_known_answers_v0_1.json");
const CORPUS_SCHEMA_JSON: &str =
    include_str!("../../../schemas/manufacturing/mfg_multiperiod_policy_cases_v0_1.schema.json");
const ANSWERS_SCHEMA_JSON: &str = include_str!(
    "../../../schemas/manufacturing/mfg_multiperiod_policy_known_answers_v0_1.schema.json"
);
const PROBABILITY_DENOMINATOR: u64 = 1_000_000;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    schema_id: String,
    benchmark_id: String,
    evidence_class: String,
    horizon: usize,
    family_manifest: Vec<FamilySpec>,
    case_count: usize,
    cases: Vec<PolicyCase>,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct FamilySpec {
    scenario_family_id: String,
    case_count: usize,
    replication_seeds: Vec<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyCase {
    case_id: String,
    scenario_family_id: String,
    seed: u64,
    horizon: usize,
    jobs: Vec<Job>,
    resources: Vec<Resource>,
    scenarios: Vec<Scenario>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Job {
    job_id: String,
    due_period: usize,
    tardiness_penalty_per_period: u64,
    unserved_penalty_units: u64,
    routes: Vec<JobRoute>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct JobRoute {
    resource_id: String,
    production_cost_units: u64,
    expected_quality_loss_units: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Resource {
    resource_id: String,
    capacity_units_by_period: Vec<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
struct Observation {
    failed_resource_ids: Vec<String>,
    unknown_resource_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct RejectedPair {
    job_id: String,
    resource_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Scenario {
    scenario_id: String,
    probability_ppm: u64,
    observations_by_period: Vec<Observation>,
    hidden_quality_rejections: Vec<RejectedPair>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct KnownAnswerFile {
    schema_id: String,
    benchmark_id: String,
    evidence_class: String,
    answer_count: usize,
    answers: Vec<KnownAnswer>,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct KnownAnswer {
    case_id: String,
    greedy_expected_cost_numerator: u64,
    scarcity_greedy_expected_cost_numerator: u64,
    exact_expected_cost_numerator: u64,
    exact_quality_rejection_numerator: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
struct Assignment {
    job_id: String,
    resource_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct DecisionNode {
    period: usize,
    scenario_ids: Vec<String>,
    observable_history: Vec<Observation>,
    completed_routes_before: BTreeMap<String, String>,
    assignments: Vec<Assignment>,
    completed_routes_after: BTreeMap<String, String>,
    expected_cost_numerator: u64,
    children: Vec<DecisionNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct PolicyForest {
    roots: Vec<DecisionNode>,
}

#[derive(Debug, Serialize)]
struct CaseResult {
    case_id: String,
    scenario_family_id: String,
    seed: u64,
    scenario_count: usize,
    decision_node_count: usize,
    greedy_expected_cost_numerator: u64,
    scarcity_greedy_expected_cost_numerator: u64,
    exact_expected_cost_numerator: u64,
    myopic_regret_numerator: u64,
    scarcity_greedy_regret_numerator: u64,
    greedy_quality_rejection_numerator: u64,
    scarcity_greedy_quality_rejection_numerator: u64,
    exact_quality_rejection_numerator: u64,
    greedy_policy: PolicyForest,
    scarcity_greedy_policy: PolicyForest,
    exact_policy: PolicyForest,
    non_anticipativity_verified: bool,
    hidden_quality_is_evaluator_only: bool,
}

#[derive(Debug, Serialize)]
struct Report {
    benchmark_id: &'static str,
    evaluator_version: &'static str,
    evidence_class: &'static str,
    family_count: usize,
    case_count: usize,
    scenario_leaf_count: usize,
    decision_node_count: usize,
    greedy_expected_cost_numerator: u64,
    scarcity_greedy_expected_cost_numerator: u64,
    exact_expected_cost_numerator: u64,
    myopic_regret_numerator: u64,
    scarcity_greedy_regret_numerator: u64,
    exact_policy_non_anticipativity_verified: bool,
    recommendations_are_non_binding: bool,
    capacity_reservations_created: bool,
    machine_commands_issued: bool,
    physical_acceptance_claimed: bool,
    production_release_authorized: bool,
    cases: Vec<CaseResult>,
    claim_ceiling: &'static str,
}

fn fail(message: impl Into<String>) -> String {
    message.into()
}

fn validate_schema(instance: &serde_json::Value, schema_source: &str) -> Result<(), String> {
    let schema: serde_json::Value = serde_json::from_str(schema_source)
        .map_err(|error| fail(format!("schema_parse:{error}")))?;
    let validator = jsonschema::draft202012::options()
        .offline()
        .build(&schema)
        .map_err(|error| fail(format!("schema_compile:{error}")))?;
    validator
        .validate(instance)
        .map_err(|error| fail(format!("schema_validation:{error}")))
}

fn canonical_observation(observation: &Observation) -> Result<Observation, String> {
    let mut canonical = observation.clone();
    canonical.failed_resource_ids.sort();
    canonical.unknown_resource_ids.sort();
    for values in [
        &canonical.failed_resource_ids,
        &canonical.unknown_resource_ids,
    ] {
        if values.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(fail("observation_contains_duplicate_resource_id"));
        }
    }
    let failed: BTreeSet<&str> = canonical
        .failed_resource_ids
        .iter()
        .map(String::as_str)
        .collect();
    if canonical
        .unknown_resource_ids
        .iter()
        .any(|id| failed.contains(id.as_str()))
    {
        return Err(fail(
            "resource_cannot_be_failed_and_unknown_in_same_observation",
        ));
    }
    Ok(canonical)
}

fn history_prefix(
    case: &PolicyCase,
    scenario_index: usize,
    period: usize,
) -> Result<Vec<Observation>, String> {
    let scenario = case
        .scenarios
        .get(scenario_index)
        .ok_or_else(|| fail("scenario_index_out_of_range"))?;
    scenario.observations_by_period[..=period]
        .iter()
        .map(canonical_observation)
        .collect()
}

fn validate_case(case: &PolicyCase) -> Result<(), String> {
    if case.case_id.trim().is_empty()
        || case.scenario_family_id.trim().is_empty()
        || case.horizon != 3
        || case.jobs.len() < 2
        || case.resources.len() < 2
        || case.scenarios.len() < 2
    {
        return Err(fail("case_identity_or_minimum_model_size_invalid"));
    }

    let mut resource_ids = HashSet::new();
    for resource in &case.resources {
        if resource.resource_id.trim().is_empty()
            || !resource_ids.insert(resource.resource_id.as_str())
            || resource.capacity_units_by_period.len() != case.horizon
        {
            return Err(fail("resource_identity_or_calendar_invalid"));
        }
    }

    let mut job_ids = HashSet::new();
    for job in &case.jobs {
        if job.job_id.trim().is_empty()
            || !job_ids.insert(job.job_id.as_str())
            || !(1..=case.horizon).contains(&job.due_period)
            || job.unserved_penalty_units == 0
            || job.routes.is_empty()
        {
            return Err(fail("job_identity_or_constraints_invalid"));
        }
        let mut route_ids = HashSet::new();
        for route in &job.routes {
            if !resource_ids.contains(route.resource_id.as_str())
                || !route_ids.insert(route.resource_id.as_str())
                || route.production_cost_units == 0
            {
                return Err(fail("job_route_identity_or_cost_invalid"));
            }
        }
    }

    let mut scenario_ids = HashSet::new();
    let mut total_probability = 0_u64;
    for (index, scenario) in case.scenarios.iter().enumerate() {
        if scenario.scenario_id.trim().is_empty()
            || !scenario_ids.insert(scenario.scenario_id.as_str())
            || scenario.probability_ppm == 0
            || scenario.observations_by_period.len() != case.horizon
        {
            return Err(fail(format!(
                "scenario_{index}_identity_probability_or_horizon_invalid"
            )));
        }
        total_probability = total_probability
            .checked_add(scenario.probability_ppm)
            .ok_or_else(|| fail("scenario_probability_overflow"))?;

        for observation in &scenario.observations_by_period {
            let canonical = canonical_observation(observation)?;
            for resource_id in canonical
                .failed_resource_ids
                .iter()
                .chain(canonical.unknown_resource_ids.iter())
            {
                if !resource_ids.contains(resource_id.as_str()) {
                    return Err(fail("observation_references_unknown_resource"));
                }
            }
        }

        let mut rejected = BTreeSet::new();
        for pair in &scenario.hidden_quality_rejections {
            let job = case
                .jobs
                .iter()
                .find(|job| job.job_id == pair.job_id)
                .ok_or_else(|| fail("hidden_quality_event_references_unknown_job"))?;
            if !job
                .routes
                .iter()
                .any(|route| route.resource_id == pair.resource_id)
                || !rejected.insert((pair.job_id.as_str(), pair.resource_id.as_str()))
            {
                return Err(fail(
                    "hidden_quality_event_references_invalid_or_duplicate_route",
                ));
            }
        }
    }
    if total_probability != PROBABILITY_DENOMINATOR {
        return Err(fail("scenario_probabilities_must_sum_to_one_million_ppm"));
    }

    Ok(())
}

fn validate_corpus(corpus: &Corpus) -> Result<(), String> {
    if corpus.schema_id != "https://luminousdynamics.io/schemas/mfg-multiperiod-policy-case-v0.1"
        || corpus.benchmark_id != "MFG-MULTIPERIOD-POLICY-001"
        || corpus.evidence_class != "synthetic"
        || corpus.horizon != 3
        || corpus.family_manifest.len() != 6
        || corpus.case_count != 12
        || corpus.cases.len() != 12
        || corpus.claim_ceiling.trim().is_empty()
    {
        return Err(fail("corpus_manifest_invalid"));
    }

    let mut family_ids = HashSet::new();
    for family in &corpus.family_manifest {
        let seeds: BTreeSet<u64> = family.replication_seeds.iter().copied().collect();
        if family.scenario_family_id.trim().is_empty()
            || !family_ids.insert(family.scenario_family_id.as_str())
            || family.case_count != 2
            || family.replication_seeds.len() != 2
            || seeds != [1_u64, 2_u64].into_iter().collect()
        {
            return Err(fail("family_manifest_invalid"));
        }
    }

    let mut case_ids = HashSet::new();
    let mut family_counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut family_seeds: BTreeMap<&str, BTreeSet<u64>> = BTreeMap::new();
    for case in &corpus.cases {
        if !family_ids.contains(case.scenario_family_id.as_str())
            || !case_ids.insert(case.case_id.as_str())
            || !(1..=2).contains(&case.seed)
            || !family_seeds
                .entry(case.scenario_family_id.as_str())
                .or_default()
                .insert(case.seed)
        {
            return Err(fail("case_identity_or_family_seed_invalid"));
        }
        *family_counts
            .entry(case.scenario_family_id.as_str())
            .or_default() += 1;
        validate_case(case)?;
    }
    for family in &corpus.family_manifest {
        if family_counts.get(family.scenario_family_id.as_str()) != Some(&2)
            || family_seeds.get(family.scenario_family_id.as_str())
                != Some(&[1_u64, 2_u64].into_iter().collect())
        {
            return Err(fail("family_case_seed_coverage_invalid"));
        }
    }
    Ok(())
}

fn route_for<'a>(job: &'a Job, resource_id: &str) -> Result<&'a JobRoute, String> {
    job.routes
        .iter()
        .find(|route| route.resource_id == resource_id)
        .ok_or_else(|| fail("assignment_uses_ineligible_resource"))
}

fn immediate_action_cost(
    case: &PolicyCase,
    period: usize,
    action: &[Assignment],
) -> Result<u64, String> {
    let mut total = 0_u64;
    for assignment in action {
        let job = case
            .jobs
            .iter()
            .find(|job| job.job_id == assignment.job_id)
            .ok_or_else(|| fail("assignment_references_unknown_job"))?;
        let route = route_for(job, &assignment.resource_id)?;
        let completion_period = period + 1;
        let late_periods = completion_period.saturating_sub(job.due_period) as u64;
        let late_cost = late_periods
            .checked_mul(job.tardiness_penalty_per_period)
            .ok_or_else(|| fail("tardiness_cost_overflow"))?;
        total = total
            .checked_add(route.production_cost_units)
            .and_then(|value| value.checked_add(route.expected_quality_loss_units))
            .and_then(|value| value.checked_add(late_cost))
            .ok_or_else(|| fail("immediate_action_cost_overflow"))?;
    }
    Ok(total)
}

struct ActionEnumerator<'a> {
    jobs: Vec<&'a Job>,
    period: usize,
    completed: &'a BTreeMap<String, String>,
    resources: BTreeMap<String, &'a Resource>,
    failed: BTreeSet<String>,
    unknown: BTreeSet<String>,
}

impl ActionEnumerator<'_> {
    fn visit(
        &self,
        index: usize,
        used: &mut BTreeMap<String, u64>,
        current: &mut Vec<Assignment>,
        out: &mut Vec<Vec<Assignment>>,
    ) {
        if index == self.jobs.len() {
            current.sort();
            out.push(current.clone());
            return;
        }
        let job = self.jobs[index];
        if self.completed.contains_key(&job.job_id) {
            self.visit(index + 1, used, current, out);
            return;
        }

        self.visit(index + 1, used, current, out);
        let mut routes = job.routes.clone();
        routes.sort_by(|a, b| a.resource_id.cmp(&b.resource_id));
        for route in routes {
            let resource_id = &route.resource_id;
            if self.failed.contains(resource_id) || self.unknown.contains(resource_id) {
                continue;
            }
            let Some(resource) = self.resources.get(resource_id) else {
                continue;
            };
            let capacity = resource.capacity_units_by_period[self.period];
            let consumed = used.get(resource_id).copied().unwrap_or(0);
            if consumed >= capacity {
                continue;
            }

            *used.entry(route.resource_id.clone()).or_default() += 1;
            current.push(Assignment {
                job_id: job.job_id.clone(),
                resource_id: route.resource_id.clone(),
            });
            self.visit(index + 1, used, current, out);
            current.pop();
            let remaining = used.get_mut(resource_id).expect("used capacity inserted");
            *remaining -= 1;
            if *remaining == 0 {
                used.remove(resource_id);
            }
        }
    }
}

fn enumerate_actions(
    case: &PolicyCase,
    period: usize,
    completed: &BTreeMap<String, String>,
    observation: &Observation,
) -> Result<Vec<Vec<Assignment>>, String> {
    let mut jobs: Vec<&Job> = case.jobs.iter().collect();
    jobs.sort_by(|a, b| a.job_id.cmp(&b.job_id));
    let resources: BTreeMap<String, &Resource> = case
        .resources
        .iter()
        .map(|resource| (resource.resource_id.clone(), resource))
        .collect();
    let enumerator = ActionEnumerator {
        jobs,
        period,
        completed,
        resources,
        failed: observation.failed_resource_ids.iter().cloned().collect(),
        unknown: observation.unknown_resource_ids.iter().cloned().collect(),
    };

    let mut out = Vec::new();
    enumerator.visit(0, &mut BTreeMap::new(), &mut Vec::new(), &mut out);
    out.sort();
    out.dedup();
    if out.is_empty() {
        return Err(fail("action_enumeration_produced_no_action"));
    }
    Ok(out)
}

fn completed_after(
    completed: &BTreeMap<String, String>,
    assignments: &[Assignment],
) -> Result<BTreeMap<String, String>, String> {
    let mut next = completed.clone();
    for assignment in assignments {
        if next
            .insert(assignment.job_id.clone(), assignment.resource_id.clone())
            .is_some()
        {
            return Err(fail("job_assigned_more_than_once"));
        }
    }
    Ok(next)
}

fn terminal_unserved_cost(
    case: &PolicyCase,
    completed: &BTreeMap<String, String>,
) -> Result<u64, String> {
    case.jobs.iter().try_fold(0_u64, |total, job| {
        let cost = if completed.contains_key(&job.job_id) {
            0
        } else {
            job.unserved_penalty_units
        };
        total
            .checked_add(cost)
            .ok_or_else(|| fail("terminal_unserved_cost_overflow"))
    })
}

type HistoryState = (Vec<Observation>, BTreeMap<String, String>);
type BestCandidate = (
    u64,
    Vec<Assignment>,
    BTreeMap<String, String>,
    Vec<DecisionNode>,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DecisionRule {
    MyopicImmediateCost,
    ScarcityAndDueDate,
    Exact,
}

fn partition_scenarios(
    case: &PolicyCase,
    scenario_indices: &[usize],
    period: usize,
    completed: &BTreeMap<String, String>,
) -> Result<BTreeMap<HistoryState, Vec<usize>>, String> {
    let mut groups: BTreeMap<HistoryState, Vec<usize>> = BTreeMap::new();
    for &index in scenario_indices {
        let key = (history_prefix(case, index, period)?, completed.clone());
        groups.entry(key).or_default().push(index);
    }
    for indices in groups.values_mut() {
        indices.sort_by(|a, b| {
            case.scenarios[*a]
                .scenario_id
                .cmp(&case.scenarios[*b].scenario_id)
        });
    }
    Ok(groups)
}

fn group_probability(case: &PolicyCase, indices: &[usize]) -> Result<u64, String> {
    indices.iter().try_fold(0_u64, |total, index| {
        total
            .checked_add(case.scenarios[*index].probability_ppm)
            .ok_or_else(|| fail("group_probability_overflow"))
    })
}

fn scarcity_priority_ranks(
    case: &PolicyCase,
    period: usize,
    completed_before: &BTreeMap<String, String>,
    observation: &Observation,
) -> BTreeMap<String, usize> {
    let failed: BTreeSet<&str> = observation
        .failed_resource_ids
        .iter()
        .map(String::as_str)
        .collect();
    let unknown: BTreeSet<&str> = observation
        .unknown_resource_ids
        .iter()
        .map(String::as_str)
        .collect();
    let resource_by_id: BTreeMap<&str, &Resource> = case
        .resources
        .iter()
        .map(|resource| (resource.resource_id.as_str(), resource))
        .collect();

    let mut remaining: Vec<(&Job, usize)> = case
        .jobs
        .iter()
        .filter(|job| !completed_before.contains_key(&job.job_id))
        .map(|job| {
            let eligible_count = job
                .routes
                .iter()
                .filter(|route| {
                    if failed.contains(route.resource_id.as_str())
                        || unknown.contains(route.resource_id.as_str())
                    {
                        return false;
                    }
                    resource_by_id
                        .get(route.resource_id.as_str())
                        .is_some_and(|resource| {
                            resource
                                .capacity_units_by_period
                                .iter()
                                .skip(period)
                                .any(|capacity| *capacity > 0)
                        })
                })
                .count();
            (job, eligible_count)
        })
        .collect();

    remaining.sort_by(|(left_job, left_count), (right_job, right_count)| {
        left_job
            .due_period
            .cmp(&right_job.due_period)
            .then(left_count.cmp(right_count))
            .then(left_job.job_id.cmp(&right_job.job_id))
    });

    remaining
        .into_iter()
        .enumerate()
        .map(|(rank, (job, _))| (job.job_id.clone(), rank))
        .collect()
}

fn action_priority_key(
    case: &PolicyCase,
    period: usize,
    action: &[Assignment],
    priority_ranks: &BTreeMap<String, usize>,
) -> (Reverse<usize>, Vec<usize>, u64, Vec<Assignment>) {
    let mut assigned_ranks: Vec<usize> = action
        .iter()
        .filter_map(|assignment| priority_ranks.get(&assignment.job_id).copied())
        .collect();
    assigned_ranks.sort_unstable();
    (
        Reverse(action.len()),
        assigned_ranks,
        immediate_action_cost(case, period, action).unwrap_or(u64::MAX),
        action.to_vec(),
    )
}

fn solve_group(
    case: &PolicyCase,
    indices: &[usize],
    period: usize,
    completed_before: &BTreeMap<String, String>,
    decision_rule: DecisionRule,
) -> Result<DecisionNode, String> {
    if indices.is_empty() || period >= case.horizon {
        return Err(fail("decision_group_empty_or_period_out_of_range"));
    }
    let history = history_prefix(case, indices[0], period)?;
    for &index in indices.iter().skip(1) {
        if history_prefix(case, index, period)? != history {
            return Err(fail(
                "decision_group_contains_distinct_observable_histories",
            ));
        }
    }
    let observation = history
        .last()
        .ok_or_else(|| fail("decision_history_missing_current_observation"))?;
    let mut actions = enumerate_actions(case, period, completed_before, observation)?;

    if decision_rule == DecisionRule::MyopicImmediateCost {
        actions.sort_by_key(|action| {
            (
                Reverse(action.len()),
                immediate_action_cost(case, period, action).unwrap_or(u64::MAX),
                action.clone(),
            )
        });
        actions.truncate(1);
    } else if decision_rule == DecisionRule::ScarcityAndDueDate {
        let priority_ranks = scarcity_priority_ranks(case, period, completed_before, observation);
        actions.sort_by_key(|action| action_priority_key(case, period, action, &priority_ranks));
        actions.truncate(1);
    }

    let probability = group_probability(case, indices)?;
    let mut best: Option<BestCandidate> = None;

    for action in actions {
        let completed_after_action = completed_after(completed_before, &action)?;
        let immediate = immediate_action_cost(case, period, &action)?;
        let mut total = immediate
            .checked_mul(probability)
            .ok_or_else(|| fail("weighted_immediate_cost_overflow"))?;
        let mut children = Vec::new();

        if period + 1 < case.horizon {
            let child_groups =
                partition_scenarios(case, indices, period + 1, &completed_after_action)?;
            for child_indices in child_groups.values() {
                let child = solve_group(
                    case,
                    child_indices,
                    period + 1,
                    &completed_after_action,
                    decision_rule,
                )?;
                total = total
                    .checked_add(child.expected_cost_numerator)
                    .ok_or_else(|| fail("policy_child_cost_overflow"))?;
                children.push(child);
            }
        } else {
            for &scenario_index in indices {
                let scenario = &case.scenarios[scenario_index];
                let terminal = terminal_unserved_cost(case, &completed_after_action)?;
                let weighted = terminal
                    .checked_mul(scenario.probability_ppm)
                    .ok_or_else(|| fail("weighted_terminal_cost_overflow"))?;
                total = total
                    .checked_add(weighted)
                    .ok_or_else(|| fail("policy_terminal_cost_overflow"))?;
            }
        }

        let replace = best.as_ref().is_none_or(|(old_total, old_action, _, _)| {
            total < *old_total || (total == *old_total && action < *old_action)
        });
        if replace {
            best = Some((total, action, completed_after_action, children));
        }
        if decision_rule != DecisionRule::Exact {
            break;
        }
    }

    let (expected_cost_numerator, assignments, completed_after_action, children) =
        best.ok_or_else(|| fail("policy_group_has_no_candidate_action"))?;
    let mut scenario_ids: Vec<String> = indices
        .iter()
        .map(|index| case.scenarios[*index].scenario_id.clone())
        .collect();
    scenario_ids.sort();

    Ok(DecisionNode {
        period,
        scenario_ids,
        observable_history: history,
        completed_routes_before: completed_before.clone(),
        assignments,
        completed_routes_after: completed_after_action,
        expected_cost_numerator,
        children,
    })
}

fn build_policy(case: &PolicyCase, decision_rule: DecisionRule) -> Result<PolicyForest, String> {
    let indices: Vec<usize> = (0..case.scenarios.len()).collect();
    let empty_completed = BTreeMap::new();
    let roots = partition_scenarios(case, &indices, 0, &empty_completed)?
        .values()
        .map(|group| solve_group(case, group, 0, &empty_completed, decision_rule))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PolicyForest { roots })
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct DecisionKey {
    period: usize,
    history: Vec<Observation>,
    completed_before: BTreeMap<String, String>,
}

fn verify_node(
    case: &PolicyCase,
    node: &DecisionNode,
    action_by_key: &mut BTreeMap<DecisionKey, Vec<Assignment>>,
) -> Result<usize, String> {
    let key = DecisionKey {
        period: node.period,
        history: node.observable_history.clone(),
        completed_before: node.completed_routes_before.clone(),
    };
    if let Some(previous) = action_by_key.insert(key, node.assignments.clone())
        && previous != node.assignments
    {
        return Err(fail(
            "non_anticipativity_violation_for_identical_observable_history",
        ));
    }

    let mut job_ids = BTreeSet::new();
    let mut resource_use: BTreeMap<&str, u64> = BTreeMap::new();
    let current_observation = node
        .observable_history
        .last()
        .ok_or_else(|| fail("policy_node_missing_current_observation"))?;
    for assignment in &node.assignments {
        if !job_ids.insert(assignment.job_id.as_str())
            || node
                .completed_routes_before
                .contains_key(&assignment.job_id)
        {
            return Err(fail("policy_node_assigns_duplicate_or_completed_job"));
        }
        let job = case
            .jobs
            .iter()
            .find(|job| job.job_id == assignment.job_id)
            .ok_or_else(|| fail("policy_assignment_unknown_job"))?;
        let route = route_for(job, &assignment.resource_id)?;
        let resource = case
            .resources
            .iter()
            .find(|resource| resource.resource_id == assignment.resource_id)
            .ok_or_else(|| fail("policy_assignment_unknown_resource"))?;
        if current_observation
            .failed_resource_ids
            .contains(&assignment.resource_id)
            || current_observation
                .unknown_resource_ids
                .contains(&assignment.resource_id)
        {
            return Err(fail("policy_uses_failed_or_unknown_resource"));
        }
        *resource_use
            .entry(assignment.resource_id.as_str())
            .or_default() += 1;
        if resource_use[assignment.resource_id.as_str()]
            > resource.capacity_units_by_period[node.period]
        {
            return Err(fail("policy_exceeds_time_indexed_resource_capacity"));
        }
        let _ = route;
    }
    if completed_after(&node.completed_routes_before, &node.assignments)?
        != node.completed_routes_after
    {
        return Err(fail("policy_completed_route_state_mismatch"));
    }

    let mut count = 1;
    for child in &node.children {
        count += verify_node(case, child, action_by_key)?;
    }
    Ok(count)
}

fn verify_policy(case: &PolicyCase, policy: &PolicyForest) -> Result<usize, String> {
    let mut actions = BTreeMap::new();
    let mut count = 0;
    for root in &policy.roots {
        count += verify_node(case, root, &mut actions)?;
    }
    Ok(count)
}

fn unique_scenario_node<'a>(
    nodes: &'a [DecisionNode],
    scenario_id: &str,
) -> Result<&'a DecisionNode, String> {
    let matches: Vec<&DecisionNode> = nodes
        .iter()
        .filter(|node| node.scenario_ids.iter().any(|id| id == scenario_id))
        .collect();
    match matches.as_slice() {
        [node] => Ok(*node),
        [] => Err(fail("scenario_missing_from_policy_decision_node")),
        _ => Err(fail("scenario_appears_in_multiple_sibling_policy_nodes")),
    }
}

// Independent path replay: deliberately does not call the optimizer's action-cost,
// action-enumeration or completed-state transition helpers.
fn replay_scenario(
    case: &PolicyCase,
    policy: &PolicyForest,
    scenario_index: usize,
) -> Result<u64, String> {
    let scenario = case
        .scenarios
        .get(scenario_index)
        .ok_or_else(|| fail("replay_scenario_index_out_of_range"))?;
    let mut candidates = policy.roots.as_slice();
    let mut completed = BTreeMap::<String, String>::new();
    let mut total_cost = 0_u64;

    for period in 0..case.horizon {
        let node = unique_scenario_node(candidates, &scenario.scenario_id)?;
        if node.period != period {
            return Err(fail("replay_policy_node_period_mismatch"));
        }
        let visible_history = history_prefix(case, scenario_index, period)?;
        if node.observable_history != visible_history {
            return Err(fail("replay_policy_visible_history_mismatch"));
        }
        if node.completed_routes_before != completed {
            return Err(fail("replay_policy_completed_before_mismatch"));
        }

        let observation = visible_history
            .last()
            .ok_or_else(|| fail("replay_history_missing_current_observation"))?;
        let mut assigned_jobs = BTreeSet::new();
        let mut resource_use = BTreeMap::<String, u64>::new();

        for assignment in &node.assignments {
            if !assigned_jobs.insert(assignment.job_id.as_str())
                || completed.contains_key(&assignment.job_id)
            {
                return Err(fail("replay_duplicate_or_already_completed_job"));
            }

            let job = case
                .jobs
                .iter()
                .find(|candidate| candidate.job_id == assignment.job_id)
                .ok_or_else(|| fail("replay_assignment_unknown_job"))?;
            let route = job
                .routes
                .iter()
                .find(|candidate| candidate.resource_id == assignment.resource_id)
                .ok_or_else(|| fail("replay_assignment_route_not_qualified_for_job"))?;
            let resource = case
                .resources
                .iter()
                .find(|candidate| candidate.resource_id == assignment.resource_id)
                .ok_or_else(|| fail("replay_assignment_unknown_resource"))?;

            if observation
                .failed_resource_ids
                .contains(&assignment.resource_id)
                || observation
                    .unknown_resource_ids
                    .contains(&assignment.resource_id)
            {
                return Err(fail("replay_assignment_uses_failed_or_unknown_resource"));
            }

            let used = resource_use
                .entry(assignment.resource_id.clone())
                .or_default();
            *used = used
                .checked_add(1)
                .ok_or_else(|| fail("replay_resource_use_overflow"))?;
            if *used > resource.capacity_units_by_period[period] {
                return Err(fail("replay_time_indexed_resource_capacity_exceeded"));
            }

            let completion_period =
                u64::try_from(period + 1).map_err(|_| fail("replay_period_conversion_overflow"))?;
            let due_period = u64::try_from(job.due_period)
                .map_err(|_| fail("replay_due_period_conversion_overflow"))?;
            let late_periods = completion_period.saturating_sub(due_period);
            let tardy_cost = late_periods
                .checked_mul(job.tardiness_penalty_per_period)
                .ok_or_else(|| fail("replay_tardiness_cost_overflow"))?;
            let route_cost = route
                .production_cost_units
                .checked_add(route.expected_quality_loss_units)
                .and_then(|value| value.checked_add(tardy_cost))
                .ok_or_else(|| fail("replay_route_cost_overflow"))?;
            total_cost = total_cost
                .checked_add(route_cost)
                .ok_or_else(|| fail("replay_total_cost_overflow"))?;
        }

        for assignment in &node.assignments {
            if completed
                .insert(assignment.job_id.clone(), assignment.resource_id.clone())
                .is_some()
            {
                return Err(fail("replay_job_completed_more_than_once"));
            }
        }
        if node.completed_routes_after != completed {
            return Err(fail("replay_completed_after_mismatch"));
        }

        if period + 1 < case.horizon {
            candidates = node.children.as_slice();
        } else if !node.children.is_empty() {
            return Err(fail("replay_terminal_node_has_children"));
        }
    }

    for job in &case.jobs {
        if !completed.contains_key(&job.job_id) {
            total_cost = total_cost
                .checked_add(job.unserved_penalty_units)
                .ok_or_else(|| fail("replay_unserved_cost_overflow"))?;
        }
    }
    Ok(total_cost)
}

fn replay_policy(case: &PolicyCase, policy: &PolicyForest) -> Result<u64, String> {
    let mut weighted_total = 0_u64;
    for (index, scenario) in case.scenarios.iter().enumerate() {
        let leaf_cost = replay_scenario(case, policy, index)?;
        let weighted = leaf_cost
            .checked_mul(scenario.probability_ppm)
            .ok_or_else(|| fail("replay_weighted_cost_overflow"))?;
        weighted_total = weighted_total
            .checked_add(weighted)
            .ok_or_else(|| fail("replay_policy_total_overflow"))?;
    }

    let reported_total = policy.roots.iter().try_fold(0_u64, |total, root| {
        total
            .checked_add(root.expected_cost_numerator)
            .ok_or_else(|| fail("replay_reported_root_cost_overflow"))
    })?;
    if weighted_total != reported_total {
        return Err(fail("independent_replay_cost_does_not_match_policy_report"));
    }
    Ok(weighted_total)
}

fn quality_rejection_numerator(case: &PolicyCase, policy: &PolicyForest) -> Result<u64, String> {
    fn visit(case: &PolicyCase, node: &DecisionNode, total: &mut u64) -> Result<(), String> {
        if node.children.is_empty() {
            for scenario_id in &node.scenario_ids {
                let scenario = case
                    .scenarios
                    .iter()
                    .find(|scenario| scenario.scenario_id == *scenario_id)
                    .ok_or_else(|| fail("policy_node_references_unknown_scenario"))?;
                for pair in &scenario.hidden_quality_rejections {
                    if node.completed_routes_after.get(&pair.job_id) == Some(&pair.resource_id) {
                        *total = total
                            .checked_add(scenario.probability_ppm)
                            .ok_or_else(|| fail("quality_rejection_probability_overflow"))?;
                    }
                }
            }
        } else {
            for child in &node.children {
                visit(case, child, total)?;
            }
        }
        Ok(())
    }

    let mut total = 0_u64;
    for root in &policy.roots {
        visit(case, root, &mut total)?;
    }
    Ok(total)
}

fn policy_node_count(policy: &PolicyForest) -> usize {
    fn count(node: &DecisionNode) -> usize {
        1 + node.children.iter().map(count).sum::<usize>()
    }
    policy.roots.iter().map(count).sum()
}

fn evaluate_case(case: &PolicyCase) -> Result<CaseResult, String> {
    let greedy_policy = build_policy(case, DecisionRule::MyopicImmediateCost)?;
    let scarcity_greedy_policy = build_policy(case, DecisionRule::ScarcityAndDueDate)?;
    let exact_policy = build_policy(case, DecisionRule::Exact)?;

    let greedy_nodes = verify_policy(case, &greedy_policy)?;
    let scarcity_greedy_nodes = verify_policy(case, &scarcity_greedy_policy)?;
    let exact_nodes = verify_policy(case, &exact_policy)?;
    if greedy_nodes != policy_node_count(&greedy_policy)
        || scarcity_greedy_nodes != policy_node_count(&scarcity_greedy_policy)
        || exact_nodes != policy_node_count(&exact_policy)
    {
        return Err(fail("policy_node_count_or_verification_mismatch"));
    }

    let greedy_replayed = replay_policy(case, &greedy_policy)?;
    let scarcity_greedy_replayed = replay_policy(case, &scarcity_greedy_policy)?;
    let exact_replayed = replay_policy(case, &exact_policy)?;

    let greedy_cost = greedy_policy.roots.iter().try_fold(0_u64, |total, node| {
        total
            .checked_add(node.expected_cost_numerator)
            .ok_or_else(|| fail("greedy_root_cost_overflow"))
    })?;
    let scarcity_greedy_cost =
        scarcity_greedy_policy
            .roots
            .iter()
            .try_fold(0_u64, |total, node| {
                total
                    .checked_add(node.expected_cost_numerator)
                    .ok_or_else(|| fail("scarcity_greedy_root_cost_overflow"))
            })?;
    let exact_cost = exact_policy.roots.iter().try_fold(0_u64, |total, node| {
        total
            .checked_add(node.expected_cost_numerator)
            .ok_or_else(|| fail("exact_root_cost_overflow"))
    })?;

    if greedy_replayed != greedy_cost
        || scarcity_greedy_replayed != scarcity_greedy_cost
        || exact_replayed != exact_cost
    {
        return Err(fail("independent_scenario_replay_cost_mismatch"));
    }
    if exact_cost > greedy_cost || exact_cost > scarcity_greedy_cost {
        return Err(fail("exact_policy_cost_exceeds_declared_greedy_baseline"));
    }

    Ok(CaseResult {
        case_id: case.case_id.clone(),
        scenario_family_id: case.scenario_family_id.clone(),
        seed: case.seed,
        scenario_count: case.scenarios.len(),
        decision_node_count: exact_nodes,
        greedy_expected_cost_numerator: greedy_cost,
        scarcity_greedy_expected_cost_numerator: scarcity_greedy_cost,
        exact_expected_cost_numerator: exact_cost,
        myopic_regret_numerator: greedy_cost - exact_cost,
        scarcity_greedy_regret_numerator: scarcity_greedy_cost - exact_cost,
        greedy_quality_rejection_numerator: quality_rejection_numerator(case, &greedy_policy)?,
        scarcity_greedy_quality_rejection_numerator: quality_rejection_numerator(
            case,
            &scarcity_greedy_policy,
        )?,
        exact_quality_rejection_numerator: quality_rejection_numerator(case, &exact_policy)?,
        greedy_policy,
        scarcity_greedy_policy,
        exact_policy,
        non_anticipativity_verified: true,
        hidden_quality_is_evaluator_only: true,
    })
}

fn validate_known_answers(answers: &KnownAnswerFile, cases: &[PolicyCase]) -> Result<(), String> {
    if answers.schema_id
        != "https://luminousdynamics.io/schemas/mfg-multiperiod-policy-known-answers-v0.1"
        || answers.benchmark_id != "MFG-MULTIPERIOD-POLICY-001"
        || answers.evidence_class != "synthetic"
        || answers.answer_count != cases.len()
        || answers.answers.len() != cases.len()
        || answers.claim_ceiling.trim().is_empty()
    {
        return Err(fail("known_answer_manifest_invalid"));
    }
    let expected: HashMap<&str, &KnownAnswer> = answers
        .answers
        .iter()
        .map(|answer| (answer.case_id.as_str(), answer))
        .collect();
    for case in cases {
        let answer = expected
            .get(case.case_id.as_str())
            .ok_or_else(|| fail("known_answer_missing_case"))?;
        let result = evaluate_case(case)?;
        if result.greedy_expected_cost_numerator != answer.greedy_expected_cost_numerator
            || result.scarcity_greedy_expected_cost_numerator
                != answer.scarcity_greedy_expected_cost_numerator
            || result.exact_expected_cost_numerator != answer.exact_expected_cost_numerator
            || result.exact_quality_rejection_numerator != answer.exact_quality_rejection_numerator
        {
            return Err(fail(format!("known_answer_mismatch:{}", case.case_id)));
        }
    }
    Ok(())
}

fn evaluate(corpus: &Corpus) -> Result<Report, String> {
    let mut cases = Vec::new();
    let mut total_greedy = 0_u64;
    let mut total_scarcity_greedy = 0_u64;
    let mut total_exact = 0_u64;
    let mut total_scenarios = 0_usize;
    let mut total_nodes = 0_usize;
    for case in &corpus.cases {
        let result = evaluate_case(case)?;
        total_greedy = total_greedy
            .checked_add(result.greedy_expected_cost_numerator)
            .ok_or_else(|| fail("report_greedy_cost_overflow"))?;
        total_scarcity_greedy = total_scarcity_greedy
            .checked_add(result.scarcity_greedy_expected_cost_numerator)
            .ok_or_else(|| fail("report_scarcity_greedy_cost_overflow"))?;
        total_exact = total_exact
            .checked_add(result.exact_expected_cost_numerator)
            .ok_or_else(|| fail("report_exact_cost_overflow"))?;
        total_scenarios += case.scenarios.len();
        total_nodes += result.decision_node_count;
        cases.push(result);
    }
    Ok(Report {
        benchmark_id: "MFG-MULTIPERIOD-POLICY-001",
        evaluator_version: "mfg-multiperiod-policy-reference-v0.1",
        evidence_class: "synthetic",
        family_count: corpus.family_manifest.len(),
        case_count: corpus.cases.len(),
        scenario_leaf_count: total_scenarios,
        decision_node_count: total_nodes,
        greedy_expected_cost_numerator: total_greedy,
        scarcity_greedy_expected_cost_numerator: total_scarcity_greedy,
        exact_expected_cost_numerator: total_exact,
        myopic_regret_numerator: total_greedy.saturating_sub(total_exact),
        scarcity_greedy_regret_numerator: total_scarcity_greedy.saturating_sub(total_exact),
        exact_policy_non_anticipativity_verified: true,
        recommendations_are_non_binding: true,
        capacity_reservations_created: false,
        machine_commands_issued: false,
        physical_acceptance_claimed: false,
        production_release_authorized: false,
        cases,
        claim_ceiling: "small synthetic multi-job multi-period policy oracle only",
    })
}

fn load_corpus() -> Result<Corpus, String> {
    let schema: serde_json::Value = serde_json::from_str(CORPUS_SCHEMA_JSON)
        .map_err(|error| fail(format!("corpus_schema_parse:{error}")))?;
    let value: serde_json::Value = serde_json::from_str(CORPUS_JSON)
        .map_err(|error| fail(format!("corpus_json_parse:{error}")))?;
    validate_schema(&value, CORPUS_SCHEMA_JSON)?;
    let _ = schema;
    serde_json::from_value(value).map_err(|error| fail(format!("corpus_deserialize:{error}")))
}

fn load_answers() -> Result<KnownAnswerFile, String> {
    let value: serde_json::Value = serde_json::from_str(ANSWERS_JSON)
        .map_err(|error| fail(format!("answers_json_parse:{error}")))?;
    validate_schema(&value, ANSWERS_SCHEMA_JSON)?;
    serde_json::from_value(value).map_err(|error| fail(format!("answers_deserialize:{error}")))
}

fn run() -> Result<(), String> {
    let corpus = load_corpus()?;
    validate_corpus(&corpus)?;
    let answers = load_answers()?;
    validate_known_answers(&answers, &corpus.cases)?;
    let report = evaluate(&corpus)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|error| fail(format!("report_serialize:{error}")))?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> Corpus {
        load_corpus().expect("multi-period corpus parses and schema validates")
    }

    #[test]
    fn strict_schemas_and_independent_known_answers_match_all_twelve_cases() {
        let corpus = corpus();
        validate_corpus(&corpus).unwrap();
        validate_known_answers(&load_answers().unwrap(), &corpus.cases).unwrap();
    }

    #[test]
    fn identical_observable_histories_share_the_same_action_despite_hidden_terminal_outcomes() {
        let corpus = corpus();
        let case = corpus
            .cases
            .iter()
            .find(|case| case.scenario_family_id == "quality-outcome-uncertainty")
            .unwrap();
        let policy = build_policy(case, DecisionRule::Exact).unwrap();
        let relevant = case
            .scenarios
            .iter()
            .filter(|scenario| {
                scenario.scenario_id == "B-AVAILABLE-NO-REJECT"
                    || scenario.scenario_id == "B-AVAILABLE-HIDDEN-REJECT"
            })
            .map(|scenario| scenario.scenario_id.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(relevant.len(), 2);
        let node = policy
            .roots
            .iter()
            .flat_map(|root| root.children.iter())
            .find(|node| {
                relevant
                    .iter()
                    .all(|id| node.scenario_ids.iter().any(|candidate| candidate == id))
            })
            .expect("same visible prefix must remain one decision node");
        assert_eq!(
            node.assignments,
            vec![Assignment {
                job_id: "JOB-FLEX".into(),
                resource_id: "RESOURCE-B".into()
            }]
        );
    }

    #[test]
    fn hidden_terminal_quality_mutations_do_not_change_policy_decisions() {
        let corpus = corpus();
        let original = corpus
            .cases
            .iter()
            .find(|case| case.scenario_family_id == "quality-outcome-uncertainty")
            .unwrap()
            .clone();
        let original_policy = build_policy(&original, DecisionRule::Exact).unwrap();

        let mut mutated = original.clone();
        for scenario in &mut mutated.scenarios {
            scenario.hidden_quality_rejections.clear();
        }
        let mutated_policy = build_policy(&mutated, DecisionRule::Exact).unwrap();
        assert_eq!(original_policy, mutated_policy);
    }

    #[test]
    fn scenario_permutation_does_not_change_optimal_policy_or_cost() {
        let corpus = corpus();
        let original = corpus.cases[0].clone();
        let expected = build_policy(&original, DecisionRule::Exact).unwrap();
        let expected_cost = expected
            .roots
            .iter()
            .map(|node| node.expected_cost_numerator)
            .sum::<u64>();

        let mut permuted = original;
        permuted.scenarios.reverse();
        let actual = build_policy(&permuted, DecisionRule::Exact).unwrap();
        let actual_cost = actual
            .roots
            .iter()
            .map(|node| node.expected_cost_numerator)
            .sum::<u64>();
        assert_eq!(expected, actual);
        assert_eq!(expected_cost, actual_cost);
    }

    #[test]
    fn unknown_resource_is_not_treated_as_admissible_capacity() {
        let corpus = corpus();
        let case = corpus
            .cases
            .iter()
            .find(|case| case.scenario_family_id == "qualification-unknown")
            .unwrap();
        let policy = build_policy(case, DecisionRule::Exact).unwrap();
        let unknown_node = policy
            .roots
            .iter()
            .flat_map(|root| root.children.iter())
            .find(|node| {
                node.observable_history.last().is_some_and(|observation| {
                    observation
                        .unknown_resource_ids
                        .iter()
                        .any(|resource| resource == "RESOURCE-B")
                })
            })
            .expect("unknown qualification branch exists");
        assert!(
            !unknown_node
                .assignments
                .iter()
                .any(|assignment| assignment.resource_id == "RESOURCE-B")
        );
    }

    #[test]
    fn myopic_policy_exposes_the_shared_bottleneck_opportunity_cost() {
        let corpus = corpus();
        let case = &corpus.cases[0];
        let greedy = build_policy(case, DecisionRule::MyopicImmediateCost).unwrap();
        let exact = build_policy(case, DecisionRule::Exact).unwrap();
        assert_eq!(greedy.roots[0].assignments[0].job_id, "JOB-FLEX");
        assert_eq!(exact.roots[0].assignments[0].job_id, "JOB-ONLY-A");
    }

    #[test]
    fn duplicate_observation_entries_are_rejected() {
        let mut observation = Observation {
            failed_resource_ids: vec!["RESOURCE-A".into(), "RESOURCE-A".into()],
            unknown_resource_ids: vec![],
        };
        assert_eq!(
            canonical_observation(&observation).unwrap_err(),
            "observation_contains_duplicate_resource_id"
        );
        observation.failed_resource_ids = vec!["RESOURCE-A".into()];
        observation.unknown_resource_ids = vec!["RESOURCE-A".into()];
        assert_eq!(
            canonical_observation(&observation).unwrap_err(),
            "resource_cannot_be_failed_and_unknown_in_same_observation"
        );
    }

    #[test]
    fn scarcity_and_due_date_baseline_prioritizes_the_only_a_job() {
        let corpus = corpus();
        let case = &corpus.cases[0];
        let policy = build_policy(case, DecisionRule::ScarcityAndDueDate).unwrap();
        assert_eq!(
            policy.roots[0].assignments,
            vec![Assignment {
                job_id: "JOB-ONLY-A".into(),
                resource_id: "RESOURCE-A".into()
            }]
        );
    }

    #[test]
    fn exact_policy_never_costs_more_than_either_greedy_baseline() {
        let corpus = corpus();
        for case in &corpus.cases {
            let greedy = build_policy(case, DecisionRule::MyopicImmediateCost).unwrap();
            let scarcity = build_policy(case, DecisionRule::ScarcityAndDueDate).unwrap();
            let exact = build_policy(case, DecisionRule::Exact).unwrap();
            let greedy_cost: u64 = greedy
                .roots
                .iter()
                .map(|node| node.expected_cost_numerator)
                .sum();
            let scarcity_cost: u64 = scarcity
                .roots
                .iter()
                .map(|node| node.expected_cost_numerator)
                .sum();
            let exact_cost: u64 = exact
                .roots
                .iter()
                .map(|node| node.expected_cost_numerator)
                .sum();
            assert!(exact_cost <= greedy_cost, "{}", case.case_id);
            assert!(exact_cost <= scarcity_cost, "{}", case.case_id);
            verify_policy(case, &exact).unwrap();
        }
    }

    #[test]
    fn independent_scenario_replay_matches_both_policy_objectives_for_all_cases() {
        let corpus = corpus();
        for case in &corpus.cases {
            let greedy = build_policy(case, DecisionRule::MyopicImmediateCost).unwrap();
            let scarcity = build_policy(case, DecisionRule::ScarcityAndDueDate).unwrap();
            let exact = build_policy(case, DecisionRule::Exact).unwrap();
            let greedy_replayed = replay_policy(case, &greedy).unwrap();
            let scarcity_replayed = replay_policy(case, &scarcity).unwrap();
            let exact_replayed = replay_policy(case, &exact).unwrap();
            let greedy_reported: u64 = greedy
                .roots
                .iter()
                .map(|node| node.expected_cost_numerator)
                .sum();
            let scarcity_reported: u64 = scarcity
                .roots
                .iter()
                .map(|node| node.expected_cost_numerator)
                .sum();
            let exact_reported: u64 = exact
                .roots
                .iter()
                .map(|node| node.expected_cost_numerator)
                .sum();
            assert_eq!(greedy_replayed, greedy_reported, "{}", case.case_id);
            assert_eq!(scarcity_replayed, scarcity_reported, "{}", case.case_id);
            assert_eq!(exact_replayed, exact_reported, "{}", case.case_id);
        }
    }

    #[test]
    fn independent_replay_rejects_time_indexed_capacity_overbooking() {
        let corpus = corpus();
        let case = &corpus.cases[0];
        let mut policy = build_policy(case, DecisionRule::Exact).unwrap();
        let root = policy.roots.first_mut().unwrap();
        root.assignments.push(Assignment {
            job_id: "JOB-FLEX".into(),
            resource_id: "RESOURCE-A".into(),
        });
        root.completed_routes_after
            .insert("JOB-FLEX".into(), "RESOURCE-A".into());
        assert_eq!(
            replay_policy(case, &policy).unwrap_err(),
            "replay_time_indexed_resource_capacity_exceeded"
        );
    }

    #[test]
    fn report_never_claims_reservations_execution_or_physical_acceptance() {
        let corpus = corpus();
        let report = evaluate(&corpus).unwrap();
        assert!(!report.capacity_reservations_created);
        assert!(!report.machine_commands_issued);
        assert!(!report.physical_acceptance_claimed);
        assert!(!report.production_release_authorized);
        assert!(report.recommendations_are_non_binding);
    }

    #[test]
    fn corpus_schema_rejects_unknown_root_fields() {
        let mut value: serde_json::Value = serde_json::from_str(CORPUS_JSON).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("unexpected".into(), serde_json::Value::Bool(true));
        assert!(validate_schema(&value, CORPUS_SCHEMA_JSON).is_err());
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
