use std::collections::{BTreeMap, BTreeSet};

const TOPOLOGY: &str = include_str!("../topology-v0.1.tsv");
const ROUTES: &str = include_str!("../routes-v0.1.tsv");
const SCENARIOS: &str = include_str!("../scenarios-v0.1.tsv");

#[derive(Clone, Debug)]
struct Dependency {
    fault_domain_id: String,
    qualification_id: String,
    valid_from_utc: String,
    valid_until_utc: String,
    declared_state: String,
}

#[derive(Clone, Debug)]
struct Route {
    id: String,
    expected_dependency_rows: usize,
    dependencies: Vec<Dependency>,
    requirement_kinds: BTreeSet<String>,
}

#[derive(Clone, Debug)]
struct Scenario {
    id: String,
    topology_id: String,
    decision_time_utc: String,
    observable_prefix_id: String,
    failed_fault_domains: BTreeSet<String>,
    revoked_qualification_ids: BTreeSet<String>,
    unknown_dependency_route_ids: BTreeSet<String>,
    event_visible_at_decision: bool,
    acceptance_gate_state: String,
    expected_available_routes: BTreeSet<String>,
    expected_unknown_routes: BTreeSet<String>,
    expected_independent_lower_bound: usize,
    expected_independent_upper_bound: usize,
    expected_accepted_output: bool,
    expected_output_disposition: String,
    non_anticipativity_pair: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RouteState {
    Available,
    Blocked,
    Unknown,
}

#[derive(Clone, Debug)]
struct EvaluatedRoute {
    id: String,
    state: RouteState,
    known_fault_domains: BTreeSet<String>,
}

#[derive(Clone, Debug)]
pub struct Evaluation {
    pub scenario_id: String,
    pub available_routes: Vec<String>,
    pub unknown_routes: Vec<String>,
    pub blocked_routes: Vec<String>,
    pub independent_lower_bound: usize,
    pub independent_upper_bound: usize,
    pub synthetic_output_accepted: bool,
    pub output_disposition: String,
    pub decision_input_fingerprint: String,
}

fn parse_rows(
    input: &str,
    expected_columns: usize,
    source: &str,
) -> Result<Vec<Vec<String>>, String> {
    let mut rows = Vec::new();
    for (index, line) in input.lines().enumerate().skip(1) {
        if line.trim().is_empty() {
            continue;
        }
        let fields = line.split('\t').map(str::to_owned).collect::<Vec<_>>();
        if fields.len() != expected_columns {
            return Err(format!(
                "{source}: line {} has {} columns; expected {expected_columns}",
                index + 1,
                fields.len()
            ));
        }
        if fields.iter().any(|field| field.trim().is_empty()) {
            return Err(format!(
                "{source}: line {} contains a blank field",
                index + 1
            ));
        }
        rows.push(fields);
    }
    Ok(rows)
}

fn parse_set(value: &str) -> BTreeSet<String> {
    if value == "-" || value.is_empty() {
        return BTreeSet::new();
    }
    value.split(';').map(str::to_owned).collect()
}

fn parse_bool(value: &str, source: &str, line: usize, field: &str) -> Result<bool, String> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!(
            "{source}: line {line}: {field} must be true or false"
        )),
    }
}

fn parse_routes(input: &str) -> Result<BTreeMap<String, Route>, String> {
    let rows = parse_rows(input, 4, "routes-v0.1.tsv")?;
    let mut routes = BTreeMap::new();
    for (index, row) in rows.into_iter().enumerate() {
        let expected = row[3].parse::<usize>().map_err(|_| {
            format!(
                "routes-v0.1.tsv: line {} has invalid dependency count",
                index + 2
            )
        })?;
        if expected == 0 {
            return Err(format!(
                "routes-v0.1.tsv: line {} has zero dependency count",
                index + 2
            ));
        }
        let route = Route {
            id: row[0].clone(),
            expected_dependency_rows: expected,
            dependencies: Vec::new(),
            requirement_kinds: BTreeSet::new(),
        };
        if routes.insert(row[0].clone(), route).is_some() {
            return Err(format!("routes-v0.1.tsv: duplicate route id {}", row[0]));
        }
    }
    if routes.is_empty() {
        return Err("routes-v0.1.tsv: empty route manifest".to_owned());
    }
    Ok(routes)
}

fn parse_topology(input: &str, route_manifest: &str) -> Result<BTreeMap<String, Route>, String> {
    let mut routes = parse_routes(route_manifest)?;
    let rows = parse_rows(input, 10, "topology-v0.1.tsv")?;

    for (index, row) in rows.into_iter().enumerate() {
        let line = index + 2;
        let route = routes
            .get_mut(&row[0])
            .ok_or_else(|| format!("topology-v0.1.tsv: line {line}: unknown route {}", row[0]))?;

        if !route.requirement_kinds.insert(row[2].clone()) {
            return Err(format!(
                "topology-v0.1.tsv: line {line}: duplicate requirement kind {} for route {}",
                row[2], row[0]
            ));
        }
        if row[3] == "-" || row[4] == "-" || row[5] == "-" || row[6] == "-" {
            return Err(format!(
                "topology-v0.1.tsv: line {line}: dependency identity fields may not be '-'"
            ));
        }
        if row[7] >= row[8] {
            return Err(format!(
                "topology-v0.1.tsv: line {line}: invalid qualification validity interval"
            ));
        }
        if !matches!(row[9].as_str(), "CURRENT" | "STALE" | "UNKNOWN") {
            return Err(format!(
                "topology-v0.1.tsv: line {line}: invalid qualification state"
            ));
        }
        route.dependencies.push(Dependency {
            fault_domain_id: row[4].clone(),
            qualification_id: row[5].clone(),
            valid_from_utc: row[7].clone(),
            valid_until_utc: row[8].clone(),
            declared_state: row[9].clone(),
        });
    }

    for (route_id, route) in &routes {
        if route.dependencies.len() != route.expected_dependency_rows {
            return Err(format!(
                "topology-v0.1.tsv: route {route_id} has {} dependency rows; manifest requires {}",
                route.dependencies.len(),
                route.expected_dependency_rows
            ));
        }
    }
    Ok(routes)
}

fn parse_scenarios(input: &str) -> Result<Vec<Scenario>, String> {
    let rows = parse_rows(input, 19, "scenarios-v0.1.tsv")?;
    let mut scenarios = Vec::new();
    let mut ids = BTreeSet::new();

    for (index, row) in rows.into_iter().enumerate() {
        let line = index + 2;
        if !ids.insert(row[0].clone()) {
            return Err(format!(
                "scenarios-v0.1.tsv: duplicate scenario id {}",
                row[0]
            ));
        }
        if row[3].len() != 20 || !row[3].ends_with('Z') {
            return Err(format!(
                "scenarios-v0.1.tsv: line {line}: decision time must be UTC RFC3339 seconds"
            ));
        }
        let event_visible = parse_bool(
            &row[9],
            "scenarios-v0.1.tsv",
            line,
            "event_visible_at_decision",
        )?;
        if row[5] == "AFTER_DECISION" && event_visible {
            return Err(format!(
                "scenarios-v0.1.tsv: line {line}: post-decision event cannot be visible at decision time"
            ));
        }
        if row[5] == "BEFORE_DECISION" && !event_visible {
            return Err(format!(
                "scenarios-v0.1.tsv: line {line}: pre-decision event must be visible"
            ));
        }
        if !matches!(row[10].as_str(), "PASS" | "FAIL" | "UNKNOWN") {
            return Err(format!(
                "scenarios-v0.1.tsv: line {line}: invalid acceptance gate state"
            ));
        }
        let lower = row[13]
            .parse::<usize>()
            .map_err(|_| format!("scenarios-v0.1.tsv: line {line}: invalid lower bound"))?;
        let upper = row[14]
            .parse::<usize>()
            .map_err(|_| format!("scenarios-v0.1.tsv: line {line}: invalid upper bound"))?;
        if lower > upper {
            return Err(format!(
                "scenarios-v0.1.tsv: line {line}: lower bound exceeds upper bound"
            ));
        }
        let pair = if row[17] == "-" {
            None
        } else {
            Some(row[17].clone())
        };
        scenarios.push(Scenario {
            id: row[0].clone(),
            topology_id: row[2].clone(),
            decision_time_utc: row[3].clone(),
            observable_prefix_id: row[4].clone(),
            failed_fault_domains: parse_set(&row[6]),
            revoked_qualification_ids: parse_set(&row[7]),
            unknown_dependency_route_ids: parse_set(&row[8]),
            event_visible_at_decision: event_visible,
            acceptance_gate_state: row[10].clone(),
            expected_available_routes: parse_set(&row[11]),
            expected_unknown_routes: parse_set(&row[12]),
            expected_independent_lower_bound: lower,
            expected_independent_upper_bound: upper,
            expected_accepted_output: parse_bool(
                &row[15],
                "scenarios-v0.1.tsv",
                line,
                "expected_accepted_output",
            )?,
            expected_output_disposition: row[16].clone(),
            non_anticipativity_pair: pair,
        });
    }
    if scenarios.is_empty() {
        return Err("scenarios-v0.1.tsv: empty scenario corpus".to_owned());
    }
    Ok(scenarios)
}

fn route_state(route: &Route, scenario: &Scenario) -> RouteState {
    let known_blocker = route.dependencies.iter().any(|dependency| {
        scenario
            .failed_fault_domains
            .contains(&dependency.fault_domain_id)
            || scenario
                .revoked_qualification_ids
                .contains(&dependency.qualification_id)
            || dependency.declared_state == "STALE"
            || scenario.decision_time_utc < dependency.valid_from_utc
            || scenario.decision_time_utc >= dependency.valid_until_utc
    });
    if known_blocker {
        return RouteState::Blocked;
    }
    let has_unknown_dependency = scenario.unknown_dependency_route_ids.contains(&route.id)
        || route
            .dependencies
            .iter()
            .any(|dependency| dependency.declared_state == "UNKNOWN");
    if has_unknown_dependency {
        return RouteState::Unknown;
    }
    RouteState::Available
}

fn max_pairwise_disjoint(
    sets: &[BTreeSet<String>],
    index: usize,
    used: &BTreeSet<String>,
) -> usize {
    if index == sets.len() {
        return 0;
    }

    let without_current = max_pairwise_disjoint(sets, index + 1, used);
    if !sets[index].is_disjoint(used) {
        return without_current;
    }

    let mut next_used = used.clone();
    next_used.extend(sets[index].iter().cloned());
    let with_current = 1 + max_pairwise_disjoint(sets, index + 1, &next_used);
    without_current.max(with_current)
}

fn independent_bound(routes: &[&EvaluatedRoute]) -> usize {
    let sets = routes
        .iter()
        .map(|route| route.known_fault_domains.clone())
        .collect::<Vec<_>>();
    max_pairwise_disjoint(&sets, 0, &BTreeSet::new())
}

fn decision_fingerprint(scenario: &Scenario) -> String {
    let mut input = format!(
        "{}|{}",
        scenario.observable_prefix_id, scenario.decision_time_utc
    );
    if scenario.event_visible_at_decision {
        input.push('|');
        input.push_str(
            &scenario
                .failed_fault_domains
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(";"),
        );
        input.push('|');
        input.push_str(
            &scenario
                .revoked_qualification_ids
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(";"),
        );
        input.push('|');
        input.push_str(
            &scenario
                .unknown_dependency_route_ids
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(";"),
        );
    }

    // FNV-1a is used only as a deterministic test fingerprint, never as a signature or integrity authority.
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn evaluate_scenario(
    routes: &BTreeMap<String, Route>,
    scenario: &Scenario,
) -> Result<Evaluation, String> {
    let mut evaluated = Vec::new();
    for route in routes.values() {
        let state = route_state(route, scenario);
        let known_fault_domains = route
            .dependencies
            .iter()
            .map(|dependency| dependency.fault_domain_id.clone())
            .collect::<BTreeSet<_>>();
        evaluated.push(EvaluatedRoute {
            id: route.id.clone(),
            state,
            known_fault_domains,
        });
    }

    for unknown_route in &scenario.unknown_dependency_route_ids {
        if !routes.contains_key(unknown_route) {
            return Err(format!(
                "scenario {} refers to unknown route {unknown_route}",
                scenario.id
            ));
        }
    }

    let available = evaluated
        .iter()
        .filter(|route| route.state == RouteState::Available)
        .collect::<Vec<_>>();
    let unknown = evaluated
        .iter()
        .filter(|route| route.state == RouteState::Unknown)
        .collect::<Vec<_>>();
    let blocked = evaluated
        .iter()
        .filter(|route| route.state == RouteState::Blocked)
        .collect::<Vec<_>>();

    let mut available_ids = available
        .iter()
        .map(|route| route.id.clone())
        .collect::<Vec<_>>();
    let mut unknown_ids = unknown
        .iter()
        .map(|route| route.id.clone())
        .collect::<Vec<_>>();
    let mut blocked_ids = blocked
        .iter()
        .map(|route| route.id.clone())
        .collect::<Vec<_>>();
    available_ids.sort();
    unknown_ids.sort();
    blocked_ids.sort();

    let lower = independent_bound(&available);
    let mut optimistic = available.clone();
    optimistic.extend(unknown.iter().copied());
    let upper = independent_bound(&optimistic);

    if lower > upper {
        return Err(format!(
            "scenario {} produced inverted independent-path bounds",
            scenario.id
        ));
    }

    let output_disposition = if available.is_empty() {
        if unknown.is_empty() {
            "BLOCKED"
        } else {
            "UNKNOWN"
        }
    } else {
        match scenario.acceptance_gate_state.as_str() {
            "PASS" => "ACCEPTED",
            "FAIL" => "REJECTED",
            "UNKNOWN" => "UNKNOWN",
            _ => "UNKNOWN",
        }
    };
    let synthetic_output_accepted = output_disposition == "ACCEPTED";

    let mut expected_available = scenario
        .expected_available_routes
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let mut expected_unknown = scenario
        .expected_unknown_routes
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    expected_available.sort();
    expected_unknown.sort();

    if available_ids != expected_available {
        return Err(format!(
            "scenario {} available-route mismatch: actual {:?}, expected {:?}",
            scenario.id, available_ids, expected_available
        ));
    }
    if unknown_ids != expected_unknown {
        return Err(format!(
            "scenario {} unknown-route mismatch: actual {:?}, expected {:?}",
            scenario.id, unknown_ids, expected_unknown
        ));
    }
    if lower != scenario.expected_independent_lower_bound
        || upper != scenario.expected_independent_upper_bound
    {
        return Err(format!(
            "scenario {} independent-path interval mismatch: actual [{lower},{upper}], expected [{},{}]",
            scenario.id,
            scenario.expected_independent_lower_bound,
            scenario.expected_independent_upper_bound
        ));
    }
    if synthetic_output_accepted != scenario.expected_accepted_output {
        return Err(format!(
            "scenario {} accepted-output mismatch: actual {synthetic_output_accepted}, expected {}",
            scenario.id, scenario.expected_accepted_output
        ));
    }
    if output_disposition != scenario.expected_output_disposition {
        return Err(format!(
            "scenario {} output disposition mismatch: actual {output_disposition}, expected {}",
            scenario.id, scenario.expected_output_disposition
        ));
    }

    Ok(Evaluation {
        scenario_id: scenario.id.clone(),
        available_routes: available_ids,
        unknown_routes: unknown_ids,
        blocked_routes: blocked_ids,
        independent_lower_bound: lower,
        independent_upper_bound: upper,
        synthetic_output_accepted,
        output_disposition: output_disposition.to_owned(),
        decision_input_fingerprint: decision_fingerprint(scenario),
    })
}

fn verify_non_anticipativity(scenarios: &[Scenario]) -> Result<(), String> {
    let mut pairs: BTreeMap<String, Vec<&Scenario>> = BTreeMap::new();
    for scenario in scenarios {
        if let Some(pair) = scenario.non_anticipativity_pair.as_deref() {
            pairs.entry(pair.to_owned()).or_default().push(scenario);
        }
    }

    for (pair_id, members) in pairs {
        if members.len() < 2 {
            return Err(format!(
                "non-anticipativity pair {pair_id} has fewer than two scenarios"
            ));
        }
        let first = members[0];
        if first.event_visible_at_decision {
            return Err(format!(
                "non-anticipativity pair {pair_id} exposes its future event"
            ));
        }
        let first_fingerprint = decision_fingerprint(first);
        for member in members.iter().skip(1) {
            if member.event_visible_at_decision
                || member.observable_prefix_id != first.observable_prefix_id
                || member.decision_time_utc != first.decision_time_utc
                || decision_fingerprint(member) != first_fingerprint
            {
                return Err(format!(
                    "non-anticipativity pair {pair_id} changes the pre-event decision input"
                ));
            }
        }
    }
    Ok(())
}

pub fn evaluate_frozen_corpus() -> Result<Vec<Evaluation>, String> {
    let routes = parse_topology(TOPOLOGY, ROUTES)?;
    let scenarios = parse_scenarios(SCENARIOS)?;
    verify_non_anticipativity(&scenarios)?;

    let mut evaluations = Vec::new();
    for scenario in &scenarios {
        if scenario.topology_id != "pump-valve-topology-v0.1" {
            return Err(format!(
                "scenario {} references unsupported topology {}",
                scenario.id, scenario.topology_id
            ));
        }
        evaluations.push(evaluate_scenario(&routes, scenario)?);
    }
    Ok(evaluations)
}

pub fn render_tsv(evaluations: &[Evaluation]) -> String {
    let mut output = String::from(
        "scenario_id\tavailable_routes\tunknown_routes\tblocked_routes\tindependent_lower_bound\tindependent_upper_bound\tsynthetic_output_accepted\toutput_disposition\tdecision_input_fingerprint\n",
    );
    for evaluation in evaluations {
        output.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            evaluation.scenario_id,
            evaluation.available_routes.join(";"),
            evaluation.unknown_routes.join(";"),
            evaluation.blocked_routes.join(";"),
            evaluation.independent_lower_bound,
            evaluation.independent_upper_bound,
            evaluation.synthetic_output_accepted,
            evaluation.output_disposition,
            evaluation.decision_input_fingerprint
        ));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_corpus_matches_all_expected_dispositions() {
        let evaluations = evaluate_frozen_corpus().expect("frozen corpus should evaluate");
        assert_eq!(evaluations.len(), 16);
    }

    #[test]
    fn nominal_path_count_is_not_independent_path_count() {
        let evaluations = evaluate_frozen_corpus().expect("frozen corpus should evaluate");
        let baseline = evaluations
            .iter()
            .find(|entry| entry.scenario_id == "S00")
            .unwrap();
        assert_eq!(baseline.available_routes.len(), 5);
        assert_eq!(baseline.independent_lower_bound, 3);
        assert_eq!(baseline.independent_upper_bound, 3);
    }

    #[test]
    fn unknown_route_is_not_admitted_and_bounds_remain_explicit() {
        let evaluations = evaluate_frozen_corpus().expect("frozen corpus should evaluate");
        let unknown = evaluations
            .iter()
            .find(|entry| entry.scenario_id == "S08")
            .unwrap();
        assert!(
            !unknown
                .available_routes
                .iter()
                .any(|route| route == "partner_d")
        );
        assert_eq!(unknown.unknown_routes, vec!["partner_d".to_owned()]);
        assert_eq!(unknown.independent_lower_bound, 3);
        assert_eq!(unknown.independent_upper_bound, 3);
    }

    #[test]
    fn route_availability_does_not_override_failed_first_article_gate() {
        let evaluations = evaluate_frozen_corpus().expect("frozen corpus should evaluate");
        let rejected = evaluations
            .iter()
            .find(|entry| entry.scenario_id == "S13")
            .unwrap();
        assert_eq!(rejected.available_routes.len(), 5);
        assert!(!rejected.synthetic_output_accepted);
        assert_eq!(rejected.output_disposition, "REJECTED");
    }

    #[test]
    fn unknown_only_route_is_not_promoted_to_available_or_blocked() {
        let evaluations = evaluate_frozen_corpus().expect("frozen corpus should evaluate");
        let unknown = evaluations
            .iter()
            .find(|entry| entry.scenario_id == "S14")
            .unwrap();
        assert!(unknown.available_routes.is_empty());
        assert_eq!(unknown.unknown_routes, vec!["independent_e".to_owned()]);
        assert_eq!(unknown.independent_lower_bound, 0);
        assert_eq!(unknown.independent_upper_bound, 1);
        assert_eq!(unknown.output_disposition, "UNKNOWN");
        assert!(!unknown.synthetic_output_accepted);
    }

    #[test]
    fn unknown_first_article_gate_is_not_reported_as_rejection() {
        let evaluations = evaluate_frozen_corpus().expect("frozen corpus should evaluate");
        let unknown = evaluations
            .iter()
            .find(|entry| entry.scenario_id == "S15")
            .unwrap();
        assert_eq!(unknown.available_routes.len(), 5);
        assert_eq!(unknown.output_disposition, "UNKNOWN");
        assert!(!unknown.synthetic_output_accepted);
    }

    #[test]
    fn no_route_is_blocked_not_an_inspection_rejection() {
        let evaluations = evaluate_frozen_corpus().expect("frozen corpus should evaluate");
        let blocked = evaluations
            .iter()
            .find(|entry| entry.scenario_id == "S10")
            .unwrap();
        assert_eq!(blocked.output_disposition, "BLOCKED");
        assert!(!blocked.synthetic_output_accepted);
    }

    #[test]
    fn common_cause_disables_both_local_routes() {
        let evaluations = evaluate_frozen_corpus().expect("frozen corpus should evaluate");
        let power_failure = evaluations
            .iter()
            .find(|entry| entry.scenario_id == "S01")
            .unwrap();
        assert!(
            !power_failure
                .available_routes
                .iter()
                .any(|route| route == "local_a")
        );
        assert!(
            !power_failure
                .available_routes
                .iter()
                .any(|route| route == "local_b")
        );
        assert_eq!(power_failure.independent_lower_bound, 2);
    }

    #[test]
    fn hidden_future_pairs_have_identical_pre_event_inputs() {
        let evaluations = evaluate_frozen_corpus().expect("frozen corpus should evaluate");
        let left = evaluations
            .iter()
            .find(|entry| entry.scenario_id == "S11")
            .unwrap();
        let right = evaluations
            .iter()
            .find(|entry| entry.scenario_id == "S12")
            .unwrap();
        assert_eq!(
            left.decision_input_fingerprint,
            right.decision_input_fingerprint
        );
        assert_ne!(left.available_routes, right.available_routes);
    }

    #[test]
    fn incomplete_topology_is_rejected_by_manifest_counts() {
        let routes =
            "route_id\tarchitecture\tpath_class\texpected_dependency_rows\nonly\tlocal\tcell\t1\n";
        let topology = "route_id\toperation_id\trequirement_kind\tdependency_id\tfault_domain_id\tqualification_id\tevidence_ref\tvalid_from_utc\tvalid_until_utc\tdeclared_state\n";
        assert!(parse_topology(topology, routes).is_err());
    }

    #[test]
    fn duplicate_requirement_kind_is_rejected() {
        let routes =
            "route_id\tarchitecture\tpath_class\texpected_dependency_rows\nr\tlocal\tcell\t2\n";
        let row = "r\top\ttool\tdep1\tfd1\tq1\tev1\t2026-01-01T00:00:00Z\t2027-01-01T00:00:00Z\tCURRENT\n";
        let topology = format!(
            "route_id\toperation_id\trequirement_kind\tdependency_id\tfault_domain_id\tqualification_id\tevidence_ref\tvalid_from_utc\tvalid_until_utc\tdeclared_state\n{row}{}",
            row.replace("dep1", "dep2")
                .replace("q1", "q2")
                .replace("ev1", "ev2")
        );
        assert!(parse_topology(&topology, routes).is_err());
    }
}

    #[test]
    fn unknown_qualification_is_unknown_and_expands_optimistic_bound() {
        let mut routes = parse_topology(TOPOLOGY, ROUTES).unwrap();
        let route = routes.get_mut("independent_e").unwrap();
        route.dependencies[0].declared_state = "UNKNOWN".to_owned();
        let mut scenario = parse_scenarios(SCENARIOS)
            .unwrap()
            .remove("S00")
            .unwrap();
        scenario.expected_available_routes.remove("independent_e");
        scenario.expected_unknown_routes.insert("independent_e".to_owned());
        scenario.expected_independent_lower_bound = 2;
        scenario.expected_independent_upper_bound = 3;
        let result = evaluate_scenario(&routes, &scenario).expect("unknown qualification should remain explicit");
        assert_eq!(result.independent_lower_bound, 2);
        assert_eq!(result.independent_upper_bound, 3);
        assert_eq!(result.unknown_routes, vec!["independent_e".to_owned()]);
    }

    #[test]
    fn stale_qualification_blocks_route() {
        let mut routes = parse_topology(TOPOLOGY, ROUTES).unwrap();
        let route = routes.get_mut("independent_e").unwrap();
        route.dependencies[0].declared_state = "STALE".to_owned();
        let scenario = parse_scenarios(SCENARIOS).unwrap().remove("S00").unwrap();
        assert_eq!(route_state(route, &scenario), RouteState::Blocked);
    }

    #[test]
    fn pairwise_independent_path_count_matches_separate_subset_oracle() {
        fn mask_set(mask: u8) -> BTreeSet<String> {
            (0..3)
                .filter(|bit| mask & (1 << bit) != 0)
                .map(|bit| format!("fd-{bit}"))
                .collect()
        }
        for a in 0..8 {
            for b in 0..8 {
                for c in 0..8 {
                    for d in 0..8 {
                        let sets = vec![mask_set(a), mask_set(b), mask_set(c), mask_set(d)];
                        let mut oracle = 0;
                        for subset in 0..(1_usize << sets.len()) {
                            let selected = (0..sets.len())
                                .filter(|index| subset & (1 << index) != 0)
                                .collect::<Vec<_>>();
                            let pairwise_disjoint = selected.iter().enumerate().all(|(i, left)| {
                                selected.iter().skip(i + 1).all(|right| {
                                    sets[*left].is_disjoint(&sets[*right])
                                })
                            });
                            if pairwise_disjoint {
                                oracle = oracle.max(selected.len());
                            }
                        }
                        assert_eq!(
                            max_pairwise_disjoint(&sets, 0, &BTreeSet::new()),
                            oracle,
                            "masks {a},{b},{c},{d}"
                        );
                    }
                }
            }
        }
    }
}
