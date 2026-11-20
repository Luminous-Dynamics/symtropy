use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::{env, fs, path::Path};

const CONTRACT_PATH: &str =
    "docs/research/port-energy-compute/OCEAN_PRODUCTIVE_RESERVE_CONTRACT_V0_1.json";
const WORKLOAD_PATH: &str =
    "docs/research/port-energy-compute/OCEAN_COMPUTE_WORKLOAD_PROFILES_V0_1.json";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Contract {
    schema_version: String,
    fixture_id: String,
    as_of: String,
    synthetic_only: bool,
    claim_ceiling: String,
    referenced_workload_fixture: String,
    reserve_site_path: String,
    reserve_capacity: ReserveCapacity,
    protected_recovery_services: Vec<ProtectedService>,
    background_preemptible_jobs: Vec<BackgroundJob>,
    recovery_budget: RecoveryBudget,
    scenarios: Vec<Scenario>,
    acceptance_rules: Vec<String>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReserveCapacity {
    resource_unit: String,
    capacity: Resources,
    headroom_policy: String,
    no_double_counting: bool,
}
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Resources {
    compute: u64,
    memory: u64,
    network: u64,
    cooling: u64,
    power: u64,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProtectedService {
    profile_id: String,
    priority: String,
    resource_demand: Resources,
    service_contract: ServiceContract,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ServiceContract {
    max_start_deferral_seconds: u64,
    latency_budget_seconds: u64,
    local_fallback_required: bool,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BackgroundJob {
    profile_id: String,
    priority: String,
    resource_demand: Resources,
    preemption: Preemption,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Preemption {
    max_quiesce_seconds: u64,
    max_checkpoint_seconds: u64,
    checkpoint_required: bool,
    checkpoint_integrity_required: bool,
    maximum_deferral_seconds: u64,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecoveryBudget {
    target_rto_seconds: u64,
    sequential_budget_steps: Vec<BudgetStep>,
    arithmetic_only_expected_total_seconds: u64,
    target_is_measured: bool,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BudgetStep {
    step: String,
    budget_seconds: u64,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    scenario_id: String,
    title: String,
    fault: String,
    expected: String,
    required_assertions: Vec<String>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkloadFixture {
    schema_version: String,
    fixture_id: String,
    as_of: String,
    synthetic_only: bool,
    claim_ceiling: String,
    energy_accounting_boundary: String,
    units: serde_json::Value,
    profiles: Vec<Profile>,
    scenarios: Vec<serde_json::Value>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Profile {
    id: String,
    maximum_deferral_seconds: u64,
    latency_budget_seconds: u64,
    local_fallback_required: bool,
    safety_critical_control_dependency: bool,
}
#[derive(Debug, Serialize)]
struct Report {
    schema_version: String,
    fixture_id: String,
    synthetic_only: bool,
    status: String,
    protected_services: usize,
    background_jobs: usize,
    normal_background_load: Resources,
    protected_recovery_load: Resources,
    background_plus_recovery_exceeds_capacity: bool,
    worst_quiesce_plus_checkpoint_seconds: u64,
    recovery_budget_total_seconds: u64,
    recovery_rto_target_seconds: u64,
    scenarios_validated: usize,
    scenario_ids: Vec<String>,
    claim_ceiling: String,
}

fn sum_resources<I: Iterator<Item = Resources>>(items: I) -> Resources {
    items.fold(Resources { compute:0, memory:0, network:0, cooling:0, power:0 }, |a,b| Resources {
        compute:a.compute+b.compute, memory:a.memory+b.memory,
        network:a.network+b.network, cooling:a.cooling+b.cooling, power:a.power+b.power
    })
}
fn fits(load: Resources, capacity: Resources) -> bool {
    load.compute <= capacity.compute && load.memory <= capacity.memory
        && load.network <= capacity.network && load.cooling <= capacity.cooling
        && load.power <= capacity.power
}
fn exceeds(a: Resources, b: Resources) -> bool {
    a.compute > b.compute || a.memory > b.memory || a.network > b.network
        || a.cooling > b.cooling || a.power > b.power
}
fn validate(c: &Contract, w: &WorkloadFixture) -> Result<Report, String> {
    if !c.synthetic_only || !w.synthetic_only || c.recovery_budget.target_is_measured
        || c.as_of.trim().is_empty() {
        return Err("fixtures and target status must remain explicitly synthetic/unmeasured".into());
    }
    if c.reserve_site_path != "warm_recovery"
        || c.reserve_capacity.resource_unit.trim().is_empty()
        || c.reserve_capacity.headroom_policy.trim().is_empty()
        || w.schema_version.trim().is_empty() || w.as_of.trim().is_empty()
        || w.claim_ceiling.trim().is_empty() || w.energy_accounting_boundary.trim().is_empty()
        || w.units.is_null() || w.scenarios.is_empty()
        || !c.reserve_capacity.no_double_counting || c.acceptance_rules.is_empty()
        || c.protected_recovery_services.is_empty() || c.background_preemptible_jobs.is_empty()
        || c.scenarios.is_empty() {
        return Err("reserve contract missing nonempty resources, jobs, scenarios or no-double-counting rule".into());
    }
    if c.referenced_workload_fixture != w.fixture_id {
        return Err("contract references a different workload fixture".into());
    }

    let mut profiles = HashMap::new();
    for p in &w.profiles {
        if profiles.insert(p.id.as_str(), p).is_some() {
            return Err(format!("duplicate workload profile {}", p.id));
        }
    }
    let mut seen_jobs = HashSet::new();
    for s in &c.protected_recovery_services {
        if !seen_jobs.insert(s.profile_id.as_str()) {
            return Err(format!("duplicate protected service {}", s.profile_id));
        }
        let p = profiles.get(s.profile_id.as_str())
            .ok_or_else(|| format!("unknown protected profile {}", s.profile_id))?;
        if p.safety_critical_control_dependency
            || p.local_fallback_required != s.service_contract.local_fallback_required
            || !s.service_contract.local_fallback_required
            || s.service_contract.max_start_deferral_seconds > p.maximum_deferral_seconds
            || s.service_contract.latency_budget_seconds > p.latency_budget_seconds {
            return Err(format!("protected service {} violates source workload contract", s.profile_id));
        }
    }

    let mut worst_preemption = 0;
    let mut background_ids = HashSet::new();
    for j in &c.background_preemptible_jobs {
        if !background_ids.insert(j.profile_id.as_str()) || !seen_jobs.insert(j.profile_id.as_str()) {
            return Err(format!("duplicate or overlapping background job {}", j.profile_id));
        }
        let p = profiles.get(j.profile_id.as_str())
            .ok_or_else(|| format!("unknown background profile {}", j.profile_id))?;
        if p.safety_critical_control_dependency || !j.preemption.checkpoint_required
            || !j.preemption.checkpoint_integrity_required
            || j.preemption.maximum_deferral_seconds > p.maximum_deferral_seconds {
            return Err(format!("background job {} has unsafe or unbounded preemption contract", j.profile_id));
        }
        worst_preemption = worst_preemption.max(
            j.preemption.max_quiesce_seconds + j.preemption.max_checkpoint_seconds
        );
    }

    let capacity = c.reserve_capacity.capacity;
    let protected = sum_resources(c.protected_recovery_services.iter().map(|x| x.resource_demand));
    let background = sum_resources(c.background_preemptible_jobs.iter().map(|x| x.resource_demand));
    if !fits(protected, capacity) {
        return Err("protected recovery workload exceeds at least one reserve resource ceiling".into());
    }
    if !fits(background, capacity) {
        return Err("normal background workload exceeds at least one reserve resource ceiling".into());
    }
    if !exceeds(Resources {
        compute:protected.compute+background.compute,
        memory:protected.memory+background.memory,
        network:protected.network+background.network,
        cooling:protected.cooling+background.cooling,
        power:protected.power+background.power
    }, capacity) {
        return Err("fixture does not exercise a real preemption/headroom conflict".into());
    }

    let budget_total: u64 = c.recovery_budget.sequential_budget_steps.iter().map(|s| s.budget_seconds).sum();
    if budget_total != c.recovery_budget.arithmetic_only_expected_total_seconds
        || budget_total > c.recovery_budget.target_rto_seconds {
        return Err("recovery budget arithmetic exceeds target or declared total".into());
    }
    let preempt_step = c.recovery_budget.sequential_budget_steps.iter()
        .find(|s| s.step == "preempt_and_checkpoint_background_jobs")
        .ok_or_else(|| "missing preempt/checkpoint budget step".to_string())?;
    if worst_preemption > preempt_step.budget_seconds {
        return Err(format!("worst quiesce+checkpoint {}s exceeds {}s recovery budget",
            worst_preemption, preempt_step.budget_seconds));
    }

    let required_scenarios: [(&str, &str);6] = [
        ("PR-001","ADMIT"),
        ("PR-002","PREEMPT_THEN_RESTORE"),
        ("PR-003","REJECT_RESERVE_CLAIM"),
        ("PR-004","QUARANTINE_BACKGROUND_JOB"),
        ("PR-005","DEFER_BACKGROUND_RESUME"),
        ("PR-006","REJECT_INFEASIBLE_RTO")
    ];
    let mut scenario_ids = Vec::new();
    let mut seen_scenarios = HashSet::new();
    for s in &c.scenarios {
        if !seen_scenarios.insert(s.scenario_id.as_str()) || s.title.trim().is_empty()
            || s.fault.trim().is_empty() || s.required_assertions.is_empty() {
            return Err(format!("invalid or duplicate scenario {}", s.scenario_id));
        }
        scenario_ids.push(s.scenario_id.clone());
    }
    for (id, expected) in required_scenarios {
        let s = c.scenarios.iter().find(|s| s.scenario_id == id)
            .ok_or_else(|| format!("missing required scenario {id}"))?;
        if s.expected != expected {
            return Err(format!("scenario {id} has unexpected disposition"));
        }
    }
    scenario_ids.sort();

    Ok(Report {
        schema_version:c.schema_version.clone(), fixture_id:c.fixture_id.clone(),
        synthetic_only:c.synthetic_only,
        status:"STRUCTURALLY_VALID_SYNTHETIC_RESERVE_CONTRACT_NOT_CAPACITY_PROOF".into(),
        protected_services:c.protected_recovery_services.len(),
        background_jobs:c.background_preemptible_jobs.len(),
        normal_background_load:background,
        protected_recovery_load:protected,
        background_plus_recovery_exceeds_capacity:true,
        worst_quiesce_plus_checkpoint_seconds:worst_preemption,
        recovery_budget_total_seconds:budget_total,
        recovery_rto_target_seconds:c.recovery_budget.target_rto_seconds,
        scenarios_validated:c.scenarios.len(), scenario_ids,
        claim_ceiling:c.claim_ceiling.clone()
    })
}

fn load<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let input = fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    serde_json::from_str(&input).map_err(|e| format!("invalid JSON {}: {e}", path.display()))
}
fn main() {
    let mut args = env::args().skip(1);
    let contract_path = args.next().unwrap_or_else(|| CONTRACT_PATH.into());
    let workload_path = args.next().unwrap_or_else(|| WORKLOAD_PATH.into());
    let result = (|| {
        let contract: Contract = load(Path::new(&contract_path))?;
        let workload: WorkloadFixture = load(Path::new(&workload_path))?;
        validate(&contract, &workload)
    })();
    match result {
        Ok(report) => match serde_json::to_string_pretty(&report) {
            Ok(json) => println!("{json}"),
            Err(e) => { eprintln!("cannot encode reserve report: {e}"); std::process::exit(2); }
        },
        Err(e) => { eprintln!("PRODUCTIVE_RESERVE_REJECTED: {e}"); std::process::exit(1); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixtures() -> (Contract, WorkloadFixture) {
        let c: Contract = serde_json::from_str(include_str!(
            "../../../docs/research/port-energy-compute/OCEAN_PRODUCTIVE_RESERVE_CONTRACT_V0_1.json"
        )).expect("reserve contract fixture parses");
        let w: WorkloadFixture = serde_json::from_str(include_str!(
            "../../../docs/research/port-energy-compute/OCEAN_COMPUTE_WORKLOAD_PROFILES_V0_1.json"
        )).expect("workload fixture parses");
        (c,w)
    }
    #[test]
    fn background_load_is_preempted_before_protected_service() {
        let (c,w)=fixtures();
        let r=validate(&c,&w).expect("synthetic reserve contract validates");
        assert!(fits(r.protected_recovery_load,c.reserve_capacity.capacity));
        assert!(fits(r.normal_background_load,c.reserve_capacity.capacity));
        assert!(r.background_plus_recovery_exceeds_capacity);
        assert_eq!(r.worst_quiesce_plus_checkpoint_seconds,480);
        assert_eq!(r.recovery_budget_total_seconds,6000);
    }
    #[test]
    fn protected_resource_overcommit_is_rejected() {
        let (mut c,w)=fixtures();
        c.protected_recovery_services[0].resource_demand.compute=110;
        assert!(validate(&c,&w).unwrap_err().contains("protected recovery workload exceeds"));
    }
    #[test]
    fn checkpoint_timeout_beyond_budget_is_rejected() {
        let (mut c,w)=fixtures();
        c.background_preemptible_jobs[1].preemption.max_checkpoint_seconds=600;
        assert!(validate(&c,&w).unwrap_err().contains("exceeds"));
    }
    #[test]
    fn missing_checkpoint_integrity_is_rejected() {
        let (mut c,w)=fixtures();
        c.background_preemptible_jobs[0].preemption.checkpoint_integrity_required=false;
        assert!(validate(&c,&w).unwrap_err().contains("unsafe or unbounded preemption"));
    }
}
