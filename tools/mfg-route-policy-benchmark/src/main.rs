use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const EMBEDDED_CORPUS: &str =
    include_str!("../../../fixtures/manufacturing/mfg_route_policy_cases_v0_1.json");
const EMBEDDED_ANSWERS: &str =
    include_str!("../../../fixtures/manufacturing/mfg_route_policy_known_answers_v0_1.json");
const EMBEDDED_CASE_SCHEMA: &str =
    include_str!("../../../schemas/manufacturing/mfg_route_policy_case_v0_1.schema.json");
const EMBEDDED_ANSWER_SCHEMA: &str =
    include_str!("../../../schemas/manufacturing/mfg_route_policy_known_answers_v0_1.schema.json");
const PPM: u64 = 1_000_000;
const POLICIES: [&str; 5] = [
    "cheapest_admissible",
    "fastest_admissible",
    "reliability_first",
    "expected_cost",
    "fixed_priority",
];

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    schema_id: String,
    benchmark_id: String,
    evidence_class: String,
    family_manifest: Vec<FamilySpec>,
    case_count: usize,
    cases: Vec<Case>,
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
struct Case {
    case_id: String,
    scenario_family_id: String,
    seed: u64,
    decision_cutoff_hour: u64,
    request: Request,
    route_candidates: Vec<Route>,
    evaluator_only: EvaluatorOnly,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    quantity_units: u64,
    deadline_hours: u64,
    budget_units: u64,
    unserved_penalty_units: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Route {
    route_id: String,
    owner_disposition: String,
    blockers: Vec<String>,
    visible_to_policy: bool,
    evidence_observed_hour: u64,
    quote_valid_until_hour: u64,
    cost_units: u64,
    lead_time_hours: u64,
    declared_capacity_units: u64,
    route_failure_probability_ppm: u64,
    common_cause_group_id: String,
    common_cause_failure_probability_ppm: u64,
    declared_priority: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvaluatorOnly {
    failed_common_cause_group_ids: Vec<String>,
    failed_route_ids: Vec<String>,
    additional_delay_hours_by_route: BTreeMap<String, u64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AnswerFile {
    schema_id: String,
    benchmark_id: String,
    evidence_class: String,
    case_count: usize,
    cases: Vec<CaseAnswer>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CaseAnswer {
    case_id: String,
    oracle_route_id: Option<String>,
    oracle_expected_cost_units_micros: u64,
    policies: Vec<PolicyAnswer>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyAnswer {
    policy_id: String,
    chosen_route_id: Option<String>,
    expected_cost_units_micros: u64,
    regret_units_micros: u64,
    synthetic_completion: bool,
    on_time: bool,
    realized_cost_units: u64,
}

#[derive(Debug, Serialize)]
struct Report {
    benchmark_id: &'static str,
    evaluator_version: &'static str,
    evidence_class: &'static str,
    family_count: usize,
    case_count: usize,
    policy_ids: Vec<&'static str>,
    case_reports: Vec<CaseReport>,
    family_summaries: Vec<FamilySummary>,
    results_are_synthetic: bool,
    physical_acceptance_claimed: bool,
    production_release_authorized: bool,
    claim_ceiling: &'static str,
}

#[derive(Debug, Serialize)]
struct CaseReport {
    case_id: String,
    scenario_family_id: String,
    seed: u64,
    eligible_route_count: usize,
    oracle_route_id: Option<String>,
    oracle_expected_cost_units_micros: u64,
    clairvoyant_lower_bound_route_id: Option<String>,
    clairvoyant_lower_bound_cost_units: u64,
    policies: Vec<PolicyResult>,
}

#[derive(Debug, Serialize)]
struct PolicyResult {
    policy_id: &'static str,
    chosen_route_id: Option<String>,
    selected_route_was_owner_admissible: bool,
    expected_cost_units_micros: u64,
    regret_units_micros: u64,
    synthetic_completion: bool,
    on_time: bool,
    realized_cost_units: u64,
    failure_reason: Option<&'static str>,
}

#[derive(Debug, Serialize)]
struct FamilySummary {
    scenario_family_id: String,
    policy_id: &'static str,
    case_count: usize,
    selection_count: usize,
    abstention_count: usize,
    synthetic_completion_count: usize,
    on_time_count: usize,
    mean_expected_regret_units_micros: u64,
    mean_realized_cost_units: u64,
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

fn fail(message: impl Into<String>) -> String {
    message.into()
}

fn validate(corpus: &Corpus) -> Result<(), String> {
    if corpus.schema_id != "https://luminousdynamics.io/schemas/mfg-route-policy-case-v0.1" {
        return Err(fail("unsupported_route_policy_schema_id"));
    }
    if corpus.benchmark_id != "MFG-ROUTE-ROBUST-001" || corpus.evidence_class != "synthetic" {
        return Err(fail("benchmark_identity_or_evidence_class_invalid"));
    }
    if corpus.claim_ceiling.trim().is_empty() {
        return Err(fail("claim_ceiling_missing"));
    }
    if corpus.family_manifest.len() != 6 || corpus.case_count != 120 || corpus.cases.len() != 120 {
        return Err(fail("expected_exactly_six_families_and_120_cases"));
    }

    let mut family_counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut family_seeds: BTreeMap<&str, BTreeSet<u64>> = BTreeMap::new();
    let mut case_ids = HashSet::new();
    let mut manifest_ids = HashSet::new();
    for family in &corpus.family_manifest {
        if !manifest_ids.insert(family.scenario_family_id.as_str()) {
            return Err(fail("duplicate_scenario_family_manifest_id"));
        }
        if family.case_count != 20 || family.replication_seeds.len() != 20 {
            return Err(fail("each_family_requires_twenty_declared_seeds"));
        }
        let expected: BTreeSet<u64> = (1..=20).collect();
        let declared: BTreeSet<u64> = family.replication_seeds.iter().copied().collect();
        if declared != expected || declared.len() != family.replication_seeds.len() {
            return Err(fail("family_seed_manifest_must_be_exactly_1_through_20"));
        }
    }

    for case in &corpus.cases {
        if !case_ids.insert(case.case_id.as_str()) {
            return Err(fail("duplicate_case_id"));
        }
        if !manifest_ids.contains(case.scenario_family_id.as_str()) {
            return Err(fail("case_references_unknown_scenario_family"));
        }
        if !(1..=20).contains(&case.seed) {
            return Err(fail("seed_outside_declared_range"));
        }
        if case.request.quantity_units == 0
            || case.request.deadline_hours == 0
            || case.request.budget_units == 0
            || case.request.unserved_penalty_units == 0
        {
            return Err(fail("request_constraints_must_be_positive"));
        }
        if case.request.budget_units > 1_000_000_000
            || case.request.unserved_penalty_units > 1_000_000_000
        {
            return Err(fail("request_value_exceeds_safe_integer_bound"));
        }
        *family_counts.entry(case.scenario_family_id.as_str()).or_default() += 1;
        if !family_seeds
            .entry(case.scenario_family_id.as_str())
            .or_default()
            .insert(case.seed)
        {
            return Err(fail("duplicate_seed_within_scenario_family"));
        }

        let mut route_ids = HashSet::new();
        let mut domain_probabilities: HashMap<&str, u64> = HashMap::new();
        let mut domain_ids = HashSet::new();
        for route in &case.route_candidates {
            if !route_ids.insert(route.route_id.as_str()) {
                return Err(fail("duplicate_route_id_within_case"));
            }
            if route.route_id.trim().is_empty() || route.common_cause_group_id.trim().is_empty() {
                return Err(fail("route_or_common_cause_identity_missing"));
            }
            if !matches!(
                route.owner_disposition.as_str(),
                "feasible" | "infeasible" | "unknown" | "unsupported"
            ) {
                return Err(fail("unsupported_owner_disposition"));
            }
            if route.owner_disposition == "feasible" && !route.blockers.is_empty() {
                return Err(fail("feasible_owner_disposition_cannot_have_blockers"));
            }
            if route.owner_disposition != "feasible" && route.blockers.is_empty() {
                return Err(fail("non_feasible_owner_disposition_requires_blocker"));
            }
            if route.visible_to_policy && route.evidence_observed_hour > case.decision_cutoff_hour {
                return Err(fail("future_evidence_leakage_at_decision_cutoff"));
            }
            if route.cost_units == 0 || route.lead_time_hours == 0 {
                return Err(fail("route_cost_and_lead_time_must_be_positive"));
            }
            if route.cost_units > 1_000_000_000 {
                return Err(fail("route_cost_exceeds_safe_integer_bound"));
            }
            if route.route_failure_probability_ppm > PPM
                || route.common_cause_failure_probability_ppm > PPM
            {
                return Err(fail("failure_probability_exceeds_one_million_ppm"));
            }
            match domain_probabilities.insert(
                route.common_cause_group_id.as_str(),
                route.common_cause_failure_probability_ppm,
            ) {
                Some(previous) if previous != route.common_cause_failure_probability_ppm => {
                    return Err(fail("common_cause_group_probability_inconsistent"));
                }
                _ => {}
            }
            domain_ids.insert(route.common_cause_group_id.as_str());
        }
        if case.route_candidates.is_empty() {
            return Err(fail("case_requires_route_candidates"));
        }
        for failed_group in &case.evaluator_only.failed_common_cause_group_ids {
            if !domain_ids.contains(failed_group.as_str()) {
                return Err(fail("hidden_outcome_references_unknown_common_cause_group"));
            }
        }
        for failed_route in &case.evaluator_only.failed_route_ids {
            if !route_ids.contains(failed_route.as_str()) {
                return Err(fail("hidden_outcome_references_unknown_route"));
            }
        }
        for route_id in case.evaluator_only.additional_delay_hours_by_route.keys() {
            if !route_ids.contains(route_id.as_str()) {
                return Err(fail("hidden_delay_references_unknown_route"));
            }
        }
    }

    for family in &corpus.family_manifest {
        let count = family_counts.get(family.scenario_family_id.as_str()).copied().unwrap_or(0);
        let seeds = family_seeds
            .get(family.scenario_family_id.as_str())
            .cloned()
            .unwrap_or_default();
        if count != 20 || seeds != (1..=20).collect() {
            return Err(fail("scenario_family_case_count_or_seed_mismatch"));
        }
    }
    Ok(())
}

fn failure_probability_ppm(route: &Route) -> u64 {
    let route_probability = route.route_failure_probability_ppm;
    let domain_probability = route.common_cause_failure_probability_ppm;
    route_probability + domain_probability
        - (route_probability * domain_probability / PPM)
}

fn expected_cost_units_micros(route: Option<&Route>, request: &Request) -> u64 {
    match route {
        Some(route) => {
            route.cost_units * PPM
                + failure_probability_ppm(route) * request.unserved_penalty_units
        }
        None => request.unserved_penalty_units * PPM,
    }
}

fn eligible_routes(case: &Case) -> Vec<&Route> {
    case.route_candidates
        .iter()
        .filter(|route| {
            route.owner_disposition == "feasible"
                && route.blockers.is_empty()
                && route.visible_to_policy
                && route.evidence_observed_hour <= case.decision_cutoff_hour
                && route.quote_valid_until_hour >= case.decision_cutoff_hour
                && route.declared_capacity_units >= case.request.quantity_units
                && route.cost_units <= case.request.budget_units
                && route.lead_time_hours <= case.request.deadline_hours
        })
        .collect()
}

fn compare_routes(a: &Route, b: &Route, request: &Request, policy: &str) -> std::cmp::Ordering {
    match policy {
        "cheapest_admissible" => a
            .cost_units
            .cmp(&b.cost_units)
            .then(a.lead_time_hours.cmp(&b.lead_time_hours))
            .then(a.route_id.cmp(&b.route_id)),
        "fastest_admissible" => a
            .lead_time_hours
            .cmp(&b.lead_time_hours)
            .then(a.cost_units.cmp(&b.cost_units))
            .then(a.route_id.cmp(&b.route_id)),
        "reliability_first" => failure_probability_ppm(a)
            .cmp(&failure_probability_ppm(b))
            .then(
                expected_cost_units_micros(Some(a), request)
                    .cmp(&expected_cost_units_micros(Some(b), request)),
            )
            .then(a.route_id.cmp(&b.route_id)),
        "expected_cost" => expected_cost_units_micros(Some(a), request)
            .cmp(&expected_cost_units_micros(Some(b), request))
            .then(a.lead_time_hours.cmp(&b.lead_time_hours))
            .then(a.route_id.cmp(&b.route_id)),
        "fixed_priority" => a
            .declared_priority
            .cmp(&b.declared_priority)
            .then(a.route_id.cmp(&b.route_id)),
        _ => std::cmp::Ordering::Equal,
    }
}

fn choose_route<'a>(case: &'a Case, policy: &str) -> Option<&'a Route> {
    eligible_routes(case)
        .into_iter()
        .min_by(|a, b| compare_routes(a, b, &case.request, policy))
}

fn oracle_route(case: &Case) -> Option<&Route> {
    eligible_routes(case).into_iter().min_by(|a, b| {
        expected_cost_units_micros(Some(a), &case.request)
            .cmp(&expected_cost_units_micros(Some(b), &case.request))
            .then(a.lead_time_hours.cmp(&b.lead_time_hours))
            .then(a.route_id.cmp(&b.route_id))
    })
}

fn failed_route(case: &Case, route: &Route) -> Option<&'static str> {
    if case
        .evaluator_only
        .failed_common_cause_group_ids
        .contains(&route.common_cause_group_id)
    {
        Some("common_cause")
    } else if case.evaluator_only.failed_route_ids.contains(&route.route_id) {
        Some("route_specific")
    } else {
        None
    }
}

fn realized_cost_units(case: &Case, route: Option<&Route>) -> u64 {
    match route {
        None => case.request.unserved_penalty_units,
        Some(route) => {
            route.cost_units
                + if failed_route(case, route).is_some() {
                    case.request.unserved_penalty_units
                } else {
                    0
                }
        }
    }
}

fn clairvoyant_route(case: &Case) -> Option<&Route> {
    eligible_routes(case).into_iter().min_by(|a, b| {
        realized_cost_units(case, Some(a))
            .cmp(&realized_cost_units(case, Some(b)))
            .then(a.lead_time_hours.cmp(&b.lead_time_hours))
            .then(a.route_id.cmp(&b.route_id))
    })
}

fn evaluate_case(case: &Case) -> CaseReport {
    let eligible = eligible_routes(case);
    let oracle = oracle_route(case);
    let oracle_cost = expected_cost_units_micros(oracle, &case.request);
    let clairvoyant = clairvoyant_route(case);
    let clairvoyant_cost = realized_cost_units(case, clairvoyant);
    let mut policies = Vec::with_capacity(POLICIES.len());

    for policy_id in POLICIES {
        let selected = choose_route(case, policy_id);
        let expected_cost = expected_cost_units_micros(selected, &case.request);
        let failure_reason = selected.and_then(|route| failed_route(case, route));
        let completion = selected.is_some() && failure_reason.is_none();
        let on_time = selected.is_some_and(|route| {
            completion
                && route.lead_time_hours
                    + case
                        .evaluator_only
                        .additional_delay_hours_by_route
                        .get(&route.route_id)
                        .copied()
                        .unwrap_or(0)
                    <= case.request.deadline_hours
        });
        policies.push(PolicyResult {
            policy_id,
            chosen_route_id: selected.map(|route| route.route_id.clone()),
            selected_route_was_owner_admissible: selected.is_none_or(|route| {
                route.owner_disposition == "feasible" && route.blockers.is_empty()
            }),
            expected_cost_units_micros: expected_cost,
            regret_units_micros: expected_cost.saturating_sub(oracle_cost),
            synthetic_completion: completion,
            on_time,
            realized_cost_units: realized_cost_units(case, selected),
            failure_reason,
        });
    }

    CaseReport {
        case_id: case.case_id.clone(),
        scenario_family_id: case.scenario_family_id.clone(),
        seed: case.seed,
        eligible_route_count: eligible.len(),
        oracle_route_id: oracle.map(|route| route.route_id.clone()),
        oracle_expected_cost_units_micros: oracle_cost,
        clairvoyant_lower_bound_route_id: clairvoyant.map(|route| route.route_id.clone()),
        clairvoyant_lower_bound_cost_units: clairvoyant_cost,
        policies,
    }
}

fn summarize_family(
    family_id: &str,
    policy_id: &'static str,
    reports: &[CaseReport],
) -> FamilySummary {
    let selected_reports: Vec<&PolicyResult> = reports
        .iter()
        .filter(|report| report.scenario_family_id == family_id)
        .flat_map(|report| report.policies.iter())
        .filter(|result| result.policy_id == policy_id)
        .collect();
    let count = selected_reports.len() as u64;
    let selection_count = selected_reports
        .iter()
        .filter(|result| result.chosen_route_id.is_some())
        .count() as u64;
    let completion_count = selected_reports
        .iter()
        .filter(|result| result.synthetic_completion)
        .count() as u64;
    let on_time_count = selected_reports.iter().filter(|result| result.on_time).count() as u64;
    let regret_sum: u64 = selected_reports
        .iter()
        .map(|result| result.regret_units_micros)
        .sum();
    let realized_cost_sum: u64 = selected_reports
        .iter()
        .map(|result| result.realized_cost_units)
        .sum();
    debug_assert_eq!(count, 20, "validated family has twenty paired cases");
    FamilySummary {
        scenario_family_id: family_id.to_string(),
        policy_id,
        case_count: count as usize,
        selection_count: selection_count as usize,
        abstention_count: (count - selection_count) as usize,
        synthetic_completion_count: completion_count as usize,
        on_time_count: on_time_count as usize,
        mean_expected_regret_units_micros: regret_sum / count,
        mean_realized_cost_units: realized_cost_sum / count,
    }
}

fn evaluate(corpus: &Corpus) -> Result<Report, String> {
    validate(corpus)?;
    let case_reports: Vec<CaseReport> = corpus.cases.iter().map(evaluate_case).collect();
    let mut family_summaries = Vec::with_capacity(corpus.family_manifest.len() * POLICIES.len());
    for family in &corpus.family_manifest {
        for policy_id in POLICIES {
            family_summaries.push(summarize_family(
                &family.scenario_family_id,
                policy_id,
                &case_reports,
            ));
        }
    }
    Ok(Report {
        benchmark_id: "MFG-ROUTE-ROBUST-001",
        evaluator_version: "0.1.0",
        evidence_class: "synthetic",
        family_count: corpus.family_manifest.len(),
        case_count: corpus.cases.len(),
        policy_ids: POLICIES.to_vec(),
        case_reports,
        family_summaries,
        results_are_synthetic: true,
        physical_acceptance_claimed: false,
        production_release_authorized: false,
        claim_ceiling: "Synthetic route-choice and policy-evaluator semantics only; no physical manufacturing qualification or performance claim.",
    })
}

fn verify_known_answers(corpus: &Corpus) -> Result<(), String> {
    validate_json_schema(EMBEDDED_ANSWERS, EMBEDDED_ANSWER_SCHEMA)?;
    let answers: AnswerFile = serde_json::from_str(EMBEDDED_ANSWERS)
        .map_err(|error| format!("known_answer_fixture_parse_error:{error}"))?;
    if answers.schema_id
        != "https://luminousdynamics.io/schemas/mfg-route-policy-known-answers-v0.1"
        || answers.benchmark_id != corpus.benchmark_id
        || answers.evidence_class != "synthetic"
        || answers.case_count != corpus.cases.len()
        || answers.cases.len() != corpus.cases.len()
    {
        return Err(fail("known_answer_manifest_mismatch"));
    }
    let report = evaluate(corpus)?;
    let answer_by_id: HashMap<&str, &CaseAnswer> = answers
        .cases
        .iter()
        .map(|answer| (answer.case_id.as_str(), answer))
        .collect();
    if answer_by_id.len() != answers.cases.len() {
        return Err(fail("duplicate_known_answer_case_id"));
    }
    for result in &report.case_reports {
        let answer = answer_by_id
            .get(result.case_id.as_str())
            .ok_or_else(|| fail("known_answer_missing_case"))?;
        if result.oracle_route_id != answer.oracle_route_id
            || result.oracle_expected_cost_units_micros != answer.oracle_expected_cost_units_micros
            || answer.policies.len() != POLICIES.len()
        {
            return Err(fail("oracle_known_answer_mismatch"));
        }
        for policy_result in &result.policies {
            let expected = answer
                .policies
                .iter()
                .find(|entry| entry.policy_id == policy_result.policy_id)
                .ok_or_else(|| fail("known_answer_missing_policy"))?;
            if policy_result.chosen_route_id != expected.chosen_route_id
                || policy_result.expected_cost_units_micros != expected.expected_cost_units_micros
                || policy_result.regret_units_micros != expected.regret_units_micros
                || policy_result.synthetic_completion != expected.synthetic_completion
                || policy_result.on_time != expected.on_time
                || policy_result.realized_cost_units != expected.realized_cost_units
            {
                return Err(fail("policy_known_answer_mismatch"));
            }
        }
    }
    Ok(())
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let content = match args.as_slice() {
        [] => EMBEDDED_CORPUS.to_string(),
        [path] => fs::read_to_string(path)
            .map_err(|error| format!("corpus_read_error:{error}"))?,
        _ => return Err(fail("usage: mfg-route-policy-benchmark [corpus.json]")),
    };
    validate_json_schema(&content, EMBEDDED_CASE_SCHEMA)?;
    let corpus: Corpus = serde_json::from_str(&content)
        .map_err(|error| format!("corpus_parse_error:{error}"))?;
    verify_known_answers(&corpus)?;
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
        serde_json::from_str(EMBEDDED_CORPUS).expect("frozen corpus parses")
    }

    #[test]
    fn draft_2020_12_schema_accepts_corpus_and_rejects_unknown_fields() {
        let schema: serde_json::Value =
            serde_json::from_str(EMBEDDED_CASE_SCHEMA).expect("schema parses");
        let validator = jsonschema::draft202012::options()
            .offline()
            .build(&schema)
            .expect("Draft 2020-12 validator builds offline");
        let instance: serde_json::Value =
            serde_json::from_str(EMBEDDED_CORPUS).expect("corpus parses");
        assert!(validator.is_valid(&instance));
        let mut tampered = instance;
        tampered.as_object_mut().unwrap().insert(
            "unrecognized_property".into(),
            serde_json::Value::Bool(true),
        );
        assert!(!validator.is_valid(&tampered));
    }

    #[test]
    fn frozen_corpus_matches_independent_known_answers() {
        let input = corpus();
        assert_eq!(input.cases.len(), 120);
        verify_known_answers(&input).expect("independent known answers match");
    }

    #[test]
    fn unknown_infeasible_and_unsupported_routes_never_enter_candidate_set() {
        let input = corpus();
        for case in &input.cases {
            for route in &case.route_candidates {
                if route.owner_disposition != "feasible" {
                    assert!(
                        !eligible_routes(case)
                            .iter()
                            .any(|candidate| candidate.route_id == route.route_id)
                    );
                }
            }
        }
    }

    #[test]
    fn future_evaluator_outcomes_do_not_change_policy_selection() {
        let input = corpus();
        let original = &input.cases[0];
        let original_choice =
            choose_route(original, "expected_cost").map(|route| route.route_id.clone());
        let mut altered = original.clone();
        altered.evaluator_only.failed_common_cause_group_ids = altered
            .route_candidates
            .iter()
            .map(|route| route.common_cause_group_id.clone())
            .collect();
        altered.evaluator_only.failed_route_ids = altered
            .route_candidates
            .iter()
            .map(|route| route.route_id.clone())
            .collect();
        altered.evaluator_only.additional_delay_hours_by_route = altered
            .route_candidates
            .iter()
            .map(|route| (route.route_id.clone(), 999))
            .collect();
        let altered_choice =
            choose_route(&altered, "expected_cost").map(|route| route.route_id.clone());
        assert_eq!(original_choice, altered_choice);
    }

    #[test]
    fn common_cause_groups_keep_one_declared_probability_per_case() {
        let input = corpus();
        let case = input
            .cases
            .iter()
            .find(|case| case.scenario_family_id == "common-cause-disruption")
            .expect("common-cause family present");
        let mut grouped: BTreeMap<&str, u64> = BTreeMap::new();
        for route in &case.route_candidates {
            if let Some(previous) = grouped.insert(
                route.common_cause_group_id.as_str(),
                route.common_cause_failure_probability_ppm,
            ) {
                assert_eq!(previous, route.common_cause_failure_probability_ppm);
            }
        }
        let shared: Vec<&Route> = case
            .route_candidates
            .iter()
            .filter(|route| route.common_cause_group_id == "SHARED-IMPORT-CONTROL")
            .collect();
        assert_eq!(shared.len(), 2);
        assert_eq!(
            shared[0].common_cause_failure_probability_ppm,
            shared[1].common_cause_failure_probability_ppm
        );
    }

    #[test]
    fn no_admissible_route_is_an_explicit_abstention_not_a_zero_cost_result() {
        let input = corpus();
        let case = input
            .cases
            .iter()
            .find(|case| case.scenario_family_id == "demand-surge-capacity" && case.seed == 20)
            .expect("no-route case present");
        assert!(eligible_routes(case).is_empty());
        let result = evaluate_case(case);
        assert!(result.policies.iter().all(|policy| policy.chosen_route_id.is_none()));
        assert!(result
            .policies
            .iter()
            .all(|policy| policy.realized_cost_units == case.request.unserved_penalty_units));
        assert_eq!(
            result.oracle_expected_cost_units_micros,
            case.request.unserved_penalty_units * PPM
        );
    }

    #[test]
    fn mismatched_common_cause_probability_is_rejected() {
        let mut input = corpus();
        let case = input
            .cases
            .iter_mut()
            .find(|case| case.scenario_family_id == "common-cause-disruption")
            .expect("common-cause family present");
        case.route_candidates[1].common_cause_failure_probability_ppm += 1;
        assert!(validate(&input)
            .unwrap_err()
            .contains("common_cause_group_probability_inconsistent"));
    }

    #[test]
    fn a_future_observation_cannot_be_exposed_at_the_decision_cutoff() {
        let mut input = corpus();
        input.cases[0].route_candidates[0].evidence_observed_hour = 1;
        assert!(validate(&input)
            .unwrap_err()
            .contains("future_evidence_leakage_at_decision_cutoff"));
    }
}
