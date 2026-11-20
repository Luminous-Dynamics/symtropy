use serde::{Deserialize, Serialize};
use std::{collections::{BTreeMap, BTreeSet}, env, fs, process};

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkloadFixture {
    schema_version: String,
    fixture_id: String,
    as_of: String,
    synthetic_only: bool,
    claim_ceiling: String,
    energy_accounting_boundary: String,
    units: WorkloadUnits,
    profiles: Vec<WorkloadProfile>,
    scenarios: Vec<WorkloadScenario>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkloadUnits {
    energy: String,
    input_volume: String,
    time: String,
    service_unit: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkloadProfile {
    id: String,
    service_family: String,
    service_unit: String,
    service_units_per_batch: u64,
    synthetic_input_bytes: u64,
    latency_budget_seconds: u64,
    deadline_seconds: u64,
    maximum_deferral_seconds: u64,
    flexibility_class: String,
    allowed_placement_strata: Vec<String>,
    local_fallback_required: bool,
    safety_critical_control_dependency: bool,
    rights_state: String,
    energy_wh: ProfileEnergy,
    synthetic_quality_gate: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileEnergy {
    compute: f64,
    network: f64,
    cooling: f64,
    total: f64,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkloadScenario {
    id: String,
    fault_or_stressor: String,
    expected_disposition: String,
    required_assertions: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PlacementFixture {
    schema_version: String,
    fixture_id: String,
    as_of: String,
    synthetic_only: bool,
    claim_ceiling: String,
    measurement_boundary: String,
    units: PlacementUnits,
    candidates: Vec<PlacementCandidate>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PlacementUnits {
    energy: String,
    power: String,
    latency: String,
    completion: String,
    carbon_factor: String,
    electricity_cost: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PlacementCandidate {
    profile_id: String,
    candidate_id: String,
    placement_stratum: String,
    candidate_available: bool,
    source_rights_allowed: bool,
    data_egress_required: bool,
    data_egress_allowed: bool,
    quality_gate_pass: bool,
    accepted_service_units: u64,
    p99_latency_seconds: u64,
    completion_seconds: u64,
    peak_power_w: u64,
    pcc_import_cap_w: u64,
    local_fallback_ready: bool,
    remote_safety_control_dependency: bool,
    energy_wh: Energy,
    carbon_intensity_gco2e_per_kwh: u64,
    electricity_cost_micro_usd_per_kwh: u64,
    synthetic_estimate: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Energy {
    compute: u64,
    network: u64,
    cooling: u64,
    facility_auxiliary: u64,
    total: u64,
}

#[derive(Debug, Serialize)]
struct Report {
    report_schema: &'static str,
    source_profile_fixture: String,
    source_placement_fixture: String,
    synthetic_only: bool,
    claim_ceiling: &'static str,
    profile_results: Vec<ProfileResult>,
}

#[derive(Debug, Serialize)]
struct ProfileResult {
    profile_id: String,
    expected_service_units: u64,
    eligible_candidate_count: usize,
    candidate_results: Vec<CandidateResult>,
    pareto_candidate_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CandidateResult {
    candidate_id: String,
    placement_stratum: String,
    eligible: bool,
    blockers: Vec<String>,
    energy_wh_per_service_unit: Option<f64>,
    carbon_g_per_service_unit: Option<f64>,
    cost_micro_usd_per_service_unit: Option<f64>,
    p99_latency_seconds: u64,
    completion_seconds: u64,
}

#[derive(Debug)]
struct Scored {
    result: CandidateResult,
    energy: f64,
    carbon: f64,
    cost: f64,
}

fn fail(message: impl AsRef<str>) -> ! {
    eprintln!("ocean-compute-placement-verifier: {}", message.as_ref());
    process::exit(2);
}

fn parse<T: for<'de> Deserialize<'de>>(path: &str) -> T {
    let bytes = fs::read(path).unwrap_or_else(|e| fail(format!("cannot read {path}: {e}")));
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|e| fail(format!("invalid JSON in {path}: {e}")))
}

fn validate_fixture_identity(workloads: &WorkloadFixture, placements: &PlacementFixture) {
    if !workloads.synthetic_only || !placements.synthetic_only {
        fail("v0.1 accepts synthetic-only fixtures; measured evidence needs a separate admission profile");
    }
    if workloads.schema_version.is_empty() || placements.schema_version.is_empty() {
        fail("schema_version must be non-empty");
    }
    if workloads.fixture_id.is_empty() || placements.fixture_id.is_empty() {
        fail("fixture_id must be non-empty");
    }
    if workloads.profiles.is_empty() || placements.candidates.is_empty() {
        fail("profiles and candidate lists must be non-empty");
    }
}

fn profile_index<'a>(workloads: &'a WorkloadFixture) -> BTreeMap<&'a str, &'a WorkloadProfile> {
    let mut index = BTreeMap::new();
    for profile in &workloads.profiles {
        if profile.id.is_empty() || profile.service_units_per_batch == 0 {
            fail("profile id and service_units_per_batch must be non-empty/positive");
        }
        if profile.deadline_seconds == 0 || profile.latency_budget_seconds == 0 {
            fail(format!("profile {} has a zero time budget", profile.id));
        }
        if profile.allowed_placement_strata.is_empty() {
            fail(format!("profile {} has no allowed placement strata", profile.id));
        }
        let unique: BTreeSet<&str> = profile.allowed_placement_strata.iter().map(String::as_str).collect();
        if unique.len() != profile.allowed_placement_strata.len() {
            fail(format!("profile {} repeats a placement stratum", profile.id));
        }
        if !profile_energy_is_closed(&profile.energy_wh) {
            fail(format!("profile {} baseline energy components do not close", profile.id));
        }
        if profile.rights_state != "synthetic_only" {
            fail(format!("profile {} is not admitted as synthetic-only by v0.1", profile.id));
        }
        if index.insert(profile.id.as_str(), profile).is_some() {
            fail(format!("duplicate profile id: {}", profile.id));
        }
    }
    index
}

fn profile_energy_is_closed(energy: &ProfileEnergy) -> bool {
    let total = energy.compute + energy.network + energy.cooling;
    (total - energy.total).abs() <= 1e-9
}

fn energy_is_closed(energy: &Energy) -> bool {
    energy.compute
        .checked_add(energy.network)
        .and_then(|v| v.checked_add(energy.cooling))
        .and_then(|v| v.checked_add(energy.facility_auxiliary))
        == Some(energy.total)
}

fn dominates(a: &Scored, b: &Scored) -> bool {
    let no_worse = a.energy <= b.energy
        && a.carbon <= b.carbon
        && a.cost <= b.cost
        && a.result.p99_latency_seconds <= b.result.p99_latency_seconds
        && a.result.completion_seconds <= b.result.completion_seconds;
    let strictly_better = a.energy < b.energy
        || a.carbon < b.carbon
        || a.cost < b.cost
        || a.result.p99_latency_seconds < b.result.p99_latency_seconds
        || a.result.completion_seconds < b.result.completion_seconds;
    no_worse && strictly_better
}

fn score_candidate(profile: &WorkloadProfile, candidate: &PlacementCandidate) -> Scored {
    let mut blockers = Vec::new();

    if !profile.allowed_placement_strata.iter().any(|s| s == &candidate.placement_stratum) {
        blockers.push("placement_not_allowed_by_profile".to_string());
    }
    if !candidate.candidate_available {
        blockers.push("candidate_unavailable".to_string());
    }
    if !candidate.source_rights_allowed {
        blockers.push("source_rights_not_admitted".to_string());
    }
    if candidate.data_egress_required && !candidate.data_egress_allowed {
        blockers.push("data_egress_not_authorized".to_string());
    }
    if !candidate.quality_gate_pass {
        blockers.push("service_quality_gate_failed".to_string());
    }
    if candidate.accepted_service_units != profile.service_units_per_batch {
        blockers.push("service_equivalence_not_preserved".to_string());
    }
    if candidate.p99_latency_seconds > profile.latency_budget_seconds {
        blockers.push("p99_latency_budget_exceeded".to_string());
    }
    if candidate.completion_seconds > profile.deadline_seconds {
        blockers.push("service_deadline_exceeded".to_string());
    }
    if candidate.peak_power_w > candidate.pcc_import_cap_w {
        blockers.push("pcc_import_cap_exceeded".to_string());
    }
    if profile.local_fallback_required && !candidate.local_fallback_ready {
        blockers.push("required_local_fallback_unavailable".to_string());
    }
    if profile.safety_critical_control_dependency || candidate.remote_safety_control_dependency {
        blockers.push("remote_safety_control_dependency_forbidden".to_string());
    }
    if !candidate.synthetic_estimate {
        blockers.push("non_synthetic_candidate_not_admitted_by_v0_1".to_string());
    }
    if !energy_is_closed(&candidate.energy) {
        blockers.push("facility_energy_balance_not_closed".to_string());
    }

    let eligible = blockers.is_empty();
    let units = profile.service_units_per_batch as f64;
    let total_wh = candidate.energy.total as f64;
    let energy_per_unit = total_wh / units;
    let carbon_g_per_unit =
        (total_wh / 1000.0) * candidate.carbon_intensity_gco2e_per_kwh as f64 / units;
    let cost_micro_usd_per_unit =
        (total_wh / 1000.0) * candidate.electricity_cost_micro_usd_per_kwh as f64 / units;

    Scored {
        result: CandidateResult {
            candidate_id: candidate.candidate_id.clone(),
            placement_stratum: candidate.placement_stratum.clone(),
            eligible,
            blockers,
            energy_wh_per_service_unit: eligible.then_some(energy_per_unit),
            carbon_g_per_service_unit: eligible.then_some(carbon_g_per_unit),
            cost_micro_usd_per_service_unit: eligible.then_some(cost_micro_usd_per_unit),
            p99_latency_seconds: candidate.p99_latency_seconds,
            completion_seconds: candidate.completion_seconds,
        },
        energy: energy_per_unit,
        carbon: carbon_g_per_unit,
        cost: cost_micro_usd_per_unit,
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let profile_path = args.get(1).map(String::as_str)
        .unwrap_or("docs/research/port-energy-compute/OCEAN_COMPUTE_WORKLOAD_PROFILES_V0_1.json");
    let placement_path = args.get(2).map(String::as_str)
        .unwrap_or("docs/research/port-energy-compute/OCEAN_COMPUTE_PLACEMENT_MATRIX_V0_1.json");

    let workloads: WorkloadFixture = parse(profile_path);
    let placements: PlacementFixture = parse(placement_path);
    validate_fixture_identity(&workloads, &placements);
    let profiles = profile_index(&workloads);

    let mut seen_candidates = BTreeSet::new();
    let mut by_profile: BTreeMap<&str, Vec<Scored>> = BTreeMap::new();

    for candidate in &placements.candidates {
        if candidate.candidate_id.is_empty() || !seen_candidates.insert(candidate.candidate_id.as_str()) {
            fail(format!("empty or duplicate candidate_id: {}", candidate.candidate_id));
        }
        let profile = profiles.get(candidate.profile_id.as_str())
            .unwrap_or_else(|| fail(format!("unknown profile_id: {}", candidate.profile_id)));
        if candidate.pcc_import_cap_w == 0 {
            fail(format!("candidate {} declares zero PCC cap", candidate.candidate_id));
        }
        let scored = score_candidate(profile, candidate);
        by_profile.entry(profile.id.as_str()).or_default().push(scored);
    }

    let mut profile_results = Vec::new();
    for profile in &workloads.profiles {
        let mut scored = by_profile.remove(profile.id.as_str()).unwrap_or_default();
        scored.sort_by(|a, b| a.result.candidate_id.cmp(&b.result.candidate_id));

        let eligible: Vec<usize> = scored.iter().enumerate()
            .filter_map(|(i, s)| s.result.eligible.then_some(i))
            .collect();

        let mut pareto_candidate_ids = Vec::new();
        for &i in &eligible {
            let dominated = eligible.iter().copied().any(|j| j != i && dominates(&scored[j], &scored[i]));
            if !dominated {
                pareto_candidate_ids.push(scored[i].result.candidate_id.clone());
            }
        }

        let eligible_candidate_count = eligible.len();
        profile_results.push(ProfileResult {
            profile_id: profile.id.clone(),
            expected_service_units: profile.service_units_per_batch,
            eligible_candidate_count,
            candidate_results: scored.into_iter().map(|s| s.result).collect(),
            pareto_candidate_ids,
        });
    }

    let report = Report {
        report_schema: "ocean-compute-placement-report-v0.1",
        source_profile_fixture: workloads.fixture_id,
        source_placement_fixture: placements.fixture_id,
        synthetic_only: true,
        claim_ceiling: "Synthetic contract comparison only; not site sizing, field performance, lifecycle assessment, environmental approval, grid connection or SOTA proof.",
        profile_results,
    };
    println!("{}", serde_json::to_string_pretty(&report).expect("report serialization"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn energy_balance_requires_all_four_terms() {
        let e = Energy { compute: 10, network: 2, cooling: 3, facility_auxiliary: 1, total: 16 };
        assert!(energy_is_closed(&e));
        let bad = Energy { total: 15, ..e };
        assert!(!energy_is_closed(&bad));
    }

    #[test]
    fn fractional_profile_energy_is_preserved_and_closed() {
        let e = ProfileEnergy { compute: 350.0, network: 35.0, cooling: 52.5, total: 437.5 };
        assert!(profile_energy_is_closed(&e));
        let bad = ProfileEnergy { total: 437.0, ..e };
        assert!(!profile_energy_is_closed(&bad));
    }

    fn sample_profile() -> WorkloadProfile {
        WorkloadProfile {
            id: "demo".to_string(),
            service_family: "test".to_string(),
            service_unit: "accepted result".to_string(),
            service_units_per_batch: 2,
            synthetic_input_bytes: 10,
            latency_budget_seconds: 60,
            deadline_seconds: 120,
            maximum_deferral_seconds: 0,
            flexibility_class: "non_deferrable".to_string(),
            allowed_placement_strata: vec!["port_edge".to_string()],
            local_fallback_required: true,
            safety_critical_control_dependency: false,
            rights_state: "synthetic_only".to_string(),
            energy_wh: ProfileEnergy { compute: 10.0, network: 2.0, cooling: 3.0, total: 15.0 },
            synthetic_quality_gate: "test".to_string(),
        }
    }

    fn sample_candidate() -> PlacementCandidate {
        PlacementCandidate {
            profile_id: "demo".to_string(),
            candidate_id: "demo:port".to_string(),
            placement_stratum: "port_edge".to_string(),
            candidate_available: true,
            source_rights_allowed: true,
            data_egress_required: true,
            data_egress_allowed: true,
            quality_gate_pass: true,
            accepted_service_units: 2,
            p99_latency_seconds: 10,
            completion_seconds: 20,
            peak_power_w: 500,
            pcc_import_cap_w: 1000,
            local_fallback_ready: true,
            remote_safety_control_dependency: false,
            energy_wh: Energy { compute: 10, network: 2, cooling: 3, facility_auxiliary: 1, total: 16 },
            carbon_intensity_gco2e_per_kwh: 200,
            electricity_cost_micro_usd_per_kwh: 100000,
            synthetic_estimate: true,
        }
    }

    #[test]
    fn rights_egress_pcc_and_fallback_are_non_compensatory() {
        let profile = sample_profile();
        let mut candidate = sample_candidate();
        candidate.source_rights_allowed = false;
        candidate.data_egress_allowed = false;
        candidate.peak_power_w = 2000;
        candidate.local_fallback_ready = false;

        let scored = score_candidate(&profile, &candidate);
        assert!(!scored.result.eligible);
        assert!(scored.result.blockers.contains(&"source_rights_not_admitted".to_string()));
        assert!(scored.result.blockers.contains(&"data_egress_not_authorized".to_string()));
        assert!(scored.result.blockers.contains(&"pcc_import_cap_exceeded".to_string()));
        assert!(scored.result.blockers.contains(&"required_local_fallback_unavailable".to_string()));
        assert!(scored.result.energy_wh_per_service_unit.is_none());
    }

    #[test]
    fn pareto_dominance_requires_no_worse_on_every_dimension() {
        let mk = |id: &str, energy: f64, carbon: f64, cost: f64, latency: u64, completion: u64| Scored {
            result: CandidateResult {
                candidate_id: id.to_string(), placement_stratum: "test".to_string(),
                eligible: true, blockers: Vec::new(),
                energy_wh_per_service_unit: Some(energy),
                carbon_g_per_service_unit: Some(carbon),
                cost_micro_usd_per_service_unit: Some(cost),
                p99_latency_seconds: latency, completion_seconds: completion,
            },
            energy, carbon, cost,
        };
        let a = mk("a", 1.0, 1.0, 1.0, 10, 10);
        let b = mk("b", 2.0, 2.0, 2.0, 20, 20);
        let c = mk("c", 0.5, 3.0, 0.5, 5, 5);
        assert!(dominates(&a, &b));
        assert!(!dominates(&c, &b));
        assert!(!dominates(&a, &a));
    }
