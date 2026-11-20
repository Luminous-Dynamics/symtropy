use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::{env, fs, path::Path};

const DEFAULT_FIXTURE: &str =
    "docs/research/port-energy-compute/OCEAN_RECOVERY_FAILURE_DOMAIN_GRAPH_V0_1.json";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Topology {
    schema_version: String,
    fixture_id: String,
    as_of: String,
    synthetic_only: bool,
    claim_ceiling: String,
    required_component_kinds: Vec<String>,
    independence_critical_kinds: Vec<String>,
    minimum_surviving_paths_after_any_single_domain_failure: usize,
    components: Vec<Component>,
    recovery_paths: Vec<RecoveryPath>,
    scenarios: Vec<Scenario>,
    site_evidence_admission_requirements: Vec<String>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Component {
    component_id: String,
    component_kind: String,
    failure_domains: Vec<String>,
    evidence_state: String,
    as_built_independence_verified: bool,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecoveryPath {
    path_id: String,
    label: String,
    role: String,
    service_tier: String,
    component_ids: Vec<String>,
    target_rpo_minutes: u64,
    target_rto_minutes: u64,
    targets_are_measured: bool,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    scenario_id: String,
    title: String,
    failed_domains: Vec<String>,
    expected_infrastructure_paths: Vec<String>,
    service_blocked_paths: Vec<String>,
    minimum_infrastructure_paths: usize,
    disposition: String,
}
#[derive(Debug, Serialize)]
struct ScenarioResult {
    scenario_id: String,
    title: String,
    infrastructure_paths: Vec<String>,
    service_paths: Vec<String>,
    passed: bool,
    disposition: String,
}
#[derive(Debug, Serialize)]
struct Report {
    schema_version: String,
    fixture_id: String,
    as_of: String,
    synthetic_only: bool,
    status: String,
    component_count: usize,
    recovery_path_count: usize,
    scenario_count: usize,
    unique_failure_domain_count: usize,
    pairwise_independence_passed: bool,
    single_domain_faults_tested: usize,
    single_domain_minimum_paths_observed: usize,
    scenario_results: Vec<ScenarioResult>,
    claim_ceiling: String,
}

fn unique(values: &[String]) -> bool {
    values.iter().collect::<HashSet<_>>().len() == values.len()
}
fn sorted(values: impl IntoIterator<Item = String>) -> Vec<String> {
    values.into_iter().collect::<BTreeSet<_>>().into_iter().collect()
}
fn load_topology(input: &str) -> Result<Topology, String> {
    serde_json::from_str(input).map_err(|e| format!("invalid topology JSON: {e}"))
}

fn validate(t: &Topology) -> Result<Report, String> {
    if !t.synthetic_only { return Err("fixture must remain synthetic_only".into()); }
    if t.components.is_empty() || t.recovery_paths.is_empty() || t.scenarios.is_empty() {
        return Err("components, paths and scenarios must be nonempty".into());
    }
    if t.minimum_surviving_paths_after_any_single_domain_failure == 0
        || !unique(&t.required_component_kinds)
        || !unique(&t.independence_critical_kinds)
        || t.site_evidence_admission_requirements.is_empty() {
        return Err("invalid minima, duplicate component kinds or missing evidence requirements".into());
    }

    let mut components: HashMap<&str, &Component> = HashMap::new();
    let mut all_domains = BTreeSet::new();
    for c in &t.components {
        if c.component_id.trim().is_empty() || c.component_kind.trim().is_empty()
            || c.failure_domains.is_empty() || !unique(&c.failure_domains) {
            return Err(format!("component {} has missing or duplicate identity/domain data", c.component_id));
        }
        if c.evidence_state != "synthetic_design_assumption" || c.as_built_independence_verified {
            return Err(format!("component {} overstates evidence", c.component_id));
        }
        for d in &c.failure_domains {
            if d.trim().is_empty() { return Err(format!("component {} has an empty failure domain", c.component_id)); }
            all_domains.insert(d.clone());
        }
        if components.insert(&c.component_id, c).is_some() {
            return Err(format!("duplicate component_id {}", c.component_id));
        }
    }

    let mut paths: HashMap<&str, &RecoveryPath> = HashMap::new();
    let mut path_domains: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for p in &t.recovery_paths {
        if p.path_id.trim().is_empty() || p.component_ids.is_empty()
            || !unique(&p.component_ids) || p.targets_are_measured {
            return Err(format!("path {} has invalid identity or claims measured targets", p.path_id));
        }
        if paths.insert(&p.path_id, p).is_some() {
            return Err(format!("duplicate recovery path {}", p.path_id));
        }
        let mut kinds = HashSet::new();
        let mut domains = BTreeSet::new();
        for id in &p.component_ids {
            let c = components.get(id.as_str())
                .ok_or_else(|| format!("path {} references unknown component {}", p.path_id, id))?;
            if !kinds.insert(c.component_kind.as_str()) {
                return Err(format!("path {} has duplicate component kind {}", p.path_id, c.component_kind));
            }
            domains.extend(c.failure_domains.iter().cloned());
        }
        for required in &t.required_component_kinds {
            if !kinds.contains(required.as_str()) {
                return Err(format!("path {} is missing component kind {}", p.path_id, required));
            }
        }
        for kind in &kinds {
            if !t.required_component_kinds.iter().any(|k| k == kind) {
                return Err(format!("path {} contains undeclared component kind {}", p.path_id, kind));
            }
        }
        path_domains.insert(p.path_id.clone(), domains);
    }

    // A redundancy label is not accepted as proof. Check critical failure-domain
    // identities across every pair of recovery paths.
    for (i, first) in t.recovery_paths.iter().enumerate() {
        for second in t.recovery_paths.iter().skip(i + 1) {
            for id in &first.component_ids {
                let a = components[id.as_str()];
                if !t.independence_critical_kinds.contains(&a.component_kind) { continue; }
                for domain in &a.failure_domains {
                    for other_id in &second.component_ids {
                        let b = components[other_id.as_str()];
                        if t.independence_critical_kinds.contains(&b.component_kind)
                            && b.failure_domains.contains(domain) {
                            return Err(format!("critical common-cause domain {domain} shared by {} and {}",
                                first.path_id, second.path_id));
                        }
                    }
                }
            }
        }
    }

    // Exhaustively remove every single declared failure domain.
    let mut min_after_single = usize::MAX;
    for failed in &all_domains {
        let remaining = t.recovery_paths.iter().filter(|p|
            path_domains[&p.path_id].iter().all(|domain| domain != failed)
        ).count();
        min_after_single = min_after_single.min(remaining);
        if remaining < t.minimum_surviving_paths_after_any_single_domain_failure {
            return Err(format!("single failure of {failed} leaves {remaining} paths; minimum is {}",
                t.minimum_surviving_paths_after_any_single_domain_failure));
        }
    }

    let known: HashSet<&str> = paths.keys().copied().collect();
    let mut scenario_ids = HashSet::new();
    let mut results = Vec::new();
    for s in &t.scenarios {
        if s.scenario_id.trim().is_empty() || !scenario_ids.insert(s.scenario_id.as_str())
            || !unique(&s.failed_domains) || !unique(&s.expected_infrastructure_paths)
            || !unique(&s.service_blocked_paths) {
            return Err(format!("scenario {} has duplicate or invalid fields", s.scenario_id));
        }
        for id in s.expected_infrastructure_paths.iter().chain(s.service_blocked_paths.iter()) {
            if !known.contains(id.as_str()) {
                return Err(format!("scenario {} references unknown path {id}", s.scenario_id));
            }
        }
        let blocked: HashSet<&str> = s.service_blocked_paths.iter().map(String::as_str).collect();
        let actual_infrastructure = sorted(t.recovery_paths.iter().filter(|p| {
            path_domains[&p.path_id].iter().all(|d| !s.failed_domains.contains(d))
        }).map(|p| p.path_id.clone()));
        let expected_infrastructure = sorted(s.expected_infrastructure_paths.clone());
        let actual_service = sorted(actual_infrastructure.iter()
            .filter(|id| !blocked.contains(id.as_str())).cloned());
        let expected_service = sorted(s.expected_infrastructure_paths.iter()
            .filter(|id| !blocked.contains(id.as_str())).cloned());
        if actual_infrastructure != expected_infrastructure || actual_service != expected_service {
            return Err(format!("scenario {} mismatch: infrastructure {:?} != {:?}; service {:?} != {:?}",
                s.scenario_id, actual_infrastructure, expected_infrastructure, actual_service, expected_service));
        }
        if actual_infrastructure.len() < s.minimum_infrastructure_paths {
            return Err(format!("scenario {} leaves too few infrastructure paths", s.scenario_id));
        }
        results.push(ScenarioResult {
            scenario_id:s.scenario_id.clone(), title:s.title.clone(),
            infrastructure_paths:actual_infrastructure, service_paths:actual_service,
            passed:true, disposition:s.disposition.clone()
        });
    }

    Ok(Report {
        schema_version:t.schema_version.clone(), fixture_id:t.fixture_id.clone(), as_of:t.as_of.clone(),
        synthetic_only:t.synthetic_only,
        status:"STRUCTURALLY_VALID_SYNTHETIC_REFERENCE_NOT_SITE_PROOF".into(),
        component_count:t.components.len(), recovery_path_count:t.recovery_paths.len(),
        scenario_count:results.len(), unique_failure_domain_count:all_domains.len(),
        pairwise_independence_passed:true, single_domain_faults_tested:all_domains.len(),
        single_domain_minimum_paths_observed:min_after_single, scenario_results:results,
        claim_ceiling:t.claim_ceiling.clone()
    })
}

fn run(path: &Path) -> Result<Report, String> {
    let input = fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    validate(&load_topology(&input)?)
}
fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| DEFAULT_FIXTURE.into());
    match run(Path::new(&path)) {
        Ok(report) => match serde_json::to_string_pretty(&report) {
            Ok(json) => println!("{json}"),
            Err(e) => { eprintln!("cannot encode verifier report: {e}"); std::process::exit(2); }
        },
        Err(e) => { eprintln!("RECOVERY_TOPOLOGY_REJECTED: {e}"); std::process::exit(1); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Topology {
        load_topology(include_str!(
            "../../../docs/research/port-energy-compute/OCEAN_RECOVERY_FAILURE_DOMAIN_GRAPH_V0_1.json"
        )).expect("fixture JSON must parse")
    }
    #[test]
    fn all_single_domain_failures_leave_two_paths() {
        let report = validate(&fixture()).expect("reference topology must validate");
        assert_eq!(report.recovery_path_count, 3);
        assert_eq!(report.scenario_count, 14);
        assert_eq!(report.single_domain_faults_tested, report.unique_failure_domain_count);
        assert_eq!(report.single_domain_minimum_paths_observed, 2);
    }
    #[test]
    fn shared_identity_domain_is_rejected() {
        let mut t = fixture();
        let c = t.components.iter_mut().find(|c| c.component_id == "warm_identity").unwrap();
        c.failure_domains = vec!["identity:port-local-issuer".into()];
        assert!(validate(&t).unwrap_err().contains("critical common-cause domain"));
    }
    #[test]
    fn production_iam_compromise_does_not_cut_recovery_paths() {
        let t = fixture();
        let s = t.scenarios.iter().find(|s| s.scenario_id == "RT-002").unwrap();
        assert_eq!(s.expected_infrastructure_paths.len(), 3);
        assert!(s.failed_domains.iter().all(|domain|
            !t.components.iter().any(|c| c.failure_domains.contains(domain))));
        validate(&t).expect("production identity is outside recovery domains");
    }
    #[test]
    fn integrity_failure_blocks_service_but_not_recovery_infrastructure() {
        let report = validate(&fixture()).expect("integrity scenario must validate");
        let s = report.scenario_results.iter().find(|s| s.scenario_id == "RT-012").unwrap();
        assert_eq!(s.infrastructure_paths.len(), 3);
        assert!(s.service_paths.is_empty());
    }
}
