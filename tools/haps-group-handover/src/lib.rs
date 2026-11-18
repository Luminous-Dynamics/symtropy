//! Deterministic HAPS group-handover admission and authority-transfer oracle.
//!
//! This crate validates cohort accounting and fail-closed source/target
//! authority semantics. It does not model radio propagation, mobility,
//! queueing distributions, or semi-Markov phase durations.

use std::collections::HashSet;
use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandoverError(String);

impl HandoverError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for HandoverError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for HandoverError {}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HandoverCase {
    pub schema_version: String,
    pub case_id: String,
    pub revision: u64,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub source: Source,
    pub authority_policy: String,
    pub target: Target,
    pub cohorts: Vec<Cohort>,
    pub scenarios: Vec<Scenario>,
    #[serde(default)]
    pub source_evidence_refs: Vec<String>,
    pub assumptions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub source_kind: String,
    pub source_uri: Option<String>,
    pub source_digest: Option<String>,
    pub rights_or_license_ref: Option<String>,
    pub available_at_decision_time: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub target_node_id: String,
    pub target_layer: String,
    pub available: bool,
    pub maximum_user_equivalents: usize,
    pub maximum_aggregate_bitrate_kbps: u64,
    pub admission_policy: String,
    pub tie_break_policy: String,
    pub backhaul_available: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cohort {
    pub cohort_id: String,
    pub service_class: String,
    pub priority_rank: u32,
    pub criticality: String,
    pub offered_user_count: usize,
    pub per_user_bitrate_kbps: u64,
    pub maximum_handover_latency_ms: f64,
    pub stateful: bool,
    pub fallback_policy: String,
    pub context_epoch: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhaseDurations {
    pub initiation: f64,
    pub preparation: f64,
    pub execution: f64,
    pub completion: f64,
}

impl PhaseDurations {
    fn total_ms(&self) -> f64 {
        self.initiation + self.preparation + self.execution + self.completion
    }

    fn elapsed_at_failure(&self, point: &str) -> f64 {
        match point {
            "initiation" => self.initiation,
            "preparation" | "stale_context" => self.initiation + self.preparation,
            "execution" => self.initiation + self.preparation + self.execution,
            "completion_ack_lost" => self.total_ms(),
            _ => 0.0,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub scenario_id: String,
    pub target_available: bool,
    pub backhaul_available: bool,
    pub failure_point: String,
    pub duplicate_completion_ack_delivery: bool,
    pub phase_durations_ms: PhaseDurations,
    pub expected_oracle: ExpectedOracle,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExpectedOracle {
    pub oracle_status: String,
    pub cohorts: Vec<CohortResult>,
    pub total_offered_ues: usize,
    pub total_admitted_ues: usize,
    pub total_completed_ues: usize,
    pub total_rejected_ues: usize,
    pub total_rolled_back_ues: usize,
    pub total_unresolved_ues: usize,
    pub total_deadline_missed_ues: usize,
    pub total_local_safe_mode_ues: usize,
    pub total_committed_ues: usize,
    pub duplicate_ack_ignored: bool,
    pub duplicate_authority_violations: usize,
    pub all_offered_users_accounted_for: bool,
    pub source_release_after_ack_only: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CohortResult {
    pub cohort_id: String,
    pub offered_ues: usize,
    pub admitted_ues: usize,
    pub completed_ues: usize,
    pub rejected_ues: usize,
    pub rolled_back_ues: usize,
    pub unresolved_ues: usize,
    pub deadline_missed_ues: usize,
    pub local_safe_mode_ues: usize,
    pub source_authoritative_ues: usize,
    pub target_authoritative_ues: usize,
    pub authority_overlap_ues: usize,
    pub reconciliation_required: bool,
    pub handover_latency_ms: f64,
    pub outcome: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ScenarioReport {
    pub scenario_id: String,
    pub result: ExpectedOracle,
    pub expected_oracle_matched: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct HandoverReport {
    pub case_id: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub target_node_id: String,
    pub scenarios: Vec<ScenarioReport>,
}

pub fn parse_case_json(bytes: &[u8]) -> Result<HandoverCase, HandoverError> {
    let case: HandoverCase = serde_json::from_slice(bytes)
        .map_err(|error| HandoverError::new(format!("invalid handover case JSON: {error}")))?;
    validate_case(&case)?;
    Ok(case)
}

pub fn validate_case(case: &HandoverCase) -> Result<(), HandoverError> {
    if case.schema_version != "haps-group-handover-case/v1" {
        return Err(HandoverError::new("unsupported schema_version"));
    }
    if case.case_id.trim().is_empty() || case.revision == 0 {
        return Err(HandoverError::new("case_id must be non-empty and revision positive"));
    }
    if case.evidence_class != "synthetic"
        || case.claim_ceiling != "synthetic_handover_accounting_oracle"
        || case.source.source_kind != "synthetic_generator"
        || case.source.available_at_decision_time
    {
        return Err(HandoverError::new("executable accepts synthetic offline cases only"));
    }
    if case.authority_policy != "release_source_only_after_target_completion_ack" {
        return Err(HandoverError::new("unsupported authority policy"));
    }
    if case.target.target_node_id.trim().is_empty()
        || !matches!(
            case.target.target_layer.as_str(),
            "haps" | "terrestrial" | "leo" | "drone_edge"
        )
        || case.target.maximum_user_equivalents == 0
        || case.target.maximum_aggregate_bitrate_kbps == 0
        || case.target.admission_policy != "priority_rank_then_cohort_id"
        || case.target.tie_break_policy != "cohort_id_ascending"
    {
        return Err(HandoverError::new("invalid target or unsupported admission policy"));
    }
    if case.cohorts.is_empty() || case.scenarios.is_empty() {
        return Err(HandoverError::new("at least one cohort and scenario are required"));
    }

    let mut cohort_ids = HashSet::with_capacity(case.cohorts.len());
    for cohort in &case.cohorts {
        if cohort.cohort_id.trim().is_empty() || !cohort_ids.insert(cohort.cohort_id.as_str()) {
            return Err(HandoverError::new(format!(
                "cohort ID is empty or duplicated: {}",
                cohort.cohort_id
            )));
        }
        if !matches!(
            cohort.service_class.as_str(),
            "safety_drone_control"
                | "emergency_alert"
                | "health"
                | "critical_infrastructure"
                | "public_broadband"
                | "iot"
                | "best_effort"
        ) || !matches!(
            cohort.criticality.as_str(),
            "safety_critical" | "emergency_critical" | "mission_critical" | "best_effort"
        ) {
            return Err(HandoverError::new(format!(
                "cohort {} has an unsupported service class or criticality",
                cohort.cohort_id
            )));
        }
        if cohort.offered_user_count == 0
            || cohort.per_user_bitrate_kbps == 0
            || !cohort.maximum_handover_latency_ms.is_finite()
            || cohort.maximum_handover_latency_ms <= 0.0
            || cohort.context_epoch == 0
        {
            return Err(HandoverError::new(format!(
                "cohort {} has invalid offered load, bitrate, deadline, or context epoch",
                cohort.cohort_id
            )));
        }
        if !matches!(
            cohort.fallback_policy.as_str(),
            "drone_local_safe_mode"
                | "route_to_terrestrial_gateway"
                | "store_and_forward"
                | "defer_best_effort"
                | "retain_source"
        ) {
            return Err(HandoverError::new(format!(
                "cohort {} has an unsupported fallback policy",
                cohort.cohort_id
            )));
        }
        if cohort.criticality == "safety_critical"
            && cohort.fallback_policy != "drone_local_safe_mode"
        {
            return Err(HandoverError::new(format!(
                "safety-critical cohort {} must use drone_local_safe_mode",
                cohort.cohort_id
            )));
        }
    }

    let mut total_offered_ues = 0usize;
    for cohort in &case.cohorts {
        total_offered_ues = total_offered_ues
            .checked_add(cohort.offered_user_count)
            .ok_or_else(|| HandoverError::new("total offered UE count overflow"))?;
    }

    let mut scenario_ids = HashSet::with_capacity(case.scenarios.len());
    for scenario in &case.scenarios {
        if scenario.scenario_id.trim().is_empty()
            || !scenario_ids.insert(scenario.scenario_id.as_str())
        {
            return Err(HandoverError::new(format!(
                "scenario ID is empty or duplicated: {}",
                scenario.scenario_id
            )));
        }
        if !matches!(
            scenario.failure_point.as_str(),
            "none"
                | "initiation"
                | "preparation"
                | "execution"
                | "completion_ack_lost"
                | "stale_context"
        ) {
            return Err(HandoverError::new(format!(
                "scenario {} has an unsupported failure point",
                scenario.scenario_id
            )));
        }
        if scenario.duplicate_completion_ack_delivery && scenario.failure_point != "none" {
            return Err(HandoverError::new(format!(
                "scenario {} injects a duplicate acknowledgement without a completed handover",
                scenario.scenario_id
            )));
        }
        if !scenario.phase_durations_ms.total_ms().is_finite() {
            return Err(HandoverError::new(format!(
                "scenario {} total phase duration overflowed",
                scenario.scenario_id
            )));
        }
        for (name, value) in [
            ("initiation", scenario.phase_durations_ms.initiation),
            ("preparation", scenario.phase_durations_ms.preparation),
            ("execution", scenario.phase_durations_ms.execution),
            ("completion", scenario.phase_durations_ms.completion),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(HandoverError::new(format!(
                    "scenario {} phase {name} must be finite and non-negative",
                    scenario.scenario_id
                )));
            }
        }
        if scenario.expected_oracle.oracle_status != "independently_calculated" {
            return Err(HandoverError::new(format!(
                "scenario {} lacks an independently_calculated oracle",
                scenario.scenario_id
            )));
        }
    }
    Ok(())
}

pub fn evaluate_case(case: &HandoverCase) -> Result<HandoverReport, HandoverError> {
    validate_case(case)?;
    let mut reports = Vec::new();
    reports
        .try_reserve_exact(case.scenarios.len())
        .map_err(|_| HandoverError::new("could not allocate scenario reports"))?;

    for scenario in &case.scenarios {
        let result = evaluate_scenario(case, scenario)?;
        let matched = result == scenario.expected_oracle;
        if !matched {
            return Err(HandoverError::new(format!(
                "expected oracle mismatch for scenario {}",
                scenario.scenario_id
            )));
        }
        reports.push(ScenarioReport {
            scenario_id: scenario.scenario_id.clone(),
            result,
            expected_oracle_matched: matched,
        });
    }

    Ok(HandoverReport {
        case_id: case.case_id.clone(),
        evidence_class: case.evidence_class.clone(),
        claim_ceiling: case.claim_ceiling.clone(),
        target_node_id: case.target.target_node_id.clone(),
        scenarios: reports,
    })
}

fn evaluate_scenario(
    case: &HandoverCase,
    scenario: &Scenario,
) -> Result<ExpectedOracle, HandoverError> {
    let target_ready = case.target.available
        && case.target.backhaul_available
        && scenario.target_available
        && scenario.backhaul_available;
    let early_failure = matches!(
        scenario.failure_point.as_str(),
        "initiation" | "preparation" | "stale_context"
    );

    let mut ordered: Vec<&Cohort> = case.cohorts.iter().collect();
    ordered.sort_by(|left, right| {
        left.priority_rank
            .cmp(&right.priority_rank)
            .then_with(|| left.cohort_id.cmp(&right.cohort_id))
    });

    let mut remaining_users = case.target.maximum_user_equivalents;
    let mut remaining_bitrate = case.target.maximum_aggregate_bitrate_kbps;
    let mut admitted_by_id = std::collections::HashMap::with_capacity(case.cohorts.len());

    if target_ready && !early_failure {
        for cohort in ordered {
            let rate_limited_users = remaining_bitrate / cohort.per_user_bitrate_kbps;
            let admitted = cohort
                .offered_user_count
                .min(remaining_users)
                .min(usize::try_from(rate_limited_users).unwrap_or(usize::MAX));
            let used_bitrate = (admitted as u64)
                .checked_mul(cohort.per_user_bitrate_kbps)
                .ok_or_else(|| HandoverError::new("admitted bitrate multiplication overflow"))?;
            remaining_users -= admitted;
            remaining_bitrate -= used_bitrate;
            admitted_by_id.insert(cohort.cohort_id.as_str(), admitted);
        }
    }

    let total_latency = scenario.phase_durations_ms.total_ms();
    let mut results = Vec::new();
    results
        .try_reserve_exact(case.cohorts.len())
        .map_err(|_| HandoverError::new("could not allocate cohort results"))?;

    for cohort in &case.cohorts {
        let admitted = *admitted_by_id.get(cohort.cohort_id.as_str()).unwrap_or(&0);
        let rejected = cohort.offered_user_count - admitted;
        let mut completed = 0usize;
        let mut rolled_back = 0usize;
        let mut unresolved = 0usize;
        let mut deadline_missed = 0usize;
        let latency = if admitted == 0 {
            if target_ready {
                scenario
                    .phase_durations_ms
                    .elapsed_at_failure(&scenario.failure_point)
            } else {
                0.0
            }
        } else {
            match scenario.failure_point.as_str() {
                "execution" => {
                    rolled_back = admitted;
                    scenario.phase_durations_ms.elapsed_at_failure("execution")
                }
                "completion_ack_lost" => {
                    unresolved = admitted;
                    total_latency
                }
                "none" => {
                    if total_latency > cohort.maximum_handover_latency_ms {
                        rolled_back = admitted;
                        deadline_missed = admitted;
                    } else {
                        completed = admitted;
                    }
                    total_latency
                }
                "initiation" | "preparation" | "stale_context" => {
                    scenario
                        .phase_durations_ms
                        .elapsed_at_failure(&scenario.failure_point)
                }
                _ => return Err(HandoverError::new("unsupported failure point during evaluation")),
            }
        };

        let local_safe_mode = if cohort.criticality == "safety_critical" {
            cohort.offered_user_count - completed
        } else {
            0
        };
        let source_authoritative = cohort.offered_user_count - completed;
        let target_authoritative = completed;
        let outcome = if unresolved > 0 {
            "reconciliation_required"
        } else if completed > 0 && (rejected > 0 || rolled_back > 0) {
            "partially_completed"
        } else if completed > 0 {
            "completed"
        } else if rolled_back > 0 {
            "rolled_back"
        } else {
            "rejected"
        };

        results.push(CohortResult {
            cohort_id: cohort.cohort_id.clone(),
            offered_ues: cohort.offered_user_count,
            admitted_ues: admitted,
            completed_ues: completed,
            rejected_ues: rejected,
            rolled_back_ues: rolled_back,
            unresolved_ues: unresolved,
            deadline_missed_ues: deadline_missed,
            local_safe_mode_ues: local_safe_mode,
            source_authoritative_ues: source_authoritative,
            target_authoritative_ues: target_authoritative,
            authority_overlap_ues: 0,
            reconciliation_required: unresolved > 0,
            handover_latency_ms: latency,
            outcome: outcome.to_owned(),
        });
    }

    let total = |select: fn(&CohortResult) -> usize| -> usize {
        results.iter().map(select).sum()
    };
    let total_offered = total(|row| row.offered_ues);
    let total_completed = total(|row| row.completed_ues);
    let total_rejected = total(|row| row.rejected_ues);
    let total_rolled_back = total(|row| row.rolled_back_ues);
    let total_unresolved = total(|row| row.unresolved_ues);
    let total_admitted = total(|row| row.admitted_ues);
    let all_accounted = total_completed + total_rejected + total_rolled_back + total_unresolved
        == total_offered;

    let source_release_after_ack_only = results.iter().all(|row| {
        row.completed_ues + row.source_authoritative_ues == row.offered_ues
            && row.target_authoritative_ues == row.completed_ues
            && row.authority_overlap_ues == 0
            && row.source_authoritative_ues + row.target_authoritative_ues == row.offered_ues
    });
    let duplicate_ack_ignored =
        scenario.duplicate_completion_ack_delivery
            && scenario.failure_point == "none"
            && total_completed > 0;

    Ok(ExpectedOracle {
        oracle_status: "independently_calculated".to_owned(),
        cohorts: results,
        total_offered_ues: total_offered,
        total_admitted_ues: total_admitted,
        total_completed_ues: total_completed,
        total_rejected_ues: total_rejected,
        total_rolled_back_ues: total_rolled_back,
        total_unresolved_ues: total_unresolved,
        total_deadline_missed_ues: total(|row| row.deadline_missed_ues),
        total_local_safe_mode_ues: total(|row| row.local_safe_mode_ues),
        total_committed_ues: total_completed,
        duplicate_ack_ignored,
        duplicate_authority_violations: 0,
        all_offered_users_accounted_for: all_accounted,
        source_release_after_ack_only,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!(
        "../../../docs/research/aviation/examples/synthetic_haps_group_handover_case_v1.json"
    );

    fn fixture() -> HandoverCase {
        parse_case_json(FIXTURE.as_bytes()).expect("fixture should parse and validate")
    }

    fn report() -> HandoverReport {
        evaluate_case(&fixture()).expect("all scenario oracles should match")
    }

    #[test]
    fn all_scenario_oracles_match() {
        let result = report();
        assert_eq!(result.scenarios.len(), 5);
        assert!(result.scenarios.iter().all(|row| row.expected_oracle_matched));
    }

    #[test]
    fn nominal_group_admission_respects_priority_and_both_target_limits() {
        let result = report();
        let nominal = &result.scenarios[0].result;
        assert_eq!(nominal.total_offered_ues, 152);
        assert_eq!(nominal.total_admitted_ues, 71);
        assert_eq!(nominal.total_completed_ues, 71);
        assert_eq!(nominal.total_rejected_ues, 81);
        assert_eq!(nominal.cohorts[0].completed_ues, 12);
        assert_eq!(nominal.cohorts[1].completed_ues, 40);
        assert_eq!(nominal.cohorts[2].completed_ues, 19);
        assert!(nominal.duplicate_ack_ignored);
    }

    #[test]
    fn backhaul_loss_rejects_target_admission_and_preserves_source_authority() {
        let result = &report().scenarios[1].result;
        assert_eq!(result.total_admitted_ues, 0);
        assert_eq!(result.total_rejected_ues, 152);
        assert_eq!(result.cohorts.iter().map(|row| row.source_authoritative_ues).sum::<usize>(), 152);
        assert_eq!(result.total_local_safe_mode_ues, 12);
        assert_eq!(result.total_committed_ues, 0);
    }

    #[test]
    fn execution_failure_rolls_back_every_admitted_user() {
        let result = &report().scenarios[2].result;
        assert_eq!(result.total_admitted_ues, 71);
        assert_eq!(result.total_rolled_back_ues, 71);
        assert_eq!(result.total_completed_ues, 0);
        assert_eq!(result.total_rejected_ues, 81);
        assert_eq!(result.total_committed_ues, 0);
        assert!(result.source_release_after_ack_only);
    }

    #[test]
    fn lost_completion_ack_is_unresolved_and_never_promoted_to_success() {
        let result = &report().scenarios[3].result;
        assert_eq!(result.total_unresolved_ues, 71);
        assert_eq!(result.total_completed_ues, 0);
        assert_eq!(result.total_committed_ues, 0);
        assert_eq!(result.total_local_safe_mode_ues, 12);
        assert!(result.cohorts.iter().filter(|row| row.unresolved_ues > 0).all(|row| {
            row.reconciliation_required
                && row.target_authoritative_ues == 0
                && row.source_authoritative_ues == row.offered_ues
        }));
    }

    #[test]
    fn safety_critical_deadline_overrun_is_rolled_back_locally() {
        let result = &report().scenarios[4].result;
        assert_eq!(result.total_completed_ues, 59);
        assert_eq!(result.total_rolled_back_ues, 12);
        assert_eq!(result.total_deadline_missed_ues, 12);
        assert_eq!(result.cohorts[0].local_safe_mode_ues, 12);
        assert_eq!(result.cohorts[0].target_authoritative_ues, 0);
        assert_eq!(result.cohorts[1].completed_ues, 40);
        assert_eq!(result.cohorts[2].completed_ues, 19);
    }

    #[test]
    fn no_cohort_can_have_simultaneous_source_and_target_authority() {
        let result = report();
        for scenario in result.scenarios {
            for cohort in scenario.result.cohorts {
                assert_eq!(cohort.authority_overlap_ues, 0);
                assert_eq!(
                    cohort.source_authoritative_ues + cohort.target_authoritative_ues,
                    cohort.offered_ues
                );
            }
            assert_eq!(scenario.result.duplicate_authority_violations, 0);
        }
    }

    #[test]
    fn safety_critical_cohort_cannot_remove_its_local_safe_fallback() {
        let mut case = fixture();
        case.cohorts[0].fallback_policy = "store_and_forward".to_owned();
        assert!(validate_case(&case).is_err());
    }

    #[test]
    fn duplicate_ack_injection_is_rejected_for_failed_transfers() {
        let mut case = fixture();
        case.scenarios[1].duplicate_completion_ack_delivery = true;
        assert!(validate_case(&case).is_err());
    }

    #[test]
    fn priority_rank_is_lower_is_higher_and_ties_use_cohort_id() {
        let mut case = fixture();
        case.cohorts[1].priority_rank = 1;
        case.cohorts[2].priority_rank = 1;
        case.cohorts[1].cohort_id = "z-emergency".to_owned();
        case.cohorts[2].cohort_id = "a-broadband".to_owned();
        let result = evaluate_scenario(&case, &case.scenarios[0]).unwrap();
        assert!(result.cohorts.iter().all(|row| row.authority_overlap_ues == 0));
        let broadband = result.cohorts.iter().find(|row| row.cohort_id == "a-broadband").unwrap();
        let emergency = result.cohorts.iter().find(|row| row.cohort_id == "z-emergency").unwrap();
        assert_eq!(broadband.completed_ues, 27);
        assert_eq!(emergency.completed_ues, 1);
    }

    #[test]
    fn partial_group_outcomes_account_for_every_user() {
        let result = report();
        for scenario in result.scenarios {
            let oracle = scenario.result;
            assert_eq!(
                oracle.total_completed_ues
                    + oracle.total_rejected_ues
                    + oracle.total_rolled_back_ues
                    + oracle.total_unresolved_ues,
                oracle.total_offered_ues
            );
            assert!(oracle.all_offered_users_accounted_for);
            assert!(oracle.source_release_after_ack_only);
        }
    }

    #[test]
    fn stale_context_fails_before_any_target_admission() {
        let mut case = fixture();
        case.scenarios[0].failure_point = "stale_context".to_owned();
        case.scenarios[0].duplicate_completion_ack_delivery = false;
        let result = evaluate_scenario(&case, &case.scenarios[0]).unwrap();
        assert_eq!(result.total_admitted_ues, 0);
        assert_eq!(result.total_rejected_ues, 152);
        assert_eq!(result.total_completed_ues, 0);
    }
}
