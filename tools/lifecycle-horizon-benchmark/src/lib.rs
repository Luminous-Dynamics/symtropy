use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

const CLAIM_ID: &str = "lifecycle_action_profile";
pub const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: String,
    pub claim_ceiling: String,
    pub functional_unit: String,
    pub scope_id: String,
    pub asset_mass_g: u64,
    pub horizon_cycles: usize,
    pub service_hours_per_cycle: u64,
    pub minimum_condition_bp: u32,
    pub degradation_per_cycle_bp: u32,
    pub maximum_repairs: u32,
    pub initial_condition_bp: u32,
    pub initial_lineage_share_bp: u32,
    pub policy_search: PolicySearchSpec,
    pub evidence: Vec<Evidence>,
    pub actions: Vec<Action>,
    pub scenarios: Vec<Scenario>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub id: String,
    pub payload: String,
    pub sha256: String,
    pub status: EvidenceStatus,
    pub claim_id: String,
    pub scope_id: String,
    pub coverage: Coverage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Action {
    pub id: String,
    pub kind: Kind,
    pub evidence_id: String,
    pub safety: Gate,
    pub function: Gate,
    pub inspection: Gate,
    pub recovery_bp: u32,
    pub lineage_after_bp: Option<u32>,
    pub retired_mass_g: u64,
    pub fates: Fates,
    pub metrics: Metrics,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct Metrics {
    pub cost_cents: u64,
    pub virgin_material_g: u64,
    pub energy_wh: u64,
    pub ghg_co2e_g: u64,
    pub water_ml: u64,
    pub residual_waste_g: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct Fates {
    pub reused_component_g: u64,
    pub recycled_material_g: u64,
    pub energy_recovery_g: u64,
    pub residual_g: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub id: String,
    pub plans: Vec<Plan>,
    pub expected_frontier: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PolicySearchSpec {
    pub action_profile_ids: Vec<String>,
    pub expected_candidate_count: usize,
    pub expected_unique_policy_count: usize,
    pub expected_eligible_count: usize,
    pub expected_rejection_counts: Vec<DispositionCount>,
    pub expected_frontier_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DispositionCount {
    pub disposition: Disposition,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub id: String,
    pub steps: Vec<String>,
    pub expected: Expected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Expected {
    pub disposition: Disposition,
    pub delivered_service_hours: u64,
    pub lineage_weighted_service_hours: u64,
    pub condition_after_cycle_bp: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    Valid,
    Stale,
    Revoked,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Coverage {
    Complete,
    Partial,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Gate {
    Pass,
    Fail,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Continue,
    Repair,
    Remanufacture,
    ReplaceRecycle,
    ReplaceEnergyRecovery,
    ReplaceMixed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    Eligible,
    InvalidPlan,
    InvalidDigest,
    RejectedEvidence,
    OutOfScope,
    BlockedIncompleteCoverage,
    BlockedUnknown,
    RejectedHardGate,
    RejectedCondition,
    RejectedRepairLimit,
    RejectedMassBalance,
    InvalidMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Totals {
    pub cost_cents: u64,
    pub virgin_material_g: u64,
    pub energy_wh: u64,
    pub ghg_co2e_g: u64,
    pub water_ml: u64,
    pub residual_waste_g: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct FateTotals {
    pub retired_mass_g: u64,
    pub reused_component_g: u64,
    pub recycled_material_g: u64,
    pub energy_recovery_g: u64,
    pub residual_g: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanResult {
    pub scenario_id: String,
    pub plan_id: String,
    pub disposition: Disposition,
    pub reason: String,
    pub complete_service_horizon: bool,
    pub delivered_service_hours: u64,
    pub lineage_weighted_service_hours: u64,
    pub condition_after_cycle_bp: Vec<u32>,
    pub final_condition_bp: u32,
    pub final_lineage_share_bp: u32,
    pub repairs_used: u32,
    pub metrics: Totals,
    pub fates: FateTotals,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScenarioResult {
    pub scenario_id: String,
    pub plans: Vec<PlanResult>,
    pub eligible_plan_ids: Vec<String>,
    pub pareto_frontier: Vec<String>,
    pub oracle_frontier: Vec<String>,
    pub transition_oracle_agreement: bool,
    pub frontier_oracle_agreement: bool,
    pub fixture_expectations_match: bool,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Report {
    pub tool_version: String,
    pub schema_version: String,
    pub source_revision: String,
    pub manifest_sha256: String,
    pub claim_ceiling: String,
    pub functional_unit: String,
    pub asset_mass_g: u64,
    pub horizon_cycles: usize,
    pub evidence_count: usize,
    pub action_count: usize,
    pub scenario_count: usize,
    pub plan_count: usize,
    pub all_evidence_digests_valid: bool,
    pub scenarios: Vec<ScenarioResult>,
    pub overall_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicySearchReport {
    pub tool_version: String,
    pub source_revision: String,
    pub manifest_sha256: String,
    pub claim_ceiling: String,
    pub functional_unit: String,
    pub horizon_cycles: usize,
    pub action_profile_ids: Vec<String>,
    pub candidate_count: usize,
    pub unique_policy_count: usize,
    pub eligible_count: usize,
    pub complete_service_horizon_count: usize,
    pub rejected_count: usize,
    pub rejection_counts: Vec<DispositionCount>,
    pub transition_oracle_agreement: bool,
    pub pareto_frontier: Vec<String>,
    pub oracle_frontier: Vec<String>,
    pub frontier_oracle_agreement: bool,
    pub fixture_expectations_match: bool,
    pub plans: Vec<PlanResult>,
    pub overall_status: String,
}

#[derive(Debug, Clone, Default)]
struct State {
    condition: u32,
    lineage: u32,
    repairs: u32,
    delivered: u64,
    lineage_hours: u64,
    trace: Vec<u32>,
    metrics: Totals,
    fates: FateTotals,
}

pub fn evaluate(bytes: &[u8], source_revision: &str) -> Result<Report, String> {
    let manifest: Manifest = serde_json::from_slice(bytes)
        .map_err(|error| format!("manifest rejected: {error}"))?;
    validate(&manifest)?;

    let evidence: BTreeMap<&str, &Evidence> = manifest.evidence.iter()
        .map(|item| (item.id.as_str(), item)).collect();
    let actions: BTreeMap<&str, &Action> = manifest.actions.iter()
        .map(|item| (item.id.as_str(), item)).collect();
    let digests: BTreeMap<String, bool> = manifest.evidence.iter()
        .map(|item| (item.id.clone(), sha256_hex(item.payload.as_bytes()) == item.sha256))
        .collect();
    let all_digests_valid = digests.values().all(|valid| *valid);

    let mut ordered_scenarios: Vec<&Scenario> = manifest.scenarios.iter().collect();
    ordered_scenarios.sort_by(|left, right| left.id.cmp(&right.id));
    let mut scenario_results = Vec::new();
    let mut plan_count = 0;

    for scenario in ordered_scenarios {
        let mut production_results = Vec::new();
        let mut reference_results = Vec::new();

        for plan in &scenario.plans {
            production_results.push(simulate(
                &manifest, scenario, plan, &actions, &evidence, &digests,
            ));
            reference_results.push(simulate_reference(
                &manifest, scenario, plan, &actions, &evidence, &digests,
            ));
        }

        production_results.sort_by(|left, right| left.plan_id.cmp(&right.plan_id));
        reference_results.sort_by(|left, right| left.plan_id.cmp(&right.plan_id));
        plan_count += production_results.len();

        let transition_agreement = production_results.iter()
            .zip(reference_results.iter())
            .all(|(left, right)| same_outcome(left, right));
        let eligible: Vec<&PlanResult> = production_results.iter()
            .filter(|plan| plan.disposition == Disposition::Eligible).collect();
        let frontier = pareto_frontier(&eligible);
        let oracle_frontier = exhaustive_frontier(&eligible);
        let frontier_agreement = frontier == oracle_frontier;
        let expected_match = expectations_match(scenario, &production_results, &frontier);
        let status = if transition_agreement && frontier_agreement && expected_match {
            "PASS"
        } else {
            "FAIL"
        };

        scenario_results.push(ScenarioResult {
            scenario_id: scenario.id.clone(),
            plans: production_results,
            eligible_plan_ids: eligible.iter().map(|plan| plan.plan_id.clone())
                .collect::<BTreeSet<_>>().into_iter().collect(),
            pareto_frontier: frontier.into_iter().collect(),
            oracle_frontier: oracle_frontier.into_iter().collect(),
            transition_oracle_agreement: transition_agreement,
            frontier_oracle_agreement: frontier_agreement,
            fixture_expectations_match: expected_match,
            status: status.to_owned(),
        });
    }

    let overall_status = if all_digests_valid && scenario_results.iter().all(|scenario| scenario.status == "PASS") {
        "PASS_SYNTHETIC_CONTRACT"
    } else {
        "FAIL"
    };

    Ok(Report {
        tool_version: TOOL_VERSION.to_owned(),
        schema_version: manifest.schema_version,
        source_revision: source_revision.to_owned(),
        manifest_sha256: sha256_hex(bytes),
        claim_ceiling: manifest.claim_ceiling,
        functional_unit: manifest.functional_unit,
        asset_mass_g: manifest.asset_mass_g,
        horizon_cycles: manifest.horizon_cycles,
        evidence_count: manifest.evidence.len(),
        action_count: manifest.actions.len(),
        scenario_count: scenario_results.len(),
        plan_count,
        all_evidence_digests_valid: all_digests_valid,
        scenarios: scenario_results,
        overall_status: overall_status.to_owned(),
    })
}


/// Exhaustively enumerate the explicitly declared finite policy universe. The
/// transition implementation and independent reference are run on every
/// ordered action sequence. Terminal condition is a maximized Pareto objective
/// so the horizon's residual asset state remains visible.
pub fn evaluate_policy_search(bytes: &[u8], source_revision: &str) -> Result<PolicySearchReport, String> {
    let manifest: Manifest = serde_json::from_slice(bytes)
        .map_err(|error| format!("manifest rejected: {error}"))?;
    validate(&manifest)?;

    let evidence: BTreeMap<&str, &Evidence> = manifest.evidence.iter()
        .map(|item| (item.id.as_str(), item)).collect();
    let actions: BTreeMap<&str, &Action> = manifest.actions.iter()
        .map(|item| (item.id.as_str(), item)).collect();
    let digests: BTreeMap<String, bool> = manifest.evidence.iter()
        .map(|item| (item.id.clone(), sha256_hex(item.payload.as_bytes()) == item.sha256))
        .collect();
    let all_digests_valid = digests.values().all(|valid| *valid);

    let mut action_profile_ids = manifest.policy_search.action_profile_ids.clone();
    action_profile_ids.sort();
    let mut sequences = Vec::new();
    enumerate_policy_sequences(
        &action_profile_ids,
        manifest.horizon_cycles,
        &mut Vec::new(),
        &mut sequences,
    );
    let mut plans = Vec::with_capacity(sequences.len());
    for steps in sequences {
        let id = steps.join("/");
        plans.push(Plan {
            id,
            steps,
            expected: Expected {
                disposition: Disposition::Eligible,
                delivered_service_hours: 0,
                lineage_weighted_service_hours: 0,
                condition_after_cycle_bp: Vec::new(),
            },
        });
    }

    let search_scenario = Scenario {
        id: "exhaustive-policy-search".into(),
        plans,
        expected_frontier: manifest.policy_search.expected_frontier_ids.clone(),
    };
    let mut production_results = Vec::with_capacity(search_scenario.plans.len());
    let mut reference_results = Vec::with_capacity(search_scenario.plans.len());
    for plan in &search_scenario.plans {
        production_results.push(simulate(
            &manifest, &search_scenario, plan, &actions, &evidence, &digests,
        ));
        reference_results.push(simulate_reference(
            &manifest, &search_scenario, plan, &actions, &evidence, &digests,
        ));
    }
    production_results.sort_by(|left, right| left.plan_id.cmp(&right.plan_id));
    reference_results.sort_by(|left, right| left.plan_id.cmp(&right.plan_id));

    let candidate_count = production_results.len();
    let unique_policy_count = production_results.iter()
        .map(|plan| plan.plan_id.as_str()).collect::<BTreeSet<_>>().len();
    let transition_oracle_agreement = production_results.iter()
        .zip(reference_results.iter())
        .all(|(left, right)| same_outcome(left, right));
    let eligible: Vec<&PlanResult> = production_results.iter()
        .filter(|plan| plan.disposition == Disposition::Eligible).collect();
    let eligible_count = eligible.len();
    let complete_service_horizon_count = production_results.iter()
        .filter(|plan| plan.complete_service_horizon).count();
    let rejected_count = candidate_count.saturating_sub(eligible_count);

    let mut dispositions: BTreeMap<Disposition, usize> = BTreeMap::new();
    for plan in &production_results {
        if plan.disposition != Disposition::Eligible {
            *dispositions.entry(plan.disposition.clone()).or_insert(0) += 1;
        }
    }
    let rejection_counts: Vec<DispositionCount> = dispositions.into_iter()
        .map(|(disposition, count)| DispositionCount { disposition, count })
        .collect();

    let frontier = terminal_reserve_frontier(&eligible);
    let oracle_frontier = terminal_reserve_exhaustive_frontier(&eligible);
    let frontier_oracle_agreement = frontier == oracle_frontier;
    let mut expected_rejections = manifest.policy_search.expected_rejection_counts.clone();
    expected_rejections.sort_by(|left, right| left.disposition.cmp(&right.disposition));
    let known_policy_ids: BTreeSet<&str> = production_results.iter()
        .map(|plan| plan.plan_id.as_str()).collect();
    let expected_frontier: BTreeSet<String> = manifest.policy_search.expected_frontier_ids
        .iter().cloned().collect();
    let fixture_expectations_match =
        candidate_count == manifest.policy_search.expected_candidate_count
        && unique_policy_count == manifest.policy_search.expected_unique_policy_count
        && eligible_count == manifest.policy_search.expected_eligible_count
        && rejection_counts == expected_rejections
        && frontier == expected_frontier
        && expected_frontier.iter().all(|id| known_policy_ids.contains(id.as_str()))
        && complete_service_horizon_count == eligible_count
        && eligible.iter().all(|plan| plan.complete_service_horizon)
        && eligible_count + rejected_count == candidate_count
        && rejection_counts.iter().map(|item| item.count).sum::<usize>() == rejected_count;

    let status = if all_digests_valid
        && transition_oracle_agreement
        && frontier_oracle_agreement
        && fixture_expectations_match
    {
        "PASS_SYNTHETIC_CONTRACT"
    } else {
        "FAIL"
    };

    Ok(PolicySearchReport {
        tool_version: TOOL_VERSION.to_owned(),
        source_revision: source_revision.to_owned(),
        manifest_sha256: sha256_hex(bytes),
        claim_ceiling: manifest.claim_ceiling,
        functional_unit: manifest.functional_unit,
        horizon_cycles: manifest.horizon_cycles,
        action_profile_ids,
        candidate_count,
        unique_policy_count,
        eligible_count,
        complete_service_horizon_count,
        rejected_count,
        rejection_counts,
        transition_oracle_agreement,
        pareto_frontier: frontier.into_iter().collect(),
        oracle_frontier: oracle_frontier.into_iter().collect(),
        frontier_oracle_agreement,
        fixture_expectations_match,
        plans: production_results,
        overall_status: status.to_owned(),
    })
}

fn enumerate_policy_sequences(
    action_ids: &[String],
    horizon: usize,
    prefix: &mut Vec<String>,
    output: &mut Vec<Vec<String>>,
) {
    if prefix.len() == horizon {
        output.push(prefix.clone());
        return;
    }
    for action_id in action_ids {
        prefix.push(action_id.clone());
        enumerate_policy_sequences(action_ids, horizon, prefix, output);
        prefix.pop();
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut result = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut result, "{byte:02x}").expect("writing into String cannot fail");
    }
    result
}

fn validate(manifest: &Manifest) -> Result<(), String> {
    if manifest.schema_version != "0.1" || manifest.claim_ceiling != "synthetic_only" {
        return Err("schema_version must be 0.1 and claim_ceiling must be synthetic_only".into());
    }
    if manifest.scope_id.trim().is_empty()
        || manifest.functional_unit.trim().is_empty()
        || manifest.asset_mass_g == 0
        || manifest.service_hours_per_cycle == 0
        || !(1..=12).contains(&manifest.horizon_cycles)
    {
        return Err("invalid identity, asset mass, service unit, or horizon".into());
    }
    if [
        manifest.minimum_condition_bp,
        manifest.degradation_per_cycle_bp,
        manifest.initial_condition_bp,
        manifest.initial_lineage_share_bp,
    ].iter().any(|value| *value > 10_000)
    {
        return Err("condition, lineage, or degradation exceeds 10000 basis points".into());
    }
    if manifest.evidence.is_empty() || manifest.actions.is_empty() || manifest.scenarios.is_empty() {
        return Err("evidence, actions, and scenarios must be non-empty".into());
    }

    let mut evidence_ids = BTreeSet::new();
    for evidence in &manifest.evidence {
        if evidence.id.trim().is_empty()
            || evidence.payload.is_empty()
            || !is_hash(&evidence.sha256)
            || !evidence_ids.insert(evidence.id.as_str())
        {
            return Err(format!("invalid or duplicate evidence {}", evidence.id));
        }
    }

    let mut action_ids = BTreeSet::new();
    for action in &manifest.actions {
        if action.id.trim().is_empty() || !action_ids.insert(action.id.as_str()) {
            return Err(format!("invalid or duplicate action {}", action.id));
        }
        if !evidence_ids.contains(action.evidence_id.as_str()) {
            return Err(format!("action {} references missing evidence", action.id));
        }
    }

    if manifest.policy_search.action_profile_ids.is_empty()
        || manifest.policy_search.action_profile_ids.len() > 8
    {
        return Err("policy search must declare between one and eight action profiles".into());
    }
    let mut search_action_ids = BTreeSet::new();
    for id in &manifest.policy_search.action_profile_ids {
        if id.trim().is_empty() || id.contains('/') || !search_action_ids.insert(id.as_str()) {
            return Err(format!("invalid or duplicate policy-search action ID {id}"));
        }
        if !action_ids.contains(id.as_str()) {
            return Err(format!("policy search references missing action profile {id}"));
        }
    }
    let mut candidate_bound = 1usize;
    for _ in 0..manifest.horizon_cycles {
        candidate_bound = candidate_bound
            .checked_mul(manifest.policy_search.action_profile_ids.len())
            .ok_or_else(|| "policy search candidate count overflow".to_owned())?;
        if candidate_bound > 1_000_000 {
            return Err("policy search exceeds the one-million-candidate safety bound".into());
        }
    }
    let mut rejection_ids = BTreeSet::new();
    for expected in &manifest.policy_search.expected_rejection_counts {
        if expected.disposition == Disposition::Eligible
            || expected.count == 0
            || !rejection_ids.insert(expected.disposition.clone())
        {
            return Err("policy-search rejection counts must be nonzero, unique, and non-eligible".into());
        }
    }
    let expected_policy_frontier: BTreeSet<&str> = manifest.policy_search.expected_frontier_ids
        .iter().map(String::as_str).collect();
    if expected_policy_frontier.len() != manifest.policy_search.expected_frontier_ids.len() {
        return Err("policy-search expected frontier contains duplicate IDs".into());
    }

    let mut scenario_ids = BTreeSet::new();
    for scenario in &manifest.scenarios {
        if scenario.id.trim().is_empty()
            || !scenario_ids.insert(scenario.id.as_str())
            || scenario.plans.is_empty()
        {
            return Err(format!("invalid scenario {}", scenario.id));
        }
        let plan_ids: BTreeSet<&str> = scenario.plans.iter()
            .map(|plan| plan.id.as_str()).collect();
        if plan_ids.len() != scenario.plans.len() {
            return Err(format!("duplicate plan ID in scenario {}", scenario.id));
        }
        let expected_frontier: BTreeSet<&str> = scenario.expected_frontier.iter()
            .map(String::as_str).collect();
        if expected_frontier.len() != scenario.expected_frontier.len()
            || !expected_frontier.is_subset(&plan_ids)
        {
            return Err(format!("invalid expected frontier in scenario {}", scenario.id));
        }
    }
    Ok(())
}

fn fresh_state(manifest: &Manifest) -> State {
    State {
        condition: manifest.initial_condition_bp,
        lineage: manifest.initial_lineage_share_bp,
        ..State::default()
    }
}

fn finish(
    scenario: &Scenario,
    plan: &Plan,
    disposition: Disposition,
    reason: String,
    state: State,
    manifest: &Manifest,
) -> PlanResult {
    PlanResult {
        scenario_id: scenario.id.clone(),
        plan_id: plan.id.clone(),
        complete_service_horizon: disposition == Disposition::Eligible
            && state.trace.len() == manifest.horizon_cycles,
        disposition,
        reason,
        delivered_service_hours: state.delivered,
        lineage_weighted_service_hours: state.lineage_hours,
        condition_after_cycle_bp: state.trace,
        final_condition_bp: state.condition,
        final_lineage_share_bp: state.lineage,
        repairs_used: state.repairs,
        metrics: state.metrics,
        fates: state.fates,
    }
}

fn evidence_block(
    action: &Action,
    evidence: &Evidence,
    manifest: &Manifest,
    digest_valid: bool,
) -> Option<(Disposition, String)> {
    if !digest_valid {
        return Some((Disposition::InvalidDigest, "evidence digest mismatch".into()));
    }
    if matches!(&evidence.status, EvidenceStatus::Stale | EvidenceStatus::Revoked) {
        return Some((Disposition::RejectedEvidence, "evidence is stale or revoked".into()));
    }
    if evidence.claim_id != CLAIM_ID || evidence.scope_id != manifest.scope_id {
        return Some((Disposition::OutOfScope, "evidence claim or scope mismatch".into()));
    }
    if evidence.coverage == Coverage::Partial {
        return Some((Disposition::BlockedIncompleteCoverage, "evidence coverage is partial".into()));
    }
    if evidence.status == EvidenceStatus::Unknown || evidence.coverage == Coverage::Unknown {
        return Some((Disposition::BlockedUnknown, "evidence state is unknown".into()));
    }
    if action.safety == Gate::Fail || action.function == Gate::Fail || action.inspection == Gate::Fail {
        return Some((Disposition::RejectedHardGate, "required safety/function/inspection gate failed".into()));
    }
    if action.safety == Gate::Unknown || action.function == Gate::Unknown || action.inspection == Gate::Unknown {
        return Some((Disposition::BlockedUnknown, "required gate is unknown".into()));
    }
    None
}

fn fates_valid(action: &Action, manifest: &Manifest) -> bool {
    let fates = &action.fates;
    let sum = u128::from(fates.reused_component_g)
        + u128::from(fates.recycled_material_g)
        + u128::from(fates.energy_recovery_g)
        + u128::from(fates.residual_g);
    let is_replacement = matches!(
        &action.kind,
        Kind::ReplaceRecycle | Kind::ReplaceEnergyRecovery | Kind::ReplaceMixed
    );
    sum == u128::from(action.retired_mass_g)
        && action.metrics.residual_waste_g == fates.residual_g
        && (if is_replacement {
            action.retired_mass_g == manifest.asset_mass_g
        } else {
            action.retired_mass_g <= manifest.asset_mass_g
        })
}

fn effects_valid(action: &Action) -> bool {
    if action.recovery_bp > 10_000 || action.lineage_after_bp.is_some_and(|value| value > 10_000) {
        return false;
    }
    match &action.kind {
        Kind::Continue => action.recovery_bp == 0 && action.lineage_after_bp.is_none(),
        Kind::Repair => action.recovery_bp > 0 && action.lineage_after_bp.is_none(),
        Kind::Remanufacture => action.recovery_bp == 0 && action.lineage_after_bp.is_some(),
        Kind::ReplaceRecycle | Kind::ReplaceEnergyRecovery | Kind::ReplaceMixed => {
            action.recovery_bp == 0 && action.lineage_after_bp == Some(0)
        }
    }
}

fn add_metrics(total: &mut Totals, metrics: &Metrics) -> bool {
    macro_rules! add {
        ($field:ident) => {
            if let Some(value) = total.$field.checked_add(metrics.$field) {
                total.$field = value;
            } else {
                return false;
            }
        };
    }
    add!(cost_cents);
    add!(virgin_material_g);
    add!(energy_wh);
    add!(ghg_co2e_g);
    add!(water_ml);
    add!(residual_waste_g);
    true
}

fn add_fates(total: &mut FateTotals, action: &Action) -> bool {
    macro_rules! add {
        ($field:ident, $value:expr) => {
            if let Some(value) = total.$field.checked_add($value) {
                total.$field = value;
            } else {
                return false;
            }
        };
    }
    add!(retired_mass_g, action.retired_mass_g);
    add!(reused_component_g, action.fates.reused_component_g);
    add!(recycled_material_g, action.fates.recycled_material_g);
    add!(energy_recovery_g, action.fates.energy_recovery_g);
    add!(residual_g, action.fates.residual_g);
    true
}

fn simulate(
    manifest: &Manifest,
    scenario: &Scenario,
    plan: &Plan,
    actions: &BTreeMap<&str, &Action>,
    evidence: &BTreeMap<&str, &Evidence>,
    digests: &BTreeMap<String, bool>,
) -> PlanResult {
    let mut state = fresh_state(manifest);
    if plan.steps.len() != manifest.horizon_cycles {
        return finish(scenario, plan, Disposition::InvalidPlan,
            "plan length differs from horizon".into(), state, manifest);
    }

    for (index, action_id) in plan.steps.iter().enumerate() {
        let Some(action) = actions.get(action_id.as_str()).copied() else {
            return finish(scenario, plan, Disposition::InvalidPlan,
                format!("unknown action {action_id}"), state, manifest);
        };
        let Some(source) = evidence.get(action.evidence_id.as_str()).copied() else {
            return finish(scenario, plan, Disposition::InvalidPlan,
                "missing evidence reference".into(), state, manifest);
        };
        if let Some((disposition, reason)) = evidence_block(
            action, source, manifest, digests.get(&source.id).copied().unwrap_or(false)
        ) {
            return finish(scenario, plan, disposition, reason, state, manifest);
        }
        if !fates_valid(action, manifest) {
            return finish(scenario, plan, Disposition::RejectedMassBalance,
                format!("cycle {} fates do not conserve mass", index + 1), state, manifest);
        }
        if !effects_valid(action) {
            return finish(scenario, plan, Disposition::InvalidMetrics,
                format!("cycle {} has an invalid state effect", index + 1), state, manifest);
        }

        match &action.kind {
            Kind::Continue => {}
            Kind::Repair => {
                if state.repairs >= manifest.maximum_repairs {
                    return finish(scenario, plan, Disposition::RejectedRepairLimit,
                        format!("cycle {} repair budget exhausted", index + 1), state, manifest);
                }
                state.repairs += 1;
                state.condition = state.condition.saturating_add(action.recovery_bp).min(10_000);
            }
            Kind::Remanufacture => {
                state.condition = 10_000;
                state.lineage = state.lineage.min(action.lineage_after_bp.unwrap_or(0));
                state.repairs = 0;
            }
            Kind::ReplaceRecycle | Kind::ReplaceEnergyRecovery | Kind::ReplaceMixed => {
                state.condition = 10_000;
                state.lineage = 0;
                state.repairs = 0;
            }
        }

        if state.condition < manifest.minimum_condition_bp {
            return finish(scenario, plan, Disposition::RejectedCondition,
                format!("cycle {} condition below minimum", index + 1), state, manifest);
        }
        if !add_metrics(&mut state.metrics, &action.metrics) || !add_fates(&mut state.fates, action) {
            return finish(scenario, plan, Disposition::InvalidMetrics,
                "cumulative metric overflow".into(), state, manifest);
        }
        let Some(weighted_hours) = manifest.service_hours_per_cycle.checked_mul(u64::from(state.lineage)) else {
            return finish(scenario, plan, Disposition::InvalidMetrics,
                "lineage service-hour overflow".into(), state, manifest);
        };
        let Some(delivered) = state.delivered.checked_add(manifest.service_hours_per_cycle) else {
            return finish(scenario, plan, Disposition::InvalidMetrics,
                "delivered service-hour overflow".into(), state, manifest);
        };
        let Some(lineage_hours) = state.lineage_hours.checked_add(weighted_hours / 10_000) else {
            return finish(scenario, plan, Disposition::InvalidMetrics,
                "lineage service-hour overflow".into(), state, manifest);
        };
        state.delivered = delivered;
        state.lineage_hours = lineage_hours;
        state.condition = state.condition.saturating_sub(manifest.degradation_per_cycle_bp);
        state.trace.push(state.condition);
    }

    finish(scenario, plan, Disposition::Eligible, "complete horizon delivered".into(), state, manifest)
}

fn simulate_reference(
    manifest: &Manifest,
    scenario: &Scenario,
    plan: &Plan,
    actions: &BTreeMap<&str, &Action>,
    evidence: &BTreeMap<&str, &Evidence>,
    digests: &BTreeMap<String, bool>,
) -> PlanResult {
    let mut state = fresh_state(manifest);
    if plan.steps.len() != manifest.horizon_cycles {
        return finish(scenario, plan, Disposition::InvalidPlan,
            "reference: incorrect horizon length".into(), state, manifest);
    }

    for (index, step) in plan.steps.iter().enumerate() {
        let action = match actions.get(step.as_str()) {
            Some(action) => *action,
            None => return finish(scenario, plan, Disposition::InvalidPlan,
                "reference: unregistered action".into(), state, manifest),
        };
        let source = match evidence.get(action.evidence_id.as_str()) {
            Some(source) => *source,
            None => return finish(scenario, plan, Disposition::InvalidPlan,
                "reference: unregistered evidence".into(), state, manifest),
        };
        if !digests.get(&source.id).copied().unwrap_or(false) {
            return finish(scenario, plan, Disposition::InvalidDigest,
                "reference: source digest mismatch".into(), state, manifest);
        }
        if matches!(&source.status, EvidenceStatus::Stale | EvidenceStatus::Revoked) {
            return finish(scenario, plan, Disposition::RejectedEvidence,
                "reference: source status is inadmissible".into(), state, manifest);
        }
        if source.claim_id != CLAIM_ID || source.scope_id != manifest.scope_id {
            return finish(scenario, plan, Disposition::OutOfScope,
                "reference: claim scope differs".into(), state, manifest);
        }
        if source.coverage == Coverage::Partial {
            return finish(scenario, plan, Disposition::BlockedIncompleteCoverage,
                "reference: incomplete coverage".into(), state, manifest);
        }
        if source.status == EvidenceStatus::Unknown || source.coverage == Coverage::Unknown {
            return finish(scenario, plan, Disposition::BlockedUnknown,
                "reference: evidence is unknown".into(), state, manifest);
        }
        let gate_failed = [&action.safety, &action.function, &action.inspection]
            .iter().any(|gate| **gate == Gate::Fail);
        let gate_unknown = [&action.safety, &action.function, &action.inspection]
            .iter().any(|gate| **gate == Gate::Unknown);
        if gate_failed {
            return finish(scenario, plan, Disposition::RejectedHardGate,
                "reference: hard gate failed".into(), state, manifest);
        }
        if gate_unknown {
            return finish(scenario, plan, Disposition::BlockedUnknown,
                "reference: gate is unknown".into(), state, manifest);
        }

        let fates = &action.fates;
        let fate_sum = u128::from(fates.reused_component_g)
            + u128::from(fates.recycled_material_g)
            + u128::from(fates.energy_recovery_g)
            + u128::from(fates.residual_g);
        let replacement = matches!(&action.kind,
            Kind::ReplaceRecycle | Kind::ReplaceEnergyRecovery | Kind::ReplaceMixed);
        if fate_sum != u128::from(action.retired_mass_g)
            || action.metrics.residual_waste_g != fates.residual_g
            || (replacement && action.retired_mass_g != manifest.asset_mass_g)
            || (!replacement && action.retired_mass_g > manifest.asset_mass_g)
        {
            return finish(scenario, plan, Disposition::RejectedMassBalance,
                format!("reference: cycle {} mass imbalance", index + 1), state, manifest);
        }
        if !effects_valid(action) {
            return finish(scenario, plan, Disposition::InvalidMetrics,
                "reference: malformed action effect".into(), state, manifest);
        }

        match &action.kind {
            Kind::Continue => {}
            Kind::Repair => {
                if manifest.maximum_repairs <= state.repairs {
                    return finish(scenario, plan, Disposition::RejectedRepairLimit,
                        "reference: repair allowance exhausted".into(), state, manifest);
                }
                state.repairs += 1;
                state.condition = std::cmp::min(10_000, state.condition.saturating_add(action.recovery_bp));
            }
            Kind::Remanufacture => {
                state.condition = 10_000;
                state.lineage = state.lineage.min(action.lineage_after_bp.unwrap_or(0));
                state.repairs = 0;
            }
            Kind::ReplaceRecycle | Kind::ReplaceEnergyRecovery | Kind::ReplaceMixed => {
                state.condition = 10_000;
                state.lineage = 0;
                state.repairs = 0;
            }
        }
        if state.condition < manifest.minimum_condition_bp {
            return finish(scenario, plan, Disposition::RejectedCondition,
                "reference: below condition floor".into(), state, manifest);
        }

        macro_rules! add_metric {
            ($field:ident, $value:expr) => {
                match state.metrics.$field.checked_add($value) {
                    Some(value) => state.metrics.$field = value,
                    None => return finish(scenario, plan, Disposition::InvalidMetrics,
                        "reference: cumulative metric overflow".into(), state, manifest),
                }
            };
        }
        add_metric!(cost_cents, action.metrics.cost_cents);
        add_metric!(virgin_material_g, action.metrics.virgin_material_g);
        add_metric!(energy_wh, action.metrics.energy_wh);
        add_metric!(ghg_co2e_g, action.metrics.ghg_co2e_g);
        add_metric!(water_ml, action.metrics.water_ml);
        add_metric!(residual_waste_g, action.metrics.residual_waste_g);

        macro_rules! add_fate {
            ($field:ident, $value:expr) => {
                match state.fates.$field.checked_add($value) {
                    Some(value) => state.fates.$field = value,
                    None => return finish(scenario, plan, Disposition::InvalidMetrics,
                        "reference: fate total overflow".into(), state, manifest),
                }
            };
        }
        add_fate!(retired_mass_g, action.retired_mass_g);
        add_fate!(reused_component_g, action.fates.reused_component_g);
        add_fate!(recycled_material_g, action.fates.recycled_material_g);
        add_fate!(energy_recovery_g, action.fates.energy_recovery_g);
        add_fate!(residual_g, action.fates.residual_g);

        let weighted_hours = match manifest.service_hours_per_cycle.checked_mul(u64::from(state.lineage)) {
            Some(value) => value / 10_000,
            None => return finish(scenario, plan, Disposition::InvalidMetrics,
                "reference: lineage arithmetic overflow".into(), state, manifest),
        };
        state.delivered = match state.delivered.checked_add(manifest.service_hours_per_cycle) {
            Some(value) => value,
            None => return finish(scenario, plan, Disposition::InvalidMetrics,
                "reference: service-hour overflow".into(), state, manifest),
        };
        state.lineage_hours = match state.lineage_hours.checked_add(weighted_hours) {
            Some(value) => value,
            None => return finish(scenario, plan, Disposition::InvalidMetrics,
                "reference: lineage-hour overflow".into(), state, manifest),
        };
        state.condition = if state.condition > manifest.degradation_per_cycle_bp {
            state.condition - manifest.degradation_per_cycle_bp
        } else {
            0
        };
        state.trace.push(state.condition);
    }

    finish(scenario, plan, Disposition::Eligible, "reference: horizon complete".into(), state, manifest)
}

fn same_outcome(left: &PlanResult, right: &PlanResult) -> bool {
    left.scenario_id == right.scenario_id
        && left.plan_id == right.plan_id
        && left.disposition == right.disposition
        && left.complete_service_horizon == right.complete_service_horizon
        && left.delivered_service_hours == right.delivered_service_hours
        && left.lineage_weighted_service_hours == right.lineage_weighted_service_hours
        && left.condition_after_cycle_bp == right.condition_after_cycle_bp
        && left.final_condition_bp == right.final_condition_bp
        && left.final_lineage_share_bp == right.final_lineage_share_bp
        && left.repairs_used == right.repairs_used
        && left.metrics == right.metrics
        && left.fates == right.fates
}

fn dominates(a: &PlanResult, b: &PlanResult) -> bool {
    let no_worse = a.metrics.cost_cents <= b.metrics.cost_cents
        && a.metrics.virgin_material_g <= b.metrics.virgin_material_g
        && a.metrics.energy_wh <= b.metrics.energy_wh
        && a.metrics.ghg_co2e_g <= b.metrics.ghg_co2e_g
        && a.metrics.water_ml <= b.metrics.water_ml
        && a.metrics.residual_waste_g <= b.metrics.residual_waste_g
        && a.lineage_weighted_service_hours >= b.lineage_weighted_service_hours;
    no_worse && (
        a.metrics.cost_cents < b.metrics.cost_cents
            || a.metrics.virgin_material_g < b.metrics.virgin_material_g
            || a.metrics.energy_wh < b.metrics.energy_wh
            || a.metrics.ghg_co2e_g < b.metrics.ghg_co2e_g
            || a.metrics.water_ml < b.metrics.water_ml
            || a.metrics.residual_waste_g < b.metrics.residual_waste_g
            || a.lineage_weighted_service_hours > b.lineage_weighted_service_hours
    )
}

fn pareto_frontier(plans: &[&PlanResult]) -> BTreeSet<String> {
    plans.iter()
        .filter(|plan| !plans.iter().any(|other| {
            other.plan_id != plan.plan_id && dominates(other, plan)
        }))
        .map(|plan| plan.plan_id.clone())
        .collect()
}

fn exhaustive_frontier(plans: &[&PlanResult]) -> BTreeSet<String> {
    let vector = |plan: &PlanResult| [
        i128::from(plan.metrics.cost_cents),
        i128::from(plan.metrics.virgin_material_g),
        i128::from(plan.metrics.energy_wh),
        i128::from(plan.metrics.ghg_co2e_g),
        i128::from(plan.metrics.water_ml),
        i128::from(plan.metrics.residual_waste_g),
        -i128::from(plan.lineage_weighted_service_hours),
    ];
    let mut result = BTreeSet::new();
    for candidate in plans {
        let candidate_vector = vector(candidate);
        let mut dominated = false;
        for other in plans {
            if candidate.plan_id == other.plan_id {
                continue;
            }
            let other_vector = vector(other);
            let no_worse = (0..7).all(|i| other_vector[i] <= candidate_vector[i]);
            let strictly_better = (0..7).any(|i| other_vector[i] < candidate_vector[i]);
            if no_worse && strictly_better {
                dominated = true;
                break;
            }
        }
        if !dominated {
            result.insert(candidate.plan_id.clone());
        }
    }
    result
}


fn terminal_reserve_dominates(a: &PlanResult, b: &PlanResult) -> bool {
    let no_worse = a.metrics.cost_cents <= b.metrics.cost_cents
        && a.metrics.virgin_material_g <= b.metrics.virgin_material_g
        && a.metrics.energy_wh <= b.metrics.energy_wh
        && a.metrics.ghg_co2e_g <= b.metrics.ghg_co2e_g
        && a.metrics.water_ml <= b.metrics.water_ml
        && a.metrics.residual_waste_g <= b.metrics.residual_waste_g
        && a.lineage_weighted_service_hours >= b.lineage_weighted_service_hours
        && a.final_condition_bp >= b.final_condition_bp;
    no_worse && (
        a.metrics.cost_cents < b.metrics.cost_cents
            || a.metrics.virgin_material_g < b.metrics.virgin_material_g
            || a.metrics.energy_wh < b.metrics.energy_wh
            || a.metrics.ghg_co2e_g < b.metrics.ghg_co2e_g
            || a.metrics.water_ml < b.metrics.water_ml
            || a.metrics.residual_waste_g < b.metrics.residual_waste_g
            || a.lineage_weighted_service_hours > b.lineage_weighted_service_hours
            || a.final_condition_bp > b.final_condition_bp
    )
}

fn terminal_reserve_frontier(plans: &[&PlanResult]) -> BTreeSet<String> {
    plans.iter()
        .filter(|plan| !plans.iter().any(|other| {
            other.plan_id != plan.plan_id && terminal_reserve_dominates(other, plan)
        }))
        .map(|plan| plan.plan_id.clone())
        .collect()
}

/// Independent exhaustive oracle: normalize all eight objectives to integer
/// minimization and check every ordered eligible pair.
fn terminal_reserve_exhaustive_frontier(plans: &[&PlanResult]) -> BTreeSet<String> {
    let vector = |plan: &PlanResult| [
        i128::from(plan.metrics.cost_cents),
        i128::from(plan.metrics.virgin_material_g),
        i128::from(plan.metrics.energy_wh),
        i128::from(plan.metrics.ghg_co2e_g),
        i128::from(plan.metrics.water_ml),
        i128::from(plan.metrics.residual_waste_g),
        -i128::from(plan.lineage_weighted_service_hours),
        -i128::from(plan.final_condition_bp),
    ];
    let mut result = BTreeSet::new();
    for candidate in plans {
        let candidate_vector = vector(candidate);
        let mut dominated = false;
        for other in plans {
            if candidate.plan_id == other.plan_id {
                continue;
            }
            let other_vector = vector(other);
            let no_worse = (0..8).all(|index| other_vector[index] <= candidate_vector[index]);
            let strictly_better = (0..8).any(|index| other_vector[index] < candidate_vector[index]);
            if no_worse && strictly_better {
                dominated = true;
                break;
            }
        }
        if !dominated {
            result.insert(candidate.plan_id.clone());
        }
    }
    result
}

fn expectations_match(scenario: &Scenario, results: &[PlanResult], frontier: &BTreeSet<String>) -> bool {
    if results.len() != scenario.plans.len() {
        return false;
    }
    for result in results {
        let Some(plan) = scenario.plans.iter().find(|plan| plan.id == result.plan_id) else {
            return false;
        };
        if result.disposition != plan.expected.disposition
            || result.delivered_service_hours != plan.expected.delivered_service_hours
            || result.lineage_weighted_service_hours != plan.expected.lineage_weighted_service_hours
            || result.condition_after_cycle_bp != plan.expected.condition_after_cycle_bp
        {
            return false;
        }
    }
    *frontier == scenario.expected_frontier.iter().cloned().collect()
}

fn is_hash(value: &str) -> bool {
    value.len() == 64
        && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../fixtures/lifecycle-horizon-v0-1.json");

    fn manifest() -> Manifest {
        serde_json::from_str(FIXTURE).expect("fixture parses")
    }
    fn report() -> Report {
        evaluate(FIXTURE.as_bytes(), "test-revision").expect("fixture evaluates")
    }
    fn plan<'a>(report: &'a Report, scenario: &str, id: &str) -> &'a PlanResult {
        report.scenarios.iter().find(|item| item.scenario_id == scenario).unwrap()
            .plans.iter().find(|item| item.plan_id == id).unwrap()
    }

    #[test]
    fn frozen_plans_match_transition_and_frontier_oracles() {
        let report = report();
        assert_eq!(report.overall_status, "PASS_SYNTHETIC_CONTRACT");
        for scenario in &report.scenarios {
            assert_eq!(scenario.status, "PASS");
            assert!(scenario.transition_oracle_agreement);
            assert!(scenario.frontier_oracle_agreement);
            assert!(scenario.fixture_expectations_match);
        }
    }

    #[test]
    fn no_maintenance_cannot_claim_full_horizon() {
        let report = report();
        let result = plan(&report, "baseline-horizon", "no-maintenance");
        assert_eq!(result.disposition, Disposition::RejectedCondition);
        assert_eq!(result.delivered_service_hours, 20_000);
        assert!(!result.complete_service_horizon);
    }

    #[test]
    fn remanufacture_restores_condition_but_reduces_lineage() {
        let report = report();
        let result = plan(&report, "baseline-horizon", "remanufacture-cycle-2");
        assert_eq!(result.delivered_service_hours, 30_000);
        assert_eq!(result.lineage_weighted_service_hours, 20_000);
        assert_eq!(result.condition_after_cycle_bp, vec![7500, 7500, 5000]);
    }

    #[test]
    fn remanufacture_cannot_restore_original_lineage_after_replacement() {
        let report = report();
        let result = plan(&report, "baseline-horizon", "replace-then-remanufacture");
        assert_eq!(result.disposition, Disposition::Eligible);
        assert_eq!(result.delivered_service_hours, 30_000);
        assert_eq!(result.lineage_weighted_service_hours, 10_000);
        assert_eq!(result.condition_after_cycle_bp, vec![7500, 7500, 7500]);
    }

    #[test]
    fn repair_preserves_more_lineage_service_than_remanufacture() {
        let report = report();
        assert_eq!(plan(&report, "baseline-horizon", "repair-cycle-2").lineage_weighted_service_hours, 30_000);
        assert_eq!(plan(&report, "baseline-horizon", "remanufacture-cycle-2").lineage_weighted_service_hours, 20_000);
    }

    #[test]
    fn evidence_and_gates_fail_closed() {
        let report = report();
        assert_eq!(plan(&report, "evidence-and-gates", "unknown-evidence").disposition, Disposition::BlockedUnknown);
        assert_eq!(plan(&report, "evidence-and-gates", "partial-evidence").disposition, Disposition::BlockedIncompleteCoverage);
        assert_eq!(plan(&report, "evidence-and-gates", "revoked-evidence").disposition, Disposition::RejectedEvidence);
        assert_eq!(plan(&report, "evidence-and-gates", "failed-inspection").disposition, Disposition::RejectedHardGate);
        assert_eq!(plan(&report, "evidence-and-gates", "scope-mismatch").disposition, Disposition::OutOfScope);
    }

    #[test]
    fn repair_budget_and_bad_fates_are_rejected() {
        let report = report();
        assert_eq!(plan(&report, "repair-limit-and-fates", "repair-too-many-times").disposition, Disposition::RejectedRepairLimit);
        assert_eq!(plan(&report, "repair-limit-and-fates", "bad-mass-fates").disposition, Disposition::RejectedMassBalance);
    }

    #[test]
    fn energy_recovery_is_not_material_recycling() {
        let report = report();
        let scenario = report.scenarios.iter().find(|item| item.scenario_id == "replacement-fates").unwrap();
        let recycle = scenario.plans.iter().find(|item| item.plan_id == "replace-recycle").unwrap();
        let energy = scenario.plans.iter().find(|item| item.plan_id == "replace-energy-recovery").unwrap();
        assert_eq!(recycle.fates.recycled_material_g, 600);
        assert_eq!(recycle.fates.energy_recovery_g, 0);
        assert_eq!(energy.fates.recycled_material_g, 100);
        assert_eq!(energy.fates.energy_recovery_g, 600);
        assert_eq!(energy.fates.retired_mass_g, 1000);
    }

    #[test]
    fn wrong_length_and_unknown_action_are_invalid_plans() {
        let report = report();
        assert_eq!(plan(&report, "repair-limit-and-fates", "short-plan").disposition, Disposition::InvalidPlan);
        assert_eq!(plan(&report, "repair-limit-and-fates", "unknown-action").disposition, Disposition::InvalidPlan);
    }

    #[test]
    fn mutated_digest_blocks_plan() {
        let mut manifest = manifest();
        manifest.evidence.iter_mut().find(|item| item.id == "ev-repair").unwrap().sha256 = "0".repeat(64);
        let bytes = serde_json::to_vec(&manifest).unwrap();
        let report = evaluate(&bytes, "test").unwrap();
        assert_eq!(report.overall_status, "FAIL");
        assert_eq!(plan(&report, "baseline-horizon", "repair-cycle-2").disposition, Disposition::InvalidDigest);
    }

    #[test]
    fn undeclared_fields_are_rejected() {
        let bad = FIXTURE.replace(
            r#""claim_ceiling": "synthetic_only""#,
            r#""claim_ceiling": "synthetic_only", "auto_approve": true"#,
        );
        assert!(evaluate(bad.as_bytes(), "test").is_err());
    }


    #[test]
    fn exhaustive_policy_search_covers_the_declared_universe_and_matches_oracles() {
        let report = evaluate_policy_search(FIXTURE.as_bytes(), "test-revision")
            .expect("policy search evaluates");
        assert_eq!(report.overall_status, "PASS_SYNTHETIC_CONTRACT");
        assert_eq!(report.candidate_count, 216);
        assert_eq!(report.unique_policy_count, 216);
        assert_eq!(report.eligible_count, 198);
        assert_eq!(report.rejected_count, 18);
        assert_eq!(report.complete_service_horizon_count, 198);
        assert!(report.transition_oracle_agreement);
        assert!(report.frontier_oracle_agreement);
        assert!(report.fixture_expectations_match);
        assert_eq!(
            report.rejection_counts,
            vec![
                DispositionCount { disposition: Disposition::RejectedCondition, count: 6 },
                DispositionCount { disposition: Disposition::RejectedRepairLimit, count: 12 },
            ]
        );
        assert_eq!(
            report.pareto_frontier,
            vec![
                "continue/continue/remanufacture".to_owned(),
                "continue/continue/repair".to_owned(),
            ]
        );
    }

    #[test]
    fn exhaustive_search_is_invariant_to_declared_action_order() {
        let mut manifest = manifest();
        let bytes_a = serde_json::to_vec(&manifest).unwrap();
        manifest.policy_search.action_profile_ids.reverse();
        let bytes_b = serde_json::to_vec(&manifest).unwrap();
        let a = evaluate_policy_search(&bytes_a, "test-revision").unwrap();
        let b = evaluate_policy_search(&bytes_b, "test-revision").unwrap();
        assert_eq!(a.candidate_count, b.candidate_count);
        assert_eq!(a.unique_policy_count, b.unique_policy_count);
        assert_eq!(a.eligible_count, b.eligible_count);
        assert_eq!(a.rejection_counts, b.rejection_counts);
        assert_eq!(a.pareto_frontier, b.pareto_frontier);
        assert_eq!(a.plans, b.plans);
    }

    #[test]
    fn changing_the_frozen_policy_frontier_fails_the_search_receipt() {
        let mut manifest = manifest();
        manifest.policy_search.expected_frontier_ids.pop();
        let bytes = serde_json::to_vec(&manifest).unwrap();
        let report = evaluate_policy_search(&bytes, "test-revision").unwrap();
        assert_eq!(report.frontier_oracle_agreement, true);
        assert_eq!(report.fixture_expectations_match, false);
        assert_eq!(report.overall_status, "FAIL");
    }

    #[test]
    fn policy_search_enforces_a_finite_candidate_budget() {
        let mut manifest = manifest();
        manifest.horizon_cycles = 12;
        manifest.policy_search.action_profile_ids = vec![
            "continue".into(), "repair".into(), "remanufacture".into(),
            "replace-recycle".into(), "replace-energy-recovery".into(), "replace-mixed".into(),
        ];
        let bytes = serde_json::to_vec(&manifest).unwrap();
        assert!(evaluate_policy_search(&bytes, "test-revision")
            .unwrap_err().contains("one-million-candidate"));
    }

    #[test]
    fn sha256_known_answer() {
        assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }
}
