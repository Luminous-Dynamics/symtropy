use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const EMBEDDED_CORPUS: &str =
    include_str!("../../../fixtures/manufacturing/mfg_route_recourse_cases_v0_1.json");
const EMBEDDED_ANSWERS: &str =
    include_str!("../../../fixtures/manufacturing/mfg_route_recourse_known_answers_v0_1.json");
const UNFULFILLED_PENALTY_PER_UNIT: u64 = 1_000;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    schema_id: String,
    benchmark_id: String,
    evidence_class: String,
    family_manifest: Vec<FamilySpec>,
    case_count: usize,
    cases: Vec<ScenarioCase>,
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
struct ScenarioCase {
    case_id: String,
    scenario_family_id: String,
    seed: u64,
    routes: Vec<Route>,
    jobs: Vec<Job>,
    decision_points: Vec<DecisionPoint>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Route {
    route_id: String,
    owner_disposition: String,
    blockers: Vec<String>,
    common_cause_domains: Vec<String>,
    capacity_units: u64,
    cost_per_unit: u64,
    lead_time_hours: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Job {
    job_id: String,
    quantity_units: u64,
    max_lead_time_hours: u64,
    max_total_cost_units: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Observation {
    observed_hour: u64,
    sequence: u64,
    event_type: String,
    target_id: String,
    #[serde(default)]
    related_id: Option<String>,
    #[serde(default)]
    value_units: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct HiddenFutureEvent {
    event_type: String,
    target_id: String,
    occurs_at_hour: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct DecisionPoint {
    decision_id: String,
    decision_hour: u64,
    observations: Vec<Observation>,
    hidden_future: Vec<HiddenFutureEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Recommendation {
    Freeze {
        pending_execution_ids: Vec<String>,
    },
    Plan {
        assignments: Vec<Assignment>,
        abstentions: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Assignment {
    job_id: String,
    route_id: String,
    quantity_units: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct KnownAnswerFile {
    schema_id: String,
    benchmark_id: String,
    evidence_class: String,
    answer_count: usize,
    answers: Vec<KnownAnswer>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct KnownAnswer {
    case_id: String,
    decision_id: String,
    recommendation: Recommendation,
    oracle_recommendation: Recommendation,
    recommendation_objective_units: Option<u64>,
    oracle_objective_units: Option<u64>,
}

#[derive(Debug, Serialize)]
struct Report {
    benchmark_id: &'static str,
    evaluator_version: &'static str,
    evidence_class: &'static str,
    scenario_family_count: usize,
    scenario_case_count: usize,
    decision_point_count: usize,
    non_anticipativity_information_set_count: usize,
    freeze_decision_count: usize,
    plan_decision_count: usize,
    assignment_count: usize,
    abstention_count: usize,
    capacity_overcommitment_count: usize,
    total_policy_objective_units: u64,
    total_oracle_objective_units: u64,
    total_regret_units: u64,
    decision_results: Vec<DecisionResult>,
    family_summaries: Vec<FamilySummary>,
    recommendations_are_non_binding: bool,
    execution_authority_used: bool,
    physical_acceptance_claimed: bool,
    production_release_authorized: bool,
    claim_ceiling: &'static str,
}

#[derive(Debug, Serialize)]
struct DecisionResult {
    case_id: String,
    scenario_family_id: String,
    seed: u64,
    decision_id: String,
    decision_hour: u64,
    information_set_digest: String,
    recommendation: Recommendation,
    oracle_recommendation: Recommendation,
    recommendation_objective_units: Option<u64>,
    oracle_objective_units: Option<u64>,
    regret_units: Option<u64>,
    capacity_plan_valid: bool,
}

#[derive(Debug, Serialize)]
struct FamilySummary {
    scenario_family_id: String,
    case_count: usize,
    decision_count: usize,
    freeze_decisions: usize,
    assignments: usize,
    abstentions: usize,
    evaluated_plan_decisions: usize,
    mean_policy_objective_units: Option<u64>,
    mean_oracle_objective_units: Option<u64>,
    mean_regret_units: Option<u64>,
}

#[derive(Debug, Clone, Default)]
struct ObservedState {
    failed_routes: BTreeSet<String>,
    failed_domains: BTreeSet<String>,
    unknown_routes: BTreeSet<String>,
    pending_executions: BTreeSet<String>,
    capacity_overrides: BTreeMap<String, u64>,
}

fn fail(message: impl Into<String>) -> String {
    message.into()
}

fn validate(corpus: &Corpus) -> Result<(), String> {
    if corpus.schema_id != "https://luminousdynamics.io/schemas/mfg-route-recourse-case-v0.1" {
        return Err(fail("unsupported_recourse_schema_id"));
    }
    if corpus.benchmark_id != "MFG-ROUTE-RECOURSE-001" || corpus.evidence_class != "synthetic" {
        return Err(fail("benchmark_identity_or_evidence_class_invalid"));
    }
    if corpus.claim_ceiling.trim().is_empty()
        || corpus.family_manifest.len() != 6
        || corpus.case_count != 120
        || corpus.cases.len() != 120
    {
        return Err(fail("manifest_or_claim_ceiling_invalid"));
    }

    let mut family_ids = HashSet::new();
    for family in &corpus.family_manifest {
        if !family_ids.insert(family.scenario_family_id.as_str()) {
            return Err(fail("duplicate_family_id"));
        }
        let seeds: BTreeSet<u64> = family.replication_seeds.iter().copied().collect();
        if family.case_count != 20
            || family.replication_seeds.len() != 20
            || seeds != (1..=20).collect()
        {
            return Err(fail("family_must_have_exactly_twenty_unique_seeds"));
        }
    }

    let mut cases_seen = HashSet::new();
    let mut family_case_counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut family_seeds: BTreeMap<&str, BTreeSet<u64>> = BTreeMap::new();
    for case in &corpus.cases {
        if !cases_seen.insert(case.case_id.as_str()) {
            return Err(fail("duplicate_case_id"));
        }
        if !family_ids.contains(case.scenario_family_id.as_str()) {
            return Err(fail("case_has_unknown_family"));
        }
        if !(1..=20).contains(&case.seed) {
            return Err(fail("case_seed_out_of_range"));
        }
        *family_case_counts.entry(case.scenario_family_id.as_str()).or_default() += 1;
        if !family_seeds
            .entry(case.scenario_family_id.as_str())
            .or_default()
            .insert(case.seed)
        {
            return Err(fail("duplicate_seed_within_family"));
        }
        if case.routes.is_empty() || case.jobs.is_empty() || case.decision_points.len() != 5 {
            return Err(fail("case_requires_routes_jobs_and_five_decision_points"));
        }

        let mut route_ids = HashSet::new();
        let mut domains = HashSet::new();
        for route in &case.routes {
            if route.route_id.trim().is_empty() || !route_ids.insert(route.route_id.as_str()) {
                return Err(fail("route_identity_missing_or_duplicate"));
            }
            if route.owner_disposition != "feasible"
                || !route.blockers.is_empty()
                || route.capacity_units > 1_000_000
                || route.cost_per_unit == 0
                || route.cost_per_unit > 1_000_000
                || route.lead_time_hours == 0
                || route.common_cause_domains.is_empty()
                || route.common_cause_domains.iter().any(|domain| domain.trim().is_empty())
            {
                return Err(fail("route_profile_invalid_or_not_admissible"));
            }
            for domain in &route.common_cause_domains {
                domains.insert(domain.as_str());
            }
        }

        let mut job_ids = HashSet::new();
        for job in &case.jobs {
            if job.job_id.trim().is_empty()
                || !job_ids.insert(job.job_id.as_str())
                || job.quantity_units == 0
                || job.quantity_units > 1_000
                || job.max_lead_time_hours == 0
                || job.max_total_cost_units == 0
            {
                return Err(fail("job_identity_or_constraints_invalid"));
            }
        }

        let mut decision_ids = HashSet::new();
        let mut decision_by_id = HashMap::new();
        for point in &case.decision_points {
            if point.decision_id.trim().is_empty()
                || !decision_ids.insert(point.decision_id.as_str())
                || point.hidden_future.is_empty()
            {
                return Err(fail("decision_identity_or_hidden_future_missing"));
            }
            let mut previous: Option<(u64, u64)> = None;
            let mut observation_ids = HashSet::new();
            for observation in &point.observations {
                if observation.target_id.trim().is_empty()
                    || !observation_ids.insert((observation.observed_hour, observation.sequence))
                    || observation.observed_hour > point.decision_hour
                {
                    return Err(fail("observation_identity_or_time_invalid"));
                }
                let order = (observation.observed_hour, observation.sequence);
                if previous.is_some_and(|value| order < value) {
                    return Err(fail("observations_not_in_event_order"));
                }
                previous = Some(order);
                match observation.event_type.as_str() {
                    "route_failed" | "route_evidence_unknown" | "calibration_stale"
                    | "material_incompatible" => {
                        if !route_ids.contains(observation.target_id.as_str()) {
                            return Err(fail("observation_references_unknown_route"));
                        }
                    }
                    "common_cause_failed" => {
                        if !domains.contains(observation.target_id.as_str()) {
                            return Err(fail("observation_references_unknown_common_cause_domain"));
                        }
                    }
                    "capacity_reduced" => {
                        let route = case.routes.iter().find(|r| r.route_id == observation.target_id);
                        let Some(route) = route else {
                            return Err(fail("capacity_observation_references_unknown_route"));
                        };
                        let Some(value) = observation.value_units else {
                            return Err(fail("capacity_reduction_missing_value"));
                        };
                        if value > route.capacity_units {
                            return Err(fail("capacity_reduction_cannot_increase_declared_capacity"));
                        }
                    }
                    "execution_outcome_unknown" | "execution_reconciled_accepted" => {}
                    "execution_reconciled_failure" => {
                        let Some(route_id) = observation.related_id.as_deref() else {
                            return Err(fail("reconciled_failure_missing_route_identity"));
                        };
                        if !route_ids.contains(route_id) {
                            return Err(fail("reconciled_failure_references_unknown_route"));
                        }
                    }
                    _ => return Err(fail("unsupported_observation_event_type")),
                }
            }
            for hidden in &point.hidden_future {
                if hidden.event_type.trim().is_empty()
                    || hidden.target_id.trim().is_empty()
                    || hidden.occurs_at_hour <= point.decision_hour
                {
                    return Err(fail("hidden_future_event_must_be_after_decision_cutoff"));
                }
            }
            decision_by_id.insert(point.decision_id.as_str(), point);
        }

        let prefix_a = decision_by_id
            .get("initial-prefix-a")
            .ok_or_else(|| fail("paired_initial_prefix_a_missing"))?;
        let prefix_b = decision_by_id
            .get("initial-prefix-b")
            .ok_or_else(|| fail("paired_initial_prefix_b_missing"))?;
        if prefix_a.decision_hour != prefix_b.decision_hour
            || prefix_a.observations != prefix_b.observations
            || prefix_a.hidden_future == prefix_b.hidden_future
        {
            return Err(fail("paired_prefix_must_match_while_hidden_future_differs"));
        }
    }

    for family in &corpus.family_manifest {
        if family_case_counts.get(family.scenario_family_id.as_str()) != Some(&20)
            || family_seeds.get(family.scenario_family_id.as_str()) != Some(&(1..=20).collect())
        {
            return Err(fail("family_case_count_or_seed_manifest_mismatch"));
        }
    }
    Ok(())
}

fn observed_state(point: &DecisionPoint) -> Result<ObservedState, String> {
    let mut state = ObservedState::default();
    for event in &point.observations {
        match event.event_type.as_str() {
            "route_failed" => {
                state.failed_routes.insert(event.target_id.clone());
            }
            "common_cause_failed" => {
                state.failed_domains.insert(event.target_id.clone());
            }
            "capacity_reduced" => {
                let value = event.value_units.ok_or_else(|| fail("capacity_reduction_missing_value"))?;
                let current = state.capacity_overrides.get(&event.target_id).copied();
                if current.is_some_and(|previous| value > previous) {
                    return Err(fail("capacity_reduction_event_cannot_restore_capacity"));
                }
                state.capacity_overrides.insert(event.target_id.clone(), value);
            }
            "route_evidence_unknown" | "calibration_stale" => {
                state.unknown_routes.insert(event.target_id.clone());
            }
            "material_incompatible" => {
                state.failed_routes.insert(event.target_id.clone());
            }
            "execution_outcome_unknown" => {
                state.pending_executions.insert(event.target_id.clone());
            }
            "execution_reconciled_failure" | "execution_reconciled_accepted" => {
                if !state.pending_executions.remove(&event.target_id) {
                    return Err(fail("execution_reconciliation_without_prior_unknown"));
                }
                if event.event_type == "execution_reconciled_failure" {
                    let route_id = event.related_id.as_deref()
                        .ok_or_else(|| fail("reconciled_failure_missing_route_identity"))?;
                    state.failed_routes.insert(route_id.to_owned());
                }
            }
            _ => return Err(fail("unsupported_observation_event_type")),
        }
    }
    Ok(state)
}

fn effective_capacity(route: &Route, state: &ObservedState) -> u64 {
    state
        .capacity_overrides
        .get(&route.route_id)
        .copied()
        .unwrap_or(route.capacity_units)
}

fn route_is_available(route: &Route, state: &ObservedState) -> bool {
    route.owner_disposition == "feasible"
        && route.blockers.is_empty()
        && !state.failed_routes.contains(&route.route_id)
        && !state.unknown_routes.contains(&route.route_id)
        && !route
            .common_cause_domains
            .iter()
            .any(|domain| state.failed_domains.contains(domain))
}

fn route_fits_job(
    route: &Route,
    job: &Job,
    state: &ObservedState,
    remaining_capacity: u64,
) -> bool {
    route_is_available(route, state)
        && remaining_capacity >= job.quantity_units
        && route.lead_time_hours <= job.max_lead_time_hours
        && route.cost_per_unit.saturating_mul(job.quantity_units) <= job.max_total_cost_units
}

fn sorted_plan(
    mut assignments: Vec<Assignment>,
    mut abstentions: Vec<String>,
) -> Recommendation {
    assignments.sort_by(|a, b| a.job_id.cmp(&b.job_id));
    abstentions.sort();
    Recommendation::Plan {
        assignments,
        abstentions,
    }
}

fn greedy_recommendation(
    case: &ScenarioCase,
    point: &DecisionPoint,
) -> Result<Recommendation, String> {
    let state = observed_state(point)?;
    if !state.pending_executions.is_empty() {
        return Ok(Recommendation::Freeze {
            pending_execution_ids: state.pending_executions.into_iter().collect(),
        });
    }

    let mut jobs = case.jobs.clone();
    jobs.sort_by(|a, b| a.job_id.cmp(&b.job_id));
    let mut remaining: BTreeMap<String, u64> = case
        .routes
        .iter()
        .map(|route| (route.route_id.clone(), effective_capacity(route, &state)))
        .collect();
    let mut assignments = Vec::new();
    let mut abstentions = Vec::new();

    for job in &jobs {
        let mut candidates: Vec<&Route> = case
            .routes
            .iter()
            .filter(|route| {
                route_fits_job(
                    route,
                    job,
                    &state,
                    remaining.get(&route.route_id).copied().unwrap_or(0),
                )
            })
            .collect();
        candidates.sort_by(|a, b| {
            a.cost_per_unit
                .cmp(&b.cost_per_unit)
                .then(a.lead_time_hours.cmp(&b.lead_time_hours))
                .then(a.route_id.cmp(&b.route_id))
        });
        if let Some(route) = candidates.first() {
            let available = remaining.get_mut(&route.route_id)
                .ok_or_else(|| fail("selected_route_missing_capacity_state"))?;
            *available -= job.quantity_units;
            assignments.push(Assignment {
                job_id: job.job_id.clone(),
                route_id: route.route_id.clone(),
                quantity_units: job.quantity_units,
            });
        } else {
            abstentions.push(job.job_id.clone());
        }
    }
    Ok(sorted_plan(assignments, abstentions))
}

fn exhaustive_visit(
    case: &ScenarioCase,
    jobs: &[Job],
    index: usize,
    state: &ObservedState,
    remaining: &mut BTreeMap<String, u64>,
    assignments: &mut Vec<Assignment>,
    abstentions: &mut Vec<String>,
    cost: u64,
    best: &mut Option<(u64, String, Recommendation)>,
) -> Result<(), String> {
    if index == jobs.len() {
        let action = sorted_plan(assignments.clone(), abstentions.clone());
        let key = serde_json::to_string(&action)
            .map_err(|error| format!("oracle_action_serialization_error:{error}"))?;
        let replace = best
            .as_ref()
            .is_none_or(|(best_cost, best_key, _)| cost < *best_cost || (cost == *best_cost && key < *best_key));
        if replace {
            *best = Some((cost, key, action));
        }
        return Ok(());
    }

    let job = &jobs[index];
    let mut routes: Vec<&Route> = case
        .routes
        .iter()
        .filter(|route| {
            route_fits_job(
                route,
                job,
                state,
                remaining.get(&route.route_id).copied().unwrap_or(0),
            )
        })
        .collect();
    routes.sort_by(|a, b| a.route_id.cmp(&b.route_id));

    for route in routes {
        let available = remaining
            .get_mut(&route.route_id)
            .ok_or_else(|| fail("oracle_route_missing_capacity_state"))?;
        *available -= job.quantity_units;
        assignments.push(Assignment {
            job_id: job.job_id.clone(),
            route_id: route.route_id.clone(),
            quantity_units: job.quantity_units,
        });
        let route_cost = route.cost_per_unit
            .checked_mul(job.quantity_units)
            .ok_or_else(|| fail("oracle_route_cost_overflow"))?;
        exhaustive_visit(
            case,
            jobs,
            index + 1,
            state,
            remaining,
            assignments,
            abstentions,
            cost.checked_add(route_cost).ok_or_else(|| fail("oracle_total_cost_overflow"))?,
            best,
        )?;
        assignments.pop();
        *remaining.get_mut(&route.route_id)
            .ok_or_else(|| fail("oracle_route_capacity_lost"))? += job.quantity_units;
    }

    abstentions.push(job.job_id.clone());
    let penalty = UNFULFILLED_PENALTY_PER_UNIT
        .checked_mul(job.quantity_units)
        .ok_or_else(|| fail("unserved_penalty_overflow"))?;
    exhaustive_visit(
        case,
        jobs,
        index + 1,
        state,
        remaining,
        assignments,
        abstentions,
        cost.checked_add(penalty).ok_or_else(|| fail("oracle_total_cost_overflow"))?,
        best,
    )?;
    abstentions.pop();
    Ok(())
}

fn exhaustive_oracle(
    case: &ScenarioCase,
    point: &DecisionPoint,
) -> Result<Recommendation, String> {
    let state = observed_state(point)?;
    if !state.pending_executions.is_empty() {
        return Ok(Recommendation::Freeze {
            pending_execution_ids: state.pending_executions.into_iter().collect(),
        });
    }
    let mut jobs = case.jobs.clone();
    jobs.sort_by(|a, b| a.job_id.cmp(&b.job_id));
    let mut remaining: BTreeMap<String, u64> = case
        .routes
        .iter()
        .map(|route| (route.route_id.clone(), effective_capacity(route, &state)))
        .collect();
    let mut best = None;
    exhaustive_visit(
        case,
        &jobs,
        0,
        &state,
        &mut remaining,
        &mut Vec::new(),
        &mut Vec::new(),
        0,
        &mut best,
    )?;
    best.map(|(_, _, action)| action)
        .ok_or_else(|| fail("exhaustive_oracle_produced_no_plan"))
}

fn objective_units(case: &ScenarioCase, action: &Recommendation) -> Result<Option<u64>, String> {
    let Recommendation::Plan {
        assignments,
        abstentions,
    } = action
    else {
        return Ok(None);
    };
    let routes: HashMap<&str, &Route> = case
        .routes
        .iter()
        .map(|route| (route.route_id.as_str(), route))
        .collect();
    let jobs: HashMap<&str, &Job> = case
        .jobs
        .iter()
        .map(|job| (job.job_id.as_str(), job))
        .collect();
    let mut total = 0_u64;
    for assignment in assignments {
        let route = routes.get(assignment.route_id.as_str())
            .ok_or_else(|| fail("objective_references_unknown_route"))?;
        total = total
            .checked_add(route.cost_per_unit.checked_mul(assignment.quantity_units)
                .ok_or_else(|| fail("objective_route_cost_overflow"))?)
            .ok_or_else(|| fail("objective_total_cost_overflow"))?;
    }
    for job_id in abstentions {
        let job = jobs.get(job_id.as_str())
            .ok_or_else(|| fail("objective_references_unknown_job"))?;
        total = total
            .checked_add(UNFULFILLED_PENALTY_PER_UNIT.checked_mul(job.quantity_units)
                .ok_or_else(|| fail("objective_unserved_cost_overflow"))?)
            .ok_or_else(|| fail("objective_total_cost_overflow"))?;
    }
    Ok(Some(total))
}

fn validate_recommendation(
    case: &ScenarioCase,
    point: &DecisionPoint,
    action: &Recommendation,
) -> Result<(), String> {
    let state = observed_state(point)?;
    if !state.pending_executions.is_empty() {
        return match action {
            Recommendation::Freeze { pending_execution_ids }
                if pending_execution_ids == &state.pending_executions.iter().cloned().collect::<Vec<_>>() =>
            {
                Ok(())
            }
            _ => Err(fail("unresolved_execution_outcome_requires_freeze")),
        };
    }

    let Recommendation::Plan {
        assignments,
        abstentions,
    } = action
    else {
        return Err(fail("freeze_without_pending_execution_not_permitted"));
    };

    let jobs: HashMap<&str, &Job> = case.jobs.iter().map(|job| (job.job_id.as_str(), job)).collect();
    let mut seen_jobs = HashSet::new();
    let mut capacity_used: BTreeMap<&str, u64> = BTreeMap::new();
    for assignment in assignments {
        let job = jobs.get(assignment.job_id.as_str())
            .ok_or_else(|| fail("assignment_references_unknown_job"))?;
        if !seen_jobs.insert(assignment.job_id.as_str())
            || assignment.quantity_units != job.quantity_units
        {
            return Err(fail("duplicate_job_assignment_or_quantity_mismatch"));
        }
        let route = case.routes.iter()
            .find(|route| route.route_id == assignment.route_id)
            .ok_or_else(|| fail("assignment_references_unknown_route"))?;
        if !route_fits_job(route, job, &state, effective_capacity(route, &state)) {
            return Err(fail("assignment_uses_inadmissible_route"));
        }
        let used = capacity_used.entry(route.route_id.as_str()).or_default();
        *used = used.checked_add(assignment.quantity_units)
            .ok_or_else(|| fail("capacity_usage_overflow"))?;
        if *used > effective_capacity(route, &state) {
            return Err(fail("capacity_overcommitment_detected"));
        }
    }
    for job_id in abstentions {
        if !jobs.contains_key(job_id.as_str()) || !seen_jobs.insert(job_id.as_str()) {
            return Err(fail("unknown_or_duplicate_abstention_job"));
        }
    }
    if seen_jobs.len() != jobs.len() {
        return Err(fail("plan_must_assign_or_abstain_every_job_exactly_once"));
    }
    Ok(())
}

fn visible_information_key(
    case: &ScenarioCase,
    point: &DecisionPoint,
) -> Result<String, String> {
    let mut routes = case.routes.clone();
    for route in &mut routes {
        route.common_cause_domains.sort();
    }
    routes.sort_by(|a, b| a.route_id.cmp(&b.route_id));
    let mut jobs = case.jobs.clone();
    jobs.sort_by(|a, b| a.job_id.cmp(&b.job_id));
    let mut observations = point.observations.clone();
    observations.sort_by(|a, b| {
        a.observed_hour.cmp(&b.observed_hour)
            .then(a.sequence.cmp(&b.sequence))
            .then(a.event_type.cmp(&b.event_type))
            .then(a.target_id.cmp(&b.target_id))
            .then(a.related_id.cmp(&b.related_id))
            .then(a.value_units.cmp(&b.value_units))
    });
    serde_json::to_string(&(point.decision_hour, routes, jobs, observations))
        .map_err(|error| format!("information_key_serialization_error:{error}"))
}

fn check_non_anticipativity(
    records: &[(String, Recommendation, Recommendation)],
) -> Result<usize, String> {
    let mut seen: HashMap<&str, (&Recommendation, &Recommendation)> = HashMap::new();
    for (key, recommendation, oracle) in records {
        if let Some((previous, previous_oracle)) = seen.get(key.as_str()) {
            if *previous != recommendation || *previous_oracle != oracle {
                return Err(fail("non_anticipativity_violation_same_visible_information_different_action"));
            }
        } else {
            seen.insert(key, (recommendation, oracle));
        }
    }
    Ok(seen.len())
}

fn validate_known_answers(corpus: &Corpus) -> Result<(), String> {
    let answers: KnownAnswerFile = serde_json::from_str(EMBEDDED_ANSWERS)
        .map_err(|error| format!("known_answer_parse_error:{error}"))?;
    if answers.schema_id != "https://luminousdynamics.io/schemas/mfg-route-recourse-known-answers-v0.1"
        || answers.benchmark_id != corpus.benchmark_id
        || answers.evidence_class != "synthetic"
        || answers.answer_count != 600
        || answers.answers.len() != 600
    {
        return Err(fail("known_answer_manifest_mismatch"));
    }
    let mut expected: HashMap<(&str, &str), &KnownAnswer> = HashMap::new();
    for answer in &answers.answers {
        if expected.insert((&answer.case_id, &answer.decision_id), answer).is_some() {
            return Err(fail("duplicate_known_answer_key"));
        }
    }
    for case in &corpus.cases {
        for point in &case.decision_points {
            let answer = expected.get(&(case.case_id.as_str(), point.decision_id.as_str()))
                .ok_or_else(|| fail("known_answer_for_decision_missing"))?;
            let recommendation = greedy_recommendation(case, point)?;
            let oracle = exhaustive_oracle(case, point)?;
            let policy_cost = objective_units(case, &recommendation)?;
            let oracle_cost = objective_units(case, &oracle)?;
            if recommendation != answer.recommendation
                || oracle != answer.oracle_recommendation
                || policy_cost != answer.recommendation_objective_units
                || oracle_cost != answer.oracle_objective_units
            {
                return Err(fail("recourse_known_answer_mismatch"));
            }
        }
    }
    Ok(())
}

fn evaluate(corpus: &Corpus) -> Result<Report, String> {
    validate(corpus)?;
    let mut decision_results = Vec::new();
    let mut information_records = Vec::new();
    let mut family_summaries = Vec::new();
    let mut freeze_count = 0;
    let mut plan_count = 0;
    let mut assignment_count = 0;
    let mut abstention_count = 0;
    let mut policy_total = 0_u64;
    let mut oracle_total = 0_u64;
    let mut regret_total = 0_u64;

    for family in &corpus.family_manifest {
        let mut family_freezes = 0;
        let mut family_assignments = 0;
        let mut family_abstentions = 0;
        let mut family_plan_count = 0;
        let mut family_policy_total = 0_u64;
        let mut family_oracle_total = 0_u64;
        let mut family_regret_total = 0_u64;
        let mut family_decisions = 0;

        for case in corpus.cases.iter().filter(|case| case.scenario_family_id == family.scenario_family_id) {
            for point in &case.decision_points {
                let recommendation = greedy_recommendation(case, point)?;
                let oracle = exhaustive_oracle(case, point)?;
                validate_recommendation(case, point, &recommendation)?;
                validate_recommendation(case, point, &oracle)?;
                let policy_cost = objective_units(case, &recommendation)?;
                let oracle_cost = objective_units(case, &oracle)?;
                let regret = policy_cost.zip(oracle_cost).map(|(p, o)| p.saturating_sub(o));
                let info_key = visible_information_key(case, point)?;
                let digest = format!("{:016x}", stable_hash(info_key.as_bytes()));
                information_records.push((info_key, recommendation.clone(), oracle.clone()));

                match &recommendation {
                    Recommendation::Freeze { .. } => {
                        freeze_count += 1;
                        family_freezes += 1;
                    }
                    Recommendation::Plan { assignments, abstentions } => {
                        plan_count += 1;
                        family_plan_count += 1;
                        assignment_count += assignments.len();
                        abstention_count += abstentions.len();
                        family_assignments += assignments.len();
                        family_abstentions += abstentions.len();
                    }
                }
                if let Some(value) = policy_cost {
                    policy_total += value;
                    family_policy_total += value;
                }
                if let Some(value) = oracle_cost {
                    oracle_total += value;
                    family_oracle_total += value;
                }
                if let Some(value) = regret {
                    regret_total += value;
                    family_regret_total += value;
                }
                family_decisions += 1;
                decision_results.push(DecisionResult {
                    case_id: case.case_id.clone(),
                    scenario_family_id: case.scenario_family_id.clone(),
                    seed: case.seed,
                    decision_id: point.decision_id.clone(),
                    decision_hour: point.decision_hour,
                    information_set_digest: digest,
                    recommendation,
                    oracle_recommendation: oracle,
                    recommendation_objective_units: policy_cost,
                    oracle_objective_units: oracle_cost,
                    regret_units: regret,
                    capacity_plan_valid: true,
                });
            }
        }

        family_summaries.push(FamilySummary {
            scenario_family_id: family.scenario_family_id.clone(),
            case_count: 20,
            decision_count: family_decisions,
            freeze_decisions: family_freezes,
            assignments: family_assignments,
            abstentions: family_abstentions,
            evaluated_plan_decisions: family_plan_count,
            mean_policy_objective_units: if family_plan_count > 0 {
                Some(family_policy_total / family_plan_count as u64)
            } else {
                None
            },
            mean_oracle_objective_units: if family_plan_count > 0 {
                Some(family_oracle_total / family_plan_count as u64)
            } else {
                None
            },
            mean_regret_units: if family_plan_count > 0 {
                Some(family_regret_total / family_plan_count as u64)
            } else {
                None
            },
        });
    }

    let information_set_count = check_non_anticipativity(&information_records)?;
    let capacity_overcommitment_count = decision_results
        .iter()
        .filter(|result| !result.capacity_plan_valid)
        .count();
    if capacity_overcommitment_count != 0 {
        return Err(fail("capacity_overcommitment_in_report"));
    }

    Ok(Report {
        benchmark_id: "MFG-ROUTE-RECOURSE-001",
        evaluator_version: "0.1.0",
        evidence_class: "synthetic",
        scenario_family_count: corpus.family_manifest.len(),
        scenario_case_count: corpus.cases.len(),
        decision_point_count: decision_results.len(),
        non_anticipativity_information_set_count: information_set_count,
        freeze_decision_count: freeze_count,
        plan_decision_count: plan_count,
        assignment_count,
        abstention_count,
        capacity_overcommitment_count,
        total_policy_objective_units: policy_total,
        total_oracle_objective_units: oracle_total,
        total_regret_units: regret_total,
        decision_results,
        family_summaries,
        recommendations_are_non_binding: true,
        execution_authority_used: false,
        physical_acceptance_claimed: false,
        production_release_authorized: false,
        claim_ceiling: "Synthetic non-anticipative policy and capacity-plan semantics only; no booking, dispatch, physical recovery, production output, product acceptance or supplier qualification.",
    })
}

fn stable_hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let content = match args.as_slice() {
        [] => EMBEDDED_CORPUS.to_owned(),
        [path] => fs::read_to_string(path)
            .map_err(|error| format!("corpus_read_error:{error}"))?,
        _ => return Err(fail("usage: mfg-route-recourse-benchmark [corpus.json]")),
    };
    let corpus: Corpus = serde_json::from_str(&content)
        .map_err(|error| format!("corpus_parse_error:{error}"))?;
    validate(&corpus)?;
    validate_known_answers(&corpus)?;
    let report = evaluate(&corpus)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|error| format!("report_serialization_error:{error}"))?
    );
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> Corpus {
        serde_json::from_str(EMBEDDED_CORPUS).expect("frozen recourse corpus parses")
    }

    fn point<'a>(case: &'a ScenarioCase, id: &str) -> &'a DecisionPoint {
        case.decision_points
            .iter()
            .find(|point| point.decision_id == id)
            .expect("decision point exists")
    }

    #[test]
    fn frozen_corpus_and_cross_language_known_answers_match() {
        let input = corpus();
        assert_eq!(input.cases.len(), 120);
        validate_known_answers(&input).expect("600 independently generated answers match");
    }

    #[test]
    fn same_observation_prefix_ignores_divergent_hidden_future() {
        let input = corpus();
        let case = &input.cases[0];
        let a = point(case, "initial-prefix-a");
        let b = point(case, "initial-prefix-b");
        assert_ne!(a.hidden_future, b.hidden_future);
        assert_eq!(visible_information_key(case, a).unwrap(), visible_information_key(case, b).unwrap());
        assert_eq!(greedy_recommendation(case, a).unwrap(), greedy_recommendation(case, b).unwrap());
    }

    #[test]
    fn observed_common_cause_failure_invalidates_every_dependent_route() {
        let input = corpus();
        let case = input.cases.iter()
            .find(|case| case.scenario_family_id == "common-cause-failure")
            .unwrap();
        let after = point(case, "after-observed-event");
        let action = greedy_recommendation(case, after).unwrap();
        let Recommendation::Plan { assignments, .. } = action else {
            panic!("known common-cause event should not freeze");
        };
        assert!(assignments.iter().all(|assignment| {
            assignment.route_id != "HUB-FDM-A" && assignment.route_id != "REGIONAL-FDM-B"
        }));
    }

    #[test]
    fn unknown_execution_outcome_freezes_every_new_recommendation() {
        let input = corpus();
        let case = &input.cases[0];
        let action = greedy_recommendation(case, point(case, "unknown-execution")).unwrap();
        assert!(matches!(action, Recommendation::Freeze { .. }));
        validate_recommendation(case, point(case, "unknown-execution"), &action).unwrap();
    }

    #[test]
    fn reconciliation_releases_freeze_but_failed_route_stays_blocked() {
        let input = corpus();
        let case = &input.cases[0];
        let action = greedy_recommendation(case, point(case, "reconciled-failure")).unwrap();
        let Recommendation::Plan { assignments, .. } = action else {
            panic!("reconciled failure should permit a new recommendation");
        };
        assert!(assignments.iter().all(|assignment| assignment.route_id != "HUB-FDM-A"));
    }

    #[test]
    fn simultaneous_assignments_never_double_count_route_capacity() {
        let input = corpus();
        let case = &input.cases.iter()
            .find(|case| case.scenario_family_id == "capacity-contraction")
            .unwrap();
        let action = greedy_recommendation(case, point(case, "after-observed-event")).unwrap();
        validate_recommendation(case, point(case, "after-observed-event"), &action).unwrap();
        let Recommendation::Plan { assignments, .. } = action else {
            panic!("capacity-contraction is a plan, not an unknown execution");
        };
        let cnc_units: u64 = assignments.iter()
            .filter(|assignment| assignment.route_id == "JOB-SHOP-CNC-C")
            .map(|assignment| assignment.quantity_units)
            .sum();
        assert!(cnc_units <= 2);
    }

    #[test]
    fn exhaustive_assignment_oracle_beats_flexible_first_greedy_case() {
        let input = corpus();
        let case = &input.cases[0];
        let decision = point(case, "initial-prefix-a");
        let greedy = greedy_recommendation(case, decision).unwrap();
        let oracle = exhaustive_oracle(case, decision).unwrap();
        let greedy_cost = objective_units(case, &greedy).unwrap().unwrap();
        let oracle_cost = objective_units(case, &oracle).unwrap().unwrap();
        assert!(oracle_cost < greedy_cost);
        validate_recommendation(case, decision, &oracle).unwrap();
    }

    #[test]
    fn zero_capacity_results_in_explicit_abstention_not_a_fabricated_route() {
        let input = corpus();
        let case = input.cases.iter()
            .find(|case| case.scenario_family_id == "demand-surge-no-capacity")
            .unwrap();
        let action = greedy_recommendation(case, point(case, "after-observed-event")).unwrap();
        let Recommendation::Plan { assignments, abstentions } = action else {
            panic!("capacity exhaustion should abstain, not freeze");
        };
        assert!(assignments.is_empty());
        assert_eq!(abstentions.len(), case.jobs.len());
    }

    #[test]
    fn adversarial_overcommitment_is_rejected() {
        let input = corpus();
        let case = &input.cases[0];
        let decision = point(case, "initial-prefix-a");
        let invalid = Recommendation::Plan {
            assignments: vec![
                Assignment {
                    job_id: "job-01-flexible".into(),
                    route_id: "HUB-FDM-A".into(),
                    quantity_units: 2,
                },
                Assignment {
                    job_id: "job-03-flexible".into(),
                    route_id: "HUB-FDM-A".into(),
                    quantity_units: 2,
                },
                Assignment {
                    job_id: "job-02-constrained".into(),
                    route_id: "REGIONAL-FDM-B".into(),
                    quantity_units: 2,
                },
            ],
            abstentions: vec![],
        };
        assert!(validate_recommendation(case, decision, &invalid)
            .unwrap_err()
            .contains("capacity_overcommitment_detected"));
    }

    #[test]
    fn non_anticipativity_checker_rejects_same_information_different_actions() {
        let input = corpus();
        let case = &input.cases[0];
        let a = point(case, "initial-prefix-a");
        let b = point(case, "initial-prefix-b");
        let key = visible_information_key(case, a).unwrap();
        let first = greedy_recommendation(case, a).unwrap();
        let mut altered = greedy_recommendation(case, b).unwrap();
        if let Recommendation::Plan { abstentions, .. } = &mut altered {
            abstentions.clear();
        }
        let records = vec![(key.clone(), first.clone(), first), (key, altered.clone(), altered)];
        assert!(check_non_anticipativity(&records)
            .unwrap_err()
            .contains("non_anticipativity_violation"));
    }

    #[test]
    fn reconciliation_without_prior_unknown_is_rejected() {
        let mut input = corpus();
        let case = &mut input.cases[0];
        let point = case.decision_points.iter_mut()
            .find(|point| point.decision_id == "reconciled-failure")
            .unwrap();
        point.observations.remove(0);
        assert!(observed_state(point)
            .unwrap_err()
            .contains("execution_reconciliation_without_prior_unknown"));
    }
}
