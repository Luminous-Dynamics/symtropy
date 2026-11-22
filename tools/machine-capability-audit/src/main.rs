use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const EMBEDDED_CORPUS: &str =
    include_str!("../../../fixtures/manufacturing/machine_capability_cases_v0_1.json");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    schema_id: String,
    status: String,
    owner_issue: String,
    schema_path: String,
    dispositions: Vec<String>,
    case_count: usize,
    cases: Vec<Case>,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    scenario_id: String,
    description: String,
    part: Part,
    routes: Vec<Route>,
    expected: Expected,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Part {
    part_id: String,
    revision: String,
    risk_class: String,
    allowed_tiers: Vec<String>,
    rights_status: String,
    revision_status: String,
    process_family: String,
    required_inspection: Vec<String>,
    post_repair_verification_required: bool,
    required_max_temperature_c: f64,
    deadline_hours: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Route {
    route_id: String,
    tier: String,
    machine_id: String,
    process_family: String,
    support_status: String,
    machine_identity_status: String,
    machine_safety_status: String,
    material_status: String,
    calibration_status: String,
    operator_status: String,
    inspection_status: String,
    post_repair_status: String,
    machine_envelope_status: String,
    max_process_temperature_c: f64,
    inspection_methods: Vec<String>,
    capacity_units: Capacity,
    lead_time_hours: f64,
    dependencies: Vec<Dependency>,
    cost: Cost,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Capacity {
    required: f64,
    available: f64,
    unit: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Dependency {
    dependency_id: String,
    criticality: String,
    sourcing_class: String,
    alternate_sources: u32,
    local_repair_status: String,
    evidence_status: String,
    requires: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cost {
    capital_allocation_zar: f64,
    material_zar: f64,
    energy_zar: f64,
    labour_zar: f64,
    setup_zar: f64,
    calibration_zar: f64,
    inspection_zar: f64,
    finishing_zar: f64,
    transport_zar: f64,
    maintenance_zar: f64,
    failure_rework_zar: f64,
    warranty_reserve_zar: f64,
    accepted_outputs: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Expected {
    disposition: String,
    feasible_route_ids: Vec<String>,
    route_results: Vec<ExpectedRouteResult>,
    local_closure_disposition: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedRouteResult {
    route_id: String,
    disposition: String,
    blockers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Disposition {
    Feasible,
    Infeasible,
    Unknown,
    Unsupported,
}

impl Disposition {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Feasible => "feasible",
            Self::Infeasible => "infeasible",
            Self::Unknown => "unknown",
            Self::Unsupported => "unsupported",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LocalClosure {
    LocallyReproducible,
    LocalServiceOnly,
    ImportDependent,
    Unknown,
}

impl LocalClosure {
    fn as_str(&self) -> &'static str {
        match self {
            Self::LocallyReproducible => "locally_reproducible",
            Self::LocalServiceOnly => "local_service_only",
            Self::ImportDependent => "import_dependent",
            Self::Unknown => "unknown",
        }
    }

    fn preference(&self) -> u8 {
        match self {
            Self::LocallyReproducible => 0,
            Self::LocalServiceOnly => 1,
            Self::ImportDependent => 2,
            Self::Unknown => 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RouteResult {
    route_id: String,
    disposition: Disposition,
    blockers: Vec<String>,
    local_closure: LocalClosure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CaseResult {
    disposition: Disposition,
    feasible_route_ids: Vec<String>,
    route_results: Vec<RouteResult>,
    best_feasible_route_local_closure: LocalClosure,
}

fn insert_status(
    status: &str,
    good: &str,
    bad: &str,
    unknown: &str,
    hard: &mut BTreeSet<String>,
    uncertain: &mut BTreeSet<String>,
    bad_code: &str,
    unknown_code: &str,
) {
    if status == bad {
        hard.insert(bad_code.to_owned());
    } else if status == unknown {
        uncertain.insert(unknown_code.to_owned());
    } else if status != good {
        hard.insert(format!("unsupported_status:{status}"));
    }
}

fn evaluate_route(part: &Part, route: &Route) -> RouteResult {
    if route.support_status == "unsupported" {
        return RouteResult {
            route_id: route.route_id.clone(),
            disposition: Disposition::Unsupported,
            blockers: vec!["process_family_unsupported".to_owned()],
            local_closure: dependency_closure(&route.dependencies),
        };
    }

    let mut hard = BTreeSet::new();
    let mut uncertain = BTreeSet::new();

    match part.rights_status.as_str() {
        "verified" => {}
        "unknown" => { uncertain.insert("rights_unknown".to_owned()); }
        "revoked" | "not_permitted" => { hard.insert("rights_revoked".to_owned()); }
        _ => { hard.insert("rights_status_unsupported".to_owned()); }
    }

    match part.revision_status.as_str() {
        "exact" => {}
        "unknown" => { uncertain.insert("product_revision_unknown".to_owned()); }
        "mismatch" => { hard.insert("product_revision_mismatch".to_owned()); }
        _ => { hard.insert("product_revision_status_unsupported".to_owned()); }
    }

    if !part.allowed_tiers.iter().any(|tier| tier == &route.tier) {
        hard.insert("tier_not_allowed".to_owned());
    }
    if part.risk_class == "P3" && (route.tier == "H0" || route.tier == "N1") {
        hard.insert("p3_forbidden_tier".to_owned());
    }
    if part.process_family != route.process_family {
        hard.insert("process_family_mismatch".to_owned());
    }

    insert_status(
        &route.machine_identity_status, "verified", "revoked", "unknown",
        &mut hard, &mut uncertain, "machine_identity_revoked", "machine_identity_unknown",
    );
    insert_status(
        &route.machine_safety_status, "qualified", "unqualified", "unknown",
        &mut hard, &mut uncertain, "machine_safety_unqualified", "machine_safety_unknown",
    );
    insert_status(
        &route.material_status, "approved", "prohibited", "unknown",
        &mut hard, &mut uncertain, "material_prohibited", "material_unknown",
    );
    insert_status(
        &route.calibration_status, "current", "expired", "unknown",
        &mut hard, &mut uncertain, "calibration_expired", "calibration_unknown",
    );
    insert_status(
        &route.operator_status, "qualified", "unqualified", "unknown",
        &mut hard, &mut uncertain, "operator_unqualified", "operator_unknown",
    );
    insert_status(
        &route.inspection_status, "qualified", "missing", "unknown",
        &mut hard, &mut uncertain, "inspection_capability_missing", "inspection_capability_unknown",
    );

    if part.required_inspection.iter().any(|method| {
        !route.inspection_methods.iter().any(|available| available == method)
    }) {
        hard.insert("inspection_method_missing".to_owned());
    }

    if part.post_repair_verification_required {
        match route.post_repair_status.as_str() {
            "complete" => {}
            "unknown" => { uncertain.insert("post_repair_verification_unknown".to_owned()); }
            "missing" | "not_required" => {
                hard.insert("post_repair_verification_missing".to_owned());
            }
            _ => { hard.insert("post_repair_status_unsupported".to_owned()); }
        }
    }

    match route.machine_envelope_status.as_str() {
        "within" => {}
        "unknown" => { uncertain.insert("machine_envelope_unknown".to_owned()); }
        "outside" => { hard.insert("machine_envelope_outside".to_owned()); }
        _ => { hard.insert("machine_envelope_status_unsupported".to_owned()); }
    }

    if route.capacity_units.available < route.capacity_units.required {
        hard.insert("capacity_insufficient".to_owned());
    }
    if route.lead_time_hours > part.deadline_hours {
        hard.insert("deadline_missed".to_owned());
    }
    if part.required_max_temperature_c > route.max_process_temperature_c {
        hard.insert("process_temperature_exceeded".to_owned());
    }

    let (disposition, blockers) = if !hard.is_empty() {
        (Disposition::Infeasible, hard.into_iter().collect())
    } else if !uncertain.is_empty() {
        (Disposition::Unknown, uncertain.into_iter().collect())
    } else {
        (Disposition::Feasible, Vec::new())
    };

    RouteResult {
        route_id: route.route_id.clone(),
        disposition,
        blockers,
        local_closure: dependency_closure(&route.dependencies),
    }
}

fn dependency_closure(dependencies: &[Dependency]) -> LocalClosure {
    if dependencies.is_empty() {
        return LocalClosure::Unknown;
    }

    let by_id: HashMap<&str, &Dependency> = dependencies
        .iter()
        .map(|dep| (dep.dependency_id.as_str(), dep))
        .collect();
    if by_id.len() != dependencies.len() {
        return LocalClosure::Unknown;
    }

    for dep in dependencies {
        if dep.requires.iter().any(|required| !by_id.contains_key(required.as_str())) {
            return LocalClosure::Unknown;
        }
    }

    fn visit(
        id: &str,
        by_id: &HashMap<&str, &Dependency>,
        visiting: &mut HashSet<String>,
        visited: &mut HashSet<String>,
    ) -> bool {
        if visited.contains(id) {
            return true;
        }
        if !visiting.insert(id.to_owned()) {
            return false;
        }
        let Some(dep) = by_id.get(id) else {
            return false;
        };
        for next in &dep.requires {
            if !visit(next, by_id, visiting, visited) {
                return false;
            }
        }
        visiting.remove(id);
        visited.insert(id.to_owned());
        true
    }

    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    for id in by_id.keys() {
        if !visit(id, &by_id, &mut visiting, &mut visited) {
            return LocalClosure::Unknown;
        }
    }

    let critical: Vec<&Dependency> = dependencies
        .iter()
        .filter(|dep| dep.criticality == "critical")
        .collect();
    if critical.is_empty() {
        return LocalClosure::Unknown;
    }

    if critical.iter().any(|dep| {
        matches!(
            dep.evidence_status.as_str(),
            "unknown" | "assumed" | "synthetic" | "revoked"
        ) || dep.sourcing_class == "unknown"
    }) {
        return LocalClosure::Unknown;
    }

    if critical
        .iter()
        .all(|dep| dep.sourcing_class == "locally_manufactured")
    {
        return LocalClosure::LocallyReproducible;
    }

    if critical.iter().any(|dep| {
        dep.sourcing_class == "imported_stocked"
            || dep.sourcing_class == "imported_long_lead"
    }) {
        return LocalClosure::ImportDependent;
    }

    if critical.iter().any(|dep| dep.sourcing_class == "local_service_imported_inputs") {
        return LocalClosure::LocalServiceOnly;
    }

    LocalClosure::Unknown
}

fn validate_case(case: &Case) -> Result<(), String> {
    if case.scenario_id.trim().is_empty() || case.description.trim().is_empty() {
        return Err("empty scenario identity or description".to_owned());
    }
    if case.part.part_id.trim().is_empty() || case.part.revision.trim().is_empty() {
        return Err("empty part identity or revision".to_owned());
    }
    if case.part.allowed_tiers.is_empty() {
        return Err("allowed_tiers cannot be empty".to_owned());
    }
    if has_duplicates(&case.part.allowed_tiers) {
        return Err("duplicate allowed_tiers".to_owned());
    }
    if case.routes.is_empty() {
        return Err("route list cannot be empty".to_owned());
    }

    let mut route_ids = BTreeSet::new();
    for route in &case.routes {
        if !route_ids.insert(route.route_id.as_str()) {
            return Err(format!("duplicate route id: {}", route.route_id));
        }
        if route.route_id.trim().is_empty() || route.machine_id.trim().is_empty() {
            return Err("empty route or machine identity".to_owned());
        }
        if route.capacity_units.required <= 0.0
            || route.capacity_units.available < 0.0
            || route.capacity_units.unit.trim().is_empty()
        {
            return Err(format!("invalid capacity units in {}", route.route_id));
        }
        if !route.lead_time_hours.is_finite()
            || !route.max_process_temperature_c.is_finite()
            || !case.part.required_max_temperature_c.is_finite()
            || !case.part.deadline_hours.is_finite()
        {
            return Err(format!("non-finite numeric input in {}", route.route_id));
        }
        if !cost_is_valid(&route.cost) {
            return Err(format!("invalid cost inputs in {}", route.route_id));
        }
        if has_duplicates(&route.inspection_methods) {
            return Err(format!("duplicate inspection methods in {}", route.route_id));
        }
        let mut dependency_ids = BTreeSet::new();
        for dep in &route.dependencies {
            if dep.dependency_id.trim().is_empty()
                || !dependency_ids.insert(dep.dependency_id.as_str())
            {
                return Err(format!("empty or duplicate dependency id in {}", route.route_id));
            }
            if has_duplicates(&dep.requires) {
                return Err(format!("duplicate dependency edge in {}", route.route_id));
            }
        }
    }

    let expected_ids: BTreeSet<&str> = case.expected.route_results
        .iter().map(|result| result.route_id.as_str()).collect();
    if expected_ids.len() != case.expected.route_results.len()
        || expected_ids != route_ids
    {
        return Err("expected route-results do not exactly cover candidate routes".to_owned());
    }

    if has_duplicates(&case.expected.feasible_route_ids) {
        return Err("duplicate feasible route ids".to_owned());
    }

    Ok(())
}

fn has_duplicates<T: Eq + std::hash::Hash>(items: &[T]) -> bool {
    let set: HashSet<&T> = items.iter().collect();
    set.len() != items.len()
}

fn cost_is_valid(cost: &Cost) -> bool {
    let values = [
        cost.capital_allocation_zar,
        cost.material_zar,
        cost.energy_zar,
        cost.labour_zar,
        cost.setup_zar,
        cost.calibration_zar,
        cost.inspection_zar,
        cost.finishing_zar,
        cost.transport_zar,
        cost.maintenance_zar,
        cost.failure_rework_zar,
        cost.warranty_reserve_zar,
        cost.accepted_outputs,
    ];
    values.iter().all(|value| value.is_finite() && *value >= 0.0)
        && cost.accepted_outputs > 0.0
}

fn evaluate_case(case: &Case) -> Result<CaseResult, String> {
    validate_case(case)?;
    let mut route_results: Vec<RouteResult> = case.routes
        .iter()
        .map(|route| evaluate_route(&case.part, route))
        .collect();
    route_results.sort_by(|a, b| a.route_id.cmp(&b.route_id));

    let feasible_route_ids: Vec<String> = route_results
        .iter()
        .filter(|result| result.disposition == Disposition::Feasible)
        .map(|result| result.route_id.clone())
        .collect();

    let disposition = if !feasible_route_ids.is_empty() {
        Disposition::Feasible
    } else if route_results.iter().any(|result| result.disposition == Disposition::Unknown) {
        Disposition::Unknown
    } else if route_results.iter().any(|result| result.disposition == Disposition::Infeasible) {
        Disposition::Infeasible
    } else {
        Disposition::Unsupported
    };

    let best_feasible_route_local_closure = route_results
        .iter()
        .filter(|result| result.disposition == Disposition::Feasible)
        .map(|result| result.local_closure.clone())
        .min_by_key(LocalClosure::preference)
        .unwrap_or(LocalClosure::Unknown);

    Ok(CaseResult {
        disposition,
        feasible_route_ids,
        route_results,
        best_feasible_route_local_closure,
    })
}

fn verify_corpus(corpus: &Corpus) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if corpus.case_count != corpus.cases.len() {
        errors.push(format!(
            "case_count is {}, but corpus contains {} cases",
            corpus.case_count,
            corpus.cases.len()
        ));
    }
    if corpus.schema_id != "MACHINE-CAPABILITY-COMPILER-CASES-0.1" {
        errors.push("unexpected schema_id".to_owned());
    }
    if corpus.cases.is_empty() {
        errors.push("corpus must not be empty".to_owned());
    }

    let mut scenario_ids = BTreeSet::new();
    for case in &corpus.cases {
        if !scenario_ids.insert(case.scenario_id.as_str()) {
            errors.push(format!("duplicate scenario id: {}", case.scenario_id));
            continue;
        }
        let actual = match evaluate_case(case) {
            Ok(result) => result,
            Err(error) => {
                errors.push(format!("{}: {error}", case.scenario_id));
                continue;
            }
        };

        if actual.disposition.as_str() != case.expected.disposition {
            errors.push(format!(
                "{}: disposition expected {}, got {}",
                case.scenario_id, case.expected.disposition, actual.disposition.as_str()
            ));
        }
        if actual.feasible_route_ids != sorted(case.expected.feasible_route_ids.clone()) {
            errors.push(format!(
                "{}: feasible routes expected {:?}, got {:?}",
                case.scenario_id,
                sorted(case.expected.feasible_route_ids.clone()),
                actual.feasible_route_ids
            ));
        }
        if actual.best_feasible_route_local_closure.as_str()
            != case.expected.local_closure_disposition
        {
            errors.push(format!(
                "{}: local closure expected {}, got {}",
                case.scenario_id,
                case.expected.local_closure_disposition,
                actual.best_feasible_route_local_closure.as_str()
            ));
        }

        let expected_by_id: BTreeMap<&str, &ExpectedRouteResult> = case.expected.route_results
            .iter().map(|result| (result.route_id.as_str(), result)).collect();
        for result in &actual.route_results {
            let Some(expected) = expected_by_id.get(result.route_id.as_str()) else {
                errors.push(format!("{}: missing expected route {}", case.scenario_id, result.route_id));
                continue;
            };
            if result.disposition.as_str() != expected.disposition {
                errors.push(format!(
                    "{} / {}: disposition expected {}, got {}",
                    case.scenario_id, result.route_id, expected.disposition, result.disposition.as_str()
                ));
            }
            if result.blockers != sorted(expected.blockers.clone()) {
                errors.push(format!(
                    "{} / {}: blockers expected {:?}, got {:?}",
                    case.scenario_id, result.route_id,
                    sorted(expected.blockers.clone()), result.blockers
                ));
            }
        }
    }

    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

fn sorted(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values
}

fn parse_corpus(input: &str) -> Result<Corpus, String> {
    serde_json::from_str(input).map_err(|error| format!("invalid corpus JSON: {error}"))
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let corpus_text = if let Some(path) = args.first() {
        fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))?
    } else {
        EMBEDDED_CORPUS.to_owned()
    };
    let corpus = parse_corpus(&corpus_text)?;
    verify_corpus(&corpus).map_err(|errors| errors.join("\n"))?;

    println!(
        "machine-capability-audit: {} synthetic known-answer cases passed",
        corpus.cases.len()
    );
    println!("schema: {}", corpus.schema_path);
    println!("status: {}", corpus.status);
    println!("owner: {}", corpus.owner_issue);
    println!("claim ceiling: {}", corpus.claim_ceiling);
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("machine-capability-audit: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> Corpus {
        parse_corpus(EMBEDDED_CORPUS).expect("embedded corpus must parse")
    }

    #[test]
    fn all_frozen_known_answers_match() {
        let parsed = corpus();
        verify_corpus(&parsed).expect("all known answers should match");
    }

    #[test]
    fn irrelevant_route_order_does_not_change_assessment() {
        let parsed = corpus();
        let mut case = parsed.cases
            .iter()
            .find(|case| case.scenario_id == "MCC-013")
            .expect("MCC-013 exists")
            .clone();

        let before = evaluate_case(&case).expect("baseline evaluation");
        case.routes.reverse();
        let after = evaluate_case(&case).expect("permuted evaluation");
        assert_eq!(before, after);
    }

    #[test]
    fn revoked_rights_cannot_be_offset_by_low_cost() {
        let parsed = corpus();
        let mut case = parsed.cases
            .iter()
            .find(|case| case.scenario_id == "MCC-001")
            .expect("MCC-001 exists")
            .clone();
        case.part.rights_status = "revoked".to_owned();

        let result = evaluate_case(&case).expect("mutated case is structurally valid");
        assert_eq!(result.disposition, Disposition::Infeasible);
        assert!(!result.feasible_route_ids.contains(&"N1-FDM-01".to_owned()));
    }

    #[test]
    fn stale_calibration_is_a_hard_blocker() {
        let parsed = corpus();
        let mut case = parsed.cases
            .iter()
            .find(|case| case.scenario_id == "MCC-001")
            .expect("MCC-001 exists")
            .clone();
        case.routes[0].calibration_status = "expired".to_owned();

        let result = evaluate_case(&case).expect("mutated case is structurally valid");
        assert_eq!(result.disposition, Disposition::Infeasible);
        assert!(result.route_results[0].blockers.contains(&"calibration_expired".to_owned()));
    }

    #[test]
    fn cyclic_dependency_never_proves_local_reproducibility() {
        let parsed = corpus();
        let case = parsed.cases
            .iter()
            .find(|case| case.scenario_id == "MCC-014")
            .expect("MCC-014 exists");
        let result = evaluate_case(case).expect("cycle is a valid adversarial fixture");
        assert_eq!(
            result.best_feasible_route_local_closure,
            LocalClosure::Unknown
        );
    }

    #[test]
    fn missing_or_unknown_evidence_never_becomes_feasible() {
        let parsed = corpus();
        for id in ["MCC-002", "MCC-004", "MCC-016"] {
            let case = parsed.cases.iter().find(|case| case.scenario_id == id).unwrap();
            let result = evaluate_case(case).expect("fixture should evaluate");
            assert_eq!(result.disposition, Disposition::Unknown, "{id}");
            assert!(result.feasible_route_ids.is_empty(), "{id}");
        }
    }
}
