use orbital_robust_compatibility_bench::{
    evaluate as evaluate_robust, RobustCase, RobustFixture, RobustStatus,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

const FIXTURE_SCHEMA: &str = "luminous.orbital-service-reconfiguration.corpus.v0.1";
const ROBUST_SCHEMA: &str = "luminous.orbital-robust-compatibility.corpus.v0.1";
const CLAIM_CEILING: &str = "synthetic-service-reconfiguration-only";

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Reconciliation {
    NotApplicable,
    Resolved,
    Unknown,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServiceStatus {
    PlannedCapacityFloorMetSynthetic,
    PlannedCapacityShortfall,
    Indeterminate,
    Rejected,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceFixture {
    pub schema: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub client_profile_id: String,
    pub client_configuration_revision: String,
    pub start_tick: u64,
    pub end_tick: u64,
    pub providers: Vec<Provider>,
    pub scenarios: Vec<Scenario>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Provider {
    pub provider_id: String,
    pub supplier_id: String,
    pub profile_id: String,
    pub configuration_revision: String,
    pub lifecycle_state_id: String,
    pub service_kind: String,
    pub capacity_per_tick: u64,
    pub available_from_tick: u64,
    pub available_until_tick: u64,
    pub failure_domains: Vec<String>,
    pub handoff_ticks: u64,
    pub requalification_ticks: u64,
    pub requires_spare: bool,
    pub spare_ready: bool,
    pub requires_propellant: bool,
    pub propellant_ready: bool,
    pub qualification: RobustCase,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub case_id: String,
    pub service_kind: String,
    pub minimum_planned_service_capacity: u64,
    pub demand: Vec<Demand>,
    pub faults: Vec<Fault>,
    pub plan: Vec<PlanStep>,
    pub expected_status: ServiceStatus,
    pub expected_reason: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Demand {
    pub tick: u64,
    pub required_units: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Fault {
    pub failure_domain_id: String,
    pub start_tick: u64,
    pub end_tick: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlanStep {
    pub provider_id: String,
    pub switch_start_tick: u64,
    pub activation_tick: u64,
    pub prior_outcome_reconciliation: Reconciliation,
}

#[derive(Debug, Clone, Serialize)]
pub struct TickResult {
    pub tick: u64,
    pub provider_id: String,
    pub qualification_status: String,
    pub provider_available: bool,
    pub demanded_units: u64,
    pub available_capacity_units: u64,
    pub unserved_demand_units: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScenarioResult {
    pub case_id: String,
    pub actual_status: ServiceStatus,
    pub actual_reason: String,
    pub expected_status: ServiceStatus,
    pub expected_reason: String,
    pub expected_result_matched: bool,
    pub available_capacity_units: u64,
    pub unserved_demand_units: u64,
    pub capacity_unavailable_ticks: u64,
    pub first_capacity_restored_tick: Option<u64>,
    pub authority_granted: bool,
    pub ticks: Vec<TickResult>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub scenario_count: usize,
    pub planned_capacity_floor_met_synthetic: usize,
    pub planned_capacity_shortfall: usize,
    pub indeterminate: usize,
    pub rejected: usize,
    pub expected_result_mismatches: usize,
    pub nominal_false_capacity_claims: usize,
    pub authority_grants: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub result: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub authority_granted: bool,
    pub summary: Summary,
    pub scenarios: Vec<ScenarioResult>,
}

#[derive(Debug, Clone, Copy)]
struct Decision {
    status: ServiceStatus,
    reason: &'static str,
}

impl Decision {
    fn met() -> Self {
        Self { status: ServiceStatus::PlannedCapacityFloorMetSynthetic, reason: "planned_capacity_floor_met" }
    }
    fn shortfall(reason: &'static str) -> Self {
        Self { status: ServiceStatus::PlannedCapacityShortfall, reason }
    }
    fn indeterminate(reason: &'static str) -> Self {
        Self { status: ServiceStatus::Indeterminate, reason }
    }
    fn rejected(reason: &'static str) -> Self {
        Self { status: ServiceStatus::Rejected, reason }
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing to String is infallible");
    }
    output
}

fn present(value: &str) -> bool {
    !value.trim().is_empty()
}

fn is_faulted(provider: &Provider, faults: &[Fault], tick: u64) -> bool {
    let implicit_supplier_domain = format!("supplier:{}", provider.supplier_id);
    faults.iter().any(|fault| {
        (provider.failure_domains.iter().any(|domain| {
            fault.failure_domain_id == *domain
        }) || fault.failure_domain_id == implicit_supplier_domain)
            && fault.start_tick <= tick
            && tick <= fault.end_tick
    })
}

fn active_fault_domains(faults: &[Fault], tick: u64) -> HashSet<&str> {
    faults
        .iter()
        .filter(|fault| fault.start_tick <= tick && tick <= fault.end_tick)
        .map(|fault| fault.failure_domain_id.as_str())
        .collect()
}

fn robust_qualification(
    provider: &Provider,
    client_profile_id: &str,
    client_configuration_revision: &str,
    tick: u64,
) -> Result<(), Decision> {
    if provider.qualification.nominal_case.provider.profile_id.as_deref()
        != Some(provider.profile_id.as_str())
        || provider.qualification.nominal_case.provider.config_revision.as_deref()
            != Some(provider.configuration_revision.as_str())
        || provider.qualification.provider_lifecycle_state_id != provider.lifecycle_state_id
    {
        return Err(Decision::indeterminate("provider_qualification_identity_mismatch"));
    }
    if provider.qualification.nominal_case.client.profile_id.as_deref()
        != Some(client_profile_id)
        || provider.qualification.nominal_case.client.config_revision.as_deref()
            != Some(client_configuration_revision)
    {
        return Err(Decision::indeterminate("qualification_wrong_client_scope"));
    }

    let mut case = provider.qualification.clone();
    case.at_tick = tick;
    case.nominal_case.at_tick = tick;
    let fixture = RobustFixture {
        schema: ROBUST_SCHEMA.to_owned(),
        evidence_class: "synthetic".to_owned(),
        claim_ceiling: "synthetic-profile-only".to_owned(),
        cases: vec![case],
    };
    let report = evaluate_robust(&fixture)
        .map_err(|_| Decision::indeterminate("robust_qualification_evaluator_error"))?;
    let Some(result) = report.cases.first() else {
        return Err(Decision::indeterminate("robust_qualification_result_missing"));
    };
    match result.actual_status {
        RobustStatus::RobustlyCompatibleSynthetic => Ok(()),
        RobustStatus::Rejected => Err(Decision::rejected("candidate_robust_qualification_rejected")),
        RobustStatus::Indeterminate => Err(Decision::indeterminate("candidate_robust_qualification_indeterminate")),
    }
}

fn preflight(
    fixture: &ServiceFixture,
    providers: &HashMap<&str, &Provider>,
    known_fault_domains: &HashSet<String>,
    scenario: &Scenario,
) -> Result<(), Decision> {
    if !present(&scenario.case_id) {
        return Err(Decision::rejected("empty_scenario_id"));
    }
    if !present(&scenario.service_kind) {
        return Err(Decision::indeterminate("missing_service_kind"));
    }
    if scenario.plan.is_empty() {
        return Err(Decision::rejected("empty_reconfiguration_plan"));
    }
    if scenario.demand.is_empty() {
        return Err(Decision::indeterminate("missing_service_demand"));
    }
    let mut demand_ticks = HashSet::new();
    for demand in &scenario.demand {
        if demand.tick < fixture.start_tick || demand.tick > fixture.end_tick {
            return Err(Decision::rejected("demand_outside_mission_horizon"));
        }
        if !demand_ticks.insert(demand.tick) {
            return Err(Decision::rejected("duplicate_demand_tick"));
        }
    }
    if scenario.minimum_planned_service_capacity == 0 {
        return Err(Decision::rejected("service_floor_must_be_positive"));
    }
    for fault in &scenario.faults {
        if !present(&fault.failure_domain_id) {
            return Err(Decision::indeterminate("missing_failure_domain_id"));
        }
        if !known_fault_domains.contains(fault.failure_domain_id.as_str()) {
            return Err(Decision::rejected("unknown_fault_domain"));
        }
        if fault.start_tick > fault.end_tick
            || fault.start_tick < fixture.start_tick
            || fault.end_tick > fixture.end_tick
        {
            return Err(Decision::rejected("invalid_fault_window"));
        }
    }

    let first = &scenario.plan[0];
    if first.switch_start_tick != fixture.start_tick
        || first.activation_tick != fixture.start_tick
        || first.prior_outcome_reconciliation != Reconciliation::NotApplicable
    {
        return Err(Decision::rejected("invalid_initial_provider_step"));
    }
    if !providers.contains_key(first.provider_id.as_str()) {
        return Err(Decision::rejected("unknown_initial_provider"));
    }

    let mut seen_providers = HashSet::new();
    let mut previous_activation: Option<u64> = None;
    for (index, step) in scenario.plan.iter().enumerate() {
        if !seen_providers.insert(step.provider_id.as_str()) {
            return Err(Decision::rejected("duplicate_provider_allocation"));
        }
        let provider = providers.get(step.provider_id.as_str())
            .ok_or(Decision::rejected("unknown_plan_provider"))?;
        if provider.service_kind != scenario.service_kind {
            return Err(Decision::rejected("provider_service_kind_mismatch"));
        }
        if provider.failure_domains.is_empty() {
            return Err(Decision::indeterminate("provider_failure_domain_graph_missing"));
        }
        if provider.failure_domains.iter().any(|domain| !present(domain)) {
            return Err(Decision::indeterminate("incomplete_provider_failure_domain"));
        }
        if provider.capacity_per_tick == 0 {
            return Err(Decision::rejected("provider_capacity_zero"));
        }
        if provider.available_from_tick > provider.available_until_tick {
            return Err(Decision::rejected("invalid_provider_availability_window"));
        }
        if provider.requires_spare && !provider.spare_ready {
            return Err(Decision::rejected("required_spare_unavailable"));
        }
        if provider.requires_propellant && !provider.propellant_ready {
            return Err(Decision::rejected("required_propellant_unavailable"));
        }
        if step.activation_tick < provider.available_from_tick
            || step.activation_tick > provider.available_until_tick
        {
            return Err(Decision::rejected("provider_unavailable_at_activation"));
        }

        if index == 0 {
            if step.activation_tick != fixture.start_tick {
                return Err(Decision::rejected("invalid_initial_provider_step"));
            }
            if let Err(decision) = robust_qualification(
                provider,
                &fixture.client_profile_id,
                &fixture.client_configuration_revision,
                step.activation_tick,
            ) {
                return Err(match decision.status {
                    ServiceStatus::Rejected => Decision::rejected("initial_provider_qualification_rejected"),
                    ServiceStatus::Indeterminate => Decision::indeterminate("initial_provider_qualification_indeterminate"),
                    _ => Decision::indeterminate("initial_provider_qualification_unknown"),
                });
            }
            previous_activation = Some(step.activation_tick);
            continue;
        }
        if previous_activation.is_some_and(|prior| step.activation_tick <= prior) {
            return Err(Decision::rejected("plan_activation_order_invalid"));
        }

        if step.switch_start_tick < fixture.start_tick
            || step.switch_start_tick > fixture.end_tick
            || step.activation_tick < step.switch_start_tick
            || step.activation_tick > fixture.end_tick
        {
            return Err(Decision::rejected("invalid_reconfiguration_window"));
        }
        if step.prior_outcome_reconciliation == Reconciliation::Unknown {
            return Err(Decision::indeterminate("prior_service_outcome_unknown_blocks_reconfiguration"));
        }
        if step.prior_outcome_reconciliation != Reconciliation::Resolved {
            return Err(Decision::rejected("prior_service_outcome_not_reconciled"));
        }
        let previous = providers.get(scenario.plan[index - 1].provider_id.as_str())
            .ok_or(Decision::rejected("unknown_previous_provider"))?;
        if !is_faulted(previous, &scenario.faults, step.switch_start_tick) {
            return Err(Decision::rejected("reconfiguration_without_active_provider_failure"));
        }

        // Handoff and requalification are sequential by default. Overlap requires
        // a separately qualified transition-dependency contract; v0.1 never assumes it.
        let transition_ticks = provider.handoff_ticks.checked_add(provider.requalification_ticks)
            .ok_or(Decision::rejected("transition_tick_overflow"))?;
        let earliest_activation = step.switch_start_tick.checked_add(transition_ticks)
            .ok_or(Decision::rejected("transition_tick_overflow"))?;
        if step.activation_tick < earliest_activation {
            return Err(Decision::rejected("handoff_or_requalification_incomplete"));
        }

        let active_domains = active_fault_domains(&scenario.faults, step.activation_tick);
        let implicit_supplier_domain = format!("supplier:{}", provider.supplier_id);
        if provider.failure_domains.iter().any(|domain| active_domains.contains(domain.as_str()))
            || active_domains.contains(implicit_supplier_domain.as_str())
        {
            return Err(Decision::rejected("fallback_shares_failed_domain"));
        }
        if let Err(decision) = robust_qualification(
            provider,
            &fixture.client_profile_id,
            &fixture.client_configuration_revision,
            step.activation_tick,
        ) {
            return Err(match decision.status {
                ServiceStatus::Rejected => Decision::rejected("candidate_robust_qualification_rejected"),
                ServiceStatus::Indeterminate => Decision::indeterminate("candidate_robust_qualification_indeterminate"),
                _ => Decision::indeterminate("candidate_robust_qualification_unknown"),
            });
        }
        previous_activation = Some(step.activation_tick);
    }
    Ok(())
}

fn evaluate_scenario(
    fixture: &ServiceFixture,
    providers: &HashMap<&str, &Provider>,
    known_fault_domains: &HashSet<&str>,
    scenario: &Scenario,
) -> ScenarioResult {
    let mut result = ScenarioResult {
        case_id: scenario.case_id.clone(),
        actual_status: ServiceStatus::Indeterminate,
        actual_reason: "not_evaluated".to_owned(),
        expected_status: scenario.expected_status,
        expected_reason: scenario.expected_reason.clone(),
        expected_result_matched: false,
        available_capacity_units: 0,
        unserved_demand_units: 0,
        capacity_unavailable_ticks: 0,
        first_capacity_restored_tick: None,
        authority_granted: false,
        ticks: Vec::new(),
    };

    if let Err(decision) = preflight(fixture, providers, known_fault_domains, scenario) {
        result.actual_status = decision.status;
        result.actual_reason = decision.reason.to_owned();
        result.expected_result_matched =
            result.actual_status == result.expected_status
                && result.actual_reason == result.expected_reason;
        return result;
    }

    let mut demand_by_tick = HashMap::new();
    for demand in &scenario.demand {
        demand_by_tick.insert(demand.tick, demand.required_units);
    }

    let mut saw_fault = false;
    let mut service_indeterminate = false;
    let mut service_rejected = false;
    for tick in fixture.start_tick..=fixture.end_tick {
        let active_step = scenario.plan.iter().rev()
            .find(|step| step.activation_tick <= tick);
        let Some(step) = active_step else {
            result.actual_status = ServiceStatus::Rejected;
            result.actual_reason = "no_active_provider_for_tick".to_owned();
            result.expected_result_matched =
                result.actual_status == result.expected_status
                    && result.actual_reason == result.expected_reason;
            return result;
        };
        let Some(provider) = providers.get(step.provider_id.as_str()).copied() else {
            result.actual_status = ServiceStatus::Rejected;
            result.actual_reason = "active_provider_missing".to_owned();
            result.expected_result_matched =
                result.actual_status == result.expected_status
                    && result.actual_reason == result.expected_reason;
            return result;
        };
        let demand = demand_by_tick.get(&tick).copied().unwrap_or(0);
        let faulted = is_faulted(provider, &scenario.faults, tick);
        if faulted {
            saw_fault = true;
        }

        let mut qualification = "not_required_no_demand".to_owned();
        let mut available = !faulted
            && provider.available_from_tick <= tick
            && tick <= provider.available_until_tick;

        if demand > 0 && available {
            match robust_qualification(
                provider,
                &fixture.client_profile_id,
                &fixture.client_configuration_revision,
                tick,
            ) {
                Ok(()) => qualification = "robustly_compatible_synthetic".to_owned(),
                Err(decision) => {
                    qualification = match decision.status {
                        ServiceStatus::Rejected => "rejected",
                        ServiceStatus::Indeterminate => "indeterminate",
                        _ => "not_qualified",
                    }.to_owned();
                    available = false;
                    if decision.status == ServiceStatus::Indeterminate {
                        service_indeterminate = true;
                    } else if decision.status == ServiceStatus::Rejected {
                        service_rejected = true;
                    }
                }
            }
        } else if demand > 0 && !available {
            qualification = "unavailable_or_faulted".to_owned();
        }

        let accepted = if available { demand.min(provider.capacity_per_tick) } else { 0 };
        let unmet = demand.saturating_sub(accepted);
        result.available_capacity_units = result.available_capacity_units.saturating_add(accepted);
        result.unserved_demand_units = result.unserved_demand_units.saturating_add(unmet);
        if demand > 0 && accepted == 0 {
            result.capacity_unavailable_ticks = result.capacity_unavailable_ticks.saturating_add(1);
        }
        if result.first_capacity_restored_tick.is_none() && saw_fault && accepted > 0 {
            result.first_capacity_restored_tick = Some(tick);
        }
        result.ticks.push(TickResult {
            tick,
            provider_id: provider.provider_id.clone(),
            qualification_status: qualification,
            provider_available: available,
            demanded_units: demand,
            available_capacity_units: accepted,
            unserved_demand_units: unmet,
        });
    }

    let decision = if service_rejected {
        Decision::rejected("active_provider_qualification_rejected")
    } else if service_indeterminate {
        Decision::indeterminate("active_provider_qualification_indeterminate")
    } else if result.available_capacity_units >= scenario.minimum_planned_service_capacity {
        Decision::met()
    } else {
        Decision::shortfall("planned_capacity_below_required_floor")
    };
    result.actual_status = decision.status;
    result.actual_reason = decision.reason.to_owned();
    result.expected_result_matched =
        result.actual_status == result.expected_status
            && result.actual_reason == result.expected_reason;
    result
}

pub fn evaluate(fixture: &ServiceFixture) -> Result<Report, String> {
    if fixture.schema != FIXTURE_SCHEMA {
        return Err(format!("unsupported service fixture schema: {}", fixture.schema));
    }
    if fixture.evidence_class != "synthetic" || fixture.claim_ceiling != CLAIM_CEILING {
        return Err("v0.1 service reconfiguration accepts synthetic evidence only".to_owned());
    }
    if !present(&fixture.client_profile_id)
        || !present(&fixture.client_configuration_revision)
        || fixture.start_tick > fixture.end_tick

    {
        return Err("invalid mission service contract".to_owned());
    }
    if fixture.providers.is_empty() || fixture.scenarios.is_empty() {
        return Err("providers and scenarios must be non-empty".to_owned());
    }

    let mut providers = HashMap::new();
    let mut known_fault_domains: HashSet<String> = HashSet::new();
    for provider in &fixture.providers {
        if !present(&provider.provider_id)
            || !present(&provider.supplier_id)
            || !present(&provider.profile_id)
            || !present(&provider.configuration_revision)
            || !present(&provider.lifecycle_state_id)
            || !present(&provider.service_kind)
        {
            return Err("provider identity incomplete".to_owned());
        }
        if providers.insert(provider.provider_id.as_str(), provider).is_some() {
            return Err(format!("duplicate provider id: {}", provider.provider_id));
        }
        known_fault_domains.insert(format!("supplier:{}", provider.supplier_id));
        for domain in &provider.failure_domains {
            if !present(domain) {
                return Err(format!("provider {} has empty failure domain", provider.provider_id));
            }
            known_fault_domains.insert(domain.clone());
        }
    }

    let mut seen = HashSet::new();
    let mut results = Vec::with_capacity(fixture.scenarios.len());
    let mut met = 0usize;
    let mut shortfall = 0usize;
    let mut indeterminate = 0usize;
    let mut rejected = 0usize;
    let mut mismatches = 0usize;
    let mut false_continuity = 0usize;

    for scenario in &fixture.scenarios {
        if !seen.insert(scenario.case_id.as_str()) {
            return Err(format!("duplicate scenario id: {}", scenario.case_id));
        }
        let result = evaluate_scenario(fixture, &providers, &known_fault_domains, scenario);
        match result.actual_status {
            ServiceStatus::PlannedCapacityFloorMetSynthetic => met += 1,
            ServiceStatus::PlannedCapacityShortfall => shortfall += 1,
            ServiceStatus::Indeterminate => indeterminate += 1,
            ServiceStatus::Rejected => rejected += 1,
        }
        if result.actual_status == ServiceStatus::PlannedCapacityFloorMetSynthetic
            && scenario.expected_status != ServiceStatus::PlannedCapacityFloorMetSynthetic
        {
            false_continuity += 1;
        }
        if !result.expected_result_matched {
            mismatches += 1;
        }
        results.push(result);
    }

    let summary = Summary {
        scenario_count: fixture.scenarios.len(),
        planned_capacity_floor_met_synthetic: met,
        planned_capacity_shortfall: shortfall,
        indeterminate,
        rejected,
        expected_result_mismatches: mismatches,
        nominal_false_capacity_claims: false_continuity,
        authority_grants: 0,
    };
    Ok(Report {
        result: if mismatches == 0 { "pass" } else { "fail" }.to_owned(),
        evidence_class: "synthetic".to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        authority_granted: false,
        summary,
        scenarios: results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> ServiceFixture {
        serde_json::from_str(include_str!("../fixtures/v0.1/cases.json"))
            .expect("service reconfiguration corpus parses")
    }

    #[test]
    fn frozen_service_corpus_matches_expected_results() {
        let report = evaluate(&fixture()).expect("service fixture evaluates");
        assert_eq!(report.summary.scenario_count, 27);
        assert_eq!(report.summary.expected_result_mismatches, 0);
        assert_eq!(report.summary.nominal_false_capacity_claims, 0);
        assert_eq!(report.summary.authority_grants, 0);
        assert!(!report.authority_granted);
    }

    #[test]
    fn unknown_prior_outcome_never_allows_reconfiguration() {
        let report = evaluate(&fixture()).expect("service fixture evaluates");
        let row = report.scenarios.iter()
            .find(|row| row.case_id == "PRIOR_OUTCOME_UNKNOWN_BLOCKS_SWITCH")
            .expect("unknown-outcome case exists");
        assert_eq!(row.actual_status, ServiceStatus::Indeterminate);
        assert_eq!(row.actual_reason, "prior_service_outcome_unknown_blocks_reconfiguration");
    }

    #[test]
    fn shared_failure_domain_cannot_be_a_fallback() {
        let report = evaluate(&fixture()).expect("service fixture evaluates");
        let row = report.scenarios.iter()
            .find(|row| row.case_id == "FALLBACK_SHARES_FAILED_POWER_DOMAIN")
            .expect("common cause case exists");
        assert_eq!(row.actual_status, ServiceStatus::Rejected);
        assert_eq!(row.actual_reason, "fallback_shares_failed_domain");
    }

    #[test]
    fn partial_service_is_counted_only_at_accepted_capacity() {
        let report = evaluate(&fixture()).expect("service fixture evaluates");
        let row = report.scenarios.iter()
            .find(|row| row.case_id == "PARTIAL_SERVICE_COUNTS_ONLY_ACCEPTED_UNITS")
            .expect("partial service case exists");
        assert_eq!(row.available_capacity_units, 10);
        assert_eq!(row.unserved_demand_units, 10);
        assert_eq!(row.actual_status, ServiceStatus::PlannedCapacityShortfall);
    }


    #[test]
    fn independent_fallback_restores_service_after_qualified_handoff() {
        let report = evaluate(&fixture()).expect("service fixture evaluates");
        let row = report.scenarios.iter()
            .find(|row| row.case_id == "INDEPENDENT_ROBUST_FAILOVER_MEETS_FLOOR")
            .expect("independent fallback case exists");
        assert_eq!(row.actual_status, ServiceStatus::PlannedCapacityFloorMetSynthetic);
        assert_eq!(row.available_capacity_units, 15);
        assert_eq!(row.first_capacity_restored_tick, Some(3));
    }


    #[test]
    fn partial_capability_provider_cannot_substitute_for_full_service() {
        let report = evaluate(&fixture()).expect("service fixture evaluates");
        let row = report.scenarios.iter()
            .find(|row| row.case_id == "ALTERNATE_ONLY_SUPPORTS_MECHANICAL_CAPTURE")
            .expect("partial capability case exists");
        assert_eq!(row.actual_status, ServiceStatus::Rejected);
        assert_eq!(row.actual_reason, "provider_service_kind_mismatch");
    }


    #[test]
    fn supplier_identity_is_an_implicit_shared_failure_domain() {
        let mut data = fixture();
        let row = data.scenarios.iter_mut()
            .find(|row| row.case_id == "SAME_SUPPLIER_ID_DEFEATS_FALLBACK")
            .expect("same-supplier case exists");
        let provider_a = data.providers.iter()
            .find(|provider| provider.provider_id == "A")
            .expect("provider A exists");
        let provider_p = data.providers.iter()
            .find(|provider| provider.provider_id == "P")
            .expect("provider P exists");
        assert_eq!(provider_a.supplier_id, provider_p.supplier_id);
        assert!(provider_a.failure_domains.iter().all(|domain|
            !provider_p.failure_domains.contains(domain)));
        let report = evaluate(&data).expect("service fixture evaluates");
        let result = report.scenarios.iter()
            .find(|result| result.case_id == "SAME_SUPPLIER_ID_DEFEATS_FALLBACK")
            .expect("same-supplier result exists");
        assert_eq!(result.actual_status, ServiceStatus::Rejected);
        assert_eq!(result.actual_reason, "fallback_shares_failed_domain");
        let _ = row;
    }

    #[test]
    fn transition_time_is_sequential_unless_overlap_is_explicitly_qualified() {
        let report = evaluate(&fixture()).expect("service fixture evaluates");
        let row = report.scenarios.iter()
            .find(|row| row.case_id == "HANDOFF_TOO_LONG")
            .expect("long sequential handoff case exists");
        assert_eq!(row.actual_status, ServiceStatus::Rejected);
        assert_eq!(row.actual_reason, "handoff_or_requalification_incomplete");
    }

    #[test]
    fn candidate_qualification_is_checked_at_activation_even_before_demand() {
        let report = evaluate(&fixture()).expect("service fixture evaluates");
        let row = report.scenarios.iter()
            .find(|row| row.case_id == "QUALIFICATION_EXPIRES_BEFORE_ACTIVATION")
            .expect("expired qualification case exists");
        assert_eq!(row.actual_status, ServiceStatus::Indeterminate);
        assert_eq!(row.actual_reason, "candidate_robust_qualification_indeterminate");
    }

    #[test]
    fn incomplete_handoff_is_not_hidden_by_nominal_capacity() {
        let report = evaluate(&fixture()).expect("service fixture evaluates");
        let row = report.scenarios.iter()
            .find(|row| row.case_id == "HANDOFF_TOO_LONG")
            .expect("long handoff case exists");
        assert_eq!(row.actual_status, ServiceStatus::Rejected);
        assert_eq!(row.actual_reason, "handoff_or_requalification_incomplete");
    }

    #[test]
    fn service_schema_is_parseable_and_versioned() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../schemas/cases.schema.json"))
                .expect("service schema parses");
        assert_eq!(
            schema["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert_eq!(
            schema["properties"]["schema"]["const"],
            "luminous.orbital-service-reconfiguration.corpus.v0.1"
        );
    }

    #[test]
    fn deterministic_failure_sets_do_not_grant_authority() {
        let report = evaluate(&fixture()).expect("service fixture evaluates");
        assert!(!report.authority_granted);
        assert!(report.scenarios.iter().all(|row| !row.authority_granted));
    }
}
