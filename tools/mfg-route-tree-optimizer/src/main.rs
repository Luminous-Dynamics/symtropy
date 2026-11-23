use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const CORPUS_JSON: &str =
    include_str!("../../../fixtures/manufacturing/mfg_route_tree_cases_v0_1.json");
const ANSWERS_JSON: &str =
    include_str!("../../../fixtures/manufacturing/mfg_route_tree_known_answers_v0_1.json");
const CORPUS_SCHEMA_JSON: &str =
    include_str!("../../../schemas/manufacturing/mfg_route_tree_case_v0_1.schema.json");
const ANSWERS_SCHEMA_JSON: &str =
    include_str!("../../../schemas/manufacturing/mfg_route_tree_known_answers_v0_1.schema.json");
const PROBABILITY_DENOMINATOR: u64 = 1_000_000;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    schema_id: String,
    benchmark_id: String,
    evidence_class: String,
    family_manifest: Vec<FamilySpec>,
    case_count: usize,
    cases: Vec<TreeCase>,
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
struct TreeCase {
    case_id: String,
    scenario_family_id: String,
    seed: u64,
    job: Job,
    routes: Vec<Route>,
    switch_penalty_per_unit: u64,
    unserved_penalty_per_unit: u64,
    quality_rejection_penalty_per_unit: u64,
    scenarios: Vec<Scenario>,
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
struct Route {
    route_id: String,
    capacity_units: u64,
    cost_units: u64,
    lead_time_hours: u64,
    common_cause_domains: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Observations {
    failed_route_ids: Vec<String>,
    failed_domains: Vec<String>,
    unknown_route_ids: Vec<String>,
    capacity_reductions: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Scenario {
    scenario_id: String,
    probability_ppm: u64,
    information_set_id: String,
    observations: Observations,
    hidden_quality_rejected_route_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct PolicyPlan {
    primary_route_id: String,
    recourse_by_information_set: BTreeMap<String, Option<String>>,
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
    greedy_plan: PolicyPlan,
    exact_policy_plan: PolicyPlan,
    greedy_expected_cost_numerator: u64,
    exact_expected_cost_numerator: u64,
    clairvoyant_lower_bound_numerator: u64,
}

#[derive(Debug, Serialize)]
struct Report {
    benchmark_id: &'static str,
    evaluator_version: &'static str,
    evidence_class: &'static str,
    family_count: usize,
    case_count: usize,
    scenario_leaf_count: usize,
    information_set_count: usize,
    greedy_expected_cost_numerator: u64,
    exact_expected_cost_numerator: u64,
    clairvoyant_lower_bound_numerator: u64,
    greedy_regret_numerator: u64,
    cases: Vec<CaseResult>,
    non_anticipativity_verified: bool,
    recommendations_are_non_binding: bool,
    execution_authority_used: bool,
    physical_acceptance_claimed: bool,
    production_release_authorized: bool,
    claim_ceiling: &'static str,
}

#[derive(Debug, Serialize)]
struct CaseResult {
    case_id: String,
    scenario_family_id: String,
    seed: u64,
    scenario_count: usize,
    information_set_count: usize,
    greedy_plan: PolicyPlan,
    exact_policy_plan: PolicyPlan,
    greedy_expected_cost_numerator: u64,
    exact_expected_cost_numerator: u64,
    clairvoyant_lower_bound_numerator: u64,
    regret_numerator: u64,
    exact_policy_is_non_anticipative: bool,
    clairvoyant_is_diagnostic_only: bool,
}

fn fail(message: impl Into<String>) -> String {
    message.into()
}

fn validate_json_schema(instance_text: &str, schema_text: &str) -> Result<(), String> {
    let instance: serde_json::Value = serde_json::from_str(instance_text)
        .map_err(|error| format!("schema_instance_parse_error:{error}"))?;
    let schema: serde_json::Value = serde_json::from_str(schema_text)
        .map_err(|error| format!("schema_document_parse_error:{error}"))?;
    let validator = jsonschema::draft202012::options()
        .offline()
        .build(&schema)
        .map_err(|error| format!("draft_2020_12_schema_compile_error:{error}"))?;
    validator
        .validate(&instance)
        .map_err(|error| format!("draft_2020_12_instance_validation_error:{error}"))
}

fn no_observations() -> Observations {
    Observations {
        failed_route_ids: Vec::new(),
        failed_domains: Vec::new(),
        unknown_route_ids: Vec::new(),
        capacity_reductions: BTreeMap::new(),
    }
}

fn route_available(route: &Route, job: &Job, observations: &Observations) -> bool {
    let capacity = observations
        .capacity_reductions
        .get(&route.route_id)
        .copied()
        .unwrap_or(route.capacity_units);
    capacity >= job.quantity_units
        && !observations.failed_route_ids.contains(&route.route_id)
        && !observations.unknown_route_ids.contains(&route.route_id)
        && !route
            .common_cause_domains
            .iter()
            .any(|domain| observations.failed_domains.contains(domain))
        && route.lead_time_hours <= job.max_lead_time_hours
        && route.cost_units.saturating_mul(job.quantity_units) <= job.max_total_cost_units
}

fn available_initial_routes(case: &TreeCase) -> Vec<&Route> {
    let none = no_observations();
    let mut routes: Vec<&Route> = case
        .routes
        .iter()
        .filter(|route| route_available(route, &case.job, &none))
        .collect();
    routes.sort_by(|a, b| {
        a.cost_units
            .cmp(&b.cost_units)
            .then(a.lead_time_hours.cmp(&b.lead_time_hours))
            .then(a.route_id.cmp(&b.route_id))
    });
    routes
}

fn information_groups(case: &TreeCase) -> BTreeMap<String, Vec<&Scenario>> {
    let mut groups: BTreeMap<String, Vec<&Scenario>> = BTreeMap::new();
    for scenario in &case.scenarios {
        groups
            .entry(scenario.information_set_id.clone())
            .or_default()
            .push(scenario);
    }
    groups
}

fn available_recourse_routes<'a>(
    case: &'a TreeCase,
    observations: &Observations,
) -> Vec<&'a Route> {
    let mut routes: Vec<&Route> = case
        .routes
        .iter()
        .filter(|route| route_available(route, &case.job, observations))
        .collect();
    routes.sort_by(|a, b| a.route_id.cmp(&b.route_id));
    routes
}

fn scenario_cost_units(
    case: &TreeCase,
    primary_route_id: &str,
    recourse_route_id: Option<&str>,
    scenario: &Scenario,
) -> Result<u64, String> {
    let quantity = case.job.quantity_units;
    match recourse_route_id {
        None => case
            .unserved_penalty_per_unit
            .checked_mul(quantity)
            .ok_or_else(|| fail("unserved_cost_overflow")),
        Some(route_id) => {
            let route = case
                .routes
                .iter()
                .find(|route| route.route_id == route_id)
                .ok_or_else(|| fail("recourse_references_unknown_route"))?;
            let mut total = route
                .cost_units
                .checked_mul(quantity)
                .ok_or_else(|| fail("route_cost_overflow"))?;
            if route_id != primary_route_id {
                total = total
                    .checked_add(
                        case.switch_penalty_per_unit
                            .checked_mul(quantity)
                            .ok_or_else(|| fail("switch_penalty_overflow"))?,
                    )
                    .ok_or_else(|| fail("scenario_cost_overflow"))?;
            }
            if scenario
                .hidden_quality_rejected_route_ids
                .contains(&route.route_id)
            {
                total = total
                    .checked_add(
                        case.quality_rejection_penalty_per_unit
                            .checked_mul(quantity)
                            .ok_or_else(|| fail("quality_rejection_penalty_overflow"))?,
                    )
                    .ok_or_else(|| fail("scenario_cost_overflow"))?;
            }
            Ok(total)
        }
    }
}

fn weighted_information_set_cost(
    case: &TreeCase,
    primary_route_id: &str,
    scenarios: &[&Scenario],
    recourse_route_id: Option<&str>,
) -> Result<u64, String> {
    let mut total = 0_u64;
    for scenario in scenarios {
        let term = scenario
            .probability_ppm
            .checked_mul(scenario_cost_units(
                case,
                primary_route_id,
                recourse_route_id,
                scenario,
            )?)
            .ok_or_else(|| fail("probability_weighted_cost_overflow"))?;
        total = total
            .checked_add(term)
            .ok_or_else(|| fail("expected_cost_numerator_overflow"))?;
    }
    Ok(total)
}

fn greedy_policy(case: &TreeCase) -> Result<PolicyPlan, String> {
    let primary = available_initial_routes(case)
        .into_iter()
        .next()
        .ok_or_else(|| fail("no_initially_admissible_route"))?;
    let mut recourse = BTreeMap::new();

    for (information_set, scenarios) in information_groups(case) {
        let observations = &scenarios[0].observations;
        let available = available_recourse_routes(case, observations);
        let selected = if available
            .iter()
            .any(|route| route.route_id == primary.route_id)
        {
            Some(primary.route_id.clone())
        } else {
            available
                .first()
                .map(|route| route.route_id.clone())
        };
        recourse.insert(information_set, selected);
    }

    Ok(PolicyPlan {
        primary_route_id: primary.route_id.clone(),
        recourse_by_information_set: recourse,
    })
}

fn exact_policy(case: &TreeCase) -> Result<(PolicyPlan, u64), String> {
    let groups = information_groups(case);
    let mut best: Option<(PolicyPlan, u64)> = None;

    for primary in available_initial_routes(case) {
        let mut recourse = BTreeMap::new();
        let mut expected_numerator = 0_u64;

        for (information_set, scenarios) in &groups {
            let observations = &scenarios[0].observations;
            let mut options: Vec<Option<String>> = available_recourse_routes(case, observations)
                .into_iter()
                .map(|route| Some(route.route_id.clone()))
                .collect();
            options.push(None);

            let mut local_best: Option<(Option<String>, u64)> = None;
            for option in options {
                let value = weighted_information_set_cost(
                    case,
                    &primary.route_id,
                    scenarios,
                    option.as_deref(),
                )?;
                let tie_key = option.as_deref().unwrap_or("~");
                let replace = local_best.as_ref().is_none_or(|(previous, old_value)| {
                    value < *old_value
                        || (value == *old_value
                            && tie_key < previous.as_deref().unwrap_or("~"))
                });
                if replace {
                    local_best = Some((option, value));
                }
            }
            let (choice, value) =
                local_best.ok_or_else(|| fail("information_set_has_no_action"))?;
            recourse.insert(information_set.clone(), choice);
            expected_numerator = expected_numerator
                .checked_add(value)
                .ok_or_else(|| fail("policy_expected_cost_overflow"))?;
        }

        let plan = PolicyPlan {
            primary_route_id: primary.route_id.clone(),
            recourse_by_information_set: recourse,
        };
        let replace = best.as_ref().is_none_or(|(old_plan, old_value)| {
            expected_numerator < *old_value
                || (expected_numerator == *old_value
                    && plan.primary_route_id < old_plan.primary_route_id)
        });
        if replace {
            best = Some((plan, expected_numerator));
        }
    }

    best.ok_or_else(|| fail("exact_policy_has_no_initial_route"))
}

fn clairvoyant_lower_bound(case: &TreeCase) -> Result<u64, String> {
    let mut best_expected = None;
    for primary in available_initial_routes(case) {
        let mut total = 0_u64;
        for scenario in &case.scenarios {
            let mut options: Vec<Option<String>> = available_recourse_routes(
                case,
                &scenario.observations,
            )
            .into_iter()
            .map(|route| Some(route.route_id.clone()))
            .collect();
            options.push(None);
            let min_cost = options
                .iter()
                .map(|choice| {
                    scenario_cost_units(
                        case,
                        &primary.route_id,
                        choice.as_deref(),
                        scenario,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .min()
                .ok_or_else(|| fail("clairvoyant_scenario_has_no_action"))?;
            let weighted = scenario
                .probability_ppm
                .checked_mul(min_cost)
                .ok_or_else(|| fail("clairvoyant_weighted_cost_overflow"))?;
            total = total
                .checked_add(weighted)
                .ok_or_else(|| fail("clairvoyant_expected_cost_overflow"))?;
        }
        if best_expected.is_none_or(|old| total < old) {
            best_expected = Some(total);
        }
    }
    best_expected.ok_or_else(|| fail("clairvoyant_has_no_initial_route"))
}

fn validate_case(case: &TreeCase) -> Result<(), String> {
    if case.case_id.trim().is_empty()
        || case.job.job_id.trim().is_empty()
        || case.job.quantity_units == 0
        || case.job.max_lead_time_hours == 0
        || case.job.max_total_cost_units == 0
        || case.switch_penalty_per_unit == 0
        || case.unserved_penalty_per_unit == 0
        || case.quality_rejection_penalty_per_unit == 0
    {
        return Err(fail("case_or_job_identity_and_constraints_invalid"));
    }
    if case.routes.is_empty() || case.scenarios.len() < 2 {
        return Err(fail("case_requires_routes_and_multiple_scenarios"));
    }

    let mut route_ids = HashSet::new();
    let mut domains = HashSet::new();
    for route in &case.routes {
        if route.route_id.trim().is_empty()
            || !route_ids.insert(route.route_id.as_str())
            || route.capacity_units == 0
            || route.cost_units == 0
            || route.lead_time_hours == 0
            || route.common_cause_domains.is_empty()
        {
            return Err(fail("route_identity_or_capability_invalid"));
        }
        for domain in &route.common_cause_domains {
            if domain.trim().is_empty() {
                return Err(fail("empty_common_cause_domain"));
            }
            domains.insert(domain.as_str());
        }
    }

    let mut scenario_ids = HashSet::new();
    let mut total_probability = 0_u64;
    let mut groups: BTreeMap<&str, &Observations> = BTreeMap::new();
    let mut group_counts: BTreeMap<&str, usize> = BTreeMap::new();
    for scenario in &case.scenarios {
        if scenario.scenario_id.trim().is_empty()
            || scenario.information_set_id.trim().is_empty()
            || !scenario_ids.insert(scenario.scenario_id.as_str())
            || scenario.probability_ppm == 0
        {
            return Err(fail("scenario_identity_or_probability_invalid"));
        }
        total_probability = total_probability
            .checked_add(scenario.probability_ppm)
            .ok_or_else(|| fail("scenario_probability_overflow"))?;
        *group_counts
            .entry(scenario.information_set_id.as_str())
            .or_default() += 1;
        if let Some(previous) = groups.insert(
            scenario.information_set_id.as_str(),
            &scenario.observations,
        ) {
            if previous != &scenario.observations {
                return Err(fail("same_information_set_has_different_observations"));
            }
        }
        for route_id in &scenario.observations.failed_route_ids {
            if !route_ids.contains(route_id.as_str()) {
                return Err(fail("observation_references_unknown_failed_route"));
            }
        }
        for route_id in &scenario.observations.unknown_route_ids {
            if !route_ids.contains(route_id.as_str()) {
                return Err(fail("observation_references_unknown_unknown_route"));
            }
        }
        for domain in &scenario.observations.failed_domains {
            if !domains.contains(domain.as_str()) {
                return Err(fail("observation_references_unknown_failed_domain"));
            }
        }
        for (route_id, capacity) in &scenario.observations.capacity_reductions {
            let route = case.routes.iter().find(|r| r.route_id == *route_id)
                .ok_or_else(|| fail("capacity_reduction_references_unknown_route"))?;
            if *capacity > route.capacity_units {
                return Err(fail("capacity_reduction_cannot_increase_capacity"));
            }
        }
        for route_id in &scenario.hidden_quality_rejected_route_ids {
            if !route_ids.contains(route_id.as_str()) {
                return Err(fail("hidden_quality_event_references_unknown_route"));
            }
        }
    }
    if total_probability != PROBABILITY_DENOMINATOR {
        return Err(fail("scenario_probabilities_must_sum_to_one_million_ppm"));
    }
    if !group_counts.values().any(|count| *count > 1) {
        return Err(fail("tree_requires_at_least_one_repeated_information_set"));
    }
    if available_initial_routes(case).is_empty() {
        return Err(fail("case_has_no_initially_admissible_route"));
    }
    Ok(())
}

fn validate_corpus(corpus: &Corpus) -> Result<(), String> {
    if corpus.schema_id != "https://luminousdynamics.io/schemas/mfg-route-tree-case-v0.1"
        || corpus.benchmark_id != "MFG-ROUTE-TREE-OPT-001"
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling.trim().is_empty()
        || corpus.family_manifest.len() != 6
        || corpus.case_count != 120
        || corpus.cases.len() != 120
    {
        return Err(fail("corpus_manifest_invalid"));
    }
    let mut family_ids = HashSet::new();
    for family in &corpus.family_manifest {
        let seeds: BTreeSet<u64> = family.replication_seeds.iter().copied().collect();
        if family.scenario_family_id.trim().is_empty()
            || !family_ids.insert(family.scenario_family_id.as_str())
            || family.case_count != 20
            || seeds != (1..=20).collect()
            || family.replication_seeds.len() != 20
        {
            return Err(fail("family_manifest_invalid"));
        }
    }
    let mut case_ids = HashSet::new();
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut seeds: BTreeMap<&str, BTreeSet<u64>> = BTreeMap::new();
    for case in &corpus.cases {
        if !family_ids.contains(case.scenario_family_id.as_str())
            || !case_ids.insert(case.case_id.as_str())
            || !(1..=20).contains(&case.seed)
        {
            return Err(fail("case_identity_or_family_invalid"));
        }
        *counts.entry(case.scenario_family_id.as_str()).or_default() += 1;
        if !seeds.entry(case.scenario_family_id.as_str()).or_default().insert(case.seed) {
            return Err(fail("duplicate_seed_within_family"));
        }
        validate_case(case)?;
    }
    for family in &corpus.family_manifest {
        if counts.get(family.scenario_family_id.as_str()) != Some(&20)
            || seeds.get(family.scenario_family_id.as_str()) != Some(&(1..=20).collect())
        {
            return Err(fail("family_case_or_seed_count_mismatch"));
        }
    }
    Ok(())
}

fn evaluate_case(case: &TreeCase) -> Result<CaseResult, String> {
    let greedy = greedy_policy(case)?;
    let (exact, exact_value) = exact_policy(case)?;
    let clairvoyant = clairvoyant_lower_bound(case)?;
    if clairvoyant > exact_value {
        return Err(fail("clairvoyant_lower_bound_exceeds_nonanticipative_optimum"));
    }
    let groups = information_groups(case);
    let mut greedy_value = 0_u64;
    for scenario in &case.scenarios {
        let choice = greedy
            .recourse_by_information_set
            .get(&scenario.information_set_id)
            .ok_or_else(|| fail("greedy_policy_missing_information_set_action"))?;
        let value = scenario_cost_units(
            case,
            &greedy.primary_route_id,
            choice.as_deref(),
            scenario,
        )?;
        greedy_value = greedy_value
            .checked_add(
                scenario
                    .probability_ppm
                    .checked_mul(value)
                    .ok_or_else(|| fail("greedy_weighted_cost_overflow"))?,
            )
            .ok_or_else(|| fail("greedy_expected_cost_overflow"))?;
    }
    if greedy_value < exact_value {
        return Err(fail("candidate_cost_below_exact_nonanticipative_oracle"));
    }
    Ok(CaseResult {
        case_id: case.case_id.clone(),
        scenario_family_id: case.scenario_family_id.clone(),
        seed: case.seed,
        scenario_count: case.scenarios.len(),
        information_set_count: groups.len(),
        greedy_plan: greedy,
        exact_policy_plan: exact,
        greedy_expected_cost_numerator: greedy_value,
        exact_expected_cost_numerator: exact_value,
        clairvoyant_lower_bound_numerator: clairvoyant,
        regret_numerator: greedy_value - exact_value,
        exact_policy_is_non_anticipative: true,
        clairvoyant_is_diagnostic_only: true,
    })
}

fn validate_known_answers(corpus: &Corpus) -> Result<(), String> {
    validate_json_schema(ANSWERS_JSON, ANSWERS_SCHEMA_JSON)?;
    let file: KnownAnswerFile = serde_json::from_str(ANSWERS_JSON)
        .map_err(|error| format!("known_answer_parse_error:{error}"))?;
    if file.schema_id != "https://luminousdynamics.io/schemas/mfg-route-tree-known-answers-v0.1"
        || file.benchmark_id != corpus.benchmark_id
        || file.evidence_class != "synthetic"
        || file.answer_count != corpus.cases.len()
        || file.answers.len() != corpus.cases.len()
    {
        return Err(fail("known_answer_manifest_mismatch"));
    }
    let by_id: HashMap<&str, &KnownAnswer> =
        file.answers.iter().map(|answer| (answer.case_id.as_str(), answer)).collect();
    if by_id.len() != file.answers.len() {
        return Err(fail("duplicate_known_answer_case_id"));
    }
    for case in &corpus.cases {
        let answer = by_id
            .get(case.case_id.as_str())
            .ok_or_else(|| fail("known_answer_missing_case"))?;
        let greedy = greedy_policy(case)?;
        let (exact, exact_value) = exact_policy(case)?;
        let clairvoyant = clairvoyant_lower_bound(case)?;
        let greedy_value = case.scenarios.iter().try_fold(0_u64, |sum, scenario| {
            let selected = greedy
                .recourse_by_information_set
                .get(&scenario.information_set_id)
                .ok_or_else(|| fail("known_answer_policy_missing_information_set"))?;
            let value = scenario_cost_units(case, &greedy.primary_route_id, selected.as_deref(), scenario)?;
            sum.checked_add(scenario.probability_ppm.checked_mul(value)
                .ok_or_else(|| fail("known_answer_greedy_weighted_cost_overflow"))?)
                .ok_or_else(|| fail("known_answer_greedy_sum_overflow"))
        })?;
        if answer.greedy_plan != greedy
            || answer.exact_policy_plan != exact
            || answer.greedy_expected_cost_numerator != greedy_value
            || answer.exact_expected_cost_numerator != exact_value
            || answer.clairvoyant_lower_bound_numerator != clairvoyant
        {
            return Err(fail("scenario_tree_known_answer_mismatch"));
        }
    }
    Ok(())
}

fn evaluate(corpus: &Corpus) -> Result<Report, String> {
    validate_corpus(corpus)?;
    let mut cases = Vec::with_capacity(corpus.cases.len());
    for case in &corpus.cases {
        cases.push(evaluate_case(case)?);
    }
    let greedy_expected_cost_numerator = cases.iter().map(|case| case.greedy_expected_cost_numerator).sum();
    let exact_expected_cost_numerator = cases.iter().map(|case| case.exact_expected_cost_numerator).sum();
    let clairvoyant_lower_bound_numerator = cases.iter().map(|case| case.clairvoyant_lower_bound_numerator).sum();
    let scenario_leaf_count = corpus.cases.iter().map(|case| case.scenarios.len()).sum();
    let information_set_count = cases.iter().map(|case| case.information_set_count).sum();
    Ok(Report {
        benchmark_id: "MFG-ROUTE-TREE-OPT-001",
        evaluator_version: "0.1.0",
        evidence_class: "synthetic",
        family_count: corpus.family_manifest.len(),
        case_count: cases.len(),
        scenario_leaf_count,
        information_set_count,
        greedy_expected_cost_numerator,
        exact_expected_cost_numerator,
        clairvoyant_lower_bound_numerator,
        greedy_regret_numerator: greedy_expected_cost_numerator - exact_expected_cost_numerator,
        cases,
        non_anticipativity_verified: true,
        recommendations_are_non_binding: true,
        execution_authority_used: false,
        physical_acceptance_claimed: false,
        production_release_authorized: false,
        claim_ceiling: "Synthetic two-stage scenario-tree optimization only; no reservations, dispatch, physical execution, product acceptance or supplier qualification.",
    })
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let content = match args.as_slice() {
        [] => CORPUS_JSON.to_owned(),
        [path] => fs::read_to_string(path).map_err(|error| format!("corpus_read_error:{error}"))?,
        _ => return Err(fail("usage: mfg-route-tree-optimizer [corpus.json]")),
    };
    validate_json_schema(&content, CORPUS_SCHEMA_JSON)?;
    let corpus: Corpus = serde_json::from_str(&content)
        .map_err(|error| format!("corpus_parse_error:{error}"))?;
    validate_corpus(&corpus)?;
    validate_known_answers(&corpus)?;
    let report = evaluate(&corpus)?;
    println!("{}", serde_json::to_string_pretty(&report)
        .map_err(|error| format!("report_serialization_error:{error}"))?);
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
        serde_json::from_str(CORPUS_JSON).expect("frozen scenario tree corpus parses")
    }

    #[test]
    fn draft_2020_12_schema_accepts_corpus_and_rejects_unknown_fields() {
        let schema: serde_json::Value = serde_json::from_str(CORPUS_SCHEMA_JSON).unwrap();
        let validator = jsonschema::draft202012::options().offline().build(&schema).unwrap();
        let instance: serde_json::Value = serde_json::from_str(CORPUS_JSON).unwrap();
        assert!(validator.is_valid(&instance));
        let mut tampered = instance;
        tampered.as_object_mut().unwrap().insert(
            "unexpected".into(),
            serde_json::Value::Bool(true),
        );
        assert!(!validator.is_valid(&tampered));
    }

    #[test]
    fn frozen_cross_language_answers_match_all_scenario_trees() {
        validate_known_answers(&corpus()).expect("120 exact policy answers match");
    }

    #[test]
    fn exact_policy_uses_one_action_per_information_set() {
        let input = corpus();
        for case in &input.cases {
            let (plan, _) = exact_policy(case).unwrap();
            for (id, scenarios) in information_groups(case) {
                assert!(scenarios.iter().all(|scenario| {
                    plan.recourse_by_information_set.get(&id).is_some()
                        && scenario.information_set_id == id
                }));
            }
        }
    }

    #[test]
    fn clairvoyant_lower_bound_never_exceeds_nonanticipative_optimum() {
        let input = corpus();
        for case in &input.cases {
            let (_, exact) = exact_policy(case).unwrap();
            let clairvoyant = clairvoyant_lower_bound(case).unwrap();
            assert!(clairvoyant <= exact);
        }
    }

    #[test]
    fn hidden_future_quality_outcome_is_not_used_to_split_information_set_action() {
        let input = corpus();
        let case = input.cases.iter()
            .find(|case| case.scenario_family_id == "quality-inspection-uncertainty")
            .unwrap();
        let pair: Vec<&Scenario> = case.scenarios.iter()
            .filter(|scenario| scenario.information_set_id == "local-down")
            .collect();
        assert_eq!(pair.len(), 2);
        assert_ne!(pair[0].hidden_quality_rejected_route_ids, pair[1].hidden_quality_rejected_route_ids);
        let (plan, _) = exact_policy(case).unwrap();
        assert_eq!(plan.recourse_by_information_set.len(), 4);
        assert!(pair.iter().all(|scenario| plan.recourse_by_information_set.contains_key(&scenario.information_set_id)));
    }

    #[test]
    fn duplicate_information_set_with_different_observations_is_rejected() {
        let mut input = corpus();
        let case = &mut input.cases[0];
        let second = case.scenarios.iter_mut()
            .find(|scenario| scenario.scenario_id == "local-down-hidden-quality-rejection")
            .unwrap();
        second.observations.failed_route_ids.clear();
        assert!(validate_case(case).unwrap_err().contains("same_information_set_has_different_observations"));
    }

    #[test]
    fn probabilities_must_sum_exactly_to_one_million() {
        let mut input = corpus();
        input.cases[0].scenarios[0].probability_ppm -= 1;
        assert!(validate_case(&input.cases[0]).unwrap_err().contains("scenario_probabilities_must_sum_to_one_million_ppm"));
    }

    #[test]
    fn exact_policy_is_no_worse_than_greedy_for_every_case() {
        let input = corpus();
        for case in &input.cases {
            let g = greedy_policy(case).unwrap();
            let (x, x_value) = exact_policy(case).unwrap();
            let g_value = case.scenarios.iter().map(|scenario| {
                scenario.probability_ppm * scenario_cost_units(
                    case,
                    &g.primary_route_id,
                    g.recourse_by_information_set[&scenario.information_set_id].as_deref(),
                    scenario,
                ).unwrap()
            }).sum::<u64>();
            assert!(x_value <= g_value);
            assert!(!x.primary_route_id.is_empty());
        }
    }
}
